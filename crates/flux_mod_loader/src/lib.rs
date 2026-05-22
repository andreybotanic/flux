#![forbid(unsafe_code)]

mod dependency;
mod loader;
mod manifest;
mod order;
mod types;
mod utils;

pub use loader::discover_and_resolve_mods;
pub use types::{DiscoveredMod, ModDiscoveryReport, ModLoaderError, ModManifest, ResolvedModOrder};
