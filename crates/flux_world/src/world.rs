use std::collections::HashMap;

use flux_content::{ContentRegistry, TileSize};

use crate::{
    CellIndex, GasLayer, GasMixture, GasPrototypeId, GridSize, ParticleCount, SolidCellLayer,
    SolidCellPrototypeId, StructureInstance, StructureInstanceId, StructureOccupancyIndex,
    StructurePlacementError, StructurePrototypeId, StructureStore, TilePos, WorldGridError,
};

#[derive(Debug, Clone)]
pub struct WorldGrid {
    size: GridSize,
    solid_cells: SolidCellLayer,
    gases: GasLayer,
    structures: StructureStore,
    structure_occupancy: StructureOccupancyIndex,
    structure_sizes: HashMap<StructurePrototypeId, TileSize>,
}

impl WorldGrid {
    pub fn new(size: GridSize) -> Result<Self, WorldGridError> {
        if size.width == 0 || size.height == 0 {
            return Err(WorldGridError::InvalidGridSize {
                width: size.width,
                height: size.height,
            });
        }

        let cell_count = size.cell_count().ok_or(WorldGridError::CellCountOverflow {
            width: size.width,
            height: size.height,
        })?;

        Ok(Self {
            size,
            solid_cells: SolidCellLayer::new(cell_count),
            gases: GasLayer::new(cell_count),
            structures: StructureStore::new(),
            structure_occupancy: StructureOccupancyIndex::default(),
            structure_sizes: HashMap::new(),
        })
    }

    #[must_use]
    pub const fn size(&self) -> GridSize {
        self.size
    }

    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.solid_cells.len()
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
        Ok(())
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
        Ok(())
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
}

#[cfg(test)]
mod tests {
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
        let world = WorldGrid::new(GridSize::new(64, 64)).expect("world should be created");
        assert_eq!(world.cell_count(), 64 * 64);
    }

    #[test]
    fn dense_layers_match_grid_area() {
        let world = WorldGrid::new(GridSize::new(33, 17)).expect("world should be created");
        let expected = 33 * 17;
        assert_eq!(world.solid_cells.len(), expected);
        assert_eq!(world.gases.len(), expected);
    }

    #[test]
    fn rejects_zero_width_or_height() {
        assert!(matches!(
            WorldGrid::new(GridSize::new(0, 4)),
            Err(WorldGridError::InvalidGridSize { .. })
        ));
        assert!(matches!(
            WorldGrid::new(GridSize::new(4, 0)),
            Err(WorldGridError::InvalidGridSize { .. })
        ));
    }

    #[test]
    fn converts_tile_pos_to_row_major_index() {
        let world = WorldGrid::new(GridSize::new(10, 10)).expect("world should be created");
        assert_eq!(world.cell_index(TilePos::new(0, 0)), Some(CellIndex(0)));
        assert_eq!(world.cell_index(TilePos::new(1, 0)), Some(CellIndex(1)));
        assert_eq!(world.cell_index(TilePos::new(0, 1)), Some(CellIndex(10)));
        assert_eq!(world.cell_index(TilePos::new(9, 9)), Some(CellIndex(99)));
    }

    #[test]
    fn rejects_out_of_bounds_tile_pos() {
        let world = WorldGrid::new(GridSize::new(4, 4)).expect("world should be created");
        assert_eq!(world.cell_index(TilePos::new(4, 0)), None);
        assert_eq!(world.cell_index(TilePos::new(0, 4)), None);
    }

    #[test]
    fn set_cell_and_gas_report_structured_out_of_bounds_errors() {
        let mut world = WorldGrid::new(GridSize::new(2, 2)).expect("world should be created");
        let oxygen = gas_id("base:gas/oxygen");
        let solid = structure_id("base:solid_cell/rock");

        let solid_error = world
            .set_solid_cell_at(TilePos::new(3, 0), Some(solid))
            .expect_err("solid set should fail");
        assert!(matches!(
            solid_error,
            WorldGridError::PositionOutOfBounds { .. }
        ));

        let gas_error = world
            .set_gas_particles(TilePos::new(0, 3), oxygen, ParticleCount(1))
            .expect_err("gas set should fail");
        assert!(matches!(
            gas_error,
            WorldGridError::PositionOutOfBounds { .. }
        ));
    }

    #[test]
    fn structures_can_be_placed_and_removed() {
        let mut world = WorldGrid::new(GridSize::new(16, 16)).expect("world should be created");
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
        let mut world = WorldGrid::new(GridSize::new(8, 8)).expect("world should be created");
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
        let mut world = WorldGrid::new(GridSize::new(16, 16)).expect("world should be created");
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
        let world = WorldGrid::new(GridSize::new(8, 8)).expect("world should be created");
        assert_eq!(world.structure_occupancy().get(TilePos::new(2, 2)), None);
    }

    #[test]
    fn cell_gas_total_particles_stays_correct_after_add_and_remove() {
        let mut world = WorldGrid::new(GridSize::new(8, 8)).expect("world should be created");
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
