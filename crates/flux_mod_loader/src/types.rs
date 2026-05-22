use std::collections::BTreeMap;
use std::path::PathBuf;

use flux_core::{ApiVersion, ModId, ModVersion};
use semver::VersionReq;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModManifest {
    pub mod_id: ModId,
    pub version: ModVersion,
    pub api_version: ApiVersion,
    pub dependencies: BTreeMap<ModId, VersionReq>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredMod {
    pub manifest: ModManifest,
    pub directory_path: PathBuf,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModOrder {
    pub ordered_mod_ids: Vec<ModId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDiscoveryReport {
    pub valid_mods: Vec<DiscoveredMod>,
    pub errors: Vec<ModLoaderError>,
    pub resolved_order: Option<ResolvedModOrder>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ModLoaderError {
    #[error("ModDiscoveryError:\n  path: {path}\n  reason: failed to inspect mods root ({reason})")]
    ModsRootRead { path: String, reason: String },

    #[error(
        "ModManifestError:\n  mod: {mod_hint}\n  file: {file}\n  field: manifest\n  reason: missing manifest.toml"
    )]
    MissingManifest { mod_hint: String, file: String },

    #[error(
        "ModManifestError:\n  mod: {mod_hint}\n  file: {file}\n  field: manifest\n  reason: {reason}"
    )]
    ReadManifest {
        mod_hint: String,
        file: String,
        reason: String,
    },

    #[error(
        "ModManifestError:\n  mod: {mod_hint}\n  file: {file}\n  field: manifest\n  reason: invalid TOML ({reason})"
    )]
    ManifestToml {
        mod_hint: String,
        file: String,
        reason: String,
    },

    #[error(
        "ModManifestError:\n  mod: {mod_hint}\n  file: {file}\n  field: {field}\n  reason: {reason}"
    )]
    ManifestField {
        mod_hint: String,
        file: String,
        field: String,
        reason: String,
    },

    #[error(
        "ModDependencyError:\n  mod: {mod_id}\n  dependency: {dependency}\n  reason: dependency references itself"
    )]
    SelfDependency { mod_id: String, dependency: String },

    #[error(
        "ModDependencyError:\n  mod: {mod_id}\n  dependency: {dependency}\n  reason: dependency not found among valid manifests"
    )]
    MissingDependency { mod_id: String, dependency: String },

    #[error(
        "ModDependencyError:\n  mod: {mod_id}\n  dependency: {dependency}\n  reason: version mismatch, required {required}, actual {actual}"
    )]
    DependencyVersionMismatch {
        mod_id: String,
        dependency: String,
        required: String,
        actual: String,
    },

    #[error("ModDependencyError:\n  reason: cycle detected ({cycle})")]
    DependencyCycle { cycle: String },
}
