use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use flux_world::{GridSize, TilePos};
use std::path::Path;

const WORLD_BACKGROUND_COLOR: Color = Color::srgb(0.09, 0.10, 0.12);
const WORLD_GRID_COLOR: Color = Color::srgb(0.27, 0.29, 0.33);
const WORLD_GRID_LINE_THICKNESS: f32 = 0.015;
const BACKGROUND_Z: f32 = -0.40;
const GRID_Z: f32 = -0.30;
const GAS_Z: f32 = -0.20;
const SOLID_SPRITE_Z: f32 = -0.10;
const STRUCTURE_SPRITE_Z: f32 = 0.0;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorldRenderSnapshot {
    pub solid_cells: Vec<SolidCellSprite>,
    pub gas_cells: Vec<DebugGasCell>,
    pub structures: Vec<StructureSprite>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolidCellSprite {
    pub tile: TilePos,
    pub image_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugGasCell {
    // S11B temporary debug primitive.
    pub tile: TilePos,
    pub base_color: Color,
    pub total_particles: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructureSprite {
    pub origin: TilePos,
    pub width: u16,
    pub height: u16,
    pub image_path: String,
}

#[derive(Debug, Resource, Clone, PartialEq)]
pub struct WorldRenderState {
    visible: bool,
    grid_size: Option<GridSize>,
    tile_pitch: f32,
    reset_camera_requested: bool,
    snapshot: WorldRenderSnapshot,
    sprites_dirty: bool,
}

impl Default for WorldRenderState {
    fn default() -> Self {
        Self {
            visible: false,
            grid_size: None,
            tile_pitch: 1.0,
            reset_camera_requested: false,
            snapshot: WorldRenderSnapshot::default(),
            sprites_dirty: false,
        }
    }
}

impl WorldRenderState {
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    #[must_use]
    pub fn grid_size(&self) -> Option<GridSize> {
        self.grid_size
    }

    #[must_use]
    pub fn tile_pitch(&self) -> f32 {
        self.tile_pitch
    }

    #[must_use]
    pub fn render_snapshot(&self) -> &WorldRenderSnapshot {
        &self.snapshot
    }

    pub fn show_world(
        &mut self,
        grid_size: GridSize,
        tile_pitch: f32,
        snapshot: WorldRenderSnapshot,
    ) {
        self.visible = true;
        self.grid_size = Some(grid_size);
        self.tile_pitch = tile_pitch.max(0.001);
        self.reset_camera_requested = true;
        self.snapshot = snapshot;
        self.sprites_dirty = true;
    }

    pub fn set_render_snapshot(&mut self, snapshot: WorldRenderSnapshot) {
        self.snapshot = snapshot;
        self.sprites_dirty = true;
    }

    #[must_use]
    pub fn sprites_dirty(&self) -> bool {
        self.sprites_dirty
    }

    pub fn mark_sprites_clean(&mut self) {
        self.sprites_dirty = false;
    }

    pub fn hide_world(&mut self) {
        self.visible = false;
        self.reset_camera_requested = false;
        self.snapshot = WorldRenderSnapshot::default();
        self.sprites_dirty = true;
    }
}

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
struct WorldCamera;

#[derive(Debug, Component)]
struct SolidCellSpriteMarker;

#[derive(Debug, Component)]
struct StructureSpriteMarker;

#[derive(Debug, Component)]
struct BackgroundSpriteMarker;

#[derive(Debug, Component)]
struct GridLineSpriteMarker;

#[derive(Debug, Component)]
struct GasCellSpriteMarker;

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

pub struct FluxRenderPlugin;

impl Plugin for FluxRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldRenderState>()
            .init_resource::<WorldCameraControlsConfig>()
            .init_resource::<WorldCameraDragState>()
            .add_systems(
                Update,
                (
                    sync_world_camera_lifecycle,
                    reset_world_camera_view,
                    world_camera_zoom,
                    world_camera_pan_keyboard,
                    world_camera_pan_drag,
                    sync_sprite_layers,
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
                Name::new("world_camera"),
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
    if !render_state.visible || !render_state.reset_camera_requested {
        return;
    }

    let Some(size) = render_state.grid_size else {
        return;
    };
    let Ok((mut transform, mut projection)) = camera_query.single_mut() else {
        return;
    };

    let width = size.width as f32 * render_state.tile_pitch;
    let height = size.height as f32 * render_state.tile_pitch;
    transform.translation = Vec3::new(width * 0.5, height * 0.5, transform.translation.z);
    if let Ok(window) = primary_window.single() {
        let (zoom_min, zoom_max) =
            compute_zoom_bounds(size, render_state.tile_pitch, window, &controls);
        controls.zoom_min = zoom_min;
        controls.zoom_max = zoom_max;
    }
    if let Projection::Orthographic(orthographic) = projection.as_mut() {
        orthographic.scale = controls.zoom_max;
    }
    // Ensure grid/gas/tiles are rebuilt using the finalized initial camera scale.
    render_state.sprites_dirty = true;
    render_state.reset_camera_requested = false;
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
        if let Some(size) = render_state.grid_size {
            let clamped =
                clamp_camera_center_to_world_bounds(desired, size, render_state.tile_pitch);
            transform.translation.x = clamped.x;
            transform.translation.y = clamped.y;
        } else {
            transform.translation.x = desired.x;
            transform.translation.y = desired.y;
        }
    }

    // Keep grid sampling stable while zoom changes by forcing a layer rebuild.
    render_state.sprites_dirty = true;
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
    if let Some(size) = render_state.grid_size {
        let clamped = clamp_camera_center_to_world_bounds(desired, size, render_state.tile_pitch);
        transform.translation.x = clamped.x;
        transform.translation.y = clamped.y;
    }
}

fn world_camera_pan_drag(
    render_state: Res<WorldRenderState>,
    config: Res<WorldCameraControlsConfig>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<WorldCameraDragState>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mut camera_query: Query<(&mut Transform, &GlobalTransform, &Camera), With<WorldCamera>>,
) {
    let mut had_motion = false;
    for _ in mouse_motion_events.read() {
        had_motion = true;
    }

    if !render_state.is_visible() || !mouse_buttons.pressed(MouseButton::Middle) {
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
        if let Some(size) = render_state.grid_size {
            let clamped =
                clamp_camera_center_to_world_bounds(desired, size, render_state.tile_pitch);
            transform.translation.x = clamped.x;
            transform.translation.y = clamped.y;
        }
    }

    drag_state.last_cursor_position = Some(cursor_position);
}

fn sync_sprite_layers(
    mut commands: Commands,
    mut render_state: ResMut<WorldRenderState>,
    asset_server: Res<AssetServer>,
    world_camera_projection: Query<&Projection, With<WorldCamera>>,
    layer_sprites: WorldLayerSpriteQuery<'_, '_>,
) {
    if !render_state.is_visible() {
        for entity in &layer_sprites {
            commands.entity(entity).despawn();
        }
        render_state.mark_sprites_clean();
        return;
    }
    if !render_state.sprites_dirty() {
        return;
    }

    for entity in &layer_sprites {
        commands.entity(entity).despawn();
    }

    let pitch = render_state.tile_pitch();
    let Some(grid_size) = render_state.grid_size() else {
        render_state.mark_sprites_clean();
        return;
    };
    let world_width = grid_size.width as f32 * pitch;
    let world_height = grid_size.height as f32 * pitch;
    let world_center = Vec2::new(world_width * 0.5, world_height * 0.5);

    commands.spawn((
        BackgroundSpriteMarker,
        Sprite::from_color(WORLD_BACKGROUND_COLOR, Vec2::new(world_width, world_height)),
        Transform::from_translation(Vec3::new(world_center.x, world_center.y, BACKGROUND_Z)),
        Name::new("world_background"),
    ));

    let min_pixel_world = world_camera_projection
        .single()
        .ok()
        .and_then(|projection| match projection {
            Projection::Orthographic(orthographic) => Some(orthographic.scale.max(0.001)),
            _ => None,
        })
        .unwrap_or(0.001);
    let line_thickness = (pitch * WORLD_GRID_LINE_THICKNESS).max(min_pixel_world);
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

    for cell in &render_state.render_snapshot().gas_cells {
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

    for cell in &render_state.render_snapshot().solid_cells {
        ensure_asset_exists(&cell.image_path);
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

    for structure in &render_state.render_snapshot().structures {
        ensure_asset_exists(&structure.image_path);
        let mut sprite =
            Sprite::from_image(asset_server.load(to_bevy_mod_asset_path(&structure.image_path)));
        sprite.custom_size = Some(Vec2::new(
            f32::from(structure.width) * pitch,
            f32::from(structure.height) * pitch,
        ));
        let center = Vec2::new(
            (structure.origin.x as f32 + f32::from(structure.width) * 0.5) * pitch,
            (structure.origin.y as f32 + f32::from(structure.height) * 0.5) * pitch,
        );
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

    render_state.mark_sprites_clean();
}

fn ensure_asset_exists(image_path: &str) {
    let path = image_path.trim().replace('\\', "/");
    let Some((namespace, rest)) = path.split_once('/') else {
        panic!(
            "WorldSpriteAssetError:\n  asset: {}\n  reason: expected mod-scoped path <mod_id>/<asset_path>",
            image_path
        );
    };
    let candidate = Path::new("mods").join(namespace).join("assets").join(rest);
    if candidate.is_file() {
        return;
    }

    panic!(
        "WorldSpriteAssetError:\n  asset: {}\n  tried_path: {}\n  reason: file not found",
        image_path,
        candidate.display()
    );
}

fn to_bevy_mod_asset_path(image_path: &str) -> String {
    let path = image_path.trim().replace('\\', "/");
    let Some((namespace, rest)) = path.split_once('/') else {
        panic!(
            "WorldSpriteAssetError:\n  asset: {}\n  reason: expected mod-scoped path <mod_id>/<asset_path>",
            image_path
        );
    };
    format!("{namespace}/assets/{rest}")
}

#[must_use]
pub fn gas_intensity_from_particles(total_particles: u64) -> f32 {
    if total_particles == 0 {
        return 0.0;
    }

    let max_particles_for_mvp = 800.0;
    let normalized = (((total_particles as f32).min(max_particles_for_mvp)) + 1.0).ln()
        / (max_particles_for_mvp + 1.0).ln();
    (0.12 + normalized * 0.73).clamp(0.12, 0.85)
}

#[must_use]
pub fn tile_to_world_center(tile: TilePos, tile_pitch: f32) -> Vec2 {
    let pitch = tile_pitch.max(0.001);
    Vec2::new((tile.x as f32 + 0.5) * pitch, (tile.y as f32 + 0.5) * pitch)
}

#[must_use]
pub fn grid_line_segments(size: GridSize, tile_pitch: f32) -> Vec<(Vec2, Vec2)> {
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
        StructureSprite, WorldCameraControlsConfig, clamp_camera_center_to_world_bounds,
        compute_zoom_bounds, ensure_asset_exists, gas_intensity_from_particles, grid_line_segments,
        tile_to_world_center,
    };
    use bevy::math::Vec2;
    use bevy::window::{Window, WindowResolution};
    use flux_world::{GridSize, TilePos};

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

    #[test]
    fn structure_sprite_uses_expected_world_bounds() {
        let pitch = 1.0;
        let structure = StructureSprite {
            origin: TilePos::new(4, 7),
            width: 2,
            height: 3,
            image_path: "textures/structures/test.png".to_owned(),
        };

        let center = Vec2::new(
            (structure.origin.x as f32 + f32::from(structure.width) * 0.5) * pitch,
            (structure.origin.y as f32 + f32::from(structure.height) * 0.5) * pitch,
        );
        let size = Vec2::new(
            f32::from(structure.width) * pitch,
            f32::from(structure.height) * pitch,
        );

        assert_eq!(center, Vec2::new(5.0, 8.5));
        assert_eq!(size, Vec2::new(2.0, 3.0));
    }

    #[test]
    #[should_panic(expected = "WorldSpriteAssetError")]
    fn missing_asset_panics_fast() {
        ensure_asset_exists("textures/structure/definitely_missing_for_test.png");
    }
}
