use exact_game::character::Character;
use exact_game::*;

pub mod gait;
pub mod locomotion;
pub mod stickman;

use locomotion::Stride;
use gait::{RUN, SPRINT};

/// Ground acceleration, m/s²: top speed in about a tenth of a second, and a
/// change of direction as fast.
const ACCEL: f32 = 40.0;
/// Braking with the stick released, m/s²: a run stops within a stride, which is
/// the distance the legs need to close into a stand without sliding.
const BRAKE: f32 = 24.0;
/// The body turns to face where it is going: by this share of the angle left per
/// second, and never faster than `TURN_MAX` radians per second, so an about-face is
/// a quick spin and not a flip.
const TURN: f32 = 18.0;
const TURN_MAX: f32 = 14.0;

#[derive(Default, Component)]
pub struct Player;

#[derive(Default, Args)]
pub struct Options {
    pub seed: u64,
    #[live]
    pub paused: bool,
    #[restart]
    pub restart: bool,
    /// The touch screen's Sprint toggle; a keyboard or pad holds the `sprint` action.
    #[live]
    pub sprint: bool,
}

pub struct ExactBeatEmUp;
impl Game for ExactBeatEmUp {
    const ID: &'static str = "exact-beat-em-up";
    const HZ: u32 = 120;
    type Args = Options;

    fn actions() -> Actions {
        Actions::new()
            .stick("move", Stick::wasd().or_arrows().or_pad(Pad::Left))
            .button("jump", &["Space", "GamepadSouth"])
            .button("sprint", &["ShiftLeft", "ShiftRight", "GamepadL3", "GamepadR1"])
    }

    fn setup(w: &mut World, args: &Options) {
        w.reseed(args.seed);
        let sky = Color::hex("#9bb8c7");
        w.insert_resource(Environment {
            background: Some(sky),
            fog: Some(Fog {
                color: Some(sky),
                ..Fog::new(0.018, 0.08)
            }),
            ..Environment::default()
        });
        w.spawn_named(
            "platform",
            (
                Transform::default(),
                Mesh::plane(36.0, 36.0),
                Material::grid(Color::hex("#365466"), 1.0),
            ),
        );

        let rig = stickman::rig(1.8);
        let player_mesh = w
            .generated_model(stickman::MODEL, stickman::model(&rig))
            .unwrap();
        let player = w.spawn_named(
            "player",
            (
                // Half a turn about Y: facing up the screen, away from the camera.
                Transform {
                    rotation: Quat::from_xyzw(0.0, 1.0, 0.0, 0.0),
                    ..Transform::default()
                },
                player_mesh,
                stickman::animator(),
                stickman::layers(),
                Stride::default(),
                Player,
                Character::new()
                    .speed(RUN)
                    .accel(ACCEL)
                    .brake(BRAKE)
                    .jump(1.35)
                    .gravity(9.81)
                    .ground(0.0)
                    .bounds_xz(-17.5..=17.5),
            ),
        );
        w.spawn_named(
            "camera",
            (
                Transform::default(),
                Camera::default(),
                Follow::new(player)
                    .offset(0.0, 6.0, 8.0)
                    .look_at_offset(0.0, 0.8, 0.0)
                    .lag(0.12),
            ),
        );
        w.spawn_named(
            "sun",
            (
                Transform::at(6.0, 11.0, 8.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
                DirectionalLight::default(),
            ),
        );
    }

    fn paused(args: &Options) -> bool {
        args.paused
    }

    fn tick(w: &mut World, input: &Input, args: &Options) {
        let sprint = args.sprint || input.held("sprint");
        let top = if sprint { SPRINT } else { RUN };
        if w.require::<Character>("player").speed != top {
            w.require_mut::<Character>("player").speed = top;
        }
        // The touch screen's Jump is a native glass button, which sits outside the
        // canvas and so posts a message; a key or pad presses the action.
        let jump = input.pressed("jump") || input.messages().iter().any(|m| m == "jump");
        let contact = w
            .character("player")
            .step(input.stick_xz("move"), jump);

        let velocity = w.require::<Character>("player").velocity;
        let planar = Vec3::new(velocity.x, 0.0, velocity.z);
        let speed = planar.length();
        if speed > 0.02 {
            // The rig's rest pose faces +Z, so the yaw is the heading itself.
            let target = Quat::from_rotation_y(math::atan2(planar.x, planar.z));
            let dt = w.dt();
            let mut pose = w.require_mut::<Transform>("player");
            let left = pose.rotation.angle_between(target);
            if left > 1e-4 {
                let step = (left * TURN).min(TURN_MAX) * dt;
                pose.rotation = pose.rotation.slerp(target, (step / left).min(1.0)).normalize();
            }
        }
        locomotion::drive(w, "player", speed, velocity.y, contact);
        animation::step(w);
    }
}
