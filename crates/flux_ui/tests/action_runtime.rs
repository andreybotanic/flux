use std::collections::BTreeSet;

use flux_ui::{
    BuiltinUiActionDispatcher, UiAction, UiActionContext, UiActionDispatcher, UiActionResult,
    UiMenuId,
};
use ron::{Options, extensions::Extensions};

#[test]
fn deserializes_builtin_actions_from_ron() {
    let options = Options::default().with_default_extension(Extensions::UNWRAP_VARIANT_NEWTYPES);

    let open_menu = options
        .from_str::<UiAction>("OpenMenu(\"base:menu/settings\")")
        .expect("open menu action should parse");
    assert_eq!(
        open_menu,
        UiAction::OpenMenu(UiMenuId(
            flux_core::NamespacedId::parse("base:menu/settings").expect("id")
        ))
    );

    let back = options
        .from_str::<UiAction>("BackMenu")
        .expect("back action should parse");
    assert_eq!(back, UiAction::BackMenu);

    let log = options
        .from_str::<UiAction>("DiagnosticLog(\"hello\")")
        .expect("log action should parse");
    assert_eq!(log, UiAction::DiagnosticLog("hello".to_owned()));

    let run_world = options
        .from_str::<UiAction>("RunWorld")
        .expect("run world action should parse");
    assert_eq!(run_world, UiAction::RunWorld);
}

#[test]
fn builtin_dispatcher_supports_open_back_and_log() {
    let main_menu = UiMenuId(flux_core::NamespacedId::parse("base:menu/main").expect("id"));
    let settings_menu = UiMenuId(flux_core::NamespacedId::parse("base:menu/settings").expect("id"));

    let mut dispatcher = BuiltinUiActionDispatcher::new(main_menu.clone());
    let mut known_menus = BTreeSet::new();
    known_menus.insert(main_menu.clone());
    known_menus.insert(settings_menu.clone());

    let mut logs = Vec::new();
    {
        let mut logger = |message: &str| logs.push(message.to_owned());
        let mut context = UiActionContext {
            known_menus: &known_menus,
            diagnostic_log: &mut logger,
        };

        let open_result = dispatcher
            .dispatch(&UiAction::OpenMenu(settings_menu.clone()), &mut context)
            .expect("open menu must succeed");
        assert_eq!(open_result, UiActionResult::MenuChanged);
        assert_eq!(dispatcher.menu_stack().current(), &settings_menu);
        assert_eq!(dispatcher.menu_stack().len(), 2);

        let back_result = dispatcher
            .dispatch(&UiAction::BackMenu, &mut context)
            .expect("back must succeed");
        assert_eq!(back_result, UiActionResult::MenuChanged);
        assert_eq!(dispatcher.menu_stack().current(), &main_menu);
        assert_eq!(dispatcher.menu_stack().len(), 1);

        let no_back_result = dispatcher
            .dispatch(&UiAction::BackMenu, &mut context)
            .expect("back on root menu should be noop");
        assert_eq!(no_back_result, UiActionResult::Noop);
        assert_eq!(dispatcher.menu_stack().current(), &main_menu);
        assert_eq!(dispatcher.menu_stack().len(), 1);

        let log_result = dispatcher
            .dispatch(&UiAction::DiagnosticLog("clicked".to_owned()), &mut context)
            .expect("log must succeed");
        assert_eq!(log_result, UiActionResult::Noop);

        let run_world_result = dispatcher
            .dispatch(&UiAction::RunWorld, &mut context)
            .expect("run world action must succeed");
        assert_eq!(run_world_result, UiActionResult::RunWorldRequested);
    }
    assert_eq!(logs, vec!["clicked".to_owned()]);
}
