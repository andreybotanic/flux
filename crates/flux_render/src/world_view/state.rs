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
}

pub(crate) struct WorldRenderStatePlugin;

impl Plugin for WorldRenderStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldRenderState>();
    }
}
