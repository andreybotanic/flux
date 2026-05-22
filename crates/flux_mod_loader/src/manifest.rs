use std::collections::BTreeMap;
use std::path::Path;

use flux_core::{ApiVersion, ModId, ModVersion};
use semver::VersionReq;
use serde::Deserialize;

use crate::types::{ModLoaderError, ModManifest};
use crate::utils::{mod_directory_name, path_for_error};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawManifest {
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

pub(crate) fn validate_manifest(
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
