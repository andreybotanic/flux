use std::time::Duration;

use flux_world::{
    GasPrototypeId, GridSize, ParticleCount, SolidCellPrototypeId, StructureInstanceId,
    StructurePrototypeId, TilePos, WorldGrid,
};

use crate::{
    BackendPolicy, CommandQueue, EventQueue, FixedTick, SimCommand, SimError, SimEvent,
    SimulationPipeline,
};

pub struct SimRuntime {
    fixed_tick: FixedTick,
    initialized: bool,
    tick_counter: u64,
    world: Option<WorldGrid>,
    world_seed: Option<u64>,
    pipeline: SimulationPipeline,
    commands: CommandQueue,
    events: EventQueue,
}

impl SimRuntime {
    pub fn new(fixed_step: Duration, backend_policy: BackendPolicy) -> Result<Self, SimError> {
        Ok(Self {
            fixed_tick: FixedTick::new(fixed_step)?,
            initialized: false,
            tick_counter: 0,
            world: None,
            world_seed: None,
            pipeline: SimulationPipeline::new_default(backend_policy)?,
            commands: CommandQueue::new(),
            events: EventQueue::new(),
        })
    }

    #[must_use]
    pub fn fixed_tick(&self) -> &FixedTick {
        &self.fixed_tick
    }

