use flux_world::WorldGrid;

use crate::gas_diffusion::GasSimulationStage;
use crate::{BackendPolicy, SimError, SimulationStage};

struct StageRegistration {
    name: &'static str,
    order: u16,
    stage: SimulationStage,
}

pub struct SimulationPipeline {
    stage_registry: Vec<StageRegistration>,
}

impl SimulationPipeline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            stage_registry: Vec::new(),
        }
    }

    pub fn new_default() -> Result<Self, SimError> {
        let mut pipeline = Self::new();
        pipeline.register_stage(SimulationStage::Gas(GasSimulationStage::new(
            1,
            BackendPolicy::CpuOnly,
        )?))?;
        Ok(pipeline)
    }

    pub fn register_stage(&mut self, stage: SimulationStage) -> Result<(), SimError> {
        let stage_name = stage.stage_name();
        if self
            .stage_registry
            .iter()
            .any(|entry| entry.name == stage_name)
        {
            return Err(SimError::DuplicateStageRegistration { stage_name });
        }
        self.stage_registry.push(StageRegistration {
            name: stage_name,
            order: stage.execution_order(),
            stage,
        });
        self.stage_registry
            .sort_by(|left, right| left.order.cmp(&right.order).then(left.name.cmp(right.name)));
        Ok(())
    }

    pub fn execute_tick(&self, tick: u64, world: &mut WorldGrid) -> Result<(), SimError> {
        for registration in &self.stage_registry {
            registration.stage.execute(tick, world)?;
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

    use crate::{BackendPolicy, SimRuntime, SimulationStage};

    use super::SimulationPipeline;

    #[test]
    fn registration_rejects_duplicate_stage_names() {
        let mut pipeline = SimulationPipeline::new();
        let stage_a = SimulationStage::Gas(
            crate::gas_diffusion::GasSimulationStage::new(1, BackendPolicy::CpuOnly)
                .expect("stage A"),
        );
        let stage_b = SimulationStage::Gas(
            crate::gas_diffusion::GasSimulationStage::new(2, BackendPolicy::CpuOnly)
                .expect("stage B"),
        );
        pipeline.register_stage(stage_a).expect("first stage");
        let error = pipeline
            .register_stage(stage_b)
            .expect_err("duplicate stage name must fail");
        assert_eq!(
            error,
            crate::SimError::DuplicateStageRegistration { stage_name: "gas" }
        );
    }

    #[test]
    fn stage_frequency_gates_execution() {
        let mut pipeline = SimulationPipeline::new();
        pipeline
            .register_stage(SimulationStage::Gas(
                crate::gas_diffusion::GasSimulationStage::new(2, BackendPolicy::CpuOnly)
                    .expect("stage"),
            ))
            .expect("stage should register");

        let mut world = WorldGrid::new(GridSize::new(3, 1)).expect("world");
        let gas = GasPrototypeId::parse("base:gas/oxygen").expect("id");
        world
            .set_gas_particles(TilePos::new(1, 0), gas.clone(), ParticleCount(120))
            .expect("seed");

        pipeline
            .execute_tick(1, &mut world)
            .expect("tick 1 should skip");
        assert_eq!(
            world
                .gas_at(TilePos::new(0, 0))
                .expect("cell")
                .particles_of(&gas),
            ParticleCount(0)
        );
        pipeline
            .execute_tick(2, &mut world)
            .expect("tick 2 should run");
        assert!(
            world
                .gas_at(TilePos::new(0, 0))
                .expect("cell")
                .particles_of(&gas)
                .0
                > 0
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
            .set_gas_particles_at(TilePos::new(1, 0), gas.clone(), ParticleCount(120))
            .expect("seed gas");
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
