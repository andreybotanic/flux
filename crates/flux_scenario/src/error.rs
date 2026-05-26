use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScenarioLoadError {
    #[error(
        "ScenarioLoadError:\n  action: load_scenarios\n  mod: {mod_id}\n  reason: mod is present in resolved order but missing from discovered set"
    )]
    ResolvedModMissing { mod_id: Box<str> },

    #[error(
        "ScenarioLoadError:\n  action: discover_scenarios\n  mod: {mod_id}\n  path: {path}\n  reason: failed to inspect directory ({reason})"
    )]
    DirectoryRead {
        mod_id: Box<str>,
        path: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "ScenarioLoadError:\n  action: read_scenario_file\n  mod: {mod_id}\n  file: {file}\n  reason: {reason}"
    )]
    FileRead {
        mod_id: Box<str>,
        file: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "ScenarioLoadError:\n  action: parse_scenario_file\n  mod: {mod_id}\n  file: {file}\n  reason: {reason}"
    )]
    FileParse {
        mod_id: Box<str>,
        file: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "ScenarioLoadError:\n  action: validate_scenario\n  mod: {mod_id}\n  file: {file}\n  scenario_id: {scenario_id}\n  field: {field}\n  reason: {reason}"
    )]
    InvalidScenarioField {
        mod_id: Box<str>,
        file: Box<str>,
        scenario_id: Box<str>,
        field: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "ScenarioLoadError:\n  action: register_scenario\n  scenario_id: {scenario_id}\n  reason: duplicate scenario id\n  existing: mod={existing_mod}, file={existing_file}\n  duplicate: mod={duplicate_mod}, file={duplicate_file}"
    )]
    DuplicateScenarioId {
        scenario_id: Box<str>,
        existing_mod: Box<str>,
        existing_file: Box<str>,
        duplicate_mod: Box<str>,
        duplicate_file: Box<str>,
    },
}
