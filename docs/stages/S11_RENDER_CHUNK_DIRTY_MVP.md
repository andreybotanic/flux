# S11 — Chunk-based render dirty MVP

## Depends on

- S01
- S07

## Можно выполнять параллельно с

- S09
- S10
- S12
- S14

## Цель этапа

Добавить минимальный рендер мира, где обновления визуала привязаны к dirty chunks.


## Требования к реализации

- Добавить минимальный render adapter.
- Отрисовать placeholder grid/world.
- Использовать `ChunkMeta.render_dirty` для визуальных обновлений.
- Добавить debug overlay или diagnostic log dirty chunks.

## Запрещено

- Не добавлять физику.
- Не добавлять GPU compute.
- Не использовать dirty chunks для пропуска симуляции.

## Ручная проверка

1. Запустить app.
2. Создать мир через debug command или сценарий, если S09 уже выполнен.
3. Изменить несколько клеток.
4. Увидеть обновление соответствующих областей или dirty chunk logs.


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
