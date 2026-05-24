# S09 — Scenario DSL runtime MVP

## Depends on

- S03
- S08

## Можно выполнять параллельно с

- S10
- S11

## Цель этапа

Добавить сценарии как модовый контент с реальным tick-aware runtime.


## Требования к реализации

- Добавить формат scenario files как модового контента.
- Формат scenario definitions: RON. Одфин файл = один сценарий.
- Сценарии лежат внутри мода в отдельной папке `scenarios`.
- Реализовать tick-aware runner.
- Минимальные команды в сценариях:
  - `Log(message)`;
  - `CreateWorld(width, height, seed)`;
  - `WaitTicks(n)`;
  - `AssertTick(n)`.
- Добавить CLI:
  - `--list-scenarios`;
  - `--run-scenario <id>` (вместо старого `--world-debug-create` - эту команду теперь можно удалить, т.к. сценарии полностью покрывают ее возможности).
- Создать мод `mods/test_scenarios`.
- Создать сценарий:
  ```ron
  Scenario(
      id: "test_scenarios:scenario/bootstrap_smoke",
      steps: [
          Log("scenario started"),
          CreateWorld(width: 64, height: 64, seed: 0),
          WaitTicks(5),
          AssertTick(5),
          Log("scenario finished"),
      ],
  )
  ```


## Ручная проверка
1. Запустить `--list-scenarios`.
2. Запустить `--run-scenario test_scenarios:scenario/bootstrap_smoke`.
3. Убедиться, что сценарий завершился успешно.


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
Обязателен запуск тестового сценария.

## Definition of Done

- Реализована только зона ответственности этапа.
- Все automated checks проходят.
- Выполнена ручная проверка из этого документа.
- Нет изменений вне зоны ответственности без объяснения.
- Отчет этапа заполнен по `docs/STAGE_COMPLETION_REPORT_TEMPLATE.md`.
