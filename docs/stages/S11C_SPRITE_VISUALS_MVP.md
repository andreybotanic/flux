# S11C — Sprite Visuals MVP

## Depends on

- `S11A_WORLD_CAMERA_AND_GRID`
- `S11B_WORLD_VISUALIZATION`

---

## Цель этапа

Перейти от debug-colored world visualization к sprite-based rendering.

Этап должен реализовать первую production-oriented visual system, но не должен вводить autotiling, 47-sprite model или neighbor-aware sprite selection.

Главная цель:

```text
prototype -> visual definition -> sprite rendering
```

Важно: рендер газа пока не меняется.

---

## Архитектурное требование

Prototype не должен хранить прямой путь к изображению.

`VisualDefinition` хранится inline внутри prototype.

```rust
pub struct SolidCellPrototype {
    pub visual: VisualDefinition,
}

pub struct StructurePrototype {
    pub visual: VisualDefinition,
}
```

---

## Реализовать

### VisualDefinition

```rust
pub enum VisualDefinition {
    SingleSprite(SingleSpriteVisual),
}
```

---

### SingleSpriteVisual

```rust
pub struct SingleSpriteVisual {
    pub image: AssetPath,
}
```

Необходимо сгенерировать минимальные спрайты для всех твердых клеток и структур. В дальнейшем они будут заменены на реальные спрайты.

---

## Prototype integration

На этапе `S11C` интеграция делается с:

```text
SolidCellPrototype
StructurePrototype
```

Пример:

```ron
SolidCellPrototype(
    id: "base:solid_cell/floor_cell",
    display_name: "$base.solid_cell.floor_cell",
    gas_permeable: false,
    visual: VisualDefinition(
        kind: SingleSprite(
            image: "textures/solid/floor_cell.png",
        ),
    ),
)
```

---

## Формат visual definition

```ron
StructurePrototype(
    id: "base:building/gas_pump",
    display_name: "$base.structure.gas_pump",
    size: (width: 2, height: 1),
    visual: VisualDefinition(
        kind: SingleSprite(
            image: "textures/structure/gas_pump.png",
        ),
    ),
)
```

---

## Рендер

Рендер solid cells должен работать через:

```text
SolidCellPrototype
    -> VisualDefinition
        -> Bevy image/sprite
```

Рендер structures должен работать через:

```text
StructurePrototype
    -> VisualDefinition
        -> Bevy image/sprite
```

Запрещено:

```text
prototype -> direct image path
```

---

## Структуры

Поскольку структуры могут иметь разные клеточные размеры, то их спрайты не обязаны быть квадратными, в отличие от спрайтов для твердых клеток. Соотношение сторон спрайта должно соответствовать соотношению сторон структуры. Спрайт должен рендериться один на всю область структуры.

---

## Asset loading

На этапе `S11C` достаточно поддержки:

```text
png
```

Спрайты должны лежать в моде, который определяет соответствующие сущности
(например, `mods/base/assets/...` для `base`-контента).

Поддержка:

```text
ktx2
texture arrays
atlas pipeline
```

не входит в этап.

---

## Что НЕ входит в этап

Этап не реализует:

```text
- autotiling;
- 47-sprite model;
- neighbor-aware visuals;
- tile transitions;
- corner blending;
- state-driven sprite selection;
- animation;
- overlays;
- lighting;
- texture atlas pipeline;
- texture arrays;
- GPU render optimization;
- sprite batching optimization.
```

---

## Важное ограничение

Несмотря на то, что в `S11C` реализуется только:

```rust
VisualDefinition::SingleSprite
```

архитектура не должна предполагать, что у prototype всегда ровно один sprite.

`VisualDefinition` должен оставаться расширяемым enum.

Будущие варианты могут включать:

```rust
AutoTile47(...)
VariantSet(...)
StateDriven(...)
```

без изменения prototype API.

---

## Разрешенный уровень реализации

На этапе допустимо:

```text
1 world cell -> 1 Bevy sprite/entity
```

Этап не обязан быть production-performance-ready.

---

## Automated checks

Обязательные команды:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Unit/integration tests:

```text
- parses inline VisualDefinition for solid and structure;
- validates prototype visual fields;
- solid cells render through VisualDefinition;
- structure render uses VisualDefinition;
- renderer does not read sprite path directly from prototype.
```

---

## Manual verification

Ожидаемый результат:

```text
- открывается окно;
- solid cells отображаются спрайтами;
- структуры отображаются спрайтами;
- разные SolidCellPrototype могут использовать разные спрайты;
- отсутствующий asset вызывает ошибку загрузки плагина;
- debug visualization газа из S11B остается работоспособной.
```

---

## Definition of Done

Этап завершен, если:

- существует `VisualDefinition`;
- реализован `SingleSpriteVisual`;
- `SolidCellPrototype` использует inline `VisualDefinition`;
- `StructurePrototype` использует inline `VisualDefinition`;
- solid cells отображаются спрайтами;
- structures отображаются спрайтами;
- prototype не хранит прямой путь к изображению;
- renderer использует `VisualDefinition`;
- architecture не зашита под single-sprite-only модель;
- autotiling/47-sprite model не реализованы;
- все automated checks проходят.

