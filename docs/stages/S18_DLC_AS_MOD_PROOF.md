# S18 — DLC as official mod proof

## Depends on

S03, S05, S06

## Можно выполнять параллельно с

S10, S11, S13

## Цель этапа

Доказать, что DLC является обычным официальным модом, а не отдельной hardcode-системой.

## Зона ответственности


- Create `mods/dlc_test` as official mod.
- Add stub entitlement enable/disable flag.
- Content appears only when enabled.


## Запрещено в этом этапе


- No real Steam API.
- No DRM.
- No hardcoded DLC content branch.


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


1. Run content summary without DLC.
2. Confirm DLC prototype absent.
3. Run with DLC enabled.
4. Confirm DLC prototype present.


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
