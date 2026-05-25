# S11 — World Render MVP (dense grid)

## Depends on

- S01
- S07

## Можно выполнять параллельно с

- S09
- S10
- S12
- S14

## Цель этапа

Добавить минимальный рендер мира для dense WorldGrid без chunk metadata.

## Требования к реализации

- Добавить минимальный render adapter.
- Отрисовать placeholder grid/world.
- Обновлять визуал по dense grid данным мира (без chunk lookup API).
- Добавить debug overlay или diagnostic log по обновленным клеткам/областям.

## Запрещено

- Не добавлять физику.
- Не добавлять GPU compute.
- Не добавлять chunk-based организацию мира обратно.

## Ручная проверка

1. Запустить app.
2. Создать мир через debug command или сценарий, если S09 уже выполнен.
3. Изменить несколько клеток.
4. Увидеть обновление соответствующих областей или диагностический лог без chunk metadata.

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
