use bevy::app::AppExit;
use bevy::log::error;
use bevy::prelude::{MessageWriter, Res, ResMut};
use flux_render::WorldRenderState;
use flux_sim::SimError;

use crate::{FluxScreenMode, FluxSimState, FluxWorldDebugContent, world_debug};

#[allow(clippy::too_many_arguments)]
pub(crate) fn drive_live_simulation(
    screen_mode: Option<Res<FluxScreenMode>>,
    sim_state: Option<ResMut<FluxSimState>>,
    world_debug_content: Option<Res<FluxWorldDebugContent>>,
    world_render_state: Option<ResMut<WorldRenderState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    let Some(screen_mode) = screen_mode else {
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

    let stepped = match step_runtime_if_needed(*screen_mode, &mut sim_state) {
        Ok(stepped) => stepped,
        Err(error) => {
            error!("live simulation step failed; shutting down app: {error}");
            app_exit.write(AppExit::error());
            return;
        }
    };
    if !stepped {
        return;
    }

    let Some(world) = sim_state.runtime.world() else {
        error!("live simulation step failed: world is missing while world_loaded=true");
        app_exit.write(AppExit::error());
        return;
    };
    let snapshot =
        match world_debug::build_world_render_snapshot(world, &world_debug_content.registry) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                error!("live simulation step failed while building render snapshot: {error}");
                app_exit.write(AppExit::error());
                return;
            }
        };
    world_render_state.update_world_snapshot(snapshot);
}

fn should_step_world_simulation(
    screen_mode: FluxScreenMode,
    world_loaded: bool,
    simulation_paused: bool,
) -> bool {
    screen_mode == FluxScreenMode::World && world_loaded && !simulation_paused
}

fn step_runtime_if_needed(
    screen_mode: FluxScreenMode,
    sim_state: &mut FluxSimState,
) -> Result<bool, SimError> {
    if !should_step_world_simulation(
        screen_mode,
        sim_state.world_loaded,
        sim_state.simulation_paused,
    ) {
        return Ok(false);
    }
    sim_state.runtime.step()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use flux_sim::{BackendPolicy, SimCommand, SimRuntime};

    use super::{should_step_world_simulation, step_runtime_if_needed};
    use crate::{FluxScreenMode, FluxSimState};

    fn create_world_sim_state() -> FluxSimState {
        let mut runtime =
            SimRuntime::new(Duration::from_millis(16), BackendPolicy::CpuOnly).expect("runtime");
        runtime
            .enqueue_command(SimCommand::CreateWorld {
                width: 8,
                height: 8,
                seed: 1,
            })
            .expect("enqueue");
        runtime.initialize().expect("initialize");
        FluxSimState {
            runtime,
            world_loaded: true,
            simulation_paused: false,
        }
    }

    #[test]
    fn only_world_mode_with_unpaused_loaded_world_can_step() {
        assert!(should_step_world_simulation(
            FluxScreenMode::World,
            true,
            false
        ));
        assert!(!should_step_world_simulation(
            FluxScreenMode::Menu,
            true,
            false
        ));
        assert!(!should_step_world_simulation(
            FluxScreenMode::World,
            false,
            false
        ));
        assert!(!should_step_world_simulation(
            FluxScreenMode::World,
            true,
            true
        ));
    }

    #[test]
    fn step_runtime_if_needed_advances_tick_when_enabled() {
        let mut sim_state = create_world_sim_state();
        let stepped =
            step_runtime_if_needed(FluxScreenMode::World, &mut sim_state).expect("step should run");

        assert!(stepped);
        assert_eq!(sim_state.runtime.tick_counter(), 1);
    }

    #[test]
    fn step_runtime_if_needed_skips_when_paused() {
        let mut sim_state = create_world_sim_state();
        sim_state.simulation_paused = true;

        let stepped =
            step_runtime_if_needed(FluxScreenMode::World, &mut sim_state).expect("no error");

        assert!(!stepped);
        assert_eq!(sim_state.runtime.tick_counter(), 0);
    }
}
