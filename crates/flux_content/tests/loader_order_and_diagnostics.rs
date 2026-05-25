use std::fs;
use std::path::Path;

use flux_content::{ContentRegistryError, load_content_registry};
use flux_mod_loader::discover_and_resolve_mods;
use tempfile::TempDir;

#[test]
fn mod_order_is_deterministic_from_resolved_order() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base/content/substances")).expect("create dir");
    fs::create_dir_all(mods_root.join("zeta/content/substances")).expect("create dir");
    fs::create_dir_all(mods_root.join("alpha/content/substances")).expect("create dir");

    write_manifest(
        &mods_root.join("base/manifest.toml"),
        r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
    );
    write_manifest(
        &mods_root.join("zeta/manifest.toml"),
        r#"
[mod]
id = "zeta"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
    );
    write_manifest(
        &mods_root.join("alpha/manifest.toml"),
        r#"
[mod]
id = "alpha"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
    );

    fs::write(
        mods_root.join("base/content/substances/base.ron"),
        r#"SubstancePrototype(id: "base:material/base", display_name: "base.substance.base")"#,
    )
    .expect("write file");
    fs::write(
        mods_root.join("zeta/content/substances/broken.ron"),
        r#"SubstancePrototype(id: "zeta:material/zeta", display_name: )"#,
    )
    .expect("write file");
    fs::write(
        mods_root.join("alpha/content/substances/broken.ron"),
        r#"SubstancePrototype(id: "alpha:material/alpha", display_name: )"#,
    )
    .expect("write file");

    let report = discover_and_resolve_mods(&mods_root);
    let resolved = report.resolved_order.expect("resolved order");
    let load_report = load_content_registry(&report.valid_mods, &resolved);
    assert!(load_report.registry.is_none());

    let ordered_error_mods: Vec<&str> = load_report
        .errors
        .iter()
        .filter_map(|error| match error {
            ContentRegistryError::FileParse { mod_id, .. } => Some(mod_id.as_ref()),
            _ => None,
        })
        .collect();
    assert_eq!(ordered_error_mods, vec!["alpha", "zeta"]);
}

#[test]
fn reports_invalid_patch_field_with_patch_source_context() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
    fs::create_dir_all(mods_root.join("test_content_mod/content/patches")).expect("create dir");

    write_manifest(
        &mods_root.join("base/manifest.toml"),
        r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
    );
    write_manifest(
        &mods_root.join("test_content_mod/manifest.toml"),
        r#"
[mod]
id = "test_content_mod"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
    );
    write_test_asset(&mods_root, "base", "textures/structure/test.png");
    fs::write(
        mods_root.join("base/content/structures/ladder.ron"),
        r#"StructurePrototype(id: "base:building/ladder", display_name: "base.structure.ladder", size: (width: 1, height: 1), visual: VisualDefinition(kind: SingleSprite(image: "textures/structure/test.png")))"#,
    )
    .expect("write structure");
    fs::write(
        mods_root.join("test_content_mod/content/patches/invalid_size.ron"),
        r#"PrototypePatch(target: "base:building/ladder", body: Structure(size: (width: 0, height: 1)))"#,
    )
    .expect("write patch");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

    assert!(load_report.registry.is_none());
    assert!(load_report.errors.iter().any(|error| {
        matches!(
            error,
            ContentRegistryError::InvalidPrototypeField {
                mod_id,
                file,
                field,
                prototype_id,
                ..
            } if mod_id.as_ref() == "test_content_mod"
                && file.ends_with("invalid_size.ron")
                && field.as_ref() == "size.width"
                && prototype_id.as_ref() == "base:building/ladder"
        )
    }));
}

fn write_manifest(path: &Path, manifest: &str) {
    fs::create_dir_all(path.parent().expect("parent dir")).expect("create parent");
    fs::write(path, manifest.trim()).expect("write manifest");
}

fn write_test_asset(mods_root: &Path, mod_id: &str, relative_path: &str) {
    let full_path = mods_root.join(mod_id).join("assets").join(relative_path);
    fs::create_dir_all(full_path.parent().expect("asset parent")).expect("create asset parent");
    fs::write(full_path, [0u8]).expect("write asset");
}
