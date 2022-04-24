pub mod sprite;
pub mod sprite_data;
pub use bevy_ecs::prelude as ecs;
pub use sprite::Sprite;
use vulkano::{descriptor_set::WriteDescriptorSet, image::StorageImage};

use crate::{engine, texture::Texture, uniform::Transformations};

pub use self::sprite_data::SpriteData;

pub fn create_sprite_material(
    engine: &mut engine::Engine,
    tex: &Texture<StorageImage>,
    globals: &Transformations,
) -> engine::MatID {
    let vs = sprite_vs::load(engine.device()).unwrap();
    let fs = sprite_fs::load(engine.device()).unwrap();

    engine.create_material(
        vs,
        fs,
        [
            WriteDescriptorSet::buffer(0, globals.get_buffer()),
            tex.describe(3),
        ],
    )
}

pub struct SpritePushConstants {
    world_x: f32,
    world_y: f32,
    tile_x: u32,
    tile_y: u32,
    tile_uv_x: f32,
    tile_uv_y: f32,
    tile_scale_x: f32,
    tile_scale_y: f32,
}

mod sprite_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/sprite/shader.vert"
    }
}

mod sprite_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/sprite/shader.frag"
    }
}
