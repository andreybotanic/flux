use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use flux_core::PrototypeId;
use flux_mod_loader::{DiscoveredMod, ResolvedModOrder};
use serde::Deserialize;
use serde::de::Deserializer;

use crate::ContentRegistry;
use crate::ContentRegistryError;
use crate::types::{
    GasPrototype, GasPrototypePatch, LocalizationKey, PrototypeBody, PrototypeKind, PrototypePatch,
    PrototypePatchBody, PrototypeSource, SolidCellPrototype, SolidCellPrototypePatch,
    StructurePrototype, StructurePrototypePatch, SubstancePrototype, SubstancePrototypePatch,
    TileSize,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ContentLoadReport {
    pub registry: Option<ContentRegistry>,
    pub errors: Vec<ContentRegistryError>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
enum ParsedPrototypeBody {
    #[serde(rename = "SubstancePrototype")]
    Substance {
        id: PrototypeId,
        display_name: LocalizationKey,
    },
    #[serde(rename = "SolidCellPrototype")]
    SolidCell {
        id: PrototypeId,
        display_name: LocalizationKey,
        gas_permeable: bool,
    },
    #[serde(rename = "StructurePrototype")]
    Structure {
        id: PrototypeId,
        display_name: LocalizationKey,
        size: TileSize,
    },
    #[serde(rename = "GasPrototype")]
    Gas {
        id: PrototypeId,
        display_name: LocalizationKey,
        molar_mass: f32,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
enum ParsedPrototypePatchWrapper {
    PrototypePatch {
        target: PrototypeId,
        body: ParsedPrototypePatchBody,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
enum ParsedPrototypePatchBody {
    Substance {
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        display_name: Option<LocalizationKey>,
    },
    SolidCell {
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        display_name: Option<LocalizationKey>,
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        gas_permeable: Option<bool>,
    },
    Structure {
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        display_name: Option<LocalizationKey>,
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        size: Option<TileSize>,
    },
    Gas {
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        display_name: Option<LocalizationKey>,
        #[serde(default, deserialize_with = "deserialize_patch_option")]
        molar_mass: Option<f32>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PatchOptionValue<T> {
    Plain(T),
    Wrapped(Option<T>),
}

fn deserialize_patch_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let parsed = PatchOptionValue::<T>::deserialize(deserializer)?;
    Ok(match parsed {
        PatchOptionValue::Plain(value) => Some(value),
        PatchOptionValue::Wrapped(value) => value,
    })
}

pub fn load_content_registry(
    valid_mods: &[DiscoveredMod],
    resolved_order: &ResolvedModOrder,
) -> ContentLoadReport {
    let mut registry = ContentRegistry::new();
    let mut errors = Vec::new();

    let mods_by_id: BTreeMap<&str, &DiscoveredMod> = valid_mods
        .iter()
        .map(|module| (module.manifest.mod_id.as_str(), module))
        .collect();

    for mod_id in &resolved_order.ordered_mod_ids {
        let module = match mods_by_id.get(mod_id.as_str()) {
            Some(module) => *module,
            None => {
                errors.push(ContentRegistryError::ResolvedModMissing {
                    mod_id: mod_id.to_string().into(),
                });
                continue;
            }
        };

        load_mod_prototypes(module, &mut registry, &mut errors);
        apply_mod_patches(module, &mut registry, &mut errors);
    }

    if errors.is_empty() {
        registry.freeze();
        ContentLoadReport {
            registry: Some(registry),
            errors,
        }
    } else {
        ContentLoadReport {
            registry: None,
            errors,
        }
    }
}

fn load_mod_prototypes(
    module: &DiscoveredMod,
    registry: &mut ContentRegistry,
    errors: &mut Vec<ContentRegistryError>,
) {
    load_mod_prototypes_for_kind(
        module,
        registry,
        errors,
        "solid_cells",
        PrototypeKind::SolidCell,
    );
    load_mod_prototypes_for_kind(
        module,
        registry,
        errors,
        "substances",
        PrototypeKind::Substance,
    );
    load_mod_prototypes_for_kind(
        module,
        registry,
        errors,
        "structures",
        PrototypeKind::Structure,
    );
    load_mod_prototypes_for_kind(module, registry, errors, "gases", PrototypeKind::Gas);
}

fn load_mod_prototypes_for_kind(
    module: &DiscoveredMod,
    registry: &mut ContentRegistry,
    errors: &mut Vec<ContentRegistryError>,
    directory_name: &str,
    expected_kind: PrototypeKind,
) {
    let dir = module.directory_path.join("content").join(directory_name);
    for file in collect_ron_files(module, &dir, errors) {
        match parse_prototype(module, &file, expected_kind) {
            Ok((prototype, source)) => {
                if let Err(error) = registry.add_prototype(prototype, source) {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
    }
}

fn apply_mod_patches(
    module: &DiscoveredMod,
    registry: &mut ContentRegistry,
    errors: &mut Vec<ContentRegistryError>,
) {
    let dir = module.directory_path.join("content").join("patches");
    let mut seen_targets: BTreeMap<PrototypeId, String> = BTreeMap::new();

    for file in collect_ron_files(module, &dir, errors) {
        match parse_patch(module, &file) {
            Ok((patch, source)) => {
                if let Some(first_file) = seen_targets.get(&patch.target) {
                    errors.push(ContentRegistryError::DuplicatePatchTargetInMod {
                        mod_id: source.mod_id.clone().into(),
                        first_file: first_file.clone().into(),
                        duplicate_file: source.file.clone().into(),
                        target: patch.target.to_string().into(),
                    });
                    continue;
                }

                seen_targets.insert(patch.target.clone(), source.file.clone());
                if let Err(error) = registry.apply_patch(patch, source) {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
    }
}

fn parse_prototype(
    module: &DiscoveredMod,
    file: &Path,
    expected_kind: PrototypeKind,
) -> Result<(PrototypeBody, PrototypeSource), ContentRegistryError> {
    let source = PrototypeSource::from_discovered(module, file);
    let body = read_file(module, file)?;
    let parsed: ParsedPrototypeBody =
        ron::from_str(&body).map_err(|error| ContentRegistryError::FileParse {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_kind: expected_kind.as_str().into(),
            reason: error.to_string().into(),
        })?;

    let prototype = match parsed {
        ParsedPrototypeBody::Substance { id, display_name } => {
            PrototypeBody::SubstancePrototype(SubstancePrototype { id, display_name })
        }
        ParsedPrototypeBody::SolidCell {
            id,
            display_name,
            gas_permeable,
        } => PrototypeBody::SolidCellPrototype(SolidCellPrototype {
            id,
            display_name,
            gas_permeable,
        }),
        ParsedPrototypeBody::Structure {
            id,
            display_name,
            size,
        } => PrototypeBody::StructurePrototype(StructurePrototype {
            id,
            display_name,
            size,
        }),
        ParsedPrototypeBody::Gas {
            id,
            display_name,
            molar_mass,
        } => PrototypeBody::GasPrototype(GasPrototype {
            id,
            display_name,
            molar_mass,
        }),
    };

    if prototype.kind() != expected_kind {
        return Err(ContentRegistryError::FileParse {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_kind: expected_kind.as_str().into(),
            reason: format!(
                "expected {} wrapper, got {} wrapper",
                expected_kind.as_str(),
                prototype.kind().as_str()
            )
            .into(),
        });
    }

    validate_prototype_id_namespace(module, file, prototype.id())?;
    validate_prototype_body(&prototype, &source)?;

    Ok((prototype, source))
}

fn parse_patch(
    module: &DiscoveredMod,
    file: &Path,
) -> Result<(PrototypePatch, PrototypeSource), ContentRegistryError> {
    let source = PrototypeSource::from_discovered(module, file);
    let body = read_file(module, file)?;
    let parsed: ParsedPrototypePatchWrapper =
        ron::from_str(&body).map_err(|error| ContentRegistryError::FileParse {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_kind: "patch".into(),
            reason: error.to_string().into(),
        })?;

    let (target, parsed_body) = match parsed {
        ParsedPrototypePatchWrapper::PrototypePatch { target, body } => (target, body),
    };

    let patch_body = match parsed_body {
        ParsedPrototypePatchBody::Substance { display_name } => {
            PrototypePatchBody::Substance(SubstancePrototypePatch { display_name })
        }
        ParsedPrototypePatchBody::SolidCell {
            display_name,
            gas_permeable,
        } => PrototypePatchBody::SolidCell(SolidCellPrototypePatch {
            display_name,
            gas_permeable,
        }),
        ParsedPrototypePatchBody::Structure { display_name, size } => {
            PrototypePatchBody::Structure(StructurePrototypePatch { display_name, size })
        }
        ParsedPrototypePatchBody::Gas {
            display_name,
            molar_mass,
        } => PrototypePatchBody::Gas(GasPrototypePatch {
            display_name,
            molar_mass,
        }),
    };

    let patch = PrototypePatch {
        target,
        body: patch_body,
    };

    Ok((patch, source))
}

fn validate_prototype_body(
    prototype: &PrototypeBody,
    source: &PrototypeSource,
) -> Result<(), ContentRegistryError> {
    match prototype {
        PrototypeBody::SubstancePrototype(_) => Ok(()),
        PrototypeBody::SolidCellPrototype(_) => Ok(()),
        PrototypeBody::StructurePrototype(prototype) => prototype.validate(source),
        PrototypeBody::GasPrototype(prototype) => prototype.validate(source),
    }
}

fn validate_prototype_id_namespace(
    module: &DiscoveredMod,
    file: &Path,
    prototype_id: &PrototypeId,
) -> Result<(), ContentRegistryError> {
    if prototype_id.namespace() == module.manifest.mod_id.as_str() {
        return Ok(());
    }

    Err(ContentRegistryError::InvalidPrototypeField {
        mod_id: module.manifest.mod_id.to_string().into(),
        file: file.to_string_lossy().to_string().into(),
        prototype_id: prototype_id.to_string().into(),
        field: "id".into(),
        reason: format!(
            "prototype namespace `{}` must match mod id `{}`",
            prototype_id.namespace(),
            module.manifest.mod_id
        )
        .into(),
    })
}

fn collect_ron_files(
    module: &DiscoveredMod,
    directory: &Path,
    errors: &mut Vec<ContentRegistryError>,
) -> Vec<PathBuf> {
    if !directory.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();
    if let Err(error) =
        collect_ron_files_recursive(module.manifest.mod_id.as_str(), directory, &mut files)
    {
        errors.push(error);
        return Vec::new();
    }

    files.sort_by(|left, right| {
        normalized_relative_path(directory, left).cmp(&normalized_relative_path(directory, right))
    });
    files
}

fn collect_ron_files_recursive(
    mod_id: &str,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), ContentRegistryError> {
    let read_dir =
        fs::read_dir(directory).map_err(|error| ContentRegistryError::DirectoryRead {
            mod_id: mod_id.to_owned().into(),
            path: directory.to_string_lossy().to_string().into(),
            reason: error.to_string().into(),
        })?;

    let mut entries = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|error| ContentRegistryError::DirectoryRead {
            mod_id: mod_id.to_owned().into(),
            path: directory.to_string_lossy().to_string().into(),
            reason: error.to_string().into(),
        })?;
        entries.push(entry.path());
    }
    entries.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });

    for entry_path in entries {
        if entry_path.is_dir() {
            collect_ron_files_recursive(mod_id, &entry_path, files)?;
            continue;
        }

        if is_ron_file(&entry_path) {
            files.push(entry_path);
        }
    }

    Ok(())
}

fn normalized_relative_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

fn is_ron_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ron"))
}

fn read_file(module: &DiscoveredMod, file: &Path) -> Result<String, ContentRegistryError> {
    fs::read_to_string(file).map_err(|error| ContentRegistryError::FileRead {
        mod_id: module.manifest.mod_id.to_string().into(),
        file: file.to_string_lossy().to_string().into(),
        reason: error.to_string().into(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use flux_mod_loader::discover_and_resolve_mods;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn loads_typed_content_and_freezes_registry() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/solid_cells")).expect("create dir");
        fs::create_dir_all(mods_root.join("base/content/substances")).expect("create dir");
        fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
        fs::create_dir_all(mods_root.join("base/content/gases")).expect("create dir");
        write_manifest(
            &mods_root.join("base/manifest.toml"),
            r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
        );
        fs::write(
            mods_root.join("base/content/solid_cells/floor_cell.ron"),
            r#"SolidCellPrototype(id: "base:solid_cell/floor_cell", display_name: "base.solid_cell.floor_cell", gas_permeable: false)"#,
        )
        .expect("write solid cell");
        fs::write(
            mods_root.join("base/content/substances/oxygen.ron"),
            r#"SubstancePrototype(id: "base:material/oxygen", display_name: "base.substance.oxygen")"#,
        )
        .expect("write substance");
        fs::write(
            mods_root.join("base/content/structures/pump.ron"),
            r#"StructurePrototype(id: "base:building/gas_pump", display_name: "base.structure.gas_pump", size: (width: 1, height: 2))"#,
        )
        .expect("write structure");
        fs::write(
            mods_root.join("base/content/gases/oxygen.ron"),
            r#"GasPrototype(id: "base:gas/oxygen", display_name: "base.gas.oxygen", molar_mass: 31.998)"#,
        )
        .expect("write gas");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report = load_content_registry(
            &report.valid_mods,
            &report.resolved_order.expect("resolved order"),
        );
        let registry = load_report.registry.expect("registry");

        assert!(load_report.errors.is_empty());
        assert!(registry.is_frozen());
        assert_eq!(registry.solid_cells_len(), 1);
        assert_eq!(registry.substances_len(), 1);
        assert_eq!(registry.structures_len(), 1);
        assert_eq!(registry.gases_len(), 1);
    }

    #[test]
    fn rejects_non_typed_ron_body() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/substances")).expect("create dir");
        write_manifest(
            &mods_root.join("base/manifest.toml"),
            r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
        );
        fs::write(
            mods_root.join("base/content/substances/broken.ron"),
            r#"(id: "base:material/oxygen", display_name: "base.substance.oxygen")"#,
        )
        .expect("write file");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report =
            load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

        assert!(load_report.registry.is_none());
        assert!(load_report.errors.iter().any(|error| {
            matches!(
                error,
                ContentRegistryError::FileParse {
                    file,
                    prototype_kind,
                    ..
                } if file.ends_with("broken.ron") && prototype_kind.as_ref() == "substance"
            )
        }));
    }

    #[test]
    fn applies_patch_and_tracks_patch_history() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
        fs::create_dir_all(mods_root.join("test_content_mod/content/patches")).expect("create dir");

        write_manifest(
            &mods_root.join("base/manifest.toml"),
            r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
        );
        write_manifest(
            &mods_root.join("test_content_mod/manifest.toml"),
            r#"
[mod]
id = "test_content_mod"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
        );

        fs::write(
            mods_root.join("base/content/structures/ladder.ron"),
            r#"StructurePrototype(id: "base:building/ladder", display_name: "base.structure.ladder", size: (width: 1, height: 1))"#,
        )
        .expect("write structure");
        fs::write(
            mods_root.join("test_content_mod/content/patches/base_building_ladder.ron"),
            r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.ladder", size: (width: 2, height: 1)))"#,
        )
        .expect("write patch");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report =
            load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));
        let registry = load_report.registry.expect("registry");

        assert!(load_report.errors.is_empty());
        let ladder = registry
            .structures()
            .find(|record| record.prototype.id.as_str() == "base:building/ladder")
            .expect("ladder");
        assert_eq!(
            ladder.prototype.display_name.as_str(),
            "test_content_mod.structure.ladder"
        );
        assert_eq!(ladder.prototype.size.width, 2);

        let patch_sources: Vec<&str> = registry
            .applied_patches_for(&ladder.prototype.id)
            .map(|patch| patch.source.mod_id.as_str())
            .collect();
        assert_eq!(patch_sources, vec!["test_content_mod"]);
    }

    #[test]
    fn rejects_duplicate_patch_target_within_mod() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
        fs::create_dir_all(mods_root.join("test_content_mod/content/patches")).expect("create dir");

        write_manifest(
            &mods_root.join("base/manifest.toml"),
            r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
        );
        write_manifest(
            &mods_root.join("test_content_mod/manifest.toml"),
            r#"
[mod]
id = "test_content_mod"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
        );
        fs::write(
            mods_root.join("base/content/structures/ladder.ron"),
            r#"StructurePrototype(id: "base:building/ladder", display_name: "base.structure.ladder", size: (width: 1, height: 1))"#,
        )
        .expect("write structure");
        fs::write(
            mods_root.join("test_content_mod/content/patches/a.ron"),
            r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.a"))"#,
        )
        .expect("write patch");
        fs::write(
            mods_root.join("test_content_mod/content/patches/b.ron"),
            r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.b"))"#,
        )
        .expect("write patch");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report =
            load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

        assert!(load_report.registry.is_none());
        assert!(load_report.errors.iter().any(|error| {
            matches!(error, ContentRegistryError::DuplicatePatchTargetInMod { target, .. } if target.as_ref() == "base:building/ladder")
        }));
    }

    #[test]
    fn later_mod_patch_overrides_earlier_mod_patch() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");

        fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
        fs::create_dir_all(mods_root.join("test_content_mod/content/patches")).expect("create dir");
        fs::create_dir_all(mods_root.join("another_test_mod/content/patches")).expect("create dir");

        write_manifest(
            &mods_root.join("base/manifest.toml"),
            r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
        );
        write_manifest(
            &mods_root.join("test_content_mod/manifest.toml"),
            r#"
[mod]
id = "test_content_mod"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
        );
        write_manifest(
            &mods_root.join("another_test_mod/manifest.toml"),
            r#"
[mod]
id = "another_test_mod"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
test_content_mod = "*"
"#,
        );

        fs::write(
            mods_root.join("base/content/structures/ladder.ron"),
            r#"StructurePrototype(id: "base:building/ladder", display_name: "base.structure.ladder", size: (width: 1, height: 1))"#,
        )
        .expect("write structure");
        fs::write(
            mods_root.join("test_content_mod/content/patches/base_building_ladder.ron"),
            r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.ladder"))"#,
        )
        .expect("write patch");
        fs::write(
            mods_root.join("another_test_mod/content/patches/base_building_ladder.ron"),
            r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "another_test_mod.structure.ladder", size: (width: 3, height: 1)))"#,
        )
        .expect("write patch");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report =
            load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));
        let registry = load_report.registry.expect("registry");

        assert!(load_report.errors.is_empty());

        let ladder = registry
            .structures()
            .find(|record| record.prototype.id.as_str() == "base:building/ladder")
            .expect("ladder");
        assert_eq!(
            ladder.prototype.display_name.as_str(),
            "another_test_mod.structure.ladder"
        );
        assert_eq!(ladder.prototype.size.width, 3);

        let patch_sources: Vec<&str> = registry
            .applied_patches_for(&ladder.prototype.id)
            .map(|patch| patch.source.mod_id.as_str())
            .collect();
        assert_eq!(patch_sources, vec!["test_content_mod", "another_test_mod"]);
    }

    #[test]
    fn patch_file_order_is_lexicographic_inside_mod() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");

        fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
        fs::create_dir_all(mods_root.join("test_content_mod/content/patches/a"))
            .expect("create dir");
        fs::create_dir_all(mods_root.join("test_content_mod/content/patches/b"))
            .expect("create dir");

        write_manifest(
            &mods_root.join("base/manifest.toml"),
            r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
        );
        write_manifest(
            &mods_root.join("test_content_mod/manifest.toml"),
            r#"
[mod]
id = "test_content_mod"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
        );
        fs::write(
            mods_root.join("base/content/structures/ladder.ron"),
            r#"StructurePrototype(id: "base:building/ladder", display_name: "base.structure.ladder", size: (width: 1, height: 1))"#,
        )
        .expect("write structure");
        fs::write(
            mods_root.join("test_content_mod/content/patches/b/002.ron"),
            r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.second"))"#,
        )
        .expect("write patch");
        fs::write(
            mods_root.join("test_content_mod/content/patches/a/001.ron"),
            r#"PrototypePatch(target: "base:building/ladder", body: Structure(display_name: "test_content_mod.structure.first"))"#,
        )
        .expect("write patch");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report =
            load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

        assert!(load_report.registry.is_none());
        assert!(load_report.errors.iter().any(|error| {
            matches!(error, ContentRegistryError::DuplicatePatchTargetInMod { first_file, duplicate_file, .. }
                if first_file.replace('\\', "/").ends_with("a/001.ron")
                    && duplicate_file.replace('\\', "/").ends_with("b/002.ron"))
        }));
    }

    #[test]
    fn rejects_empty_patch() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
        fs::create_dir_all(mods_root.join("test_content_mod/content/patches")).expect("create dir");

        write_manifest(
            &mods_root.join("base/manifest.toml"),
            r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
        );
        write_manifest(
            &mods_root.join("test_content_mod/manifest.toml"),
            r#"
[mod]
id = "test_content_mod"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
        );
        fs::write(
            mods_root.join("base/content/structures/ladder.ron"),
            r#"StructurePrototype(id: "base:building/ladder", display_name: "base.structure.ladder", size: (width: 1, height: 1))"#,
        )
        .expect("write structure");
        fs::write(
            mods_root.join("test_content_mod/content/patches/empty.ron"),
            r#"PrototypePatch(target: "base:building/ladder", body: Structure())"#,
        )
        .expect("write patch");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report =
            load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

        assert!(load_report.registry.is_none());
        assert!(load_report.errors.iter().any(|error| {
            matches!(error, ContentRegistryError::EmptyPatchBody { target, .. } if target.as_ref() == "base:building/ladder")
        }));
    }

    #[test]
    fn rejects_patch_target_not_found() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
        fs::create_dir_all(mods_root.join("test_content_mod/content/patches")).expect("create dir");

        write_manifest(
            &mods_root.join("base/manifest.toml"),
            r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
        );
        write_manifest(
            &mods_root.join("test_content_mod/manifest.toml"),
            r#"
[mod]
id = "test_content_mod"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
        );
        fs::write(
            mods_root.join("base/content/structures/ladder.ron"),
            r#"StructurePrototype(id: "base:building/ladder", display_name: "base.structure.ladder", size: (width: 1, height: 1))"#,
        )
        .expect("write structure");
        fs::write(
            mods_root.join("test_content_mod/content/patches/unknown.ron"),
            r#"PrototypePatch(target: "base:building/unknown", body: Structure(display_name: "test_content_mod.structure.unknown"))"#,
        )
        .expect("write patch");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report =
            load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

        assert!(load_report.registry.is_none());
        assert!(load_report.errors.iter().any(|error| {
            matches!(error, ContentRegistryError::PatchTargetNotFound { target, .. } if target.as_ref() == "base:building/unknown")
        }));
    }

    #[test]
    fn rejects_patch_kind_mismatch() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
        fs::create_dir_all(mods_root.join("test_content_mod/content/patches")).expect("create dir");

        write_manifest(
            &mods_root.join("base/manifest.toml"),
            r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
        );
        write_manifest(
            &mods_root.join("test_content_mod/manifest.toml"),
            r#"
[mod]
id = "test_content_mod"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
        );
        fs::write(
            mods_root.join("base/content/structures/ladder.ron"),
            r#"StructurePrototype(id: "base:building/ladder", display_name: "base.structure.ladder", size: (width: 1, height: 1))"#,
        )
        .expect("write structure");
        fs::write(
            mods_root.join("test_content_mod/content/patches/wrong_kind.ron"),
            r#"PrototypePatch(target: "base:building/ladder", body: Gas(display_name: "test_content_mod.gas.fake"))"#,
        )
        .expect("write patch");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report =
            load_content_registry(&report.valid_mods, &report.resolved_order.expect("order"));

        assert!(load_report.registry.is_none());
        assert!(load_report.errors.iter().any(|error| {
            matches!(error, ContentRegistryError::PatchKindMismatch { target, .. } if target.as_ref() == "base:building/ladder")
        }));
    }

    #[test]
    fn mod_order_is_deterministic_from_resolved_order() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/substances")).expect("create dir");
        fs::create_dir_all(mods_root.join("zeta/content/substances")).expect("create dir");
        fs::create_dir_all(mods_root.join("alpha/content/substances")).expect("create dir");

        write_manifest(
            &mods_root.join("base/manifest.toml"),
            r#"
[mod]
id = "base"
version = "1.0.0"
api_version = "0.1.0"
"#,
        );
        write_manifest(
            &mods_root.join("zeta/manifest.toml"),
            r#"
[mod]
id = "zeta"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
        );
        write_manifest(
            &mods_root.join("alpha/manifest.toml"),
            r#"
[mod]
id = "alpha"
version = "1.0.0"
api_version = "0.1.0"

[dependencies]
base = "*"
"#,
        );

        fs::write(
            mods_root.join("base/content/substances/base.ron"),
            r#"SubstancePrototype(id: "base:material/base", display_name: "base.substance.base")"#,
        )
        .expect("write file");
        fs::write(
            mods_root.join("zeta/content/substances/broken.ron"),
            r#"SubstancePrototype(id: "zeta:material/zeta", display_name: )"#,
        )
        .expect("write file");
        fs::write(
            mods_root.join("alpha/content/substances/broken.ron"),
            r#"SubstancePrototype(id: "alpha:material/alpha", display_name: )"#,
        )
        .expect("write file");

        let report = discover_and_resolve_mods(&mods_root);
        let resolved = report.resolved_order.expect("resolved order");
        let load_report = load_content_registry(&report.valid_mods, &resolved);
        assert!(load_report.registry.is_none());

        let ordered_error_mods: Vec<&str> = load_report
            .errors
            .iter()
            .filter_map(|error| match error {
                ContentRegistryError::FileParse { mod_id, .. } => Some(mod_id.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(ordered_error_mods, vec!["alpha", "zeta"]);
    }

    fn write_manifest(path: &Path, manifest: &str) {
        fs::create_dir_all(path.parent().expect("parent dir")).expect("create parent");
        fs::write(path, manifest.trim()).expect("write manifest");
    }
}
