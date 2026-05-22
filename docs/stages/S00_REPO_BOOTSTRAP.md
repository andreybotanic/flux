# S00 — Bootstrap workspace

## Depends on

Нет.

## Можно выполнять параллельно с

Нет.

## Цель этапа

Проверить и зафиксировать стартовый workspace, CI, hooks и документацию.

## Зона ответственности


- Verify root starter files are present.
- Verify minimal workspace crates `flux_core` and `flux_app` compile.
- Verify CI script and git hooks exist.
- Do not add gameplay or Bevy yet unless explicitly moved to S01.


## Запрещено в этом этапе


- No mod loader.
- No Bevy shell.
- No world model.
- No UI registry.


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
# or, on Windows:
python scripts/check_plan_index.py
```

## Manual verification


1. Run `cargo run -p flux_app -- --version`.
2. Run CI command for your environment:
   - Unix/Git Bash: `bash scripts/ci.sh`
   - Windows CMD/PowerShell: `scripts\ci.cmd`
3. Confirm docs and hooks exist.


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
- python3 scripts/check_plan_index.py (or python/py -3 on Windows): pass/fail

Touched files:
- ...

Known limitations:
- ...
```
