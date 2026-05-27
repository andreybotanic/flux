use std::time::Duration;

use flux_world::{GridSize, WorldGrid};

use crate::{CommandQueue, EventQueue, FixedTick, SimCommand, SimError, SimEvent};

#[derive(Debug, Clone)]
pub struct SimRuntime {
    fixed_tick: FixedTick,
    initialized: bool,
    tick_counter: u64,
    world: Option<WorldGrid>,
    world_seed: Option<u64>,
    commands: CommandQueue,
    events: EventQueue,
}

impl SimRuntime {
    pub fn new(fixed_step: Duration) -> Result<Self, SimError> {
        Ok(Self {
            fixed_tick: FixedTick::new(fixed_step)?,
            initialized: false,
            tick_counter: 0,
            world: None,
            world_seed: None,
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
    pub fn world_mut(&mut self) -> Option<&mut WorldGrid> {
        self.world.as_mut()
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

    fn process_queued_commands(&mut self) -> Result<(), SimError> {
        while let Some(command) = self.commands.dequeue() {
            self.process_command(command)?;
        }
        Ok(())
    }

    /// Executes one simulation step for one fixed tick.
    ///
    /// S08 keeps this as a no-op simulation body, but the method is explicit
    /// so later stages can attach real simulation work to the fixed tick loop.
    pub fn step(&mut self) -> Result<(), SimError> {
        self.add_ticks(1)?;
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

    use flux_world::{GridSize, WorldGrid};

    use crate::{SimCommand, SimError, SimEvent, SimRuntime};

    fn runtime() -> SimRuntime {
        SimRuntime::new(Duration::from_millis(16)).expect("runtime should be created")
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
}
