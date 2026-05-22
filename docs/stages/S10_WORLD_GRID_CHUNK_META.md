# S10 — WorldGrid SoA + chunk metadata

## Depends on

S02

## Можно выполнять параллельно с

S03, S04, S05

## Цель этапа

Создать модель мира в плотных массивах с chunk metadata, но без физической симуляции.

## Зона ответственности


- Create `flux_world`.
- Implement `WorldGrid` SoA arrays, dimensions, indexing, `ChunkMeta`, dirty flags and chunk bounds.


## Запрещено в этом этапе


- No Bevy entities per cell.
- No physics.
- No sleeping chunks.


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


1. Run debug command `--world-debug-create 64x64` or equivalent.
2. Confirm summary includes world size, cell count, chunk size and chunk count.


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
