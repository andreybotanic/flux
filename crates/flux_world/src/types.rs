use flux_core::PrototypeId;

pub type SolidCellPrototypeId = PrototypeId;
pub type GasPrototypeId = PrototypeId;
pub type StructurePrototypeId = PrototypeId;
pub type SubstancePrototypeId = PrototypeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    pub width: u32,
    pub height: u32,
}

impl GridSize {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub fn cell_count(self) -> Option<usize> {
        let count = u64::from(self.width) * u64::from(self.height);
        usize::try_from(count).ok()
    }

    #[must_use]
    pub const fn contains(self, pos: TilePos) -> bool {
        pos.x < self.width && pos.y < self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TilePos {
    pub x: u32,
    pub y: u32,
}

impl TilePos {
    #[must_use]
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellIndex(pub usize);
