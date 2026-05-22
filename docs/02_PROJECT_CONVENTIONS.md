# FluxEngine — зафиксированные conventions

Этот документ фиксирует решения, которые нельзя менять локально внутри этапов.

## 1. ID format

Единственный допустимый формат ID:

```text
namespace:path/to/item
```

Примеры:

```text
base:material/oxygen
base:material/water
base:building/gas_pump
base:ui/main_menu
base:overlay/temperature
example_mod:scenario/bootstrap_smoke
```

### 1.1. Namespace

Namespace:

- lower snake case;
- начинается с латинской буквы;
- допускает латинские буквы, цифры и `_`;
- не содержит `-`, `.`, `/`, `:`.

Примеры:

```text
base
advanced_chemistry
cool_pipes
```

### 1.2. Path

Path:

- slash-separated;
- каждый segment lower snake case;
- первый segment обычно указывает категорию: `material`, `building`, `ui`, `scenario`, `overlay`, `recipe`;
- не содержит пробелов, точек, обратных слешей.

Примеры:

```text
material/oxygen
building/gas_pump
ui/settings/pipes_tab
scenario/bootstrap_smoke
```

### 1.3. Forbidden ID formats

Не использовать:

```text
base.building.gas_pump
base/building/gas_pump
building:gas_pump
base:Building/GasPump
```

## 2. Data formats

| Назначение | Формат | Комментарий |
|---|---|---|
| mod manifest | TOML | `mods/<mod_id>/manifest.toml` |
| project/runtime config | TOML | user/project settings |
| content prototypes | RON | readable typed data |
| content patches | RON | same format family as prototypes |
| scenario definitions | RON | declarative scenario DSL |
| save manifest | JSON | easy diagnostics/versioning |
| save payload/chunks | binary | exact format вводится позже |
| command/replay log | JSON или NDJSON | deterministic replay/debug |
| diagnostic summaries | JSON | CI/scenario artifacts |
| screenshots | PNG | scenario artifacts |

YAML запрещен для игровых данных, сценариев, контента и runtime-конфигов.

## 3. Rust naming

### 3.1. Crates

Все internal crates называются через snake case и префикс `flux_`:

```text
flux_core
flux_content
flux_mod_loader
flux_ui
flux_world
flux_sim
flux_save
flux_gpu
flux_render
```

### 3.2. Public types

Публичные типы — `PascalCase`:

```rust
NamespacedId
ContentRegistry
WorldGrid
ChunkMeta
ScenarioRunner
```

### 3.3. Acronyms

В публичных типах использовать Rust-style CamelCase acronyms:

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
WASMPluginRuntime
```

### 3.4. Suffixes

- ID wrapper types end with `Id`: `MaterialId`, `UiPanelId`.
- Handles end with `Handle`: `PrototypeHandle`.
- Errors end with `Error`: `ModManifestError`.
- Registries end with `Registry`: `ContentRegistry`.
- Runners end with `Runner`: `ScenarioRunner`.
- Commands end with `Command` or are enum variants inside `SimCommand`.
- Events end with `Event` or are enum variants inside `SimEvent`.

## 4. File and directory naming

- Rust modules/files: `snake_case.rs`.
- Stage docs: `S00_REPO_BOOTSTRAP.md`.
- Mod directories: same as mod ID namespace, lower snake case.
- Content files: descriptive lower snake case, e.g. `materials.ron`, `buildings.ron`.

## 5. Cargo/workspace conventions

- Workspace crates live under `crates/`.
- Public API crates should avoid Bevy dependency unless the crate's responsibility explicitly requires Bevy.
- `flux_core` must remain Bevy-free.
- Avoid adding dependencies without clear stage need.
- Prefer small internal crates with explicit responsibility.

## 6. Serialization conventions

- Any type crossing mod/save/scenario boundaries must have an explicit schema decision.
- Do not serialize internal memory layout accidentally.
- Do not serialize numeric registry indices as stable save identifiers.
- Saves must use stable namespaced IDs where content identity matters.
