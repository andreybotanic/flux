# FluxEngine Rewrite — DAG и индекс этапов

Этот документ описывает зависимости между этапами. Если два этапа не зависят друг от друга, они могут выполняться параллельно, но только при соблюдении указанной зоны ответственности.

## DAG верхнего уровня

```text
S00
 ├─ S01
 └─ S02
     ├─ S03
     │   ├─ S04
     │   │   └─ S09
     │   ├─ S05
     │   │   ├─ S06
     │   │   │   ├─ S07
     │   │   │   └─ S18
     │   │   └─ S08
     │   │       └─ S09
     │   └─ S16
     └─ S10
         ├─ S11
         │   ├─ S12
         │   ├─ S14
         │   │   └─ S15
         │   └─ S17
         └─ S13

S19 depends_on: S07, S09, S12, S13, S15
```

## Dependency edges

```text
S00 -> S01
S00 -> S02
S02 -> S03
S03 -> S04
S03 -> S05
S05 -> S06
S06 -> S07
S05 -> S08
S04 -> S09
S08 -> S09
S02 -> S10
S10 -> S11
S11 -> S12
S10 -> S13
S01 -> S13
S11 -> S14
S14 -> S15
S03 -> S16
S11 -> S17
S03 -> S18
S05 -> S18
S06 -> S18
S07 -> S19
S09 -> S19
S12 -> S19
S13 -> S19
S15 -> S19
```

## Этапы

| ID | Документ | Название | Depends on | Можно параллельно с |
|---|---|---|---|---|
| S00 | `docs/stages/S00_REPO_BOOTSTRAP.md` | Bootstrap workspace | — | — |
| S01 | `docs/stages/S01_APP_SHELL.md` | Bevy app shell + diagnostics | S00 | S02 |
| S02 | `docs/stages/S02_CORE_TYPES.md` | Core IDs, versions, errors | S00 | S01 |
| S03 | `docs/stages/S03_MOD_MANIFEST_LOADER.md` | Mod manifest discovery/validation | S02 | S10 |
| S04 | `docs/stages/S04_SCENARIO_DSL_MVP.md` | Declarative scenario DSL MVP | S03 | S05 |
| S05 | `docs/stages/S05_CONTENT_REGISTRY_MVP.md` | Content registry MVP | S03 | S04 |
| S06 | `docs/stages/S06_BASE_MOD_MVP.md` | base mod MVP | S05 | S08 |
| S07 | `docs/stages/S07_EXTERNAL_TEST_MOD.md` | External test mod + patching | S06 | S10, S11 |
| S08 | `docs/stages/S08_UI_REGISTRY_MVP.md` | UI registry MVP | S05 | S06 |
| S09 | `docs/stages/S09_SCENARIO_RUNNER_UI_SCREENSHOT.md` | Scenario runner with UI, screenshot and diagnostic logs | S04, S08 | S10, S11 |
| S10 | `docs/stages/S10_WORLD_GRID_CHUNK_META.md` | WorldGrid SoA + chunk metadata | S02 | S03, S04, S05 |
| S11 | `docs/stages/S11_FIXED_TICK_AND_COMMANDS.md` | Fixed tick + command/event loop | S10 | S08 |
| S12 | `docs/stages/S12_SAVE_LOAD_MVP.md` | Save/load MVP | S11 | S13, S14 |
| S13 | `docs/stages/S13_RENDER_CHUNK_DIRTY_MVP.md` | Chunk-based render dirty MVP | S10, S01 | S12, S14 |
| S14 | `docs/stages/S14_CPU_SIM_TOY.md` | CPU toy simulation | S11 | S12, S13 |
| S15 | `docs/stages/S15_REPLAY_DETERMINISM.md` | Replay/determinism harness | S14 | S16 |
| S16 | `docs/stages/S16_WASM_PLUGIN_SPIKE.md` | WASM plugin spike | S03 | S15, S17 |
| S17 | `docs/stages/S17_GPU_BACKEND_SPIKE.md` | GPU backend contract + compute spike | S11 | S16 |
| S18 | `docs/stages/S18_DLC_AS_MOD_PROOF.md` | DLC as official mod proof | S03, S05, S06 | S10, S11, S13 |
| S19 | `docs/stages/S19_INTEGRATION_VERTICAL_SLICE.md` | Integration vertical slice | S07, S09, S12, S13, S15 | — |

## Правила параллельной работы

### Ветки не должны менять чужую зону ответственности

Если этап работает над `flux_ui`, он не должен одновременно менять `flux_world`, кроме явно описанных публичных интерфейсов.

### Общие типы меняются только через отдельный этап

Если для работы этапа требуется изменить `flux_core`, а это не указано в зоне ответственности, нужно:

1. остановиться;
2. описать недостающий тип/контракт;
3. сделать отдельный маленький этап или обновить текущий документ;
4. только потом менять `flux_core`.

### Нельзя "по пути" реализовывать будущие этапы

Например:

- в S05 нельзя делать полноценный UI;
- в S08 нельзя делать сценарный runner;
- в S10 нельзя делать физическую симуляцию газа;
- в S13 нельзя делать GPU backend;
- в S16 нельзя менять формат контента.

## Общая команда проверки

```bash
./scripts/ci.sh
```

Эквивалентный минимум:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check_plan_index.py
```
