use enum_dispatch::enum_dispatch;
use flux_core::PrototypeId;
use flux_sim::SimRuntime;
use serde::Deserialize;

use crate::ScenarioRunError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioDefinition {
    pub id: PrototypeId,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[enum_dispatch(ScenarioStepRunner)]
pub enum ScenarioStep {
    #[serde(rename = "Log")]
    LogStep(LogStep),
    #[serde(rename = "CreateWorld")]
    CreateWorldStep(CreateWorldStep),
    #[serde(rename = "WaitTicks")]
    WaitTicksStep(WaitTicksStep),
    #[serde(rename = "AssertTick")]
    AssertTickStep(AssertTickStep),
}

#[enum_dispatch]
pub trait ScenarioStepRunner {
    fn run(
        &self,
        runtime: &mut SimRuntime,
        scenario_id: &str,
        step_index: usize,
    ) -> Result<(), ScenarioRunError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LogStep(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateWorldStep {
    pub width: u32,
    pub height: u32,
    pub seed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct WaitTicksStep(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct AssertTickStep(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioSource {
    pub mod_id: String,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedScenario {
    pub definition: ScenarioDefinition,
    pub source: ScenarioSource,
}
