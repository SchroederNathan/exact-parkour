use exact_game::*;
use exact_game_physics::{self as physics, CapsuleController, Collider, Shape};

pub mod body;
pub mod arena;
pub mod debug;
pub mod gait;
pub mod hold;
pub mod ledge;
pub mod locomotion;
pub mod park;
pub mod race;
pub mod ragdoll;
pub mod rise;
pub mod stickman;

use body::Parkour;
use locomotion::Stride;

/// Half the side of the square park, metres.
pub const PARK: f32 = 30.0;
pub const RACE_MODE: u32 = 0;
pub const ARENA_MODE: u32 = 1;

#[derive(Default, Component)]
pub struct Player;

#[derive(Default, Args)]
pub struct Options {
    pub seed: u64,
    #[live]
    pub mode: u32,
    #[live]
    pub race: u32,
    #[live]
    pub paused: bool,
    #[restart]
    pub restart: bool,
}

#[derive(Default, Data)]
pub struct Hud {
    pub mode: String,
    pub race: String,
    pub subtitle: String,
    pub seconds: f32,
    pub finished: bool,
    pub round: u32,
    pub enemies: u32,
    pub hits: u32,
    pub phase: String,
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
            .button("slide", &["KeyC", "ControlLeft", "GamepadEast"])
    }

    fn register(w: &mut World, _: &std::collections::BTreeMap<&str, Value>) {
        arena::register(w);
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
        physics::register(w);
        w.resource_mut::<physics::Physics>().gravity = Vec3::new(0.0, -body::GRAVITY, 0.0);
        let ground = Mesh::plane(PARK * 2.0, PARK * 2.0);
        w.spawn_named(
            "platform",
            (
                Transform::default(),
                // A metre of ground under the surface. The plane's own collider is a
                // 1 cm slab, which the capsule dips through on its first step.
                Collider {
                    shape: Shape::Box {
                        half: Vec3::new(PARK, 1.0, PARK),
                    },
                    offset: Vec3::new(0.0, -1.0, 0.0),
                    layer: park::SOLID,
                    ..Collider::default()
                },
                ground,
                Material::grid(Color::hex("#365466"), 1.0),
            ),
        );
        // Unseen walls keep every body in the park.
        for (name, x, z) in [
            ("wall-east", 1.0, 0.0),
            ("wall-west", -1.0, 0.0),
            ("wall-south", 0.0, 1.0),
            ("wall-north", 0.0, -1.0),
        ] {
            let across = Vec3::new(z, 0.0, x).abs();
            w.spawn_named(
                name,
                (
                    Transform::at(x * (PARK + 0.5), 20.0, z * (PARK + 0.5)),
                    Collider {
                        shape: Shape::Box {
                            half: across * PARK + Vec3::new(0.5, 20.0, 0.5),
                        },
                        ..Collider::default()
                    },
                ),
            );
        }

        if args.mode == RACE_MODE {
            park::setup(w);
            race::setup(w, args.race as u8);
        }

        let rig = stickman::rig(body::HEIGHT);
        let player_mesh = w
            .generated_model(stickman::MODEL, stickman::model(&rig))
            .unwrap();
        // The capsule stays upright; the visual is its child and does the turning.
        // It starts half a turn about Y: facing up the screen, away from the camera.
        let half_turn = std::f32::consts::PI;
        let player_start = if args.mode == RACE_MODE {
            race::start_position(args.race as u8)
        } else {
            Vec3::ZERO
        };
        let player = w.spawn_named(
            "player",
            (
                Transform::at(player_start.x, body::HALF + 0.01, player_start.z),
                CapsuleController {
                    radius: body::RADIUS,
                    height: body::HEIGHT,
                    ..CapsuleController::default()
                },
                Collider {
                    layer: park::BODY,
                    ..Collider::default()
                },
                Parkour {
                    yaw: half_turn,
                    ..Parkour::default()
                },
                ragdoll::Ragdoll::default(),
                Player,
            ),
        );
        if args.mode == ARENA_MODE {
            arena::setup(w, player);
        }
        w.spawn_named(
            "stickman",
            (
                Parent(player),
                Transform {
                    position: Vec3::new(0.0, -body::HALF - 0.01, 0.0),
                    rotation: Quat::from_rotation_y(half_turn),
                    ..Transform::default()
                },
                player_mesh,
                stickman::animator(),
                stickman::layers(),
                Stride::default(),
            ),
        );
        w.spawn_named(
            "camera",
            (
                Transform::default(),
                Camera::default(),
                Follow::new(player)
                    .offset(0.0, 6.0, 8.0)
                    .look_at_offset(0.0, 0.8 - body::HALF, 0.0)
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
        debug::commands(w, input);
        if args.mode == ARENA_MODE {
            arena::tick(w);
        } else {
            park::tick(w);
        }
        let motion = body::step(w, input);
        physics::step(w);
        locomotion::drive(w, "stickman", motion);
        animation::step(w);
        body::present(w);
        if args.mode == RACE_MODE {
            race::tick(w);
        }
    }
}
