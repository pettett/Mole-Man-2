use crate::{engine, rendering::Renderer, texture::Texture, uniform::Transformations};

use rand::Rng;
use std::sync::{Arc, Mutex};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    descriptor_set::WriteDescriptorSet,
    image::StorageImage,
};

use bevy_ecs::prelude as ecs;

use super::{
    offset, tilemap_fs, tilemap_vs, Orientation, Tile, TileData, TilemapData, TilemapSpriteConfig,
    HEIGHT, WIDTH,
};

#[derive(ecs::Component)]
pub struct TilemapRenderer {
    tiles: [[Tile; HEIGHT]; WIDTH],
    dirty: bool,
    instance_count: u32,
    sprite: Arc<Mutex<TilemapSpriteConfig>>,
    map_buffer: Arc<CpuAccessibleBuffer<TilemapData>>,

    //the actual GPU texture reference for the texture
    texture: Texture<StorageImage>,
}

impl TilemapRenderer {
    pub fn new(
        sprite: Arc<Mutex<TilemapSpriteConfig>>,
        texture: Texture<StorageImage>,
        engine: &engine::Engine,
    ) -> Self {
        let map_buffer = {
            let sprite_lock = sprite.lock().unwrap();

            let [tile_width, tile_height] = sprite_lock.tile_size_uv();

            CpuAccessibleBuffer::from_data(
                engine.device(),
                BufferUsage::all(), //TODO: this should be more specific?
                false,
                TilemapData {
                    tile_width,
                    tile_height,
                    sheet_width: sprite_lock.grid_width(),
                    grid_width: 16,
                    tiles: [TileData {
                        sheet_pos: 0,
                        grid_pos: 0,
                    }; 16 * 16],
                },
            )
            .expect("failed to create buffer")
        };

        let mut rng = rand::thread_rng();

        let mut s = Self {
            tiles: [[Tile::Filled(Orientation::all()); 16]; 16],
            sprite,
            instance_count: 0,
            texture,
            dirty: true,
            map_buffer,
        };

        for x in 0..16 {
            for y in 0..16 {
                if rng.gen_bool(0.1) {
                    s.toggle(x, y);
                }
            }
        }

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

    pub fn get_orientation_offset_mut(
        &mut self,
        x: usize,
        y: usize,
        off_x: isize,
        off_y: isize,
    ) -> Option<&mut Orientation> {
        if let Some(Tile::Filled(o)) = self.get_tile_offset_mut(x, y, off_x, off_y) {
            Some(o)
        } else {
            None
        }
    }

    pub fn remove_orientation(
        &mut self,
        x: usize,
        y: usize,
        off_x: isize,
        off_y: isize,
        dif: Orientation,
    ) {
        if let Some(o) = self.get_orientation_offset_mut(x, y, off_x, off_y) {
            *o -= dif
        }
    }
    ///Mark tile at `[x][y]` as having a new [`Tile`] adjacent
    ///
    ///## Returns
    ///The change in orientation of the effected tile
    pub fn add_orientation(
        &mut self,
        x: usize,
        y: usize,
        off_x: isize,
        off_y: isize,
        dif: Orientation,
    ) -> Orientation {
        if let Some(o) = self.get_orientation_offset_mut(x, y, off_x, off_y) {
            o.insert(dif);
            dif
        } else {
            Orientation::NONE
        }
    }

    pub fn get_tile_offset_mut(
        &mut self,
        x: usize,
        y: usize,
        off_x: isize,
        off_y: isize,
    ) -> Option<&mut Tile> {
        if let Some((x, y)) = offset(x, y, off_x, off_y) {
            Some(&mut self.tiles[x][y])
        } else {
            None
        }
    }
    pub fn get_tile_offset(&self, x: usize, y: usize, off_x: isize, off_y: isize) -> Option<&Tile> {
        if let Some((x, y)) = offset(x, y, off_x, off_y) {
            Some(&self.tiles[x][y])
        } else {
            None
        }
    }
    pub fn tile_exists(&self, x: usize, y: usize, off_x: isize, off_y: isize) -> bool {
        return self.get_tile_offset(x, y, off_x, off_y).is_some();
    }

    pub fn tile_filled(&self, x: usize, y: usize, off_x: isize, off_y: isize) -> bool {
        if let Some(Tile::None) = self.get_tile_offset(x, y, off_x, off_y) {
            false
        } else {
            true
        }
    }

    pub fn update_orientation_offset(&mut self, x: usize, y: usize, off_x: isize, off_y: isize) {
        //Reset the orientation of this tile
        let mut orientation = Orientation::NONE;

        //then build it back up by looking at if adjacent tiles exist
        if self.tile_filled(x, y, off_x + 1, off_y) {
            orientation |= Orientation::E;
        }
        if self.tile_filled(x, y, off_x - 1, off_y) {
            orientation |= Orientation::W;
        }
        if self.tile_filled(x, y, off_x, off_y + 1) {
            orientation |= Orientation::N;
        }
        if self.tile_filled(x, y, off_x, off_y - 1) {
            orientation |= Orientation::S;
        }
        if self.tile_filled(x, y, off_x + 1, off_y - 1) {
            orientation |= Orientation::SE;
        }
        if self.tile_filled(x, y, off_x - 1, off_y - 1) {
            orientation |= Orientation::SW;
        }
        if self.tile_filled(x, y, off_x + 1, off_y + 1) {
            orientation |= Orientation::NE;
        }
        if self.tile_filled(x, y, off_x - 1, off_y + 1) {
            orientation |= Orientation::NW;
        }

        if let Some(o) = self.get_orientation_offset_mut(x, y, off_x, off_y) {
            *o = orientation
        }
    }

    ///Toggle the tile at `[x][y]`, updating orientations around the tiles
    pub fn toggle(&mut self, x: usize, y: usize) {
        self.tiles[x][y] = match self.tiles[x][y] {
            Tile::Filled(_) => Tile::None,
            Tile::None => Tile::Filled(Orientation::NONE),
        };
        //Update the orientations of all the tiles we touched
        for off_x in -1..=1 {
            for off_y in -1..=1 {
                self.update_orientation_offset(x, y, off_x, off_y);
            }
        }

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
