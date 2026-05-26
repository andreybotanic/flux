use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::math::Vec2;
use bevy::prelude::{
    App, ButtonInput, Camera, Camera2d, Commands, Component, Entity, GlobalTransform, KeyCode,
    MessageReader, Plugin, Projection, Query, Res, ResMut, Resource, Time, Transform, Update, Vec3,
    Window, With,
};
use bevy::window::PrimaryWindow;
use flux_world::GridSize;

use super::state::WorldRenderState;

#[derive(Debug, Resource, Clone, PartialEq)]
pub struct WorldCameraControlsConfig {
    pub zoom_min: f32,
    pub zoom_max: f32,
    pub zoom_sensitivity: f32,
    pub keyboard_pan_speed: f32,
    pub drag_pan_speed: f32,
    pub fit_margin_fraction: f32,
    pub target_cell_screen_px: f32,
}

impl Default for WorldCameraControlsConfig {
    fn default() -> Self {
        Self {
            zoom_min: 0.25,
            zoom_max: 6.0,
            zoom_sensitivity: 0.15,
            keyboard_pan_speed: 16.0,
            drag_pan_speed: 1.0,
            fit_margin_fraction: 0.06,
            target_cell_screen_px: 100.0,
        }
    }
}

#[derive(Debug, Resource, Default, Clone, PartialEq)]
struct WorldCameraDragState {
    last_cursor_position: Option<Vec2>,
}

#[derive(Debug, Component)]
pub(crate) struct WorldCamera;

pub(crate) struct WorldCameraPlugin;

impl Plugin for WorldCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldCameraControlsConfig>()
            .init_resource::<WorldCameraDragState>()
            .add_systems(
                Update,
                (
                    sync_world_camera_lifecycle,
                    reset_world_camera_view,
                    world_camera_zoom,
                    world_camera_pan_keyboard,
                    world_camera_pan_drag,
                ),
            );
    }
}

fn sync_world_camera_lifecycle(
    mut commands: Commands,
    render_state: Res<WorldRenderState>,
    world_cameras: Query<Entity, With<WorldCamera>>,
) {
    if render_state.is_visible() {
        if world_cameras.is_empty() {
            commands.spawn((
                WorldCamera,
                Camera2d,
                Camera {
                    order: 0,
                    is_active: true,
                    ..Default::default()
                },
                bevy::prelude::Name::new("world_camera"),
            ));
        }
        return;
    }

    for entity in &world_cameras {
        commands.entity(entity).despawn();
    }
}

fn reset_world_camera_view(
    mut render_state: ResMut<WorldRenderState>,
    mut controls: ResMut<WorldCameraControlsConfig>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<WorldCamera>>,
) {
    if !render_state.is_visible() || !render_state.reset_camera_requested() {
        return;
    }

    let Some(size) = render_state.grid_size() else {
        return;
    };
    let Ok((mut transform, mut projection)) = camera_query.single_mut() else {
        return;
    };

    let width = size.width as f32 * render_state.tile_pitch();
    let height = size.height as f32 * render_state.tile_pitch();
    transform.translation = Vec3::new(width * 0.5, height * 0.5, transform.translation.z);
    if let Ok(window) = primary_window.single() {
        let (zoom_min, zoom_max) =
            compute_zoom_bounds(size, render_state.tile_pitch(), window, &controls);
        controls.zoom_min = zoom_min;
        controls.zoom_max = zoom_max;
    }
    if let Projection::Orthographic(orthographic) = projection.as_mut() {
        orthographic.scale = controls.zoom_max;
    }
    render_state.mark_grid_dirty();
    render_state.clear_reset_camera_request();
}

