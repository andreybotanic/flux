#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Once;
use std::time::Duration;

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{InstanceFlags, RenderCreation, WgpuSettings};
use bevy::window::{Window, WindowPlugin};
use flux_core::PrototypeId;
use flux_mod_loader::DiscoveredMod;
use flux_sim::SimRuntime;

#[derive(Debug, Clone, PartialEq, Eq)]
enum RunMode {
    Version,
    ListMods,
    ListContent,
    ListScenarios,
    RunScenario { scenario_id: PrototypeId },
    Windowed,
    Headless,
}

#[derive(Debug, PartialEq, Eq)]
enum CliError {
    UnknownArgument(String),
    ConflictingArguments,
    MissingArgumentValue(&'static str),
    InvalidScenarioId(String),
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match parse_run_mode(&args) {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    match mode {
        RunMode::Version => {
            println!("{}", flux_core::engine_label());
        }
        RunMode::ListMods => {
            let exit_code = run_list_mods();
            std::process::exit(exit_code);
        }
        RunMode::ListContent => {
            let exit_code = run_list_content();
            std::process::exit(exit_code);
        }
        RunMode::ListScenarios => {
            let exit_code = run_list_scenarios();
            std::process::exit(exit_code);
        }
        RunMode::RunScenario { scenario_id } => {
            let exit_code = run_scenario_by_id(&scenario_id);
            std::process::exit(exit_code);
        }
        RunMode::Windowed => run_windowed(),
        RunMode::Headless => run_headless(),
    }
}

fn parse_run_mode(args: &[String]) -> Result<RunMode, CliError> {
    let mut wants_version = false;
    let mut wants_headless = false;
    let mut wants_list_mods = false;
    let mut wants_list_content = false;
    let mut wants_list_scenarios = false;
    let mut scenario_id: Option<PrototypeId> = None;

    let mut index = 0usize;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--version" | "-V" => wants_version = true,
            "--headless" => wants_headless = true,
            "--list-mods" => wants_list_mods = true,
            "--list-content" => wants_list_content = true,
            "--list-scenarios" => wants_list_scenarios = true,
            "--run-scenario" => {
                let value = args
                    .get(index + 1)
                    .ok_or(CliError::MissingArgumentValue("--run-scenario"))?;
                let parsed = PrototypeId::parse(value)
                    .map_err(|_| CliError::InvalidScenarioId(value.clone()))?;
                scenario_id = Some(parsed);
                index += 1;
            }
            other => return Err(CliError::UnknownArgument(other.to_owned())),
        }
        index += 1;
    }

    let selected_modes = usize::from(wants_version)
        + usize::from(wants_headless)
        + usize::from(wants_list_mods)
        + usize::from(wants_list_content)
        + usize::from(wants_list_scenarios)
        + usize::from(scenario_id.is_some());

    if selected_modes > 1 {
        return Err(CliError::ConflictingArguments);
    }

    if wants_version {
        return Ok(RunMode::Version);
    }

    if wants_list_mods {
        return Ok(RunMode::ListMods);
    }

    if wants_list_content {
        return Ok(RunMode::ListContent);
    }

    if wants_list_scenarios {
        return Ok(RunMode::ListScenarios);
    }

    if let Some(scenario_id) = scenario_id {
        return Ok(RunMode::RunScenario { scenario_id });
    }

    if wants_headless {
        return Ok(RunMode::Headless);
    }

    Ok(RunMode::Windowed)
}

fn run_windowed() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                filter: "info,wgpu=warn,naga=warn".to_owned(),
                ..Default::default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    instance_flags: InstanceFlags::empty(),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: flux_core::ENGINE_NAME.to_owned(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
    );
    app.add_systems(Startup, windowed_diagnostics);
    app.run();
}

fn run_headless() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LogPlugin {
        filter: "info,wgpu=warn,naga=warn".to_owned(),
        ..Default::default()
    });
    app.add_systems(Startup, headless_diagnostics);
    app.finish();
    app.cleanup();
    app.update();
    info!("headless diagnostics completed");
}

