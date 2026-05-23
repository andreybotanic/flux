use std::fmt::{Display, Formatter};
use std::str::FromStr;

use flux_core::PrototypeId;
use flux_mod_loader::DiscoveredMod;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ContentRegistryError;

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

impl Display for LocalizationKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LocalizationKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for LocalizationKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LocalizationKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileSize {
    pub width: u16,
    pub height: u16,
}

impl TileSize {
    pub fn validate(
        &self,
        prototype_id: &PrototypeId,
        source: &PrototypeSource,
    ) -> Result<(), ContentRegistryError> {
        if self.width == 0 {
            return Err(ContentRegistryError::InvalidPrototypeField {
                mod_id: source.mod_id.clone().into(),
                file: source.file.clone().into(),
                prototype_id: prototype_id.to_string().into(),
                field: "size.width".into(),
                reason: "must be greater than 0".into(),
            });
        }

        if self.height == 0 {
            return Err(ContentRegistryError::InvalidPrototypeField {
                mod_id: source.mod_id.clone().into(),
                file: source.file.clone().into(),
                prototype_id: prototype_id.to_string().into(),
                field: "size.height".into(),
                reason: "must be greater than 0".into(),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubstancePrototype {
    pub id: PrototypeId,
    pub display_name: LocalizationKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolidCellPrototype {
    pub id: PrototypeId,
    pub display_name: LocalizationKey,
    pub gas_permeable: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasPrototype {
    pub id: PrototypeId,
    pub display_name: LocalizationKey,
    pub molar_mass: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructurePrototype {
    pub id: PrototypeId,
    pub display_name: LocalizationKey,
    pub size: TileSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrototypeKind {
    Substance,
    SolidCell,
    Structure,
    Gas,
}

impl PrototypeKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Substance => "substance",
            Self::SolidCell => "solid_cell",
            Self::Structure => "structure",
            Self::Gas => "gas",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrototypeSource {
    pub mod_id: String,
    pub file: String,
}

impl PrototypeSource {
    #[must_use]
    pub fn from_discovered(module: &DiscoveredMod, file: &std::path::Path) -> Self {
        Self {
            mod_id: module.manifest.mod_id.to_string(),
            file: file.to_string_lossy().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubstanceRecord {
    pub prototype: SubstancePrototype,
    pub source: PrototypeSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolidCellRecord {
    pub prototype: SolidCellPrototype,
    pub source: PrototypeSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureRecord {
    pub prototype: StructurePrototype,
    pub source: PrototypeSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GasRecord {
    pub prototype: GasPrototype,
    pub source: PrototypeSource,
}
