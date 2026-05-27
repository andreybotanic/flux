use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::OnceLock;

use flux_world::{GasLayer, GasMixture, GasPrototypeId, WorldGrid};
use log::warn;

use crate::{
    BackendPolicy, GasSimulationBackend, GasStageWorldView, SimError, SimulationStageConfig,
};

mod cpu_backend;
mod gpu_backend;

use cpu_backend::GasDiffusionCpuBackend;
use gpu_backend::{GasDiffusionGpuBackend, GpuBackendInitError};

const GAS_SIMULATION_STAGE_NAME: &str = "gas";
const GAS_SIMULATION_STAGE_ORDER: u16 = 100;

pub struct GasSimulationStage {
    config: SimulationStageConfig,
    cpu_backend: GasDiffusionCpuBackend,
    gas_prototypes: Mutex<Vec<GasPrototypeId>>,
    gpu_backend: OnceLock<Result<GasDiffusionGpuBackend, GpuBackendInitError>>,
    gpu_runtime_failure_reason: Mutex<Option<String>>,
}

impl GasSimulationStage {
    pub fn new(frequency_divider: u64, backend_policy: BackendPolicy) -> Result<Self, SimError> {
        Ok(Self {
            config: SimulationStageConfig::new(
                GAS_SIMULATION_STAGE_NAME,
                GAS_SIMULATION_STAGE_ORDER,
                frequency_divider,
                backend_policy,
            )?,
            cpu_backend: GasDiffusionCpuBackend,
            gas_prototypes: Mutex::new(Vec::new()),
            gpu_backend: OnceLock::new(),
            gpu_runtime_failure_reason: Mutex::new(None),
        })
    }

    #[must_use]
    pub fn config(&self) -> &SimulationStageConfig {
        &self.config
    }

    pub fn set_gas_prototypes(&self, mut gas_prototypes: Vec<GasPrototypeId>) {
        gas_prototypes.sort();
        gas_prototypes.dedup();
        let mut current = self
            .gas_prototypes
            .lock()
            .expect("gas prototype list lock must not be poisoned");
        *current = gas_prototypes;
    }

    fn resolve_gpu_backend(&self) -> Result<&GasDiffusionGpuBackend, SimError> {
        let backend = self.gpu_backend.get_or_init(|| {
            let gas_prototypes = self
                .gas_prototypes
                .lock()
                .expect("gas prototype list lock must not be poisoned")
                .clone();
            GasDiffusionGpuBackend::new(self.config.stage_name, gas_prototypes)
        });
        match backend {
            Ok(backend) => Ok(backend),
            Err(GpuBackendInitError::AdapterUnavailable) => Err(SimError::GpuAdapterUnavailable {
                stage_name: self.config.stage_name,
            }),
            Err(GpuBackendInitError::DeviceRequestFailed(reason)) => {
                Err(SimError::GpuDeviceRequestFailed {
                    stage_name: self.config.stage_name,
                    reason: reason.clone(),
                })
            }
        }
    }

    fn execute_cpu(
        &self,
        tick: u64,
        gas_layer: &mut GasLayer,
        world_view: &GasStageWorldView,
        fallback_reason: Option<&SimError>,
    ) -> Result<(), SimError> {
        if let Some(reason) = fallback_reason {
            warn!(
                target: "flux_sim::gas_diffusion",
                "stage={} tick={} policy={} fallback_backend=cpu reason={}",
                self.config.stage_name,
                tick,
                self.config.backend_policy,
                reason
            );
        }
        self.cpu_backend.execute(tick, gas_layer, world_view)?;
        Ok(())
    }

    fn execute_gpu(
        &self,
        tick: u64,
        gas_layer: &mut GasLayer,
        world_view: &GasStageWorldView,
    ) -> Result<(), SimError> {
        if let Some(reason) = self
            .gpu_runtime_failure_reason
            .lock()
            .expect("gpu runtime failure lock must not be poisoned")
            .clone()
        {
            return Err(SimError::GpuExecutionFailed {
                stage_name: self.config.stage_name,
                reason: format!("gpu backend disabled after previous failure: {reason}"),
            });
        }
        let backend = self.resolve_gpu_backend()?;
        backend.execute(tick, gas_layer, world_view)?;
        Ok(())
    }

