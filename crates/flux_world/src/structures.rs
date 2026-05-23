use std::collections::HashMap;

use flux_content::TileSize;
use slotmap::{SlotMap, new_key_type};

use crate::{StructurePrototypeId, TilePos};

new_key_type! {
    pub struct StructureInstanceId;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureInstance {
    pub prototype: StructurePrototypeId,
    pub origin: TilePos,
    pub size: TileSize,
}

impl StructureInstance {
    #[must_use]
    pub fn occupied_tiles(&self) -> Vec<TilePos> {
        let mut positions =
            Vec::with_capacity(usize::from(self.size.width) * usize::from(self.size.height));
        for dy in 0..u32::from(self.size.height) {
            for dx in 0..u32::from(self.size.width) {
                positions.push(TilePos {
                    x: self.origin.x + dx,
                    y: self.origin.y + dy,
                });
            }
        }
        positions
    }
}

#[derive(Debug, Clone)]
pub struct StructureStore {
    pub instances: SlotMap<StructureInstanceId, StructureInstance>,
}

impl StructureStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            instances: SlotMap::with_key(),
        }
    }

    pub fn insert(&mut self, instance: StructureInstance) -> StructureInstanceId {
        self.instances.insert(instance)
    }

    #[must_use]
    pub fn get(&self, id: StructureInstanceId) -> Option<&StructureInstance> {
        self.instances.get(id)
    }

    pub fn remove(&mut self, id: StructureInstanceId) -> Option<StructureInstance> {
        self.instances.remove(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

impl Default for StructureStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StructureOccupancyIndex {
    occupied: HashMap<TilePos, StructureInstanceId>,
}

impl StructureOccupancyIndex {
    #[must_use]
    pub fn get(&self, pos: TilePos) -> Option<StructureInstanceId> {
        self.occupied.get(&pos).copied()
    }

    #[must_use]
    pub fn is_occupied(&self, pos: TilePos) -> bool {
        self.occupied.contains_key(&pos)
    }

    pub(crate) fn occupy(
        &mut self,
        pos: TilePos,
        instance_id: StructureInstanceId,
    ) -> Option<StructureInstanceId> {
        self.occupied.insert(pos, instance_id)
    }

    pub(crate) fn clear_tile(&mut self, pos: TilePos) {
        self.occupied.remove(&pos);
    }
}
