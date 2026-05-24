use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimCommand {
    CreateWorld { width: u32, height: u32, seed: u64 },
    WaitTicks { ticks: u64 },
}

#[derive(Debug, Default, Clone)]
pub struct CommandQueue {
    pending: VecDeque<SimCommand>,
}

impl CommandQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&mut self, command: SimCommand) {
        self.pending.push_back(command);
    }

    #[must_use]
    pub fn dequeue(&mut self) -> Option<SimCommand> {
        self.pending.pop_front()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandQueue, SimCommand};

    #[test]
    fn queue_is_fifo() {
        let mut queue = CommandQueue::new();
        queue.enqueue(SimCommand::WaitTicks { ticks: 1 });
        queue.enqueue(SimCommand::WaitTicks { ticks: 2 });
        queue.enqueue(SimCommand::WaitTicks { ticks: 3 });

        assert_eq!(queue.dequeue(), Some(SimCommand::WaitTicks { ticks: 1 }));
        assert_eq!(queue.dequeue(), Some(SimCommand::WaitTicks { ticks: 2 }));
        assert_eq!(queue.dequeue(), Some(SimCommand::WaitTicks { ticks: 3 }));
        assert!(queue.is_empty());
    }
}
