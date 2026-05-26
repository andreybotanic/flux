use flux_scenario::{ScenarioDefinition, ScenarioStep};
use flux_ui::{
    BindingAction, BuiltinUiActionDispatcher, UiMenuDefinition, UiMenuId, UiRegistry, WidgetKind,
};

use super::runtime::find_widget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScenarioValidationError {
    pub(super) step_index: usize,
    pub(super) step_kind: &'static str,
    pub(super) reason: String,
}

#[derive(Debug, Clone)]
pub(super) struct ScenarioValidationState {
    pub(super) world_loaded: bool,
    pub(super) sim_paused: bool,
    pub(super) world_open: bool,
    pub(super) dispatcher: BuiltinUiActionDispatcher,
    pub(super) known_menus: std::collections::BTreeSet<UiMenuId>,
}

pub(super) fn validate_scenario_steps(
    scenario: &ScenarioDefinition,
    registry: &UiRegistry,
    state: &mut ScenarioValidationState,
) -> Result<(), ScenarioValidationError> {
    for (step_index, step) in scenario.steps.iter().enumerate() {
        validate_step(step_index, step, registry, state)?;
    }
    Ok(())
}

fn validate_step(
    step_index: usize,
    step: &ScenarioStep,
    registry: &UiRegistry,
    state: &mut ScenarioValidationState,
) -> Result<(), ScenarioValidationError> {
    match step {
        ScenarioStep::LogStep(_) => Ok(()),
        ScenarioStep::CreateWorldStep(_) => {
            state.world_loaded = true;
            state.world_open = true;
            state.sim_paused = false;
            Ok(())
        }
        ScenarioStep::WaitTicksStep(_) => ensure_sim_time_allowed(step_index, step, state),
        ScenarioStep::WaitSimulationTimeStep(_) => ensure_sim_time_allowed(step_index, step, state),
        ScenarioStep::AssertTickStep(_) => Ok(()),
        ScenarioStep::WaitRealtimeStep(_) => {
            if state.sim_paused {
                Ok(())
            } else {
                Err(validation_error(
                    step_index,
                    step,
                    "WaitRealtime is allowed only while simulation is paused",
                ))
            }
        }
        ScenarioStep::PauseSimulationStep(_) => {
            if !state.world_loaded {
                return Err(validation_error(
                    step_index,
                    step,
                    "PauseSimulation requires loaded world",
                ));
            }
            if state.sim_paused {
                return Err(validation_error(
                    step_index,
                    step,
                    "PauseSimulation is invalid while simulation is already paused",
                ));
            }
            let ScenarioStep::PauseSimulationStep(step) = step else {
                unreachable!("matched by caller");
            };
            state.sim_paused = step.delay_ms == 0;
            Ok(())
        }
        ScenarioStep::ResumeSimulationStep(_) => {
            if !state.world_loaded {
                return Err(validation_error(
                    step_index,
                    step,
                    "ResumeSimulation requires loaded world",
                ));
            }
            if !state.sim_paused {
                return Err(validation_error(
                    step_index,
                    step,
                    "ResumeSimulation requires simulation to be paused",
                ));
            }
            if !state.world_open {
                return Err(validation_error(
                    step_index,
                    step,
                    "ResumeSimulation is not available while menu UI is open",
                ));
            }
            state.sim_paused = false;
            Ok(())
        }
        ScenarioStep::SetCameraPivotStep(_) | ScenarioStep::SetCameraZoomStep(_) => {
            if !state.world_open {
                return Err(validation_error(
                    step_index,
                    step,
                    "camera control is allowed only when world view is open",
                ));
            }
            Ok(())
        }
        ScenarioStep::OpenMenuStep(step) => {
            if !state.known_menus.contains(&step.0) {
                return Err(validation_error_kind(
                    step_index,
                    "OpenMenu",
                    format!("menu `{}` is not registered", step.0),
                ));
            }
            state
                .dispatcher
                .open_menu(&step.0, &state.known_menus)
                .map_err(|error| {
                    validation_error_kind(step_index, "OpenMenu", format!("{error}"))
                })?;
            state.world_open = false;
            if state.world_loaded {
                state.sim_paused = true;
            }
            Ok(())
        }
        ScenarioStep::ClickStep(step) => {
            if state.world_open {
                return Err(validation_error_kind(
                    step_index,
                    "Click",
                    "Click is allowed only when UI menu is open",
                ));
            }
            let menu =
                current_menu_by_dispatcher(registry, &state.dispatcher).ok_or_else(|| {
                    validation_error_kind(step_index, "Click", "current menu is not available")
                })?;
            let node = find_widget(menu, &step.0).ok_or_else(|| {
                validation_error_kind(
                    step_index,
                    "Click",
                    format!("widget `{}` is not available in current menu", step.0),
                )
            })?;
            let WidgetKind::Button(button) = &node.kind else {
                return Err(validation_error_kind(
                    step_index,
                    "Click",
                    format!("widget `{}` is not a button", step.0),
                ));
            };
            apply_click_action_for_validation(step_index, button.action.clone(), state)?;
            Ok(())
        }
        ScenarioStep::TakeScreenshotStep(step) => {
            if !is_filename_only(&step.0) {
                return Err(validation_error_kind(
                    step_index,
                    "TakeScreenshot",
                    "TakeScreenshot path must be a file name without directories",
                ));
            }
            if !step.0.to_ascii_lowercase().ends_with(".png") {
                return Err(validation_error_kind(
                    step_index,
                    "TakeScreenshot",
                    "TakeScreenshot file name must end with .png",
                ));
            }
            Ok(())
        }
        ScenarioStep::AssertUiExistsStep(step) => {
            if state.world_open {
                return Err(validation_error_kind(
                    step_index,
                    "AssertUiExists",
                    "AssertUiExists is allowed only when UI menu is open",
                ));
            }
            let menu =
                current_menu_by_dispatcher(registry, &state.dispatcher).ok_or_else(|| {
                    validation_error_kind(
                        step_index,
                        "AssertUiExists",
                        "current menu is not available",
                    )
                })?;
            if find_widget(menu, &step.0).is_none() {
                return Err(validation_error_kind(
                    step_index,
                    "AssertUiExists",
                    format!("widget `{}` is not available in current menu", step.0),
                ));
            }
            Ok(())
        }
    }
}

