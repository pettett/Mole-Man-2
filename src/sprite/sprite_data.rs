pub use crate::sprite::Sprite;
use crate::transform::position::Position;
pub use bevy_ecs::prelude as ecs;
use std::sync::Arc;

use super::SpritePushConstants;

///The component to draw a sprite in the world
///
///
#[derive(ecs::Component)]
pub struct SpriteData {
    pub sprite: Arc<Sprite>,
    /// Coordinate of tile to render
    pub tile_x: usize,
    pub tile_y: usize,
}

impl SpriteData {
    /// Generate push constants from context
    pub fn get_push_constants(&self, pos: &Position) -> SpritePushConstants {
        let [tile_uv_x, tile_uv_y] = self.sprite.tile_size_uv();
        SpritePushConstants {
            world_x: pos.0,
            world_y: pos.1,
            tile_x: self.tile_x as u32,
            tile_y: self.tile_y as u32,
            tile_uv_x,
            tile_uv_y,
            tile_scale_x: self.sprite.tile_width as f32 / 8.0,
            tile_scale_y: self.sprite.tile_height as f32 / 8.0,
        }
    }
}
