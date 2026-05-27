use std::time::Duration;

use flux_core::{NamespacedId, PrototypeId};
use flux_scenario::{SetCameraPivotStep, SetCameraZoomStep};
use flux_ui::{
    BindingAction, BuiltinUiActionDispatcher, ContainerLayout, ContainerWidget, LocalizationKey,
    UiMenuDefinition, UiMenuId, UiRegistry, UiWidgetId, WidgetKind, WidgetNode,
};

use super::runtime::{
    append_visual_delay_after_step, append_visual_delay_after_step_if_needed, wait_deadline,
};
use super::validation::{
    ScenarioValidationError, ScenarioValidationState, simulation_ticks_for_delay,
    validate_scenario_steps,
};
use flux_scenario::{
    AssertUiExistsStep, ClickStep, LoadGameStep, OpenMenuStep, PauseSimulationStep, SaveGameStep,
    ScenarioDefinition, ScenarioStep, TakeScreenshotStep, WaitRealtimeStep, WaitSimulationTimeStep,
    WaitTicksStep,
};
use flux_ui::TextWidget;

fn localization(value: &str) -> LocalizationKey {
    LocalizationKey::parse(value).expect("valid localization key")
}

fn menu_id(value: &str) -> UiMenuId {
    UiMenuId(NamespacedId::parse(value).expect("menu id"))
}

fn widget_id(value: &str) -> UiWidgetId {
    UiWidgetId(NamespacedId::parse(value).expect("widget id"))
}

fn build_registry() -> UiRegistry {
    let mut registry = UiRegistry::new();
    let root = WidgetNode {
        id: widget_id("base:widget/main/root"),
        kind: WidgetKind::Container(ContainerWidget {
            layout: ContainerLayout::Vertical,
        }),
        children: vec![
            WidgetNode {
                id: widget_id("base:widget/main/run_world"),
                kind: WidgetKind::Button(flux_ui::ButtonWidget {
                    text: localization("$base.menu.main.run_world"),
                    action: BindingAction::RunWorld,
                }),
                children: Vec::new(),
            },
            WidgetNode {
                id: widget_id("base:widget/main/open_settings"),
                kind: WidgetKind::Button(flux_ui::ButtonWidget {
                    text: localization("$base.menu.main.settings"),
                    action: BindingAction::OpenMenu(menu_id("base:menu/settings")),
                }),
                children: Vec::new(),
            },
            WidgetNode {
                id: widget_id("base:widget/main/save_slot_a"),
                kind: WidgetKind::Button(flux_ui::ButtonWidget {
                    text: localization("$base.menu.main.save_slot_a"),
                    action: BindingAction::SaveGame("slot_a".to_owned()),
                }),
                children: Vec::new(),
            },
            WidgetNode {
                id: widget_id("base:widget/main/load_slot_a"),
                kind: WidgetKind::Button(flux_ui::ButtonWidget {
                    text: localization("$base.menu.main.load_slot_a"),
                    action: BindingAction::LoadGame("slot_a".to_owned()),
                }),
                children: Vec::new(),
            },
            WidgetNode {
                id: widget_id("base:widget/main/back"),
                kind: WidgetKind::Button(flux_ui::ButtonWidget {
                    text: localization("$base.menu.main.back"),
                    action: BindingAction::BackMenu,
                }),
                children: Vec::new(),
            },
        ],
    };
    registry
        .add_menu(
            UiMenuDefinition {
                id: menu_id("base:menu/main"),
                root,
            },
            flux_ui::UiSource {
                mod_id: "base".to_owned(),
                file: "mods/base/ui/menus/main.ron".to_owned(),
            },
        )
        .expect("main menu");

    let settings_root = WidgetNode {
        id: widget_id("base:widget/settings/root"),
        kind: WidgetKind::Container(ContainerWidget {
            layout: ContainerLayout::Vertical,
        }),
        children: vec![
            WidgetNode {
                id: widget_id("base:widget/settings/back"),
                kind: WidgetKind::Button(flux_ui::ButtonWidget {
                    text: localization("$base.menu.settings.back"),
                    action: BindingAction::BackMenu,
                }),
                children: Vec::new(),
            },
            WidgetNode {
                id: widget_id("base:widget/settings/title"),
                kind: WidgetKind::Text(TextWidget {
                    text: localization("$base.menu.settings.title"),
                }),
                children: Vec::new(),
            },
        ],
    };
    registry
        .add_menu(
            UiMenuDefinition {
                id: menu_id("base:menu/settings"),
                root: settings_root,
            },
            flux_ui::UiSource {
                mod_id: "base".to_owned(),
                file: "mods/base/ui/menus/settings.ron".to_owned(),
            },
        )
        .expect("settings menu");
    registry.freeze();
    registry
}

