use std::fmt::{Display, Formatter};

use flux_world::WorldGrid;

use crate::SimError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimulationStageId {
    GasDiffusion,
}

impl SimulationStageId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GasDiffusion => "gas_diffusion",
        }
    }
}

impl Display for SimulationStageId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

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
    pub stage_id: SimulationStageId,
    pub frequency_divider: u64,
    pub backend_policy: BackendPolicy,
}

impl SimulationStageConfig {
    pub fn new(
        stage_id: SimulationStageId,
        frequency_divider: u64,
        backend_policy: BackendPolicy,
    ) -> Result<Self, SimError> {
        if frequency_divider == 0 {
            return Err(SimError::InvalidStageFrequencyDivider {
                stage_id,
                frequency_divider,
            });
        }
        Ok(Self {
            stage_id,
            frequency_divider,
            backend_policy,
        })
    }
}

pub struct StageExecutionContext<'a> {
    pub tick: u64,
    pub world: &'a mut WorldGrid,
    pub gas_permeability_mask: &'a [bool],
}

pub trait SimulationStageBackend: Send + Sync {
    fn backend_id(&self) -> SimulationBackendId;

    fn execute(&self, context: &mut StageExecutionContext<'_>) -> Result<(), SimError>;
}

pub trait SimulationStage: Send + Sync {
    fn config(&self) -> &SimulationStageConfig;

    fn resolve_backend(
        &self,
        policy: BackendPolicy,
    ) -> Result<&dyn SimulationStageBackend, SimError>;

    fn execute(
        &self,
        tick: u64,
        world: &mut WorldGrid,
        gas_permeability_mask: &[bool],
    ) -> Result<(), SimError> {
        let config = self.config();
        if !tick.is_multiple_of(config.frequency_divider) {
            return Ok(());
        }
        let backend = self.resolve_backend(config.backend_policy)?;
        let mut context = StageExecutionContext {
            tick,
            world,
            gas_permeability_mask,
        };
        backend.execute(&mut context)
    }
}
