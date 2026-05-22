# S17 — GPU backend contract + compute spike

## Depends on

S11

## Можно выполнять параллельно с

S16

## Цель этапа

Добавить контракт simulation backend и экспериментальный GPU compute spike на простой операции.

## Зона ответственности


- Add `SimulationBackend` trait.
- Keep `CpuSimulationBackend` as baseline.
- Add GPU compute spike for one simple operation or toy heat step with CPU reference.


## Запрещено в этом этапе


- No GPU-only simulation.
- No removal of CPU path.
- No full simulation port.


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


1. Run CPU backend.
2. Run GPU backend flag.
3. Confirm selected backend log or graceful fallback.


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
