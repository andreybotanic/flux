use bevy::prelude::Color;
use flux_content::ContentRegistry;
use flux_core::PrototypeId;
use flux_render::{DebugGasCell, SolidCellSprite, StructureSprite, WorldRenderSnapshot};
use flux_world::{ParticleCount, StructurePlacementError, TilePos, WorldGrid, WorldGridError};

// S11B temporary debug-only world seeding/snapshot module.
// This module is expected to be replaced once the production world visualization pipeline appears.

#[allow(dead_code)]
const TEMP_GAS_PARTICLES_LOW: u64 = 35;
#[allow(dead_code)]
const TEMP_GAS_PARTICLES_MEDIUM: u64 = 180;
#[allow(dead_code)]
const TEMP_GAS_PARTICLES_HIGH: u64 = 640;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum WorldDebugPopulateError {
    MissingSolidPrototype,
    MissingGasPrototype,
    MissingStructurePrototype,
    SetSolidFailed {
        pos: TilePos,
        source: WorldGridError,
    },
    SetGasFailed {
        pos: TilePos,
        gas: PrototypeId,
        source: WorldGridError,
    },
    PlaceStructureFailed {
        prototype: PrototypeId,
        origin: TilePos,
        source: StructurePlacementError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorldRenderSnapshotError {
    MissingSolidPrototypeInRegistry { prototype: PrototypeId },
    MissingStructurePrototypeInRegistry { prototype: PrototypeId },
}

impl std::fmt::Display for WorldDebugPopulateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSolidPrototype => write!(
                f,
                "WorldDebugPopulateError:\n  layer: solid\n  reason: content registry has no solid cell prototypes"
            ),
            Self::MissingGasPrototype => write!(
                f,
                "WorldDebugPopulateError:\n  layer: gas\n  reason: content registry has no gas prototypes"
            ),
            Self::MissingStructurePrototype => write!(
                f,
                "WorldDebugPopulateError:\n  layer: structures\n  reason: content registry has no structure prototypes"
            ),
            Self::SetSolidFailed { pos, source } => write!(
                f,
                "WorldDebugPopulateError:\n  layer: solid\n  pos: ({},{})\n  reason: {}",
                pos.x, pos.y, source
            ),
            Self::SetGasFailed { pos, gas, source } => write!(
                f,
                "WorldDebugPopulateError:\n  layer: gas\n  gas: {}\n  pos: ({},{})\n  reason: {}",
                gas, pos.x, pos.y, source
            ),
            Self::PlaceStructureFailed {
                prototype,
                origin,
                source,
            } => write!(
                f,
                "WorldDebugPopulateError:\n  layer: structures\n  prototype: {}\n  origin: ({},{})\n  reason: {}",
                prototype, origin.x, origin.y, source
            ),
        }
    }
}

impl std::fmt::Display for WorldRenderSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSolidPrototypeInRegistry { prototype } => write!(
                f,
                "WorldRenderSnapshotError:\n  layer: solid\n  prototype: {}\n  reason: prototype is missing in content registry",
                prototype
            ),
            Self::MissingStructurePrototypeInRegistry { prototype } => write!(
                f,
                "WorldRenderSnapshotError:\n  layer: structure\n  prototype: {}\n  reason: prototype is missing in content registry",
                prototype
            ),
        }
    }
}

impl std::error::Error for WorldRenderSnapshotError {}

#[allow(dead_code)]
pub(crate) fn populate_world_debug_mvp(
    world: &mut WorldGrid,
    registry: &ContentRegistry,
) -> Result<(), WorldDebugPopulateError> {
    let solid_ids = registry
        .solid_cells()
        .map(|record| record.prototype.id.clone())
        .take(2)
        .collect::<Vec<_>>();
    if solid_ids.is_empty() {
        return Err(WorldDebugPopulateError::MissingSolidPrototype);
    }
    let primary_solid = solid_ids.first().expect("checked non-empty").clone();

    let gas_ids = registry
        .gases()
        .map(|record| record.prototype.id.clone())
        .take(3)
        .collect::<Vec<_>>();
    if gas_ids.is_empty() {
        return Err(WorldDebugPopulateError::MissingGasPrototype);
    }

    let structure_ids = registry
        .structures()
        .map(|record| record.prototype.id.clone())
        .take(2)
        .collect::<Vec<_>>();
    if structure_ids.is_empty() {
        return Err(WorldDebugPopulateError::MissingStructurePrototype);
    }

    for x in 2..18 {
        let pos = TilePos::new(x, 3);
        let solid_id = if x % 2 == 0 {
            primary_solid.clone()
        } else {
            solid_ids
                .get(1)
                .cloned()
                .unwrap_or_else(|| primary_solid.clone())
        };
        world
            .set_solid_cell_at(pos, Some(solid_id))
            .map_err(|source| WorldDebugPopulateError::SetSolidFailed { pos, source })?;
    }
    for y in 4..11 {
        let pos = TilePos::new(10, y);
        world
            .set_solid_cell_at(pos, Some(primary_solid.clone()))
            .map_err(|source| WorldDebugPopulateError::SetSolidFailed { pos, source })?;
    }

    let gas_positions = [
        (TilePos::new(20, 8), TEMP_GAS_PARTICLES_LOW),
        (TilePos::new(22, 8), TEMP_GAS_PARTICLES_MEDIUM),
        (TilePos::new(24, 8), TEMP_GAS_PARTICLES_HIGH),
    ];
    for (index, (pos, particles)) in gas_positions.iter().copied().enumerate() {
        let gas = gas_ids[index % gas_ids.len()].clone();
        world
            .set_gas_particles(pos, gas.clone(), ParticleCount(particles))
            .map_err(|source| WorldDebugPopulateError::SetGasFailed { pos, gas, source })?;
    }

    world.refresh_structure_sizes_from_registry(registry);
    for (index, structure_id) in structure_ids.into_iter().enumerate() {
        let origin = TilePos::new(28 + (index as u32) * 4, 6 + (index as u32) * 3);
        world
            .place_structure(structure_id.clone(), origin)
            .map_err(|source| WorldDebugPopulateError::PlaceStructureFailed {
                prototype: structure_id,
                origin,
                source,
            })?;
    }

    Ok(())
}

