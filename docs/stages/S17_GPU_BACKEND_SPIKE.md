# S17 — GPU backend contract + compute spike

## Depends on

- S08

## Можно выполнять параллельно с

- S15
- S16
- S18

## Цель этапа

Добавить контракт simulation backend и экспериментальный GPU compute spike.

## Roadmap revision note

Раньше зависел от S11; теперь от нового S08.


## Требования к реализации

- Добавить `SimulationBackend` trait.
- Реализовать `CpuSimulationBackend` как baseline.
- Добавить GPU compute spike для простой операции.
- GPU не должен быть обязательным для запуска.
- Должен быть CPU fallback.

## Ручная проверка

1. Запустить app с CPU backend.
2. Запустить app с GPU backend flag.
3. Проверить diagnostic log выбранного backend.
4. Если GPU недоступен, fallback должен быть понятным.


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
