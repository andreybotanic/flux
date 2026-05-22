# S11 — Fixed tick + command/event loop

## Depends on

S10

## Можно выполнять параллельно с

S08

## Цель этапа

Добавить фиксированный тик симуляции и базовый command/event pipeline.

## Зона ответственности


- Create `flux_sim` or equivalent.
- Add fixed tick counter, `SimCommand`, `SimEvent`, command queue, event queue.
- Add minimal `CreateWorld` command and `WorldCreated` event.


## Запрещено в этом этапе


- No gas/liquid physics.
- No save/load.
- No GPU.


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


1. Run scenario: create 64x64 world, wait 5 ticks, log tick count.
2. Confirm tick count is deterministic.


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
