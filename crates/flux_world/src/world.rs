use std::collections::HashMap;

use flux_content::{ContentRegistry, TileSize};

use crate::{
    CellIndex, ChunkCoord, DirtyKind, GasLayer, GasMixture, GasPrototypeId, GridSize,
    ParticleCount, SolidCellLayer, SolidCellPrototypeId, StructureInstance, StructureInstanceId,
    StructureOccupancyIndex, StructurePlacementError, StructurePrototypeId, StructureStore,
    TilePos, TileRect, WorldGridError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkMeta {
    pub coord: ChunkCoord,
    pub bounds: TileRect,
    pub render_dirty: bool,
    pub save_dirty: bool,
}

#[derive(Debug, Clone)]
pub struct WorldGrid {
    size: GridSize,
    chunk_size: u32,
    solid_cells: SolidCellLayer,
    gases: GasLayer,
    chunks: Vec<ChunkMeta>,
    structures: StructureStore,
    structure_occupancy: StructureOccupancyIndex,
    structure_sizes: HashMap<StructurePrototypeId, TileSize>,
    chunk_cols: u32,
    chunk_rows: u32,
}

impl WorldGrid {
    pub fn new(size: GridSize, chunk_size: u32) -> Result<Self, WorldGridError> {
        if size.width == 0 || size.height == 0 {
            return Err(WorldGridError::InvalidGridSize {
                width: size.width,
                height: size.height,
            });
        }
        if chunk_size == 0 {
            return Err(WorldGridError::InvalidChunkSize { chunk_size });
        }

        let cell_count = size.cell_count().ok_or(WorldGridError::CellCountOverflow {
            width: size.width,
            height: size.height,
        })?;
        let chunk_cols = size.width.div_ceil(chunk_size);
        let chunk_rows = size.height.div_ceil(chunk_size);
        let chunks = build_chunks(size, chunk_size, chunk_cols, chunk_rows);

        Ok(Self {
            size,
            chunk_size,
            solid_cells: SolidCellLayer::new(cell_count),
            gases: GasLayer::new(cell_count),
            chunks,
            structures: StructureStore::new(),
            structure_occupancy: StructureOccupancyIndex::default(),
            structure_sizes: HashMap::new(),
            chunk_cols,
            chunk_rows,
        })
    }

    #[must_use]
    pub const fn size(&self) -> GridSize {
        self.size
    }

    #[must_use]
    pub const fn chunk_size(&self) -> u32 {
        self.chunk_size
    }

    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.solid_cells.len()
    }

    #[must_use]
    pub const fn chunk_cols(&self) -> u32 {
        self.chunk_cols
    }

    #[must_use]
    pub const fn chunk_rows(&self) -> u32 {
        self.chunk_rows
    }

    #[must_use]
    pub fn chunks(&self) -> &[ChunkMeta] {
        &self.chunks
    }

    #[must_use]
    pub fn structures(&self) -> &StructureStore {
        &self.structures
    }

    #[must_use]
    pub fn structure_occupancy(&self) -> &StructureOccupancyIndex {
        &self.structure_occupancy
    }

    #[must_use]
    pub fn solid_cell_at(&self, pos: TilePos) -> Option<Option<SolidCellPrototypeId>> {
        self.cell_index(pos)
            .and_then(|index| self.solid_cells.get(index.0))
    }

    pub fn set_solid_cell_at(
        &mut self,
        pos: TilePos,
        solid: Option<SolidCellPrototypeId>,
    ) -> Result<(), WorldGridError> {
        let index = self
            .cell_index(pos)
            .ok_or_else(|| WorldGridError::position_out_of_bounds(pos, self.size))?;
        self.solid_cells
            .set(index.0, solid)
            .expect("index validated by cell_index");
        self.mark_cell_dirty(pos, DirtyKind::RenderAndSave)
    }

    #[must_use]
    pub fn gas_at(&self, pos: TilePos) -> Option<&GasMixture> {
        self.cell_index(pos)
            .and_then(|index| self.gases.get(index.0))
    }

    pub fn set_gas_particles(
        &mut self,
        pos: TilePos,
        gas: GasPrototypeId,
        particles: ParticleCount,
    ) -> Result<(), WorldGridError> {
        let index = self
            .cell_index(pos)
            .ok_or_else(|| WorldGridError::position_out_of_bounds(pos, self.size))?;
        self.gases
            .get_mut(index.0)
            .expect("index validated by cell_index")
            .set_particles(gas, particles);
        self.mark_cell_dirty(pos, DirtyKind::RenderAndSave)
    }

    #[must_use]
    pub fn cell_index(&self, pos: TilePos) -> Option<CellIndex> {
        if !self.size.contains(pos) {
            return None;
        }
        let width = u64::from(self.size.width);
        let x = u64::from(pos.x);
        let y = u64::from(pos.y);
        let index = y.checked_mul(width)?.checked_add(x)?;
        let index = usize::try_from(index).ok()?;
        Some(CellIndex(index))
    }

    #[must_use]
    pub fn chunk_coord_for_pos(&self, pos: TilePos) -> Option<ChunkCoord> {
        if !self.size.contains(pos) {
            return None;
        }
        Some(ChunkCoord {
            x: pos.x / self.chunk_size,
            y: pos.y / self.chunk_size,
        })
    }

    pub fn mark_cell_dirty(
        &mut self,
        pos: TilePos,
        dirty: DirtyKind,
    ) -> Result<(), WorldGridError> {
        let coord = self
            .chunk_coord_for_pos(pos)
            .ok_or_else(|| WorldGridError::position_out_of_bounds(pos, self.size))?;
        let chunk_index = self.chunk_index(coord)?;
        let chunk = self
            .chunks
            .get_mut(chunk_index)
            .expect("chunk index validated by chunk_index");
        match dirty {
            DirtyKind::Render => chunk.render_dirty = true,
            DirtyKind::Save => chunk.save_dirty = true,
            DirtyKind::RenderAndSave => {
                chunk.render_dirty = true;
                chunk.save_dirty = true;
            }
        }
        Ok(())
    }

    pub fn refresh_structure_sizes_from_registry(&mut self, registry: &ContentRegistry) -> usize {
        self.structure_sizes.clear();
        for record in registry.structures() {
            self.structure_sizes
                .insert(record.prototype.id.clone(), record.prototype.size);
        }
        self.structure_sizes.len()
    }

    pub fn place_structure(
        &mut self,
        prototype: StructurePrototypeId,
        origin: TilePos,
    ) -> Result<StructureInstanceId, StructurePlacementError> {
        let size = self
            .structure_sizes
            .get(&prototype)
            .copied()
            .ok_or_else(|| StructurePlacementError::MissingPrototypeSize {
                prototype_id: prototype.clone(),
            })?;

        let occupied = self.footprint_tiles(origin, size)?;
        for pos in &occupied {
            if let Some(existing) = self.structure_occupancy.get(*pos) {
                return Err(StructurePlacementError::Occupied {
                    pos_x: pos.x,
                    pos_y: pos.y,
                    existing,
                });
            }
        }

        let instance_id = self.structures.insert(StructureInstance {
            prototype,
            origin,
            size,
        });
        for pos in &occupied {
            self.structure_occupancy.occupy(*pos, instance_id);
            self.mark_cell_dirty(*pos, DirtyKind::RenderAndSave)
                .map_err(|_| StructurePlacementError::OutOfBounds {
                    pos_x: pos.x,
                    pos_y: pos.y,
                })?;
        }
        Ok(instance_id)
    }

    pub fn remove_structure(
        &mut self,
        instance_id: StructureInstanceId,
    ) -> Result<(), StructurePlacementError> {
        let instance = self
            .structures
            .remove(instance_id)
            .ok_or(StructurePlacementError::InstanceNotFound { instance_id })?;
        let occupied = self.footprint_tiles(instance.origin, instance.size)?;
        for pos in occupied {
            self.structure_occupancy.clear_tile(pos);
            self.mark_cell_dirty(pos, DirtyKind::RenderAndSave)
                .map_err(|_| StructurePlacementError::OutOfBounds {
                    pos_x: pos.x,
                    pos_y: pos.y,
                })?;
        }
        Ok(())
    }

    fn footprint_tiles(
        &self,
        origin: TilePos,
        size: TileSize,
    ) -> Result<Vec<TilePos>, StructurePlacementError> {
        let mut positions = Vec::with_capacity(usize::from(size.width) * usize::from(size.height));
        for dy in 0..u32::from(size.height) {
            for dx in 0..u32::from(size.width) {
                let x = origin
                    .x
                    .checked_add(dx)
                    .ok_or(StructurePlacementError::OutOfBounds {
                        pos_x: origin.x,
                        pos_y: origin.y,
                    })?;
                let y = origin
                    .y
                    .checked_add(dy)
                    .ok_or(StructurePlacementError::OutOfBounds {
                        pos_x: origin.x,
                        pos_y: origin.y,
                    })?;
                let pos = TilePos { x, y };
                if !self.size.contains(pos) {
                    return Err(StructurePlacementError::OutOfBounds { pos_x: x, pos_y: y });
                }
                positions.push(pos);
            }
        }
        Ok(positions)
    }

    fn chunk_index(&self, coord: ChunkCoord) -> Result<usize, WorldGridError> {
        if coord.x >= self.chunk_cols || coord.y >= self.chunk_rows {
            return Err(WorldGridError::chunk_out_of_bounds(
                coord,
                self.chunk_cols,
                self.chunk_rows,
            ));
        }
        let index_u64 = u64::from(coord.y) * u64::from(self.chunk_cols) + u64::from(coord.x);
        usize::try_from(index_u64).map_err(|_| {
            WorldGridError::chunk_out_of_bounds(coord, self.chunk_cols, self.chunk_rows)
        })
    }
}

