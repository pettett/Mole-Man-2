pub mod position;
pub use position::Position;
#[derive(crate::ecs::Component)]
pub struct Bobble;

pub fn bobble_on_update(
    time: crate::ecs::Res<crate::Time>,
    mut query: crate::ecs::Query<&mut Position, crate::ecs::With<Bobble>>,
) {
    query.for_each_mut(|mut p| {
        println!("Bobbling {} {:?}", time.t, p);
        p.0 = time.t.sin();
        p.1 = time.t.cos();
    })
}