fn validation_state(registry: &UiRegistry) -> ScenarioValidationState {
    let known_menus = registry.menu_ids();
    ScenarioValidationState {
        world_loaded: false,
        sim_paused: false,
        world_open: false,
        dispatcher: BuiltinUiActionDispatcher::new(menu_id("base:menu/main")),
        known_menus,
    }
}

fn validate_single_step(step: ScenarioStep) -> ScenarioValidationError {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/invalid").expect("id"),
        steps: vec![step],
    };
    validate_scenario_steps(&scenario, &registry, &mut state).expect_err("must fail")
}

#[test]
fn rejects_wait_ticks_without_loaded_world() {
    let error = validate_single_step(ScenarioStep::WaitTicksStep(WaitTicksStep(1)));
    assert_eq!(error.step_kind, "WaitTicks");
}

#[test]
fn rejects_wait_simulation_time_without_loaded_world() {
    let error = validate_single_step(ScenarioStep::WaitSimulationTimeStep(
        WaitSimulationTimeStep { delay_ms: 1000 },
    ));
    assert_eq!(error.step_kind, "WaitSimulationTime");
}

#[test]
fn rejects_wait_realtime_when_not_paused() {
    let error = validate_single_step(ScenarioStep::WaitRealtimeStep(WaitRealtimeStep {
        delay_ms: 10,
    }));
    assert_eq!(error.step_kind, "WaitRealtime");
}

#[test]
fn rejects_resume_when_not_paused() {
    let error = validate_single_step(ScenarioStep::ResumeSimulationStep(
        flux_scenario::ResumeSimulationStep {},
    ));
    assert_eq!(error.step_kind, "ResumeSimulation");
}

#[test]
fn rejects_pause_without_loaded_world() {
    let error = validate_single_step(ScenarioStep::PauseSimulationStep(PauseSimulationStep {
        delay_ms: 0,
    }));
    assert_eq!(error.step_kind, "PauseSimulation");
}

#[test]
fn rejects_save_game_without_loaded_world() {
    let error = validate_single_step(ScenarioStep::SaveGameStep(SaveGameStep(
        "slot_a".to_owned(),
    )));
    assert_eq!(error.step_kind, "SaveGame");
}

#[test]
fn rejects_open_ui_with_unknown_menu() {
    let error = validate_single_step(ScenarioStep::OpenMenuStep(OpenMenuStep(menu_id(
        "base:menu/unknown",
    ))));
    assert_eq!(error.step_kind, "OpenMenu");
}

#[test]
fn load_game_step_enables_simulation_wait_steps() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/load_then_wait").expect("id"),
        steps: vec![
            ScenarioStep::LoadGameStep(LoadGameStep("slot_a".to_owned())),
            ScenarioStep::WaitTicksStep(WaitTicksStep(1)),
        ],
    };
    validate_scenario_steps(&scenario, &registry, &mut state).expect("scenario must validate");
}

#[test]
fn rejects_click_when_widget_not_visible_in_current_menu() {
    let error = validate_single_step(ScenarioStep::ClickStep(ClickStep(widget_id(
        "base:widget/settings/back",
    ))));
    assert_eq!(error.step_kind, "Click");
}

#[test]
fn rejects_assert_ui_exists_when_widget_not_visible_in_current_menu() {
    let error = validate_single_step(ScenarioStep::AssertUiExistsStep(AssertUiExistsStep(
        widget_id("base:widget/settings/back"),
    )));
    assert_eq!(error.step_kind, "AssertUiExists");
}

#[test]
fn rejects_camera_pivot_when_world_not_open() {
    let error = validate_single_step(ScenarioStep::SetCameraPivotStep(SetCameraPivotStep {
        x: 1,
        y: 1,
    }));
    assert_eq!(error.step_kind, "SetCameraPivot");
}

#[test]
fn rejects_camera_zoom_when_world_not_open() {
    let error = validate_single_step(ScenarioStep::SetCameraZoomStep(SetCameraZoomStep {
        zoom: 1.2,
    }));
    assert_eq!(error.step_kind, "SetCameraZoom");
}

