use thiserror::Error;

use crate::{BackendPolicy, SimulationStageId};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SimError {
    #[error("invalid fixed tick step: duration must be greater than zero")]
    InvalidFixedTickStep,

    #[error("invalid world size: width and height must be greater than zero, got {width}x{height}")]
    InvalidWorldSize { width: u32, height: u32 },

    #[error("failed to create world {width}x{height}: {source}")]
    WorldCreationFailed {
        width: u32,
        height: u32,
        source: flux_world::WorldGridError,
    },

    #[error("tick counter overflow: current={current}, delta={delta}")]
    TickCounterOverflow { current: u64, delta: u64 },

    #[error(
        "invalid stage frequency divider for stage `{stage_id}`: frequency_divider must be greater than zero, got {frequency_divider}"
    )]
    InvalidStageFrequencyDivider {
        stage_id: SimulationStageId,
        frequency_divider: u64,
    },

    #[error("duplicate stage registration for stage `{stage_id}`")]
    DuplicateStageRegistration { stage_id: SimulationStageId },

    #[error("cannot resolve backend for stage `{stage_id}` with policy `{backend_policy}` in S14")]
    BackendResolutionFailed {
        stage_id: SimulationStageId,
        backend_policy: BackendPolicy,
    },

    #[error("gas permeability mask size mismatch: expected={expected}, actual={actual}")]
    GasPermeabilityMaskSizeMismatch { expected: usize, actual: usize },

    #[error("failed to apply gas diffusion snapshot to world: {source}")]
    GasSnapshotApplyFailed { source: flux_world::WorldGridError },

    #[error("gas particle operation failed during diffusion: {source}")]
    GasParticleOpFailed { source: flux_world::GasMixtureError },

    #[error("gas conservation violated for gas `{gas}`: before={before}, after={after}")]
    GasConservationViolated {
        gas: String,
        before: u64,
        after: u64,
    },
}
