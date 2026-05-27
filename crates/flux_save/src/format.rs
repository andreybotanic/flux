use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveWorldDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerBlockInfo {
    pub layer: String,
    pub region: String,
    pub offset: u64,
    pub length: u64,
    pub encoding: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveManifest {
    pub format_version: u32,
    pub save_id: String,
    pub world_dimensions: SaveWorldDimensions,
    pub seed: u64,
    pub tick: u64,
    pub registry_signature_placeholder: String,
    pub layers: Vec<LayerBlockInfo>,
}
