# S07 — WorldGrid SoA + chunk metadata

## Depends on

- S02

## Можно выполнять параллельно с

- S04
- S05
- S06

## Цель этапа

Реализовать минимальную модель мира WorldGrid с плотными SoA-слоями для клеточной симуляции и chunk metadata.
Этап должен зафиксировать, как именно данные мира хранятся в памяти, но не должен реализовывать физику газа, жидкости, теплопроводность, строительство или поведение структур.
Главная цель: WorldGrid умеет хранить твердые клетки, газовые смеси и разреженные структуры.

## Зона ответственности
- Создать crate `flux_world`;
- базовые координаты мира;
- индексирование клеток;
- плотные SoA-слои:
  - solid-cell layer;
  - gas layer;
- sparse-хранилище структур;
- occupancy index для структур;
- chunk metadata;
- dirty flags для render/save;
- unit tests для layout/invariants;
- debug summary command или manual verification path.

## Используемые prototype types
На этом этапе WorldGrid работает только с уже существующими prototype ID следующих типов:
- SolidCellPrototype
- GasPrototype
- StructurePrototype
- SubstancePrototype

WorldGrid не должен владеть самими prototype definitions.
Он хранит только ссылки/handles/IDs на зарегистрированные прототипы.


## Основной принцип хранения
Плотные массивы используются только для данных, которые потенциально читаются массово по всем клеткам:
- solid cells
- gas components

Структуры хранятся разреженно: StructureStore + StructureOccupancyIndex

## Required data layout
Минимальная целевая форма:
```rust
pub struct WorldGrid {
    pub size: GridSize,
    pub chunk_size: u32,

    pub solid_cells: SolidCellLayer,
    pub gases: GasLayer,

    pub chunks: Vec<ChunkMeta>,

    pub structures: StructureStore,
    pub structure_occupancy: StructureOccupancyIndex,
}
```

WorldGrid должен содержать все необходимые API для получения данных о мире и для его мутации (не через прямой доступ к вложенным объектам, а через API каждого отдельного слоя).

### GasLayer
```rust
pub struct GasLayer {
    pub cells: Vec<GasMixture>,
}

pub struct GasMixture {
    components: Vec<GasComponent>,
}

pub struct GasComponent {
    pub gas: GasPrototypeId,
    pub particles: ParticleCount,
}

pub struct ParticleCount(pub u64);
```

GasMixture должен скрывать Vec за методами, чтобы не ломались инварианты:
```rust
impl GasMixture {
    pub fn components(&self) -> &[GasComponent];

    pub fn particles_of(
        &self,
        gas: GasPrototypeId,
    ) -> ParticleCount;

    pub fn set_particles(
        &mut self,
        gas: GasPrototypeId,
        particles: ParticleCount,
    );

    pub fn add_particles(
        &mut self,
        gas: GasPrototypeId,
        particles: ParticleCount,
    ) -> Result<(), GasMixtureError>;

    pub fn remove_particles(
        &mut self,
        gas: GasPrototypeId,
        particles: ParticleCount,
    ) -> Result<(), GasMixtureError>;

    pub fn clear_gas(
        &mut self,
        gas: GasPrototypeId,
    );

    pub fn clear_all(&mut self);

    pub fn total_particles(&self) -> ParticleCount;
}
```

Инварианты:
- в одной смеси не может быть двух компонентов с одинаковым GasPrototypeId;
- компоненты с 0 particles не хранятся;
- total_particles = сумма particles всех компонентов;
- GasMixture может быть пустой.

### SolidCellLayer
Твердые клетки мира хранятся плотным SoA-слоем.
```rust
pub struct SolidCellLayer {
    pub cells: Vec<Option<SolidCellPrototypeId>>,
}
```

### StructureStore
Структуры хранятся разреженно.
```rust
pub struct StructureStore {
    pub instances: SlotMap<StructureInstanceId, StructureInstance>,
}

pub struct StructureInstance {
    pub prototype: StructurePrototypeId,
    pub origin: TilePos,
}
```

### StructureOccupancyIndex
Occupancy index нужен для быстрых запросов: какая структура занимает эту клетку?
Но он не является основным хранилищем структур.
```rust
pub struct StructureOccupancyIndex {
    occupied: HashMap<TilePos, StructureInstanceId>,
}
```

Минимальный API:
```rust
impl StructureOccupancyIndex {
    pub fn get(
        &self,
        pos: TilePos,
    ) -> Option<StructureInstanceId>;

    pub fn is_occupied(
        &self,
        pos: TilePos,
    ) -> bool;
}
```

Размещение строений может быть минимальным:
```rust
impl WorldGrid {
    pub fn place_structure(
        &mut self,
        prototype: StructurePrototypeId,
        origin: TilePos,
    ) -> Result<StructureInstanceId, StructurePlacementError>;
}
```

Требования:
- структура не может выходить за границы мира;
- на этапе S07 структура не может пересекаться с другой структурой;
- все занятые клетки должны попасть в occupancy index;
- удаление структуры должно очищать occupancy index.

## Public API WorldGrid
```rust
impl WorldGrid {
    pub fn new(
        size: GridSize,
        chunk_size: u32,
    ) -> Result<Self, WorldGridError>;

    pub fn cell_index(
        &self,
        pos: TilePos,
    ) -> Option<CellIndex>;

    pub fn chunk_coord_for_pos(
        &self,
        pos: TilePos,
    ) -> Option<ChunkCoord>;

    pub fn mark_cell_dirty(
        &mut self,
        pos: TilePos,
        dirty: DirtyKind,
    ) -> Result<(), WorldGridError>;
}
```

## Chunk metadata
Chunk metadata вводится как организационный слой, но не как culling/sleeping механика.
```rust
pub struct ChunkMeta {
    pub coord: ChunkCoord,
    pub bounds: TileRect,
}
```
Требования:
- чанки покрывают весь мир;
- каждая клетка принадлежит ровно одному чанку;
- крайние чанки могут быть меньше chunk_size, если размер мира не делится нацело.

## Запрещено

- Не использовать Bevy ECS для клеток.
- Не добавлять физику.
- Не добавлять render.
- Не делать sleeping chunks.

## Ручная проверка

1. Запустить debug command `--world-debug-create 64x64` или эквивалент.
2. Убедиться, что summary показывает размер мира, число клеток, chunk size и число чанков.


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Unit tests для flux_world:

WorldGrid:
- creates grid with expected cell count;
- rejects zero width/height;
- rejects zero chunk size;
- converts TilePos -> CellIndex row-major;
- rejects out-of-bounds TilePos;
- computes chunk coords;
- chunks cover all cells exactly once;
- edge chunks have correct bounds.

SolidCellLayer:
- default cells are empty;
- can set solid cell prototype id.

GasLayer:
- default cell has no gas components;
- cell may contain multiple gas components;
- total_particles sums all components;
- zero-particle components are dropped;
- duplicate gas ids are merged or rejected according to chosen rule;
- component order is deterministic;
- clear removes all gas components.

StructureStore / Occupancy:
- structure can be placed;
- structure outside world is rejected;
- overlapping structures are rejected;
- occupied cells resolve to placed structure id;
- unoccupied cells return None.


## Definition of Done

- Реализована только зона ответственности этапа.
- Все automated checks проходят.
- Выполнена ручная проверка из этого документа.
- Нет изменений вне зоны ответственности без объяснения.
- Отчет этапа заполнен по `docs/STAGE_COMPLETION_REPORT_TEMPLATE.md`.
