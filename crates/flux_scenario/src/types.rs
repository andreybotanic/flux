use flux_core::PrototypeId;
use flux_ui::{UiMenuId, UiWidgetId};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioDefinition {
    pub id: PrototypeId,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum ScenarioStep {
    #[serde(rename = "Log")]
    LogStep(LogStep),
    #[serde(rename = "CreateWorld")]
    CreateWorldStep(CreateWorldStep),
    #[serde(rename = "WaitTicks")]
    WaitTicksStep(WaitTicksStep),
    #[serde(rename = "AssertTick")]
    AssertTickStep(AssertTickStep),
    #[serde(rename = "OpenMenu", alias = "OpenUi")]
    OpenMenuStep(OpenMenuStep),
    #[serde(rename = "Click")]
    ClickStep(ClickStep),
    #[serde(rename = "WaitSimulationTime")]
    WaitSimulationTimeStep(WaitSimulationTimeStep),
    #[serde(rename = "PauseSimulation")]
    PauseSimulationStep(PauseSimulationStep),
    #[serde(rename = "WaitRealtime")]
    WaitRealtimeStep(WaitRealtimeStep),
    #[serde(rename = "ResumeSimulation")]
    ResumeSimulationStep(ResumeSimulationStep),
    #[serde(rename = "TakeScreenshot")]
    TakeScreenshotStep(TakeScreenshotStep),
    #[serde(rename = "AssertUiExists")]
    AssertUiExistsStep(AssertUiExistsStep),
    #[serde(rename = "SetCameraPivot")]
    SetCameraPivotStep(SetCameraPivotStep),
    #[serde(rename = "SetCameraZoom")]
    SetCameraZoomStep(SetCameraZoomStep),
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct OpenMenuStep(pub UiMenuId);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ClickStep(pub UiWidgetId);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitSimulationTimeStep {
    pub delay_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PauseSimulationStep {
    #[serde(default)]
    pub delay_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitRealtimeStep {
    pub delay_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResumeSimulationStep {}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct TakeScreenshotStep(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct AssertUiExistsStep(pub UiWidgetId);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetCameraPivotStep {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "SetCameraZoomStepRepr")]
pub struct SetCameraZoomStep {
    pub zoom: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum SetCameraZoomStepRepr {
    Scalar(f32),
    Named {
        #[serde(alias = "z")]
        zoom: f32,
    },
}

impl From<SetCameraZoomStepRepr> for SetCameraZoomStep {
    fn from(value: SetCameraZoomStepRepr) -> Self {
        match value {
            SetCameraZoomStepRepr::Scalar(zoom) => Self { zoom },
            SetCameraZoomStepRepr::Named { zoom } => Self { zoom },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioSource {
    pub mod_id: String,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedScenario {
    pub definition: ScenarioDefinition,
    pub source: ScenarioSource,
}

impl ScenarioStep {
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            ScenarioStep::LogStep(_) => "Log",
            ScenarioStep::CreateWorldStep(_) => "CreateWorld",
            ScenarioStep::WaitTicksStep(_) => "WaitTicks",
            ScenarioStep::AssertTickStep(_) => "AssertTick",
            ScenarioStep::OpenMenuStep(_) => "OpenMenu",
            ScenarioStep::ClickStep(_) => "Click",
            ScenarioStep::WaitSimulationTimeStep(_) => "WaitSimulationTime",
            ScenarioStep::PauseSimulationStep(_) => "PauseSimulation",
            ScenarioStep::WaitRealtimeStep(_) => "WaitRealtime",
            ScenarioStep::ResumeSimulationStep(_) => "ResumeSimulation",
            ScenarioStep::TakeScreenshotStep(_) => "TakeScreenshot",
            ScenarioStep::AssertUiExistsStep(_) => "AssertUiExists",
            ScenarioStep::SetCameraPivotStep(_) => "SetCameraPivot",
            ScenarioStep::SetCameraZoomStep(_) => "SetCameraZoom",
        }
    }
}
