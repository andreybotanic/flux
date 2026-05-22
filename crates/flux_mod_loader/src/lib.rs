#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use flux_core::{ApiVersion, ModId, ModVersion};
use semver::VersionReq;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModManifest {
    pub mod_id: ModId,
    pub version: ModVersion,
    pub api_version: ApiVersion,
    pub dependencies: BTreeMap<ModId, VersionReq>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredMod {
    pub manifest: ModManifest,
    pub directory_path: PathBuf,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModOrder {
    pub ordered_mod_ids: Vec<ModId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDiscoveryReport {
    pub valid_mods: Vec<DiscoveredMod>,
    pub errors: Vec<ModLoaderError>,
    pub resolved_order: Option<ResolvedModOrder>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ModLoaderError {
    #[error("ModDiscoveryError:\n  path: {path}\n  reason: failed to inspect mods root ({reason})")]
    ModsRootRead { path: String, reason: String },

    #[error(
        "ModManifestError:\n  mod: {mod_hint}\n  file: {file}\n  field: manifest\n  reason: missing manifest.toml"
    )]
    MissingManifest { mod_hint: String, file: String },

    #[error(
        "ModManifestError:\n  mod: {mod_hint}\n  file: {file}\n  field: manifest\n  reason: {reason}"
    )]
    ReadManifest {
        mod_hint: String,
        file: String,
        reason: String,
    },

    #[error(
        "ModManifestError:\n  mod: {mod_hint}\n  file: {file}\n  field: manifest\n  reason: invalid TOML ({reason})"
    )]
    ManifestToml {
        mod_hint: String,
        file: String,
        reason: String,
    },

    #[error(
        "ModManifestError:\n  mod: {mod_hint}\n  file: {file}\n  field: {field}\n  reason: {reason}"
    )]
    ManifestField {
        mod_hint: String,
        file: String,
        field: String,
        reason: String,
    },

    #[error(
        "ModDependencyError:\n  mod: {mod_id}\n  dependency: {dependency}\n  reason: dependency references itself"
    )]
    SelfDependency { mod_id: String, dependency: String },

    #[error(
        "ModDependencyError:\n  mod: {mod_id}\n  dependency: {dependency}\n  reason: dependency not found among valid manifests"
    )]
    MissingDependency { mod_id: String, dependency: String },

    #[error(
        "ModDependencyError:\n  mod: {mod_id}\n  dependency: {dependency}\n  reason: version mismatch, required {required}, actual {actual}"
    )]
    DependencyVersionMismatch {
        mod_id: String,
        dependency: String,
        required: String,
        actual: String,
    },

    #[error("ModDependencyError:\n  reason: cycle detected ({cycle})")]
    DependencyCycle { cycle: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    #[serde(rename = "mod")]
    mod_section: RawModSection,
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModSection {
    id: String,
    version: String,
    api_version: String,
}

pub fn discover_and_resolve_mods(mods_root: &Path) -> ModDiscoveryReport {
    let mut errors = Vec::new();

    if !mods_root.exists() {
        return ModDiscoveryReport {
            valid_mods: Vec::new(),
            errors,
            resolved_order: Some(ResolvedModOrder {
                ordered_mod_ids: Vec::new(),
            }),
        };
    }

    if !mods_root.is_dir() {
        errors.push(ModLoaderError::ModsRootRead {
            path: path_for_error(mods_root),
            reason: "path exists but is not a directory".to_owned(),
        });
        return ModDiscoveryReport {
            valid_mods: Vec::new(),
            errors,
            resolved_order: None,
        };
    }

    let read_dir = match fs::read_dir(mods_root) {
        Ok(iter) => iter,
        Err(error) => {
            errors.push(ModLoaderError::ModsRootRead {
                path: path_for_error(mods_root),
                reason: error.to_string(),
            });
            return ModDiscoveryReport {
                valid_mods: Vec::new(),
                errors,
                resolved_order: None,
            };
        }
    };

    let mut mod_directories = Vec::new();
    for entry_result in read_dir {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(ModLoaderError::ModsRootRead {
                    path: path_for_error(mods_root),
                    reason: error.to_string(),
                });
                continue;
            }
        };

        let path = entry.path();
        if path.is_dir() {
            mod_directories.push(path);
        }
    }

    mod_directories.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });

    let mut valid_mods = Vec::new();
    for mod_directory in mod_directories {
        let manifest_path = mod_directory.join("manifest.toml");
        if !manifest_path.is_file() {
            errors.push(ModLoaderError::MissingManifest {
                mod_hint: mod_directory_name(&mod_directory),
                file: path_for_error(&manifest_path),
            });
            continue;
        }

        let manifest_source = match fs::read_to_string(&manifest_path) {
            Ok(source) => source,
            Err(error) => {
                errors.push(ModLoaderError::ReadManifest {
                    mod_hint: mod_directory_name(&mod_directory),
                    file: path_for_error(&manifest_path),
                    reason: error.to_string(),
                });
                continue;
            }
        };

        let raw_manifest: RawManifest = match toml::from_str(&manifest_source) {
            Ok(raw) => raw,
            Err(error) => {
                errors.push(ModLoaderError::ManifestToml {
                    mod_hint: mod_directory_name(&mod_directory),
                    file: path_for_error(&manifest_path),
                    reason: error.to_string(),
                });
                continue;
            }
        };

        match validate_manifest(&raw_manifest, &mod_directory, &manifest_path) {
            Ok(manifest) => {
                valid_mods.push(DiscoveredMod {
                    manifest,
                    directory_path: mod_directory,
                    manifest_path,
                });
            }
            Err(mut manifest_errors) => {
                errors.append(&mut manifest_errors);
            }
        }
    }

    valid_mods.sort_by(|left, right| {
        left.manifest
            .mod_id
            .as_str()
            .cmp(right.manifest.mod_id.as_str())
    });

    let (valid_mods, missing_dependency_errors) = filter_mods_with_missing_dependencies(valid_mods);
    let dependency_errors_found = !missing_dependency_errors.is_empty();
    errors.extend(missing_dependency_errors);

    let version_errors = validate_dependency_versions(&valid_mods);
    let version_errors_found = !version_errors.is_empty();
    errors.extend(version_errors);

    let resolved_order = if dependency_errors_found || version_errors_found {
        None
    } else {
        match resolve_load_order(&valid_mods) {
            Ok(order) => Some(order),
            Err(cycle_error) => {
                errors.push(cycle_error);
                None
            }
        }
    };

    ModDiscoveryReport {
        valid_mods,
        errors,
        resolved_order,
    }
}

