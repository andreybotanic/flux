# S12 — Scenario runner with UI, screenshot and diagnostic logs

## Depends on

- S09
- S10
- S11

## Можно выполнять параллельно с

- S13
- S14

## Цель этапа

Расширить сценарии до имитации базовой активности игрока в UI и снятия скриншотов.


## Требования к реализации

- Расширить scenario steps:
  - `OpenUi(panel_id)`;
  - `Click(widget_id)`;
  - `WaitFrames(n)` или `WaitTicks(n)`;
  - `TakeScreenshot(path)`;
  - `AssertUiExists(widget_id)`;
  - `AssertLogContains(text)`.
- Сценарии должны кликать UI через stable IDs, а не через координаты.
- Сценарий должен создавать артефакты:
  - diagnostic log;
  - screenshot или explicit skipped status;
  - summary.

## Ручная проверка

1. Запустить UI smoke scenario.
2. Проверить summary.
3. Проверить diagnostic log.
4. Проверить наличие скриншота или понятный статус `screenshot skipped`.


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