fn build_chunks(
    size: GridSize,
    chunk_size: u32,
    chunk_cols: u32,
    chunk_rows: u32,
) -> Vec<ChunkMeta> {
    let mut chunks = Vec::with_capacity(
        usize::try_from(u64::from(chunk_cols) * u64::from(chunk_rows))
            .expect("chunk count always fits in usize because cells fit in usize"),
    );

    for chunk_y in 0..chunk_rows {
        for chunk_x in 0..chunk_cols {
            let origin_x = chunk_x * chunk_size;
            let origin_y = chunk_y * chunk_size;
            let width = chunk_size.min(size.width - origin_x);
            let height = chunk_size.min(size.height - origin_y);
            chunks.push(ChunkMeta {
                coord: ChunkCoord {
                    x: chunk_x,
                    y: chunk_y,
                },
                bounds: TileRect::new(TilePos::new(origin_x, origin_y), width, height),
                render_dirty: false,
                save_dirty: false,
            });
        }
    }

    chunks
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use flux_content::TileSize;
    use flux_core::PrototypeId;

    use super::*;

    fn structure_id(value: &str) -> PrototypeId {
        PrototypeId::parse(value).expect("valid structure id")
    }

    fn gas_id(value: &str) -> PrototypeId {
        PrototypeId::parse(value).expect("valid gas id")
    }

    fn set_test_structure_size(
        world: &mut WorldGrid,
        prototype: StructurePrototypeId,
        size: TileSize,
    ) {
        world.structure_sizes.insert(prototype, size);
    }

    #[test]
    fn creates_grid_with_expected_cell_count() {
        let world = WorldGrid::new(GridSize::new(64, 64), 16).expect("world should be created");
        assert_eq!(world.cell_count(), 64 * 64);
    }

    #[test]
    fn rejects_zero_width_height_or_chunk_size() {
        assert!(matches!(
            WorldGrid::new(GridSize::new(0, 4), 16),
            Err(WorldGridError::InvalidGridSize { .. })
        ));
        assert!(matches!(
            WorldGrid::new(GridSize::new(4, 0), 16),
            Err(WorldGridError::InvalidGridSize { .. })
        ));
        assert!(matches!(
            WorldGrid::new(GridSize::new(4, 4), 0),
            Err(WorldGridError::InvalidChunkSize { .. })
        ));
    }

    #[test]
    fn converts_tile_pos_to_row_major_index() {
        let world = WorldGrid::new(GridSize::new(10, 10), 4).expect("world should be created");
        assert_eq!(world.cell_index(TilePos::new(0, 0)), Some(CellIndex(0)));
        assert_eq!(world.cell_index(TilePos::new(1, 0)), Some(CellIndex(1)));
        assert_eq!(world.cell_index(TilePos::new(0, 1)), Some(CellIndex(10)));
        assert_eq!(world.cell_index(TilePos::new(9, 9)), Some(CellIndex(99)));
    }

    #[test]
    fn rejects_out_of_bounds_tile_pos() {
        let world = WorldGrid::new(GridSize::new(4, 4), 2).expect("world should be created");
        assert_eq!(world.cell_index(TilePos::new(4, 0)), None);
        assert_eq!(world.cell_index(TilePos::new(0, 4)), None);
        assert_eq!(world.chunk_coord_for_pos(TilePos::new(4, 0)), None);
    }

    #[test]
    fn computes_chunk_coords() {
        let world = WorldGrid::new(GridSize::new(64, 64), 16).expect("world should be created");
        assert_eq!(
            world.chunk_coord_for_pos(TilePos::new(0, 0)),
            Some(ChunkCoord::new(0, 0))
        );
        assert_eq!(
            world.chunk_coord_for_pos(TilePos::new(17, 3)),
            Some(ChunkCoord::new(1, 0))
        );
        assert_eq!(
            world.chunk_coord_for_pos(TilePos::new(63, 63)),
            Some(ChunkCoord::new(3, 3))
        );
    }

    #[test]
    fn chunks_cover_all_cells_exactly_once() {
        let world = WorldGrid::new(GridSize::new(33, 17), 8).expect("world should be created");
        let mut covered = HashSet::new();
        for chunk in world.chunks() {
            for y in chunk.bounds.origin.y..(chunk.bounds.origin.y + chunk.bounds.height) {
                for x in chunk.bounds.origin.x..(chunk.bounds.origin.x + chunk.bounds.width) {
                    let pos = TilePos::new(x, y);
                    assert!(
                        chunk.bounds.contains(pos),
                        "chunk bounds must contain every iterated position"
                    );
                    assert!(
                        covered.insert(pos),
                        "every cell must belong to exactly one chunk"
                    );
                }
            }
        }
        assert_eq!(covered.len(), world.cell_count());
    }

    #[test]
    fn edge_chunks_have_correct_bounds() {
        let world = WorldGrid::new(GridSize::new(18, 10), 8).expect("world should be created");
        assert_eq!(world.chunk_cols(), 3);
        assert_eq!(world.chunk_rows(), 2);

        let last = world.chunks().last().expect("must have chunks");
        assert_eq!(last.coord, ChunkCoord::new(2, 1));
        assert_eq!(last.bounds.origin, TilePos::new(16, 8));
        assert_eq!(last.bounds.width, 2);
        assert_eq!(last.bounds.height, 2);
    }

    #[test]
    fn structures_can_be_placed_and_removed() {
        let mut world = WorldGrid::new(GridSize::new(16, 16), 8).expect("world should be created");
        let prototype = structure_id("base:structure/pump");
        set_test_structure_size(
            &mut world,
            prototype.clone(),
            TileSize {
                width: 2,
                height: 2,
            },
        );

        let instance = world
            .place_structure(prototype, TilePos::new(3, 4))
            .expect("structure should be placed");
        assert_eq!(world.structures().len(), 1);
        let placed = world
            .structures()
            .get(instance)
            .expect("placed instance should be available in store");
        assert_eq!(placed.size.width, 2);
        assert_eq!(placed.size.height, 2);
        assert!(world.structure_occupancy().is_occupied(TilePos::new(3, 4)));
        assert!(world.structure_occupancy().is_occupied(TilePos::new(4, 5)));

        world
            .remove_structure(instance)
            .expect("structure should be removed");
        assert_eq!(world.structures().len(), 0);
        assert!(!world.structure_occupancy().is_occupied(TilePos::new(3, 4)));
        assert!(!world.structure_occupancy().is_occupied(TilePos::new(4, 5)));
    }

    #[test]
    fn rejects_structure_outside_world() {
        let mut world = WorldGrid::new(GridSize::new(8, 8), 4).expect("world should be created");
        let prototype = structure_id("base:structure/pump");
        set_test_structure_size(
            &mut world,
            prototype.clone(),
            TileSize {
                width: 2,
                height: 2,
            },
        );

        let result = world.place_structure(prototype, TilePos::new(7, 7));
        assert!(matches!(
            result,
            Err(StructurePlacementError::OutOfBounds { .. })
        ));
    }

    #[test]
    fn rejects_overlapping_structures() {
        let mut world = WorldGrid::new(GridSize::new(16, 16), 4).expect("world should be created");
        let prototype = structure_id("base:structure/pump");
        set_test_structure_size(
            &mut world,
            prototype.clone(),
            TileSize {
                width: 2,
                height: 1,
            },
        );
        world
            .place_structure(prototype.clone(), TilePos::new(4, 4))
            .expect("first structure should be placed");

        let result = world.place_structure(prototype, TilePos::new(5, 4));
        assert!(matches!(
            result,
            Err(StructurePlacementError::Occupied { .. })
        ));
    }

    #[test]
    fn unoccupied_cells_return_none() {
        let world = WorldGrid::new(GridSize::new(8, 8), 4).expect("world should be created");
        assert_eq!(world.structure_occupancy().get(TilePos::new(2, 2)), None);
    }

    #[test]
    fn cell_gas_total_particles_stays_correct_after_add_and_remove() {
        let mut world = WorldGrid::new(GridSize::new(8, 8), 4).expect("world should be created");
        let pos = TilePos::new(2, 3);
        let oxygen = gas_id("base:gas/oxygen");
        let hydrogen = gas_id("base:gas/hydrogen");
        let cell_index = world.cell_index(pos).expect("cell should be in bounds").0;

        world
            .gases
            .get_mut(cell_index)
            .expect("cell should exist")
            .add_particles(oxygen.clone(), ParticleCount(10))
            .expect("oxygen should be added");
        assert_eq!(
            world
                .gas_at(pos)
                .expect("cell should exist")
                .total_particles(),
            ParticleCount(10)
        );

        world
            .gases
            .get_mut(cell_index)
            .expect("cell should exist")
            .add_particles(hydrogen.clone(), ParticleCount(5))
            .expect("hydrogen should be added");
        assert_eq!(
            world
                .gas_at(pos)
                .expect("cell should exist")
                .total_particles(),
            ParticleCount(15)
        );

        world
            .gases
            .get_mut(cell_index)
            .expect("cell should exist")
            .add_particles(oxygen.clone(), ParticleCount(3))
            .expect("oxygen should be increased");
        assert_eq!(
            world
                .gas_at(pos)
                .expect("cell should exist")
                .total_particles(),
            ParticleCount(18)
        );

        world
            .gases
            .get_mut(cell_index)
            .expect("cell should exist")
            .remove_particles(oxygen, ParticleCount(4))
            .expect("oxygen should be removed");
        assert_eq!(
            world
                .gas_at(pos)
                .expect("cell should exist")
                .total_particles(),
            ParticleCount(14)
        );
    }
}
