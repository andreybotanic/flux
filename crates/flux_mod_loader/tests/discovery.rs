use std::fs;
use std::path::Path;

use flux_core::ModId;
use flux_mod_loader::{ModLoaderError, discover_and_resolve_mods};
use tempfile::TempDir;

#[test]
fn discovers_valid_mod_and_resolves_empty_deps() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base")).expect("create base");
    write_manifest(
        &mods_root.join("base/manifest.toml"),
        r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
    );

    let report = discover_and_resolve_mods(&mods_root);
    assert!(report.errors.is_empty());
    assert_eq!(report.valid_mods.len(), 1);
    assert_eq!(report.valid_mods[0].manifest.mod_id.as_str(), "base");
    assert_eq!(
        report
            .resolved_order
            .expect("must resolve")
            .ordered_mod_ids
            .iter()
            .map(ModId::as_str)
            .collect::<Vec<_>>(),
        vec!["base"]
    );
}

#[test]
fn reports_invalid_toml_with_file_path() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("bad_mod")).expect("create bad_mod");
    write_manifest(
        &mods_root.join("bad_mod/manifest.toml"),
        "[mod\nid = \"bad_mod\"",
    );

    let report = discover_and_resolve_mods(&mods_root);
    assert!(report.valid_mods.is_empty());
    assert!(matches!(
        report.errors[0],
        ModLoaderError::ManifestToml {
            ref mod_hint,
            ref file,
            ..
        } if mod_hint == "bad_mod" && file.ends_with("manifest.toml")
    ));
}

#[test]
fn reports_field_errors_for_invalid_id_and_versions() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("good_dir")).expect("create good_dir");
    write_manifest(
        &mods_root.join("good_dir/manifest.toml"),
        r#"
[mod]
id = "Bad-Id"
version = "1.0"
api_version = "x.y.z"
"#,
    );

    let report = discover_and_resolve_mods(&mods_root);
    assert!(report.valid_mods.is_empty());
    assert_eq!(report.errors.len(), 3);
    assert!(report.errors.iter().any(|error| {
        matches!(
            error,
            ModLoaderError::ManifestField {
                field,
                ..
            } if field == "mod.id"
        )
    }));
    assert!(report.errors.iter().any(|error| {
        matches!(
            error,
            ModLoaderError::ManifestField {
                field,
                ..
            } if field == "mod.version"
        )
    }));
    assert!(report.errors.iter().any(|error| {
        matches!(
            error,
            ModLoaderError::ManifestField {
                field,
                ..
            } if field == "mod.api_version"
        )
    }));
}

#[test]
fn reports_invalid_dependency_key_and_constraint() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("test_mod")).expect("create test_mod");
    write_manifest(
        &mods_root.join("test_mod/manifest.toml"),
        r#"
[mod]
id = "test_mod"
version = "1.2.3"
api_version = "0.1.0"

[dependencies]
"Bad-Dep" = ">=1.0"
base = "base >> 1.0"
"#,
    );

    let report = discover_and_resolve_mods(&mods_root);
    assert!(report.valid_mods.is_empty());
    assert_eq!(report.errors.len(), 2);
    assert!(report.errors.iter().any(|error| {
        matches!(error, ModLoaderError::ManifestField { field, .. } if field == "dependencies.Bad-Dep")
    }));
    assert!(report.errors.iter().any(|error| {
        matches!(error, ModLoaderError::ManifestField { field, .. } if field == "dependencies.base")
    }));
}

#[test]
fn rejects_directory_id_mismatch() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("dir_mod")).expect("create dir_mod");
    write_manifest(
        &mods_root.join("dir_mod/manifest.toml"),
        r#"
[mod]
id = "other_mod"
version = "1.0.0"
api_version = "0.1.0"
"#,
    );

    let report = discover_and_resolve_mods(&mods_root);
    assert!(report.valid_mods.is_empty());
    assert!(report.errors.iter().any(|error| {
        matches!(error, ModLoaderError::ManifestField { field, reason, .. } if field == "mod.id" && reason.contains("must match directory name"))
    }));
}

#[test]
fn reports_missing_dependency_marks_mod_as_invalid() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base")).expect("create base");
    fs::create_dir_all(mods_root.join("consumer")).expect("create consumer");

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
        &mods_root.join("consumer/manifest.toml"),
        r#"
[mod]
id = "consumer"
version = "0.1.0"
api_version = "0.1.0"

