# S10 — UI registry MVP

## Depends on

- S04

## Можно выполнять параллельно с

- S05
- S06
- S08
- S09

## Цель этапа

Создать декларативный UI registry и первые extension points без сложного UI.


## Требования к реализации

- Создать crate `flux_ui`.
- Добавить:
  - `UiRegistry`;
  - `UiPanelId`;
  - `UiActionId`;
  - `UiExtensionPointId`;
  - widgets `Panel`, `Label`, `Button`;
  - operation `AppendChild`.
- UI definitions загружаются из модов.
- Мод не должен напрямую spawn-ить Bevy UI entities.

## Ручная проверка

1. Запустить app.
2. Увидеть минимальное меню/панель.
3. Добавить UI extension из test mod.
4. Убедиться, что новая кнопка появилась.


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check_plan_index.py
```

## Definition of Done

- Реализована только зона ответственности этапа.
- Все automated checks проходят.
- Выполнена ручная проверка из этого документа.
- Нет изменений вне зоны ответственности без объяснения.
- Отчет этапа заполнен по `docs/STAGE_COMPLETION_REPORT_TEMPLATE.md`.
