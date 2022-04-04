use crate::engine;

use bevy_ecs::prelude as ecs;

#[derive(ecs::Component)]
pub struct Renderer {
    pub material: engine::MatID,
}
