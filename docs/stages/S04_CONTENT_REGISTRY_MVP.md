# S04 — Content registry MVP

## Depends on

- S03

## Можно выполнять параллельно с

- S07

## Цель этапа

Добавить минимальный content registry с загрузкой прототипов без игровой симуляции.


## Требования к реализации

- Создать crate `flux_content`.
- Реализовать `ContentRegistry`.
- На этом этапе намеренно реализуются только два минимальных типа прототипов:
  - `SubstancePrototype` (id: PrototypeId, display_name: LocalizationKey);
  - `StructurePrototype` (id: PrototypeId, display_name: LocalizationKey, size: TileSize).
- Загрузка content files идет из модов, найденных и отсортированных через `S03`.
- Формат content files: RON.
- Поддержать минимальный deterministic patching.
- После `freeze` registry становится immutable для runtime.

## Запрещено

- Не создавать мир.
- Не создавать UI.
- Не добавлять сценарии.
- Не добавлять физику газа/жидкости.
- Не добавлять DLC.

## Ручная проверка

1. Создать content file с одним веществом.
2. Запустить diagnostic command, печатающий registry summary.
3. Убедиться, что материал отображается в summary.
4. Добавить duplicate ID.
5. Убедиться, что ошибка указывает ID и файл.


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
