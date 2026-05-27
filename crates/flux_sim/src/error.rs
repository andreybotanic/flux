use thiserror::Error;

use crate::BackendPolicy;

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

    #[error("world mutation failed: world is not loaded for operation `{operation}`")]
    WorldNotLoadedForMutation { operation: &'static str },

    #[error("world mutation failed during `{operation}`: {source}")]
    WorldMutationFailed {
        operation: &'static str,
        source: flux_world::WorldGridError,
    },

    #[error("structure mutation failed during `{operation}`: {source}")]
    StructureMutationFailed {
        operation: &'static str,
        source: flux_world::StructurePlacementError,
    },

    #[error(
        "invalid stage frequency divider for stage `{stage_name}`: frequency_divider must be greater than zero, got {frequency_divider}"
    )]
    InvalidStageFrequencyDivider {
        stage_name: &'static str,
        frequency_divider: u64,
    },

    #[error("duplicate stage registration for stage `{stage_name}`")]
    DuplicateStageRegistration { stage_name: &'static str },

    #[error(
        "cannot resolve backend for stage `{stage_name}` with policy `{backend_policy}` in S14"
    )]
    BackendResolutionFailed {
        stage_name: &'static str,
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
