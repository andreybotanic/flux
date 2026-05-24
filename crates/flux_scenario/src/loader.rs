use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use flux_mod_loader::{DiscoveredMod, ResolvedModOrder};
use ron::{Options, extensions::Extensions};
use serde::Deserialize;

use crate::{LoadedScenario, ScenarioDefinition, ScenarioLoadError, ScenarioSource, ScenarioStep};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioLoadReport {
    pub scenarios: Vec<LoadedScenario>,
    pub errors: Vec<ScenarioLoadError>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
enum ParsedScenarioFile {
    Scenario {
        id: flux_core::PrototypeId,
        steps: Vec<ScenarioStep>,
    },
}

pub fn load_scenarios(
    valid_mods: &[DiscoveredMod],
    resolved_order: &ResolvedModOrder,
) -> ScenarioLoadReport {
    let mut errors = Vec::new();
    let mut scenarios = Vec::new();
    let mut seen_ids: BTreeMap<flux_core::PrototypeId, ScenarioSource> = BTreeMap::new();

    let mods_by_id: BTreeMap<&str, &DiscoveredMod> = valid_mods
        .iter()
        .map(|module| (module.manifest.mod_id.as_str(), module))
        .collect();

    for mod_id in &resolved_order.ordered_mod_ids {
        let module = match mods_by_id.get(mod_id.as_str()) {
            Some(module) => *module,
            None => {
                errors.push(ScenarioLoadError::ResolvedModMissing {
                    mod_id: mod_id.to_string().into(),
                });
                continue;
            }
        };

        let directory = module.directory_path.join("scenarios");
        for file in collect_ron_files(module, &directory, &mut errors) {
            match parse_scenario_file(module, &file) {
                Ok(loaded) => {
                    if let Some(existing_source) = seen_ids.get(&loaded.definition.id) {
                        errors.push(ScenarioLoadError::DuplicateScenarioId {
                            scenario_id: loaded.definition.id.to_string().into(),
                            existing_mod: existing_source.mod_id.clone().into(),
                            existing_file: existing_source.file.clone().into(),
                            duplicate_mod: loaded.source.mod_id.clone().into(),
                            duplicate_file: loaded.source.file.clone().into(),
                        });
                        continue;
                    }

                    seen_ids.insert(loaded.definition.id.clone(), loaded.source.clone());
                    scenarios.push(loaded);
                }
                Err(error) => errors.push(error),
            }
        }
    }

    ScenarioLoadReport { scenarios, errors }
}

fn parse_scenario_file(
    module: &DiscoveredMod,
    file: &Path,
) -> Result<LoadedScenario, ScenarioLoadError> {
    let source = ScenarioSource {
        mod_id: module.manifest.mod_id.to_string(),
        file: file.to_string_lossy().to_string(),
    };
    let body = fs::read_to_string(file).map_err(|error| ScenarioLoadError::FileRead {
        mod_id: source.mod_id.clone().into(),
        file: source.file.clone().into(),
        reason: error.to_string().into(),
    })?;

    let ron_options =
        Options::default().with_default_extension(Extensions::UNWRAP_VARIANT_NEWTYPES);
    let parsed: ParsedScenarioFile =
        ron_options
            .from_str(&body)
            .map_err(|error| ScenarioLoadError::FileParse {
                mod_id: source.mod_id.clone().into(),
                file: source.file.clone().into(),
                reason: error.to_string().into(),
            })?;

    let definition = match parsed {
        ParsedScenarioFile::Scenario { id, steps } => ScenarioDefinition { id, steps },
    };

    validate_scenario_id_namespace(module, &source, &definition)?;

    Ok(LoadedScenario { definition, source })
}

fn validate_scenario_id_namespace(
    module: &DiscoveredMod,
    source: &ScenarioSource,
    scenario: &ScenarioDefinition,
) -> Result<(), ScenarioLoadError> {
    if scenario.id.namespace() == module.manifest.mod_id.as_str() {
        return Ok(());
    }

    Err(ScenarioLoadError::InvalidScenarioField {
        mod_id: source.mod_id.clone().into(),
        file: source.file.clone().into(),
        scenario_id: scenario.id.to_string().into(),
        field: "id".into(),
        reason: format!(
            "scenario namespace `{}` must match mod id `{}`",
            scenario.id.namespace(),
            module.manifest.mod_id
        )
        .into(),
    })
}

fn collect_ron_files(
    module: &DiscoveredMod,
    directory: &Path,
    errors: &mut Vec<ScenarioLoadError>,
) -> Vec<PathBuf> {
    if !directory.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();
    if let Err(error) =
        collect_ron_files_recursive(module.manifest.mod_id.as_str(), directory, &mut files)
    {
        errors.push(error);
        return Vec::new();
    }

    files.sort_by(|left, right| {
        normalized_relative_path(directory, left).cmp(&normalized_relative_path(directory, right))
    });
    files
}

fn collect_ron_files_recursive(
    mod_id: &str,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), ScenarioLoadError> {
    let read_dir = fs::read_dir(directory).map_err(|error| ScenarioLoadError::DirectoryRead {
        mod_id: mod_id.to_owned().into(),
        path: directory.to_string_lossy().to_string().into(),
        reason: error.to_string().into(),
    })?;

    let mut entries = BTreeSet::new();
    for entry in read_dir {
        let entry = entry.map_err(|error| ScenarioLoadError::DirectoryRead {
            mod_id: mod_id.to_owned().into(),
            path: directory.to_string_lossy().to_string().into(),
            reason: error.to_string().into(),
        })?;
        entries.insert(entry.path());
    }

    for entry_path in entries {
        if entry_path.is_dir() {
            collect_ron_files_recursive(mod_id, &entry_path, files)?;
            continue;
        }

        if is_ron_file(&entry_path) {
            files.push(entry_path);
        }
    }

    Ok(())
}

fn normalized_relative_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

fn is_ron_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ron"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use flux_mod_loader::{ModLoaderError, discover_and_resolve_mods};
    use tempfile::TempDir;

    use crate::{ScenarioLoadError, load_scenarios};

    #[test]
    fn parses_valid_scenario() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");

        create_mod(
            &mods_root,
            "test_scenarios",
            None,
            &[(
                "scenarios/bootstrap_smoke.ron",
                r#"
Scenario(
    id: "test_scenarios:scenario/bootstrap_smoke",
    steps: [Log("started"), CreateWorld(width: 64, height: 64, seed: 0), WaitTicks(5), AssertTick(5)],
)
"#,
            )],
        );

        let report = discover_and_resolve_mods(&mods_root);
        assert!(report.errors.is_empty());
        let resolved_order = report.resolved_order.expect("resolved order");
        let load_report = load_scenarios(&report.valid_mods, &resolved_order);

        assert!(load_report.errors.is_empty());
        assert_eq!(load_report.scenarios.len(), 1);
        assert_eq!(
            load_report.scenarios[0].definition.id.as_str(),
            "test_scenarios:scenario/bootstrap_smoke"
        );
    }

    #[test]
    fn reports_namespace_mismatch() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");

        create_mod(
            &mods_root,
            "test_scenarios",
            None,
            &[(
                "scenarios/bootstrap_smoke.ron",
                r#"
Scenario(
    id: "other_mod:scenario/bootstrap_smoke",
    steps: [WaitTicks(1)],
)
"#,
            )],
        );

        let report = discover_and_resolve_mods(&mods_root);
        assert!(report.errors.is_empty());
        let resolved_order = report.resolved_order.expect("resolved order");
        let load_report = load_scenarios(&report.valid_mods, &resolved_order);

        assert_eq!(load_report.scenarios.len(), 0);
        assert_eq!(load_report.errors.len(), 1);
        assert!(matches!(
            &load_report.errors[0],
            ScenarioLoadError::InvalidScenarioField { field, .. } if field.as_ref() == "id"
        ));
    }

    #[test]
    fn reports_duplicate_scenario_id() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");

        create_mod(
            &mods_root,
            "test_scenarios",
            None,
            &[
                (
                    "scenarios/a.ron",
                    r#"
Scenario(
    id: "test_scenarios:scenario/bootstrap_smoke",
    steps: [WaitTicks(1)],
)
"#,
                ),
                (
                    "scenarios/b.ron",
                    r#"
Scenario(
    id: "test_scenarios:scenario/bootstrap_smoke",
    steps: [WaitTicks(2)],
)
"#,
                ),
            ],
        );

        let report = discover_and_resolve_mods(&mods_root);
        assert!(report.errors.is_empty());
        let resolved_order = report.resolved_order.expect("resolved order");
        let load_report = load_scenarios(&report.valid_mods, &resolved_order);

        assert_eq!(load_report.scenarios.len(), 1);
        assert_eq!(load_report.errors.len(), 1);
        assert!(matches!(
            &load_report.errors[0],
            ScenarioLoadError::DuplicateScenarioId { .. }
        ));
    }

    #[test]
    fn scenario_loading_is_deterministic_by_resolved_order_and_path() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");

        create_mod(
            &mods_root,
            "base",
            None,
            &[(
                "scenarios/z_last.ron",
                r#"
Scenario(
    id: "base:scenario/z_last",
    steps: [WaitTicks(1)],
)
"#,
            )],
        );
        create_mod(
            &mods_root,
            "test_scenarios",
            Some("base"),
            &[(
                "scenarios/a_first.ron",
                r#"
Scenario(
    id: "test_scenarios:scenario/a_first",
    steps: [WaitTicks(1)],
)
"#,
            )],
        );

        let report = discover_and_resolve_mods(&mods_root);
        assert!(report.errors.is_empty());
        let resolved_order = report.resolved_order.expect("resolved order");
        let load_report = load_scenarios(&report.valid_mods, &resolved_order);

        assert!(load_report.errors.is_empty());
        let ids = load_report
            .scenarios
            .iter()
            .map(|scenario| scenario.definition.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec!["base:scenario/z_last", "test_scenarios:scenario/a_first"]
        );
    }

    #[test]
    fn reports_parse_error() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");

        create_mod(
            &mods_root,
            "test_scenarios",
            None,
            &[(
                "scenarios/invalid.ron",
                "Scenario(id: \"test_scenarios:scenario/x\",",
            )],
        );

        let report = discover_and_resolve_mods(&mods_root);
        assert!(report.errors.is_empty());
        let resolved_order = report.resolved_order.expect("resolved order");
        let load_report = load_scenarios(&report.valid_mods, &resolved_order);

        assert_eq!(load_report.scenarios.len(), 0);
        assert_eq!(load_report.errors.len(), 1);
        assert!(matches!(
            &load_report.errors[0],
            ScenarioLoadError::FileParse { .. }
        ));
    }

    fn create_mod(
        mods_root: &Path,
        mod_id: &str,
        dependency: Option<&str>,
        files: &[(&str, &str)],
    ) {
        let mod_dir = mods_root.join(mod_id);
        fs::create_dir_all(&mod_dir).expect("create mod dir");
        fs::write(mod_dir.join("manifest.toml"), manifest(mod_id, dependency)).expect("manifest");

        for (relative_path, source) in files {
            let path = mod_dir.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create parent");
            }
            fs::write(path, source.trim()).expect("write scenario");
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

    #[test]
    fn discovery_still_validates_dependencies() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        create_mod(&mods_root, "consumer", Some("missing"), &[]);

        let report = discover_and_resolve_mods(&mods_root);
        assert!(report.errors.iter().any(|error| {
            matches!(
                error,
                ModLoaderError::MissingDependency {
                    mod_id,
                    dependency
                } if mod_id == "consumer" && dependency == "missing"
            )
        }));
    }
}
