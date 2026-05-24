use std::fs;
use std::path::Path;

use flux_mod_loader::discover_and_resolve_mods;
use flux_ui::{UiRegistryError, WidgetKind, load_ui_registry};
use tempfile::TempDir;

#[test]
fn applies_extension_and_keeps_action_object_in_button() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");

    create_mod(
        &mods_root,
        "base",
        None,
        &[
            (
                "ui/menus/main.ron",
                r#"
UiMenu(
    id: "base:menu/main",
    root: Container(
        id: "base:widget/main/root",
        layout: Vertical,
        children: [
            ExtensionPoint(
                id: "base:widget/main/actions_slot",
                extension_point: "base:ui_ext/main/actions",
            ),
        ],
    ),
)
"#,
            ),
            (
                "ui/menus/settings.ron",
                r#"
UiMenu(
    id: "base:menu/settings",
    root: Container(
        id: "base:widget/settings/root",
        layout: Vertical,
        children: [],
    ),
)
"#,
            ),
        ],
    );

    create_mod(
        &mods_root,
        "example_ui",
        Some("base"),
        &[
            (
                "ui/menus/debug.ron",
                r#"
UiMenu(
    id: "example_ui:menu/debug",
    root: Container(
        id: "example_ui:widget/debug/root",
        layout: Vertical,
        children: [],
    ),
)
"#,
            ),
            (
                "ui/extensions/main_actions.ron",
                r#"
UiExtension(
    target: "base:ui_ext/main/actions",
    operation: AppendChild,
    child: Button(
        id: "example_ui:widget/main/debug_button",
        text: "$example_ui.debug",
        action: OpenMenu("example_ui:menu/debug"),
    ),
)
"#,
            ),
        ],
    );

    let mod_report = discover_and_resolve_mods(&mods_root);
    assert!(
        mod_report.errors.is_empty(),
        "mod errors: {:?}",
        mod_report.errors
    );
    let resolved_order = mod_report.resolved_order.expect("resolved order");

    let ui_report = load_ui_registry(&mod_report.valid_mods, &resolved_order);
    assert!(
        ui_report.errors.is_empty(),
        "ui errors: {:?}",
        ui_report.errors
    );

    let registry = ui_report.registry.expect("registry should be built");
    let main_menu_id =
        flux_ui::UiMenuId(flux_core::NamespacedId::parse("base:menu/main").expect("id"));
    let main_menu = registry.menu(&main_menu_id).expect("main menu must exist");

    let slot = find_extension_slot(&main_menu.root, "base:ui_ext/main/actions")
        .expect("extension slot should exist");
    let appended_button = slot
        .children
        .iter()
        .find(|node| node.id.0.as_str() == "example_ui:widget/main/debug_button")
        .expect("extension button should be appended");

    match &appended_button.kind {
        WidgetKind::Button(button) => {
            assert_eq!(button.text.as_str(), "$example_ui.debug");
            assert_eq!(
                button.action,
                flux_ui::UiAction::OpenMenu(flux_ui::UiMenuId(
                    flux_core::NamespacedId::parse("example_ui:menu/debug").expect("id")
                ))
            );
        }
        other => panic!("expected button, got {other:?}"),
    }
}

#[test]
fn reports_unknown_open_menu_target() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");

    create_mod(
        &mods_root,
        "base",
        None,
        &[(
            "ui/menus/main.ron",
            r#"
UiMenu(
    id: "base:menu/main",
    root: Container(
        id: "base:widget/main/root",
        layout: Vertical,
        children: [
            Button(
                id: "base:widget/main/missing",
                text: "$base.missing",
                action: OpenMenu("base:menu/does_not_exist"),
            ),
        ],
    ),
)
"#,
        )],
    );

    let mod_report = discover_and_resolve_mods(&mods_root);
    assert!(
        mod_report.errors.is_empty(),
        "mod errors: {:?}",
        mod_report.errors
    );
    let resolved_order = mod_report.resolved_order.expect("resolved order");

    let ui_report = load_ui_registry(&mod_report.valid_mods, &resolved_order);
    assert!(ui_report.registry.is_none());
    assert!(ui_report.errors.iter().any(|error| {
        matches!(
            error,
            UiRegistryError::OpenMenuTargetNotFound { target_menu, .. }
                if target_menu.as_ref() == "base:menu/does_not_exist"
        )
    }));
}

