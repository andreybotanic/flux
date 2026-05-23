#![forbid(unsafe_code)]

mod error;
mod gas;
mod solid;
mod structures;
mod types;
mod world;

pub use error::{GasMixtureError, StructurePlacementError, WorldGridError};
pub use gas::{GasComponent, GasLayer, GasMixture, ParticleCount};
pub use solid::SolidCellLayer;
pub use structures::{
    StructureInstance, StructureInstanceId, StructureOccupancyIndex, StructureStore,
};
pub use types::{
    CellIndex, ChunkCoord, DirtyKind, GasPrototypeId, GridSize, SolidCellPrototypeId,
    StructurePrototypeId, SubstancePrototypeId, TilePos, TileRect,
};
pub use world::{ChunkMeta, WorldGrid};