fn ensure_sim_time_allowed(
    step_index: usize,
    step: &ScenarioStep,
    state: &ScenarioValidationState,
) -> Result<(), ScenarioValidationError> {
    if !state.world_loaded {
        return Err(validation_error(
            step_index,
            step,
            "simulation wait is allowed only when world is loaded",
        ));
    }
    if !state.world_open {
        return Err(validation_error(
            step_index,
            step,
            "simulation wait is allowed only when world view is open",
        ));
    }
    if state.sim_paused {
        return Err(validation_error(
            step_index,
            step,
            "simulation wait is forbidden while simulation is paused",
        ));
    }
    Ok(())
}

fn apply_click_action_for_validation(
    step_index: usize,
    action: BindingAction,
    state: &mut ScenarioValidationState,
) -> Result<(), ScenarioValidationError> {
    match action {
        BindingAction::OpenMenu(menu_id) => {
            state
                .dispatcher
                .open_menu(&menu_id, &state.known_menus)
                .map_err(|error| validation_error_kind(step_index, "Click", format!("{error}")))?;
            state.world_open = false;
            if state.world_loaded {
                state.sim_paused = true;
            }
        }
        BindingAction::BackMenu => {
            if state.dispatcher.back_menu() {
                state.world_open = false;
                if state.world_loaded {
                    state.sim_paused = true;
                }
            } else if state.world_loaded {
                state.world_open = true;
                state.sim_paused = false;
            }
        }
        BindingAction::DiagnosticLog(_) => {}
        BindingAction::RunWorld => {
            state.world_loaded = true;
            state.world_open = true;
            state.sim_paused = false;
        }
        BindingAction::ToggleSimulation => {
            if state.world_loaded {
                state.sim_paused = !state.sim_paused;
            }
        }
    }
    Ok(())
}

fn current_menu_by_dispatcher<'a>(
    registry: &'a UiRegistry,
    dispatcher: &BuiltinUiActionDispatcher,
) -> Option<&'a UiMenuDefinition> {
    let current = dispatcher.menu_stack().current();
    registry.menu(current)
}

pub(super) fn simulation_ticks_for_delay(runtime: &flux_sim::SimRuntime, delay_ms: u64) -> u64 {
    let step_ms = runtime.fixed_tick().step().as_millis() as u64;
    if step_ms == 0 {
        return 0;
    }
    delay_ms / step_ms
}

fn is_filename_only(value: &str) -> bool {
    let path = std::path::Path::new(value);
    path.file_name().is_some()
        && path.components().count() == 1
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains(':')
}

fn validation_error(
    step_index: usize,
    step: &ScenarioStep,
    reason: impl Into<String>,
) -> ScenarioValidationError {
    ScenarioValidationError {
        step_index,
        step_kind: step.kind(),
        reason: reason.into(),
    }
}

fn validation_error_kind(
    step_index: usize,
    step_kind: &'static str,
    reason: impl Into<String>,
) -> ScenarioValidationError {
    ScenarioValidationError {
        step_index,
        step_kind,
        reason: reason.into(),
    }
}
