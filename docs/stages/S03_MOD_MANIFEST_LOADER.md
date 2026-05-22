# S03 — Mod manifest discovery/validation

## Depends on

S02

## Можно выполнять параллельно с

S10

## Цель этапа

Научить проект находить моды и валидировать их TOML-манифесты без загрузки контента.

## Зона ответственности


- Create `flux_mod_loader`.
- Load and validate TOML `manifest.toml` files under `mods/`.
- Resolve dependencies and deterministic load order.


## Запрещено в этом этапе


- No content prototypes.
- No scenario execution.
- No WASM runtime.


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


1. Create a valid temporary mod manifest.
2. Run `cargo run -p flux_app -- --list-mods` or equivalent diagnostic command.
3. Corrupt the manifest and verify structured error output.


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
