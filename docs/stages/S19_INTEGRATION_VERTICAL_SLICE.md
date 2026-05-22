# S19 — Integration vertical slice

## Depends on

- S06
- S12
- S13
- S15

## Можно выполнять параллельно с

- Нет.

## Цель этапа

Собрать первый вертикальный срез платформы: моды, контент, UI, мир, сценарий, скриншот, сохранение, replay.

## Roadmap revision note

Финальный интеграционный этап первого roadmap-среза.


## Требования к реализации

Создать end-to-end scenario:

- загрузить `base`;
- загрузить `test_content_mod`;
- открыть UI;
- сделать screenshot;
- создать мир;
- применить простое изменение мира;
- дождаться нескольких ticks;
- сохранить;
- загрузить;
- проверить world hash/replay;
- записать summary.

## Ручная проверка

1. Запустить `platform_vertical_slice` scenario.
2. Открыть summary.
3. Проверить diagnostic log.
4. Проверить screenshot.
5. Проверить save/load.
6. Запустить replay и сверить hash.


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
