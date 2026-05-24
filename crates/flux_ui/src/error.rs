use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UiRegistryError {
    #[error(
        "UiRegistryError:\n  action: load_ui\n  mod: {mod_id}\n  reason: mod is present in resolved order but missing from discovered set"
    )]
    ResolvedModMissing { mod_id: Box<str> },

    #[error(
        "UiRegistryError:\n  action: discover_ui\n  mod: {mod_id}\n  path: {path}\n  reason: failed to inspect directory ({reason})"
    )]
    DirectoryRead {
        mod_id: Box<str>,
        path: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "UiRegistryError:\n  action: read_ui_file\n  mod: {mod_id}\n  file: {file}\n  reason: {reason}"
    )]
    FileRead {
        mod_id: Box<str>,
        file: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "UiRegistryError:\n  action: parse_ui_file\n  mod: {mod_id}\n  file: {file}\n  reason: {reason}"
    )]
    FileParse {
        mod_id: Box<str>,
        file: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "UiRegistryError:\n  action: register_menu\n  menu_id: {menu_id}\n  reason: duplicate menu id\n  existing: mod={existing_mod}, file={existing_file}\n  duplicate: mod={duplicate_mod}, file={duplicate_file}"
    )]
    DuplicateMenuId {
        menu_id: Box<str>,
        existing_mod: Box<str>,
        existing_file: Box<str>,
        duplicate_mod: Box<str>,
        duplicate_file: Box<str>,
    },

    #[error(
        "UiRegistryError:\n  action: register_widget\n  widget_id: {widget_id}\n  reason: duplicate widget id\n  existing: mod={existing_mod}, file={existing_file}\n  duplicate: mod={duplicate_mod}, file={duplicate_file}"
    )]
    DuplicateWidgetId {
        widget_id: Box<str>,
        existing_mod: Box<str>,
        existing_file: Box<str>,
        duplicate_mod: Box<str>,
        duplicate_file: Box<str>,
    },

    #[error(
        "UiRegistryError:\n  action: register_extension_point\n  extension_point_id: {extension_point_id}\n  reason: duplicate extension point id\n  existing: mod={existing_mod}, file={existing_file}\n  duplicate: mod={duplicate_mod}, file={duplicate_file}"
    )]
    DuplicateExtensionPointId {
        extension_point_id: Box<str>,
        existing_mod: Box<str>,
        existing_file: Box<str>,
        duplicate_mod: Box<str>,
        duplicate_file: Box<str>,
    },

    #[error(
        "UiRegistryError:\n  action: validate_ui\n  mod: {mod_id}\n  file: {file}\n  field: {field}\n  value: {value}\n  reason: namespace `{actual_namespace}` must match mod id `{expected_mod_id}`"
    )]
    NamespaceMismatch {
        mod_id: Box<str>,
        file: Box<str>,
        field: Box<str>,
        value: Box<str>,
        actual_namespace: Box<str>,
        expected_mod_id: Box<str>,
    },

    #[error(
        "UiRegistryError:\n  action: apply_extension\n  mod: {mod_id}\n  file: {file}\n  target: {target}\n  reason: extension point not found"
    )]
    ExtensionTargetNotFound {
        mod_id: Box<str>,
        file: Box<str>,
        target: Box<str>,
    },

    #[error(
        "UiRegistryError:\n  action: validate_actions\n  menu_id: {menu_id}\n  widget_id: {widget_id}\n  action: OpenMenu\n  target_menu: {target_menu}\n  reason: target menu not found"
    )]
    OpenMenuTargetNotFound {
        menu_id: Box<str>,
        widget_id: Box<str>,
        target_menu: Box<str>,
    },
}