    fn mark_gpu_runtime_failure(&self, error: &SimError) {
        let mut stored_reason = self
            .gpu_runtime_failure_reason
            .lock()
            .expect("gpu runtime failure lock must not be poisoned");
        if stored_reason.is_none() {
            *stored_reason = Some(error.to_string());
        }
    }

    pub fn execute(&self, tick: u64, world: &mut WorldGrid) -> Result<(), SimError> {
        if !tick.is_multiple_of(self.config.frequency_divider) {
            return Ok(());
        }

        let world_view = GasStageWorldView { size: world.size() };
        let gas_layer = world.gas_layer_mut();
        match self.config.backend_policy {
            BackendPolicy::CpuOnly => self.execute_cpu(tick, gas_layer, &world_view, None),
            BackendPolicy::PreferGpu { cpu_fallback } => {
                let gpu_result = self.execute_gpu(tick, gas_layer, &world_view);
                match gpu_result {
                    Ok(()) => Ok(()),
                    Err(error) if cpu_fallback => {
                        self.mark_gpu_runtime_failure(&error);
                        self.execute_cpu(tick, gas_layer, &world_view, Some(&error))
                    }
                    Err(error) => {
                        self.mark_gpu_runtime_failure(&error);
                        Err(SimError::BackendFallbackDisabled {
                            stage_name: self.config.stage_name,
                            backend_policy: self.config.backend_policy,
                            reason: error.to_string(),
                        })
                    }
                }
            }
        }
    }
}

pub(super) fn total_particles_by_gas(
    cells: &[GasMixture],
    permeability_mask: &[bool],
) -> BTreeMap<String, u64> {
    let mut totals = BTreeMap::new();
    for (cell_index, mixture) in cells.iter().enumerate() {
        if !permeability_mask[cell_index] {
            continue;
        }
        for component in mixture.components() {
            *totals.entry(component.gas.to_string()).or_insert(0) += component.particles.0;
        }
    }
    totals
}