fn windowed_diagnostics() {
    info!("startup mode=windowed engine={}", flux_core::engine_label());
    info!("window initialized");
}

fn headless_diagnostics() {
    info!("startup mode=headless engine={}", flux_core::engine_label());
    info!("headless diagnostics initialized");
}

fn run_list_mods() -> i32 {
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

fn run_list_content() -> i32 {
    let report = flux_mod_loader::discover_and_resolve_mods(Path::new("mods"));

    if !report.errors.is_empty() {
        eprintln!("errors: {}", report.errors.len());
        for error in &report.errors {
            eprintln!("{error}");
        }
        return 1;
    }

    let resolved_order = report
        .resolved_order
        .as_ref()
        .expect("resolved order must exist when there are no mod errors");

    let content_report = flux_content::load_content_registry(&report.valid_mods, resolved_order);
    if !content_report.errors.is_empty() {
        eprintln!("errors: {}", content_report.errors.len());
        for error in &content_report.errors {
            eprintln!("{error}");
        }
        return 1;
    }

    let registry = content_report
        .registry
        .expect("content registry must exist when there are no content errors");

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

fn run_list_scenarios() -> i32 {
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

fn run_scenario_by_id(scenario_id: &PrototypeId) -> i32 {
    const SCENARIO_FIXED_STEP: Duration = Duration::from_millis(16);
    const SCENARIO_CHUNK_SIZE: u32 = 16;

    init_cli_bevy_logging();

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

    let mut runtime = match SimRuntime::new(SCENARIO_FIXED_STEP, SCENARIO_CHUNK_SIZE) {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to initialize simulation runtime: {error}");
            return 1;
        }
    };

    match flux_scenario::run_scenario(&mut runtime, &scenario.definition) {
        Ok(summary) => {
            info!(
                "scenario_id={} scenario finished steps={} final_tick={}",
                summary.scenario_id, summary.executed_steps, summary.final_tick
            );
            0
        }
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn init_cli_bevy_logging() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        use bevy::log::tracing_subscriber::prelude::*;

        let env_filter = bevy::log::tracing_subscriber::EnvFilter::new("info,wgpu=warn,naga=warn");
        let fmt_layer = bevy::log::tracing_subscriber::fmt::layer();
        let subscriber = bevy::log::tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer);
        bevy::log::tracing::subscriber::set_global_default(subscriber)
            .expect("bevy tracing subscriber should initialize once");
    });
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScenarioLookupError {
    NotFound { scenario_id: String },
}

fn find_scenario_by_id<'a>(
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

fn print_patch_trail(
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

fn print_discovered_mod(module: &DiscoveredMod) {
    println!(
        "- id={} version={} api_version={} path={}",
        module.manifest.mod_id,
        module.manifest.version,
        module.manifest.api_version,
        module.directory_path.display()
    );
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::UnknownArgument(argument) => {
                write!(
                    f,
                    "unknown argument: {argument}. Supported args: --version, -V, --headless, --list-mods, --list-content, --list-scenarios, --run-scenario <id>"
                )
            }
            CliError::ConflictingArguments => {
                write!(
                    f,
                    "arguments --version, --headless, --list-mods, --list-content, --list-scenarios, and --run-scenario are mutually exclusive"
                )
            }
            CliError::MissingArgumentValue(flag) => {
                write!(f, "missing value for argument {flag}")
            }
            CliError::InvalidScenarioId(value) => {
                write!(
                    f,
                    "invalid scenario id `{value}`, expected namespace:path format"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flux_scenario::{
        LoadedScenario, ScenarioDefinition, ScenarioSource, ScenarioStep, WaitTicksStep,
    };

    #[test]
    fn parses_version_flag() {
        assert_eq!(
            parse_run_mode(&["--version".to_owned()]),
            Ok(RunMode::Version)
        );
        assert_eq!(parse_run_mode(&["-V".to_owned()]), Ok(RunMode::Version));
    }

    #[test]
    fn parses_headless_flag() {
        assert_eq!(
            parse_run_mode(&["--headless".to_owned()]),
            Ok(RunMode::Headless)
        );
    }

    #[test]
    fn parses_list_mods_flag() {
        assert_eq!(
            parse_run_mode(&["--list-mods".to_owned()]),
            Ok(RunMode::ListMods)
        );
    }

    #[test]
    fn parses_list_content_flag() {
        assert_eq!(
            parse_run_mode(&["--list-content".to_owned()]),
            Ok(RunMode::ListContent)
        );
    }

    #[test]
    fn parses_list_scenarios_flag() {
        assert_eq!(
            parse_run_mode(&["--list-scenarios".to_owned()]),
            Ok(RunMode::ListScenarios)
        );
    }

    #[test]
    fn parses_run_scenario_flag() {
        assert_eq!(
            parse_run_mode(&[
                "--run-scenario".to_owned(),
                "test_scenarios:scenario/bootstrap_smoke".to_owned()
            ]),
            Ok(RunMode::RunScenario {
                scenario_id: PrototypeId::parse("test_scenarios:scenario/bootstrap_smoke")
                    .expect("valid id")
            })
        );
    }

    #[test]
    fn defaults_to_windowed_mode() {
        assert_eq!(parse_run_mode(&[]), Ok(RunMode::Windowed));
    }

    #[test]
    fn rejects_unknown_argument() {
        assert_eq!(
            parse_run_mode(&["--unknown".to_owned()]),
            Err(CliError::UnknownArgument("--unknown".to_owned()))
        );
    }

    #[test]
    fn rejects_conflicting_arguments() {
        assert_eq!(
            parse_run_mode(&["--version".to_owned(), "--headless".to_owned()]),
            Err(CliError::ConflictingArguments)
        );
        assert_eq!(
            parse_run_mode(&["--list-mods".to_owned(), "--headless".to_owned()]),
            Err(CliError::ConflictingArguments)
        );
        assert_eq!(
            parse_run_mode(&["--version".to_owned(), "--list-mods".to_owned()]),
            Err(CliError::ConflictingArguments)
        );
        assert_eq!(
            parse_run_mode(&["--list-content".to_owned(), "--headless".to_owned()]),
            Err(CliError::ConflictingArguments)
        );
        assert_eq!(
            parse_run_mode(&[
                "--run-scenario".to_owned(),
                "test_scenarios:scenario/bootstrap_smoke".to_owned(),
                "--headless".to_owned()
            ]),
            Err(CliError::ConflictingArguments)
        );
    }

    #[test]
    fn rejects_invalid_run_scenario_arguments() {
        assert_eq!(
            parse_run_mode(&["--run-scenario".to_owned()]),
            Err(CliError::MissingArgumentValue("--run-scenario"))
        );
        assert_eq!(
            parse_run_mode(&["--run-scenario".to_owned(), "invalid".to_owned()]),
            Err(CliError::InvalidScenarioId("invalid".to_owned()))
        );
    }

    #[test]
    fn reports_nonexistent_scenario_id() {
        let scenarios = vec![LoadedScenario {
            definition: ScenarioDefinition {
                id: PrototypeId::parse("test_scenarios:scenario/bootstrap_smoke").expect("id"),
                steps: vec![ScenarioStep::WaitTicksStep(WaitTicksStep(1))],
            },
            source: ScenarioSource {
                mod_id: "test_scenarios".to_owned(),
                file: "mods/test_scenarios/scenarios/bootstrap_smoke.ron".to_owned(),
            },
        }];
        let missing_id =
            PrototypeId::parse("test_scenarios:scenario/does_not_exist").expect("valid id");

        let error = find_scenario_by_id(&scenarios, &missing_id).expect_err("must be missing");
        assert_eq!(
            error,
            ScenarioLookupError::NotFound {
                scenario_id: "test_scenarios:scenario/does_not_exist".to_owned()
            }
        );
        assert!(
            error
                .to_string()
                .contains("scenario_id: test_scenarios:scenario/does_not_exist")
        );
    }
}
