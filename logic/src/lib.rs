use exact_game::*;
use exact_game_physics::{self as physics, CapsuleController, Collider, Shape};

pub mod air;
pub mod arena;
pub mod body;
pub mod camera;
pub mod city;
pub mod climb;
pub mod debug;
pub mod flow;
pub mod gait;
pub mod hold;
pub mod ledge;
pub mod locomotion;
mod mannequin;
pub mod kit;
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
const LEGACY_MODE: u32 = 2;
/// The start screen's view of the chosen body: it stands on open ground and the
/// eye goes round it.
pub const LOCKER_MODE: u32 = 4;
/// Where the locker's eye starts from the body's centre, metres: in front of it,
/// a little above. The body faces south there, into the sun. And how fast the eye
/// goes round, radians a second.
const LOCKER_EYE: Vec3 = Vec3::new(0.0, 0.12, 1.75);
const LOCKER_TURN: f32 = 0.5;
/// Neon Rain: the market street and its roofs, in `city.rs`.
pub const CITY_MODE: u32 = 3;

#[derive(Default, Component)]
pub struct Player;

#[derive(Args)]
pub struct Options {
    pub seed: u64,
    /// The body that the player has: 0 is the mannequin, 1 and 2 are the bodies of
    /// `stickman::BODIES`. A change builds the world again.
    pub character: u32,
    #[live]
    pub mode: u32,
    #[live]
    pub race: u32,
    #[live]
    pub paused: bool,
    #[restart]
    pub restart: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            seed: 0,
            character: 0,
            mode: LEGACY_MODE,
            race: 0,
            paused: false,
            restart: false,
        }
    }
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
    pub move_x: f32,
    pub move_y: f32,
}

#[derive(Default, Resource)]
pub struct TouchStick {
    pub x: f32,
    pub y: f32,
}

pub struct ExactBeatEmUp;
impl Game for ExactBeatEmUp {
    const ID: &'static str = "exact-beat-em-up";
    const HZ: u32 = 120;
    const ASSETS: &'static [&'static str] = &stickman::BODIES;
    /// The city's models and its sky.
    const STREAMED: &'static [&'static str] = city::MODELS;
    type Args = Options;

    fn validate(args: &Options) -> Result<(), String> {
        if args.character as usize > stickman::BODIES.len() {
            return Err(format!("character {} is not a body", args.character));
        }
        Ok(())
    }

    fn actions() -> Actions {
        Actions::new()
            .stick("move", Stick::wasd().or_arrows().or_pad(Pad::Left))
            .button("orbit", &["MouseLeft"])
            .button("jump", &["Space", "GamepadSouth"])
            .button(
                "sprint",
                &["ShiftLeft", "ShiftRight", "GamepadL3", "GamepadR1"],
            )
            .button("slide", &["KeyC", "ControlLeft", "GamepadEast"])
    }

    fn register(w: &mut World, _: &std::collections::BTreeMap<&str, Value>) {
        arena::register(w);
        camera::register(w);
        w.register_resource::<TouchStick>();
        w.register_resource::<city::CityState>();
    }

    fn setup(w: &mut World, args: &Options) {
        w.reseed(args.seed);
        w.insert_resource(TouchStick::default());
        physics::register(w);
        w.resource_mut::<physics::Physics>().gravity = Vec3::new(0.0, -body::GRAVITY, 0.0);
        if args.mode == CITY_MODE {
            city::setup(w);
        } else {
            let sky = Color::hex("#9bb8c7");
            w.insert_resource(Environment {
                background: Some(sky),
                fog: Some(Fog {
                    color: Some(sky),
                    ..Fog::new(0.018, 0.08)
                }),
                ..Environment::default()
            });
            let ground = Mesh::plane(PARK * 2.0, PARK * 2.0);
            w.spawn_named(
                "platform",
                (
                    Transform::default(),
                    Collider {
                        shape: Shape::Box { half: Vec3::new(PARK, 1.0, PARK) },
                        offset: Vec3::new(0.0, -1.0, 0.0),
                        layer: park::SOLID,
                        ..Collider::default()
                    },
                    ground,
                    Material::grid(Color::hex("#365466"), 1.0),
                ),
            );
            for (name, x, z) in [("wall-east", 1.0, 0.0), ("wall-west", -1.0, 0.0), ("wall-south", 0.0, 1.0), ("wall-north", 0.0, -1.0)] {
                let across = Vec3::new(z, 0.0, x).abs();
                w.spawn_named(
                    name,
                    (
                        Transform::at(x * (PARK + 0.5), 20.0, z * (PARK + 0.5)),
                        Collider { shape: Shape::Box { half: across * PARK + Vec3::new(0.5, 20.0, 0.5) }, ..Collider::default() },
                    ),
                );
            }
        }

        if args.mode != ARENA_MODE && args.mode != LOCKER_MODE && args.mode != CITY_MODE {
            park::setup(w);
        }
        if args.mode == RACE_MODE {
            race::setup(w, args.race as u8);
        }

        let rig = stickman::rig(body::HEIGHT);
        let model = match args.character {
            0 => stickman::model(&rig),
            body => {
                let name = stickman::BODIES[body as usize - 1];
                let body = w.model(name).expect("setup waits for the bodies");
                stickman::dressed(&rig, body).unwrap_or_else(|why| panic!("{name}: {why}"))
            }
        };
        let player_mesh = w.generated_model(stickman::MODEL, model).unwrap();
        // The capsule stays upright; the visual is its child and does the turning.
        // It starts half a turn about Y: facing up the screen, away from the camera.
        // In the locker it faces the other way, toward the eye.
        let half_turn = if args.mode == LOCKER_MODE {
            0.0
        } else {
            std::f32::consts::PI
        };
        let player_start = match args.mode {
            RACE_MODE => race::start_position(args.race as u8),
            CITY_MODE => city::START,
            _ => Vec3::ZERO,
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
                flow::Flow::default(),
                hold::Limbs::default(),
            ),
        );
        camera::setup(w);
        if args.mode == LOCKER_MODE {
            camera::set_offset(w, LOCKER_EYE);
            camera::present(w, body::Motion::default());
        }
        if args.mode != CITY_MODE {
            w.spawn_named(
                "sun",
                (
                    Transform::at(6.0, 11.0, 8.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
                    DirectionalLight::default(),
                ),
            );
        }
    }

    fn paused(args: &Options) -> bool {
        args.paused
    }

    fn tick(w: &mut World, input: &Input, args: &Options) {
        let stick = input.stick("move");
        {
            let mut touch = w.resource_mut::<TouchStick>();
            touch.x = stick.x;
            touch.y = stick.y;
        }
        debug::commands(w, input);
        camera::input(w, input);
        match args.mode {
            ARENA_MODE => arena::tick(w),
            LOCKER_MODE => {
                // Every tick, so a drag on the picture does not take the eye away.
                let turn = LOCKER_TURN * w.tick() as f32 * w.dt();
                camera::set_offset(w, Quat::from_rotation_y(turn) * LOCKER_EYE);
            }
            CITY_MODE => {}
            _ => park::tick(w),
        }
        let motion = body::step(w, input);
        physics::step(w);
        locomotion::drive(w, "stickman", motion);
        animation::step(w);
        body::present(w, motion);
        camera::present(w, motion);
        if args.mode == RACE_MODE {
            race::tick(w);
        }
        if args.mode == CITY_MODE {
            city::tick(w);
        }
    }
}
