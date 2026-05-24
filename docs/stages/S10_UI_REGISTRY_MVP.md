# S10 — UI registry MVP

## Depends on

- S04

## Можно выполнять параллельно с

- S05
- S06
- S08
- S09

## Цель этапа

Создать расширяемую декларативную UI-систему, где Menu — первый реализованный host type, но модель не зашита только под меню.
`mod -> UI definitions -> UiRegistry -> validated widget tree -> Bevy adapter`


## Архитектурное требование

В flux_ui должна быть общая модель UI-host:
```rust
pub enum UiDefinition {
    Menu(UiMenuDefinition),
    // future: Window(...),
    // future: Panel(...),
    // future: Overlay(...),
}
```
В S09 реализуется только: `UiDefinition::Menu`

Но код не должен называться/строиться так, будто весь UI всегда является меню.

## Реализация

### Host type
```rust
pub struct UiMenuDefinition {
    pub id: UiMenuId,
    pub root: WidgetNode,
}
```

### ID-типы
```rust
pub struct UiMenuId(pub NamespacedId);
pub struct UiWidgetId(pub NamespacedId);
pub struct UiActionId(pub NamespacedId);
pub struct UiExtensionPointId(pub NamespacedId);
```

### Widgets
```rust
pub enum WidgetKind {
    Container(ContainerWidget),
    Text(TextWidget),
    Button(ButtonWidget),
    ExtensionPoint(ExtensionPointWidget),
}

pub struct WidgetNode {
    pub id: UiWidgetId,
    pub kind: WidgetKind,
    pub children: Vec<WidgetNode>,
}
```

### Actions
```rust
pub enum UiAction {
    OpenMenu(UiMenuId),
    Back,
    DiagnosticLog(String),
}
```

### ExtensionPoint
UiExtensionPoint — стабильный слот внутри widget tree.

Моды могут добавлять элементы только в extension points:
```ron
UiExtension(
    target: "base:ui_ext/main_menu/actions",
    operation: AppendChild,
    child: Button(
        id: "example_mod:widget/main_menu/debug",
        text: "$example_mod.debug",
        action: OpenMenu("example_mod:menu/debug"),
    ),
)
```
Запрещено использовать UiMenuId как extension target.

## RON-примеры

### Создание нового меню
```ron
UiMenu(
    id: "base:menu/main",
    root: Container(
        id: "base:widget/main_menu/root",
        layout: Vertical,
        children: [
            Text(
                id: "base:widget/main_menu/title",
                text: "$base.menu.main.title",
            ),
            Button(
                id: "base:widget/main_menu/settings",
                text: "$base.menu.settings",
                action: OpenMenu("base:menu/settings"),
            ),
            ExtensionPoint(
                id: "base:widget/main_menu/actions_slot",
                extension_point: "base:ui_ext/main_menu/actions",
            ),
        ],
    ),
)
```

### Another menu from mod
```ron
UiMenu(
    id: "example_mod:menu/debug",
    root: Container(
        id: "example_mod:widget/debug/root",
        layout: Vertical,
        children: [
            Text(
                id: "example_mod:widget/debug/title",
                text: "$example_mod.debug.title",
            ),

            Button(
                id: "example_mod:widget/debug/back",
                text: "$example_mod.back",
                action: OpenMenu("base:menu/main"),
            ),
        ],
    ),
)
```

### Расширение существующего меню
```ron
UiExtension(
    target: "base:ui_ext/main_menu/actions",
    operation: AppendChild,
    child: Button(
        id: "example_mod:widget/main_menu/debug_button",
        text: "$example_mod.debug",
        action: OpenMenu("example_mod:menu/debug"),
    ),
)
```

То есть:
```
UiMenu      -> объявляет UI
ExtensionPoint -> объявляет slot
UiExtension -> добавляет widget в slot
```

## Кнопки и действия

Кнопка содержит UiActionId.
```rust
pub struct ButtonWidget {
    pub text: LocalizationKey,
    pub action: UiActionId,
}

pub struct UiActionId(pub NamespacedId);
```

Кнопка не содержит callback, функцию или enum действия.

При нажатии кнопки runtime отправляет UiActionId в UiActionDispatcher.

```rust
pub trait UiActionDispatcher {
    fn dispatch(
        &mut self,
        action: &UiActionId,
        context: &UiActionContext,
    ) -> UiActionResult;
}
```

В S09 достаточно реализовать builtin action handlers для:
- открытия другого меню
- возврат в предыдущее меню
- diagnostic logging

Для реализации функционала "возврат в предыдущее меню" все меню должны открываться в стэк. Тогда возврат должен просто удалить вершину стэка и показать меню, которое после этого оказалось на вершине.

Action system не должна быть menu-only. В будущем actions должны позволять:
- открывать окна;
- отправлять gameplay commands;
- запускать сценарии;
- вызывать код модов;
- вызывать WASM handlers;
- выполнять другие runtime-действия.

Пример кнопки:
```ron
Button(
    id: "base:widget/main_menu/settings",
    text: "$base.menu.settings",
    action: "base:action/open_settings_menu",
)
```

Пример регистрации builtin action:
```rust
dispatcher.register(
    "base:action/open_settings_menu",
    BuiltinAction::OpenMenu(
        "base:menu/settings",
    ),
);
```

## Bevy adapter

В S09 adapter обязан уметь только: `UiMenuDefinition -> Bevy UI entities`

Но интерфейс лучше назвать нейтрально:
```
UiRuntime
UiPresenter
UiHostRuntime
```

## Ручная проверка

1. Запустить app.
2. Увидеть базовое меню.
3. Добавить мод, добавляющий новое меню и расширяющий текущее меню новой кнопкой, которая открывает новое меню.
4. Убедиться, что новая кнопка появилась и она открывает новое меню.


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Definition of Done
- flux_ui создан.
- Есть расширяемый UiDefinition, но реализован только Menu.
- Моды могут создавать новые меню.
- Моды могут добавлять widgets в extension points.
- Кнопки поддерживают все доступные действия.
- OpenMenu реально переключает текущее меню.
- Архитектура не menu-only.
- Все проверки проходят.