    #[must_use]
    pub fn tick_counter(&self) -> u64 {
        self.tick_counter
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    #[must_use]
    pub fn world(&self) -> Option<&WorldGrid> {
        self.world.as_ref()
    }

    #[must_use]
    pub fn world_seed(&self) -> Option<u64> {
        self.world_seed
    }

    #[must_use]
    pub fn commands(&self) -> &CommandQueue {
        &self.commands
    }

    #[must_use]
    pub fn events(&self) -> &EventQueue {
        &self.events
    }

    pub fn enqueue_command(&mut self, command: SimCommand) -> Result<(), SimError> {
        self.commands.enqueue(command);
        Ok(())
    }

    pub fn initialize(&mut self) -> Result<(), SimError> {
        self.process_queued_commands()?;
        self.initialized = true;
        Ok(())
    }

    pub fn load_world_state(&mut self, world: WorldGrid, seed: u64, tick: u64) {
        self.world = Some(world);
        self.world_seed = Some(seed);
        self.tick_counter = tick;
        self.initialized = true;
    }

    pub fn set_solid_cell_at(
        &mut self,
        pos: TilePos,
        solid: Option<SolidCellPrototypeId>,
    ) -> Result<(), SimError> {
        let world = self.world_for_mutation("set_solid_cell_at")?;
        world
            .set_solid_cell_at(pos, solid)
            .map_err(|source| SimError::WorldMutationFailed {
                operation: "set_solid_cell_at",
                source,
            })
    }

    pub fn set_gas_particles_at(
        &mut self,
        pos: TilePos,
        gas: GasPrototypeId,
        particles: ParticleCount,
    ) -> Result<(), SimError> {
        let world = self.world_for_mutation("set_gas_particles_at")?;
        world
            .set_gas_particles(pos, gas, particles)
            .map_err(|source| SimError::WorldMutationFailed {
                operation: "set_gas_particles_at",
                source,
            })
    }

    pub fn set_gas_prototypes(&mut self, gas_prototypes: Vec<GasPrototypeId>) {
        self.pipeline.set_gas_prototypes(gas_prototypes);
    }

    pub fn place_structure(
        &mut self,
        prototype: StructurePrototypeId,
        origin: TilePos,
    ) -> Result<StructureInstanceId, SimError> {
        let world = self.world_for_mutation("place_structure")?;
        world.place_structure(prototype, origin).map_err(|source| {
            SimError::StructureMutationFailed {
                operation: "place_structure",
                source,
            }
        })
    }

    pub fn remove_structure(&mut self, instance_id: StructureInstanceId) -> Result<(), SimError> {
        let world = self.world_for_mutation("remove_structure")?;
        world
            .remove_structure(instance_id)
            .map_err(|source| SimError::StructureMutationFailed {
                operation: "remove_structure",
                source,
            })
    }

    fn world_for_mutation(&mut self, operation: &'static str) -> Result<&mut WorldGrid, SimError> {
        self.world
            .as_mut()
            .ok_or(SimError::WorldNotLoadedForMutation { operation })
    }

    fn process_queued_commands(&mut self) -> Result<(), SimError> {
        while let Some(command) = self.commands.dequeue() {
            self.process_command(command)?;
        }
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), SimError> {
        self.add_ticks(1)?;
        if let Some(world) = self.world.as_mut() {
            self.pipeline.execute_tick(self.tick_counter, world)?;
        }
        Ok(())
    }

    fn process_command(&mut self, command: SimCommand) -> Result<(), SimError> {
        match command {
            SimCommand::CreateWorld {
                width,
                height,
                seed,
            } => self.create_world(width, height, seed),
            SimCommand::WaitTicks { ticks } => self.wait_ticks(ticks),
        }
    }

    fn create_world(&mut self, width: u32, height: u32, seed: u64) -> Result<(), SimError> {
        if width == 0 || height == 0 {
            return Err(SimError::InvalidWorldSize { width, height });
        }

        let size = GridSize::new(width, height);
        let world = WorldGrid::new(size).map_err(|source| SimError::WorldCreationFailed {
            width,
            height,
            source,
        })?;
        self.world = Some(world);
        self.world_seed = Some(seed);
        self.events.enqueue(SimEvent::WorldCreated {
            width,
            height,
            seed,
        });
        Ok(())
    }

    fn wait_ticks(&mut self, ticks: u64) -> Result<(), SimError> {
        for _ in 0..ticks {
            self.step()?;
        }
        Ok(())
    }

    fn add_ticks(&mut self, delta: u64) -> Result<(), SimError> {
        self.tick_counter =
            self.tick_counter
                .checked_add(delta)
                .ok_or(SimError::TickCounterOverflow {
                    current: self.tick_counter,
                    delta,
                })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use flux_world::{
        GasPrototypeId, GridSize, ParticleCount, SolidCellPrototypeId, TilePos, WorldGrid,
    };

    use crate::{BackendPolicy, SimCommand, SimError, SimEvent, SimRuntime};

    fn runtime() -> SimRuntime {
        SimRuntime::new(Duration::from_millis(16), BackendPolicy::CpuOnly)
            .expect("runtime should be created")
    }

    #[test]
    fn wait_ticks_advances_counter_deterministically() {
        let mut runtime = runtime();
        runtime
            .enqueue_command(SimCommand::CreateWorld {
                width: 64,
                height: 64,
                seed: 7,
            })
            .expect("enqueue should succeed");
        runtime
            .enqueue_command(SimCommand::WaitTicks { ticks: 5 })
            .expect("enqueue should succeed");

        runtime.initialize().expect("commands should be processed");

        assert_eq!(runtime.tick_counter(), 5);
    }

    #[test]
    fn create_world_emits_world_created_event_once() {
        let mut runtime = runtime();
        runtime
            .enqueue_command(SimCommand::CreateWorld {
                width: 64,
                height: 64,
                seed: 123,
            })
            .expect("enqueue should succeed");
        runtime.initialize().expect("command should succeed");

        let events = runtime.events().iter().cloned().collect::<Vec<_>>();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            SimEvent::WorldCreated {
                width: 64,
                height: 64,
                seed: 123
            }
        );
    }

    #[test]
    fn command_processing_is_fifo_for_mixed_commands() {
        let mut runtime = runtime();
        runtime
            .enqueue_command(SimCommand::WaitTicks { ticks: 2 })
            .expect("enqueue should succeed");
        runtime
            .enqueue_command(SimCommand::CreateWorld {
                width: 8,
                height: 8,
                seed: 1,
            })
            .expect("enqueue should succeed");
        runtime
            .enqueue_command(SimCommand::WaitTicks { ticks: 3 })
            .expect("enqueue should succeed");

        runtime.initialize().expect("commands should be processed");

        assert_eq!(runtime.tick_counter(), 5);
        assert!(runtime.world().is_some());
        assert_eq!(runtime.world_seed(), Some(1));
    }

    #[test]
    fn invalid_world_size_returns_structured_error_without_success_event() {
        let mut runtime = runtime();
        runtime
            .enqueue_command(SimCommand::CreateWorld {
                width: 0,
                height: 64,
                seed: 1,
            })
            .expect("enqueue should succeed");

        let error = runtime
            .initialize()
            .expect_err("world creation with zero width should fail");
        assert_eq!(
            error,
            SimError::InvalidWorldSize {
                width: 0,
                height: 64
            }
        );
        assert!(runtime.events().is_empty());
        assert!(runtime.world().is_none());
    }

    #[test]
    fn initialize_can_be_called_multiple_times() {
        let mut runtime = runtime();
        runtime
            .enqueue_command(SimCommand::WaitTicks { ticks: 1 })
            .expect("enqueue should succeed");
        runtime
            .initialize()
            .expect("first initialize should succeed");
        runtime
            .enqueue_command(SimCommand::WaitTicks { ticks: 2 })
            .expect("enqueue should succeed");
        runtime
            .initialize()
            .expect("second initialize should also succeed");

        assert_eq!(runtime.tick_counter(), 3);
    }

    #[test]
    fn enqueue_after_initialize_is_allowed() {
        let mut runtime = runtime();
        runtime.initialize().expect("initialize should succeed");

        runtime
            .enqueue_command(SimCommand::WaitTicks { ticks: 1 })
            .expect("enqueue after initialize should succeed");
        assert_eq!(runtime.tick_counter(), 0);

        runtime
            .initialize()
            .expect("newly queued command should be processed");
        assert_eq!(runtime.tick_counter(), 1);
    }

    #[test]
    fn load_world_state_replaces_world_seed_and_tick() {
        let mut runtime = runtime();
        let world = WorldGrid::new(GridSize::new(4, 5)).expect("world");

        runtime.load_world_state(world, 999, 321);

        assert_eq!(runtime.world_seed(), Some(999));
        assert_eq!(runtime.tick_counter(), 321);
        assert_eq!(runtime.world().expect("world").size(), GridSize::new(4, 5));
        assert!(runtime.is_initialized());
    }

    #[test]
    fn gas_diffusion_runs_inside_step() {
        let mut runtime = runtime();
        runtime
            .enqueue_command(SimCommand::CreateWorld {
                width: 3,
                height: 1,
                seed: 1,
            })
            .expect("enqueue");
        runtime.initialize().expect("init");
        let oxygen = GasPrototypeId::parse("base:gas/oxygen").expect("id");
        runtime
            .set_gas_particles_at(TilePos::new(1, 0), oxygen.clone(), ParticleCount(120))
            .expect("seed gas");

        runtime.step().expect("step");

        let left = runtime
            .world()
            .expect("world")
            .gas_at(TilePos::new(0, 0))
            .expect("cell")
            .particles_of(&oxygen)
            .0;
        assert!(left > 0, "expected diffusion to move gas to left neighbor");
    }

    #[test]
    fn set_solid_cell_rebuilds_gas_permeability_mask_immediately() {
        let mut runtime = runtime();
        runtime
            .enqueue_command(SimCommand::CreateWorld {
                width: 2,
                height: 1,
                seed: 1,
            })
            .expect("enqueue");
        runtime.initialize().expect("init");
        let oxygen = GasPrototypeId::parse("base:gas/oxygen").expect("id");
        let solid = SolidCellPrototypeId::parse("base:solid_cell/floor_cell").expect("id");
        runtime
            .set_gas_particles_at(TilePos::new(0, 0), oxygen.clone(), ParticleCount(120))
            .expect("seed gas");
        runtime
            .set_solid_cell_at(TilePos::new(1, 0), Some(solid))
            .expect("set solid");

        runtime.step().expect("step");

        let right = runtime
            .world()
            .expect("world")
            .gas_at(TilePos::new(1, 0))
            .expect("cell")
            .particles_of(&oxygen)
            .0;
        assert_eq!(right, 0);
    }

    #[test]
    fn set_solid_cell_without_world_returns_structured_error() {
        let mut runtime = runtime();
        let solid = SolidCellPrototypeId::parse("base:solid_cell/floor_cell").expect("id");
        let error = runtime
            .set_solid_cell_at(TilePos::new(0, 0), Some(solid))
            .expect_err("must fail");
        assert_eq!(
            error,
            SimError::WorldNotLoadedForMutation {
                operation: "set_solid_cell_at",
            }
        );
    }

    #[test]
    fn set_gas_prototypes_is_allowed_without_world() {
        let mut runtime = runtime();
        runtime.set_gas_prototypes(vec![GasPrototypeId::parse("base:gas/oxygen").expect("id")]);
    }
}
