use std::collections::{BTreeMap, BTreeSet};

use crate::types::{DiscoveredMod, ModLoaderError};

pub(crate) fn filter_mods_with_missing_dependencies(
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

pub(crate) fn validate_dependency_versions(valid_mods: &[DiscoveredMod]) -> Vec<ModLoaderError> {
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
