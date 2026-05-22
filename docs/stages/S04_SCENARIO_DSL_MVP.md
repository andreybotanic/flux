# S04 — Declarative scenario DSL MVP

## Depends on

S03

## Можно выполнять параллельно с

S05

## Цель этапа

Добавить минимальный RON-формат сценариев как модового контента, пока без UI и без мира.

## Зона ответственности


- Add RON scenario file loading.
- Scenarios are declared by mods.
- Implement `Log`, `WaitTicks`, minimal assert/log behavior.
- Add `--list-scenarios` and `--run-scenario <id>`.


## Запрещено в этом этапе


- No UI automation.
- No screenshot.
- No world creation.


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


1. Add `mods/test_scenarios` with a RON scenario.
2. Run `--list-scenarios`.
3. Run `--run-scenario test_scenarios:scenario/log_smoke`.


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
