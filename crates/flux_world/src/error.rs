use thiserror::Error;

use crate::{GasPrototypeId, GridSize, StructureInstanceId, StructurePrototypeId, TilePos};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorldGridError {
    #[error("invalid grid size: width and height must be greater than zero, got {width}x{height}")]
    InvalidGridSize { width: u32, height: u32 },

    #[error("grid cell count overflow for {width}x{height}")]
    CellCountOverflow { width: u32, height: u32 },

    #[error("tile position ({pos_x},{pos_y}) is out of bounds for grid {width}x{height}")]
    PositionOutOfBounds {
        pos_x: u32,
        pos_y: u32,
        width: u32,
        height: u32,
    },
}

impl WorldGridError {
    #[must_use]
    pub fn position_out_of_bounds(pos: TilePos, size: GridSize) -> Self {
        Self::PositionOutOfBounds {
            pos_x: pos.x,
            pos_y: pos.y,
            width: size.width,
            height: size.height,
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GasMixtureError {
    #[error("adding particles would overflow for gas `{gas}`: current={current}, delta={delta}")]
    ParticleOverflow {
        gas: GasPrototypeId,
        current: u64,
        delta: u64,
    },

    #[error(
        "cannot remove {requested} particles from gas `{gas}` because only {available} are present"
    )]
    NotEnoughParticles {
        gas: GasPrototypeId,
        available: u64,
        requested: u64,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StructurePlacementError {
    #[error("missing structure size for prototype `{prototype_id}`")]
    MissingPrototypeSize { prototype_id: StructurePrototypeId },

    #[error("structure footprint contains out-of-bounds tile at ({pos_x},{pos_y})")]
    OutOfBounds { pos_x: u32, pos_y: u32 },

    #[error(
        "cannot place structure: tile ({pos_x},{pos_y}) is already occupied by instance {existing:?}"
    )]
    Occupied {
        pos_x: u32,
        pos_y: u32,
        existing: StructureInstanceId,
    },

    #[error("structure instance {instance_id:?} not found")]
    InstanceNotFound { instance_id: StructureInstanceId },
}
