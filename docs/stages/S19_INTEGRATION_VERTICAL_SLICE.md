# S19 — Integration vertical slice

## Depends on

S07, S09, S12, S13, S15

## Можно выполнять параллельно с

Нет.

## Цель этапа

Собрать первый вертикальный срез платформы: моды, контент, UI, мир, сценарий, скриншот, сохранение, replay.

## Зона ответственности


- Create end-to-end scenario loading base/test mod, opening UI, screenshotting, creating world, saving/loading, replaying and hashing.
- Fix only integration bugs between already built systems.


## Запрещено в этом этапе


- No new major architecture.
- No gameplay scope expansion.
- No GPU implementation.


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


1. Run `test_scenarios:scenario/platform_vertical_slice`.
2. Inspect summary, log, screenshots, command log and save.
3. Run replay and compare hash.


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