fn validate_manifest(
    raw: &RawManifest,
    mod_directory: &Path,
    manifest_path: &Path,
) -> Result<ModManifest, Vec<ModLoaderError>> {
    let mut errors = Vec::new();
    let mod_hint = mod_directory_name(mod_directory);
    let file = path_for_error(manifest_path);

    let mod_id = match ModId::parse(&raw.mod_section.id) {
        Ok(mod_id) => Some(mod_id),
        Err(error) => {
            errors.push(ModLoaderError::ManifestField {
                mod_hint: mod_hint.clone(),
                file: file.clone(),
                field: "mod.id".to_owned(),
                reason: error.to_string(),
            });
            None
        }
    };

    let version = match ModVersion::parse(&raw.mod_section.version) {
        Ok(version) => Some(version),
        Err(error) => {
            errors.push(ModLoaderError::ManifestField {
                mod_hint: mod_hint.clone(),
                file: file.clone(),
                field: "mod.version".to_owned(),
                reason: error.to_string(),
            });
            None
        }
    };

    let api_version = match ApiVersion::parse(&raw.mod_section.api_version) {
        Ok(version) => Some(version),
        Err(error) => {
            errors.push(ModLoaderError::ManifestField {
                mod_hint: mod_hint.clone(),
                file: file.clone(),
                field: "mod.api_version".to_owned(),
                reason: error.to_string(),
            });
            None
        }
    };

    if let Some(parsed_mod_id) = mod_id.as_ref() {
        let directory_name = mod_directory_name(mod_directory);
        if parsed_mod_id.as_str() != directory_name {
            errors.push(ModLoaderError::ManifestField {
                mod_hint: mod_hint.clone(),
                file: file.clone(),
                field: "mod.id".to_owned(),
                reason: format!(
                    "manifest mod id `{}` must match directory name `{}`",
                    parsed_mod_id.as_str(),
                    directory_name
                ),
            });
        }
    }

    let mut dependencies = BTreeMap::new();
    for (dependency_key, dependency_constraint) in &raw.dependencies {
        let dependency_mod_id = match ModId::parse(dependency_key) {
            Ok(parsed) => parsed,
            Err(error) => {
                errors.push(ModLoaderError::ManifestField {
                    mod_hint: mod_hint.clone(),
                    file: file.clone(),
                    field: format!("dependencies.{dependency_key}"),
                    reason: error.to_string(),
                });
                continue;
            }
        };

        let version_req = match VersionReq::parse(dependency_constraint) {
            Ok(parsed) => parsed,
            Err(parse_error) => {
                errors.push(ModLoaderError::ManifestField {
                    mod_hint: mod_hint.clone(),
                    file: file.clone(),
                    field: format!("dependencies.{}", dependency_mod_id.as_str()),
                    reason: format!(
                        "invalid dependency constraint `{dependency_constraint}` ({parse_error})"
                    ),
                });
                continue;
            }
        };

        dependencies.insert(dependency_mod_id, version_req);
    }

    if let Some(parsed_mod_id) = mod_id.as_ref() {
        for dependency_mod_id in dependencies.keys() {
            if dependency_mod_id == parsed_mod_id {
                errors.push(ModLoaderError::SelfDependency {
                    mod_id: parsed_mod_id.as_str().to_owned(),
                    dependency: dependency_mod_id.as_str().to_owned(),
                });
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ModManifest {
        mod_id: mod_id.expect("checked above"),
        version: version.expect("checked above"),
        api_version: api_version.expect("checked above"),
        dependencies,
    })
}

fn filter_mods_with_missing_dependencies(
    mods: Vec<DiscoveredMod>,
) -> (Vec<DiscoveredMod>, Vec<ModLoaderError>) {
    let mut errors = Vec::new();
    let mut current = mods;

    loop {
        let known_mod_ids: BTreeSet<&str> = current
            .iter()
            .map(|module| module.manifest.mod_id.as_str())
            .collect();

        let mut invalid_ids = BTreeSet::new();
        for module in &current {
            for dependency_id in module.manifest.dependencies.keys() {
                if !known_mod_ids.contains(dependency_id.as_str()) {
                    errors.push(ModLoaderError::MissingDependency {
                        mod_id: module.manifest.mod_id.as_str().to_owned(),
                        dependency: dependency_id.as_str().to_owned(),
                    });
                    invalid_ids.insert(module.manifest.mod_id.clone());
                }
            }
        }

        if invalid_ids.is_empty() {
            break;
        }

        current.retain(|module| !invalid_ids.contains(&module.manifest.mod_id));
    }

    (current, errors)
}

fn validate_dependency_versions(valid_mods: &[DiscoveredMod]) -> Vec<ModLoaderError> {
    let mut errors = Vec::new();
    let mod_versions: BTreeMap<&str, &semver::Version> = valid_mods
        .iter()
        .map(|module| {
            (
                module.manifest.mod_id.as_str(),
                module.manifest.version.as_semver(),
            )
        })
        .collect();

    for module in valid_mods {
        for (dependency_id, version_req) in &module.manifest.dependencies {
            let dependency_version = mod_versions
                .get(dependency_id.as_str())
                .expect("missing dependencies are filtered before version checks");

            if !version_req.matches(dependency_version) {
                errors.push(ModLoaderError::DependencyVersionMismatch {
                    mod_id: module.manifest.mod_id.as_str().to_owned(),
                    dependency: dependency_id.as_str().to_owned(),
                    required: version_req.to_string(),
                    actual: dependency_version.to_string(),
                });
            }
        }
    }

    errors
}

fn resolve_load_order(valid_mods: &[DiscoveredMod]) -> Result<ResolvedModOrder, ModLoaderError> {
    if valid_mods.is_empty() {
        return Ok(ResolvedModOrder {
            ordered_mod_ids: Vec::new(),
        });
    }

    let mut in_degree: BTreeMap<ModId, usize> = BTreeMap::new();
    let mut edges: BTreeMap<ModId, Vec<ModId>> = BTreeMap::new();

    for module in valid_mods {
        let mod_id = module.manifest.mod_id.clone();
        in_degree.entry(mod_id.clone()).or_insert(0);
        edges.entry(mod_id.clone()).or_default();

        for dependency_id in module.manifest.dependencies.keys() {
            let dependents = edges.entry(dependency_id.clone()).or_default();
            dependents.push(mod_id.clone());
            *in_degree.entry(mod_id.clone()).or_insert(0) += 1;
        }
    }

    let mut ready = BTreeSet::new();
    for (mod_id, degree) in &in_degree {
        if *degree == 0 {
            ready.insert(mod_id.clone());
        }
    }

    let mut ordered_mod_ids = Vec::new();
    while let Some(next) = ready.iter().next().cloned() {
        ready.remove(&next);
        ordered_mod_ids.push(next.clone());

        let dependents = edges.get(&next).cloned().unwrap_or_default();
        for dependent in dependents {
            if let Some(current_degree) = in_degree.get_mut(&dependent) {
                *current_degree = current_degree.saturating_sub(1);
                if *current_degree == 0 {
                    ready.insert(dependent);
                }
            }
        }
    }

    if ordered_mod_ids.len() != valid_mods.len() {
        let cycle_nodes = in_degree
            .iter()
            .filter_map(|(mod_id, degree)| (*degree > 0).then_some(mod_id.as_str()))
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(ModLoaderError::DependencyCycle { cycle: cycle_nodes });
    }

    Ok(ResolvedModOrder { ordered_mod_ids })
}

fn mod_directory_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path_for_error(path))
}

fn path_for_error(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

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
}
