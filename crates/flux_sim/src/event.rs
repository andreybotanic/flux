use std::collections::{VecDeque, vec_deque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimEvent {
    WorldCreated { width: u32, height: u32, seed: u64 },
}

#[derive(Debug, Default, Clone)]
pub struct EventQueue {
    events: VecDeque<SimEvent>,
}

impl EventQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&mut self, event: SimEvent) {
        self.events.push_back(event);
    }

    #[must_use]
    pub fn dequeue(&mut self) -> Option<SimEvent> {
        self.events.pop_front()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn iter(&self) -> vec_deque::Iter<'_, SimEvent> {
        self.events.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{EventQueue, SimEvent};

    #[test]
    fn queue_is_fifo() {
        let mut queue = EventQueue::new();
        queue.enqueue(SimEvent::WorldCreated {
            width: 8,
            height: 8,
            seed: 1,
        });
        queue.enqueue(SimEvent::WorldCreated {
            width: 16,
            height: 16,
            seed: 2,
        });

        assert_eq!(
            queue.dequeue(),
            Some(SimEvent::WorldCreated {
                width: 8,
                height: 8,
                seed: 1
            })
        );
        assert_eq!(
            queue.dequeue(),
            Some(SimEvent::WorldCreated {
                width: 16,
                height: 16,
                seed: 2
            })
        );
        assert!(queue.is_empty());
    }
}
