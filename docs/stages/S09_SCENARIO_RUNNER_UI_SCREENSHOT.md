# S09 — Scenario runner with UI, screenshot and diagnostic logs

## Depends on

S04, S08

## Можно выполнять параллельно с

S10, S11

## Цель этапа

Расширить сценарии до уровня базовой UI-автоматизации, скриншотов и диагностических логов.

## Зона ответственности


- Add scenario steps: `OpenUi`, `Click`, `AssertUiExists`, `TakeScreenshot`, `AssertLogContains`.
- Store logs/screenshots under `target/flux_scenarios/...`.


## Запрещено в этом этапе


- No gameplay automation beyond UI MVP.
- No coordinate-based clicks if stable widget IDs exist.


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


1. Run UI smoke scenario.
2. Check diagnostic log.
3. Check screenshot or explicit skipped status in headless mode.


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
