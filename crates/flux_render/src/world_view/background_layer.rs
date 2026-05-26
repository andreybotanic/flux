use bevy::prelude::{Commands, Component, Name, Sprite, Transform, Vec2, Vec3};
use flux_world::GridSize;

const WORLD_BACKGROUND_COLOR: bevy::prelude::Color = bevy::prelude::Color::srgb(0.09, 0.10, 0.12);
const BACKGROUND_Z: f32 = -0.40;

#[derive(Debug, Component)]
pub(crate) struct BackgroundSpriteMarker;

pub(crate) fn spawn_background(commands: &mut Commands, grid_size: GridSize, pitch: f32) {
    let world_width = grid_size.width as f32 * pitch;
    let world_height = grid_size.height as f32 * pitch;
    let world_center = Vec2::new(world_width * 0.5, world_height * 0.5);

    commands.spawn((
        BackgroundSpriteMarker,
        Sprite::from_color(WORLD_BACKGROUND_COLOR, Vec2::new(world_width, world_height)),
        Transform::from_translation(Vec3::new(world_center.x, world_center.y, BACKGROUND_Z)),
        Name::new("world_background"),
    ));
}
