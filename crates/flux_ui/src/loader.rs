use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use flux_mod_loader::{DiscoveredMod, ResolvedModOrder};
use ron::{Options, extensions::Extensions};
use serde::Deserialize;

use crate::error::UiRegistryError;
use crate::registry::UiRegistry;
use crate::types::{
    ButtonWidget, ContainerWidget, ExtensionPointWidget, TextWidget, UiExtensionDefinition,
    UiExtensionOperation, UiMenuDefinition, UiSource, WidgetKind, WidgetNode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLoadReport {
    pub registry: Option<UiRegistry>,
    pub errors: Vec<UiRegistryError>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
enum ParsedUiFile {
    UiMenu {
        id: crate::types::UiMenuId,
        root: ParsedWidgetNode,
    },
    UiExtension {
        target: crate::types::UiExtensionPointId,
        operation: UiExtensionOperation,
        child: ParsedWidgetNode,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
enum ParsedWidgetNode {
    Container {
        id: crate::types::UiWidgetId,
        layout: crate::types::ContainerLayout,
        #[serde(default)]
        children: Vec<ParsedWidgetNode>,
    },
    Text {
        id: crate::types::UiWidgetId,
        text: crate::types::LocalizationKey,
    },
    Button {
        id: crate::types::UiWidgetId,
        text: crate::types::LocalizationKey,
        action: crate::types::BindingAction,
    },
    ExtensionPoint {
        id: crate::types::UiWidgetId,
        extension_point: crate::types::UiExtensionPointId,
    },
}

enum UiFileDefinition {
    Menu(UiMenuDefinition),
    Extension(UiExtensionDefinition),
}

pub fn load_ui_registry(
    valid_mods: &[DiscoveredMod],
    resolved_order: &ResolvedModOrder,
) -> UiLoadReport {
    let mods_by_id: BTreeMap<&str, &DiscoveredMod> = valid_mods
        .iter()
        .map(|module| (module.manifest.mod_id.as_str(), module))
        .collect();

    let mut menu_definitions = Vec::new();
    let mut extension_definitions = Vec::new();
    let mut errors = Vec::new();

    for mod_id in &resolved_order.ordered_mod_ids {
        let module = match mods_by_id.get(mod_id.as_str()) {
            Some(module) => *module,
            None => {
                errors.push(UiRegistryError::ResolvedModMissing {
                    mod_id: mod_id.to_string().into(),
                });
                continue;
            }
        };

        let ui_dir = module.directory_path.join("ui");
        for file in collect_ron_files(module, &ui_dir, &mut errors) {
            match parse_ui_file(module, &file) {
                Ok((UiFileDefinition::Menu(menu), source)) => {
                    menu_definitions.push((menu, source));
                }
                Ok((UiFileDefinition::Extension(extension), source)) => {
                    extension_definitions.push((extension, source));
                }
                Err(error) => errors.push(error),
            }
        }
    }

    let mut registry = UiRegistry::new();

    for (menu, source) in menu_definitions {
        if let Err(error) = validate_menu_namespaces(&menu, &source) {
            errors.push(error);
            continue;
        }
        if let Err(error) = registry.add_menu(menu, source) {
            errors.push(error);
        }
    }

    for (extension, source) in extension_definitions {
        if let Err(error) = validate_extension_namespaces(&extension, &source) {
            errors.push(error);
            continue;
        }
        if let Err(error) = registry.apply_extension(extension, source) {
            errors.push(error);
        }
    }

    if let Err(validation_errors) = registry.validate_open_menu_targets() {
        errors.extend(validation_errors);
    }

    if errors.is_empty() {
        registry.freeze();
        UiLoadReport {
            registry: Some(registry),
            errors,
        }
    } else {
        UiLoadReport {
            registry: None,
            errors,
        }
    }
}

fn parse_ui_file(
    module: &DiscoveredMod,
    file: &Path,
) -> Result<(UiFileDefinition, UiSource), UiRegistryError> {
    let source = UiSource {
        mod_id: module.manifest.mod_id.to_string(),
        file: file.to_string_lossy().to_string(),
    };

    let body = read_file(module, file, &source)?;
    let options = Options::default().with_default_extension(Extensions::UNWRAP_VARIANT_NEWTYPES);
    let parsed: ParsedUiFile =
        options
            .from_str(&body)
            .map_err(|error| UiRegistryError::FileParse {
                mod_id: source.mod_id.clone().into(),
                file: source.file.clone().into(),
                reason: error.to_string().into(),
            })?;

    match parsed {
        ParsedUiFile::UiMenu { id, root } => Ok((
            UiFileDefinition::Menu(UiMenuDefinition {
                id,
                root: root.into_widget_node(),
            }),
            source,
        )),
        ParsedUiFile::UiExtension {
            target,
            operation,
            child,
        } => Ok((
            UiFileDefinition::Extension(UiExtensionDefinition {
                target,
                operation,
                child: child.into_widget_node(),
            }),
            source,
        )),
    }
}

fn validate_menu_namespaces(
    menu: &UiMenuDefinition,
    source: &UiSource,
) -> Result<(), UiRegistryError> {
    validate_namespace(source, "id", menu.id.0.as_str(), menu.id.0.namespace())?;

    validate_widget_tree_namespaces(&menu.root, source)
}

fn validate_extension_namespaces(
    extension: &UiExtensionDefinition,
    source: &UiSource,
) -> Result<(), UiRegistryError> {
    validate_widget_tree_namespaces(&extension.child, source)
}

fn validate_widget_tree_namespaces(
    node: &WidgetNode,
    source: &UiSource,
) -> Result<(), UiRegistryError> {
    validate_namespace(
        source,
        "widget.id",
        node.id.0.as_str(),
        node.id.0.namespace(),
    )?;

    if let WidgetKind::ExtensionPoint(extension_point) = &node.kind {
        validate_namespace(
            source,
            "widget.extension_point",
            extension_point.extension_point.0.as_str(),
            extension_point.extension_point.0.namespace(),
        )?;
    }

    for child in &node.children {
        validate_widget_tree_namespaces(child, source)?;
    }

    Ok(())
}

fn validate_namespace(
    source: &UiSource,
    field: &str,
    value: &str,
    actual_namespace: &str,
) -> Result<(), UiRegistryError> {
    if actual_namespace == source.mod_id {
        return Ok(());
    }

    Err(UiRegistryError::NamespaceMismatch {
        mod_id: source.mod_id.clone().into(),
        file: source.file.clone().into(),
        field: field.to_owned().into(),
        value: value.to_owned().into(),
        actual_namespace: actual_namespace.to_owned().into(),
        expected_mod_id: source.mod_id.clone().into(),
    })
}

fn collect_ron_files(
    module: &DiscoveredMod,
    directory: &Path,
    errors: &mut Vec<UiRegistryError>,
) -> Vec<PathBuf> {
    if !directory.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();
    if let Err(error) =
        collect_ron_files_recursive(module.manifest.mod_id.as_str(), directory, &mut files)
    {
        errors.push(error);
        return Vec::new();
    }

    files.sort_by(|left, right| {
        normalized_relative_path(directory, left).cmp(&normalized_relative_path(directory, right))
    });
    files
}

fn collect_ron_files_recursive(
    mod_id: &str,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), UiRegistryError> {
    let read_dir = fs::read_dir(directory).map_err(|error| UiRegistryError::DirectoryRead {
        mod_id: mod_id.to_owned().into(),
        path: directory.to_string_lossy().to_string().into(),
        reason: error.to_string().into(),
    })?;

    let mut entries = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|error| UiRegistryError::DirectoryRead {
            mod_id: mod_id.to_owned().into(),
            path: directory.to_string_lossy().to_string().into(),
            reason: error.to_string().into(),
        })?;
        entries.push(entry.path());
    }

    entries.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });

    for entry_path in entries {
        if entry_path.is_dir() {
            collect_ron_files_recursive(mod_id, &entry_path, files)?;
            continue;
        }

        if is_ron_file(&entry_path) {
            files.push(entry_path);
        }
    }

    Ok(())
}

