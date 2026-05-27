use std::cmp::Ordering;
use std::fs;
use std::path::Path;

use flux_content::ContentRegistry;
use flux_world::{GridSize, ParticleCount, TilePos, WorldGrid};

use crate::error::SaveIoError;
use crate::format::{LayerBlockInfo, SaveManifest, SaveWorldDimensions};

const SAVE_FORMAT_VERSION: u32 = 1;
const REGISTRY_SIGNATURE_PLACEHOLDER: &str = "placeholder";
const LAYER_REGION_FULL: &str = "full";
const LAYER_SOLID: &str = "solid_cells";
const LAYER_GASES: &str = "gases";
const LAYER_STRUCTURES: &str = "structures";
const ENCODING_SOLID_V1: &str = "solid_v1";
const ENCODING_GASES_V1: &str = "gases_v1";
const ENCODING_STRUCTURES_V1: &str = "structures_v1";

#[derive(Debug, Clone)]
pub struct LoadedGameState {
    pub manifest: SaveManifest,
    pub world: WorldGrid,
    pub seed: u64,
    pub tick: u64,
}

pub fn save_game(
    root_dir: &Path,
    save_id: &str,
    world: &WorldGrid,
    seed: u64,
    tick: u64,
) -> Result<SaveManifest, SaveIoError> {
    validate_save_id(save_id)?;

    let save_dir = root_dir.join(save_id);
    fs::create_dir_all(&save_dir).map_err(|error| SaveIoError::CreateSaveDirectory {
        save_id: save_id.to_owned(),
        path: save_dir.to_string_lossy().to_string(),
        reason: error.to_string(),
    })?;

    let (payload, layers) = encode_payload(save_id, world)?;
    let payload_file = save_dir.join("payload.bin");
    fs::write(&payload_file, &payload).map_err(|error| SaveIoError::WriteSaveFile {
        save_id: save_id.to_owned(),
        file: payload_file.to_string_lossy().to_string(),
        reason: error.to_string(),
    })?;

    let size = world.size();
    let manifest = SaveManifest {
        format_version: SAVE_FORMAT_VERSION,
        save_id: save_id.to_owned(),
        world_dimensions: SaveWorldDimensions {
            width: size.width,
            height: size.height,
        },
        seed,
        tick,
        registry_signature_placeholder: REGISTRY_SIGNATURE_PLACEHOLDER.to_owned(),
        layers,
    };
    let manifest_file = save_dir.join("manifest.json");
    let manifest_body =
        serde_json::to_vec_pretty(&manifest).map_err(|error| SaveIoError::InvalidManifest {
            save_id: save_id.to_owned(),
            field: "manifest".to_owned(),
            reason: error.to_string(),
        })?;
    fs::write(&manifest_file, manifest_body).map_err(|error| SaveIoError::WriteSaveFile {
        save_id: save_id.to_owned(),
        file: manifest_file.to_string_lossy().to_string(),
        reason: error.to_string(),
    })?;

    Ok(manifest)
}

pub fn load_game(
    root_dir: &Path,
    save_id: &str,
    content_registry: &ContentRegistry,
) -> Result<LoadedGameState, SaveIoError> {
    validate_save_id(save_id)?;

    let save_dir = root_dir.join(save_id);
    let manifest_file = save_dir.join("manifest.json");
    let manifest_body =
        fs::read_to_string(&manifest_file).map_err(|error| SaveIoError::ReadSaveFile {
            save_id: save_id.to_owned(),
            file: manifest_file.to_string_lossy().to_string(),
            reason: error.to_string(),
        })?;
    let manifest: SaveManifest =
        serde_json::from_str(&manifest_body).map_err(|error| SaveIoError::ParseManifest {
            save_id: save_id.to_owned(),
            file: manifest_file.to_string_lossy().to_string(),
            reason: error.to_string(),
        })?;
    validate_manifest(save_id, &manifest)?;

    let payload_file = save_dir.join("payload.bin");
    let payload = fs::read(&payload_file).map_err(|error| SaveIoError::ReadSaveFile {
        save_id: save_id.to_owned(),
        file: payload_file.to_string_lossy().to_string(),
        reason: error.to_string(),
    })?;

    let size = GridSize::new(
        manifest.world_dimensions.width,
        manifest.world_dimensions.height,
    );
    let mut world = WorldGrid::new(size).map_err(|error| SaveIoError::RestoreWorld {
        save_id: save_id.to_owned(),
        reason: error.to_string(),
    })?;

    let solid_block = payload_block(save_id, &payload, &manifest, LAYER_SOLID)?;
    decode_solid_layer(save_id, solid_block, &mut world)?;

    let gas_block = payload_block(save_id, &payload, &manifest, LAYER_GASES)?;
    decode_gas_layer(save_id, gas_block, &mut world)?;

    world.refresh_structure_sizes_from_registry(content_registry);
    let structures_block = payload_block(save_id, &payload, &manifest, LAYER_STRUCTURES)?;
    decode_structures_layer(save_id, structures_block, &mut world)?;
    let seed = manifest.seed;
    let tick = manifest.tick;

    Ok(LoadedGameState {
        manifest,
        world,
        seed,
        tick,
    })
}

