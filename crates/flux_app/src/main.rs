#![forbid(unsafe_code)]

mod helpers;
mod input_actions;
mod input_bindings;
mod scenario_runner;
mod simulation_driver;
mod world_debug;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use bevy::app::AppExit;
use bevy::asset::AssetPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{InstanceFlags, RenderCreation, WgpuSettings};
use bevy::window::{Window, WindowPlugin};
use flux_core::{NamespacedId, PrototypeId};
use flux_mod_loader::DiscoveredMod;
use flux_render::{FluxRenderPlugin, WorldRenderState};
use flux_sim::SimRuntime;
use flux_ui::{
    BindingAction, BuiltinUiActionDispatcher, ContainerLayout, UiMenuDefinition, UiMenuId,
    WidgetKind, WidgetNode,
};
use helpers::{find_scenario_by_id, format_error_block, print_discovered_mod, print_patch_trail};
use input_actions::{
    ActionExecutionFlow, InputActionRegistry, default_input_action_registry, execute_binding_action,
};
use input_bindings::{default_input_bindings, handle_input_bindings};
use scenario_runner::ScenarioRunConfig;
use simulation_driver::drive_live_simulation;

#[cfg(test)]
use helpers::ScenarioLookupError;

#[derive(Debug, Clone, PartialEq, Eq)]
enum RunMode {
    Version,
    ListMods,
    ListContent,
    ListScenarios,
    RunScenario {
        scenario_id: PrototypeId,
        visual_delay_ms: u64,
    },
    Windowed,
    Headless,
}

#[derive(Debug, PartialEq, Eq)]
enum CliError {
    UnknownArgument(String),
    ConflictingArguments,
    MissingArgumentValue(&'static str),
    InvalidScenarioId(String),
    InvalidScenarioVisualDelay(String),
    ScenarioVisualDelayRequiresRunScenario,
}

#[derive(Component)]
struct FluxUiRoot;

#[derive(Component, Clone)]
struct FluxUiButtonAction(BindingAction);

#[derive(Message, Clone)]
struct UiButtonPressed {
    action: BindingAction,
}

#[derive(Component)]
struct FluxUiCamera;

#[derive(Resource)]
struct FluxUiState {
    registry: flux_ui::UiRegistry,
    dispatcher: BuiltinUiActionDispatcher,
    known_menus: BTreeSet<UiMenuId>,
    needs_rebuild: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
enum FluxScreenMode {
    Menu,
    World,
}

#[derive(Resource)]
struct FluxSimState {
    runtime: SimRuntime,
    world_loaded: bool,
    simulation_paused: bool,
}

#[derive(Resource)]
struct FluxWorldDebugContent {
    // S11B temporary: content snapshot used to seed/debug-render world layers.
    registry: flux_content::ContentRegistry,
}

type UiButtonInteractionChanges<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static FluxUiButtonAction),
    (Changed<Interaction>, With<Button>),
>;

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
        } => {
            let exit_code = run_scenario_by_id(&scenario_id, visual_delay_ms);
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
    let mut scenario_visual_delay_ms: Option<u64> = None;

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
            "--scenario-visual-delay-ms" => {
                let value = args
                    .get(index + 1)
                    .ok_or(CliError::MissingArgumentValue("--scenario-visual-delay-ms"))?;
                let parsed = value
                    .parse::<u64>()
                    .map_err(|_| CliError::InvalidScenarioVisualDelay(value.clone()))?;
                scenario_visual_delay_ms = Some(parsed);
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

    if scenario_visual_delay_ms.is_some() && scenario_id.is_none() {
        return Err(CliError::ScenarioVisualDelayRequiresRunScenario);
    }

    if let Some(scenario_id) = scenario_id {
        return Ok(RunMode::RunScenario {
            scenario_id,
            visual_delay_ms: scenario_visual_delay_ms.unwrap_or(0),
        });
    }

    if wants_headless {
        return Ok(RunMode::Headless);
    }

    Ok(RunMode::Windowed)
}

fn run_windowed() {
    let asset_root = resolve_asset_root();
    let mut app = App::new();
    app.add_message::<UiButtonPressed>();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: asset_root,
                ..Default::default()
            })
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
    app.add_plugins(FluxRenderPlugin);
    app.add_systems(
        Startup,
        (
            windowed_diagnostics,
            setup_primary_ui_camera,
            setup_sim_runtime,
            setup_flux_ui_runtime,
        ),
    );
    app.add_systems(
        Update,
        (
            sync_ui_camera_activity,
            handle_input_bindings,
            drive_live_simulation.after(handle_input_bindings),
            emit_ui_button_press_events,
            handle_ui_button_actions,
            rebuild_flux_ui_if_needed,
        ),
    );
    app.run();
}

