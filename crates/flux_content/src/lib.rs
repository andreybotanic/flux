#![forbid(unsafe_code)]

mod error;
mod loader;
mod registry;
mod types;

pub use error::ContentRegistryError;
pub use loader::{ContentLoadReport, load_content_registry};
pub use registry::{ContentRegistry, FrozenContentRegistry, RegistryState};
pub use types::{
    AppliedPrototypePatch, AssetPath, GasPrototype, GasPrototypePatch, GasRecord, LocalizationKey,
    PatchResult, Prototype, PrototypeBody, PrototypeKind, PrototypePatch, PrototypePatchBody,
    PrototypePatchFor, PrototypeSource, SingleSpriteVisual, SolidCellPrototype,
    SolidCellPrototypePatch, SolidCellRecord, StructurePrototype, StructurePrototypePatch,
    StructureRecord, SubstancePrototype, SubstancePrototypePatch, SubstanceRecord, TileSize,
    VisualDefinition, VisualDefinitionKind,
};
