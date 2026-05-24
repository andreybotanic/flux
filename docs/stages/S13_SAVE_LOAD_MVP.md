# S13 — Save/load MVP

## Depends on

- S08
- S09

## Можно выполнять параллельно с

- S11
- S14

## Цель этапа

Добавить минимальное сохранение и загрузку мира с manifest активных модов и scenario steps.


## Требования к реализации

- Создать crate `flux_save`.
- Сохранять:
  - save manifest;
  - world dimensions;
  - seed;
  - tick;
  - минимальные слои WorldGrid;
  - active mods list;
  - registry signature/hash placeholder.
- Добавить scenario steps:
  - `SaveGame(name)`;
  - `LoadGame(name)`;
  - `AssertWorldLoaded`.

## Ручная проверка

1. Запустить scenario:
   - CreateWorld;
   - WaitTicks 3;
   - SaveGame;
   - LoadGame;
   - AssertWorldLoaded.
2. Проверить файл сейва на диске.
3. Убедиться, что manifest содержит active mods.


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
