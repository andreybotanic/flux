use crate::SolidCellPrototypeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolidCellLayer {
    cells: Vec<Option<SolidCellPrototypeId>>,
}

impl SolidCellLayer {
    #[must_use]
    pub fn new(cell_count: usize) -> Self {
        Self {
            cells: vec![None; cell_count],
        }
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<Option<SolidCellPrototypeId>> {
        self.cells.get(index).cloned()
    }

    pub fn set(
        &mut self,
        index: usize,
        solid: Option<SolidCellPrototypeId>,
    ) -> Option<Option<SolidCellPrototypeId>> {
        self.cells
            .get_mut(index)
            .map(|slot| std::mem::replace(slot, solid))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use flux_core::PrototypeId;

    use super::*;

    #[test]
    fn default_cells_are_empty() {
        let layer = SolidCellLayer::new(8);
        assert_eq!(layer.len(), 8);
        assert_eq!(layer.get(0), Some(None));
    }

    #[test]
    fn can_set_solid_cell_id() {
        let mut layer = SolidCellLayer::new(1);
        let wall = PrototypeId::parse("base:solid_cell/wall").expect("valid prototype id");

        assert_eq!(layer.set(0, Some(wall.clone())), Some(None));
        assert_eq!(layer.get(0), Some(Some(wall)));
    }
}
