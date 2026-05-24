use std::time::Duration;

use crate::SimError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedTick {
    step: Duration,
    accumulator: Duration,
}

impl FixedTick {
    pub fn new(step: Duration) -> Result<Self, SimError> {
        if step.is_zero() {
            return Err(SimError::InvalidFixedTickStep);
        }
        Ok(Self {
            step,
            accumulator: Duration::ZERO,
        })
    }

    #[must_use]
    pub fn step(&self) -> Duration {
        self.step
    }

    #[must_use]
    pub fn pending_time(&self) -> Duration {
        self.accumulator
    }

    pub fn advance(&mut self, delta: Duration) -> Result<u64, SimError> {
        self.accumulator =
            self.accumulator
                .checked_add(delta)
                .ok_or(SimError::TickAccumulatorOverflow {
                    delta_nanos: delta.as_nanos(),
                })?;

        let mut ticks = 0u64;
        while self.accumulator >= self.step {
            self.accumulator -= self.step;
            ticks = ticks.checked_add(1).ok_or(SimError::TickCountOverflow)?;
        }

        Ok(ticks)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::FixedTick;

    #[test]
    fn rejects_zero_step() {
        assert!(FixedTick::new(Duration::ZERO).is_err());
    }

    #[test]
    fn deterministic_across_frame_partitioning() {
        let step = Duration::from_millis(100);
        let mut a = FixedTick::new(step).expect("step is valid");
        let mut b = FixedTick::new(step).expect("step is valid");

        let a_ticks = [
            a.advance(Duration::from_millis(30)).expect("advance"),
            a.advance(Duration::from_millis(30)).expect("advance"),
            a.advance(Duration::from_millis(40)).expect("advance"),
            a.advance(Duration::from_millis(200)).expect("advance"),
        ]
        .into_iter()
        .sum::<u64>();

        let b_ticks = [
            b.advance(Duration::from_millis(100)).expect("advance"),
            b.advance(Duration::from_millis(200)).expect("advance"),
        ]
        .into_iter()
        .sum::<u64>();

        assert_eq!(a_ticks, 3);
        assert_eq!(b_ticks, 3);
        assert_eq!(a.pending_time(), Duration::ZERO);
        assert_eq!(b.pending_time(), Duration::ZERO);
    }
}
