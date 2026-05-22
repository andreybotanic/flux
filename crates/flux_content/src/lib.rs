#![forbid(unsafe_code)]

mod error;
mod loader;
mod registry;
mod types;

pub use error::ContentRegistryError;
pub use loader::{ContentLoadReport, load_content_registry};
pub use registry::{ContentRegistry, FrozenContentRegistry, RegistryState};
pub use types::{
    LocalizationKey, PrototypeKind, PrototypeSource, StructurePrototype, StructureRecord,
    SubstancePrototype, SubstanceRecord, TileSize,
};