fn validate_save_id(save_id: &str) -> Result<(), SaveIoError> {
    if save_id.is_empty() {
        return Err(SaveIoError::InvalidSaveId {
            save_id: save_id.to_owned(),
            reason: "save id must not be empty".to_owned(),
        });
    }
    if save_id.len() > 64 {
        return Err(SaveIoError::InvalidSaveId {
            save_id: save_id.to_owned(),
            reason: "save id must be at most 64 characters".to_owned(),
        });
    }
    if !save_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err(SaveIoError::InvalidSaveId {
            save_id: save_id.to_owned(),
            reason: "save id allows only ASCII letters, digits, `_`, `-`".to_owned(),
        });
    }
    Ok(())
}

fn validate_manifest(save_id: &str, manifest: &SaveManifest) -> Result<(), SaveIoError> {
    if manifest.format_version != SAVE_FORMAT_VERSION {
        return Err(SaveIoError::InvalidManifest {
            save_id: save_id.to_owned(),
            field: "format_version".to_owned(),
            reason: format!(
                "unsupported format version {}, expected {}",
                manifest.format_version, SAVE_FORMAT_VERSION
            ),
        });
    }
    if manifest.save_id != save_id {
        return Err(SaveIoError::InvalidManifest {
            save_id: save_id.to_owned(),
            field: "save_id".to_owned(),
            reason: format!(
                "manifest save id `{}` does not match requested `{save_id}`",
                manifest.save_id
            ),
        });
    }
    if manifest.world_dimensions.width == 0 || manifest.world_dimensions.height == 0 {
        return Err(SaveIoError::InvalidManifest {
            save_id: save_id.to_owned(),
            field: "world_dimensions".to_owned(),
            reason: "world dimensions must be greater than zero".to_owned(),
        });
    }
    ensure_layer_present(save_id, manifest, LAYER_SOLID)?;
    ensure_layer_present(save_id, manifest, LAYER_GASES)?;
    ensure_layer_present(save_id, manifest, LAYER_STRUCTURES)?;
    Ok(())
}

fn ensure_layer_present(
    save_id: &str,
    manifest: &SaveManifest,
    layer: &str,
) -> Result<(), SaveIoError> {
    if manifest.layers.iter().any(|entry| entry.layer == layer) {
        return Ok(());
    }
    Err(SaveIoError::InvalidManifest {
        save_id: save_id.to_owned(),
        field: "layers".to_owned(),
        reason: format!("required layer `{layer}` is missing"),
    })
}

fn payload_block<'a>(
    save_id: &str,
    payload: &'a [u8],
    manifest: &SaveManifest,
    layer: &str,
) -> Result<&'a [u8], SaveIoError> {
    let info = manifest
        .layers
        .iter()
        .find(|entry| entry.layer == layer)
        .ok_or_else(|| SaveIoError::InvalidManifest {
            save_id: save_id.to_owned(),
            field: "layers".to_owned(),
            reason: format!("layer `{layer}` metadata is missing"),
        })?;

    let offset = usize::try_from(info.offset).map_err(|_| SaveIoError::InvalidManifest {
        save_id: save_id.to_owned(),
        field: "layers.offset".to_owned(),
        reason: format!("layer `{layer}` offset is out of range"),
    })?;
    let length = usize::try_from(info.length).map_err(|_| SaveIoError::InvalidManifest {
        save_id: save_id.to_owned(),
        field: "layers.length".to_owned(),
        reason: format!("layer `{layer}` length is out of range"),
    })?;
    let end = offset
        .checked_add(length)
        .ok_or_else(|| SaveIoError::InvalidManifest {
            save_id: save_id.to_owned(),
            field: "layers".to_owned(),
            reason: format!("layer `{layer}` offset+length overflow"),
        })?;
    if end > payload.len() {
        return Err(SaveIoError::InvalidManifest {
            save_id: save_id.to_owned(),
            field: "layers".to_owned(),
            reason: format!(
                "layer `{layer}` points outside payload (end={end}, payload={})",
                payload.len()
            ),
        });
    }
    Ok(&payload[offset..end])
}

