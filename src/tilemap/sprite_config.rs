use std::collections::HashMap;

use bevy_ecs::prelude as ecs;

use super::TileRequirements;

///Store config for a tilemap sprite
#[derive(ecs::Component)]
pub struct TilemapSpriteConfig {
    ///Valid placements for tile (usize,usize)
    pub orientations: HashMap<(usize, usize), TileRequirements>,

    pub grid_width: usize,
    pub grid_height: usize,

    pub tile_width: usize,
    pub tile_height: usize,
}

impl TilemapSpriteConfig {
    pub fn new(grid_width: usize, grid_height: usize) -> Self {
        Self {
            tile_width: 8,
            tile_height: 8,

            grid_width,
            grid_height,
            orientations: HashMap::default(),
        }
    }

    pub fn tile_size_uv(&self) -> [f32; 2] {
        [1.0 / self.grid_width as f32, 1.0 / self.grid_height as f32]
    }

    pub fn grid_width(&self) -> u32 {
        self.grid_width as u32
    }

    pub fn position_uv(&self, x: usize, y: usize) -> ([f32; 2], [f32; 2]) {
        let [tile_width, tile_height] = self.tile_size_uv();

        (
            [tile_width * x as f32, tile_height * y as f32],
            [tile_width * (x + 1) as f32, tile_height * (y + 1) as f32],
        )
    }
}
