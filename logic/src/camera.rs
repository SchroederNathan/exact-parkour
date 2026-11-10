//! A low chase camera with a temporary manual orbit and a stable movement frame.
use exact_game::*;
use exact_game_physics::{self as physics, Shape};

use crate::{body, park};

const DISTANCE: f32 = 4.8;
const AIM_HEIGHT: f32 = 1.35 - body::HALF;
const SENSITIVITY: f32 = 0.004;
const LOOK_HOLD: f32 = 0.85;

#[derive(Clone, Resource)]
pub struct Orbit {
    pub yaw: f32,
    pub elevation: f32,
    pub distance: f32,
    movement_yaw: f32,
    stick_direction: Vec3,
    look_hold: f32,
    previous_player: Vec3,
    aim: Vec3,
    phase: f32,
    bump: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            elevation: 0.16,
            distance: DISTANCE,
            movement_yaw: 0.0,
            stick_direction: Vec3::ZERO,
            look_hold: 0.0,
            previous_player: Vec3::ZERO,
            aim: Vec3::ZERO,
            phase: 0.0,
            bump: 0.0,
        }
    }
}

pub fn setup(w: &mut World) {
    let player = w.require::<Transform>("player").position;
    let aim = player + Vec3::Y * AIM_HEIGHT;
    w.insert_resource(Orbit {
        aim,
        previous_player: player,
        ..Orbit::default()
    });
    w.spawn_named(
        "camera",
        (
            Transform::default(),
            Camera {
                fov_y_degrees: 64.0,
                near: 0.05,
                ..Camera::default()
            },
        ),
    );
    present(w, body::Motion::default());
}

pub fn input(w: &mut World, input: &Input) {
    // Control-button contacts are separate from canvas pointers. Include a flick
    // released within one tick, but never turn for an unpressed mouse hover.
    let pointer = input.pointer().filter(|p| {
        p.press_origin.is_some() && (p.down || input.pressed("orbit") || input.released("orbit"))
    });
    let delta = pointer.map_or(Vec2::ZERO, |p| p.delta);
    let stick = input.stick_xz("move");
    let direction = if stick.length_squared() > 0.0001 {
        stick.normalize()
    } else {
        Vec3::ZERO
    };
    let dt = w.dt();
    let mut orbit = w.resource_mut::<Orbit>();
    // Automatic camera rotation must not bend a held stick into a circle. A
    // fresh push or deliberate direction change uses the current view again.
    if direction == Vec3::ZERO || orbit.stick_direction.distance_squared(direction) > 0.0025 {
        orbit.movement_yaw = orbit.yaw;
        orbit.stick_direction = direction;
    }
    if pointer.is_some() {
        orbit.look_hold = LOOK_HOLD;
    } else {
        orbit.look_hold = (orbit.look_hold - dt).max(0.0);
    }
    orbit.yaw = math::wrap_angle(orbit.yaw - delta.x * SENSITIVITY);
    orbit.movement_yaw = math::wrap_angle(orbit.movement_yaw - delta.x * SENSITIVITY);
    orbit.elevation = (orbit.elevation + delta.y * SENSITIVITY).clamp(-0.12, 0.95);
}

pub fn movement(w: &World, stick: Vec3) -> Vec3 {
    Quat::from_rotation_y(w.resource::<Orbit>().movement_yaw) * stick
}

pub fn set_offset(w: &mut World, offset: Vec3) {
    let from_aim = offset - Vec3::Y * AIM_HEIGHT;
    let mut orbit = w.resource_mut::<Orbit>();
    orbit.distance = from_aim.length().max(0.5);
    orbit.yaw = math::atan2(from_aim.x, from_aim.z);
    orbit.movement_yaw = orbit.yaw;
    orbit.look_hold = LOOK_HOLD;
    orbit.elevation = math::asin((from_aim.y / orbit.distance).clamp(-1.0, 1.0)).clamp(-0.12, 0.95);
}

pub fn present(w: &mut World, motion: body::Motion) {
    let dt = w.dt();
    let player = w.require::<Transform>("player").position;
    let desired_aim = player + Vec3::Y * AIM_HEIGHT;
    let time = w.seconds() as f32;
    let (aim, offset, up) = {
        let mut orbit = w.resource_mut::<Orbit>();
        let travelled = (player - orbit.previous_player) * Vec3::new(1.0, 0.0, 1.0);
        orbit.previous_player = player;
        if orbit.look_hold == 0.0
            && orbit.stick_direction != Vec3::ZERO
            && travelled.length_squared() > (dt * 0.35) * (dt * 0.35)
            && travelled.length_squared() < 1.0
        {
            let behind = math::atan2(-travelled.x, -travelled.z);
            let turn = math::wrap_angle(behind - orbit.yaw) * (1.0 - math::exp(-dt / 0.28));
            orbit.yaw = math::wrap_angle(orbit.yaw + turn.clamp(-3.8 * dt, 3.8 * dt));
        }
        if orbit.aim.distance(desired_aim) > 8.0 {
            orbit.aim = desired_aim;
        } else {
            orbit.aim = orbit.aim.lerp(desired_aim, 1.0 - math::exp(-dt / 0.075));
        }
        orbit.phase =
            (orbit.phase + dt * (8.0 + motion.speed * 1.5)).rem_euclid(std::f32::consts::TAU);
        orbit.bump = (orbit.bump * math::exp(-dt * 10.0)).max((motion.impact * 0.006).min(0.085));
        let running = if motion.grounded {
            (motion.speed / 6.0).min(1.0)
        } else {
            0.0
        };
        let sway = 0.004 + running * 0.022;
        let shake = Vec3::new(
            math::sin(orbit.phase) * sway,
            math::cos(orbit.phase * 2.0) * sway * 0.65 - orbit.bump,
            0.0,
        );
        let turn = Quat::from_rotation_y(orbit.yaw);
        let offset = turn
            * (Quat::from_rotation_x(-orbit.elevation) * Vec3::new(0.0, 0.0, orbit.distance)
                + shake);
        let roll = math::sin(time * 1.7) * 0.0015 + math::sin(orbit.phase) * running * 0.003;
        let up = turn * Quat::from_rotation_z(roll) * Vec3::Y;
        (orbit.aim, offset, up)
    };
    // Sweep the camera's volume from the player, including the follow lag, so
    // orbiting next to a wall cannot put the lens through it.
    let travel = aim + offset - desired_aim;
    let length = travel.length();
    let distance = physics::sweep(
        w,
        &Shape::Sphere { radius: 0.22 },
        Transform::at(desired_aim.x, desired_aim.y, desired_aim.z),
        travel,
        park::SOLID,
    )
    .map_or(length, |hit| (hit.distance - 0.04).max(0.0));
    let eye = desired_aim + travel.normalize_or_zero() * distance;
    let target = if eye.distance_squared(aim) < 0.01 {
        aim - offset
    } else {
        aim
    };
    *w.require_mut::<Transform>("camera") =
        Transform::at(eye.x, eye.y, eye.z).looking_at(target, up);
}
