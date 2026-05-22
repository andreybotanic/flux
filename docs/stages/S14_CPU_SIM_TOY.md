# S14 — CPU toy simulation

## Depends on

S11

## Можно выполнять параллельно с

S12, S13

## Цель этапа

Добавить первую простую CPU-симуляцию, чтобы проверить fixed tick и структуру мира.

## Зона ответственности


- Add first CPU toy simulation, e.g. heat diffusion.
- Use double buffering or delta/apply.
- Add scenario commands/asserts for temperature.


## Запрещено в этом этапе


- No full gas simulation.
- No GPU.
- No order-dependent in-place update without tests.


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


1. Create 32x32 world.
2. Heat a rectangle.
3. Wait 20 ticks.
4. Confirm heat spreads in logs/overlay.


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
