#![forbid(unsafe_code)]

mod error;
mod loader;
mod types;

pub use error::ScenarioLoadError;
pub use loader::{ScenarioLoadReport, load_scenarios};
pub use types::{
    AssertTickStep, AssertUiExistsStep, ClickStep, CreateWorldStep, LoadGameStep, LoadedScenario,
    LogStep, OpenMenuStep, PauseSimulationStep, ResumeSimulationStep, SaveGameStep,
    ScenarioDefinition, ScenarioSource, ScenarioStep, SetCameraPivotStep, SetCameraZoomStep,
    TakeScreenshotStep, WaitRealtimeStep, WaitSimulationTimeStep, WaitTicksStep,
};