fn world_camera_zoom(
    mut render_state: ResMut<WorldRenderState>,
    config: Res<WorldCameraControlsConfig>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut wheel_events: MessageReader<MouseWheel>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<WorldCamera>>,
) {
    let mut accumulated = 0.0f32;
    for event in wheel_events.read() {
        accumulated += event.y;
    }

    if !render_state.is_visible() || accumulated == 0.0 {
        return;
    }

    let Ok((mut transform, mut projection)) = camera_query.single_mut() else {
        return;
    };
    let Ok(window) = primary_window.single() else {
        return;
    };
    let cursor_position = window.cursor_position();
    let Projection::Orthographic(orthographic) = projection.as_mut() else {
        return;
    };
    let world_before_zoom = cursor_position.map(|cursor| {
        cursor_to_world_position(
            cursor,
            window,
            transform.translation.truncate(),
            orthographic.scale,
        )
    });
    let delta_zoom = -accumulated * config.zoom_sensitivity;
    let multiplier = (1.0 + delta_zoom).max(0.05);
    orthographic.scale = (orthographic.scale * multiplier).clamp(config.zoom_min, config.zoom_max);

    if let (Some(cursor), Some(world_before)) = (cursor_position, world_before_zoom) {
        let world_after = cursor_to_world_position(
            cursor,
            window,
            transform.translation.truncate(),
            orthographic.scale,
        );
        let correction = world_before - world_after;
        let desired = transform.translation.truncate() + correction;
        if let Some(size) = render_state.grid_size() {
            let clamped =
                clamp_camera_center_to_world_bounds(desired, size, render_state.tile_pitch());
            transform.translation.x = clamped.x;
            transform.translation.y = clamped.y;
        } else {
            transform.translation.x = desired.x;
            transform.translation.y = desired.y;
        }
    }

    render_state.mark_grid_dirty();
}

fn world_camera_pan_keyboard(
    render_state: Res<WorldRenderState>,
    config: Res<WorldCameraControlsConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<WorldCamera>>,
) {
    if !render_state.is_visible() {
        return;
    }

    let mut direction = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if direction == Vec2::ZERO {
        return;
    }

    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };
    let movement = direction.normalize_or_zero() * config.keyboard_pan_speed * time.delta_secs();
    let desired = transform.translation.truncate() + movement;
    if let Some(size) = render_state.grid_size() {
        let clamped = clamp_camera_center_to_world_bounds(desired, size, render_state.tile_pitch());
        transform.translation.x = clamped.x;
        transform.translation.y = clamped.y;
    }
}

fn world_camera_pan_drag(
    render_state: Res<WorldRenderState>,
    config: Res<WorldCameraControlsConfig>,
    mouse_buttons: Res<ButtonInput<bevy::prelude::MouseButton>>,
    mut drag_state: ResMut<WorldCameraDragState>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mut camera_query: Query<(&mut Transform, &GlobalTransform, &Camera), With<WorldCamera>>,
) {
    let mut had_motion = false;
    for _ in mouse_motion_events.read() {
        had_motion = true;
    }

    if !render_state.is_visible() || !mouse_buttons.pressed(bevy::prelude::MouseButton::Middle) {
        drag_state.last_cursor_position = None;
        return;
    }

    if !had_motion {
        return;
    }

    let Some(cursor_position) = primary_window
        .single()
        .ok()
        .and_then(Window::cursor_position)
    else {
        return;
    };

    let Ok((mut transform, camera_transform, camera)) = camera_query.single_mut() else {
        drag_state.last_cursor_position = Some(cursor_position);
        return;
    };

    let Some(last_cursor_position) = drag_state.last_cursor_position else {
        drag_state.last_cursor_position = Some(cursor_position);
        return;
    };

    let world_prev = camera
        .viewport_to_world_2d(camera_transform, last_cursor_position)
        .ok();
    let world_curr = camera
        .viewport_to_world_2d(camera_transform, cursor_position)
        .ok();
    if let (Some(world_prev), Some(world_curr)) = (world_prev, world_curr) {
        let correction = (world_prev - world_curr) * config.drag_pan_speed;
        let desired = transform.translation.truncate() + correction;
        if let Some(size) = render_state.grid_size() {
            let clamped =
                clamp_camera_center_to_world_bounds(desired, size, render_state.tile_pitch());
            transform.translation.x = clamped.x;
            transform.translation.y = clamped.y;
        }
    }

    drag_state.last_cursor_position = Some(cursor_position);
}

