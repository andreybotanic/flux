use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use flux_core::PrototypeId;
use flux_mod_loader::{DiscoveredMod, ResolvedModOrder};

use crate::ContentRegistry;
use crate::ContentRegistryError;
use crate::types::{
    GasPrototype, PrototypeSource, SolidCellPrototype, StructurePrototype, SubstancePrototype,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ContentLoadReport {
    pub registry: Option<ContentRegistry>,
    pub errors: Vec<ContentRegistryError>,
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

        load_mod_solid_cells(module, &mut registry, &mut errors);
        load_mod_substances(module, &mut registry, &mut errors);
        load_mod_structures(module, &mut registry, &mut errors);
        load_mod_gases(module, &mut registry, &mut errors);
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

fn load_mod_substances(
    module: &DiscoveredMod,
    registry: &mut ContentRegistry,
    errors: &mut Vec<ContentRegistryError>,
) {
    let dir = module.directory_path.join("content").join("substances");
    for file in collect_ron_files(module, &dir, errors) {
        match parse_substance(module, &file) {
            Ok((prototype, source)) => {
                if let Err(error) = registry.add_substance(prototype, source) {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
    }
}

fn load_mod_solid_cells(
    module: &DiscoveredMod,
    registry: &mut ContentRegistry,
    errors: &mut Vec<ContentRegistryError>,
) {
    let dir = module.directory_path.join("content").join("solid_cells");
    for file in collect_ron_files(module, &dir, errors) {
        match parse_solid_cell(module, &file) {
            Ok((prototype, source)) => {
                if let Err(error) = registry.add_solid_cell(prototype, source) {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
    }
}

fn load_mod_structures(
    module: &DiscoveredMod,
    registry: &mut ContentRegistry,
    errors: &mut Vec<ContentRegistryError>,
) {
    let dir = module.directory_path.join("content").join("structures");
    for file in collect_ron_files(module, &dir, errors) {
        match parse_structure(module, &file) {
            Ok((prototype, source)) => {
                if let Err(error) = prototype.size.validate(&prototype.id, &source) {
                    errors.push(error);
                    continue;
                }

                if let Err(error) = registry.add_structure(prototype, source) {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
    }
}

fn load_mod_gases(
    module: &DiscoveredMod,
    registry: &mut ContentRegistry,
    errors: &mut Vec<ContentRegistryError>,
) {
    let dir = module.directory_path.join("content").join("gases");
    for file in collect_ron_files(module, &dir, errors) {
        match parse_gas(module, &file) {
            Ok((prototype, source)) => {
                if let Err(error) = registry.add_gas(prototype, source) {
                    errors.push(error);
                }
            }
            Err(error) => errors.push(error),
        }
    }
}

fn parse_solid_cell(
    module: &DiscoveredMod,
    file: &Path,
) -> Result<(SolidCellPrototype, PrototypeSource), ContentRegistryError> {
    let source = PrototypeSource::from_discovered(module, file);
    let body = read_file(module, file)?;
    let prototype: SolidCellPrototype =
        ron::from_str(&body).map_err(|error| ContentRegistryError::FileParse {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_kind: "solid_cell".into(),
            reason: error.to_string().into(),
        })?;

    validate_prototype_id_namespace(module, file, &prototype.id)?;
    Ok((prototype, source))
}

fn parse_substance(
    module: &DiscoveredMod,
    file: &Path,
) -> Result<(SubstancePrototype, PrototypeSource), ContentRegistryError> {
    let source = PrototypeSource::from_discovered(module, file);
    let body = read_file(module, file)?;
    let prototype: SubstancePrototype =
        ron::from_str(&body).map_err(|error| ContentRegistryError::FileParse {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_kind: "substance".into(),
            reason: error.to_string().into(),
        })?;

    validate_prototype_id_namespace(module, file, &prototype.id)?;
    Ok((prototype, source))
}

fn parse_structure(
    module: &DiscoveredMod,
    file: &Path,
) -> Result<(StructurePrototype, PrototypeSource), ContentRegistryError> {
    let source = PrototypeSource::from_discovered(module, file);
    let body = read_file(module, file)?;
    let prototype: StructurePrototype =
        ron::from_str(&body).map_err(|error| ContentRegistryError::FileParse {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_kind: "structure".into(),
            reason: error.to_string().into(),
        })?;

    validate_prototype_id_namespace(module, file, &prototype.id)?;
    Ok((prototype, source))
}

fn parse_gas(
    module: &DiscoveredMod,
    file: &Path,
) -> Result<(GasPrototype, PrototypeSource), ContentRegistryError> {
    let source = PrototypeSource::from_discovered(module, file);
    let body = read_file(module, file)?;
    let prototype: GasPrototype =
        ron::from_str(&body).map_err(|error| ContentRegistryError::FileParse {
            mod_id: source.mod_id.clone().into(),
            file: source.file.clone().into(),
            prototype_kind: "gas".into(),
            reason: error.to_string().into(),
        })?;

    validate_prototype_id_namespace(module, file, &prototype.id)?;
    validate_gas_molar_mass(&source, &prototype)?;
    Ok((prototype, source))
}

fn validate_gas_molar_mass(
    source: &PrototypeSource,
    prototype: &GasPrototype,
) -> Result<(), ContentRegistryError> {
    if prototype.molar_mass.is_finite() && prototype.molar_mass > 0.0 {
        return Ok(());
    }

    Err(ContentRegistryError::InvalidPrototypeField {
        mod_id: source.mod_id.clone().into(),
        file: source.file.clone().into(),
        prototype_id: prototype.id.to_string().into(),
        field: "molar_mass".into(),
        reason: format!(
            "molar_mass must be finite and greater than zero, got {}",
            prototype.molar_mass
        )
        .into(),
    })
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
    fn loads_content_and_freezes_registry() {
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
            "(id: \"base:solid_cell/floor_cell\", display_name: \"base.solid_cell.floor_cell\", gas_permeable: false)",
        )
        .expect("write solid cell");
        fs::write(
            mods_root.join("base/content/substances/oxygen.ron"),
            "(id: \"base:material/oxygen\", display_name: \"base.substance.oxygen\")",
        )
        .expect("write substance");
        fs::write(
            mods_root.join("base/content/structures/pump.ron"),
            "(id: \"base:building/gas_pump\", display_name: \"base.structure.gas_pump\", size: (width: 1, height: 2))",
        )
        .expect("write structure");
        fs::write(
            mods_root.join("base/content/gases/oxygen.ron"),
            "(id: \"base:gas/oxygen\", display_name: \"base.gas.oxygen\", molar_mass: 31.998)",
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
    fn reports_duplicate_id_with_file_paths() {
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
            mods_root.join("base/content/substances/a.ron"),
            "(id: \"base:material/oxygen\", display_name: \"base.substance.oxygen\")",
        )
        .expect("write file");
        fs::write(
            mods_root.join("base/content/substances/b.ron"),
            "(id: \"base:material/oxygen\", display_name: \"base.substance.oxygen_duplicate\")",
        )
        .expect("write file");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report = load_content_registry(
            &report.valid_mods,
            &report.resolved_order.expect("resolved order"),
        );

        assert!(load_report.registry.is_none());
        assert!(load_report.errors.iter().any(|error| {
            matches!(error, ContentRegistryError::DuplicatePrototypeId { prototype_id, existing_file, duplicate_file, .. }
                if prototype_id.as_ref() == "base:material/oxygen" && existing_file.ends_with("a.ron") && duplicate_file.ends_with("b.ron"))
        }));
    }

    #[test]
    fn reports_invalid_structure_size() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/structures")).expect("create dir");
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
            mods_root.join("base/content/structures/invalid.ron"),
            "(id: \"base:building/gas_pump\", display_name: \"base.structure.gas_pump\", size: (width: 0, height: 1))",
        )
        .expect("write file");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report = load_content_registry(
            &report.valid_mods,
            &report.resolved_order.expect("resolved order"),
        );

        assert!(load_report.registry.is_none());
        assert!(load_report.errors.iter().any(|error| {
            matches!(error, ContentRegistryError::InvalidPrototypeField { field, .. } if field.as_ref() == "size.width")
        }));
    }

    #[test]
    fn reports_invalid_ron_with_file_path() {
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
            "(id: \"base:material/oxygen\", display_name: )",
        )
        .expect("write file");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report = load_content_registry(
            &report.valid_mods,
            &report.resolved_order.expect("resolved order"),
        );

        assert!(load_report.registry.is_none());
        assert!(load_report.errors.iter().any(|error| {
            matches!(error, ContentRegistryError::FileParse { file, prototype_kind, .. }
                if file.ends_with("broken.ron") && prototype_kind.as_ref() == "substance")
        }));
    }

    #[test]
    fn ignores_ron_files_outside_type_directories() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/substances")).expect("create dir");
        fs::create_dir_all(mods_root.join("base/content/misc")).expect("create dir");
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
            mods_root.join("base/content/substances/oxygen.ron"),
            "(id: \"base:material/oxygen\", display_name: \"base.substance.oxygen\")",
        )
        .expect("write file");
        fs::write(
            mods_root.join("base/content/misc/ignored.ron"),
            "(id: \"base:material/ignored\", display_name: \"base.substance.ignored\")",
        )
        .expect("write file");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report = load_content_registry(
            &report.valid_mods,
            &report.resolved_order.expect("resolved order"),
        );
        let registry = load_report.registry.expect("registry");

        assert!(load_report.errors.is_empty());
        assert_eq!(registry.substances_len(), 1);
    }

    #[test]
    fn file_order_is_lexicographic() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mods_root = temp_dir.path().join("mods");
        fs::create_dir_all(mods_root.join("base/content/substances/a")).expect("create dir");
        fs::create_dir_all(mods_root.join("base/content/substances/b")).expect("create dir");
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
            mods_root.join("base/content/substances/b/002.ron"),
            "(id: \"base:material/zeta\", display_name: \"base.substance.zeta\")",
        )
        .expect("write file");
        fs::write(
            mods_root.join("base/content/substances/a/001.ron"),
            "(id: \"base:material/alpha\", display_name: \"base.substance.alpha\")",
        )
        .expect("write file");

        let report = discover_and_resolve_mods(&mods_root);
        let load_report = load_content_registry(
            &report.valid_mods,
            &report.resolved_order.expect("resolved order"),
        );
        let registry = load_report.registry.expect("registry");

        let ordered_ids: Vec<&str> = registry
            .substances()
            .map(|record| record.prototype.id.as_str())
            .collect();
        assert_eq!(
            ordered_ids,
            vec!["base:material/alpha", "base:material/zeta"]
        );
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
            "(id: \"base:material/base\", display_name: \"base.substance.base\")",
        )
        .expect("write file");
        fs::write(
            mods_root.join("zeta/content/substances/broken.ron"),
            "(id: \"zeta:material/zeta\", display_name: )",
        )
        .expect("write file");
        fs::write(
            mods_root.join("alpha/content/substances/broken.ron"),
            "(id: \"alpha:material/alpha\", display_name: )",
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
