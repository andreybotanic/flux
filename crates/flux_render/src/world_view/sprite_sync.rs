use bevy::prelude::{
    App, AssetServer, Commands, Entity, Or, Plugin, Projection, Query, Res, ResMut, Update, With,
};

use super::background_layer::{BackgroundSpriteMarker, spawn_background};
use super::camera::WorldCamera;
use super::gas_layer::{GasCellSpriteMarker, spawn_gas_layer};
use super::grid_layer::{GridLineSpriteMarker, compute_grid_line_thickness, spawn_grid_layer};
use super::solid_layer::{SolidCellSpriteMarker, spawn_solid_layer};
use super::state::WorldRenderState;
use super::structure_layer::{StructureSpriteMarker, spawn_structure_layer};

type WorldLayerSpriteQuery<'w, 's> = Query<
    'w,
    's,
    Entity,
    Or<(
        With<BackgroundSpriteMarker>,
        With<GridLineSpriteMarker>,
        With<GasCellSpriteMarker>,
        With<SolidCellSpriteMarker>,
        With<StructureSpriteMarker>,
    )>,
>;

type GridLayerSpriteQuery<'w, 's> = Query<'w, 's, Entity, With<GridLineSpriteMarker>>;

pub(crate) struct WorldSpriteSyncPlugin;

impl Plugin for WorldSpriteSyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_sprite_layers);
    }
}

fn sync_sprite_layers(
    mut commands: Commands,
    mut render_state: ResMut<WorldRenderState>,
    asset_server: Res<AssetServer>,
    world_camera_projection: Query<&Projection, With<WorldCamera>>,
    layer_sprites: WorldLayerSpriteQuery<'_, '_>,
    grid_sprites: GridLayerSpriteQuery<'_, '_>,
) {
    if !render_state.is_visible() {
        despawn_entities(&mut commands, &layer_sprites);
        render_state.mark_sprites_clean();
        return;
    }
    if !render_state.sprites_dirty() && !render_state.grid_dirty() {
        return;
    }

    let pitch = render_state.tile_pitch();
    let Some(grid_size) = render_state.grid_size() else {
        render_state.mark_sprites_clean();
        return;
    };
    let line_thickness = compute_grid_line_thickness(pitch, world_camera_projection.single().ok());

    if render_state.sprites_dirty() {
        despawn_entities(&mut commands, &layer_sprites);
        spawn_background(&mut commands, grid_size, pitch);
        spawn_grid_layer(&mut commands, grid_size, pitch, line_thickness);

        {
            let snapshot = render_state.render_snapshot();
            spawn_gas_layer(&mut commands, &snapshot.gas_cells, pitch);
            spawn_solid_layer(&mut commands, &asset_server, &snapshot.solid_cells, pitch);
            spawn_structure_layer(&mut commands, &asset_server, &snapshot.structures, pitch);
        }

        render_state.mark_sprites_clean();
        return;
    }

    if render_state.grid_dirty() {
        despawn_entities(&mut commands, &grid_sprites);
        spawn_grid_layer(&mut commands, grid_size, pitch, line_thickness);
        render_state.mark_grid_clean();
    }
}

fn despawn_entities(
    commands: &mut Commands,
    entities: &Query<Entity, impl bevy::ecs::query::QueryFilter>,
) {
    for entity in entities {
        commands.entity(entity).despawn();
    }
}
