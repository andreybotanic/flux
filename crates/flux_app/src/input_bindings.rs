use std::collections::HashMap;

use bevy::app::AppExit;
use bevy::prelude::*;
use flux_core::NamespacedId;
use flux_render::WorldRenderState;
use flux_ui::{BindingAction, UiMenuId};

use crate::input_actions::{ActionExecutionFlow, InputActionRegistry, execute_binding_action};
use crate::{FluxScreenMode, FluxSimState, FluxUiState, FluxWorldDebugContent};

#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyChord {
    primary: KeyCode,
    modifiers: Vec<KeyCode>,
}

impl KeyChord {
    fn single(primary: KeyCode) -> Self {
        Self {
            primary,
            modifiers: Vec::new(),
        }
    }

    fn matches(&self, keyboard: &ButtonInput<KeyCode>) -> bool {
        keyboard.just_pressed(self.primary)
            && self
                .modifiers
                .iter()
                .all(|modifier| keyboard.pressed(*modifier))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyBinding {
    chord: KeyChord,
    action: BindingAction,
}

#[derive(Resource, Debug, Clone)]
pub(crate) struct FluxInputBindings {
    bindings_by_primary: HashMap<KeyCode, Vec<KeyBinding>>,
}

impl FluxInputBindings {
    fn from_bindings(bindings: Vec<KeyBinding>) -> Self {
        let mut bindings_by_primary: HashMap<KeyCode, Vec<KeyBinding>> = HashMap::new();
        for binding in bindings {
            bindings_by_primary
                .entry(binding.chord.primary)
                .or_default()
                .push(binding);
        }
        Self {
            bindings_by_primary,
        }
    }

    fn bindings_for_primary(&self, primary: KeyCode) -> Option<&[KeyBinding]> {
        self.bindings_by_primary.get(&primary).map(Vec::as_slice)
    }
}

pub(super) fn default_input_bindings() -> FluxInputBindings {
    FluxInputBindings::from_bindings(vec![
        KeyBinding {
            chord: KeyChord::single(KeyCode::Escape),
            action: BindingAction::OpenMenu(UiMenuId(
                NamespacedId::parse("base:menu/main").expect("valid menu id"),
            )),
        },
        KeyBinding {
            chord: KeyChord::single(KeyCode::Space),
            action: BindingAction::ToggleSimulation,
        },
    ])
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_input_bindings(
    keyboard: Res<ButtonInput<KeyCode>>,
    bindings: Option<Res<FluxInputBindings>>,
    action_registry: Option<Res<InputActionRegistry>>,
    ui_state: Option<ResMut<FluxUiState>>,
    screen_mode: Option<ResMut<FluxScreenMode>>,
    sim_state: Option<ResMut<FluxSimState>>,
    world_debug_content: Option<Res<FluxWorldDebugContent>>,
    world_render_state: Option<ResMut<WorldRenderState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    let Some(bindings) = bindings else {
        return;
    };
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
    let Some(action) = resolve_triggered_action(&keyboard, &bindings) else {
        return;
    };

    if execute_binding_action(
        &action,
        &action_registry,
        &mut ui_state,
        &mut screen_mode,
        &mut sim_state,
        &world_debug_content,
        &mut world_render_state,
    ) == ActionExecutionFlow::Stop
    {
        app_exit.write(AppExit::error());
    }
}

fn resolve_triggered_action(
    keyboard: &ButtonInput<KeyCode>,
    bindings: &FluxInputBindings,
) -> Option<BindingAction> {
    for primary in keyboard.get_just_pressed() {
        let Some(primary_bindings) = bindings.bindings_for_primary(*primary) else {
            continue;
        };
        if let Some(binding) = primary_bindings
            .iter()
            .find(|binding| binding.chord.matches(keyboard))
        {
            return Some(binding.action.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_chord_matches_single_key_press() {
        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::Escape);

        let chord = KeyChord::single(KeyCode::Escape);
        assert!(chord.matches(&keyboard));
    }

    #[test]
    fn key_chord_requires_all_modifiers() {
        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::ControlLeft);
        keyboard.press(KeyCode::KeyM);

        let chord = KeyChord {
            primary: KeyCode::KeyM,
            modifiers: vec![KeyCode::ControlLeft],
        };
        assert!(chord.matches(&keyboard));

        let missing_shift_chord = KeyChord {
            primary: KeyCode::KeyM,
            modifiers: vec![KeyCode::ControlLeft, KeyCode::ShiftLeft],
        };
        assert!(!missing_shift_chord.matches(&keyboard));
    }

    #[test]
    fn default_input_bindings_include_escape_and_space() {
        let bindings = default_input_bindings();
        let main_menu = UiMenuId(NamespacedId::parse("base:menu/main").expect("id"));

        assert!(
            bindings
                .bindings_for_primary(KeyCode::Escape)
                .expect("escape binding")
                .iter()
                .any(|binding| {
                    binding.chord == KeyChord::single(KeyCode::Escape)
                        && binding.action == BindingAction::OpenMenu(main_menu.clone())
                })
        );
        assert!(
            bindings
                .bindings_for_primary(KeyCode::Space)
                .expect("space binding")
                .iter()
                .any(|binding| {
                    binding.chord == KeyChord::single(KeyCode::Space)
                        && binding.action == BindingAction::ToggleSimulation
                })
        );
    }

    #[test]
    fn lookup_uses_primary_key_index() {
        let bindings = FluxInputBindings::from_bindings(vec![
            KeyBinding {
                chord: KeyChord::single(KeyCode::Escape),
                action: BindingAction::ToggleSimulation,
            },
            KeyBinding {
                chord: KeyChord::single(KeyCode::Space),
                action: BindingAction::ToggleSimulation,
            },
        ]);

        assert_eq!(
            bindings
                .bindings_for_primary(KeyCode::Escape)
                .expect("escape binding")
                .len(),
            1
        );
        assert!(bindings.bindings_for_primary(KeyCode::Enter).is_none());
    }
}
