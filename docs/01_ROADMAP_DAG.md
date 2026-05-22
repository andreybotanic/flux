# FluxEngine Rewrite — DAG и индекс этапов

Этот документ описывает актуальный roadmap после исправления зависимости сценариев от fixed tick/runtime.

## Важная миграционная пометка

Этап `S03_MOD_MANIFEST_LOADER` уже выполнен и не переименовывается.

Начиная с `S04`, roadmap исправлен:

- бывший ранний `S04_SCENARIO_DSL_MVP` удален как некорректный;
- сценарии теперь появляются в `S09_SCENARIO_DSL_RUNTIME_MVP`;
- `S09` зависит от `S03` и `S08`, потому что `WaitTicks` требует FluxEngine simulation tick;
- Bevy `Update`/`FixedUpdate` не считается достаточным контрактом для сценариев и replay.

## Новый DAG верхнего уровня

```text
S00
 ├─ S01
 └─ S02
     ├─ S03
     │   └─ S04
     │       ├─ S05
     │       │   ├─ S06
     │       │   └─ S18
     │       └─ S10
     └─ S07
         └─ S08
             ├─ S09
             │   ├─ S12
             │   ├─ S13
             │   └─ S15
             ├─ S14
             │   └─ S15
             ├─ S16
             └─ S17

S11 depends_on: S01, S07
S19 depends_on: S06, S12, S13, S15
```

## Dependency edges

```text
S00 -> S01
S00 -> S02
S02 -> S03
S03 -> S04
S04 -> S05
S05 -> S06
S02 -> S07
S07 -> S08
S03 -> S09
S08 -> S09
S04 -> S10
S01 -> S11
S07 -> S11
S09 -> S12
S10 -> S12
S11 -> S12
S08 -> S13
S09 -> S13
S08 -> S14
S09 -> S15
S14 -> S15
S03 -> S16
S08 -> S16
S08 -> S17
S03 -> S18
S04 -> S18
S05 -> S18
S06 -> S19
S12 -> S19
S13 -> S19
S15 -> S19
```

## Этапы

| ID | Документ | Название | Depends on | Можно параллельно с |
|---|---|---|---|---|
| S00 | `docs/stages/S00_REPO_BOOTSTRAP.md` | Repo bootstrap | — | — |
| S01 | `docs/stages/S01_APP_SHELL.md` | Bevy app shell + diagnostics | S00 | S02 |
| S02 | `docs/stages/S02_CORE_TYPES.md` | Core IDs, versions, errors | S00 | S01 |
| S03 | `docs/stages/S03_MOD_MANIFEST_LOADER.md` | Mod manifest discovery/validation | S02 | S07 |
| S04 | `docs/stages/S04_CONTENT_REGISTRY_MVP.md` | Content registry MVP | S03 | S07 |
| S05 | `docs/stages/S05_BASE_MOD_MVP.md` | Base mod MVP | S04 | S10 |
| S06 | `docs/stages/S06_EXTERNAL_TEST_MOD.md` | External test mod + patching | S05 | S07, S08, S10 |
| S07 | `docs/stages/S07_WORLD_GRID_CHUNK_META.md` | WorldGrid SoA + chunk metadata | S02 | S04, S05, S06 |
| S08 | `docs/stages/S08_FIXED_TICK_AND_COMMANDS.md` | Fixed tick + command/event loop | S07 | S10 |
| S09 | `docs/stages/S09_SCENARIO_DSL_RUNTIME_MVP.md` | Scenario DSL runtime MVP | S03, S08 | S10, S11 |
| S10 | `docs/stages/S10_UI_REGISTRY_MVP.md` | UI registry MVP | S04 | S05, S06, S08, S09 |
| S11 | `docs/stages/S11_RENDER_CHUNK_DIRTY_MVP.md` | Chunk-based render dirty MVP | S01, S07 | S09, S10, S12, S14 |
| S12 | `docs/stages/S12_SCENARIO_RUNNER_UI_SCREENSHOT.md` | Scenario runner with UI, screenshot and diagnostic logs | S09, S10, S11 | S13, S14 |
| S13 | `docs/stages/S13_SAVE_LOAD_MVP.md` | Save/load MVP | S08, S09 | S11, S14 |
| S14 | `docs/stages/S14_CPU_SIM_TOY.md` | CPU toy simulation | S08 | S11, S12, S13, S16 |
| S15 | `docs/stages/S15_REPLAY_DETERMINISM.md` | Replay/determinism harness | S09, S14 | S16, S17 |
| S16 | `docs/stages/S16_WASM_PLUGIN_SPIKE.md` | WASM plugin spike | S03, S08 | S14, S15, S17 |
| S17 | `docs/stages/S17_GPU_BACKEND_SPIKE.md` | GPU backend contract + compute spike | S08 | S15, S16, S18 |
| S18 | `docs/stages/S18_DLC_AS_MOD_PROOF.md` | DLC as official mod proof | S03, S04, S05 | S17 |
| S19 | `docs/stages/S19_INTEGRATION_VERTICAL_SLICE.md` | Integration vertical slice | S06, S12, S13, S15 | — |

## Правила параллельной работы

- Нельзя менять смысл уже завершенного `S03` без отдельной миграционной задачи.
- Нельзя использовать `WaitTicks` до `S08`.
- Нельзя добавлять scenario runtime до `S09`.
- Если этапу нужен новый общий тип из `flux_core`, это нужно явно указать в отчете.
- Один этап — одна ветка и один PR.
