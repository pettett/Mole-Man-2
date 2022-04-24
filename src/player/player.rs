#[derive(crate::ecs::Component, Clone, Copy)]
pub struct Player {
    pub speed: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self { speed: 1.0 }
    }
}
