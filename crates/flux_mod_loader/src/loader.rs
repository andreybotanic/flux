use std::fs;
use std::path::Path;

use crate::dependency::{filter_mods_with_missing_dependencies, validate_dependency_versions};
use crate::manifest::{RawManifest, validate_manifest};
use crate::order::resolve_load_order;
use crate::types::{DiscoveredMod, ModDiscoveryReport, ModLoaderError, ResolvedModOrder};
use crate::utils::{mod_directory_name, path_for_error};

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
