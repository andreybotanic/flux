# S06 — External test mod + patching

## Depends on

- S05

## Можно выполнять параллельно с

- S07
- S08
- S10

## Цель этапа

Проверить, что внешний мод может добавить контент и пропатчить контент `base`.


## Требования к реализации

- Создать `mods/test_content_mod`.
- Мод должен:
  - зависеть от `base`;
  - добавить новый материал;
  - добавить новое здание;
  - пропатчить одно поле у прототипа из `base`.
- Diagnostic summary должен показывать source mod и applied patches.

## Ручная проверка

1. Запустить content summary.
2. Убедиться, что внешний материал появился.
3. Убедиться, что patch к `base` применен.
4. Испортить dependency.
5. Убедиться, что ошибка понятна.


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
