use std::time::Duration;

use bevy::asset::AssetPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{InstanceFlags, RenderCreation, WgpuSettings};
use bevy::window::{Window, WindowPlugin};
use flux_render::FluxRenderPlugin;
use flux_sim::{BackendPolicy, SimRuntime};
use flux_world::GasPrototypeId;

use crate::app_state::{
    FluxBackendPolicy, FluxScreenMode, FluxSimState, FluxUiCamera, UiButtonPressed,
};
use crate::input_bindings::handle_input_bindings;
use crate::simulation_driver::drive_live_simulation;
use crate::ui_runtime::{
    emit_ui_button_press_events, handle_ui_button_actions, rebuild_flux_ui_if_needed,
    setup_flux_ui_runtime,
};

pub(crate) fn run_windowed(backend_policy: BackendPolicy) {
    let asset_root = resolve_asset_root();
    let mut app = App::new();
    app.add_message::<UiButtonPressed>();
    app.insert_resource(FluxBackendPolicy(backend_policy));
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
            setup_flux_ui_runtime,
            setup_sim_runtime,
        )
            .chain(),
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

pub(crate) fn run_headless() {
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

pub(crate) fn windowed_diagnostics() {
    info!("startup mode=windowed engine={}", flux_core::engine_label());
    info!("window initialized");
}

pub(crate) fn headless_diagnostics() {
    info!("startup mode=headless engine={}", flux_core::engine_label());
    info!("headless diagnostics initialized");
}

pub(crate) fn setup_primary_ui_camera(mut commands: Commands) {
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

pub(crate) fn sync_ui_camera_activity(
    screen_mode: Option<Res<FluxScreenMode>>,
    mut cameras: Query<&mut Camera, With<FluxUiCamera>>,
) {
    let world_mode = matches!(screen_mode, Some(mode) if *mode == FluxScreenMode::World);
    for mut camera in &mut cameras {
        camera.is_active = !world_mode;
    }
}

pub(crate) fn setup_sim_runtime(
    mut commands: Commands,
    backend_policy: Option<Res<FluxBackendPolicy>>,
    world_debug_content: Option<Res<crate::FluxWorldDebugContent>>,
) {
    const FIXED_STEP: Duration = Duration::from_millis(16);
    let backend_policy = backend_policy
        .as_ref()
        .map(|policy| policy.0)
        .unwrap_or(BackendPolicy::CpuOnly);
    let mut runtime = SimRuntime::new(FIXED_STEP, backend_policy).unwrap_or_else(|error| {
        panic!("windowed startup failed: cannot create simulation runtime: {error}")
    });
    if let Some(world_debug_content) = world_debug_content {
        runtime.set_gas_prototypes(collect_registry_gas_prototypes(
            &world_debug_content.registry,
        ));
    }
    commands.insert_resource(FluxSimState {
        runtime,
        world_loaded: false,
        simulation_paused: false,
    });
    commands.insert_resource(FluxScreenMode::Menu);
}

fn collect_registry_gas_prototypes(
    registry: &flux_content::ContentRegistry,
) -> Vec<GasPrototypeId> {
    registry
        .gases()
        .map(|record| record.prototype.id.clone())
        .collect()
}

fn resolve_asset_root() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|error| {
        panic!("windowed startup failed: cannot resolve current dir: {error}")
    });
    cwd.to_string_lossy().into_owned()
}
