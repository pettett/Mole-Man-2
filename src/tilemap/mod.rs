use std::sync::Arc;

use crate::{engine, texture::Texture, uniform::Transformations};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use rand::Rng;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    descriptor_set::WriteDescriptorSet,
    image::StorageImage,
};
bitflags! {
    pub struct Orientation: u8 {
        const NONE = 0;
        const N  = 0b00000001;
        const E  = 0b00000010;
        const S  = 0b00000100;
        const W  = 0b00001000;
        const NW = 0b00010000;
        const SW = 0b00100000;
        const NE = 0b01000000;
        const SE = 0b10000000;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Filled(Orientation),
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
    dirty: bool,
    instance_count: u32,
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
                tiles: [TileData {
                    sheet_pos: 0,
                    grid_pos: 0,
                }; 16 * 16],
            },
        )
        .expect("failed to create buffer");

        let mut s = Self {
            tiles: [[Tile::Filled(Orientation::all()); 16]; 16],
            texture,
            instance_count: 0,
            dirty: true,
            map_buffer,
        };

        s.apply_changes();

        s
    }

    fn get_sheet_pos(&self, o: Orientation) -> u32 {
        //TODO:
        let n = o.contains(Orientation::N);
        let e = o.contains(Orientation::E);
        let s = o.contains(Orientation::S);
        let w = o.contains(Orientation::W);

        let nw = o.contains(Orientation::NW);
        let ne = o.contains(Orientation::NE);
        let sw = o.contains(Orientation::SW);
        let se = o.contains(Orientation::SE);

        let mut rng = rand::thread_rng();

        let (x, y) = match (n, e, s, w, nw, ne, sw, se) {
            //air above
            (false, true, true, true, _, _, _, _) => (rng.gen_range(1..=4), 0),
            (true, false, true, true, _, _, _, _) => (5, rng.gen_range(1..=4)),
            (true, true, false, true, _, _, _, _) => (rng.gen_range(1..=4), 5),
            (true, true, true, false, _, _, _, _) => (0, rng.gen_range(1..=4)),
            // bottom right corner
            (true, false, false, true, _, _, _, _) => (5, 5),
            // top left corner
            (false, true, true, false, _, _, _, _) => (0, 0),
            // top right corner
            (false, false, true, true, _, _, _, _) => (5, 0),
            // bottom left corner
            (true, true, false, false, _, _, _, _) => (0, 5),
            _ => (rng.gen_range(1..=4), rng.gen_range(1..=4)),
        };

        x + y * 16
    }

    pub fn apply_changes(&mut self) {
        //TODO: this would appear in some kind of update function every frame

        if self.dirty {
            let mut tiles = [TileData {
                sheet_pos: 0,
                grid_pos: 0,
            }; 16 * 16];

            let mut i = 0;

            for x in 0..16 {
                for y in 0..16 {
                    if let Tile::Filled(o) = self.tiles[x][y] {
                        tiles[i] = TileData {
                            grid_pos: (y * 16 + x) as u32,
                            sheet_pos: self.get_sheet_pos(o),
                        };
                        i += 1;
                    }
                }
            }

            let mut w = self.map_buffer.write().unwrap();

            (*w).tiles = tiles;

            self.instance_count = i as u32;
        }

        self.dirty = false;
    }

    pub fn tile(&self, x: usize, y: usize) -> &Tile {
        &self.tiles[x][y]
    }

    pub fn toggle(&mut self, x: usize, y: usize) {
        self.tiles[x][y] = match self.tiles[x][y] {
            Tile::Filled(_) => {
                if x > 0 {
                    if let Tile::Filled(o) = &mut self.tiles[x - 1][y] {
                        *o -= Orientation::E
                    }
                }

                if x < 15 {
                    if let Tile::Filled(o) = &mut self.tiles[x + 1][y] {
                        *o -= Orientation::W
                    }
                }

                if y > 0 {
                    if let Tile::Filled(o) = &mut self.tiles[x][y - 1] {
                        *o -= Orientation::N
                    }
                }

                if y < 15 {
                    if let Tile::Filled(o) = &mut self.tiles[x][y + 1] {
                        *o -= Orientation::S
                    }
                }

                Tile::None
            }
            Tile::None => Tile::Filled(Orientation::NONE),
        };

        self.dirty = true;
    }

    pub fn instance_count(&self) -> u32 {
        self.instance_count
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
