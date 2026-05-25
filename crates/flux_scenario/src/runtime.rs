use flux_sim::{SimCommand, SimRuntime};
use tracing::{error, info};

use crate::{
    AssertTickStep, CreateWorldStep, LogStep, ScenarioDefinition, ScenarioRunError, ScenarioStep,
    ScenarioStepRunner, WaitTicksStep,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioRunSummary {
    pub scenario_id: String,
    pub executed_steps: usize,
    pub final_tick: u64,
}

pub fn run_scenario(
    runtime: &mut SimRuntime,
    scenario: &ScenarioDefinition,
) -> Result<ScenarioRunSummary, ScenarioRunError> {
    let scenario_id = scenario.id.to_string();

    for (step_index, step) in scenario.steps.iter().enumerate() {
        step.run(runtime, &scenario_id, step_index)?;
        if matches!(
            step,
            ScenarioStep::CreateWorldStep(_) | ScenarioStep::WaitTicksStep(_)
        ) {
            initialize_runtime(runtime, &scenario_id, step_index)?;
        }
    }

    Ok(ScenarioRunSummary {
        scenario_id,
        executed_steps: scenario.steps.len(),
        final_tick: runtime.tick_counter(),
    })
}

fn initialize_runtime(
    runtime: &mut SimRuntime,
    scenario_id: &str,
    step_index: usize,
) -> Result<(), ScenarioRunError> {
    runtime
        .initialize()
        .map_err(|source| ScenarioRunError::SimCommandFailed {
            scenario_id: scenario_id.to_owned().into(),
            step_index,
            step_kind: "RuntimeInitialize".into(),
            source,
        })
}

impl ScenarioStep {
    pub fn run(
        &self,
        runtime: &mut SimRuntime,
        scenario_id: &str,
        step_index: usize,
    ) -> Result<(), ScenarioRunError> {
        ScenarioStepRunner::run(self, runtime, scenario_id, step_index)
    }
}

impl ScenarioStepRunner for LogStep {
    fn run(
        &self,
        runtime: &mut SimRuntime,
        scenario_id: &str,
        step_index: usize,
    ) -> Result<(), ScenarioRunError> {
        let _ = (runtime, step_index);
        let message = &self.0;
        info!("scenario_id={scenario_id} {message}");
        Ok(())
    }
}

impl ScenarioStepRunner for CreateWorldStep {
    fn run(
        &self,
        runtime: &mut SimRuntime,
        scenario_id: &str,
        step_index: usize,
    ) -> Result<(), ScenarioRunError> {
        runtime
            .enqueue_command(SimCommand::CreateWorld {
                width: self.width,
                height: self.height,
                seed: self.seed,
            })
            .map_err(|source| ScenarioRunError::SimCommandFailed {
                scenario_id: scenario_id.to_owned().into(),
                step_index,
                step_kind: "CreateWorld".into(),
                source,
            })?;
        Ok(())
    }
}

impl ScenarioStepRunner for WaitTicksStep {
    fn run(
        &self,
        runtime: &mut SimRuntime,
        scenario_id: &str,
        step_index: usize,
    ) -> Result<(), ScenarioRunError> {
        let ticks = self.0;
        runtime
            .enqueue_command(SimCommand::WaitTicks { ticks })
            .map_err(|source| ScenarioRunError::SimCommandFailed {
                scenario_id: scenario_id.to_owned().into(),
                step_index,
                step_kind: "WaitTicks".into(),
                source,
            })?;
        Ok(())
    }
}

impl ScenarioStepRunner for AssertTickStep {
    fn run(
        &self,
        runtime: &mut SimRuntime,
        scenario_id: &str,
        step_index: usize,
    ) -> Result<(), ScenarioRunError> {
        let expected = self.0;
        let actual = runtime.tick_counter();
        if actual == expected {
            info!(
                "scenario_id={scenario_id} assert_tick passed expected_tick={}",
                expected
            );
            return Ok(());
        }

        error!(
            "scenario_id={scenario_id} assert_tick failed expected_tick={} actual_tick={actual}",
            expected
        );
        Err(ScenarioRunError::AssertTickFailed {
            scenario_id: scenario_id.to_owned().into(),
            step_index,
            expected,
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use flux_core::PrototypeId;
    use flux_sim::SimRuntime;

    use crate::{
        AssertTickStep, CreateWorldStep, LogStep, ScenarioDefinition, ScenarioRunError,
        ScenarioStep, WaitTicksStep, run_scenario,
    };

    fn runtime() -> SimRuntime {
        SimRuntime::new(Duration::from_millis(16)).expect("runtime should be created")
    }

    #[test]
    fn run_scenario_executes_steps_and_asserts_tick() {
        let scenario = ScenarioDefinition {
            id: PrototypeId::parse("test_scenarios:scenario/bootstrap_smoke").expect("valid id"),
            steps: vec![
                ScenarioStep::LogStep(LogStep("scenario started".to_owned())),
                ScenarioStep::CreateWorldStep(CreateWorldStep {
                    width: 64,
                    height: 64,
                    seed: 0,
                }),
                ScenarioStep::WaitTicksStep(WaitTicksStep(5)),
                ScenarioStep::AssertTickStep(AssertTickStep(5)),
                ScenarioStep::LogStep(LogStep("scenario finished".to_owned())),
            ],
        };
        let mut runtime = runtime();

        let summary = run_scenario(&mut runtime, &scenario).expect("scenario should succeed");

        assert_eq!(
            summary.scenario_id,
            "test_scenarios:scenario/bootstrap_smoke"
        );
        assert_eq!(summary.executed_steps, 5);
        assert_eq!(summary.final_tick, 5);
    }

    #[test]
    fn assert_tick_mismatch_is_structured_error() {
        let scenario = ScenarioDefinition {
            id: PrototypeId::parse("test_scenarios:scenario/tick_fail").expect("valid id"),
            steps: vec![
                ScenarioStep::CreateWorldStep(CreateWorldStep {
                    width: 16,
                    height: 16,
                    seed: 1,
                }),
                ScenarioStep::WaitTicksStep(WaitTicksStep(2)),
                ScenarioStep::AssertTickStep(AssertTickStep(5)),
            ],
        };
        let mut runtime = runtime();

        let error = run_scenario(&mut runtime, &scenario).expect_err("assert tick should fail");
        assert_eq!(
            error,
            ScenarioRunError::AssertTickFailed {
                scenario_id: "test_scenarios:scenario/tick_fail".into(),
                step_index: 2,
                expected: 5,
                actual: 2
            }
        );
    }

    #[test]
    fn scenario_without_assert_runs_in_step_order() {
        let scenario = ScenarioDefinition {
            id: PrototypeId::parse("test_scenarios:scenario/no_assert").expect("valid id"),
            steps: vec![
                ScenarioStep::CreateWorldStep(CreateWorldStep {
                    width: 16,
                    height: 16,
                    seed: 1,
                }),
                ScenarioStep::WaitTicksStep(WaitTicksStep(3)),
                ScenarioStep::LogStep(LogStep("done".to_owned())),
            ],
        };
        let mut runtime = runtime();

        let summary = run_scenario(&mut runtime, &scenario).expect("scenario should succeed");

        assert_eq!(summary.executed_steps, 3);
        assert_eq!(summary.final_tick, 3);
    }

    #[test]
    fn commands_after_assert_are_executed_in_order() {
        let scenario = ScenarioDefinition {
            id: PrototypeId::parse("test_scenarios:scenario/command_after_assert")
                .expect("valid id"),
            steps: vec![
                ScenarioStep::CreateWorldStep(CreateWorldStep {
                    width: 16,
                    height: 16,
                    seed: 1,
                }),
                ScenarioStep::WaitTicksStep(WaitTicksStep(2)),
                ScenarioStep::AssertTickStep(AssertTickStep(2)),
                ScenarioStep::WaitTicksStep(WaitTicksStep(1)),
                ScenarioStep::AssertTickStep(AssertTickStep(3)),
            ],
        };
        let mut runtime = runtime();

        let summary = run_scenario(&mut runtime, &scenario).expect("scenario should succeed");

        assert_eq!(summary.executed_steps, 5);
        assert_eq!(summary.final_tick, 3);
    }

    #[test]
    fn multiple_wait_assert_blocks_preserve_command_order() {
        let scenario = ScenarioDefinition {
            id: PrototypeId::parse("test_scenarios:scenario/multi_phase_order").expect("valid id"),
            steps: vec![
                ScenarioStep::CreateWorldStep(CreateWorldStep {
                    width: 16,
                    height: 16,
                    seed: 1,
                }),
                ScenarioStep::WaitTicksStep(WaitTicksStep(1)),
                ScenarioStep::AssertTickStep(AssertTickStep(1)),
                ScenarioStep::WaitTicksStep(WaitTicksStep(4)),
                ScenarioStep::AssertTickStep(AssertTickStep(5)),
                ScenarioStep::WaitTicksStep(WaitTicksStep(2)),
                ScenarioStep::AssertTickStep(AssertTickStep(7)),
            ],
        };
        let mut runtime = runtime();

        let summary = run_scenario(&mut runtime, &scenario).expect("scenario should succeed");

        assert_eq!(summary.executed_steps, 7);
        assert_eq!(summary.final_tick, 7);
    }
}
