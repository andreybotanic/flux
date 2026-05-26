use bevy::math::Vec2;
use flux_world::TilePos;

#[must_use]
pub(crate) fn tile_to_world_center(tile: TilePos, tile_pitch: f32) -> Vec2 {
    let pitch = tile_pitch.max(0.001);
    Vec2::new((tile.x as f32 + 0.5) * pitch, (tile.y as f32 + 0.5) * pitch)
}

#[must_use]
pub(crate) fn to_bevy_mod_asset_path(image_path: &str) -> String {
    let path = image_path.trim().replace('\\', "/");
    let Some((namespace, rest)) = path.split_once('/') else {
        panic!(
            "WorldSpriteAssetError:\n  asset: {}\n  reason: expected mod-scoped path <mod_id>/<asset_path>",
            image_path
        );
    };
    format!("{namespace}/assets/{rest}")
}

#[cfg(test)]
mod tests {
    use super::tile_to_world_center;
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
}