fn encode_payload(
    save_id: &str,
    world: &WorldGrid,
) -> Result<(Vec<u8>, Vec<LayerBlockInfo>), SaveIoError> {
    let mut payload = Vec::new();
    let mut layers = Vec::with_capacity(3);

    append_layer(
        save_id,
        LAYER_SOLID,
        ENCODING_SOLID_V1,
        &mut payload,
        &mut layers,
        |buffer| encode_solid_layer(save_id, world, buffer),
    )?;
    append_layer(
        save_id,
        LAYER_GASES,
        ENCODING_GASES_V1,
        &mut payload,
        &mut layers,
        |buffer| encode_gas_layer(save_id, world, buffer),
    )?;
    append_layer(
        save_id,
        LAYER_STRUCTURES,
        ENCODING_STRUCTURES_V1,
        &mut payload,
        &mut layers,
        |buffer| encode_structures_layer(save_id, world, buffer),
    )?;

    Ok((payload, layers))
}

fn append_layer(
    save_id: &str,
    layer_name: &str,
    encoding: &str,
    payload: &mut Vec<u8>,
    layers: &mut Vec<LayerBlockInfo>,
    encoder: impl FnOnce(&mut Vec<u8>) -> Result<(), SaveIoError>,
) -> Result<(), SaveIoError> {
    let offset = u64::try_from(payload.len()).map_err(|_| SaveIoError::EncodePayload {
        save_id: save_id.to_owned(),
        layer: layer_name.to_owned(),
        reason: "payload size exceeds u64".to_owned(),
    })?;
    let mut block = Vec::new();
    encoder(&mut block)?;
    let length = u64::try_from(block.len()).map_err(|_| SaveIoError::EncodePayload {
        save_id: save_id.to_owned(),
        layer: layer_name.to_owned(),
        reason: "layer size exceeds u64".to_owned(),
    })?;
    payload.extend_from_slice(&block);
    layers.push(LayerBlockInfo {
        layer: layer_name.to_owned(),
        region: LAYER_REGION_FULL.to_owned(),
        offset,
        length,
        encoding: encoding.to_owned(),
    });
    Ok(())
}

fn encode_solid_layer(
    save_id: &str,
    world: &WorldGrid,
    buffer: &mut Vec<u8>,
) -> Result<(), SaveIoError> {
    let size = world.size();
    for y in 0..size.height {
        for x in 0..size.width {
            let pos = TilePos::new(x, y);
            let cell = world
                .solid_cell_at(pos)
                .ok_or_else(|| SaveIoError::EncodePayload {
                    save_id: save_id.to_owned(),
                    layer: LAYER_SOLID.to_owned(),
                    reason: format!("cell ({x},{y}) is out of bounds"),
                })?;
            match cell {
                Some(solid) => {
                    buffer.push(1);
                    write_string(save_id, LAYER_SOLID, buffer, solid.as_str())?;
                }
                None => buffer.push(0),
            }
        }
    }
    Ok(())
}

fn encode_gas_layer(
    save_id: &str,
    world: &WorldGrid,
    buffer: &mut Vec<u8>,
) -> Result<(), SaveIoError> {
    let size = world.size();
    for y in 0..size.height {
        for x in 0..size.width {
            let pos = TilePos::new(x, y);
            let mixture = world
                .gas_at(pos)
                .ok_or_else(|| SaveIoError::EncodePayload {
                    save_id: save_id.to_owned(),
                    layer: LAYER_GASES.to_owned(),
                    reason: format!("cell ({x},{y}) is out of bounds"),
                })?;
            let components = mixture.components();
            let count =
                u16::try_from(components.len()).map_err(|_| SaveIoError::EncodePayload {
                    save_id: save_id.to_owned(),
                    layer: LAYER_GASES.to_owned(),
                    reason: format!("too many gas components in cell ({x},{y})"),
                })?;
            write_u16(buffer, count);
            for component in components {
                write_string(save_id, LAYER_GASES, buffer, component.gas.as_str())?;
                write_u64(buffer, component.particles.0);
            }
        }
    }
    Ok(())
}

