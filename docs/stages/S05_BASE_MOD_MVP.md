# S05 — Base mod MVP

## Depends on

- S04

## Можно выполнять параллельно с

- S10

## Цель этапа

Сделать базовую игру обычным модом `base` с минимальным контентом.


## Требования к реализации

- Создать `mods/base/manifest.toml`.
- Добавить минимальный content:
  - 2-3 материала;
  - 1 тестовое здание.
- `base` должен грузиться тем же pipeline, что и любой внешний мод.
- Запрещена special-case логика вида `if mod_id == "base"`.

## Ручная проверка

1. Запустить diagnostic mode/list mods.
2. Убедиться, что `base` найден как мод.
3. Запустить content summary.
4. Убедиться, что материалы и здание из `base` попали в registry.


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check_plan_index.py
```

## Definition of Done

- Реализована только зона ответственности этапа.
- Все automated checks проходят.
- Выполнена ручная проверка из этого документа.
- Нет изменений вне зоны ответственности без объяснения.
- Отчет этапа заполнен по `docs/STAGE_COMPLETION_REPORT_TEMPLATE.md`.