#[test]
fn rejects_screenshot_with_directory_path() {
    let error = validate_single_step(ScenarioStep::TakeScreenshotStep(TakeScreenshotStep(
        "nested/screen.png".to_owned(),
    )));
    assert_eq!(error.step_kind, "TakeScreenshot");
}

#[test]
fn rejects_screenshot_with_non_png_extension() {
    let error = validate_single_step(ScenarioStep::TakeScreenshotStep(TakeScreenshotStep(
        "screen.jpg".to_owned(),
    )));
    assert_eq!(error.step_kind, "TakeScreenshot");
}

#[test]
fn rejects_click_on_non_button_widget() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/click_non_button").expect("id"),
        steps: vec![
            ScenarioStep::OpenMenuStep(OpenMenuStep(menu_id("base:menu/settings"))),
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/settings/title"))),
        ],
    };
    let error = validate_scenario_steps(&scenario, &registry, &mut state).expect_err("must fail");
    assert_eq!(error.step_kind, "Click");
}

#[test]
fn rejects_wait_ticks_when_world_is_paused_by_open_ui() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/invalid_wait_ticks_paused").expect("id"),
        steps: vec![
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/run_world"))),
            ScenarioStep::OpenMenuStep(OpenMenuStep(menu_id("base:menu/settings"))),
            ScenarioStep::WaitTicksStep(WaitTicksStep(1)),
        ],
    };
    let error = validate_scenario_steps(&scenario, &registry, &mut state).expect_err("must fail");
    assert_eq!(error.step_kind, "WaitTicks");
}

#[test]
fn rejects_wait_simulation_time_when_world_is_paused_by_open_ui() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/invalid_wait_time_paused").expect("id"),
        steps: vec![
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/run_world"))),
            ScenarioStep::OpenMenuStep(OpenMenuStep(menu_id("base:menu/settings"))),
            ScenarioStep::WaitSimulationTimeStep(WaitSimulationTimeStep { delay_ms: 32 }),
        ],
    };
    let error = validate_scenario_steps(&scenario, &registry, &mut state).expect_err("must fail");
    assert_eq!(error.step_kind, "WaitSimulationTime");
}

#[test]
fn rejects_pause_when_already_paused() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/invalid_pause_twice").expect("id"),
        steps: vec![
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/run_world"))),
            ScenarioStep::PauseSimulationStep(PauseSimulationStep { delay_ms: 0 }),
            ScenarioStep::PauseSimulationStep(PauseSimulationStep { delay_ms: 0 }),
        ],
    };
    let error = validate_scenario_steps(&scenario, &registry, &mut state).expect_err("must fail");
    assert_eq!(error.step_kind, "PauseSimulation");
}

#[test]
fn rejects_click_when_world_view_is_open_even_if_widget_exists_in_registry() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/invalid_click_world_open").expect("id"),
        steps: vec![
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/run_world"))),
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/open_settings"))),
        ],
    };
    let error = validate_scenario_steps(&scenario, &registry, &mut state).expect_err("must fail");
    assert_eq!(error.step_kind, "Click");
}

#[test]
fn rejects_assert_ui_exists_when_world_view_is_open_even_if_widget_exists_in_registry() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/invalid_assert_world_open").expect("id"),
        steps: vec![
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/run_world"))),
            ScenarioStep::AssertUiExistsStep(AssertUiExistsStep(widget_id(
                "base:widget/main/open_settings",
            ))),
        ],
    };
    let error = validate_scenario_steps(&scenario, &registry, &mut state).expect_err("must fail");
    assert_eq!(error.step_kind, "AssertUiExists");
}

#[test]
fn validates_happy_path_chain() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/happy").expect("id"),
        steps: vec![
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/run_world"))),
            ScenarioStep::WaitTicksStep(WaitTicksStep(3)),
            ScenarioStep::PauseSimulationStep(PauseSimulationStep { delay_ms: 0 }),
            ScenarioStep::WaitRealtimeStep(WaitRealtimeStep { delay_ms: 10 }),
            ScenarioStep::ResumeSimulationStep(flux_scenario::ResumeSimulationStep {}),
            ScenarioStep::WaitSimulationTimeStep(WaitSimulationTimeStep { delay_ms: 48 }),
            ScenarioStep::SetCameraPivotStep(SetCameraPivotStep { x: 5, y: 5 }),
            ScenarioStep::SetCameraZoomStep(SetCameraZoomStep { zoom: 1.5 }),
            ScenarioStep::OpenMenuStep(OpenMenuStep(menu_id("base:menu/settings"))),
            ScenarioStep::AssertUiExistsStep(AssertUiExistsStep(widget_id(
                "base:widget/settings/back",
            ))),
            ScenarioStep::TakeScreenshotStep(TakeScreenshotStep("screen.png".to_owned())),
        ],
    };

    validate_scenario_steps(&scenario, &registry, &mut state).expect("scenario must validate");
}