pub(crate) fn build_world_render_snapshot(
    world: &WorldGrid,
    registry: &ContentRegistry,
) -> Result<WorldRenderSnapshot, WorldRenderSnapshotError> {
    let mut snapshot = WorldRenderSnapshot::default();
    let size = world.size();

    for y in 0..size.height {
        for x in 0..size.width {
            let pos = TilePos::new(x, y);

            if let Some(Some(solid)) = world.solid_cell_at(pos) {
                let record = registry.solid_cell(&solid).ok_or_else(|| {
                    WorldRenderSnapshotError::MissingSolidPrototypeInRegistry {
                        prototype: solid.clone(),
                    }
                })?;
                let visual_mod_id = registry
                    .solid_cell_visual_mod_id(&solid)
                    .unwrap_or(record.source.mod_id.as_str());
                snapshot.solid_cells.push(SolidCellSprite {
                    tile: pos,
                    image_path: format!(
                        "mods/{}/assets/{}",
                        visual_mod_id,
                        record.prototype.visual.image_path().as_str()
                    ),
                });
            }

            if let Some(mixture) = world.gas_at(pos) {
                let total_particles = mixture.total_particles().0;
                if total_particles > 0 {
                    let gas_color_key = mixture
                        .components()
                        .iter()
                        .max_by_key(|component| component.particles.0)
                        .map(|component| component.gas.as_str())
                        .unwrap_or("base:gas/unknown");
                    snapshot.gas_cells.push(DebugGasCell {
                        tile: pos,
                        base_color: stable_debug_color(gas_color_key, 0.72, 0.53),
                        total_particles,
                    });
                }
            }
        }
    }

    for structure in world.structures().instances.values() {
        let record = registry.structure(&structure.prototype).ok_or_else(|| {
            WorldRenderSnapshotError::MissingStructurePrototypeInRegistry {
                prototype: structure.prototype.clone(),
            }
        })?;
        let visual_mod_id = registry
            .structure_visual_mod_id(&structure.prototype)
            .unwrap_or(record.source.mod_id.as_str());
        snapshot.structures.push(StructureSprite {
            origin: structure.origin,
            width: structure.size.width,
            height: structure.size.height,
            image_path: format!(
                "mods/{}/assets/{}",
                visual_mod_id,
                record.prototype.visual.image_path().as_str()
            ),
        });
    }

    Ok(snapshot)
}

#[must_use]
fn stable_debug_color(key: &str, saturation: f32, lightness: f32) -> Color {
    let hue = stable_hue_from_key(key);
    Color::hsl(hue, saturation.clamp(0.0, 1.0), lightness.clamp(0.0, 1.0))
}

#[must_use]
fn stable_hue_from_key(key: &str) -> f32 {
    let mut hash: u32 = 2_166_136_261;
    for byte in key.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    (hash % 360) as f32
}

#[cfg(test)]
mod tests {
    use flux_content::{
        AssetPath, ContentRegistry, LocalizationKey, PrototypePatch, PrototypePatchBody,
        PrototypeSource, SingleSpriteVisual, SolidCellPrototype, StructurePrototype,
        StructurePrototypePatch, TileSize, VisualDefinition, VisualDefinitionKind,
    };
    use flux_core::PrototypeId;
    use flux_world::{GridSize, TilePos, WorldGrid};

    use super::{build_world_render_snapshot, populate_world_debug_mvp};

    fn source() -> PrototypeSource {
        PrototypeSource {
            mod_id: "base".to_owned(),
            file: "mods/base/content/test.ron".to_owned(),
        }
    }

    fn patch_source() -> PrototypeSource {
        PrototypeSource {
            mod_id: "test_content_mod".to_owned(),
            file: "mods/test_content_mod/content/patches/base_building_gas_pump.ron".to_owned(),
        }
    }

    fn key(value: &str) -> LocalizationKey {
        LocalizationKey::parse(value).expect("valid localization key")
    }

    fn id(value: &str) -> PrototypeId {
        PrototypeId::parse(value).expect("valid prototype id")
    }

