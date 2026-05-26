mod background_layer;
mod camera;
mod common;
mod gas_layer;
mod grid_layer;
mod solid_layer;
mod sprite_sync;
mod state;
mod structure_layer;

use bevy::prelude::{App, Plugin};

pub use camera::WorldCameraControlsConfig;
pub use state::{
    DebugGasCell, SolidCellSprite, StructureSprite, WorldRenderSnapshot, WorldRenderState,
};

pub struct FluxRenderPlugin;

impl Plugin for FluxRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            state::WorldRenderStatePlugin,
            camera::WorldCameraPlugin,
            sprite_sync::WorldSpriteSyncPlugin,
        ));
    }
}