fn resolve_asset_root() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|error| {
        panic!("windowed startup failed: cannot resolve current dir: {error}")
    });
    cwd.to_string_lossy().into_owned()
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

fn setup_primary_ui_camera(mut commands: Commands) {
    commands.spawn((
        FluxUiCamera,
        Camera2d,
        Camera {
            order: 1,
            is_active: true,
            ..Default::default()
        },
    ));
    info!("ui camera initialized");
}

fn sync_ui_camera_activity(
    screen_mode: Option<Res<FluxScreenMode>>,
    mut cameras: Query<&mut Camera, With<FluxUiCamera>>,
) {
    let world_mode = matches!(screen_mode, Some(mode) if *mode == FluxScreenMode::World);
    for mut camera in &mut cameras {
        camera.is_active = !world_mode;
    }
}

fn setup_sim_runtime(mut commands: Commands) {
    const FIXED_STEP: Duration = Duration::from_millis(16);
    let runtime = SimRuntime::new(FIXED_STEP).unwrap_or_else(|error| {
        panic!("windowed startup failed: cannot create simulation runtime: {error}")
    });
    commands.insert_resource(FluxSimState {
        runtime,
        world_loaded: false,
        simulation_paused: false,
    });
    commands.insert_resource(FluxScreenMode::Menu);
}

fn setup_flux_ui_runtime(mut commands: Commands) {
    let report = flux_mod_loader::discover_and_resolve_mods(Path::new("mods"));
    if !report.errors.is_empty() {
        panic!(
            "ui startup failed during mod discovery:\n{}",
            format_error_block(&report.errors)
        );
    }

    let resolved_order = match report.resolved_order.as_ref() {
        Some(order) => order,
        None => {
            panic!("ui startup failed: resolved mod order is missing");
        }
    };

    let ui_report = flux_ui::load_ui_registry(&report.valid_mods, resolved_order);
    if !ui_report.errors.is_empty() {
        panic!(
            "ui startup failed during UI registration:\n{}",
            format_error_block(&ui_report.errors)
        );
    }

    let registry = match ui_report.registry {
        Some(registry) => registry,
        None => {
            panic!("ui startup failed: ui registry is missing");
        }
    };

    let known_menus = registry.menu_ids();
    info!("ui registry loaded: menus={}", known_menus.len());
    let initial_menu = resolve_initial_menu(&known_menus)
        .unwrap_or_else(|reason| panic!("ui startup failed: {reason}"));
    info!("ui initial menu: {}", initial_menu);

    commands.insert_resource(FluxUiState {
        registry,
        dispatcher: BuiltinUiActionDispatcher::new(initial_menu),
        known_menus,
        needs_rebuild: true,
    });
    commands.insert_resource(default_input_action_registry());
    commands.insert_resource(default_input_bindings());

    let content_report = flux_content::load_content_registry(&report.valid_mods, resolved_order);
    if !content_report.errors.is_empty() {
        panic!(
            "world debug startup failed during content registry load:\n{}",
            format_error_block(&content_report.errors)
        );
    }
    let content_registry = content_report
        .registry
        .unwrap_or_else(|| panic!("world debug startup failed: content registry is missing"));
    commands.insert_resource(FluxWorldDebugContent {
        registry: content_registry,
    });
}

fn resolve_initial_menu(known_menus: &BTreeSet<UiMenuId>) -> Result<UiMenuId, String> {
    const BASE_MAIN_MENU_ID: &str = "base:menu/main";

    let parsed = NamespacedId::parse(BASE_MAIN_MENU_ID).map_err(|_| {
        format!("invalid hardcoded initial menu id `{BASE_MAIN_MENU_ID}` (must be namespace:path)")
    })?;
    let initial_menu = UiMenuId(parsed);

    if known_menus.contains(&initial_menu) {
        return Ok(initial_menu);
    }

    Err(format!(
        "required initial menu `{BASE_MAIN_MENU_ID}` is not loaded"
    ))
}

