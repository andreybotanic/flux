# S14 — CPU toy simulation

## Цель этапа

Создать базовую систему simulation stages и реализовать первый stage: gas_diffusion.

На этапе реализуется только CPU backend, но архитектура должна поддерживать будущие backend policies и разные backend-ы для разных stages.


## Реализовать

### Simulation pipeline

Симуляция должна состоять из stages.

Каждый stage должен иметь:
- `SimulationStageId`;
- собственный делитель частоты симуляции относительно базовой частоты;
- backend policy.

Pipeline должен:
- регистрировать stages;
- выполнять stages по frequency;
- обеспечивать deterministic execution order;
- быть готовым к будущему parallel execution.

На этапе `S14` stages выполняются последовательно.


## Backend policy

Добавить BackendPolicy - способ выбора конкретного бэкенда для stage.

На этапе реализовать только: CpuOnly - всегда использовать CPU backend.

Будущие policies (`Gpu`, `Auto` и т.д.) не реализуются.


## Gas diffusion stage

Реализовать простую диффузию газов.

Требования:
- симуляция использует `WorldGrid.gases`;
- газ хранится в частицах;
- используется только CPU backend;
- используется только Von Neumann neighborhood:
  - left;
  - right;
  - up;
  - down.


## Double buffering

Газовая симуляция обязана использовать double buffering.

Запрещено:
```text
in-place update, зависящий от порядка обхода клеток
```

Симуляция должна читать previous state и записывать next state отдельно.


## Conservation

Симуляция газа обязана сохранять общее количество частиц каждого газа.

Требование:
```text
total_particles_by_gas(before) == total_particles_by_gas(after)
```


## Solid cells

Solid cells блокируют газ.

Требования:

- газ не перетекает в solid cell;
- газ не хранится в solid cell;
- наличие газа в solid cell должно приводить к validation error или deterministic cleanup behavior;
- выбранное поведение должно быть зафиксировано тестами;
- в целях оптимизации при симуляции газа нельзя напрямую читать данные о solid cell: при загрузке мира нужно строить "маску проницаемости" - плотный массив из bool, который указывает, может ли в клетке быть газ или нет.


## Determinism

Повторный запуск одного и того же сценария с одинаковым мировым seed должен давать одинаковый результат.


## Scenario integration

В сценарные шаги нужно добавить набор ассертов для проверки количества газа в клетках и во всем мире:
- AssertGasParticlesEq(gas, value)
- AssertGasParticlesNotEq(gas, value)
- AssertGasParticlesLess(gas, value)
- AssertGasParticlesLessOrEq(gas, value)
- AssertGasParticlesGrater(gas, value)
- AssertGasParticlesGraterOrEq(gas, value)
- AssertGasParticlesEq(gas, cell, value)
- AssertGasParticlesNotEq(gas, cell, value)
- AssertGasParticlesLess(gas, cell, value)
- AssertGasParticlesLessOrEq(gas, cell, value)
- AssertGasParticlesGrater(gas, cell, value)
- AssertGasParticlesGraterOrEq(gas, cell, value)

Во всех этих ассертах:
- gas - конкретный id газа или Null если нужно сравнить общее число частиц газовой смеси;
- cell - координаты клетки;
- value - целое число >= 0.

Добавить smoke scenario для diffusion с проверкой, что газ распространяется и сохраняется его количество. Использовать новые ассерты.


## Не реализовывать
```text
- GPU backend;
- parallel execution;
- pressure simulation;
- temperature interaction;
- liquids;
- gas pipes;
- pumps;
- vents;
- overlays;
- mod-defined simulation stages;
- runtime scripting hooks.
```


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Добавить запуск сценариев и проверку их логов.

Должно быть два сценария:
- загрузка slot_a, симуляция, проверки, сохранение в slot_b (slot_a перезаписывать нельзя!),
- загрузка slot_b, без запуска симуляции, проверки на те же значения что и в первом сценарии - сценарии должны показать, что газ после симуляции корректно созраняется и загружается.

## Definition of Done

- Реализована только зона ответственности этапа.
- Все automated checks проходят.
- Выполнена ручная проверка из этого документа.
- Нет изменений вне зоны ответственности без объяснения.
- Отчет этапа заполнен по `docs/STAGE_COMPLETION_REPORT_TEMPLATE.md`.
