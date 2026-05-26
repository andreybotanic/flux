use bevy::prelude::{Commands, Component, Name, Projection, Sprite, Transform, Vec2, Vec3};
use flux_world::GridSize;

const WORLD_GRID_COLOR: bevy::prelude::Color = bevy::prelude::Color::srgb(0.27, 0.29, 0.33);
const WORLD_GRID_LINE_THICKNESS: f32 = 0.015;
const GRID_Z: f32 = -0.30;

#[derive(Debug, Component)]
pub(crate) struct GridLineSpriteMarker;

#[must_use]
pub(crate) fn compute_grid_line_thickness(tile_pitch: f32, projection: Option<&Projection>) -> f32 {
    let min_pixel_world = projection
        .and_then(|value| match value {
            Projection::Orthographic(orthographic) => Some(orthographic.scale.max(0.001)),
            _ => None,
        })
        .unwrap_or(0.001);
    (tile_pitch * WORLD_GRID_LINE_THICKNESS).max(min_pixel_world)
}

pub(crate) fn spawn_grid_layer(
    commands: &mut Commands,
    grid_size: GridSize,
    pitch: f32,
    line_thickness: f32,
) {
    for (start, end) in grid_line_segments(grid_size, pitch) {
        let (center, size) = if (start.x - end.x).abs() < f32::EPSILON {
            (
                Vec2::new(start.x, (start.y + end.y) * 0.5),
                Vec2::new(line_thickness, (end.y - start.y).abs().max(line_thickness)),
            )
        } else {
            (
                Vec2::new((start.x + end.x) * 0.5, start.y),
                Vec2::new((end.x - start.x).abs().max(line_thickness), line_thickness),
            )
        };
        commands.spawn((
            GridLineSpriteMarker,
            Sprite::from_color(WORLD_GRID_COLOR, size),
            Transform::from_translation(Vec3::new(center.x, center.y, GRID_Z)),
            Name::new("world_grid_line"),
        ));
    }
}

#[must_use]
pub(crate) fn grid_line_segments(size: GridSize, tile_pitch: f32) -> Vec<(Vec2, Vec2)> {
    let pitch = tile_pitch.max(0.001);
    let width = size.width as f32 * pitch;
    let height = size.height as f32 * pitch;
    let mut lines = Vec::with_capacity(size.width as usize + size.height as usize + 2);

    for x in 0..=size.width {
        let world_x = x as f32 * pitch;
        lines.push((Vec2::new(world_x, 0.0), Vec2::new(world_x, height)));
    }
    for y in 0..=size.height {
        let world_y = y as f32 * pitch;
        lines.push((Vec2::new(0.0, world_y), Vec2::new(width, world_y)));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::grid_line_segments;
    use flux_world::GridSize;

    #[test]
    fn generates_expected_line_count_and_bounds_for_grid() {
        let lines = grid_line_segments(GridSize::new(64, 64), 1.0);
        assert_eq!(lines.len(), 130);

        let first_vertical = lines[0];
        assert_eq!(first_vertical.0, bevy::math::Vec2::new(0.0, 0.0));
        assert_eq!(first_vertical.1, bevy::math::Vec2::new(0.0, 64.0));

        let last_vertical = lines[64];
        assert_eq!(last_vertical.0, bevy::math::Vec2::new(64.0, 0.0));
        assert_eq!(last_vertical.1, bevy::math::Vec2::new(64.0, 64.0));

        let first_horizontal = lines[65];
        assert_eq!(first_horizontal.0, bevy::math::Vec2::new(0.0, 0.0));
        assert_eq!(first_horizontal.1, bevy::math::Vec2::new(64.0, 0.0));

        let last_horizontal = lines[129];
        assert_eq!(last_horizontal.0, bevy::math::Vec2::new(0.0, 64.0));
        assert_eq!(last_horizontal.1, bevy::math::Vec2::new(64.0, 64.0));
    }
}
