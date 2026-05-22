# S07 — WorldGrid SoA + chunk metadata

## Depends on

- S02

## Можно выполнять параллельно с

- S04
- S05
- S06

## Цель этапа

Создать модель мира в плотных массивах с chunk metadata, но без физической симуляции.

## Roadmap revision note

Раньше это был S10.


## Требования к реализации

- Создать crate `flux_world`.
- Реализовать `WorldGrid` на плотных SoA-массивах.
- Добавить:
  - размеры мира;
  - индексирование клеток;
  - минимальные слои `solid`, `temperature`, `building`;
  - `ChunkMeta`;
  - dirty flags для render/save;
  - bounds для чанков.

## Запрещено

- Не использовать Bevy ECS для клеток.
- Не добавлять физику.
- Не добавлять render.
- Не делать sleeping chunks.

## Ручная проверка

1. Запустить debug command `--world-debug-create 64x64` или эквивалент.
2. Убедиться, что summary показывает размер мира, число клеток, chunk size и число чанков.


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
