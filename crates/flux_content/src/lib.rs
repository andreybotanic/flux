#![forbid(unsafe_code)]

mod error;
mod loader;
mod registry;
mod types;

pub use error::ContentRegistryError;
pub use loader::{ContentLoadReport, load_content_registry};
pub use registry::{ContentRegistry, FrozenContentRegistry, RegistryState};
pub use types::{
    GasPrototype, GasRecord, LocalizationKey, PrototypeKind, PrototypeSource, SolidCellPrototype,
    SolidCellRecord, StructurePrototype, StructureRecord, SubstancePrototype, SubstanceRecord,
    TileSize,
};
