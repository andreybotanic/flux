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

Prototype должен ссылаться на отдельную visual definition:

```rust
pub struct SolidCellPrototype {
    pub visual: VisualDefinitionId,
}
```

---

## Реализовать

### VisualDefinitionId

```rust
pub struct VisualDefinitionId(pub NamespacedId);
```

---

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

Необходимо сгенерировать минимальные спрайты для всех твердых клеток и структур. В альнейшем они будут заменены на реальные спрайты.

---

### Visual registry

Добавить:

```text
VisualDefinition registry
```

Реестр должен:

- загружать visual definitions из модов;
- валидировать duplicate IDs;
- валидировать references из prototype;
- поддерживать lookup по `VisualDefinitionId`.

---

## Prototype integration

На этапе `S11C` достаточно интеграции только с:

```text
SolidCellPrototype
```

Пример:

```ron
SolidCellPrototype(
    id: "base:solid_cell/granite",
    display_name: "$base.solid_cell.granite",
    visual: "base:visual/solid/granite",
)
```

---

## Формат visual definition

```ron
VisualDefinition(
    id: "base:visual/solid/granite",
    kind: SingleSprite(
        image: "base/textures/solid/granite.png",
    ),
)
```

VisualDefinition разрешается хранить в том же файле, что и SolidCellPrototype, к которому этот VisualDefinition относится.

---

## Рендер

Рендер solid cells должен работать через:

```text
SolidCellPrototype
    -> VisualDefinitionId
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
- registers visual definition;
- rejects duplicate visual definition id;
- validates prototype visual reference;
- missing visual reference returns structured error;
- missing texture asset uses fallback rendering path;
- solid cells render through VisualDefinition;
- renderer does not read sprite path directly from prototype.
```

---

## Manual verification

Добавить debug scene:

```bash
cargo run -p flux_app -- --world-sprite-debug
```

Ожидаемый результат:

```text
- открывается окно;
- solid cells отображаются спрайтами;
- разные SolidCellPrototype могут использовать разные спрайты;
- отсутствующий asset не вызывает panic;
- fallback visualization работает;
- debug visualization газа из S11B остается работоспособной.
```

---

## Definition of Done

Этап завершен, если:

- существует `VisualDefinitionId`;
- существует `VisualDefinition registry`;
- реализован `SingleSpriteVisual`;
- `SolidCellPrototype` использует `VisualDefinitionId`;
- solid cells отображаются спрайтами;
- prototype не хранит прямой путь к изображению;
- fallback rendering работает;
- renderer использует visual registry;
- architecture не зашита под single-sprite-only модель;
- autotiling/47-sprite model не реализованы;
- все automated checks проходят.