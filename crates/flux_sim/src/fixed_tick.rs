use std::time::Duration;

use crate::SimError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedTick {
    step: Duration,
}

impl FixedTick {
    pub fn new(step: Duration) -> Result<Self, SimError> {
        if step.is_zero() {
            return Err(SimError::InvalidFixedTickStep);
        }
        Ok(Self { step })
    }

    #[must_use]
    pub fn step(&self) -> Duration {
        self.step
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
    fn stores_non_zero_step() {
        let step = Duration::from_millis(100);
        let tick = FixedTick::new(step).expect("step is valid");

        assert_eq!(tick.step(), step);
    }
}
