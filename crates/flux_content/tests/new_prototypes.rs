use std::fs;
use std::path::Path;

use flux_content::{ContentRegistryError, load_content_registry};
use flux_mod_loader::discover_and_resolve_mods;
use tempfile::TempDir;

#[test]
fn reports_gas_parse_error_with_context() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base/content/gases")).expect("create dir");
    write_manifest(
        &mods_root.join("base/manifest.toml"),
        r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
    );
    fs::write(
        mods_root.join("base/content/gases/broken.ron"),
        "(id: \"base:gas/oxygen\", display_name: )",
    )
    .expect("write file");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

    assert!(load_report.registry.is_none());
    assert!(load_report.errors.iter().any(|error| {
        matches!(
            error,
            ContentRegistryError::FileParse { mod_id, file, prototype_kind, .. }
                if mod_id.as_ref() == "base"
                    && file.ends_with("broken.ron")
                    && prototype_kind.as_ref() == "gas"
        )
    }));
}

#[test]
fn reports_solid_cell_namespace_error_with_context() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base/content/solid_cells")).expect("create dir");
    write_manifest(
        &mods_root.join("base/manifest.toml"),
        r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
    );
    fs::write(
        mods_root.join("base/content/solid_cells/floor_cell.ron"),
        "(id: \"other:solid_cell/floor_cell\", display_name: \"base.solid_cell.floor_cell\", gas_permeable: false)",
    )
    .expect("write file");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

    assert!(load_report.registry.is_none());
    assert!(load_report.errors.iter().any(|error| {
        matches!(
            error,
            ContentRegistryError::InvalidPrototypeField { mod_id, file, field, .. }
                if mod_id.as_ref() == "base"
                    && file.ends_with("floor_cell.ron")
                    && field.as_ref() == "id"
        )
    }));
}

#[test]
fn accepts_valid_gas_molar_mass() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base/content/gases")).expect("create dir");
    write_manifest(
        &mods_root.join("base/manifest.toml"),
        r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
    );
    fs::write(
        mods_root.join("base/content/gases/oxygen.ron"),
        "(id: \"base:gas/oxygen\", display_name: \"base.gas.oxygen\", molar_mass: 31.998)",
    )
    .expect("write file");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));
    let registry = load_report.registry.expect("registry");

    assert!(load_report.errors.is_empty());
    assert_eq!(registry.gases_len(), 1);
}

#[test]
fn reports_invalid_gas_molar_mass_with_context() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base/content/gases")).expect("create dir");
    write_manifest(
        &mods_root.join("base/manifest.toml"),
        r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
    );
    fs::write(
        mods_root.join("base/content/gases/invalid.ron"),
        "(id: \"base:gas/oxygen\", display_name: \"base.gas.oxygen\", molar_mass: 0.0)",
    )
    .expect("write file");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

    assert!(load_report.registry.is_none());
    assert!(load_report.errors.iter().any(|error| {
        matches!(
            error,
            ContentRegistryError::InvalidPrototypeField { mod_id, file, field, prototype_id, .. }
                if mod_id.as_ref() == "base"
                    && file.ends_with("invalid.ron")
                    && field.as_ref() == "molar_mass"
                    && prototype_id.as_ref() == "base:gas/oxygen"
        )
    }));
}

fn write_manifest(path: &Path, manifest: &str) {
    fs::create_dir_all(path.parent().expect("parent dir")).expect("create parent");
    fs::write(path, manifest.trim()).expect("write manifest");
}
