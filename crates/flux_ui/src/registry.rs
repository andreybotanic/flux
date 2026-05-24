use std::collections::{BTreeMap, BTreeSet};

use crate::error::UiRegistryError;
use crate::types::{
    UiAction, UiExtensionDefinition, UiMenuDefinition, UiMenuId, UiSource, UiWidgetId, WidgetKind,
    WidgetNode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiRegistryState {
    Building,
    Frozen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiMenuRecord {
    pub definition: UiMenuDefinition,
    pub source: UiSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IdSource {
    mod_id: String,
    file: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExtensionPointLocation {
    menu_id: UiMenuId,
    source: IdSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRegistry {
    state: UiRegistryState,
    menus: BTreeMap<UiMenuId, UiMenuRecord>,
    widget_index: BTreeMap<UiWidgetId, IdSource>,
    extension_points: BTreeMap<crate::types::UiExtensionPointId, ExtensionPointLocation>,
}

impl UiRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: UiRegistryState::Building,
            menus: BTreeMap::new(),
            widget_index: BTreeMap::new(),
            extension_points: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn state(&self) -> UiRegistryState {
        self.state
    }

    #[must_use]
    pub fn is_frozen(&self) -> bool {
        self.state == UiRegistryState::Frozen
    }

    pub fn freeze(&mut self) {
        self.state = UiRegistryState::Frozen;
    }

    #[must_use]
    pub fn menus_len(&self) -> usize {
        self.menus.len()
    }

    pub fn menus(&self) -> impl Iterator<Item = &UiMenuRecord> {
        self.menus.values()
    }

    pub fn menu(&self, menu_id: &UiMenuId) -> Option<&UiMenuDefinition> {
        self.menus.get(menu_id).map(|entry| &entry.definition)
    }

    #[must_use]
    pub fn menu_ids(&self) -> BTreeSet<UiMenuId> {
        self.menus.keys().cloned().collect()
    }

    pub fn add_menu(
        &mut self,
        menu: UiMenuDefinition,
        source: UiSource,
    ) -> Result<(), UiRegistryError> {
        self.ensure_mutable();

        if let Some(existing) = self.menus.get(&menu.id) {
            return Err(UiRegistryError::DuplicateMenuId {
                menu_id: menu.id.to_string().into(),
                existing_mod: existing.source.mod_id.clone().into(),
                existing_file: existing.source.file.clone().into(),
                duplicate_mod: source.mod_id.clone().into(),
                duplicate_file: source.file.clone().into(),
            });
        }

        self.register_widget_tree_ids(&menu.id, &menu.root, &source)?;
        self.menus.insert(
            menu.id.clone(),
            UiMenuRecord {
                definition: menu,
                source,
            },
        );

        Ok(())
    }

    pub fn apply_extension(
        &mut self,
        extension: UiExtensionDefinition,
        source: UiSource,
    ) -> Result<(), UiRegistryError> {
        self.ensure_mutable();

        let location = self
            .extension_points
            .get(&extension.target)
            .ok_or_else(|| UiRegistryError::ExtensionTargetNotFound {
                mod_id: source.mod_id.clone().into(),
                file: source.file.clone().into(),
                target: extension.target.to_string().into(),
            })?
            .clone();

        self.register_widget_tree_ids(&location.menu_id, &extension.child, &source)?;

        let menu_record = self
            .menus
            .get_mut(&location.menu_id)
            .expect("extension point location must reference existing menu");

        let target_node =
            find_extension_point_mut(&mut menu_record.definition.root, &extension.target)
                .expect("extension point index must be in sync with menu tree");
        target_node.children.push(extension.child);

        Ok(())
    }

    pub fn validate_open_menu_targets(&self) -> Result<(), Vec<UiRegistryError>> {
        let known_menus = self.menu_ids();
        let mut errors = Vec::new();

        for menu_record in self.menus.values() {
            let menu_id = &menu_record.definition.id;
            menu_record.definition.root.visit(&mut |node| {
                if let WidgetKind::Button(button) = &node.kind
                    && let UiAction::OpenMenu(target_menu) = &button.action
                    && !known_menus.contains(target_menu)
                {
                    errors.push(UiRegistryError::OpenMenuTargetNotFound {
                        menu_id: menu_id.to_string().into(),
                        widget_id: node.id.to_string().into(),
                        target_menu: target_menu.to_string().into(),
                    });
                }
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn ensure_mutable(&self) {
        assert!(
            self.state == UiRegistryState::Building,
            "ui registry must stay mutable only in building stage"
        );
    }

    fn register_widget_tree_ids(
        &mut self,
        menu_id: &UiMenuId,
        root: &WidgetNode,
        source: &UiSource,
    ) -> Result<(), UiRegistryError> {
        self.register_node(menu_id, root, source)
    }

    fn register_node(
        &mut self,
        menu_id: &UiMenuId,
        node: &WidgetNode,
        source: &UiSource,
    ) -> Result<(), UiRegistryError> {
        if let Some(existing) = self.widget_index.get(&node.id) {
            return Err(UiRegistryError::DuplicateWidgetId {
                widget_id: node.id.to_string().into(),
                existing_mod: existing.mod_id.clone().into(),
                existing_file: existing.file.clone().into(),
                duplicate_mod: source.mod_id.clone().into(),
                duplicate_file: source.file.clone().into(),
            });
        }

        let id_source = IdSource {
            mod_id: source.mod_id.clone(),
            file: source.file.clone(),
        };
        self.widget_index.insert(node.id.clone(), id_source.clone());

        if let WidgetKind::ExtensionPoint(extension_point) = &node.kind {
            if let Some(existing) = self.extension_points.get(&extension_point.extension_point) {
                return Err(UiRegistryError::DuplicateExtensionPointId {
                    extension_point_id: extension_point.extension_point.to_string().into(),
                    existing_mod: existing.source.mod_id.clone().into(),
                    existing_file: existing.source.file.clone().into(),
                    duplicate_mod: source.mod_id.clone().into(),
                    duplicate_file: source.file.clone().into(),
                });
            }

            self.extension_points.insert(
                extension_point.extension_point.clone(),
                ExtensionPointLocation {
                    menu_id: menu_id.clone(),
                    source: id_source.clone(),
                },
            );
        }

        for child in &node.children {
            self.register_node(menu_id, child, source)?;
        }

        Ok(())
    }
}

fn find_extension_point_mut<'a>(
    node: &'a mut WidgetNode,
    target: &crate::types::UiExtensionPointId,
) -> Option<&'a mut WidgetNode> {
    if let WidgetKind::ExtensionPoint(extension_point) = &node.kind
        && &extension_point.extension_point == target
    {
        return Some(node);
    }

    for child in &mut node.children {
        if let Some(found) = find_extension_point_mut(child, target) {
            return Some(found);
        }
    }

    None
}

impl Default for UiRegistry {
    fn default() -> Self {
        Self::new()
    }
}
