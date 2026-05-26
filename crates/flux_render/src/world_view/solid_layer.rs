use bevy::prelude::{AssetServer, Commands, Component, Name, Sprite, Transform, Vec2, Vec3};

use super::common::{tile_to_world_center, to_bevy_mod_asset_path};
use super::state::SolidCellSprite;

const SOLID_SPRITE_Z: f32 = -0.10;

#[derive(Debug, Component)]
pub(crate) struct SolidCellSpriteMarker;

pub(crate) fn spawn_solid_layer(
    commands: &mut Commands,
    asset_server: &AssetServer,
    solid_cells: &[SolidCellSprite],
    pitch: f32,
) {
    for cell in solid_cells {
        let mut sprite =
            Sprite::from_image(asset_server.load(to_bevy_mod_asset_path(&cell.image_path)));
        sprite.custom_size = Some(Vec2::splat(pitch));
        let center = tile_to_world_center(cell.tile, pitch);
        commands.spawn((
            SolidCellSpriteMarker,
            sprite,
            Transform::from_translation(Vec3::new(center.x, center.y, SOLID_SPRITE_Z)),
            Name::new(format!("solid_sprite_{}_{}", cell.tile.x, cell.tile.y)),
        ));
    }
}
