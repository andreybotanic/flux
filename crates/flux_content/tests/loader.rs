use std::fs;
use std::path::Path;

use flux_content::{ContentRegistryError, load_content_registry};
use flux_mod_loader::discover_and_resolve_mods;
use tempfile::TempDir;

#[test]
fn loads_typed_content_and_freezes_registry() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base/content/solid_cells")).expect("create dir");
    fs::create_dir_all(mods_root.join("base/content/substances")).expect("create dir");
    fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
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
    write_test_asset(&mods_root, "base", "textures/solid/test.png");
    write_test_asset(&mods_root, "base", "textures/structure/test.png");
    fs::write(
        mods_root.join("base/content/solid_cells/floor_cell.ron"),
        r#"SolidCellPrototype(id: "base:solid_cell/floor_cell", display_name: "base.solid_cell.floor_cell", gas_permeable: false, visual: VisualDefinition(kind: SingleSprite(image: "textures/solid/test.png")))"#,
    )
    .expect("write solid cell");
    fs::write(
        mods_root.join("base/content/substances/oxygen.ron"),
        r#"SubstancePrototype(id: "base:material/oxygen", display_name: "base.substance.oxygen")"#,
    )
    .expect("write substance");
    fs::write(
        mods_root.join("base/content/structures/pump.ron"),
        r#"StructurePrototype(id: "base:building/gas_pump", display_name: "base.structure.gas_pump", size: (width: 1, height: 2), visual: VisualDefinition(kind: SingleSprite(image: "textures/structure/test.png")))"#,
    )
    .expect("write structure");
    fs::write(
        mods_root.join("base/content/gases/oxygen.ron"),
        r#"GasPrototype(id: "base:gas/oxygen", display_name: "base.gas.oxygen", molar_mass: 31.998)"#,
    )
    .expect("write gas");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report = load_content_registry(
        &report.valid_mods,
        &report.resolved_order.expect("resolved order"),
    );
    let registry = load_report.registry.expect("registry");

    assert!(load_report.errors.is_empty());
    assert!(registry.is_frozen());
    assert_eq!(registry.solid_cells_len(), 1);
    assert_eq!(registry.substances_len(), 1);
    assert_eq!(registry.structures_len(), 1);
    assert_eq!(registry.gases_len(), 1);
}

#[test]
fn rejects_non_typed_ron_body() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base/content/substances")).expect("create dir");
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
        mods_root.join("base/content/substances/broken.ron"),
        r#"(id: "base:material/oxygen", display_name: "base.substance.oxygen")"#,
    )
    .expect("write file");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

    assert!(load_report.registry.is_none());
    assert!(load_report.errors.iter().any(|error| {
        matches!(
            error,
            ContentRegistryError::FileParse {
                file,
                prototype_kind,
                ..
            } if file.ends_with("broken.ron") && prototype_kind.as_ref() == "substance"
        )
    }));
}

#[test]
fn applies_patch_and_tracks_patch_history() {
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
    write_test_asset(
        &mods_root,
        "test_content_mod",
        "textures/structure/patched.png",
    );

    fs::write(
        mods_root.join("base/content/structures/ladder.ron"),
        r#"StructurePrototype(id: "base:building/ladder", display_name: "base.structure.ladder", size: (width: 1, height: 1), visual: VisualDefinition(kind: SingleSprite(image: "textures/structure/test.png")))"#,
    )
    .expect("write structure");
    fs::write(
        mods_root.join("test_content_mod/content/patches/base_building_ladder.ron"),
        r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.ladder", size: (width: 2, height: 1), visual: Some(VisualDefinition(kind: SingleSprite(image: "textures/structure/patched.png")))))"#,
    )
    .expect("write patch");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));
    assert!(
        load_report.errors.is_empty(),
        "unexpected content errors: {:?}",
        load_report.errors
    );
    let registry = load_report.registry.expect("registry");
    let ladder = registry
        .structures()
        .find(|record| record.prototype.id.as_str() == "base:building/ladder")
        .expect("ladder");
    assert_eq!(
        ladder.prototype.display_name.as_str(),
        "test_content_mod.structure.ladder"
    );
    assert_eq!(ladder.prototype.size.width, 2);
    assert_eq!(
        ladder.prototype.visual.image_path().as_str(),
        "textures/structure/patched.png"
    );
    assert_eq!(
        registry.structure_visual_mod_id(&ladder.prototype.id),
        Some("test_content_mod")
    );

    let patch_sources: Vec<&str> = registry
        .applied_patches_for(&ladder.prototype.id)
        .map(|patch| patch.source.mod_id.as_str())
        .collect();
    assert_eq!(patch_sources, vec!["test_content_mod"]);
}

#[test]
fn rejects_duplicate_patch_target_within_mod() {
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
        mods_root.join("test_content_mod/content/patches/a.ron"),
        r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.a"))"#,
    )
    .expect("write patch");
    fs::write(
        mods_root.join("test_content_mod/content/patches/b.ron"),
        r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.b"))"#,
    )
    .expect("write patch");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

    assert!(load_report.registry.is_none());
    assert!(load_report.errors.iter().any(|error| {
        matches!(error, ContentRegistryError::DuplicatePatchTargetInMod { target, .. } if target.as_ref() == "base:building/ladder")
    }));
}

