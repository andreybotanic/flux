use bevy::log::{error, info};
use bevy::prelude::Resource;
use flux_render::WorldRenderState;
use flux_sim::SimCommand;
use flux_ui::{BindingAction, UiMenuId};

use crate::{FluxScreenMode, FluxSimState, FluxUiState, FluxWorldDebugContent, world_debug};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionExecutionFlow {
    Continue,
    Stop,
}

pub(crate) struct ActionExecutionContext<'a> {
    ui_state: &'a mut FluxUiState,
    screen_mode: &'a mut FluxScreenMode,
    sim_state: &'a mut FluxSimState,
    world_debug_content: &'a FluxWorldDebugContent,
    world_render_state: &'a mut WorldRenderState,
}

trait ExecutableInputAction {
    fn execute(&self, context: &mut ActionExecutionContext<'_>) -> ActionExecutionFlow;
}

#[derive(Resource, Default)]
pub(crate) struct InputActionRegistry;

impl InputActionRegistry {
    fn execute(
        &self,
        action: &BindingAction,
        context: &mut ActionExecutionContext<'_>,
    ) -> ActionExecutionFlow {
        ExecutableInputAction::execute(action, context)
    }
}

impl ExecutableInputAction for BindingAction {
    fn execute(&self, context: &mut ActionExecutionContext<'_>) -> ActionExecutionFlow {
        match self {
            BindingAction::OpenMenu(menu_id) => OpenMenuAction { menu_id }.execute(context),
            BindingAction::BackMenu => BackMenuAction.execute(context),
            BindingAction::RunWorld => RunWorldAction.execute(context),
            BindingAction::DiagnosticLog(message) => {
                DiagnosticLogAction { message }.execute(context)
            }
            BindingAction::ToggleSimulation => ToggleSimulationAction.execute(context),
        }
    }
}

struct OpenMenuAction<'a> {
    menu_id: &'a UiMenuId,
}

impl ExecutableInputAction for OpenMenuAction<'_> {
    fn execute(&self, context: &mut ActionExecutionContext<'_>) -> ActionExecutionFlow {
        if context.ui_state.dispatcher.menu_stack().current() == self.menu_id {
            return ActionExecutionFlow::Continue;
        }
        if let Err(error) = context
            .ui_state
            .dispatcher
            .open_menu(self.menu_id, &context.ui_state.known_menus)
        {
            error!("ui action dispatch failed: {error}");
            return ActionExecutionFlow::Continue;
        }
        context.ui_state.needs_rebuild = true;
        *context.screen_mode = FluxScreenMode::Menu;
        context.sim_state.simulation_paused = true;
        ActionExecutionFlow::Continue
    }
}

struct BackMenuAction;

impl ExecutableInputAction for BackMenuAction {
    fn execute(&self, context: &mut ActionExecutionContext<'_>) -> ActionExecutionFlow {
        if context.ui_state.dispatcher.back_menu() {
            context.ui_state.needs_rebuild = true;
            *context.screen_mode = FluxScreenMode::Menu;
            context.sim_state.simulation_paused = true;
            return ActionExecutionFlow::Continue;
        }
        if context.sim_state.world_loaded {
            *context.screen_mode = FluxScreenMode::World;
            context.ui_state.needs_rebuild = false;
            context.sim_state.simulation_paused = false;
        }
        ActionExecutionFlow::Continue
    }
}

struct DiagnosticLogAction<'a> {
    message: &'a str,
}

impl ExecutableInputAction for DiagnosticLogAction<'_> {
    fn execute(&self, _context: &mut ActionExecutionContext<'_>) -> ActionExecutionFlow {
        info!("ui action log: {}", self.message);
        ActionExecutionFlow::Continue
    }
}

struct RunWorldAction;

impl ExecutableInputAction for RunWorldAction {
    fn execute(&self, context: &mut ActionExecutionContext<'_>) -> ActionExecutionFlow {
        if let Err(error) = context
            .sim_state
            .runtime
            .enqueue_command(SimCommand::CreateWorld {
                width: 64,
                height: 64,
                seed: 1,
            })
        {
            error!("RunWorld failed to enqueue CreateWorld command: {error}");
            return ActionExecutionFlow::Continue;
        }

        if let Err(error) = context.sim_state.runtime.initialize() {
            error!("RunWorld failed to initialize simulation runtime: {error}");
            return ActionExecutionFlow::Continue;
        }

        let Some(world) = context.sim_state.runtime.world_mut() else {
            error!("RunWorld failed: world is missing after initialization");
            return ActionExecutionFlow::Continue;
        };

        if let Err(error) =
            world_debug::populate_world_debug_mvp(world, &context.world_debug_content.registry)
        {
            error!(
                "RunWorld temporary S11B world population failed (continuing with partial world): {error}"
            );
        }

        let snapshot = match world_debug::build_world_render_snapshot(
            world,
            &context.world_debug_content.registry,
        ) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                error!(
                    "RunWorld failed while building render snapshot; shutting down app: {error}"
                );
                return ActionExecutionFlow::Stop;
            }
        };

        let world_size = world.size();
        context
            .world_render_state
            .show_world(world_size, 1.0, snapshot);
        *context.screen_mode = FluxScreenMode::World;
        context.ui_state.needs_rebuild = false;
        context.sim_state.world_loaded = true;
        context.sim_state.simulation_paused = false;
        info!(
            "world view activated: size={}x{} seed={}",
            world_size.width,
            world_size.height,
            context.sim_state.runtime.world_seed().unwrap_or_default()
        );
        ActionExecutionFlow::Continue
    }
}

struct ToggleSimulationAction;

impl ExecutableInputAction for ToggleSimulationAction {
    fn execute(&self, context: &mut ActionExecutionContext<'_>) -> ActionExecutionFlow {
        if context.sim_state.world_loaded {
            context.sim_state.simulation_paused = !context.sim_state.simulation_paused;
        } else {
            info!("ToggleSimulation ignored: world is not loaded");
        }
        ActionExecutionFlow::Continue
    }
}

pub(crate) fn default_input_action_registry() -> InputActionRegistry {
    InputActionRegistry
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_binding_action(
    action: &BindingAction,
    registry: &InputActionRegistry,
    ui_state: &mut FluxUiState,
    screen_mode: &mut FluxScreenMode,
    sim_state: &mut FluxSimState,
    world_debug_content: &FluxWorldDebugContent,
    world_render_state: &mut WorldRenderState,
) -> ActionExecutionFlow {
    let mut context = ActionExecutionContext {
        ui_state,
        screen_mode,
        sim_state,
        world_debug_content,
        world_render_state,
    };
    registry.execute(action, &mut context)
}
