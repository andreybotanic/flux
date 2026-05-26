use bevy::prelude::{Color, Commands, Component, Name, Sprite, Transform, Vec2, Vec3};

use super::common::tile_to_world_center;
use super::state::DebugGasCell;

const GAS_Z: f32 = -0.20;

#[derive(Debug, Component)]
pub(crate) struct GasCellSpriteMarker;

pub(crate) fn spawn_gas_layer(commands: &mut Commands, gas_cells: &[DebugGasCell], pitch: f32) {
    for cell in gas_cells {
        let center = tile_to_world_center(cell.tile, pitch);
        let intensity = gas_intensity_from_particles(cell.total_particles);
        let srgb = cell.base_color.to_srgba();
        let color = Color::srgba(srgb.red, srgb.green, srgb.blue, intensity);
        commands.spawn((
            GasCellSpriteMarker,
            Sprite::from_color(color, Vec2::splat(pitch)),
            Transform::from_translation(Vec3::new(center.x, center.y, GAS_Z)),
            Name::new(format!("gas_sprite_{}_{}", cell.tile.x, cell.tile.y)),
        ));
    }
}

#[must_use]
pub(crate) fn gas_intensity_from_particles(total_particles: u64) -> f32 {
    if total_particles == 0 {
        return 0.0;
    }

    let max_particles_for_mvp = 800.0;
    let normalized = (((total_particles as f32).min(max_particles_for_mvp)) + 1.0).ln()
        / (max_particles_for_mvp + 1.0).ln();
    (0.12 + normalized * 0.73).clamp(0.12, 0.85)
}

#[cfg(test)]
mod tests {
    use super::gas_intensity_from_particles;

    #[test]
    fn gas_intensity_is_monotonic_and_deterministic() {
        let points = [0, 1, 10, 100, 500, 1_000, 10_000];
        let values = points
            .iter()
            .map(|value| gas_intensity_from_particles(*value))
            .collect::<Vec<_>>();

        assert_eq!(values[0], 0.0);
        for pair in values.windows(2).skip(1) {
            assert!(pair[1] >= pair[0]);
        }
        assert_eq!(
            gas_intensity_from_particles(500),
            gas_intensity_from_particles(500)
        );
    }
}
