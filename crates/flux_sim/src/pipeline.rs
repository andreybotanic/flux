use std::collections::BTreeSet;

use flux_world::WorldGrid;

use crate::gas_diffusion::GasDiffusionStage;
use crate::{BackendPolicy, SimError, SimulationStage, SimulationStageId};

pub struct SimulationPipeline {
    stages: Vec<Box<dyn SimulationStage>>,
    stage_ids: BTreeSet<SimulationStageId>,
}

impl SimulationPipeline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            stage_ids: BTreeSet::new(),
        }
    }

    pub fn new_default() -> Result<Self, SimError> {
        let mut pipeline = Self::new();
        pipeline.register_stage(Box::new(GasDiffusionStage::new(1, BackendPolicy::CpuOnly)?))?;
        Ok(pipeline)
    }

    pub fn register_stage(&mut self, stage: Box<dyn SimulationStage>) -> Result<(), SimError> {
        let stage_id = stage.config().stage_id;
        if self.stage_ids.contains(&stage_id) {
            return Err(SimError::DuplicateStageRegistration { stage_id });
        }
        self.stage_ids.insert(stage_id);
        self.stages.push(stage);
        Ok(())
    }

    pub fn execute_tick(
        &self,
        tick: u64,
        world: &mut WorldGrid,
        gas_permeability_mask: &[bool],
    ) -> Result<(), SimError> {
        for stage in &self.stages {
            stage.execute(tick, world, gas_permeability_mask)?;
        }
        Ok(())
    }
}

impl Default for SimulationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use flux_world::{GasPrototypeId, GridSize, ParticleCount, TilePos, WorldGrid};

    use crate::{
        BackendPolicy, SimRuntime, SimulationBackendId, SimulationStage, SimulationStageBackend,
        SimulationStageConfig, SimulationStageId, StageExecutionContext,
    };

    use super::SimulationPipeline;

    #[derive(Default)]
    struct SetCellBackend {
        tick_offset: u64,
    }

    impl SimulationStageBackend for SetCellBackend {
        fn backend_id(&self) -> SimulationBackendId {
            SimulationBackendId::Cpu
        }

        fn execute(&self, context: &mut StageExecutionContext<'_>) -> Result<(), crate::SimError> {
            let gas = GasPrototypeId::parse("base:gas/oxygen").expect("id");
            let value = context.tick.saturating_add(self.tick_offset);
            context
                .world
                .set_gas_particles(TilePos::new(0, 0), gas, ParticleCount(value))
                .map_err(|source| crate::SimError::GasSnapshotApplyFailed { source })?;
            Ok(())
        }
    }

    struct TestStage {
        config: SimulationStageConfig,
        backend: SetCellBackend,
    }

    impl SimulationStage for TestStage {
        fn config(&self) -> &SimulationStageConfig {
            &self.config
        }

        fn resolve_backend(
            &self,
            policy: BackendPolicy,
        ) -> Result<&dyn SimulationStageBackend, crate::SimError> {
            if policy != BackendPolicy::CpuOnly {
                return Err(crate::SimError::BackendResolutionFailed {
                    stage_id: self.config.stage_id,
                    backend_policy: policy,
                });
            }
            Ok(&self.backend)
        }
    }

    #[test]
    fn registration_rejects_duplicate_stage_ids() {
        let mut pipeline = SimulationPipeline::new();
        let stage_a = TestStage {
            config: SimulationStageConfig::new(
                SimulationStageId::GasDiffusion,
                1,
                BackendPolicy::CpuOnly,
            )
            .expect("config"),
            backend: SetCellBackend::default(),
        };
        let stage_b = TestStage {
            config: SimulationStageConfig::new(
                SimulationStageId::GasDiffusion,
                2,
                BackendPolicy::CpuOnly,
            )
            .expect("config"),
            backend: SetCellBackend::default(),
        };
        pipeline
            .register_stage(Box::new(stage_a))
            .expect("first stage");
        let error = pipeline
            .register_stage(Box::new(stage_b))
            .expect_err("duplicate stage id must fail");
        assert!(matches!(
            error,
            crate::SimError::DuplicateStageRegistration {
                stage_id: SimulationStageId::GasDiffusion
            }
        ));
    }

    #[test]
    fn stage_frequency_gates_execution() {
        let mut pipeline = SimulationPipeline::new();
        let stage = TestStage {
            config: SimulationStageConfig::new(
                SimulationStageId::GasDiffusion,
                2,
                BackendPolicy::CpuOnly,
            )
            .expect("config"),
            backend: SetCellBackend { tick_offset: 100 },
        };
        pipeline
            .register_stage(Box::new(stage))
            .expect("stage should register");

        let mut world = WorldGrid::new(GridSize::new(2, 2)).expect("world");
        let mask = world.build_gas_permeability_mask();
        pipeline
            .execute_tick(1, &mut world, &mask)
            .expect("tick 1 should skip");
        assert_eq!(
            world
                .gas_at(TilePos::new(0, 0))
                .expect("cell")
                .total_particles(),
            ParticleCount(0)
        );
        pipeline
            .execute_tick(2, &mut world, &mask)
            .expect("tick 2 should run");
        assert_eq!(
            world
                .gas_at(TilePos::new(0, 0))
                .expect("cell")
                .total_particles(),
            ParticleCount(102)
        );
    }

    #[test]
    fn default_runtime_pipeline_registers_and_runs_gas_stage() {
        let mut runtime = SimRuntime::new(Duration::from_millis(16)).expect("runtime");
        runtime
            .enqueue_command(crate::SimCommand::CreateWorld {
                width: 3,
                height: 1,
                seed: 1,
            })
            .expect("enqueue");
        runtime
            .initialize()
            .expect("runtime should process commands");
        let gas = GasPrototypeId::parse("base:gas/oxygen").expect("id");
        runtime
            .world_mut()
            .expect("world")
            .set_gas_particles(TilePos::new(1, 0), gas.clone(), ParticleCount(120))
            .expect("seed gas");
        runtime
            .initialize()
            .expect("no-op initialize after world mutation");
        runtime.step().expect("step should run");
        let left = runtime
            .world()
            .expect("world")
            .gas_at(TilePos::new(0, 0))
            .expect("cell")
            .particles_of(&gas);
        assert!(left.0 > 0, "gas did not diffuse to neighbor");
    }
}
