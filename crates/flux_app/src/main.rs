#![forbid(unsafe_code)]

mod app_bootstrap;
mod app_state;
mod cli;
mod cli_commands;
mod helpers;
mod input_actions;
mod input_bindings;
mod scenario_runner;
mod simulation_driver;
mod ui_runtime;
mod world_debug;

use cli::{RunMode, parse_run_mode};

use app_bootstrap::{run_headless, run_windowed};
pub(crate) use app_bootstrap::{
    setup_primary_ui_camera, setup_sim_runtime, sync_ui_camera_activity, windowed_diagnostics,
};
pub(crate) use app_state::{
    FluxBackendPolicy, FluxScreenMode, FluxSimState, FluxUiState, FluxWorldDebugContent,
    UiButtonPressed,
};
use cli_commands::{run_list_content, run_list_mods, run_list_scenarios, run_scenario_by_id};
pub(crate) use ui_runtime::{
    emit_ui_button_press_events, handle_ui_button_actions, rebuild_flux_ui_if_needed,
    setup_flux_ui_runtime,
};

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
        RunMode::RunScenario {
            scenario_id,
            visual_delay_ms,
            backend_policy,
        } => {
            let exit_code = run_scenario_by_id(&scenario_id, visual_delay_ms, backend_policy);
            std::process::exit(exit_code);
        }
        RunMode::Windowed { backend_policy } => run_windowed(backend_policy),
        RunMode::Headless => run_headless(),
    }
}