pub(super) fn ensure_conservation(
    before: &BTreeMap<String, u64>,
    after: &BTreeMap<String, u64>,
) -> Result<(), SimError> {
    let mut keys = before.keys().cloned().collect::<Vec<_>>();
    for key in after.keys() {
        if !before.contains_key(key) {
            keys.push(key.clone());
        }
    }
    keys.sort();
    keys.dedup();
    for key in keys {
        let before_value = before.get(&key).copied().unwrap_or(0);
        let after_value = after.get(&key).copied().unwrap_or(0);
        if before_value != after_value {
            return Err(SimError::GasConservationViolated {
                gas: key,
                before: before_value,
                after: after_value,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use flux_world::{GasPrototypeId, GridSize, ParticleCount, TilePos, WorldGrid};

    use crate::{BackendPolicy, SimError};

    use super::GasSimulationStage;

    fn gas_id(value: &str) -> GasPrototypeId {
        GasPrototypeId::parse(value).expect("valid gas id")
    }

    fn set_stage_gas_channels(stage: &GasSimulationStage, gases: &[GasPrototypeId]) {
        stage.set_gas_prototypes(gases.to_vec());
    }

    #[test]
    fn diffusion_spreads_to_von_neumann_neighbors_and_keeps_total() {
        let mut world = WorldGrid::new(GridSize::new(3, 3)).expect("world");
        let gas = gas_id("base:gas/oxygen");
        world
            .set_gas_particles(TilePos::new(1, 1), gas.clone(), ParticleCount(400))
            .expect("set center gas");
        let stage = GasSimulationStage::new(1, BackendPolicy::CpuOnly).expect("stage");
        stage.execute(1, &mut world).expect("diffusion should run");

        let center = world
            .gas_at(TilePos::new(1, 1))
            .expect("cell")
            .particles_of(&gas)
            .0;
        let left = world
            .gas_at(TilePos::new(0, 1))
            .expect("cell")
            .particles_of(&gas)
            .0;
        let right = world
            .gas_at(TilePos::new(2, 1))
            .expect("cell")
            .particles_of(&gas)
            .0;
        let up = world
            .gas_at(TilePos::new(1, 0))
            .expect("cell")
            .particles_of(&gas)
            .0;
        let down = world
            .gas_at(TilePos::new(1, 2))
            .expect("cell")
            .particles_of(&gas)
            .0;
        let diagonal = world
            .gas_at(TilePos::new(0, 0))
            .expect("cell")
            .particles_of(&gas)
            .0;

        assert!(center < 400, "center should lose particles");
        assert!(left > 0 && right > 0 && up > 0 && down > 0);
        assert_eq!(diagonal, 0, "diagonal cell must stay unchanged");

        let total = [center, left, right, up, down, diagonal]
            .into_iter()
            .sum::<u64>()
            + world
                .gas_at(TilePos::new(2, 0))
                .expect("cell")
                .particles_of(&gas)
                .0
            + world
                .gas_at(TilePos::new(0, 2))
                .expect("cell")
                .particles_of(&gas)
                .0
            + world
                .gas_at(TilePos::new(2, 2))
                .expect("cell")
                .particles_of(&gas)
                .0;
        assert_eq!(total, 400);
    }

    #[test]
    fn solid_cells_are_blocked_and_cleaned_deterministically() {
        let mut world = WorldGrid::new(GridSize::new(2, 1)).expect("world");
        let gas = gas_id("base:gas/oxygen");
        let solid = gas_id("base:solid_cell/floor_cell");
        world
            .set_solid_cell_at(TilePos::new(0, 0), Some(solid))
            .expect("set solid");
        world
            .set_gas_particles(TilePos::new(0, 0), gas.clone(), ParticleCount(80))
            .expect("inject invalid gas");
        world
            .set_gas_particles(TilePos::new(1, 0), gas.clone(), ParticleCount(120))
            .expect("set valid gas");
        let stage = GasSimulationStage::new(1, BackendPolicy::CpuOnly).expect("stage");
        stage.execute(1, &mut world).expect("diffusion should run");

        assert_eq!(
            world
                .gas_at(TilePos::new(0, 0))
                .expect("cell")
                .particles_of(&gas),
            ParticleCount(0)
        );
        assert_eq!(
            world
                .gas_at(TilePos::new(1, 0))
                .expect("cell")
                .particles_of(&gas),
            ParticleCount(120)
        );
    }

    #[test]
    fn same_input_produces_same_output() {
        let gas = gas_id("base:gas/oxygen");
        let stage = GasSimulationStage::new(1, BackendPolicy::CpuOnly).expect("stage");

        let mut world_a = WorldGrid::new(GridSize::new(4, 1)).expect("world");
        let mut world_b = WorldGrid::new(GridSize::new(4, 1)).expect("world");
        world_a
            .set_gas_particles(TilePos::new(1, 0), gas.clone(), ParticleCount(200))
            .expect("seed");
        world_b
            .set_gas_particles(TilePos::new(1, 0), gas.clone(), ParticleCount(200))
            .expect("seed");
        stage.execute(1, &mut world_a).expect("run A");
        stage.execute(1, &mut world_b).expect("run B");

        for x in 0..4 {
            assert_eq!(
                world_a.gas_at(TilePos::new(x, 0)),
                world_b.gas_at(TilePos::new(x, 0))
            );
        }
    }

    #[test]
    fn prefer_gpu_strict_returns_structured_error_without_fallback_if_gpu_unavailable() {
        let mut world = WorldGrid::new(GridSize::new(3, 1)).expect("world");
        let gas = gas_id("base:gas/oxygen");
        world
            .set_gas_particles(TilePos::new(1, 0), gas, ParticleCount(120))
            .expect("seed");
        let stage = GasSimulationStage::new(
            1,
            BackendPolicy::PreferGpu {
                cpu_fallback: false,
            },
        )
        .expect("stage");
        set_stage_gas_channels(&stage, &[gas_id("base:gas/oxygen")]);

        let result = stage.execute(1, &mut world);
        if let Err(SimError::BackendFallbackDisabled { backend_policy, .. }) = &result {
            assert_eq!(
                *backend_policy,
                BackendPolicy::PreferGpu {
                    cpu_fallback: false
                }
            );
            return;
        }
        result.expect("strict policy should pass only when gpu backend is available");
    }

    #[test]
    fn gpu_diffusion_matches_cpu_for_multi_gas_smoke_when_gpu_is_available() {
        let oxygen = gas_id("base:gas/oxygen");
        let hydrogen = gas_id("base:gas/hydrogen");
        let solid = gas_id("base:solid_cell/floor_cell");

        let mut world_cpu = WorldGrid::new(GridSize::new(4, 2)).expect("cpu world");
        world_cpu
            .set_gas_particles(TilePos::new(1, 0), oxygen.clone(), ParticleCount(200))
            .expect("seed oxygen");
        world_cpu
            .set_gas_particles(TilePos::new(2, 0), hydrogen.clone(), ParticleCount(80))
            .expect("seed hydrogen");
        world_cpu
            .set_solid_cell_at(TilePos::new(3, 1), Some(solid))
            .expect("set solid");

        let mut world_gpu = world_cpu.clone();
        let cpu_stage = GasSimulationStage::new(1, BackendPolicy::CpuOnly).expect("cpu stage");
        let gpu_stage = GasSimulationStage::new(
            1,
            BackendPolicy::PreferGpu {
                cpu_fallback: false,
            },
        )
        .expect("gpu stage");
        set_stage_gas_channels(&gpu_stage, &[oxygen.clone(), hydrogen.clone()]);

        cpu_stage.execute(1, &mut world_cpu).expect("cpu run");
        let gpu_result = gpu_stage.execute(1, &mut world_gpu);
        if matches!(gpu_result, Err(SimError::BackendFallbackDisabled { .. })) {
            return;
        }
        gpu_result.expect("gpu run must succeed when backend is available");
        assert_eq!(world_cpu.gas_snapshot(), world_gpu.gas_snapshot());
    }

    #[test]
    fn prefer_gpu_fallback_true_uses_cpu_when_gpu_path_cannot_represent_particles() {
        let oxygen = gas_id("base:gas/oxygen");
        let large_count = u64::from(u32::MAX) + 512;
        let mut world = WorldGrid::new(GridSize::new(3, 1)).expect("world");
        world
            .set_gas_particles(
                TilePos::new(1, 0),
                oxygen.clone(),
                ParticleCount(large_count),
            )
            .expect("seed");
        let stage = GasSimulationStage::new(1, BackendPolicy::PreferGpu { cpu_fallback: true })
            .expect("stage");
        set_stage_gas_channels(&stage, std::slice::from_ref(&oxygen));

        stage
            .execute(1, &mut world)
            .expect("fallback=true must use cpu backend");
        let total = world
            .gas_at(TilePos::new(0, 0))
            .expect("cell")
            .particles_of(&oxygen)
            .0
            + world
                .gas_at(TilePos::new(1, 0))
                .expect("cell")
                .particles_of(&oxygen)
                .0
            + world
                .gas_at(TilePos::new(2, 0))
                .expect("cell")
                .particles_of(&oxygen)
                .0;
        assert_eq!(total, large_count);
    }

    #[test]
    fn prefer_gpu_strict_returns_error_when_gpu_path_cannot_represent_particles() {
        let oxygen = gas_id("base:gas/oxygen");
        let large_count = u64::from(u32::MAX) + 512;
        let mut world = WorldGrid::new(GridSize::new(3, 1)).expect("world");
        world
            .set_gas_particles(TilePos::new(1, 0), oxygen, ParticleCount(large_count))
            .expect("seed");
        let stage = GasSimulationStage::new(
            1,
            BackendPolicy::PreferGpu {
                cpu_fallback: false,
            },
        )
        .expect("stage");
        set_stage_gas_channels(&stage, &[gas_id("base:gas/oxygen")]);

        let error = stage
            .execute(1, &mut world)
            .expect_err("strict mode must fail without fallback");
        if let SimError::BackendFallbackDisabled { reason, .. } = error {
            assert!(
                reason.contains("exceed u32"),
                "expected explicit reason about u32 limit, got: {reason}"
            );
        } else {
            panic!("unexpected error variant");
        }
    }
}
