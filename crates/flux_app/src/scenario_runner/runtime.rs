use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use bevy::app::AppExit;
use bevy::log::tracing_subscriber::Layer;
use bevy::log::{BoxedLayer, info};
use bevy::prelude::{
    App, Commands, IntoScheduleConfigs, MessageWriter, PluginGroup, Res, ResMut, Resource,
};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::WindowPlugin;
use bevy::{
    asset::AssetPlugin, log::LogPlugin, prelude::Update, render::RenderPlugin, window::Window,
};
use flux_render::{FluxRenderPlugin, WorldRenderState};
use flux_scenario::{
    AssertUiExistsStep, ClickStep, CreateWorldStep, LoadedScenario, OpenMenuStep,
    PauseSimulationStep, ScenarioDefinition, ScenarioStep, TakeScreenshotStep, WaitRealtimeStep,
    WaitSimulationTimeStep, WaitTicksStep,
};
use flux_sim::SimCommand;
use flux_ui::{
    BindingAction, BuiltinUiActionDispatcher, UiMenuDefinition, UiWidgetId, WidgetKind, WidgetNode,
};

use super::artifacts::scenario_artifact_dir;
use super::validation::{
    ScenarioValidationState, simulation_ticks_for_delay, validate_scenario_steps,
};
use crate::{
    FluxScreenMode, FluxSimState, FluxUiState, FluxWorldDebugContent, setup_flux_ui_runtime,
    setup_primary_ui_camera, setup_sim_runtime, windowed_diagnostics, world_debug,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScenarioRunConfig {
    pub visual_delay_ms: u64,
}

#[derive(Resource)]
struct ScenarioBootstrapConfig {
    scenario: LoadedScenario,
    config: ScenarioRunConfig,
}

#[derive(Resource, Debug, Clone)]
struct ScenarioLogLayerConfig {
    diagnostic_log_path: PathBuf,
}

#[derive(Resource)]
struct ScenarioRuntimeState {
    scenario: ScenarioDefinition,
    visual_delay_ms: u64,
    current_step: usize,
    sim_paused: bool,
    world_loaded: bool,
    world_open: bool,
    waiting_until: Option<Duration>,
    waiting_capture: Option<ScreenshotCaptureWait>,
    resume_after_wait: bool,
    pending_exit: Option<AppExit>,
    artifact_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScreenshotCaptureWait {
    output_path: PathBuf,
    deadline: Duration,
}

const SCREENSHOT_WRITE_TIMEOUT_MS: u64 = 30_000;

pub(crate) fn run_scenario_windowed(scenario: &LoadedScenario, config: ScenarioRunConfig) -> i32 {
    let cwd = std::env::current_dir()
        .unwrap_or_else(|error| panic!("scenario startup failed: cannot resolve cwd: {error}"));
    let asset_root = cwd.to_string_lossy().into_owned();
    let artifact_dir = cwd.join(scenario_artifact_dir(&scenario.definition.id));
    if let Err(error) = fs::create_dir_all(&artifact_dir) {
        panic!(
            "scenario startup failed: cannot create artifact directory `{}`: {error}",
            artifact_dir.display()
        );
    }
    let diagnostic_log_path = artifact_dir.join("diagnostic.log");
    if let Err(error) = fs::write(&diagnostic_log_path, "") {
        panic!(
            "scenario startup failed: cannot initialize diagnostic log `{}`: {error}",
            diagnostic_log_path.display()
        );
    }

    let mut app = App::new();
    app.insert_resource(ScenarioLogLayerConfig {
        diagnostic_log_path,
    });
    app.add_message::<crate::UiButtonPressed>();
    app.add_plugins(
        bevy::prelude::DefaultPlugins
            .set(AssetPlugin {
                file_path: asset_root,
                ..Default::default()
            })
            .set(LogPlugin {
                filter: "info,wgpu=warn,naga=warn".to_owned(),
                custom_layer: scenario_diagnostic_log_layer,
                ..Default::default()
            })
            .set(RenderPlugin::default())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: flux_core::ENGINE_NAME.to_owned(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
    );
    app.add_plugins(FluxRenderPlugin);
    app.insert_resource(ScenarioBootstrapConfig {
        scenario: scenario.clone(),
        config,
    });
    app.add_systems(
        bevy::prelude::Startup,
        (
            windowed_diagnostics,
            setup_primary_ui_camera,
            setup_sim_runtime,
            setup_flux_ui_runtime,
            setup_scenario_runtime_state,
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (
            crate::sync_ui_camera_activity,
            crate::input_bindings::handle_input_bindings,
            crate::emit_ui_button_press_events,
            crate::handle_ui_button_actions,
            crate::rebuild_flux_ui_if_needed,
            drive_scenario_runtime.after(crate::rebuild_flux_ui_if_needed),
        ),
    );
    let exit = app.run();
    if exit.is_success() { 0 } else { 1 }
}

fn setup_scenario_runtime_state(
    mut commands: Commands,
    bootstrap: Res<ScenarioBootstrapConfig>,
    ui_state: Option<Res<FluxUiState>>,
) {
    let Some(ui_state) = ui_state else {
        panic!("scenario startup failed: ui state is missing");
    };

    let scenario_id = bootstrap.scenario.definition.id.clone();
    let artifact_dir = std::env::current_dir()
        .unwrap_or_else(|error| panic!("scenario startup failed: cannot resolve cwd: {error}"))
        .join(scenario_artifact_dir(&scenario_id));
    if let Err(error) = fs::create_dir_all(&artifact_dir) {
        panic!(
            "scenario startup failed: cannot create artifact directory `{}`: {error}",
            artifact_dir.display()
        );
    }

    let mut state = ScenarioRuntimeState {
        scenario: bootstrap.scenario.definition.clone(),
        visual_delay_ms: bootstrap.config.visual_delay_ms,
        current_step: 0,
        sim_paused: false,
        world_loaded: false,
        world_open: false,
        waiting_until: None,
        waiting_capture: None,
        resume_after_wait: false,
        pending_exit: None,
        artifact_dir,
    };
    let initial_menu = ui_state.dispatcher.menu_stack().current().clone();
    let mut validation_state = ScenarioValidationState {
        world_loaded: false,
        sim_paused: false,
        world_open: false,
        dispatcher: BuiltinUiActionDispatcher::new(initial_menu),
        known_menus: ui_state.known_menus.clone(),
    };

    if let Err(error) = validate_scenario_steps(
        &bootstrap.scenario.definition,
        &ui_state.registry,
        &mut validation_state,
    ) {
        push_diag(
            &mut state,
            format!(
                "validation failed step_index={} step={} reason={}",
                error.step_index, error.step_kind, error.reason
            ),
        );
        state.pending_exit = Some(AppExit::error());
    } else {
        push_diag(&mut state, "scenario validation passed".to_owned());
    }

    commands.insert_resource(state);
}

#[allow(clippy::too_many_arguments)]
fn drive_scenario_runtime(
    mut commands: Commands,
    time_real: Res<bevy::prelude::Time<bevy::time::Real>>,
    mut app_exit: MessageWriter<AppExit>,
    runtime_state: Option<ResMut<ScenarioRuntimeState>>,
    ui_state: Option<ResMut<FluxUiState>>,
    screen_mode: Option<ResMut<FluxScreenMode>>,
    sim_state: Option<ResMut<FluxSimState>>,
    world_debug_content: Option<Res<FluxWorldDebugContent>>,
    world_render_state: Option<ResMut<WorldRenderState>>,
) {
    let Some(mut runtime_state) = runtime_state else {
        return;
    };

    if let Some(exit) = runtime_state.pending_exit.clone() {
        app_exit.write(exit);
        return;
    }

    let now = time_real.elapsed();
    if let Some(until) = runtime_state.waiting_until {
        if now < until {
            return;
        }
        runtime_state.waiting_until = None;
        if runtime_state.resume_after_wait {
            runtime_state.resume_after_wait = false;
            runtime_state.sim_paused = false;
            runtime_state.world_open = true;
        }
    }

    if let Some(waiting_capture) = runtime_state.waiting_capture.as_ref() {
        let output_path = waiting_capture.output_path.clone();
        let deadline = waiting_capture.deadline;
        if output_path.is_file() {
            push_diag(
                &mut runtime_state,
                format!("screenshot written: {}", output_path.display()),
            );
            runtime_state.waiting_capture = None;
        } else if now < deadline {
            return;
        } else {
            let reason = format!(
                "scenario runtime failed: screenshot was not written in time: {}",
                output_path.display()
            );
            push_runtime_failure(&mut runtime_state, &mut app_exit, &reason);
            return;
        }
    }

    if runtime_state.current_step >= runtime_state.scenario.steps.len() {
        let final_tick = sim_state
            .as_ref()
            .map(|state| state.runtime.tick_counter())
            .unwrap_or_default();
        let finished_line = format!(
            "scenario finished: steps={} final_tick={final_tick}",
            runtime_state.scenario.steps.len(),
        );
        push_diag(&mut runtime_state, finished_line);
        app_exit.write(AppExit::Success);
        runtime_state.pending_exit = Some(AppExit::Success);
        return;
    }

    let Some(mut ui_state) = ui_state else {
        push_runtime_failure(
            &mut runtime_state,
            &mut app_exit,
            "scenario runtime failed: ui state is missing",
        );
        return;
    };
    let Some(mut screen_mode) = screen_mode else {
        push_runtime_failure(
            &mut runtime_state,
            &mut app_exit,
            "scenario runtime failed: screen mode state is missing",
        );
        return;
    };
    let Some(mut sim_state) = sim_state else {
        push_runtime_failure(
            &mut runtime_state,
            &mut app_exit,
            "scenario runtime failed: simulation state is missing",
        );
        return;
    };
    let Some(world_debug_content) = world_debug_content else {
        push_runtime_failure(
            &mut runtime_state,
            &mut app_exit,
            "scenario runtime failed: world debug content is missing",
        );
        return;
    };
    let Some(mut world_render_state) = world_render_state else {
        push_runtime_failure(
            &mut runtime_state,
            &mut app_exit,
            "scenario runtime failed: world render state is missing",
        );
        return;
    };

    let step_index = runtime_state.current_step;
    let step = runtime_state.scenario.steps[step_index].clone();
    let step_kind = step.kind();
    let result = execute_step(
        &mut commands,
        &mut runtime_state,
        &step,
        &mut ui_state,
        &mut screen_mode,
        &mut sim_state,
        &world_debug_content,
        &mut world_render_state,
        now,
    );
    match result {
        Ok(()) => {
            push_diag(
                &mut runtime_state,
                format!("step_index={step_index} step={step_kind} status=ok"),
            );
            runtime_state.current_step += 1;
            let has_more_steps = runtime_state.current_step < runtime_state.scenario.steps.len();
            runtime_state.waiting_until = append_visual_delay_after_step_if_needed(
                runtime_state.waiting_until,
                now,
                runtime_state.visual_delay_ms,
                has_more_steps,
            );
        }
        Err(reason) => {
            push_diag(
                &mut runtime_state,
                format!("step_index={step_index} step={step_kind} status=failed reason={reason}"),
            );
            app_exit.write(AppExit::error());
            runtime_state.pending_exit = Some(AppExit::error());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_step(
    commands: &mut Commands,
    runtime_state: &mut ScenarioRuntimeState,
    step: &ScenarioStep,
    ui_state: &mut FluxUiState,
    screen_mode: &mut FluxScreenMode,
    sim_state: &mut FluxSimState,
    world_debug_content: &FluxWorldDebugContent,
    world_render_state: &mut WorldRenderState,
    now: Duration,
) -> Result<(), String> {
    match step {
        ScenarioStep::LogStep(step) => {
            push_diag(runtime_state, step.0.clone());
            Ok(())
        }
        ScenarioStep::CreateWorldStep(step) => create_world(
            runtime_state,
            step,
            sim_state,
            world_debug_content,
            world_render_state,
            screen_mode,
            ui_state,
        ),
        ScenarioStep::WaitTicksStep(step) => wait_ticks(sim_state, step),
        ScenarioStep::AssertTickStep(step) => assert_tick(sim_state, step),
        ScenarioStep::OpenMenuStep(step) => open_menu(runtime_state, step, ui_state, screen_mode),
        ScenarioStep::ClickStep(step) => click_widget(
            runtime_state,
            step,
            ui_state,
            sim_state,
            world_debug_content,
            world_render_state,
            screen_mode,
        ),
        ScenarioStep::WaitSimulationTimeStep(step) => wait_simulation_time(sim_state, step),
        ScenarioStep::PauseSimulationStep(step) => pause_simulation(runtime_state, step, now),
        ScenarioStep::WaitRealtimeStep(step) => wait_realtime(runtime_state, step, now),
        ScenarioStep::ResumeSimulationStep(_) => {
            resume_simulation(runtime_state, screen_mode, ui_state)
        }
        ScenarioStep::TakeScreenshotStep(step) => {
            take_screenshot(commands, runtime_state, step, now)
        }
        ScenarioStep::AssertUiExistsStep(step) => assert_ui_exists(runtime_state, step, ui_state),
        ScenarioStep::SetCameraPivotStep(step) => {
            world_render_state.request_camera_pivot(step.x, step.y);
            Ok(())
        }
        ScenarioStep::SetCameraZoomStep(step) => {
            world_render_state.request_camera_zoom(step.zoom);
            Ok(())
        }
    }
}

fn create_world(
    runtime_state: &mut ScenarioRuntimeState,
    step: &CreateWorldStep,
    sim_state: &mut FluxSimState,
    world_debug_content: &FluxWorldDebugContent,
    world_render_state: &mut WorldRenderState,
    screen_mode: &mut FluxScreenMode,
    ui_state: &mut FluxUiState,
) -> Result<(), String> {
    sim_state
        .runtime
        .enqueue_command(SimCommand::CreateWorld {
            width: step.width,
            height: step.height,
            seed: step.seed,
        })
        .map_err(|error| format!("cannot enqueue CreateWorld: {error}"))?;
    sim_state
        .runtime
        .initialize()
        .map_err(|error| format!("cannot initialize runtime after CreateWorld: {error}"))?;
    let Some(world) = sim_state.runtime.world_mut() else {
        return Err("world is missing after CreateWorld".to_owned());
    };
    world_debug::populate_world_debug_mvp(world, &world_debug_content.registry)
        .map_err(|error| format!("world population failed: {error}"))?;
    let snapshot = world_debug::build_world_render_snapshot(world, &world_debug_content.registry)
        .map_err(|error| format!("world render snapshot failed: {error}"))?;
    world_render_state.show_world(world.size(), 1.0, snapshot);
    ui_state.dispatcher.reset_menu_stack_to_root();
    *screen_mode = FluxScreenMode::World;
    ui_state.needs_rebuild = false;
    runtime_state.world_loaded = true;
    runtime_state.world_open = true;
    runtime_state.sim_paused = false;
    Ok(())
}

fn wait_ticks(sim_state: &mut FluxSimState, step: &WaitTicksStep) -> Result<(), String> {
    sim_state
        .runtime
        .enqueue_command(SimCommand::WaitTicks { ticks: step.0 })
        .map_err(|error| format!("cannot enqueue WaitTicks: {error}"))?;
    sim_state
        .runtime
        .initialize()
        .map_err(|error| format!("cannot initialize runtime after WaitTicks: {error}"))?;
    Ok(())
}

fn wait_simulation_time(
    sim_state: &mut FluxSimState,
    step: &WaitSimulationTimeStep,
) -> Result<(), String> {
    let ticks = simulation_ticks_for_delay(&sim_state.runtime, step.delay_ms);
    if ticks == 0 {
        return Ok(());
    }
    sim_state
        .runtime
        .enqueue_command(SimCommand::WaitTicks { ticks })
        .map_err(|error| format!("cannot enqueue WaitSimulationTime: {error}"))?;
    sim_state
        .runtime
        .initialize()
        .map_err(|error| format!("cannot initialize runtime after WaitSimulationTime: {error}"))?;
    Ok(())
}

fn assert_tick(
    sim_state: &mut FluxSimState,
    step: &flux_scenario::AssertTickStep,
) -> Result<(), String> {
    let actual = sim_state.runtime.tick_counter();
    if actual == step.0 {
        return Ok(());
    }
    Err(format!(
        "assert tick failed expected={} actual={actual}",
        step.0
    ))
}

fn open_menu(
    runtime_state: &mut ScenarioRuntimeState,
    step: &OpenMenuStep,
    ui_state: &mut FluxUiState,
    screen_mode: &mut FluxScreenMode,
) -> Result<(), String> {
    apply_ui_action(
        runtime_state,
        &BindingAction::OpenMenu(step.0.clone()),
        ui_state,
        None,
        None,
        None,
        screen_mode,
    )
}

fn click_widget(
    runtime_state: &mut ScenarioRuntimeState,
    step: &ClickStep,
    ui_state: &mut FluxUiState,
    sim_state: &mut FluxSimState,
    world_debug_content: &FluxWorldDebugContent,
    world_render_state: &mut WorldRenderState,
    screen_mode: &mut FluxScreenMode,
) -> Result<(), String> {
    if runtime_state.world_open {
        return Err("Click is available only when UI menu is open".to_owned());
    }

    let action = {
        let widget_id = &step.0;
        let current_menu_id = ui_state.dispatcher.menu_stack().current();
        let Some(menu) = ui_state.registry.menu(current_menu_id) else {
            return Err("current menu is unavailable".to_owned());
        };
        let Some(node) = find_widget(menu, widget_id) else {
            return Err(format!(
                "widget `{widget_id}` is not available in current menu"
            ));
        };
        let WidgetKind::Button(button) = &node.kind else {
            return Err(format!("widget `{widget_id}` is not a button"));
        };
        button.action.clone()
    };
    apply_ui_action(
        runtime_state,
        &action,
        ui_state,
        Some(sim_state),
        Some(world_debug_content),
        Some(world_render_state),
        screen_mode,
    )
}

fn pause_simulation(
    runtime_state: &mut ScenarioRuntimeState,
    step: &PauseSimulationStep,
    now: Duration,
) -> Result<(), String> {
    runtime_state.sim_paused = true;
    if step.delay_ms > 0 {
        runtime_state.waiting_until = Some(wait_deadline(now, step.delay_ms)?);
        runtime_state.resume_after_wait = true;
    }
    Ok(())
}

fn wait_realtime(
    runtime_state: &mut ScenarioRuntimeState,
    step: &WaitRealtimeStep,
    now: Duration,
) -> Result<(), String> {
    runtime_state.waiting_until = Some(wait_deadline(now, step.delay_ms)?);
    Ok(())
}

pub(super) fn wait_deadline(now: Duration, delay_ms: u64) -> Result<Duration, String> {
    let delay = Duration::from_millis(delay_ms);
    now.checked_add(delay)
        .ok_or_else(|| format!("wait delay overflow: now={now:?} delay_ms={delay_ms}"))
}

fn resume_simulation(
    runtime_state: &mut ScenarioRuntimeState,
    screen_mode: &mut FluxScreenMode,
    ui_state: &mut FluxUiState,
) -> Result<(), String> {
    if !runtime_state.world_loaded {
        return Err("ResumeSimulation requires loaded world".to_owned());
    }
    if !runtime_state.sim_paused {
        return Err("ResumeSimulation requires simulation to be paused".to_owned());
    }
    if !runtime_state.world_open {
        return Err("ResumeSimulation is not available while menu UI is open".to_owned());
    }
    runtime_state.sim_paused = false;
    *screen_mode = FluxScreenMode::World;
    ui_state.needs_rebuild = false;
    Ok(())
}

fn take_screenshot(
    commands: &mut Commands,
    runtime_state: &mut ScenarioRuntimeState,
    step: &TakeScreenshotStep,
    now: Duration,
) -> Result<(), String> {
    let filename = &step.0;
    let path = runtime_state.artifact_dir.join(filename);
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "cannot remove existing screenshot `{}` before capture: {error}",
                path.display()
            ));
        }
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()));
    runtime_state.waiting_capture = Some(ScreenshotCaptureWait {
        output_path: path,
        deadline: now.saturating_add(Duration::from_millis(SCREENSHOT_WRITE_TIMEOUT_MS)),
    });
    Ok(())
}

fn assert_ui_exists(
    runtime_state: &ScenarioRuntimeState,
    step: &AssertUiExistsStep,
    ui_state: &FluxUiState,
) -> Result<(), String> {
    if runtime_state.world_open {
        return Err("AssertUiExists is available only when UI menu is open".to_owned());
    }

    let widget_id = &step.0;
    let current_menu_id = ui_state.dispatcher.menu_stack().current();
    let Some(menu) = ui_state.registry.menu(current_menu_id) else {
        return Err("current menu is unavailable".to_owned());
    };
    if find_widget(menu, widget_id).is_some() {
        Ok(())
    } else {
        Err(format!(
            "widget `{widget_id}` is not available in current menu"
        ))
    }
}

fn apply_ui_action(
    runtime_state: &mut ScenarioRuntimeState,
    action: &BindingAction,
    ui_state: &mut FluxUiState,
    sim_state: Option<&mut FluxSimState>,
    world_debug_content: Option<&FluxWorldDebugContent>,
    world_render_state: Option<&mut WorldRenderState>,
    screen_mode: &mut FluxScreenMode,
) -> Result<(), String> {
    match action {
        BindingAction::OpenMenu(menu_id) => {
            if ui_state.dispatcher.menu_stack().current() != menu_id {
                ui_state
                    .dispatcher
                    .open_menu(menu_id, &ui_state.known_menus)
                    .map_err(|error| error.to_string())?;
            }
            runtime_state.world_open = false;
            if runtime_state.world_loaded {
                runtime_state.sim_paused = true;
            }
            *screen_mode = FluxScreenMode::Menu;
            ui_state.needs_rebuild = true;
            Ok(())
        }
        BindingAction::BackMenu => {
            if ui_state.dispatcher.back_menu() {
                runtime_state.world_open = false;
                if runtime_state.world_loaded {
                    runtime_state.sim_paused = true;
                }
                *screen_mode = FluxScreenMode::Menu;
                ui_state.needs_rebuild = true;
            } else if runtime_state.world_loaded {
                runtime_state.world_open = true;
                runtime_state.sim_paused = false;
                *screen_mode = FluxScreenMode::World;
                ui_state.needs_rebuild = false;
            }
            Ok(())
        }
        BindingAction::DiagnosticLog(message) => {
            info!("ui action log: {message}");
            Ok(())
        }
        BindingAction::RunWorld => {
            let Some(sim_state) = sim_state else {
                return Err("RunWorld requires simulation runtime context".to_owned());
            };
            let Some(world_debug_content) = world_debug_content else {
                return Err("RunWorld requires world debug content".to_owned());
            };
            let Some(world_render_state) = world_render_state else {
                return Err("RunWorld requires world render state".to_owned());
            };
            create_world(
                runtime_state,
                &CreateWorldStep {
                    width: 64,
                    height: 64,
                    seed: 1,
                },
                sim_state,
                world_debug_content,
                world_render_state,
                screen_mode,
                ui_state,
            )
        }
        BindingAction::ToggleSimulation => {
            if runtime_state.world_loaded {
                runtime_state.sim_paused = !runtime_state.sim_paused;
            }
            Ok(())
        }
    }
}

pub(super) fn append_visual_delay_after_step(
    waiting_until: Option<Duration>,
    now: Duration,
    visual_delay_ms: u64,
) -> Option<Duration> {
    if visual_delay_ms == 0 {
        return waiting_until;
    }

    let visual_delay = Duration::from_millis(visual_delay_ms);
    let base = waiting_until.filter(|until| *until > now).unwrap_or(now);
    Some(base.saturating_add(visual_delay))
}

pub(super) fn append_visual_delay_after_step_if_needed(
    waiting_until: Option<Duration>,
    now: Duration,
    visual_delay_ms: u64,
    has_more_steps: bool,
) -> Option<Duration> {
    if !has_more_steps {
        return waiting_until;
    }

    append_visual_delay_after_step(waiting_until, now, visual_delay_ms)
}

pub(super) fn find_widget<'a>(
    menu: &'a UiMenuDefinition,
    id: &UiWidgetId,
) -> Option<&'a WidgetNode> {
    find_widget_in_tree(&menu.root, id)
}

fn find_widget_in_tree<'a>(node: &'a WidgetNode, id: &UiWidgetId) -> Option<&'a WidgetNode> {
    if node.id == *id {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_widget_in_tree(child, id) {
            return Some(found);
        }
    }
    None
}

fn push_diag(_state: &mut ScenarioRuntimeState, line: String) {
    info!("{line}");
}

fn push_runtime_failure(
    state: &mut ScenarioRuntimeState,
    app_exit: &mut MessageWriter<AppExit>,
    reason: &str,
) {
    push_diag(state, reason.to_owned());
    app_exit.write(AppExit::error());
    state.pending_exit = Some(AppExit::error());
}

fn scenario_diagnostic_log_layer(app: &mut App) -> Option<BoxedLayer> {
    let config = app.world().get_resource::<ScenarioLogLayerConfig>()?;
    let file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&config.diagnostic_log_path)
        .ok()?;
    let filter = bevy::log::tracing_subscriber::filter::FilterFn::new(|meta| {
        meta.target()
            .starts_with("flux_app::scenario_runner::runtime")
    });
    let layer = bevy::log::tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(Mutex::new(file))
        .with_filter(filter);
    Some(Box::new(layer))
}
