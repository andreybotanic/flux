use std::collections::BTreeSet;
use std::path::Path;

use bevy::app::AppExit;
use bevy::prelude::*;
use flux_core::NamespacedId;
use flux_render::WorldRenderState;
use flux_ui::{
    BuiltinUiActionDispatcher, ContainerLayout, UiMenuDefinition, UiMenuId, WidgetKind, WidgetNode,
};

use crate::app_state::{
    FluxScreenMode, FluxSimState, FluxUiButtonAction, FluxUiRoot, FluxUiState,
    FluxWorldDebugContent, UiButtonInteractionChanges, UiButtonPressed,
};
use crate::helpers::format_error_block;
use crate::input_actions::{
    ActionExecutionFlow, InputActionRegistry, default_input_action_registry, execute_binding_action,
};
use crate::input_bindings::default_input_bindings;

pub(crate) fn setup_flux_ui_runtime(mut commands: Commands) {
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

pub(crate) fn resolve_initial_menu(known_menus: &BTreeSet<UiMenuId>) -> Result<UiMenuId, String> {
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
pub(crate) fn emit_ui_button_press_events(
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
pub(crate) fn handle_ui_button_actions(
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

pub(crate) fn rebuild_flux_ui_if_needed(
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

#[cfg(test)]
mod tests {
    use super::*;

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
