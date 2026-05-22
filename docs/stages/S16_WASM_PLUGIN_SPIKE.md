# S16 — WASM plugin spike

## Depends on

S03

## Можно выполнять параллельно с

S15, S17

## Цель этапа

Проверить runtime-плагины через WASM на минимальном событии/команде.

## Зона ответственности


- Create experimental `flux_mod_runtime`.
- Load minimal WASM plugin or stub runtime.
- Host event -> plugin -> validated command -> host applies command.


## Запрещено в этом этапе


- No direct Bevy World access.
- No direct WorldGrid access.
- No full plugin API.


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


1. Enable test WASM mod.
2. Run app/scenario.
3. Confirm plugin diagnostic log appears.
4. Break plugin and confirm host reports structured error.


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
