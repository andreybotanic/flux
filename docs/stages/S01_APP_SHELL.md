# S01 — Bevy app shell + diagnostics

## Depends on

S00

## Можно выполнять параллельно с

S02

## Цель этапа

Создать минимальную Bevy-оболочку приложения с диагностическим режимом запуска.

## Зона ответственности


- Work in `flux_app`.
- Add Bevy `App` and a window titled `FluxEngine`.
- Add `--version` and diagnostic startup logs.
- Optional: add `--headless` if compatible with the selected Bevy setup.


## Запрещено в этом этапе


- No game world.
- No mod loader.
- No UI registry.
- No scenario runner.


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


1. Run `cargo run -p flux_app`.
2. Confirm a `FluxEngine` window opens.
3. Run `cargo run -p flux_app -- --version`.


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
