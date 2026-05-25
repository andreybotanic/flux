#![forbid(unsafe_code)]

mod world_view;

pub use world_view::{
    // S11B temporary debug-visualization exports.
    DebugGasCell,
    DebugSolidCell,
    DebugStructureOverlay,
    FluxRenderPlugin,
    WorldCameraControlsConfig,
    WorldDebugSnapshot,
    WorldRenderState,
    gas_intensity_from_particles,
    grid_line_segments,
    tile_to_world_center,
};
