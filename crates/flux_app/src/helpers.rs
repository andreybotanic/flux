use flux_core::PrototypeId;
use flux_mod_loader::DiscoveredMod;

pub(crate) fn format_error_block<T: std::fmt::Display>(errors: &[T]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScenarioLookupError {
    NotFound { scenario_id: String },
}

pub(crate) fn find_scenario_by_id<'a>(
    scenarios: &'a [flux_scenario::LoadedScenario],
    scenario_id: &PrototypeId,
) -> Result<&'a flux_scenario::LoadedScenario, ScenarioLookupError> {
    scenarios
        .iter()
        .find(|candidate| candidate.definition.id == *scenario_id)
        .ok_or_else(|| ScenarioLookupError::NotFound {
            scenario_id: scenario_id.to_string(),
        })
}

impl std::fmt::Display for ScenarioLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioLookupError::NotFound { scenario_id } => write!(
                f,
                "ScenarioRunError:\n  action: run_scenario\n  scenario_id: {scenario_id}\n  reason: scenario not found"
            ),
        }
    }
}

pub(crate) fn print_patch_trail(
    registry: &flux_content::ContentRegistry,
    prototype_id: &flux_core::PrototypeId,
) {
    let patches = registry
        .applied_patches_for(prototype_id)
        .collect::<Vec<_>>();
    if patches.is_empty() {
        return;
    }

    println!("  patches:");
    for patch in patches {
        println!(
            "  - kind={} source_mod={} source_file={}",
            patch.patch_kind.as_str(),
            patch.source.mod_id,
            patch.source.file
        );
    }
}

pub(crate) fn print_discovered_mod(module: &DiscoveredMod) {
    println!(
        "- id={} version={} api_version={} path={}",
        module.manifest.mod_id,
        module.manifest.version,
        module.manifest.api_version,
        module.directory_path.display()
    );
}
