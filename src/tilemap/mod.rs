use std::ops::Range;
pub mod renderer;
pub mod sprite_config;
pub mod sprite_config_editor;
pub mod tile_requirements;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

use bevy_ecs::prelude as ecs;

pub use self::renderer::*;
pub use self::sprite_config::*;
pub use self::sprite_config_editor::*;
pub use self::tile_requirements::*;

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

///Tilemap system to fix any that are marked as dirty
pub fn update_tilemaps(mut query: ecs::Query<&mut TilemapRenderer>) {
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
