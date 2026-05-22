# S18 — DLC as official mod proof

## Depends on

- S03
- S04
- S05

## Можно выполнять параллельно с

- S17

## Цель этапа

Доказать, что DLC является обычным официальным модом, а не отдельной hardcode-системой.

## Roadmap revision note

Зависимости обновлены под новую нумерацию.


## Требования к реализации

- Создать `mods/dlc_test`.
- DLC должен иметь обычный manifest и content.
- Добавить stub entitlement check через config/CLI.
- Disabled DLC не монтирует контент.
- Enabled DLC добавляет контент в registry.

## Ручная проверка

1. Запустить content summary без DLC.
2. Убедиться, что DLC-прототипа нет.
3. Запустить content summary с DLC.
4. Убедиться, что DLC-прототип появился.


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