#[test]
fn later_mod_patch_overrides_earlier_mod_patch() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");

    fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
    fs::create_dir_all(mods_root.join("test_content_mod/content/patches")).expect("create dir");
    fs::create_dir_all(mods_root.join("another_test_mod/content/patches")).expect("create dir");

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
    write_manifest(
        &mods_root.join("another_test_mod/manifest.toml"),
        r#"
[mod]
id = "another_test_mod"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
test_content_mod = "*"
"#,
    );
    write_test_asset(&mods_root, "base", "textures/structure/test.png");

    fs::write(
        mods_root.join("base/content/structures/ladder.ron"),
        r#"StructurePrototype(id: "base:building/ladder", display_name: "base.structure.ladder", size: (width: 1, height: 1), visual: VisualDefinition(kind: SingleSprite(image: "textures/structure/test.png")))"#,
    )
    .expect("write structure");
    fs::write(
        mods_root.join("test_content_mod/content/patches/base_building_ladder.ron"),
        r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.ladder"))"#,
    )
    .expect("write patch");
    fs::write(
        mods_root.join("another_test_mod/content/patches/base_building_ladder.ron"),
        r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "another_test_mod.structure.ladder", size: (width: 3, height: 1)))"#,
    )
    .expect("write patch");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));
    let registry = load_report.registry.expect("registry");

    assert!(load_report.errors.is_empty());

    let ladder = registry
        .structures()
        .find(|record| record.prototype.id.as_str() == "base:building/ladder")
        .expect("ladder");
    assert_eq!(
        ladder.prototype.display_name.as_str(),
        "another_test_mod.structure.ladder"
    );
    assert_eq!(ladder.prototype.size.width, 3);
    assert_eq!(
        registry.structure_visual_mod_id(&ladder.prototype.id),
        Some("base")
    );

    let patch_sources: Vec<&str> = registry
        .applied_patches_for(&ladder.prototype.id)
        .map(|patch| patch.source.mod_id.as_str())
        .collect();
    assert_eq!(patch_sources, vec!["test_content_mod", "another_test_mod"]);
}

#[test]
fn patch_file_order_is_lexicographic_inside_mod() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");

    fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
    fs::create_dir_all(mods_root.join("test_content_mod/content/patches/a")).expect("create dir");
    fs::create_dir_all(mods_root.join("test_content_mod/content/patches/b")).expect("create dir");

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
        mods_root.join("test_content_mod/content/patches/b/002.ron"),
        r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.second"))"#,
    )
    .expect("write patch");
    fs::write(
        mods_root.join("test_content_mod/content/patches/a/001.ron"),
        r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.first"))"#,
    )
    .expect("write patch");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

    assert!(load_report.registry.is_none());
    assert!(load_report.errors.iter().any(|error| {
        matches!(error, ContentRegistryError::DuplicatePatchTargetInMod { first_file, duplicate_file, .. }
            if first_file.replace('\\', "/").ends_with("a/001.ron")
                && duplicate_file.replace('\\', "/").ends_with("b/002.ron"))
    }));
}

#[test]
fn rejects_empty_patch() {
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
        mods_root.join("test_content_mod/content/patches/empty.ron"),
        r#"PrototypePatch(target: "base:building/ladder", body: Structure())"#,
    )
    .expect("write patch");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

    assert!(load_report.registry.is_none());
    assert!(load_report.errors.iter().any(|error| {
        matches!(error, ContentRegistryError::EmptyPatchBody { target, .. } if target.as_ref() == "base:building/ladder")
    }));
}

#[test]
fn rejects_patch_target_not_found() {
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
        mods_root.join("test_content_mod/content/patches/unknown.ron"),
        r#"PrototypePatch(target: "base:building/unknown", body: Structure(display_name: "test_content_mod.structure.unknown"))"#,
    )
    .expect("write patch");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

    assert!(load_report.registry.is_none());
    assert!(load_report.errors.iter().any(|error| {
        matches!(error, ContentRegistryError::PatchTargetNotFound { target, .. } if target.as_ref() == "base:building/unknown")
    }));
}

#[test]
fn rejects_patch_kind_mismatch() {
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
        mods_root.join("test_content_mod/content/patches/wrong_kind.ron"),
        r#"PrototypePatch(target: "base:building/ladder", body: Gas(display_name: "test_content_mod.gas.fake"))"#,
    )
    .expect("write patch");

    let report = discover_and_resolve_mods(&mods_root);
    let load_report =
        load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

    assert!(load_report.registry.is_none());
    assert!(load_report.errors.iter().any(|error| {
        matches!(error, ContentRegistryError::PatchKindMismatch { target, .. } if target.as_ref() == "base:building/ladder")
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
