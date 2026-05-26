use std::collections::BTreeSet;

use flux_ui::{BindingAction, BuiltinUiActionDispatcher, UiAction, UiMenuId};
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

    assert!(
        options
            .from_str::<UiAction>("DiagnosticLog(\"hello\")")
            .is_err()
    );
    assert!(options.from_str::<UiAction>("RunWorld").is_err());
    assert!(options.from_str::<UiAction>("ToggleSimulation").is_err());

    let log = options
        .from_str::<BindingAction>("DiagnosticLog(\"hello\")")
        .expect("button log action should parse");
    assert_eq!(log, BindingAction::DiagnosticLog("hello".to_owned()));

    let run_world = options
        .from_str::<BindingAction>("RunWorld")
        .expect("button run world action should parse");
    assert_eq!(run_world, BindingAction::RunWorld);
}

#[test]
fn builtin_dispatcher_supports_open_back_and_log() {
    let main_menu = UiMenuId(flux_core::NamespacedId::parse("base:menu/main").expect("id"));
    let settings_menu = UiMenuId(flux_core::NamespacedId::parse("base:menu/settings").expect("id"));

    let mut dispatcher = BuiltinUiActionDispatcher::new(main_menu.clone());
    let mut known_menus = BTreeSet::new();
    known_menus.insert(main_menu.clone());
    known_menus.insert(settings_menu.clone());

    dispatcher
        .open_menu(&settings_menu, &known_menus)
        .expect("open menu must succeed");
    assert_eq!(dispatcher.menu_stack().current(), &settings_menu);
    assert_eq!(dispatcher.menu_stack().len(), 2);

    let back_result = dispatcher.back_menu();
    assert!(back_result);
    assert_eq!(dispatcher.menu_stack().current(), &main_menu);
    assert_eq!(dispatcher.menu_stack().len(), 1);

    let no_back_result = dispatcher.back_menu();
    assert!(!no_back_result);
    assert_eq!(dispatcher.menu_stack().current(), &main_menu);
    assert_eq!(dispatcher.menu_stack().len(), 1);

    dispatcher
        .open_menu(&settings_menu, &known_menus)
        .expect("open menu must succeed");
    assert_eq!(dispatcher.menu_stack().len(), 2);
    dispatcher.reset_menu_stack_to_root();
    assert_eq!(dispatcher.menu_stack().current(), &main_menu);
    assert_eq!(dispatcher.menu_stack().len(), 1);
}