#[test]
fn reports_duplicate_widget_id() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");

    create_mod(
        &mods_root,
        "base",
        None,
        &[(
            "ui/menus/main.ron",
            r#"
UiMenu(
    id: "base:menu/main",
    root: Container(
        id: "base:widget/main/root",
        layout: Vertical,
        children: [
            Text(id: "base:widget/main/dup", text: "$base.one"),
            Text(id: "base:widget/main/dup", text: "$base.two"),
        ],
    ),
)
"#,
        )],
    );

    let mod_report = discover_and_resolve_mods(&mods_root);
    assert!(
        mod_report.errors.is_empty(),
        "mod errors: {:?}",
        mod_report.errors
    );
    let resolved_order = mod_report.resolved_order.expect("resolved order");

    let ui_report = load_ui_registry(&mod_report.valid_mods, &resolved_order);
    assert!(ui_report.registry.is_none());
    assert!(ui_report.errors.iter().any(|error| {
        matches!(
            error,
            UiRegistryError::DuplicateWidgetId { widget_id, .. }
                if widget_id.as_ref() == "base:widget/main/dup"
        )
    }));
}

#[test]
fn reports_duplicate_menu_id() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");

    create_mod(
        &mods_root,
        "base",
        None,
        &[
            (
                "ui/menus/main_a.ron",
                r#"
UiMenu(
    id: "base:menu/main",
    root: Container(
        id: "base:widget/main_a/root",
        layout: Vertical,
        children: [],
    ),
)
"#,
            ),
            (
                "ui/menus/main_b.ron",
                r#"
UiMenu(
    id: "base:menu/main",
    root: Container(
        id: "base:widget/main_b/root",
        layout: Vertical,
        children: [],
    ),
)
"#,
            ),
        ],
    );

    let mod_report = discover_and_resolve_mods(&mods_root);
    assert!(
        mod_report.errors.is_empty(),
        "mod errors: {:?}",
        mod_report.errors
    );
    let resolved_order = mod_report.resolved_order.expect("resolved order");

    let ui_report = load_ui_registry(&mod_report.valid_mods, &resolved_order);
    assert!(ui_report.registry.is_none());
    assert!(ui_report.errors.iter().any(|error| {
        matches!(
            error,
            UiRegistryError::DuplicateMenuId { menu_id, .. }
                if menu_id.as_ref() == "base:menu/main"
        )
    }));
}

fn find_extension_slot<'a>(
    root: &'a flux_ui::WidgetNode,
    extension_point_id: &str,
) -> Option<&'a flux_ui::WidgetNode> {
    if let WidgetKind::ExtensionPoint(extension_point) = &root.kind
        && extension_point.extension_point.0.as_str() == extension_point_id
    {
        return Some(root);
    }

    for child in &root.children {
        if let Some(found) = find_extension_slot(child, extension_point_id) {
            return Some(found);
        }
    }

    None
}

fn create_mod(mods_root: &Path, mod_id: &str, dependency: Option<&str>, files: &[(&str, &str)]) {
    let mod_dir = mods_root.join(mod_id);
    fs::create_dir_all(&mod_dir).expect("create mod dir");
    fs::write(mod_dir.join("manifest.toml"), manifest(mod_id, dependency)).expect("manifest");

    for (relative_path, source) in files {
        let file_path = mod_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(file_path, source.trim()).expect("write ui file");
    }
}

fn manifest(mod_id: &str, dependency: Option<&str>) -> String {
    let mut source = format!(
        r#"
[mod]
id = "{mod_id}"
version = "1.0.0"
api_version = "0.1.0"
"#
    );
    if let Some(dependency) = dependency {
        source.push_str("\n[dependencies]\n");
        source.push_str(&format!("{dependency} = \"*\"\n"));
    }
    source.trim().to_owned()
}
