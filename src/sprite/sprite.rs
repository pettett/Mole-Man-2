use vulkano::image::StorageImage;

use crate::texture::Texture;

///Stores texture related information
pub struct Sprite {
    /// Amount of tiles horizontally
    pub grid_width: usize,
    pub grid_height: usize,

    ///Width of a tile (sub-sprite) inside the grid
    pub tile_width: usize,
    pub tile_height: usize,
}

impl Sprite {
    /// Get the width and height of a tile in uv space
    pub fn tile_size_uv(&self) -> [f32; 2] {
        [1.0 / self.grid_width as f32, 1.0 / self.grid_height as f32]
    }
    /// Get the top left point of a tile in uv space
    pub fn position_uv(&self, x: usize, y: usize) -> ([f32; 2], [f32; 2]) {
        let [tile_width, tile_height] = self.tile_size_uv();

        (
            [tile_width * x as f32, tile_height * y as f32],
            [tile_width * (x + 1) as f32, tile_height * (y + 1) as f32],
        )
    }
}
