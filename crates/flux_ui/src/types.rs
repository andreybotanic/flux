#![forbid(unsafe_code)]

use std::fmt::{Display, Formatter};

use flux_core::NamespacedId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UiMenuId(pub NamespacedId);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UiWidgetId(pub NamespacedId);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UiExtensionPointId(pub NamespacedId);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationKey(String);

impl LocalizationKey {
    pub fn parse(value: &str) -> Result<Self, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("value must not be empty".to_owned());
        }
        Ok(Self(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerLayout {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerWidget {
    pub layout: ContainerLayout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextWidget {
    pub text: LocalizationKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonWidget {
    pub text: LocalizationKey,
    pub action: BindingAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionPointWidget {
    pub extension_point: UiExtensionPointId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetKind {
    Container(ContainerWidget),
    Text(TextWidget),
    Button(ButtonWidget),
    ExtensionPoint(ExtensionPointWidget),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WidgetNode {
    pub id: UiWidgetId,
    pub kind: WidgetKind,
    pub children: Vec<WidgetNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiMenuDefinition {
    pub id: UiMenuId,
    pub root: WidgetNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiDefinition {
    Menu(UiMenuDefinition),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAction {
    OpenMenu(UiMenuId),
    BackMenu,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingAction {
    OpenMenu(UiMenuId),
    BackMenu,
    DiagnosticLog(String),
    RunWorld,
    SaveGame(String),
    LoadGame(String),
    ToggleSimulation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiExtensionOperation {
    AppendChild,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiExtensionDefinition {
    pub target: UiExtensionPointId,
    pub operation: UiExtensionOperation,
    pub child: WidgetNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiSource {
    pub mod_id: String,
    pub file: String,
}

impl Display for UiMenuId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Display for UiWidgetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Display for UiExtensionPointId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Display for LocalizationKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LocalizationKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

impl Serialize for LocalizationKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl WidgetNode {
    pub fn visit(&self, visitor: &mut dyn FnMut(&WidgetNode)) {
        visitor(self);
        for child in &self.children {
            child.visit(visitor);
        }
    }

    pub fn visit_mut(&mut self, visitor: &mut dyn FnMut(&mut WidgetNode)) {
        visitor(self);
        for child in &mut self.children {
            child.visit_mut(visitor);
        }
    }
}
