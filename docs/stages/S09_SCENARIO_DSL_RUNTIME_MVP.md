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
- Формат scenario definitions: RON.
- Сценарии объявляются в manifest через `scenarios = [...]`.
- Реализовать tick-aware runner.
- Минимальные steps:
  - `Log(message)`;
  - `CreateWorld(width, height, seed)`;
  - `WaitTicks(n)`;
  - `AssertTick(n)`;
  - `AssertLogContains(text)`.
- Добавить CLI:
  - `--list-scenarios`;
  - `--run-scenario <id>`.

## Почему этап стоит здесь

`WaitTicks` требует `S08`, потому что Bevy Update/FixedUpdate — это не тот же самый контракт, что FluxEngine simulation tick.

## Ручная проверка

1. Создать `mods/test_scenarios`.
2. Добавить scenario file:

```ron
Scenario(
    id: "test_scenarios:scenario/bootstrap_smoke",
    steps: [
        Log("scenario started"),
        CreateWorld(width: 64, height: 64, seed: 1),
        WaitTicks(5),
        AssertTick(5),
        Log("scenario finished"),
    ],
)
```

3. Запустить `--list-scenarios`.
4. Запустить `--run-scenario test_scenarios:scenario/bootstrap_smoke`.
5. Убедиться, что сценарий завершился успешно.


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