#[allow(clippy::too_many_arguments)]
fn emit_ui_button_press_events(
    mut pressed_events: MessageWriter<UiButtonPressed>,
    interactions: UiButtonInteractionChanges<'_, '_>,
) {
    for (interaction, button_action) in &interactions {
        if *interaction == Interaction::Pressed {
            pressed_events.write(UiButtonPressed {
                action: button_action.0.clone(),
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_ui_button_actions(
    action_registry: Option<Res<InputActionRegistry>>,
    ui_state: Option<ResMut<FluxUiState>>,
    screen_mode: Option<ResMut<FluxScreenMode>>,
    sim_state: Option<ResMut<FluxSimState>>,
    world_debug_content: Option<Res<FluxWorldDebugContent>>,
    world_render_state: Option<ResMut<WorldRenderState>>,
    mut app_exit: MessageWriter<AppExit>,
    mut pressed_events: MessageReader<UiButtonPressed>,
) {
    let Some(action_registry) = action_registry else {
        return;
    };
    let Some(mut ui_state) = ui_state else {
        return;
    };
    let Some(mut screen_mode) = screen_mode else {
        return;
    };
    let Some(mut sim_state) = sim_state else {
        return;
    };
    let Some(world_debug_content) = world_debug_content else {
        return;
    };
    let Some(mut world_render_state) = world_render_state else {
        return;
    };
    for pressed in pressed_events.read() {
        if *screen_mode == FluxScreenMode::World {
            continue;
        }
        if execute_binding_action(
            &pressed.action,
            &action_registry,
            &mut ui_state,
            &mut screen_mode,
            sim_state.as_mut(),
            &world_debug_content,
            &mut world_render_state,
        ) == ActionExecutionFlow::Stop
        {
            app_exit.write(AppExit::error());
            return;
        }
    }
}

fn rebuild_flux_ui_if_needed(
    mut commands: Commands,
    ui_state: Option<ResMut<FluxUiState>>,
    screen_mode: Option<Res<FluxScreenMode>>,
    existing_roots: Query<Entity, With<FluxUiRoot>>,
) {
    if matches!(screen_mode, Some(mode) if *mode == FluxScreenMode::World) {
        for entity in &existing_roots {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Some(mut ui_state) = ui_state else {
        return;
    };
    if !ui_state.needs_rebuild {
        return;
    }

    for entity in &existing_roots {
        commands.entity(entity).despawn();
    }

    let current_menu_id = ui_state.dispatcher.menu_stack().current().clone();
    let Some(menu_definition) = ui_state.registry.menu(&current_menu_id) else {
        error!("ui rebuild skipped: current menu not found ({current_menu_id})");
        ui_state.needs_rebuild = false;
        return;
    };

    spawn_menu_ui(&mut commands, menu_definition);
    ui_state.needs_rebuild = false;
}

fn spawn_menu_ui(commands: &mut Commands, menu: &UiMenuDefinition) {
    let root_entity = commands
        .spawn((
            FluxUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.02, 0.02, 0.03)),
        ))
        .id();

    spawn_widget_tree(commands, root_entity, &menu.root);
}

fn spawn_widget_tree(commands: &mut Commands, parent_entity: Entity, node: &WidgetNode) {
    let widget_entity = match &node.kind {
        WidgetKind::Container(container) => {
            let flex_direction = match container.layout {
                ContainerLayout::Vertical => FlexDirection::Column,
                ContainerLayout::Horizontal => FlexDirection::Row,
            };
            commands
                .spawn((
                    Node {
                        flex_direction,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(8.0),
                        column_gap: Val::Px(8.0),
                        ..Default::default()
                    },
                    Name::new(node.id.to_string()),
                ))
                .id()
        }
        WidgetKind::Text(text) => commands
            .spawn((
                Text::new(text.text.as_str().to_owned()),
                TextFont {
                    font_size: 28.0,
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                Name::new(node.id.to_string()),
            ))
            .id(),
        WidgetKind::Button(button) => {
            let button_entity = commands
                .spawn((
                    Button,
                    Node {
                        border: UiRect::all(Val::Px(2.0)),
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                        min_width: Val::Px(280.0),
                        min_height: Val::Px(52.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    BorderColor::all(Color::WHITE),
                    BackgroundColor(Color::srgb(0.24, 0.46, 0.85)),
                    FluxUiButtonAction(button.action.clone()),
                    Name::new(node.id.to_string()),
                ))
                .id();
            let label_entity = commands
                .spawn((
                    Text::new(button.text.as_str().to_owned()),
                    TextFont {
                        font_size: 22.0,
                        ..Default::default()
                    },
                    TextColor(Color::WHITE),
                ))
                .id();
            commands.entity(button_entity).add_child(label_entity);
            button_entity
        }
        WidgetKind::ExtensionPoint(_) => commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    ..Default::default()
                },
                Name::new(node.id.to_string()),
            ))
            .id(),
    };

    commands.entity(parent_entity).add_child(widget_entity);

    if matches!(
        node.kind,
        WidgetKind::Container(_) | WidgetKind::ExtensionPoint(_)
    ) {
        for child in &node.children {
            spawn_widget_tree(commands, widget_entity, child);
        }
    }
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

fn run_scenario_by_id(scenario_id: &PrototypeId, visual_delay_ms: u64) -> i32 {
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

    scenario_runner::run_scenario_windowed(scenario, ScenarioRunConfig { visual_delay_ms })
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

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::UnknownArgument(argument) => {
                write!(
                    f,
                    "unknown argument: {argument}. Supported args: --version, -V, --headless, --list-mods, --list-content, --list-scenarios, --run-scenario <id>, --scenario-visual-delay-ms <ms>"
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
            CliError::InvalidScenarioVisualDelay(value) => write!(
                f,
                "invalid value for --scenario-visual-delay-ms `{value}`, expected non-negative integer milliseconds"
            ),
            CliError::ScenarioVisualDelayRequiresRunScenario => write!(
                f,
                "--scenario-visual-delay-ms can be used only together with --run-scenario <id>"
            ),
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
                    .expect("valid id"),
                visual_delay_ms: 0,
            })
        );
    }

    #[test]
    fn parses_run_scenario_with_visual_delay_flag() {
        assert_eq!(
            parse_run_mode(&[
                "--run-scenario".to_owned(),
                "test_scenarios:scenario/bootstrap_smoke".to_owned(),
                "--scenario-visual-delay-ms".to_owned(),
                "250".to_owned(),
            ]),
            Ok(RunMode::RunScenario {
                scenario_id: PrototypeId::parse("test_scenarios:scenario/bootstrap_smoke")
                    .expect("valid id"),
                visual_delay_ms: 250,
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
        assert_eq!(
            parse_run_mode(&["--scenario-visual-delay-ms".to_owned(), "100".to_owned()]),
            Err(CliError::ScenarioVisualDelayRequiresRunScenario)
        );
        assert_eq!(
            parse_run_mode(&[
                "--run-scenario".to_owned(),
                "test_scenarios:scenario/bootstrap_smoke".to_owned(),
                "--scenario-visual-delay-ms".to_owned(),
                "bad".to_owned(),
            ]),
            Err(CliError::InvalidScenarioVisualDelay("bad".to_owned()))
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

    #[test]
    fn resolves_base_main_menu_as_initial_menu() {
        let mut known_menus = BTreeSet::new();
        let main_menu = UiMenuId(NamespacedId::parse("base:menu/main").expect("id"));
        known_menus.insert(main_menu.clone());
        known_menus.insert(UiMenuId(
            NamespacedId::parse("example_ui:menu/debug").expect("id"),
        ));

        let resolved = resolve_initial_menu(&known_menus).expect("initial menu must resolve");
        assert_eq!(resolved, main_menu);
    }

    #[test]
    fn rejects_missing_base_main_menu() {
        let mut known_menus = BTreeSet::new();
        known_menus.insert(UiMenuId(
            NamespacedId::parse("example_ui:menu/debug").expect("id"),
        ));

        let error = resolve_initial_menu(&known_menus).expect_err("must fail");
        assert!(error.contains("required initial menu `base:menu/main` is not loaded"));
    }
}
