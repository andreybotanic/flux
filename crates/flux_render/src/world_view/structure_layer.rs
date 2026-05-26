use bevy::math::Vec2;
use bevy::prelude::{AssetServer, Commands, Component, Name, Sprite, Transform, Vec3};

use super::common::to_bevy_mod_asset_path;
use super::state::StructureSprite;

const STRUCTURE_SPRITE_Z: f32 = 0.0;

#[derive(Debug, Component)]
pub(crate) struct StructureSpriteMarker;

pub(crate) fn spawn_structure_layer(
    commands: &mut Commands,
    asset_server: &AssetServer,
    structures: &[StructureSprite],
    pitch: f32,
) {
    for structure in structures {
        let mut sprite =
            Sprite::from_image(asset_server.load(to_bevy_mod_asset_path(&structure.image_path)));
        let (center, size) = structure_sprite_bounds(structure, pitch);
        sprite.custom_size = Some(size);
        commands.spawn((
            StructureSpriteMarker,
            sprite,
            Transform::from_translation(Vec3::new(center.x, center.y, STRUCTURE_SPRITE_Z)),
            Name::new(format!(
                "structure_sprite_{}_{}",
                structure.origin.x, structure.origin.y
            )),
        ));
    }
}

#[must_use]
fn structure_sprite_bounds(structure: &StructureSprite, pitch: f32) -> (Vec2, Vec2) {
    let center = Vec2::new(
        (structure.origin.x as f32 + f32::from(structure.width) * 0.5) * pitch,
        (structure.origin.y as f32 + f32::from(structure.height) * 0.5) * pitch,
    );
    let size = Vec2::new(
        f32::from(structure.width) * pitch,
        f32::from(structure.height) * pitch,
    );
    (center, size)
}

#[cfg(test)]
mod tests {
    use super::structure_sprite_bounds;
    use crate::world_view::StructureSprite;
    use flux_world::TilePos;

    #[test]
    fn structure_sprite_uses_expected_world_bounds() {
        let pitch = 1.0;
        let structure = StructureSprite {
            origin: TilePos::new(4, 7),
            width: 2,
            height: 3,
            image_path: "textures/structures/test.png".to_owned(),
        };

        let (center, size) = structure_sprite_bounds(&structure, pitch);

        assert_eq!(center, bevy::math::Vec2::new(5.0, 8.5));
        assert_eq!(size, bevy::math::Vec2::new(2.0, 3.0));
    }
}
