use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use flux_core::PrototypeId;
use flux_mod_loader::{DiscoveredMod, ResolvedModOrder};
use serde::Deserialize;
use serde::de::Deserializer;

use crate::ContentRegistry;
use crate::ContentRegistryError;
use crate::types::{
    GasPrototype, GasPrototypePatch, LocalizationKey, PrototypeBody, PrototypeKind, PrototypePatch,
    PrototypePatchBody, PrototypeSource, SolidCellPrototype, SolidCellPrototypePatch,
    StructurePrototype, StructurePrototypePatch, SubstancePrototype, SubstancePrototypePatch,
    TileSize,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ContentLoadReport {
    pub registry: Option<ContentRegistry>,
    pub errors: Vec<ContentRegistryError>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
enum ParsedPrototypeBody {
    #[serde(rename = "SubstancePrototype")]
    Substance {
        id: PrototypeId,
        display_name: LocalizationKey,
    },
    #[serde(rename = "SolidCellPrototype")]
    SolidCell {
        id: PrototypeId,
        display_name: LocalizationKey,
        gas_permeable: bool,
    },
    #[serde(rename = "StructurePrototype")]
    Structure {
        id: PrototypeId,
        display_name: LocalizationKey,
        size: TileSize,
    },
    #[serde(rename = "GasPrototype")]
    Gas {
        id: PrototypeId,
        display_name: LocalizationKey,
        molar_mass: f32,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
enum ParsedPrototypePatchWrapper {
    PrototypePatch {
        target: PrototypeId,
        body: ParsedPrototypePatchBody,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
enum ParsedPrototypePatchBody {
    Substance {
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        display_name: Option<LocalizationKey>,
    },
    SolidCell {
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        display_name: Option<LocalizationKey>,
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        gas_permeable: Option<bool>,
    },
    Structure {
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        display_name: Option<LocalizationKey>,
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        size: Option<TileSize>,
    },
    Gas {
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        display_name: Option<LocalizationKey>,
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        molar_mass: Option<f32>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PatchOptionValue<T> {
    Plain(T),
    Wrapped(Option<T>),
}

fn deserialize_patch_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let parsed = PatchOptionValue::<T>::deserialize(deserializer)?;
    Ok(match parsed {
        PatchOptionValue::Plain(value) => Some(value),
        PatchOptionValue::Wrapped(value) => value,
    })
}

pub fn load_content_registry(
    valid_mods: &[DiscoveredMod],
    resolved_order: &ResolvedModOrder,
) -> ContentLoadReport {
    let mut registry = ContentRegistry::new();
    let mut errors = Vec::new();

    let mods_by_id: BTreeMap<&str, &DiscoveredMod> = valid_mods
        .iter()
        .map(|module| (module.manifest.mod_id.as_str(), module))
        .collect();

    for mod_id in &resolved_order.ordered_mod_ids {
        let module = match mods_by_id.get(mod_id.as_str()) {
            Some(module) => *module,
            None => {
                errors.push(ContentRegistryError::ResolvedModMissing {
                    mod_id: mod_id.to_string().into(),
                });
                continue;
            }
        };

        load_mod_prototypes(module, &mut registry, &mut errors);
        apply_mod_patches(module, &mut registry, &mut errors);
    }

    if errors.is_empty() {
        registry.freeze();
        ContentLoadReport {
            registry: Some(registry),
            errors,
        }
    } else {
        ContentLoadReport {
            registry: None,
            errors,
        }
    }
}

fn load_mod_prototypes(
    module: &DiscoveredMod,
    registry: &mut ContentRegistry,
    errors: &mut Vec<ContentRegistryError>,
) {
    load_mod_prototypes_for_kind(
        module,
        registry,
        errors,
        "solid_cells",
        PrototypeKind::SolidCell,
    );
    load_mod_prototypes_for_kind(
        module,
        registry,
        errors,
        "substances",
        PrototypeKind::Substance,
    );
    load_mod_prototypes_for_kind(
        module,
        registry,
        errors,
        "structures",
        PrototypeKind::Structure,
    );
    load_mod_prototypes_for_kind(module, registry, errors, "gases", PrototypeKind::Gas);
}

fn load_mod_prototypes_for_kind(
    module: &DiscoveredMod,
    registry: &mut ContentRegistry,
    errors: &mut Vec<ContentRegistryError>,
    directory_name: &str,
    expected_kind: PrototypeKind,
) {
    let dir = module.directory_path.join("content").join(directory_name);
    for file in collect_ron_files(module, &dir, errors) {
        match parse_prototype(module, &file, expected_kind) {
            Ok((prototype, source)) => {
                if let Err(error) = registry.add_prototype(prototype, source) {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
    }
}

fn apply_mod_patches(
    module: &DiscoveredMod,
    registry: &mut ContentRegistry,
    errors: &mut Vec<ContentRegistryError>,
) {
    let dir = module.directory_path.join("content").join("patches");
    let mut seen_targets: BTreeMap<PrototypeId, String> = BTreeMap::new();

    for file in collect_ron_files(module, &dir, errors) {
        match parse_patch(module, &file) {
            Ok((patch, source)) => {
                if let Some(first_file) = seen_targets.get(&patch.target) {
                    errors.push(ContentRegistryError::DuplicatePatchTargetInMod {
                        mod_id: source.mod_id.clone().into(),
                        first_file: first_file.clone().into(),
                        duplicate_file: source.file.clone().into(),
                        target: patch.target.to_string().into(),
                    });
                    continue;
                }

                seen_targets.insert(patch.target.clone(), source.file.clone());
                if let Err(error) = registry.apply_patch(patch, source) {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
    }
}

fn parse_prototype(
    module: &DiscoveredMod,
    file: &Path,
    expected_kind: PrototypeKind,
) -> Result<(PrototypeBody, PrototypeSource), ContentRegistryError> {
    let source = PrototypeSource::from_discovered(module, file);
    let body = read_file(module, file)?;
    let parsed: ParsedPrototypeBody =
        ron::from_str(&body).map_err(|error| ContentRegistryError::FileParse {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_kind: expected_kind.as_str().into(),
            reason: error.to_string().into(),
        })?;

    let prototype = match parsed {
        ParsedPrototypeBody::Substance { id, display_name } => {
            PrototypeBody::SubstancePrototype(SubstancePrototype { id, display_name })
        }
        ParsedPrototypeBody::SolidCell {
            id,
            display_name,
            gas_permeable,
        } => PrototypeBody::SolidCellPrototype(SolidCellPrototype {
            id,
            display_name,
            gas_permeable,
        }),
        ParsedPrototypeBody::Structure {
            id,
            display_name,
            size,
        } => PrototypeBody::StructurePrototype(StructurePrototype {
            id,
            display_name,
            size,
        }),
        ParsedPrototypeBody::Gas {
            id,
            display_name,
            molar_mass,
        } => PrototypeBody::GasPrototype(GasPrototype {
            id,
            display_name,
            molar_mass,
        }),
    };

    if prototype.kind() != expected_kind {
        return Err(ContentRegistryError::FileParse {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_kind: expected_kind.as_str().into(),
            reason: format!(
                "expected {} wrapper, got {} wrapper",
                expected_kind.as_str(),
                prototype.kind().as_str()
            )
            .into(),
        });
    }

    validate_prototype_id_namespace(module, file, prototype.id())?;
    validate_prototype_body(&prototype, &source)?;

    Ok((prototype, source))
}

fn parse_patch(
    module: &DiscoveredMod,
    file: &Path,
) -> Result<(PrototypePatch, PrototypeSource), ContentRegistryError> {
    let source = PrototypeSource::from_discovered(module, file);
    let body = read_file(module, file)?;
    let parsed: ParsedPrototypePatchWrapper =
        ron::from_str(&body).map_err(|error| ContentRegistryError::FileParse {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_kind: "patch".into(),
            reason: error.to_string().into(),
        })?;

    let (target, parsed_body) = match parsed {
        ParsedPrototypePatchWrapper::PrototypePatch { target, body } => (target, body),
    };

    let patch_body = match parsed_body {
        ParsedPrototypePatchBody::Substance { display_name } => {
            PrototypePatchBody::Substance(SubstancePrototypePatch { display_name })
        }
        ParsedPrototypePatchBody::SolidCell {
            display_name,
            gas_permeable,
        } => PrototypePatchBody::SolidCell(SolidCellPrototypePatch {
            display_name,
            gas_permeable,
        }),
        ParsedPrototypePatchBody::Structure { display_name, size } => {
            PrototypePatchBody::Structure(StructurePrototypePatch { display_name, size })
        }
        ParsedPrototypePatchBody::Gas {
            display_name,
            molar_mass,
        } => PrototypePatchBody::Gas(GasPrototypePatch {
            display_name,
            molar_mass,
        }),
    };

    let patch = PrototypePatch {
        target,
        body: patch_body,
    };

    Ok((patch, source))
}

fn validate_prototype_body(
    prototype: &PrototypeBody,
    source: &PrototypeSource,
) -> Result<(), ContentRegistryError> {
    prototype.validate(source)
}

fn validate_prototype_id_namespace(
    module: &DiscoveredMod,
    file: &Path,
    prototype_id: &PrototypeId,
) -> Result<(), ContentRegistryError> {
    if prototype_id.namespace() == module.manifest.mod_id.as_str() {
        return Ok(());
    }

    Err(ContentRegistryError::InvalidPrototypeField {
        mod_id: module.manifest.mod_id.to_string().into(),
        file: file.to_string_lossy().to_string().into(),
        prototype_id: prototype_id.to_string().into(),
        field: "id".into(),
        reason: format!(
            "prototype namespace `{}` must match mod id `{}`",
            prototype_id.namespace(),
            module.manifest.mod_id
        )
        .into(),
    })
}

fn collect_ron_files(
    module: &DiscoveredMod,
    directory: &Path,
    errors: &mut Vec<ContentRegistryError>,
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
) -> Result<(), ContentRegistryError> {
    let read_dir =
        fs::read_dir(directory).map_err(|error| ContentRegistryError::DirectoryRead {
            mod_id: mod_id.to_owned().into(),
            path: directory.to_string_lossy().to_string().into(),
            reason: error.to_string().into(),
        })?;

    let mut entries = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|error| ContentRegistryError::DirectoryRead {
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

fn read_file(module: &DiscoveredMod, file: &Path) -> Result<String, ContentRegistryError> {
    fs::read_to_string(file).map_err(|error| ContentRegistryError::FileRead {
        mod_id: module.manifest.mod_id.to_string().into(),
        file: file.to_string_lossy().to_string().into(),
        reason: error.to_string().into(),
    })
}
