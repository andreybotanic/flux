use bevy::prelude::{App, Plugin, Resource};
use flux_world::{GridSize, TilePos};

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
    pub tile: TilePos,
    pub base_color: bevy::prelude::Color,
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
    pending_camera_pivot: Option<(u32, u32)>,
    pending_camera_zoom: Option<f32>,
    snapshot: WorldRenderSnapshot,
    sprites_dirty: bool,
    grid_dirty: bool,
}

impl Default for WorldRenderState {
    fn default() -> Self {
        Self {
            visible: false,
            grid_size: None,
            tile_pitch: 1.0,
            reset_camera_requested: false,
            pending_camera_pivot: None,
            pending_camera_zoom: None,
            snapshot: WorldRenderSnapshot::default(),
            sprites_dirty: false,
            grid_dirty: false,
        }
    }
}

impl WorldRenderState {
    #[must_use]
    pub(crate) fn is_visible(&self) -> bool {
        self.visible
    }

    #[must_use]
    pub(crate) fn grid_size(&self) -> Option<GridSize> {
        self.grid_size
    }

    #[must_use]
    pub(crate) fn tile_pitch(&self) -> f32 {
        self.tile_pitch
    }

    #[must_use]
    pub(crate) fn render_snapshot(&self) -> &WorldRenderSnapshot {
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
        self.grid_dirty = true;
    }

    pub fn request_camera_pivot(&mut self, x: u32, y: u32) {
        self.pending_camera_pivot = Some((x, y));
    }

    pub fn request_camera_zoom(&mut self, zoom: f32) {
        self.pending_camera_zoom = Some(zoom);
    }

    #[must_use]
    pub(crate) fn sprites_dirty(&self) -> bool {
        self.sprites_dirty
    }

    pub(crate) fn mark_sprites_clean(&mut self) {
        self.sprites_dirty = false;
        self.grid_dirty = false;
    }

    #[must_use]
    pub(crate) fn grid_dirty(&self) -> bool {
        self.grid_dirty
    }

    pub(crate) fn mark_grid_clean(&mut self) {
        self.grid_dirty = false;
    }

    pub(crate) fn mark_grid_dirty(&mut self) {
        self.grid_dirty = true;
    }

    #[must_use]
    pub(crate) fn reset_camera_requested(&self) -> bool {
        self.reset_camera_requested
    }

    pub(crate) fn clear_reset_camera_request(&mut self) {
        self.reset_camera_requested = false;
    }

    pub(crate) fn take_pending_camera_pivot(&mut self) -> Option<(u32, u32)> {
        self.pending_camera_pivot.take()
    }

    pub(crate) fn take_pending_camera_zoom(&mut self) -> Option<f32> {
        self.pending_camera_zoom.take()
    }
}

pub(crate) struct WorldRenderStatePlugin;

impl Plugin for WorldRenderStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldRenderState>();
    }
}
