use std::fmt::{Display, Formatter};

use flux_world::{GasLayer, GridSize, WorldGrid};

use crate::SimError;
use crate::gas_diffusion::GasSimulationStage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimulationBackendId {
    Cpu,
}

impl SimulationBackendId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
        }
    }
}

impl Display for SimulationBackendId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendPolicy {
    CpuOnly,
}

impl BackendPolicy {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CpuOnly => "cpu_only",
        }
    }
}

impl Display for BackendPolicy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationStageConfig {
    pub stage_name: &'static str,
    pub execution_order: u16,
    pub frequency_divider: u64,
    pub backend_policy: BackendPolicy,
}

impl SimulationStageConfig {
    pub fn new(
        stage_name: &'static str,
        execution_order: u16,
        frequency_divider: u64,
        backend_policy: BackendPolicy,
    ) -> Result<Self, SimError> {
        if frequency_divider == 0 {
            return Err(SimError::InvalidStageFrequencyDivider {
                stage_name,
                frequency_divider,
            });
        }
        Ok(Self {
            stage_name,
            execution_order,
            frequency_divider,
            backend_policy,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GasStageWorldView {
    pub size: GridSize,
}

pub trait GasSimulationBackend: Send + Sync {
    fn backend_id(&self) -> SimulationBackendId;

    fn execute(
        &self,
        tick: u64,
        gas_layer: &mut GasLayer,
        world: &GasStageWorldView,
    ) -> Result<(), SimError>;
}

pub enum SimulationStage {
    Gas(GasSimulationStage),
}

impl SimulationStage {
    #[must_use]
    pub fn stage_name(&self) -> &'static str {
        match self {
            Self::Gas(stage) => stage.config().stage_name,
        }
    }

    #[must_use]
    pub fn execution_order(&self) -> u16 {
        match self {
            Self::Gas(stage) => stage.config().execution_order,
        }
    }

    pub fn execute(&self, tick: u64, world: &mut WorldGrid) -> Result<(), SimError> {
        match self {
            Self::Gas(stage) => stage.execute(tick, world),
        }
    }
}
