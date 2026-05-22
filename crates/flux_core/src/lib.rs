#![forbid(unsafe_code)]

mod error;
mod id;
mod version;

pub use error::{IdError, ModIdError, NamespacedIdError, VersionParseError};
pub use id::{ModId, NamespacedId, PrototypeId};
pub use version::{ApiVersion, EngineVersion, ModVersion, engine_version};

/// Human-readable engine name used by diagnostics and CLI output.
pub const ENGINE_NAME: &str = "FluxEngine";

/// Bootstrap version for this binary.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[must_use]
pub fn engine_label() -> String {
    format!("{ENGINE_NAME} {ENGINE_VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_label_contains_name() {
        assert!(engine_label().contains(ENGINE_NAME));
    }
}