[dependencies]
base = ">=2.0"
missing_mod = "*"
"#,
    );

    let report = discover_and_resolve_mods(&mods_root);
    assert_eq!(report.valid_mods.len(), 1);
    assert!(report.resolved_order.is_none());
    assert!(report.errors.iter().any(|error| {
        matches!(
            error,
            ModLoaderError::MissingDependency {
                mod_id,
                dependency,
            } if mod_id == "consumer" && dependency == "missing_mod"
        )
    }));
    assert!(
        !report
            .valid_mods
            .iter()
            .any(|module| { module.manifest.mod_id.as_str() == "consumer" })
    );
}

#[test]
fn reports_version_mismatch_when_dependency_exists() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("base")).expect("create base");
    fs::create_dir_all(mods_root.join("consumer")).expect("create consumer");

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
        &mods_root.join("consumer/manifest.toml"),
        r#"
[mod]
id = "consumer"
version = "0.1.0"
api_version = "0.1.0"

[dependencies]
base = ">=2.0"
"#,
    );

    let report = discover_and_resolve_mods(&mods_root);
    assert_eq!(report.valid_mods.len(), 2);
    assert!(report.resolved_order.is_none());
    assert!(report.errors.iter().any(|error| {
        matches!(
            error,
            ModLoaderError::DependencyVersionMismatch {
                mod_id,
                dependency,
                ..
            } if mod_id == "consumer" && dependency == "base"
        )
    }));
}

#[test]
fn reports_cycle() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("a")).expect("create a");
    fs::create_dir_all(mods_root.join("b")).expect("create b");

    write_manifest(
        &mods_root.join("a/manifest.toml"),
        r#"
[mod]
id = "a"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
b = "*"
"#,
    );
    write_manifest(
        &mods_root.join("b/manifest.toml"),
        r#"
[mod]
id = "b"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
a = "*"
"#,
    );

    let report = discover_and_resolve_mods(&mods_root);
    assert!(report.resolved_order.is_none());
    assert!(
        report
            .errors
            .iter()
            .any(|error| { matches!(error, ModLoaderError::DependencyCycle { .. }) })
    );
}

#[test]
fn cycle_error_does_not_include_non_cycle_dependents() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");
    fs::create_dir_all(mods_root.join("a")).expect("create a");
    fs::create_dir_all(mods_root.join("b")).expect("create b");
    fs::create_dir_all(mods_root.join("c")).expect("create c");

    write_manifest(
        &mods_root.join("a/manifest.toml"),
        r#"
[mod]
id = "a"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
b = "*"
"#,
    );
    write_manifest(
        &mods_root.join("b/manifest.toml"),
        r#"
[mod]
id = "b"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
a = "*"
"#,
    );
    write_manifest(
        &mods_root.join("c/manifest.toml"),
        r#"
[mod]
id = "c"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
a = "*"
"#,
    );

    let report = discover_and_resolve_mods(&mods_root);
    let cycle = report
        .errors
        .iter()
        .find_map(|error| match error {
            ModLoaderError::DependencyCycle { cycle } => Some(cycle.as_str()),
            _ => None,
        })
        .expect("cycle error must be present");

    assert_eq!(cycle, "a -> b -> a");
}

#[test]
fn load_order_is_deterministic_with_tie_break_by_mod_id() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mods_root = temp_dir.path().join("mods");

    fs::create_dir_all(mods_root.join("base")).expect("create base");
    fs::create_dir_all(mods_root.join("alpha")).expect("create alpha");
    fs::create_dir_all(mods_root.join("zeta")).expect("create zeta");

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

    let report = discover_and_resolve_mods(&mods_root);
    let resolved = report.resolved_order.expect("resolved");
    let order: Vec<&str> = resolved.ordered_mod_ids.iter().map(ModId::as_str).collect();
    assert_eq!(order, vec!["base", "alpha", "zeta"]);
}

#[test]
fn missing_mods_directory_is_successful() {
    let temp_dir = TempDir::new().expect("tempdir");
    let missing = temp_dir.path().join("mods");

    let report = discover_and_resolve_mods(&missing);
    assert!(report.errors.is_empty());
    assert!(report.valid_mods.is_empty());
    assert!(report.resolved_order.is_some());
}

fn write_manifest(path: &Path, manifest: &str) {
    fs::write(path, manifest.trim()).expect("write manifest");
}
