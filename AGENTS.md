# AGENTS.md

Этот файл обязателен к прочтению каждым человеком или ИИ-агентом перед работой над FluxEngine.

## 1. Обязательные документы перед началом работы

Перед любым изменением кода прочитать:

1. `docs/00_OVERVIEW.md`
2. `docs/01_ROADMAP_DAG.md`
3. `docs/02_PROJECT_CONVENTIONS.md`
4. ТЗ текущего этапа из `docs/stages/`
5. Этот файл `AGENTS.md`

Если инструкция в ТЗ противоречит overview или conventions, нужно остановиться и явно зафиксировать конфликт в отчете.

## 2. Режим работы

Каждая задача соответствует одному этапу `Sxx`.

Агент обязан:

- реализовать только зону ответственности этапа;
- не делать будущие этапы “заодно”;
- не менять публичные контракты без необходимости;
- не скрывать недоделки;
- оставлять проект в компилируемом состоянии;
- добавлять тесты к новой логике;
- добавлять manual verification path.

## 3. Зафиксированные глобальные решения

### 3.1. Структура ID

Используется только формат:

```text
namespace:path/to/item
```

Примеры:

```text
base:material/oxygen
base:building/gas_pump
base:ui/main_menu
example_mod:scenario/bootstrap_smoke
```

Запрещены альтернативные форматы:

```text
base.building.gas_pump
material:oxygen
base/gas_pump
```

### 3.2. Форматы данных

| Назначение | Формат |
|---|---|
| mod manifest | TOML |
| project/runtime config | TOML |
| content prototypes | RON |
| content patches | RON |
| scenario definitions | RON |
| save manifest | JSON |
| save payload/chunks | binary, формат вводится отдельным этапом |
| command/replay log | JSON или NDJSON |
| diagnostic summaries | JSON |
| screenshots | PNG |

YAML не используется для игровых данных, сценариев, контента или конфигов проекта.

### 3.3. Naming conventions

Crates:

```text
flux_core
flux_world
flux_sim
flux_mod_loader
```

IDs:

```text
lower_snake_case_namespace:lower_snake_case/path_segments
```

Public Rust types:

```text
PascalCase
```

Аббревиатуры в публичных типах пишутся в Rust-style CamelCase:

```rust
UiRegistry
GpuBackend
CpuSimulationBackend
DlcEntitlement
WasmPluginRuntime
```

Не использовать:

```rust
UIRegistry
GPUBackend
CPUBackend
DLCEntitlement
```

### 3.4. BOM policy

- UTF-8 BOM (`EF BB BF`) обязателен в файлах с расширением `.md`.
- Для всех остальных файлов BOM запрещен.

## 4. Архитектурные запреты

### 4.1. Запрещено хранить тайлы как Bevy entities

Клетки мира должны жить в `flux_world` в плотных массивах.

### 4.2. Запрещено давать модам прямой доступ к Bevy World

Публичный модовый API должен быть через:

- декларативные прототипы;
- события;
- команды;
- сценарии;
- валидируемые extension points.

### 4.3. Запрещено hardcode-ID игрового контента

Нельзя использовать enum как источник истины для материалов, зданий, блоков, UI-панелей, оверлеев.

### 4.4. Запрещено делать sleeping chunks на раннем этапе

Chunk metadata можно использовать для dirty render/save/profiling, но нельзя пропускать симуляцию чанка по признаку “далеко от камеры” или “неактивен”, пока отдельный этап не докажет корректность такой оптимизации.

### 4.5. Запрещено делать GPU-only логику без CPU reference

Любая GPU-симуляция должна иметь CPU fallback/reference.

## 5. Обязательные команды перед завершением этапа

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check_plan_index.py
```

Если этап добавляет сценарий:

```bash
cargo run -p flux_app -- --run-scenario <scenario_id>
```

Если этап добавляет UI/рендер:

- запустить приложение вручную;
- выполнить manual verification из ТЗ;
- сохранить скриншот, если ТЗ требует.

## 6. Требования к тестовым сценариям

Сценарный мод — часть продукта, а не test-only хак.

Сценарии должны постепенно получить возможности:

- ждать N тиков;
- писать диагностический лог;
- создавать мир;
- выполнять команды игрока;
- открывать/закрывать UI;
- нажимать элементы UI через stable UI IDs;
- делать скриншоты;
- сохранять/загружать игру;
- делать assert состояния.

## 7. Требования к диагностике

Любой loader/registry/runner должен возвращать структурированные ошибки.

Плохая ошибка:

```text
failed to load mod
```

Хорошая ошибка:

```text
ModManifestError:
  mod: example_mod
  file: mods/example_mod/manifest.toml
  field: depends[0]
  reason: invalid dependency constraint "base >> 1.0"
```

## 8. Требования к отчету после этапа

```text
Implemented:
- ...

Manual verification:
- command: ...
- expected result: ...
- actual result: ...

Automated checks:
- cargo fmt --all --check: pass/fail
- cargo clippy ...: pass/fail
- cargo test --workspace: pass/fail
- python3 scripts/check_plan_index.py: pass/fail

Touched files:
- ...

Known limitations:
- ...
```

## 9. Definition of Done для любого этапа

Этап не завершен, если:

- код не компилируется;
- нет тестов для новой логики;
- нет ручной проверки;
- нет понятного visible result;
- изменены файлы вне зоны ответственности без объяснения;
- появились временные hardcode-решения, не указанные в ТЗ;
- сценарный runner сломан;
- `base` mod не загружается после изменений, начиная с этапа S06.
