use std::collections::BTreeMap;
use std::path::Path;

use flux_core::PrototypeId;
use flux_mod_loader::DiscoveredMod;
use flux_sim::BackendPolicy;

use crate::helpers::{
    find_scenario_by_id, format_error_block, print_discovered_mod, print_patch_trail,
};
use crate::scenario_runner::ScenarioRunConfig;

pub(crate) fn run_list_mods() -> i32 {
    let report = flux_mod_loader::discover_and_resolve_mods(Path::new("mods"));
    let by_mod_id: BTreeMap<&str, &DiscoveredMod> = report
        .valid_mods
        .iter()
        .map(|module| (module.manifest.mod_id.as_str(), module))
        .collect();

    println!("valid mods: {}", report.valid_mods.len());
    if !report.valid_mods.is_empty() {
        if report.errors.is_empty() {
            let order = report
                .resolved_order
                .as_ref()
                .expect("resolved order must exist when there are no errors");
            println!("resolved load order:");
            for mod_id in &order.ordered_mod_ids {
                if let Some(module) = by_mod_id.get(mod_id.as_str()) {
                    print_discovered_mod(module);
                }
            }
        } else {
            println!("valid mods (unordered due to errors):");
            for module in &report.valid_mods {
                print_discovered_mod(module);
            }
        }
    }

    if !report.errors.is_empty() {
        eprintln!("errors: {}", report.errors.len());
        for error in &report.errors {
            eprintln!("{error}");
        }
        return 1;
    }

    0
}

pub(crate) fn run_list_content() -> i32 {
    let registry = match load_content_registry_from_mods() {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    println!(
        "content summary: solid_cells={} substances={} structures={} gases={}",
        registry.solid_cells_len(),
        registry.substances_len(),
        registry.structures_len(),
        registry.gases_len()
    );

    println!("solid cells:");
    for record in registry.solid_cells() {
        println!(
            "- id={} display_name={} gas_permeable={} source_mod={} source_file={}",
            record.prototype.id,
            record.prototype.display_name,
            record.prototype.gas_permeable,
            record.source.mod_id,
            record.source.file
        );
        print_patch_trail(&registry, &record.prototype.id);
    }

    println!("substances:");
    for record in registry.substances() {
        println!(
            "- id={} display_name={} source_mod={} source_file={}",
            record.prototype.id,
            record.prototype.display_name,
            record.source.mod_id,
            record.source.file
        );
        print_patch_trail(&registry, &record.prototype.id);
    }

    println!("structures:");
    for record in registry.structures() {
        println!(
            "- id={} display_name={} size={}x{} source_mod={} source_file={}",
            record.prototype.id,
            record.prototype.display_name,
            record.prototype.size.width,
            record.prototype.size.height,
            record.source.mod_id,
            record.source.file
        );
        print_patch_trail(&registry, &record.prototype.id);
    }

    println!("gases:");
    for record in registry.gases() {
        println!(
            "- id={} display_name={} molar_mass={} source_mod={} source_file={}",
            record.prototype.id,
            record.prototype.display_name,
            record.prototype.molar_mass,
            record.source.mod_id,
            record.source.file
        );
        print_patch_trail(&registry, &record.prototype.id);
    }

    0
}

pub(crate) fn run_list_scenarios() -> i32 {
    let scenarios = match load_scenarios_from_mods() {
        Ok(scenarios) => scenarios,
        Err(exit_code) => return exit_code,
    };

    println!("scenarios: {}", scenarios.len());
    for scenario in &scenarios {
        println!(
            "- id={} steps={} source_mod={} source_file={}",
            scenario.definition.id,
            scenario.definition.steps.len(),
            scenario.source.mod_id,
            scenario.source.file
        );
    }

    0
}

pub(crate) fn run_scenario_by_id(
    scenario_id: &PrototypeId,
    visual_delay_ms: u64,
    backend_policy: BackendPolicy,
) -> i32 {
    let scenarios = match load_scenarios_from_mods() {
        Ok(scenarios) => scenarios,
        Err(exit_code) => return exit_code,
    };

    let scenario = match find_scenario_by_id(&scenarios, scenario_id) {
        Ok(scenario) => scenario,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    crate::scenario_runner::run_scenario_windowed(
        scenario,
        ScenarioRunConfig {
            visual_delay_ms,
            backend_policy,
        },
    )
}

fn load_content_registry_from_mods() -> Result<flux_content::ContentRegistry, String> {
    let report = flux_mod_loader::discover_and_resolve_mods(Path::new("mods"));
    if !report.errors.is_empty() {
        return Err(format!(
            "mod resolution failed while loading content registry:\n{}",
            format_error_block(&report.errors)
        ));
    }
    let resolved_order = report.resolved_order.as_ref().ok_or_else(|| {
        "mod resolution failed while loading content registry: resolved order is missing".to_owned()
    })?;
    let content_report = flux_content::load_content_registry(&report.valid_mods, resolved_order);
    if !content_report.errors.is_empty() {
        return Err(format!(
            "content registry load failed:\n{}",
            format_error_block(&content_report.errors)
        ));
    }
    content_report
        .registry
        .ok_or_else(|| "content registry load failed: registry is missing".to_owned())
}

fn load_scenarios_from_mods() -> Result<Vec<flux_scenario::LoadedScenario>, i32> {
    let report = flux_mod_loader::discover_and_resolve_mods(Path::new("mods"));
    if !report.errors.is_empty() {
        eprintln!("errors: {}", report.errors.len());
        for error in &report.errors {
            eprintln!("{error}");
        }
        return Err(1);
    }

    let resolved_order = report
        .resolved_order
        .as_ref()
        .expect("resolved order must exist when there are no mod errors");
    let scenario_report = flux_scenario::load_scenarios(&report.valid_mods, resolved_order);
    if !scenario_report.errors.is_empty() {
        eprintln!("errors: {}", scenario_report.errors.len());
        for error in &scenario_report.errors {
            eprintln!("{error}");
        }
        return Err(1);
    }

    Ok(scenario_report.scenarios)
}