fn encode_structures_layer(
    save_id: &str,
    world: &WorldGrid,
    buffer: &mut Vec<u8>,
) -> Result<(), SaveIoError> {
    let mut structures = world
        .structures()
        .instances
        .values()
        .cloned()
        .collect::<Vec<_>>();
    structures.sort_by(compare_structures_for_save);
    let count = u32::try_from(structures.len()).map_err(|_| SaveIoError::EncodePayload {
        save_id: save_id.to_owned(),
        layer: LAYER_STRUCTURES.to_owned(),
        reason: "too many structures".to_owned(),
    })?;
    write_u32(buffer, count);
    for structure in structures {
        write_string(
            save_id,
            LAYER_STRUCTURES,
            buffer,
            structure.prototype.as_str(),
        )?;
        write_u32(buffer, structure.origin.x);
        write_u32(buffer, structure.origin.y);
        write_u16(buffer, structure.size.width);
        write_u16(buffer, structure.size.height);
    }
    Ok(())
}

fn compare_structures_for_save(
    left: &flux_world::StructureInstance,
    right: &flux_world::StructureInstance,
) -> Ordering {
    left.origin
        .y
        .cmp(&right.origin.y)
        .then_with(|| left.origin.x.cmp(&right.origin.x))
        .then_with(|| left.prototype.as_str().cmp(right.prototype.as_str()))
        .then_with(|| left.size.width.cmp(&right.size.width))
        .then_with(|| left.size.height.cmp(&right.size.height))
}

fn decode_solid_layer(
    save_id: &str,
    bytes: &[u8],
    world: &mut WorldGrid,
) -> Result<(), SaveIoError> {
    let size = world.size();
    let mut cursor = 0usize;
    for y in 0..size.height {
        for x in 0..size.width {
            let flag = read_u8(save_id, LAYER_SOLID, bytes, &mut cursor)?;
            let solid = match flag {
                0 => None,
                1 => {
                    let value = read_string(save_id, LAYER_SOLID, bytes, &mut cursor)?;
                    Some(
                        flux_world::SolidCellPrototypeId::parse(&value).map_err(|error| {
                            SaveIoError::DecodePayload {
                                save_id: save_id.to_owned(),
                                layer: LAYER_SOLID.to_owned(),
                                reason: format!(
                                    "invalid solid id `{value}` for cell ({x},{y}): {error}"
                                ),
                            }
                        })?,
                    )
                }
                other => {
                    return Err(SaveIoError::DecodePayload {
                        save_id: save_id.to_owned(),
                        layer: LAYER_SOLID.to_owned(),
                        reason: format!("invalid solid flag {other} for cell ({x},{y})"),
                    });
                }
            };
            world
                .set_solid_cell_at(TilePos::new(x, y), solid)
                .map_err(|error| SaveIoError::RestoreWorld {
                    save_id: save_id.to_owned(),
                    reason: error.to_string(),
                })?;
        }
    }
    if cursor != bytes.len() {
        return Err(SaveIoError::DecodePayload {
            save_id: save_id.to_owned(),
            layer: LAYER_SOLID.to_owned(),
            reason: format!(
                "solid layer has trailing bytes: consumed={cursor} total={}",
                bytes.len()
            ),
        });
    }
    Ok(())
}

fn decode_gas_layer(save_id: &str, bytes: &[u8], world: &mut WorldGrid) -> Result<(), SaveIoError> {
    let size = world.size();
    let mut cursor = 0usize;
    for y in 0..size.height {
        for x in 0..size.width {
            let count = read_u16(save_id, LAYER_GASES, bytes, &mut cursor)?;
            for _ in 0..count {
                let gas_id = read_string(save_id, LAYER_GASES, bytes, &mut cursor)?;
                let gas = flux_world::GasPrototypeId::parse(&gas_id).map_err(|error| {
                    SaveIoError::DecodePayload {
                        save_id: save_id.to_owned(),
                        layer: LAYER_GASES.to_owned(),
                        reason: format!("invalid gas id `{gas_id}`: {error}"),
                    }
                })?;
                let particles = read_u64(save_id, LAYER_GASES, bytes, &mut cursor)?;
                world
                    .set_gas_particles(TilePos::new(x, y), gas, ParticleCount(particles))
                    .map_err(|error| SaveIoError::RestoreWorld {
                        save_id: save_id.to_owned(),
                        reason: error.to_string(),
                    })?;
            }
        }
    }
    if cursor != bytes.len() {
        return Err(SaveIoError::DecodePayload {
            save_id: save_id.to_owned(),
            layer: LAYER_GASES.to_owned(),
            reason: format!(
                "gas layer has trailing bytes: consumed={cursor} total={}",
                bytes.len()
            ),
        });
    }
    Ok(())
}