    fn visual(path: &str) -> VisualDefinition {
        VisualDefinition {
            kind: VisualDefinitionKind::SingleSprite(SingleSpriteVisual {
                image: AssetPath::parse(path).expect("valid asset path"),
            }),
        }
    }

    fn registry_with_full_debug_content_unfrozen() -> ContentRegistry {
        let mut registry = ContentRegistry::new();
        registry
            .add_solid_cell(
                SolidCellPrototype {
                    id: id("base:solid_cell/floor_cell"),
                    display_name: key("base.solid.floor_cell"),
                    gas_permeable: false,
                    visual: visual("textures/solid/floor_cell.png"),
                },
                source(),
            )
            .expect("solid prototype should be accepted");
        registry
            .add_gas(
                flux_content::GasPrototype {
                    id: id("base:gas/oxygen"),
                    display_name: key("base.gas.oxygen"),
                    molar_mass: 31.998,
                },
                source(),
            )
            .expect("gas prototype should be accepted");
        registry
            .add_gas(
                flux_content::GasPrototype {
                    id: id("base:gas/hydrogen"),
                    display_name: key("base.gas.hydrogen"),
                    molar_mass: 2.016,
                },
                source(),
            )
            .expect("gas prototype should be accepted");
        registry
            .add_structure(
                StructurePrototype {
                    id: id("base:building/gas_pump"),
                    display_name: key("base.structure.gas_pump"),
                    size: TileSize {
                        width: 2,
                        height: 1,
                    },
                    visual: visual("textures/structure/gas_pump.png"),
                },
                source(),
            )
            .expect("structure prototype should be accepted");
        registry
            .add_structure(
                StructurePrototype {
                    id: id("base:building/vent"),
                    display_name: key("base.structure.vent"),
                    size: TileSize {
                        width: 1,
                        height: 1,
                    },
                    visual: visual("textures/structure/vent.png"),
                },
                source(),
            )
            .expect("structure prototype should be accepted");
        registry
    }

    fn registry_with_full_debug_content() -> ContentRegistry {
        let mut registry = registry_with_full_debug_content_unfrozen();
        registry.freeze();
        registry
    }

    #[test]
    fn temporary_population_fills_all_debug_layers_when_content_exists() {
        let registry = registry_with_full_debug_content();
        let mut world = WorldGrid::new(GridSize::new(64, 64)).expect("world should be created");

        populate_world_debug_mvp(&mut world, &registry).expect("population should succeed");
        let snapshot =
            build_world_render_snapshot(&world, &registry).expect("snapshot build should succeed");

        assert!(!snapshot.solid_cells.is_empty());
        assert!(!snapshot.gas_cells.is_empty());
        assert!(!snapshot.structures.is_empty());
    }

    #[test]
    fn missing_prototypes_return_structured_error_without_panic() {
        let empty_registry = ContentRegistry::new();
        let mut world = WorldGrid::new(GridSize::new(64, 64)).expect("world should be created");

        let error = populate_world_debug_mvp(&mut world, &empty_registry)
            .expect_err("empty registry should be rejected");
        let rendered = error.to_string();

        assert!(rendered.contains("WorldDebugPopulateError"));
        assert!(rendered.contains("layer: solid"));
    }

    #[test]
    fn snapshot_builder_returns_structured_error_for_missing_solid_prototype() {
        let registry = registry_with_full_debug_content();
        let mut world = WorldGrid::new(GridSize::new(4, 4)).expect("world should be created");
        let missing_id = id("base:solid_cell/missing_for_snapshot_test");
        world
            .set_solid_cell_at(TilePos::new(1, 1), Some(missing_id.clone()))
            .expect("set solid");

        let error = build_world_render_snapshot(&world, &registry)
            .expect_err("missing prototype must return error");
        let rendered = error.to_string();

        assert!(rendered.contains("WorldRenderSnapshotError"));
        assert!(rendered.contains("layer: solid"));
        assert!(rendered.contains(missing_id.as_str()));
    }

    #[test]
    fn snapshot_uses_patch_mod_namespace_for_structure_visual() {
        let mut registry = registry_with_full_debug_content_unfrozen();
        let target = id("base:building/gas_pump");
        let patch = PrototypePatch {
            target: target.clone(),
            body: PrototypePatchBody::Structure(StructurePrototypePatch {
                display_name: None,
                size: None,
                visual: Some(visual("textures/structure/patched_from_mod.png")),
            }),
        };
        registry
            .apply_patch(patch, patch_source())
            .expect("patch should be accepted");
        registry.freeze();

        let mut world = WorldGrid::new(GridSize::new(16, 16)).expect("world should be created");
        world.refresh_structure_sizes_from_registry(&registry);
        world
            .place_structure(target, TilePos::new(4, 4))
            .expect("structure placement should succeed");

        let snapshot =
            build_world_render_snapshot(&world, &registry).expect("snapshot build should succeed");
        assert!(snapshot.structures.iter().any(|entry| entry.image_path
            == "mods/test_content_mod/assets/textures/structure/patched_from_mod.png"));
    }
}
