# S15 — GPU backend contract + compute spike

## Цель этапа

Добавить GPU backend для симуляции газа.


## Требования к реализации

Добавить второй SimulationBackendId: Gpu.

Реализовать второй BackendPolicy: `PreferGpu(cpu_fallback: bool)`.

Эта политика пытается использовать GPU, если он есть и если у SimulationStage есть реализация этого бэкенда:
- Если cpu_fallback установлен в true, то при отсутствии GPU или если у SimulationStage нет реализации GPU бэкенда - используется Cpu бэкенд.
- Если cpu_fallback установлен в false, то при отсутствии GPU или если у SimulationStage нет реализации GPU бэкенда - приложение должно завершаться с понятной ошибкой.

Для всего flux_app нужно добавить CLI параметр `--backend-policy <BackendPolicy>` для выбора политики. Эта политика должна применяться сразу ко всем этапам симуляции. Если параметр не указан, то дефолтное значение - `BackendPolicy::CpuOnly`.

Реализовать GPU версию диффузии газа через compute shader на WGSL. На данном этапе **не требуется** абсолютно строгое соответствие симуляции на CPU и на GPU. `S15` нужен для проверки самой концепции разных бэкендов.


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Необходимо запустить все существующие сценарии: они не должны давать ошибок. Сценарии, имеющие симуляцию, нужно запускать с разными BackendPolicy: результаты должны быть одинаковыми.

## Ручная проверка

1. Запустить app с CPU backend.
2. Запустить app с GPU backend.
3. Проверить diagnostic log выбранного backend.
4. Если GPU недоступен, fallback должен быть понятным.


## Definition of Done

- Реализована только зона ответственности этапа.
- Все automated checks проходят.
- Выполнена ручная проверка из этого документа.
- Нет изменений вне зоны ответственности без объяснения.
- Отчет этапа заполнен по `docs/STAGE_COMPLETION_REPORT_TEMPLATE.md`.