#[must_use]
fn compute_zoom_bounds(
    size: GridSize,
    tile_pitch: f32,
    window: &Window,
    config: &WorldCameraControlsConfig,
) -> (f32, f32) {
    let pitch = tile_pitch.max(0.001);
    let margin = config.fit_margin_fraction.clamp(0.0, 0.45);
    let usable_width = (window.width() * (1.0 - margin * 2.0)).max(1.0);
    let usable_height = (window.height() * (1.0 - margin * 2.0)).max(1.0);
    let grid_width = size.width as f32 * pitch;
    let grid_height = size.height as f32 * pitch;
    let fit_scale = (grid_width / usable_width)
        .max(grid_height / usable_height)
        .max(0.001);
    let target_scale = (pitch / config.target_cell_screen_px.max(1.0)).max(0.0001);

    let zoom_min = target_scale.min(fit_scale);
    let zoom_max = fit_scale.max(zoom_min);
    (zoom_min, zoom_max)
}

#[must_use]
fn clamp_camera_center_to_world_bounds(
    camera_center: Vec2,
    size: GridSize,
    tile_pitch: f32,
) -> Vec2 {
    let pitch = tile_pitch.max(0.001);
    let max_x = size.width as f32 * pitch;
    let max_y = size.height as f32 * pitch;
    Vec2::new(
        camera_center.x.clamp(0.0, max_x),
        camera_center.y.clamp(0.0, max_y),
    )
}

#[must_use]
fn cursor_to_world_position(
    cursor: Vec2,
    window: &Window,
    camera_world_xy: Vec2,
    orthographic_scale: f32,
) -> Vec2 {
    let centered = Vec2::new(
        cursor.x - window.width() * 0.5,
        window.height() * 0.5 - cursor.y,
    );
    camera_world_xy + centered * orthographic_scale
}

#[cfg(test)]
mod tests {
    use super::{
        WorldCameraControlsConfig, clamp_camera_center_to_world_bounds, compute_zoom_bounds,
    };
    use bevy::math::Vec2;
    use bevy::window::{Window, WindowResolution};
    use flux_world::GridSize;

    #[test]
    fn computes_zoom_bounds_for_fit_and_target_cell_size() {
        let window = Window {
            resolution: WindowResolution::new(1920, 1080),
            ..Default::default()
        };
        let config = WorldCameraControlsConfig::default();
        let (zoom_min, zoom_max) =
            compute_zoom_bounds(GridSize::new(64, 64), 1.0, &window, &config);

        assert!(zoom_max > zoom_min);

        let cell_size_at_max_zoom_in = 1.0 / zoom_min;
        assert!((95.0..=105.0).contains(&cell_size_at_max_zoom_in));

        let visible_width = window.width() * zoom_max;
        let visible_height = window.height() * zoom_max;
        assert!(visible_width >= 64.0);
        assert!(visible_height >= 64.0);
    }

    #[test]
    fn clamps_camera_center_to_world_bounds() {
        let size = GridSize::new(64, 64);
        let pitch = 1.0;

        let inside = clamp_camera_center_to_world_bounds(Vec2::new(16.0, 20.0), size, pitch);
        assert_eq!(inside, Vec2::new(16.0, 20.0));

        let below = clamp_camera_center_to_world_bounds(Vec2::new(-5.0, -1.0), size, pitch);
        assert_eq!(below, Vec2::new(0.0, 0.0));

        let above = clamp_camera_center_to_world_bounds(Vec2::new(100.0, 70.0), size, pitch);
        assert_eq!(above, Vec2::new(64.0, 64.0));
    }
}
