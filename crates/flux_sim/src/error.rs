use thiserror::Error;

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
}
