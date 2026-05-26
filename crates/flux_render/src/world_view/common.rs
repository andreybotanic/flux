use bevy::math::Vec2;
use flux_world::TilePos;

#[must_use]
pub(crate) fn tile_to_world_center(tile: TilePos, tile_pitch: f32) -> Vec2 {
    let pitch = tile_pitch.max(0.001);
    Vec2::new((tile.x as f32 + 0.5) * pitch, (tile.y as f32 + 0.5) * pitch)
}

#[must_use]
pub(crate) fn normalize_bevy_asset_path(image_path: &str) -> String {
    image_path.trim().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{normalize_bevy_asset_path, tile_to_world_center};
    use flux_world::TilePos;

    #[test]
    fn maps_tile_positions_to_world_centers_from_bottom_left() {
        assert_eq!(
            tile_to_world_center(TilePos::new(0, 0), 1.0),
            bevy::math::Vec2::new(0.5, 0.5)
        );
        assert_eq!(
            tile_to_world_center(TilePos::new(1, 0), 1.0),
            bevy::math::Vec2::new(1.5, 0.5)
        );
        assert_eq!(
            tile_to_world_center(TilePos::new(0, 1), 1.0),
            bevy::math::Vec2::new(0.5, 1.5)
        );
    }

    #[test]
    fn normalizes_asset_path_without_forcing_mod_namespace() {
        assert_eq!(
            normalize_bevy_asset_path("mods/base/assets/textures/solid/floor_cell.png"),
            "mods/base/assets/textures/solid/floor_cell.png"
        );
        assert_eq!(
            normalize_bevy_asset_path("textures\\ui\\button.png"),
            "textures/ui/button.png"
        );
        assert_eq!(normalize_bevy_asset_path("sprite.png"), "sprite.png");
    }
}
