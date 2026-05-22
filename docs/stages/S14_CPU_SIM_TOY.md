# S14 — CPU toy simulation

## Depends on

- S08

## Можно выполнять параллельно с

- S11
- S12
- S13
- S16

## Цель этапа

Добавить первую простую CPU-симуляцию, чтобы проверить fixed tick и структуру мира.

## Roadmap revision note

Сохраняет номер, но теперь зависит от нового S08.


## Требования к реализации

- Добавить простую CPU toy-модель, например теплопроводность.
- Использовать double buffering или delta/apply фазу.
- Добавить scenario commands:
  - `SetTemperatureRect`;
  - `AssertTemperatureApprox`.
- Результат должен быть детерминированным при одинаковом seed/initial state.

## Ручная проверка

1. Создать мир 32x32.
2. Нагреть прямоугольник.
3. Подождать 20 тиков.
4. Увидеть в логах/оверлее, что тепло распространяется.


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
