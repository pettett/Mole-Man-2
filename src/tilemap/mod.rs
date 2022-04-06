use std::{
    cell::RefCell,
    collections::HashMap,
    ops::Range,
    sync::{Arc, Mutex},
};
pub mod editor;

use crate::{engine, rendering::Renderer, texture::Texture, uniform::Transformations};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use rand::Rng;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    descriptor_set::WriteDescriptorSet,
    image::StorageImage,
};

use bevy_ecs::prelude as ecs;

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

impl Orientation {
    pub fn orient(off_x: isize, off_y: isize) -> Option<Orientation> {
        match (off_x, off_y) {
            (1, 0) /* _*/ => Some(Orientation::E),
            (-1, 0)/* _*/ => Some(Orientation::W),
            (0, 1) /* _*/ => Some(Orientation::N),
            (0, -1)/* _*/ => Some(Orientation::S),
            (1, 1) /* _*/ => Some(Orientation::NE),
            (-1, 1) /*_*/ => Some(Orientation::NW),
            (1, -1) /*_*/ => Some(Orientation::SE),
            (-1, -1)/*_*/ => Some(Orientation::SW),
            _ => None,
        }
    }
}

const WIDTH: usize = 16;
const HEIGHT: usize = 16;

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
    //FIXME: These variables are named *wrong*
    tile_width: f32,
    tile_height: f32,
    grid_width: u32,
    sheet_width: u32,
    tiles: [TileData; WIDTH * HEIGHT],
}
#[derive(ecs::Component)]
pub struct Tilemap {
    tiles: [[Tile; HEIGHT]; WIDTH],
    dirty: bool,
    instance_count: u32,
    sprite: Arc<Mutex<TilemapSpriteConfig>>,
    map_buffer: Arc<CpuAccessibleBuffer<TilemapData>>,

    //the actual GPU texture reference for the texture
    texture: Texture<StorageImage>,
}
#[derive(Default, Clone, Copy)]
pub struct TileRequirements {
    n: Option<bool>,
    s: Option<bool>,
    e: Option<bool>,
    w: Option<bool>,

    ne: Option<bool>,
    nw: Option<bool>,
    se: Option<bool>,
    sw: Option<bool>,
}

impl TileRequirements {
    /// Get the tile requirement that corresponds to this direction - mutable
    pub fn get_requirement_mut(&mut self, o: Orientation) -> Result<&mut Option<bool>, ()> {
        match o {
            Orientation::N => Ok(&mut self.n),
            Orientation::S => Ok(&mut self.s),
            Orientation::E => Ok(&mut self.e),
            Orientation::W => Ok(&mut self.w),

            Orientation::NE => Ok(&mut self.ne),
            Orientation::NW => Ok(&mut self.nw),
            Orientation::SE => Ok(&mut self.se),
            Orientation::SW => Ok(&mut self.sw),

            _ => Err(()),
        }
    }
    /// Get the tile requirement that corresponds to this direction
    pub fn get_requirement(&self, o: Orientation) -> Option<&Option<bool>> {
        match o {
            Orientation::N => Some(&self.n),
            Orientation::S => Some(&self.s),
            Orientation::E => Some(&self.e),
            Orientation::W => Some(&self.w),

            Orientation::NE => Some(&self.ne),
            Orientation::NW => Some(&self.nw),
            Orientation::SE => Some(&self.se),
            Orientation::SW => Some(&self.sw),

            _ => None,
        }
    }
}

///Store config for a tilemap sprite
#[derive(ecs::Component)]
pub struct TilemapSpriteConfig {
    ///Valid placements for tile (usize,usize)
    orientations: HashMap<(usize, usize), TileRequirements>,

    grid_width: usize,
    grid_height: usize,

    tile_width: usize,
    tile_height: usize,
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

///Tilemap system to fix any that are marked as dirty
pub fn update_tilemaps(mut query: ecs::Query<&mut Tilemap>) {
    query.for_each_mut(|mut tilemap| tilemap.apply_changes())
}

///Does offsetting this usize place it within the range?
fn offset_in_range(i: usize, off_i: isize, range: Range<usize>) -> bool {
    let v = i as isize + off_i;

    v >= 0 && range.contains(&(v as usize))
}

/// Attempt to offset a coordinate, returning [`None`] if it is out of bounds
pub fn offset(x: usize, y: usize, off_x: isize, off_y: isize) -> Option<(usize, usize)> {
    if offset_in_range(x, off_x, 0..WIDTH) && offset_in_range(y, off_y, 0..HEIGHT) {
        Some(((x as isize + off_x) as usize, (y as isize + off_y) as usize))
    } else {
        None
    }
}

impl Tilemap {
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