#[test]
fn rejects_resume_when_menu_ui_is_open() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/invalid_resume_menu").expect("id"),
        steps: vec![
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/run_world"))),
            ScenarioStep::OpenMenuStep(OpenMenuStep(menu_id("base:menu/settings"))),
            ScenarioStep::ResumeSimulationStep(flux_scenario::ResumeSimulationStep {}),
        ],
    };
    let error = validate_scenario_steps(&scenario, &registry, &mut state).expect_err("must fail");
    assert_eq!(error.step_kind, "ResumeSimulation");
}

#[test]
fn back_on_root_menu_returns_to_world_and_resumes_simulation() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/back_root_returns_world").expect("id"),
        steps: vec![
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/run_world"))),
            ScenarioStep::OpenMenuStep(OpenMenuStep(menu_id("base:menu/settings"))),
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/settings/back"))),
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/back"))),
            ScenarioStep::WaitTicksStep(WaitTicksStep(1)),
        ],
    };

    validate_scenario_steps(&scenario, &registry, &mut state).expect("scenario must validate");
}

#[test]
fn open_menu_main_after_world_does_not_duplicate_root_menu() {
    let registry = build_registry();
    let mut state = validation_state(&registry);
    let scenario = ScenarioDefinition {
        id: PrototypeId::parse("test_scenarios:scenario/open_main_no_duplicate").expect("id"),
        steps: vec![
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/run_world"))),
            ScenarioStep::OpenMenuStep(OpenMenuStep(menu_id("base:menu/main"))),
            ScenarioStep::ClickStep(ClickStep(widget_id("base:widget/main/back"))),
            ScenarioStep::WaitTicksStep(WaitTicksStep(1)),
        ],
    };

    validate_scenario_steps(&scenario, &registry, &mut state).expect("scenario must validate");
}

#[test]
fn simulation_time_uses_floor_tick_conversion() {
    let runtime = flux_sim::SimRuntime::new(Duration::from_millis(16)).expect("runtime");
    assert_eq!(simulation_ticks_for_delay(&runtime, 1000), 62);
    assert_eq!(simulation_ticks_for_delay(&runtime, 15), 0);
    assert_eq!(simulation_ticks_for_delay(&runtime, 16), 1);
}

#[test]
fn visual_delay_sets_wait_when_step_did_not_set_wait() {
    let now = Duration::from_millis(100);
    let actual = append_visual_delay_after_step(None, now, 50);
    assert_eq!(actual, Some(Duration::from_millis(150)));
}

#[test]
fn visual_delay_is_added_on_top_of_existing_step_wait() {
    let now = Duration::from_millis(100);
    let actual = append_visual_delay_after_step(Some(Duration::from_millis(300)), now, 50);
    assert_eq!(actual, Some(Duration::from_millis(350)));
}

#[test]
fn visual_delay_uses_now_when_existing_wait_is_already_elapsed() {
    let now = Duration::from_millis(100);
    let actual = append_visual_delay_after_step(Some(Duration::from_millis(80)), now, 50);
    assert_eq!(actual, Some(Duration::from_millis(150)));
}

#[test]
fn visual_delay_is_not_added_when_step_is_last() {
    let now = Duration::from_millis(100);
    let actual = append_visual_delay_after_step_if_needed(None, now, 50, false);
    assert_eq!(actual, None);
}

#[test]
fn visual_delay_is_added_when_more_steps_exist() {
    let now = Duration::from_millis(100);
    let actual =
        append_visual_delay_after_step_if_needed(Some(Duration::from_millis(300)), now, 50, true);
    assert_eq!(actual, Some(Duration::from_millis(350)));
}

#[test]
fn wait_deadline_adds_delay_without_overflow() {
    let now = Duration::from_secs(5);
    let deadline = wait_deadline(now, 250).expect("deadline");
    assert_eq!(deadline, Duration::from_millis(5_250));
}

#[test]
fn wait_deadline_returns_error_on_overflow() {
    let now = Duration::MAX;
    let error = wait_deadline(now, 1).expect_err("overflow must fail");
    assert!(error.contains("wait delay overflow"));
}