fn decode_structures_layer(
    save_id: &str,
    bytes: &[u8],
    world: &mut WorldGrid,
) -> Result<(), SaveIoError> {
    let mut cursor = 0usize;
    let count = read_u32(save_id, LAYER_STRUCTURES, bytes, &mut cursor)?;
    for _ in 0..count {
        let prototype_raw = read_string(save_id, LAYER_STRUCTURES, bytes, &mut cursor)?;
        let prototype =
            flux_world::StructurePrototypeId::parse(&prototype_raw).map_err(|error| {
                SaveIoError::DecodePayload {
                    save_id: save_id.to_owned(),
                    layer: LAYER_STRUCTURES.to_owned(),
                    reason: format!("invalid structure prototype id `{prototype_raw}`: {error}"),
                }
            })?;
        let origin = TilePos::new(
            read_u32(save_id, LAYER_STRUCTURES, bytes, &mut cursor)?,
            read_u32(save_id, LAYER_STRUCTURES, bytes, &mut cursor)?,
        );
        let expected_width = read_u16(save_id, LAYER_STRUCTURES, bytes, &mut cursor)?;
        let expected_height = read_u16(save_id, LAYER_STRUCTURES, bytes, &mut cursor)?;
        let instance_id = world
            .place_structure(prototype.clone(), origin)
            .map_err(|error| SaveIoError::RestoreWorld {
                save_id: save_id.to_owned(),
                reason: format!(
                    "cannot place structure `{}` at ({},{}): {error}",
                    prototype, origin.x, origin.y
                ),
            })?;
        let placed =
            world
                .structures()
                .get(instance_id)
                .ok_or_else(|| SaveIoError::RestoreWorld {
                    save_id: save_id.to_owned(),
                    reason: "placed structure instance is missing".to_owned(),
                })?;
        if placed.size.width != expected_width || placed.size.height != expected_height {
            return Err(SaveIoError::RestoreWorld {
                save_id: save_id.to_owned(),
                reason: format!(
                    "structure `{}` size mismatch: expected {}x{}, actual {}x{}",
                    prototype,
                    expected_width,
                    expected_height,
                    placed.size.width,
                    placed.size.height
                ),
            });
        }
    }
    if cursor != bytes.len() {
        return Err(SaveIoError::DecodePayload {
            save_id: save_id.to_owned(),
            layer: LAYER_STRUCTURES.to_owned(),
            reason: format!(
                "structures layer has trailing bytes: consumed={cursor} total={}",
                bytes.len()
            ),
        });
    }
    Ok(())
}

