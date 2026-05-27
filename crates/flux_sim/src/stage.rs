use std::fmt::{Display, Formatter};

use flux_world::{GasLayer, GasPrototypeId, GridSize, WorldGrid};

use crate::SimError;
use crate::gas_diffusion::GasSimulationStage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimulationBackendId {
    Cpu,
    Gpu,
}

impl SimulationBackendId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
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
    PreferGpu { cpu_fallback: bool },
}

impl BackendPolicy {
    #[must_use]
    pub const fn as_cli_value(self) -> &'static str {
        match self {
            Self::CpuOnly => "cpu_only",
            Self::PreferGpu { cpu_fallback: true } => "prefer_gpu",
            Self::PreferGpu {
                cpu_fallback: false,
            } => "prefer_gpu_strict",
        }
    }

    pub fn parse_cli_value(value: &str) -> Option<Self> {
        match value {
            "cpu_only" => Some(Self::CpuOnly),
            "prefer_gpu" => Some(Self::PreferGpu { cpu_fallback: true }),
            "prefer_gpu_strict" => Some(Self::PreferGpu {
                cpu_fallback: false,
            }),
            _ => None,
        }
    }

    #[must_use]
    pub const fn supported_cli_values() -> &'static [&'static str] {
        &["cpu_only", "prefer_gpu", "prefer_gpu_strict"]
    }
}

impl Display for BackendPolicy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_cli_value())
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn set_gas_prototypes(&self, gas_prototypes: Vec<GasPrototypeId>) {
        match self {
            Self::Gas(stage) => stage.set_gas_prototypes(gas_prototypes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BackendPolicy;

    #[test]
    fn backend_policy_cli_aliases_roundtrip() {
        assert_eq!(
            BackendPolicy::parse_cli_value("cpu_only"),
            Some(BackendPolicy::CpuOnly)
        );
        assert_eq!(
            BackendPolicy::parse_cli_value("prefer_gpu"),
            Some(BackendPolicy::PreferGpu { cpu_fallback: true })
        );
        assert_eq!(
            BackendPolicy::parse_cli_value("prefer_gpu_strict"),
            Some(BackendPolicy::PreferGpu {
                cpu_fallback: false
            })
        );
        assert_eq!(
            BackendPolicy::parse_cli_value("invalid"),
            None,
            "unknown aliases must be rejected"
        );
    }
}
