# S08 — Fixed tick + command/event loop

## Depends on

- S07

## Можно выполнять параллельно с

- S10

## Цель этапа

Добавить фиксированный тик симуляции и базовый command/event pipeline.


## Требования к реализации

- Создать crate `flux_sim` или соответствующий модуль.
- Добавить:
  - `FixedTick`;
  - `SimCommand`;
  - `SimEvent`;
  - `CommandQueue`;
  - `EventQueue`;
  - deterministic tick counter.
- Минимальная команда: `CreateWorld { width, height, seed }`.
- Минимальное событие: `WorldCreated { width, height }`.

## Важное уточнение

Именно этот этап впервые вводит понятие FluxEngine simulation tick. До S08 нельзя использовать `WaitTicks` в сценариях.

## Ручная проверка

1. Запустить debug command, создающий мир 64x64.
2. Выполнить 5 fixed ticks.
3. Убедиться, что tick counter равен 5.


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