fn normalized_relative_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

fn is_ron_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ron"))
}

fn read_file(
    module: &DiscoveredMod,
    file: &Path,
    source: &UiSource,
) -> Result<String, UiRegistryError> {
    fs::read_to_string(file).map_err(|error| UiRegistryError::FileRead {
        mod_id: module.manifest.mod_id.to_string().into(),
        file: source.file.clone().into(),
        reason: error.to_string().into(),
    })
}

impl ParsedWidgetNode {
    fn into_widget_node(self) -> WidgetNode {
        match self {
            ParsedWidgetNode::Container {
                id,
                layout,
                children,
            } => WidgetNode {
                id,
                kind: WidgetKind::Container(ContainerWidget { layout }),
                children: children
                    .into_iter()
                    .map(ParsedWidgetNode::into_widget_node)
                    .collect(),
            },
            ParsedWidgetNode::Text { id, text } => WidgetNode {
                id,
                kind: WidgetKind::Text(TextWidget { text }),
                children: Vec::new(),
            },
            ParsedWidgetNode::Button { id, text, action } => WidgetNode {
                id,
                kind: WidgetKind::Button(ButtonWidget { text, action }),
                children: Vec::new(),
            },
            ParsedWidgetNode::ExtensionPoint {
                id,
                extension_point,
            } => WidgetNode {
                id,
                kind: WidgetKind::ExtensionPoint(ExtensionPointWidget { extension_point }),
                children: Vec::new(),
            },
        }
    }
}
