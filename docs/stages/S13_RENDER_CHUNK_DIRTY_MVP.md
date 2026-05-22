# S13 — Chunk-based render dirty MVP

## Depends on

S10, S01

## Можно выполнять параллельно с

S12, S14

## Цель этапа

Добавить минимальный рендер мира, где обновления визуала привязаны к dirty chunks.

## Зона ответственности


- Add minimal render adapter.
- Render placeholder world/grid.
- Use `ChunkMeta.render_dirty` for visual updates/logging.


## Запрещено в этом этапе


- No GPU compute.
- No full atlas/texture array.
- No real building sprites.


## Глобальные требования, которые нужно соблюдать

- ID только в формате `namespace:path/to/item`, например `base:building/gas_pump`.
- Manifest/config — TOML.
- Content/scenarios/patches — RON.
- Save manifest/diagnostic summaries/replay logs — JSON или NDJSON согласно `docs/02_PROJECT_CONVENTIONS.md`.
- Crates — `flux_*`.
- Public types — `PascalCase`, acronyms as `Ui`, `Gpu`, `Cpu`, `Dlc`, `Wasm`.
- Моды не получают прямой доступ к Bevy World.
- Клетки мира не являются Bevy entities.

## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check_plan_index.py
```

## Manual verification


1. Start app and create a world.
2. Modify cells with debug/scenario command.
3. Confirm dirty chunk visualization/logging.


## Definition of Done

- Automated checks passed.
- Manual verification completed.
- Stage responsibility implemented and documented.
- No future stage implemented “along the way”.
- No global convention violated.

## Ожидаемый отчет исполнителя

```text
Implemented:
- ...

Manual verification:
- command: ...
- expected result: ...
- actual result: ...

Automated checks:
- cargo fmt --all --check: pass/fail
- cargo clippy --workspace --all-targets -- -D warnings: pass/fail
- cargo test --workspace: pass/fail
- python3 scripts/check_plan_index.py: pass/fail

Touched files:
- ...

Known limitations:
- ...
```
