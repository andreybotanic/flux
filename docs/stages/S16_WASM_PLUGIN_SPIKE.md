# S16 — WASM plugin spike

## Depends on

- S03
- S08

## Можно выполнять параллельно с

- S14
- S15
- S17

## Цель этапа

Проверить runtime-плагины через WASM на минимальном событии/команде.


## Требования к реализации

- Создать экспериментальный `flux_mod_runtime`.
- Подключить выбранный WASM runtime или documented stub, если runtime слишком тяжел для этого этапа.
- Минимальный контракт:
  - host event -> wasm plugin -> validated command -> host applies command.
- Плагин получает событие `GameStarted`/`ScenarioStarted` и возвращает diagnostic/log command.

## Запрещено

- Не давать WASM доступ к Bevy World.
- Не давать WASM доступ к WorldGrid напрямую.
- Не переносить scenario DSL в WASM.

## Ручная проверка

1. Включить test wasm mod.
2. Запустить app/scenario.
3. Убедиться, что plugin diagnostic log появился.
4. Сломать plugin и убедиться, что host не падает panic-ом.


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Definition of Done

- Реализована только зона ответственности этапа.
- Все automated checks проходят.
- Выполнена ручная проверка из этого документа.
- Нет изменений вне зоны ответственности без объяснения.
- Отчет этапа заполнен по `docs/STAGE_COMPLETION_REPORT_TEMPLATE.md`.
