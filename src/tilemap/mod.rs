use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    descriptor_set::WriteDescriptorSet,
    image::StorageImage,
};

use crate::{engine, texture::Texture, uniform::Transformations};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Filled,
    None,
}

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct TileData {
    sheet_pos: u32,
    grid_pos: u32,
}

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct TilemapData {
    //width and height of cells in UV space
    tile_width: f32,
    tile_height: f32,
    grid_width: u32,
    sheet_width: u32,
    tiles: [TileData; 16 * 16],
}

pub struct Tilemap {
    tiles: [[Tile; 16]; 16],
    texture: Texture<StorageImage>,
    map_buffer: Arc<CpuAccessibleBuffer<TilemapData>>,
}

impl Tilemap {
    pub fn new(texture: Texture<StorageImage>, engine: &engine::Engine) -> Self {
        let map_buffer = CpuAccessibleBuffer::from_data(
            engine.device(),
            BufferUsage::all(), //TODO: this should be more specific?
            false,
            TilemapData {
                tile_width: 8.0 / texture.get_size()[0] as f32,
                tile_height: 8.0 / texture.get_size()[1] as f32,
                sheet_width: texture.get_size()[0] / 8,
                grid_width: 16,
                tiles: (0..16 * 16)
                    .map(|i| TileData {
                        grid_pos: i,
                        sheet_pos: 0,
                    })
                    .collect::<Vec<TileData>>()
                    .as_slice()
                    .try_into()
                    .unwrap(),
            },
        )
        .expect("failed to create buffer");

        Self {
            tiles: [[Tile::Filled; 16]; 16],
            texture,
            map_buffer,
        }
    }

    pub fn tile(&self, x: usize, y: usize) -> &Tile {
        &self.tiles[x][y]
    }

    pub fn toggle(&mut self, x: usize, y: usize) {
        self.tiles[x][y] = if self.tiles[x][y] == Tile::Filled {
            Tile::None
        } else {
            Tile::Filled
        }
    }

    pub fn instance_count(&self) -> u32 {
        16 * 16
    }

    pub fn create_material(
        &self,
        engine: &mut engine::Engine,
        globals: &Transformations,
    ) -> engine::MatID {
        let vs = tilemap_vs::load(engine.device()).unwrap();
        let fs = tilemap_fs::load(engine.device()).unwrap();

        engine.create_material(
            vs,
            fs,
            [
                WriteDescriptorSet::buffer(0, globals.get_buffer()),
                WriteDescriptorSet::buffer(1, self.map_buffer.clone()),
                self.texture.describe(3),
            ],
        )
    }
}
mod tilemap_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/tilemap/shader.frag"
    }
}
mod tilemap_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/tilemap/shader.vert"
    }
}
