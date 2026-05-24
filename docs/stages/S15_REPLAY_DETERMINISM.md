# S15 — Replay/determinism harness

## Depends on

- S09
- S14

## Можно выполнять параллельно с

- S16
- S17

## Цель этапа

Добавить replay harness, command log и проверку воспроизводимости результата.


## Требования к реализации

- Записывать command log:
  - tick;
  - command;
  - source.
- Добавить replay mode.
- Добавить state hash на выбранных слоях мира.
- Добавить assertions:
  - `AssertWorldHash(hash)`;
  - или `RecordWorldHash(label)` + comparison.

## Ручная проверка

1. Запустить deterministic scenario.
2. Получить hash.
3. Запустить replay.
4. Убедиться, что hash совпадает.


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Definition of Done

- Реализована только зона ответственности этапа.
- Все automated checks проходят.
- Выполнена ручная проверка из этого документа.
- Нет изменений вне зоны ответственности без объяснения.
- Отчет этапа заполнен по `docs/STAGE_COMPLETION_REPORT_TEMPLATE.md`.
