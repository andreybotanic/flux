use std::fmt::{Display, Formatter};
use std::str::FromStr;

use flux_core::PrototypeId;
use flux_mod_loader::DiscoveredMod;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ContentRegistryError;

pub type PatchResult = Result<(), String>;

pub trait Prototype: Sized {
    type Patch: PrototypePatchFor<Self>;

    const KIND: PrototypeKind;
}

pub trait PrototypePatchFor<P: Prototype> {
    fn is_empty(&self) -> bool;

    fn apply_to(self, target: &mut P) -> PatchResult;
}

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

impl StructurePrototype {
    pub fn validate(&self, source: &PrototypeSource) -> Result<(), ContentRegistryError> {
        self.size.validate(&self.id, source)
    }
}

impl GasPrototype {
    pub fn validate(&self, source: &PrototypeSource) -> Result<(), ContentRegistryError> {
        if self.molar_mass.is_finite() && self.molar_mass > 0.0 {
            return Ok(());
        }

        Err(ContentRegistryError::InvalidPrototypeField {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_id: self.id.to_string().into(),
            field: "molar_mass".into(),
            reason: format!(
                "molar_mass must be finite and greater than zero, got {}",
                self.molar_mass
            )
            .into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubstancePrototypePatch {
    #[serde(default)]
    pub display_name: Option<LocalizationKey>,
}

impl PrototypePatchFor<SubstancePrototype> for SubstancePrototypePatch {
    fn is_empty(&self) -> bool {
        self.display_name.is_none()
    }

    fn apply_to(self, target: &mut SubstancePrototype) -> PatchResult {
        if let Some(display_name) = self.display_name {
            target.display_name = display_name;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolidCellPrototypePatch {
    #[serde(default)]
    pub display_name: Option<LocalizationKey>,
    #[serde(default)]
    pub gas_permeable: Option<bool>,
}

impl PrototypePatchFor<SolidCellPrototype> for SolidCellPrototypePatch {
    fn is_empty(&self) -> bool {
        self.display_name.is_none() && self.gas_permeable.is_none()
    }

    fn apply_to(self, target: &mut SolidCellPrototype) -> PatchResult {
        if let Some(display_name) = self.display_name {
            target.display_name = display_name;
        }
        if let Some(gas_permeable) = self.gas_permeable {
            target.gas_permeable = gas_permeable;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructurePrototypePatch {
    #[serde(default)]
    pub display_name: Option<LocalizationKey>,
    #[serde(default)]
    pub size: Option<TileSize>,
}

impl PrototypePatchFor<StructurePrototype> for StructurePrototypePatch {
    fn is_empty(&self) -> bool {
        self.display_name.is_none() && self.size.is_none()
    }

    fn apply_to(self, target: &mut StructurePrototype) -> PatchResult {
        if let Some(display_name) = self.display_name {
            target.display_name = display_name;
        }
        if let Some(size) = self.size {
            target.size = size;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasPrototypePatch {
    #[serde(default)]
    pub display_name: Option<LocalizationKey>,
    #[serde(default)]
    pub molar_mass: Option<f32>,
}

impl PrototypePatchFor<GasPrototype> for GasPrototypePatch {
    fn is_empty(&self) -> bool {
        self.display_name.is_none() && self.molar_mass.is_none()
    }

    fn apply_to(self, target: &mut GasPrototype) -> PatchResult {
        if let Some(display_name) = self.display_name {
            target.display_name = display_name;
        }
        if let Some(molar_mass) = self.molar_mass {
            target.molar_mass = molar_mass;
        }
        Ok(())
    }
}

macro_rules! define_prototype_kinds {
    ($(
        $kind_variant:ident => (
            kind_str: $kind_str:literal,
            prototype: $prototype_ty:ty,
            prototype_body: $prototype_body_variant:ident,
            patch: $patch_ty:ty,
            patch_body: $patch_body_variant:ident
        )
    ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum PrototypeKind {
            $($kind_variant),+
        }

        impl PrototypeKind {
            #[must_use]
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$kind_variant => $kind_str),+
                }
            }
        }

        #[derive(Debug, Clone, PartialEq)]
        pub enum PrototypeBody {
            $($prototype_body_variant($prototype_ty)),+
        }

        impl PrototypeBody {
            #[must_use]
            pub fn kind(&self) -> PrototypeKind {
                match self {
                    $(Self::$prototype_body_variant(_) => PrototypeKind::$kind_variant),+
                }
            }

            #[must_use]
            pub fn id(&self) -> &PrototypeId {
                match self {
                    $(Self::$prototype_body_variant(prototype) => &prototype.id),+
                }
            }
        }

        #[derive(Debug, Clone, PartialEq)]
        pub enum PrototypePatchBody {
            $($patch_body_variant($patch_ty)),+
        }

        impl PrototypePatchBody {
            #[must_use]
            pub fn kind(&self) -> PrototypeKind {
                match self {
                    $(Self::$patch_body_variant(_) => PrototypeKind::$kind_variant),+
                }
            }

            #[must_use]
            pub fn is_empty(&self) -> bool {
                match self {
                    $(Self::$patch_body_variant(body) => body.is_empty()),+
                }
            }
        }

        $(
            impl Prototype for $prototype_ty {
                type Patch = $patch_ty;

                const KIND: PrototypeKind = PrototypeKind::$kind_variant;
            }
        )+
    };
}

define_prototype_kinds! {
    Substance => (
        kind_str: "substance",
        prototype: SubstancePrototype,
        prototype_body: SubstancePrototype,
        patch: SubstancePrototypePatch,
        patch_body: Substance
    ),
    SolidCell => (
        kind_str: "solid_cell",
        prototype: SolidCellPrototype,
        prototype_body: SolidCellPrototype,
        patch: SolidCellPrototypePatch,
        patch_body: SolidCell
    ),
    Structure => (
        kind_str: "structure",
        prototype: StructurePrototype,
        prototype_body: StructurePrototype,
        patch: StructurePrototypePatch,
        patch_body: Structure
    ),
    Gas => (
        kind_str: "gas",
        prototype: GasPrototype,
        prototype_body: GasPrototype,
        patch: GasPrototypePatch,
        patch_body: Gas
    ),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrototypePatch {
    pub target: PrototypeId,
    pub body: PrototypePatchBody,
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
pub struct AppliedPrototypePatch {
    pub source: PrototypeSource,
    pub patch_kind: PrototypeKind,
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
