# FluxEngine Rewrite — общий overview

Этот документ нужно читать перед реализацией каждого этапа. Его задача — сохранять целостность архитектуры, чтобы отдельные задачи не превращали проект в набор несовместимых решений.

## 1. Цель проекта

FluxEngine — 2D-игра в духе Oxygen Not Included:

- мир состоит из тайлов;
- в клетках могут быть твердые тела, жидкости, газы, температура, здания, предметы;
- симуляция должна быть достаточно физичной, но игровой и управляемой;
- игра должна изначально поддерживать моды и DLC;
- моды должны уметь добавлять контент, патчить существующий контент, добавлять UI, влиять на визуальное отображение и описывать пользовательские сценарии;
- часть вычислений в будущем должна выноситься на GPU.

## 2. Главный архитектурный принцип

Проект не должен быть просто “игрой на Bevy”.

Правильная формулировка:

> FluxEngine — это модифицируемая симуляционная платформа, у которой Bevy используется как runtime-shell, renderer, input/UI layer и интеграционная среда.

Следствие:

- Bevy ECS не является моделью мира;
- моды не получают прямой доступ к `bevy::prelude::World`;
- игровой контент описывается через реестры и стабильные namespaced ID;
- `base` — это не хардкодная часть игры, а официальный базовый мод;
- DLC — это официальный модовый слой;
- UI расширяется через registry/extension points;
- GPU является backend-ускорителем отдельных расчетов, а не обязательной основой всей игры.

## 3. Зафиксированные решения

### 3.1. ID

Единственный допустимый формат:

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

### 3.2. Форматы данных

| Назначение | Формат |
|---|---|
| mod manifest | TOML |
| project/runtime config | TOML |
| content prototypes | RON |
| content patches | RON |
| scenario definitions | RON |
| save manifest | JSON |
| save payload/chunks | binary |
| command/replay log | JSON или NDJSON |
| diagnostic summaries | JSON |
| screenshots | PNG |

### 3.3. Naming conventions

- Crates: `flux_core`, `flux_world`, `flux_sim`.
- IDs: lower snake case namespace and lower snake case slash-separated path.
- Public Rust types: `PascalCase`.
- Acronyms in public Rust types: `Ui`, `Gpu`, `Cpu`, `Dlc`, `Wasm`, not `UI`, `GPU`, `CPU`, `DLC`, `WASM`.

Подробности: `docs/02_PROJECT_CONVENTIONS.md`.

## 4. Модель моддинга

Моды проходят через lifecycle:

1. Discover — найти моды.
2. Resolve — проверить зависимости и load order.
3. Settings stage — объявить настройки.
4. Prototype stage — зарегистрировать и пропатчить контент.
5. Validation stage — проверить схемы, ссылки, ID, ассеты.
6. Freeze registries — сделать runtime-реестры immutable.
7. Save migration stage — применить миграции при загрузке сейва.
8. Runtime stage — моды получают события и отправляют команды.

## 5. Сценарные моды — обязательная часть платформы

Сценарный мод нужен для:

- smoke-тестов;
- регрессионных тестов;
- автопрогонов;
- тестирования UI;
- снятия скриншотов;
- записи диагностических логов;
- воспроизведения действий игрока;
- проверки сохранений/загрузок;
- будущих benchmark-сценариев.

Сценарии должны быть модовым контентом, а не отдельным test-only хаком.

Пример будущего формата:

```ron
Scenario(
  id: "test_scenarios:scenario/bootstrap_smoke",
  steps: [
    WaitTicks(5),
    Log("scenario started"),
    OpenMenu("base:menu/main"),
    TakeScreenshot("main_menu.png"),
    CloseMenu,
    CreateWorld(width: 64, height: 64, seed: 1),
    SaveGame("scenario_save_01"),
    LoadGame("scenario_save_01"),
    AssertWorldLoaded,
    Log("scenario finished"),
  ],
)
```

## 6. Модель мира

На старте используется компромисс:

- данные мира лежат в глобальных SoA-массивах;
- поверх них есть `ChunkMeta`;
- весь мир обсчитывается каждый fixed tick;
- dirty flags используются только для рендера, сохранения и диагностики.

Чанки не используются для пропуска симуляции на ранних этапах.

## 7. Fixed timestep

Симуляция должна идти в fixed timestep.

Принципиально:

- симуляция не должна зависеть от FPS;
- порядок обхода не должен случайно менять результат;
- для физических слоев предпочтительны double buffering или фазы delta/apply;
- поведение, нужное для тестов, должно быть воспроизводимым.

## 8. UI

UI строится через собственный слой поверх Bevy UI:

- `UiRegistry`;
- `MenuRegistry`;
- `WindowRegistry`;
- `WidgetRegistry`;
- `ActionRegistry`;
- `ThemeRegistry`;
- `BindingRegistry`.

Мод не должен напрямую спавнить Bevy UI.

## 9. GPU

GPU не должен внедряться до появления CPU reference implementation.

Правильная последовательность:

1. CPU reference.
2. Абстракция backend.
3. GPU spike на одном простом расчете.
4. Сравнение CPU/GPU на тестовой сцене.
5. Fallback на CPU.

## 10. Критерии качества каждого этапа

Каждый этап должен иметь:

- четкую зону ответственности;
- список разрешенных файлов/крейтoв;
- список запрещенных изменений;
- automated checks;
- manual verification;
- Definition of Done;
- список зависимостей;
- артефакт, который можно увидеть или запустить вручную.

Нельзя завершать этап словами “архитектура подготовлена”, если нет проверяемого результата.
