# S02 — Core IDs, versions, errors

## Depends on

S00

## Можно выполнять параллельно с

S01

## Цель этапа

Создать базовые типы ID, версий и ошибок для последующих crates.

## Зона ответственности


- Work in `flux_core`.
- Add `NamespacedId`, `ModId`, `PrototypeId`, `ApiVersion`, version wrappers and structured errors.
- Implement parsing/validation for `namespace:path/to/item`.


## Запрещено в этом этапе


- No content registry.
- No Bevy dependency in `flux_core`.
- No concrete game materials/buildings.


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


1. Run `cargo test -p flux_core`.
2. Confirm invalid IDs report concrete reasons.


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
