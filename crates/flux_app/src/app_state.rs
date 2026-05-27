use std::collections::BTreeSet;

use bevy::prelude::{Button, Changed, Component, Interaction, Message, Query, Resource, With};
use flux_sim::{BackendPolicy, SimRuntime};
use flux_ui::{BindingAction, BuiltinUiActionDispatcher, UiMenuId};

#[derive(Component)]
pub(crate) struct FluxUiRoot;

#[derive(Component, Clone)]
pub(crate) struct FluxUiButtonAction(pub(crate) BindingAction);

#[derive(Message, Clone)]
pub(crate) struct UiButtonPressed {
    pub(crate) action: BindingAction,
}

#[derive(Component)]
pub(crate) struct FluxUiCamera;

#[derive(Resource)]
pub(crate) struct FluxUiState {
    pub(crate) registry: flux_ui::UiRegistry,
    pub(crate) dispatcher: BuiltinUiActionDispatcher,
    pub(crate) known_menus: BTreeSet<UiMenuId>,
    pub(crate) needs_rebuild: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub(crate) enum FluxScreenMode {
    Menu,
    World,
}

#[derive(Resource)]
pub(crate) struct FluxSimState {
    pub(crate) runtime: SimRuntime,
    pub(crate) world_loaded: bool,
    pub(crate) simulation_paused: bool,
}

#[derive(Resource)]
pub(crate) struct FluxBackendPolicy(pub(crate) BackendPolicy);

#[derive(Resource)]
pub(crate) struct FluxWorldDebugContent {
    // S11B temporary: content snapshot used to seed/debug-render world layers.
    pub(crate) registry: flux_content::ContentRegistry,
}

pub(crate) type UiButtonInteractionChanges<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static FluxUiButtonAction),
    (Changed<Interaction>, With<Button>),
>;
