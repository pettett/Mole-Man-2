use crate::{transform, Time};

pub mod velocity;
pub use velocity::Velocity;
///Update position of physics objects
/// TODO: On fixed update
pub fn on_update(
    time: crate::ecs::Res<Time>,
    mut query: crate::ecs::Query<(&Velocity, &mut transform::Position)>,
) {
    query.for_each_mut(|(vel, mut pos)| {
        pos.0 += vel.0 * time.dt;
        pos.1 += vel.1 * time.dt;
    })
}
