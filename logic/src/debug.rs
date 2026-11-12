//! Commands for looking at the game from outside: an agent or a test posts them to
//! the world as messages (`post world "goto -2 0 -3 180"`). Nothing in the app
//! sends them.
//!
//! - `goto <x> <y> <z> <yaw degrees>`: stand the body with its feet there, facing
//!   that way; zero faces south (+Z).
//! - `camera <x> <y> <z>`: hold the camera that far from the body's centre, looking
//!   at it. `camera 0 0 0`, or a drag on the scene, gives it back to the rig. A
//!   fourth number raises the point it looks at: `camera 0.6 0.1 -0.6 -0.8` is a
//!   close view of the feet.
//! - `roller <kind> <speed>`: send roller `kind` at the body from behind its back.
use crate::body::{self, Parkour};
use crate::park;
use exact_game::*;
use exact_game_physics::CapsuleController;

fn numbers(words: &[&str]) -> Option<Vec<f32>> {
    words.iter().map(|w| w.parse().ok()).collect()
}

/// Carry out this tick's commands. Call first in the tick.
pub fn commands(w: &mut World, input: &Input) {
    for message in input.messages() {
        let words: Vec<&str> = message.split_whitespace().collect();
        let Some((&name, rest)) = words.split_first() else {
            continue;
        };
        let Some(n) = numbers(rest) else {
            continue;
        };
        match (name, n.as_slice()) {
            ("goto", &[x, y, z, yaw]) => {
                let yaw = yaw * (std::f32::consts::PI / 180.0);
                w.require_mut::<Transform>("player").position =
                    Vec3::new(x, y + body::HALF + 0.02, z);
                w.require_mut::<Transform>("stickman").position =
                    Vec3::new(0.0, -body::HALF - 0.01, 0.0);
                *w.require_mut::<Parkour>("player") = Parkour {
                    yaw,
                    ..Parkour::default()
                };
                crate::ragdoll::stop(w);
                let mut c = w.require_mut::<CapsuleController>("player");
                c.velocity = Vec3::ZERO;
                c.grounded = false;
                c.support = None;
            }
            ("camera", &[x, y, z]) => {
                crate::camera::set_offset(w, Vec3::new(x, y, z));
            }
            ("camera", &[x, y, z, lift]) => {
                crate::camera::look_at(w, Vec3::new(x, y, z), lift);
            }
            ("roller", &[kind, speed]) => {
                let (at, yaw) = {
                    let at = w.require::<Transform>("player").position;
                    (at, w.require::<Parkour>("player").yaw)
                };
                let (sin, cos) = math::sin_cos(yaw);
                let facing = Vec3::new(sin, 0.0, cos);
                let from = Vec3::new(at.x, at.y - 0.2, at.z) - facing * 1.4;
                park::release(w, kind as u32, from, facing * speed);
            }
            _ => {}
        }
    }
}
