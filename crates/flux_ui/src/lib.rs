#![forbid(unsafe_code)]

mod error;
mod loader;
mod registry;
mod runtime;
mod types;

pub use error::UiRegistryError;
pub use loader::{UiLoadReport, load_ui_registry};
pub use registry::{UiMenuRecord, UiRegistry, UiRegistryState};
pub use runtime::{
    BuiltinUiActionDispatcher, UiActionContext, UiActionDispatcher, UiActionResult, UiMenuStack,
    UiRuntimeError,
};
pub use types::{
    ButtonWidget, ContainerLayout, ContainerWidget, ExtensionPointWidget, LocalizationKey,
    TextWidget, UiAction, UiDefinition, UiExtensionDefinition, UiExtensionOperation,
    UiExtensionPointId, UiMenuDefinition, UiMenuId, UiSource, UiWidgetId, WidgetKind, WidgetNode,
};
