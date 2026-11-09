//! The player's body: an upright capsule that the physics controller moves through
//! the world, and what that body is doing.
use crate::gait::{RUN, SPRINT};
use exact_game::motion::{Gravity, Jump, Move};
use exact_game::*;
use exact_game_physics::{self as physics, CapsuleController};

/// The capsule: total height and radius in metres. Its centre is `HALF` above the feet.
pub const HEIGHT: f32 = 1.8;
pub const RADIUS: f32 = 0.3;
pub const HALF: f32 = HEIGHT / 2.0;
/// Real gravity, shared with every rigid body in the park, m/s².
pub const GRAVITY: f32 = 9.81;
/// Height of a standing jump, metres.
pub const JUMP: f32 = 0.75;
/// Ground acceleration, m/s²: top speed in about a tenth of a second, and a change
/// of direction as fast.
const ACCEL: f32 = 40.0;
/// Braking with the stick released, m/s²: a run stops within a stride, which is the
/// distance the legs need to close into a stand without sliding.
const BRAKE: f32 = 24.0;
/// The body turns to face where it is going: by this share of the angle left per
/// second, and never faster than `TURN_MAX` radians per second, so an about-face is
/// a quick spin and not a flip.
const TURN: f32 = 18.0;
const TURN_MAX: f32 = 14.0;

/// What the animation needs to know about this tick's motion.
#[derive(Clone, Copy, Debug, Default)]
pub struct Motion {
    /// Ground speed, m/s.
    pub speed: f32,
    /// Vertical velocity, m/s.
    pub rising: f32,
    pub grounded: bool,
    /// Touched down this tick.
    pub landed: bool,
}

/// The body's own state, beside the controller's.
#[derive(Default, Component)]
pub struct Parkour {
    /// Heading about Y in radians; zero faces +Z. The visual turns toward it.
    pub yaw: f32,
    /// Whether the last tick ended on the ground.
    pub grounded: bool,
}

/// Feet position of a capsule centre.
pub fn feet(centre: Vec3) -> Vec3 {
    centre - Vec3::Y * HALF
}

/// Read the controls, move the capsule and turn the visual. Call before `physics::step`.
pub fn step(w: &mut World, input: &Input) -> Motion {
    let dt = w.dt();
    let was_grounded = w.require::<Parkour>("player").grounded;
    let velocity = {
        let mut c = w.require_mut::<CapsuleController>("player");
        let speed = if input.held("sprint") { SPRINT } else { RUN };
        Move {
            speed,
            accel: ACCEL,
            brake: BRAKE,
        }
        .step(&mut c.velocity, input.stick_xz("move"), dt);
        if c.grounded && input.pressed("jump") {
            Jump {
                height: JUMP,
                gravity: GRAVITY,
            }
            .start(&mut c.velocity);
        }
        Gravity(GRAVITY).step(&mut c.velocity, dt);
        c.velocity
    };
    let moved = physics::capsule(w, "player").step(velocity);
    let planar = Vec3::new(moved.displacement.x, 0.0, moved.displacement.z) / dt;
    let speed = planar.length();

    let yaw = {
        let mut body = w.require_mut::<Parkour>("player");
        if speed > 0.02 {
            // The rig's rest pose faces +Z, so the yaw is the heading itself.
            let target = math::atan2(planar.x, planar.z);
            let left = wrap_angle(target - body.yaw);
            let turn = (left.abs() * TURN).min(TURN_MAX) * dt;
            body.yaw = wrap_angle(body.yaw + left.clamp(-turn, turn));
        }
        if body.grounded != moved.grounded {
            body.grounded = moved.grounded;
        }
        body.yaw
    };
    let facing = Quat::from_rotation_y(yaw);
    if w.require::<Transform>("stickman").rotation != facing {
        w.require_mut::<Transform>("stickman").rotation = facing;
    }
    Motion {
        speed,
        rising: velocity.y,
        grounded: moved.grounded,
        landed: moved.grounded && !was_grounded,
    }
}

/// An angle brought into (-π, π].
pub fn wrap_angle(a: f32) -> f32 {
    let turns = a / std::f32::consts::TAU;
    (turns - math::floor(turns + 0.5)) * std::f32::consts::TAU
}