fn write_u16(buffer: &mut Vec<u8>, value: u16) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(buffer: &mut Vec<u8>, value: u32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(buffer: &mut Vec<u8>, value: u64) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_string(
    save_id: &str,
    layer: &str,
    buffer: &mut Vec<u8>,
    value: &str,
) -> Result<(), SaveIoError> {
    let bytes = value.as_bytes();
    let len = u16::try_from(bytes.len()).map_err(|_| SaveIoError::EncodePayload {
        save_id: save_id.to_owned(),
        layer: layer.to_owned(),
        reason: format!("string too long ({})", value.len()),
    })?;
    write_u16(buffer, len);
    buffer.extend_from_slice(bytes);
    Ok(())
}

fn read_u8(
    save_id: &str,
    layer: &str,
    input: &[u8],
    cursor: &mut usize,
) -> Result<u8, SaveIoError> {
    let bytes = read_exact(save_id, layer, input, cursor, 1)?;
    Ok(bytes[0])
}

fn read_u16(
    save_id: &str,
    layer: &str,
    input: &[u8],
    cursor: &mut usize,
) -> Result<u16, SaveIoError> {
    let bytes = read_exact(save_id, layer, input, cursor, 2)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(
    save_id: &str,
    layer: &str,
    input: &[u8],
    cursor: &mut usize,
) -> Result<u32, SaveIoError> {
    let bytes = read_exact(save_id, layer, input, cursor, 4)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(
    save_id: &str,
    layer: &str,
    input: &[u8],
    cursor: &mut usize,
) -> Result<u64, SaveIoError> {
    let bytes = read_exact(save_id, layer, input, cursor, 8)?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn read_string(
    save_id: &str,
    layer: &str,
    input: &[u8],
    cursor: &mut usize,
) -> Result<String, SaveIoError> {
    let len = usize::from(read_u16(save_id, layer, input, cursor)?);
    let bytes = read_exact(save_id, layer, input, cursor, len)?;
    let value = std::str::from_utf8(bytes).map_err(|error| SaveIoError::DecodePayload {
        save_id: save_id.to_owned(),
        layer: layer.to_owned(),
        reason: format!("invalid utf-8 string: {error}"),
    })?;
    Ok(value.to_owned())
}

fn read_exact<'a>(
    save_id: &str,
    layer: &str,
    input: &'a [u8],
    cursor: &mut usize,
    len: usize,
) -> Result<&'a [u8], SaveIoError> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| SaveIoError::DecodePayload {
            save_id: save_id.to_owned(),
            layer: layer.to_owned(),
            reason: "cursor overflow while reading payload".to_owned(),
        })?;
    if end > input.len() {
        return Err(SaveIoError::DecodePayload {
            save_id: save_id.to_owned(),
            layer: layer.to_owned(),
            reason: format!(
                "unexpected end of payload at offset {}, requested {} bytes",
                *cursor, len
            ),
        });
    }
    let slice = &input[*cursor..end];
    *cursor = end;
    Ok(slice)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use flux_content::{
        AssetPath, ContentRegistry, LocalizationKey, SingleSpriteVisual, StructurePrototype,
        TileSize, VisualDefinition, VisualDefinitionKind,
    };
    use flux_world::{GridSize, TilePos, WorldGrid};

    use super::{load_game, save_game};

    fn id(value: &str) -> flux_core::PrototypeId {
        flux_core::PrototypeId::parse(value).expect("id")
    }

    fn key(value: &str) -> LocalizationKey {
        LocalizationKey::parse(value).expect("key")
    }

    fn visual(path: &str) -> VisualDefinition {
        VisualDefinition {
            kind: VisualDefinitionKind::SingleSprite(SingleSpriteVisual {
                image: AssetPath::parse(path).expect("asset path"),
            }),
        }
    }

    fn registry_for_structures() -> ContentRegistry {
        let mut registry = ContentRegistry::new();
        registry
            .add_structure(
                StructurePrototype {
                    id: id("base:building/gas_pump"),
                    display_name: key("$base.structure.gas_pump"),
                    size: TileSize {
                        width: 2,
                        height: 1,
                    },
                    visual: visual("textures/structure/gas_pump.png"),
                },
                flux_content::PrototypeSource {
                    mod_id: "base".to_owned(),
                    file: "mods/base/content/structures/gas_pump.ron".to_owned(),
                },
            )
            .expect("structure");
        registry.freeze();
        registry
    }

    #[test]
    fn save_and_load_round_trip_restores_world_tick_and_seed() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let root = temp_dir.path();
        let registry = registry_for_structures();

        let mut world = WorldGrid::new(GridSize::new(8, 6)).expect("world");
        world
            .set_solid_cell_at(TilePos::new(2, 3), Some(id("base:solid_cell/floor_cell")))
            .expect("solid");
        world
            .set_gas_particles(
                TilePos::new(1, 1),
                id("base:gas/oxygen"),
                flux_world::ParticleCount(42),
            )
            .expect("gas");
        world.refresh_structure_sizes_from_registry(&registry);
        world
            .place_structure(id("base:building/gas_pump"), TilePos::new(3, 2))
            .expect("structure");

        save_game(root, "slot_a", &world, 123, 777).expect("save");
        let loaded = load_game(root, "slot_a", &registry).expect("load");

        assert_eq!(loaded.seed, 123);
        assert_eq!(loaded.tick, 777);
        assert_eq!(loaded.world.size(), GridSize::new(8, 6));
        assert_eq!(
            loaded.world.solid_cell_at(TilePos::new(2, 3)),
            Some(Some(id("base:solid_cell/floor_cell")))
        );
        assert_eq!(
            loaded
                .world
                .gas_at(TilePos::new(1, 1))
                .expect("gas cell")
                .total_particles()
                .0,
            42
        );
        assert_eq!(loaded.world.structures().len(), 1);
        assert!(
            Path::new(root)
                .join("slot_a")
                .join("manifest.json")
                .is_file()
        );
        assert!(Path::new(root).join("slot_a").join("payload.bin").is_file());
    }

    #[test]
    fn rejects_invalid_save_id() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let world = WorldGrid::new(GridSize::new(2, 2)).expect("world");
        let error = save_game(temp_dir.path(), "slot/a", &world, 1, 1).expect_err("must fail");
        assert!(error.to_string().contains("validate_save_id"));
    }
}
