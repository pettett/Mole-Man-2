use winit::event::{ElementState, VirtualKeyCode};

pub mod player;
pub use player::Player;

use crate::{physics::velocity::Velocity, transform, InputEvent, Time};

pub fn on_keyboard_input(
    input: crate::ecs::Res<InputEvent>,
    mut query: crate::ecs::Query<(&Player, &mut Velocity)>,
) {
    match input.keycode {
        VirtualKeyCode::W | VirtualKeyCode::S => {
            let dir = if (input.keycode == VirtualKeyCode::W)
                && (input.state == ElementState::Pressed)
            {
                1.0 // If upward direction and pressed are the same (up + pressed) | (down + released)
            } else if (input.keycode == VirtualKeyCode::S) && (input.state == ElementState::Pressed)
            {
                -1.0
            } else {
                0.0
            };

            query.for_each_mut(|(p, mut vel)| {
                vel.1 = p.speed * dir;
            });
        }
        _ => (),
    }
}
