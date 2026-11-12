//! Neon Rain: a market street at night, in the rain, and the roofs above it.
//!
//! The street runs north (-Z) from the start to the tall building `north`, which
//! carries the pink sign: the goal, in view from the first step. The route
//! crosses the street three times, each time higher:
//!
//! 1. From the street up a crate and a stall to the roof `west-low`.
//! 2. East through the covered walkway to the roof `east-low`.
//! 3. Up the fire escape to the roof `east-high`.
//! 4. West over the roof of the skybridge to the roof `west-high`.
//! 5. North across the alley to the roof `north`, and to the sign.
//!
//! Nothing is painted to show the way. Each step ends at a light: lanterns over
//! the crate, the lamps in the walkway, the red lamp on the fire escape, the
//! cyan lamps on the skybridge and the pink sign. One cyan ring stands at the
//! place the route goes next.
//!
//! `BUILDINGS` and `ROUTE` are tables, so `logic/tests/city.rs` checks every
//! step against what the body can climb and jump.
use exact_game::emitter::{Shape as Spawn, WorldSpace};
use exact_game::*;
use exact_game_physics::{Collider, Shape};

use crate::body;
use crate::kit::{self, Piece, Surface};

/// Where the body starts: the south end of the street, facing north.
pub const START: Vec3 = Vec3::new(0.0, 0.0, 9.0);
/// Half the width of the street.
pub const STREET: f32 = 6.0;

/// One building: a box from the street to its roof.
pub struct Building {
    pub name: &'static str,
    pub x: [f32; 2],
    pub z: [f32; 2],
    pub roof: f32,
    pub walls: &'static Surface,
}

const fn building(name: &'static str, x: [f32; 2], z: [f32; 2], roof: f32, walls: &'static Surface) -> Building {
    Building {
        name,
        x,
        z,
        roof,
        walls,
    }
}

/// The buildings of the route, and the ones that close the district in. The
/// outer ones are higher than every roof of the route, so they are its walls.
pub const BUILDINGS: &[Building] = &[
    building("west-start", [-18.0, -6.0], [0.0, 12.0], 7.0, &kit::WINDOWS),
    building("west-low", [-18.0, -6.0], [-13.0, -1.0], 4.6, &kit::TILE),
    building("west-high", [-18.0, -6.0], [-30.0, -15.0], 10.0, &kit::WINDOWS),
    building("east-start", [6.0, 18.0], [0.0, 12.0], 8.0, &kit::BRICK),
    building("east-low", [6.0, 18.0], [-14.2, -1.0], 5.6, &kit::CONCRETE),
    building("east-high", [6.0, 18.0], [-30.0, -18.0], 8.4, &kit::WINDOWS),
    building("north", [-12.0, 12.0], [-44.0, -32.0], 11.4, &kit::WINDOWS),
    building("outer-west-a", [-42.0, -20.0], [-6.0, 22.0], 15.0, &kit::WINDOWS),
    building("outer-west-b", [-42.0, -20.0], [-30.0, -8.0], 21.0, &kit::BRICK),
    building("outer-west-c", [-42.0, -14.0], [-56.0, -32.0], 17.0, &kit::WINDOWS),
    building("outer-east-a", [20.0, 42.0], [-4.0, 22.0], 18.0, &kit::WINDOWS),
    building("outer-east-b", [20.0, 42.0], [-30.0, -6.0], 14.0, &kit::CONCRETE),
    building("outer-east-c", [19.0, 42.0], [-56.0, -32.0], 23.0, &kit::WINDOWS),
    building("outer-north", [-12.0, 17.0], [-70.0, -46.0], 26.0, &kit::WINDOWS),
    building("outer-south-a", [-30.0, -2.0], [26.0, 46.0], 19.0, &kit::WINDOWS),
    building("outer-south-b", [0.0, 30.0], [26.0, 46.0], 15.0, &kit::BRICK),
];

/// The building named `name`; a misspelled name is a bug in the caller.
pub fn named(name: &str) -> &'static Building {
    BUILDINGS
        .iter()
        .find(|b| b.name == name)
        .unwrap_or_else(|| panic!("city: no building `{name}`"))
}

/// The covered walkway crosses the street here, its base on the roof `west-low`.
pub const WALKWAY_AT: Vec3 = Vec3::new(0.0, 4.6, -10.0);
/// The fire escape: its low landing is level with the roof `east-low`, and its
/// back is on the south wall of `east-high`.
pub const ESCAPE_AT: Vec3 = Vec3::new(9.0, 5.6 - kit::ESCAPE_LOW, -16.12);
/// The skybridge: its roof is one mantle above `east-high` and one below `west-high`.
pub const SKYBRIDGE_AT: Vec3 = Vec3::new(0.0, 9.2 - kit::SKYBRIDGE_TOP, -24.0);
/// The stall the route climbs, its back on the wall of `west-low`.
pub const STALL_AT: Vec3 = Vec3::new(-4.3, 0.0, -5.0);
/// The crate at the south end of that stall.
pub const CRATE_AT: Vec3 = Vec3::new(-4.3, 0.5, -1.85);
pub const CRATE: Vec3 = Vec3::new(1.3, 1.0, 1.3);
/// The pink sign on the roof `north`.
pub const SIGN_AT: Vec3 = Vec3::new(0.0, 11.4, -40.5);

/// One step of the route: where the feet are when it is done, and what the
/// player is asked to do to get there.
pub struct Step {
    pub ask: &'static str,
    pub at: [f32; 3],
}

pub const ROUTE: &[Step] = &[
    Step {
        ask: "Climb the stall to the roof",
        at: [-7.6, 4.6, -5.0],
    },
    Step {
        ask: "Cross the covered walkway",
        at: [7.6, 5.6, -10.0],
    },
    Step {
        ask: "Take the fire escape up",
        at: [10.3, 8.4, -19.6],
    },
    Step {
        ask: "Run the roof of the skybridge",
        at: [-7.8, 10.0, -24.0],
    },
    Step {
        ask: "Jump the alley to the pink sign",
        at: [0.0, 11.4, -37.0],
    },
];
/// How near the feet must come to the end of a step, metres.
pub const REACH: f32 = 2.4;

#[derive(Default, Resource)]
pub struct CityState {
    /// The step the player is on; `ROUTE.len()` when the route is done.
    pub step: u32,
    pub started: u64,
    /// The tick the last step ended; the clock stops there.
    pub finished_at: u64,
}

/// Every model the city shows, for `Game::STREAMED`.
pub const MODELS: &[&str] = &[
    "awning.model",
    "awning-far.model",
    "balcony.model",
    "balcony-far.model",
    "billboard.model",
    "billboard-far.model",
    "crane.model",
    "crane-far.model",
    "elevator.model",
    "elevator-far.model",
    "facade.model",
    "facade-far.model",
    "fire-escape.model",
    "fire-escape-far.model",
    "garden.model",
    "garden-far.model",
    "hvac.model",
    "hvac-far.model",
    "ladder.model",
    "ladder-far.model",
    "neon.model",
    "neon-far.model",
    "pipes.model",
    "pipes-far.model",
    "pole.model",
    "pole-far.model",
    "scaffold.model",
    "scaffold-far.model",
    "skybridge.model",
    "skybridge-far.model",
    "stall.model",
    "stall-far.model",
    "viaduct.model",
    "viaduct-far.model",
    "walkway.model",
    "walkway-far.model",
    "zip-anchor.model",
    "zip-anchor-far.model",
    "district.model",
    "sky.tex",
    "cloud.tex",
];

const WARM: [f32; 3] = [1.0, 0.66, 0.36];
const RED: [f32; 3] = [1.0, 0.24, 0.12];
const PINK: [f32; 3] = [1.0, 0.18, 0.62];
const CYAN: [f32; 3] = [0.2, 0.85, 1.0];
const GREEN: [f32; 3] = [0.3, 1.0, 0.55];
const WHITE: [f32; 3] = [0.82, 0.9, 1.0];

/// A light with no body. `candela` lights a surface `d` metres away with
/// `candela / d²` lux; the moon gives 1,600 lux.
fn lamp(w: &mut World, name: &str, at: Vec3, color: [f32; 3], candela: f32, range: f32) -> Entity {
    w.spawn_named(
        name,
        (
            Transform::at(at.x, at.y, at.z),
            PointLight {
                color,
                intensity: candela,
                range,
            },
        ),
    )
}

/// A small glowing ball: a lantern or a bulb.
fn bulb(w: &mut World, at: Vec3, radius: f32, color: [f32; 3], glow: f32) {
    w.spawn((
        Transform::at(at.x, at.y, at.z),
        Mesh::sphere(radius),
        Material {
            color: [color[0] * 0.4, color[1] * 0.4, color[2] * 0.4, 1.0],
            emissive: [color[0] * glow, color[1] * glow, color[2] * glow],
            roughness: 0.6,
            ..Material::default()
        },
    ));
}

/// Lanterns on a line across the street at `z`, which hangs lowest in the middle.
fn lanterns(w: &mut World, index: usize, z: f32, height: f32, color: [f32; 3]) {
    const COUNT: usize = 7;
    for i in 0..COUNT {
        let t = i as f32 / (COUNT - 1) as f32;
        let x = (t * 2.0 - 1.0) * (STREET - 0.4);
        let sag = 1.0 - (t * 2.0 - 1.0) * (t * 2.0 - 1.0);
        bulb(w, Vec3::new(x, height - sag * 0.9, z), 0.17, color, 3.0);
    }
    lamp(
        w,
        &format!("lantern-light-{index}"),
        Vec3::new(0.0, height - 1.2, z),
        color,
        9000.0,
        11.0,
    );
}

/// A neon sign on a wall: its image, centre, the degrees its face is turned from
/// south (+Z), its size, and the colour of the light it throws.
struct Sign {
    image: &'static str,
    at: [f32; 3],
    yaw: f32,
    size: f32,
    color: [f32; 3],
}

const SIGNS: &[Sign] = &[
    Sign {
        image: "sign-sake.tex",
        at: [STREET - 0.06, 5.4, 6.5],
        yaw: -90.0,
        size: 2.6,
        color: PINK,
    },
    Sign {
        image: "sign-hotel.tex",
        at: [-STREET + 0.06, 7.2, -20.0],
        yaw: 90.0,
        size: 3.0,
        color: CYAN,
    },
    Sign {
        image: "sign-ramen.tex",
        at: [STREET - 0.06, 3.9, -7.0],
        yaw: -90.0,
        size: 3.2,
        color: RED,
    },
    Sign {
        image: "sign-pharmacy.tex",
        at: [3.6, 7.4, -31.94],
        yaw: 0.0,
        size: 2.4,
        color: GREEN,
    },
    Sign {
        image: "sign-sake.tex",
        at: [-STREET + 0.06, 4.9, 4.4],
        yaw: 90.0,
        size: 2.2,
        color: PINK,
    },
];

/// Crates and bins along the walls, to vault on the way up the street: centre
/// on the ground (x, z) and size.
const BINS: &[([f32; 2], [f32; 3])] = &[
    ([0.9, 3.4], [2.6, 0.8, 0.7]),
    ([-1.6, -11.5], [0.7, 1.0, 2.4]),
    ([2.6, -15.2], [1.4, 1.1, 1.0]),
    ([-4.9, -9.8], [1.2, 1.2, 1.2]),
    ([4.8, -1.2], [1.1, 0.7, 1.1]),
];

/// Far towers, past the fog's first reach, where only the windows carry: centre
/// on the ground (x, z), side and height.
const TOWERS: &[[f32; 4]] = &[
    [-70.0, -40.0, 22.0, 74.0],
    [-58.0, 30.0, 18.0, 52.0],
    [-96.0, -6.0, 26.0, 96.0],
    [64.0, -52.0, 20.0, 66.0],
    [82.0, 6.0, 24.0, 88.0],
    [58.0, 44.0, 18.0, 48.0],
    [-26.0, -98.0, 24.0, 82.0],
    [22.0, -104.0, 20.0, 110.0],
    [70.0, -96.0, 22.0, 58.0],
    [-74.0, -92.0, 20.0, 64.0],
    [-22.0, 78.0, 22.0, 70.0],
    [34.0, 84.0, 20.0, 56.0],
];

/// Every textured box and sign of the district: the street, the buildings, the
/// crates and the far towers. `setup` shows it as `district.model` and gives each
/// box of `boxes` a collider.
pub fn district() -> kit::District {
    let mut d = kit::District::default();
    // The street: wet asphalt over everything, a metre thick.
    d.block(
        "street",
        Vec3::new(0.0, -1.0, -12.0),
        Vec3::new(150.0, 2.0, 150.0),
        &kit::CONCRETE,
        &kit::ASPHALT,
    );
    for b in BUILDINGS {
        let size = Vec3::new(b.x[1] - b.x[0], b.roof, b.z[1] - b.z[0]);
        let at = Vec3::new((b.x[0] + b.x[1]) / 2.0, b.roof / 2.0, (b.z[0] + b.z[1]) / 2.0);
        d.block(b.name, at, size, b.walls, &kit::ROOF);
    }
    d.block("crate", CRATE_AT, CRATE, &kit::CORRUGATED, &kit::PLATE);
    d.block(
        "crate-small",
        CRATE_AT + Vec3::new(1.2, -0.2, 0.3),
        Vec3::new(0.8, 0.6, 0.8),
        &kit::CORRUGATED,
        &kit::PLATE,
    );
    for (i, (at, size)) in BINS.iter().enumerate() {
        d.block(
            &format!("bin-{i}"),
            Vec3::new(at[0], size[1] / 2.0, at[1]),
            Vec3::from_array(*size),
            &kit::CORRUGATED,
            &kit::PLATE,
        );
    }
    for [x, z, side, height] in TOWERS {
        d.scenery(
            Vec3::new(*x, height / 2.0, *z),
            Vec3::new(*side, *height, *side),
            &kit::TOWER,
            &kit::ROOF,
        );
    }
    for sign in SIGNS {
        d.sign(sign.image, Vec3::from_array(sign.at), sign.yaw, sign.size, sign.size);
    }
    d
}

fn dress(w: &mut World, name: &str, piece: &Piece, at: [f32; 3], yaw: f32) {
    kit::place(w, name, piece, Vec3::from_array(at), yaw);
}

/// Build the district. Call in place of the park's ground, walls and sun.
pub fn setup(w: &mut World) {
    // Night under a low cloud: the sky map lights what no lamp reaches, and the
    // fog takes its colour from the glow of the street.
    let haze = Color([0.055, 0.045, 0.09]);
    w.insert_resource(Environment {
        background: Some(haze),
        zenith: [0.012, 0.016, 0.035],
        horizon: [0.07, 0.05, 0.11],
        ground: [0.02, 0.016, 0.028],
        ambient: 1.0,
        sun_disc: 0.0,
        exposure: 1.4,
        fog: Some(Fog {
            color: Some(haze),
            ..Fog::new(0.011, 0.045)
        }),
        bloom: Some(Bloom {
            threshold: 0.9,
            intensity: 0.32,
            radius: 2.2,
        }),
        ..Environment::default()
    });
    w.insert_resource(EnvironmentMap {
        texture: "sky.tex".into(),
        intensity: 1.25,
        rgbm: 0.0,
        visible: true,
        rotation: 0.6,
    });
    w.spawn_named(
        "moon",
        (
            Transform::at(-14.0, 30.0, 12.0).looking_at(Vec3::new(0.0, 0.0, -18.0), Vec3::Y),
            DirectionalLight {
                color: [0.55, 0.66, 1.0],
                illuminance: 1600.0,
                shadows: true,
            },
        ),
    );

    w.spawn_named(
        "district",
        (Transform::default(), Mesh::asset("district.model")),
    );
    for b in district().boxes {
        kit::collider(w, &b.name, b.at, Quat::IDENTITY, b.size);
    }
    for (i, sign) in SIGNS.iter().enumerate() {
        let at = Vec3::from_array(sign.at);
        let out = Quat::from_rotation_y(sign.yaw.to_radians()) * Vec3::Z * 1.2;
        lamp(w, &format!("sign-light-{i}"), at + out, sign.color, 14000.0, 12.0);
    }
    // Unseen walls behind the outer buildings and across the ends of the street.
    for (name, at, half) in [
        ("edge-west", [-19.5, 30.0, -10.0], [0.5, 30.0, 40.0]),
        ("edge-east", [19.5, 30.0, -10.0], [0.5, 30.0, 40.0]),
        ("edge-north", [0.0, 30.0, -45.0], [20.0, 30.0, 0.5]),
        ("edge-south", [0.0, 30.0, 25.0], [20.0, 30.0, 0.5]),
    ] {
        w.spawn_named(
            name,
            (
                Transform::at(at[0], at[1], at[2]),
                Collider {
                    shape: Shape::Box {
                        half: Vec3::from_array(half),
                    },
                    ..Collider::default()
                },
            ),
        );
    }
    route(w);
    market(w);
    roofs(w);
    weather(w);

    let first = Vec3::from_array(ROUTE[0].at);
    w.spawn_named(
        "beacon",
        (
            Transform {
                position: first + Vec3::Y * 1.3,
                rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ..Transform::default()
            },
            Mesh::torus(0.75, 0.05),
            Material {
                color: [0.05, 0.3, 0.35, 1.0],
                emissive: [0.15, 1.1, 1.3],
                ..Material::default()
            },
            PointLight {
                color: CYAN,
                intensity: 3500.0,
                range: 8.0,
            },
        ),
    );
    w.insert_resource(CityState {
        step: 0,
        started: w.tick(),
        finished_at: 0,
    });
}

/// The pieces the route runs on.
fn route(w: &mut World) {
    // 1. The crate and the stall under the west wall.
    kit::place(w, "stall-route", &kit::STALL, STALL_AT, 90.0);
    lamp(w, "stall-route-light", STALL_AT + Vec3::new(2.4, 2.0, 0.0), WARM, 16000.0, 10.0);
    lamp(w, "crate-light", CRATE_AT + Vec3::new(0.6, 2.4, 0.6), WARM, 9000.0, 8.0);
    bulb(w, CRATE_AT + Vec3::new(-0.2, 2.9, 0.0), 0.2, RED, 3.5);

    // 2. The covered walkway, lit warm from end to end.
    kit::place(w, "walkway", &kit::WALKWAY, WALKWAY_AT, 0.0);
    for (i, x) in [-5.0, 0.0, 5.0].into_iter().enumerate() {
        lamp(
            w,
            &format!("walkway-light-{i}"),
            WALKWAY_AT + Vec3::new(x, 2.7, 0.0),
            WARM,
            7000.0,
            8.0,
        );
    }

    // 3. The fire escape, under a red lamp.
    kit::place(w, "fire-escape", &kit::FIRE_ESCAPE, ESCAPE_AT, 0.0);
    lamp(w, "fire-escape-light", ESCAPE_AT + Vec3::new(-1.0, 4.4, 1.2), RED, 14000.0, 10.0);
    bulb(w, ESCAPE_AT + Vec3::new(2.6, 6.4, -1.7), 0.16, RED, 4.0);

    // 4. The skybridge, with cyan lamps along its roof.
    kit::place(w, "skybridge", &kit::SKYBRIDGE, SKYBRIDGE_AT, 0.0);
    for (i, x) in [-6.2, -2.0, 2.0, 6.2].into_iter().enumerate() {
        let at = SKYBRIDGE_AT + Vec3::new(x, kit::SKYBRIDGE_TOP + 0.05, 1.2);
        bulb(w, at, 0.09, CYAN, 5.0);
        bulb(w, at - Vec3::Z * 2.4, 0.09, CYAN, 5.0);
        if i % 2 == 0 {
            lamp(
                w,
                &format!("skybridge-light-{i}"),
                SKYBRIDGE_AT + Vec3::new(x, kit::SKYBRIDGE_TOP + 0.9, 0.0),
                CYAN,
                6000.0,
                9.0,
            );
        }
    }
    lamp(w, "skybridge-under", SKYBRIDGE_AT + Vec3::new(0.0, -0.4, 0.0), CYAN, 9000.0, 10.0);

    // 5. The pink sign, and its light on the roof and down the street.
    kit::place(w, "sign-goal", &kit::NEON, SIGN_AT, 0.0);
    lamp(w, "sign-goal-light", SIGN_AT + Vec3::new(0.0, 3.6, 3.0), PINK, 60000.0, 30.0);
    lamp(w, "sign-goal-street", Vec3::new(0.0, 9.0, -29.0), PINK, 40000.0, 26.0);
}

/// What makes the street a place: stalls, awnings, balconies, signs, poles.
fn market(w: &mut World) {
    for (i, (x, z, yaw)) in [
        (4.3, 5.0, -90.0),
        (-4.3, 6.0, 90.0),
        (4.3, -7.0, -90.0),
        (4.3, -21.5, -90.0),
        (-4.3, -26.5, 90.0),
    ]
    .into_iter()
    .enumerate()
    {
        let name = format!("stall-{i}");
        kit::place(w, &name, &kit::STALL, Vec3::new(x, 0.0, z), yaw);
        let front = if x > 0.0 { -2.4 } else { 2.4 };
        lamp(
            w,
            &format!("{name}-light"),
            Vec3::new(x + front, 2.0, z),
            WARM,
            15000.0,
            10.0,
        );
    }
    for (i, (z, height, color)) in [
        (2.5, 6.4, WARM),
        (-3.0, 6.8, RED),
        (-15.5, 7.2, WARM),
        (-28.0, 6.6, RED),
    ]
    .into_iter()
    .enumerate()
    {
        lanterns(w, i, z, height, color);
    }
    let (west, east) = (-STREET, STREET);
    dress(w, "awning-0", &kit::AWNING, [west, 2.7, 2.0], 90.0);
    dress(w, "awning-1", &kit::AWNING, [east, 2.7, 9.0], -90.0);
    dress(w, "awning-2", &kit::AWNING, [east, 2.7, -12.0], -90.0);
    dress(w, "awning-3", &kit::AWNING, [west, 2.7, -18.0], 90.0);
    dress(w, "awning-4", &kit::AWNING, [east, 2.7, -27.0], -90.0);
    dress(w, "balcony-0", &kit::BALCONY, [west, 3.3, 9.0], 90.0);
    dress(w, "balcony-1", &kit::BALCONY, [east, 4.2, 3.0], -90.0);
    dress(w, "balcony-2", &kit::BALCONY, [west, 5.6, -28.0], 90.0);
    dress(w, "balcony-3", &kit::BALCONY, [east, 4.3, -20.5], -90.0);
    dress(w, "balcony-4", &kit::BALCONY, [-6.5, 3.4, -32.0], 0.0);
    dress(w, "balcony-5", &kit::BALCONY, [6.5, 3.4, -32.0], 0.0);
    dress(w, "balcony-6", &kit::BALCONY, [0.0, 7.2, -32.0], 0.0);
    dress(w, "pipes-0", &kit::PIPES, [east, 0.0, -3.2], -90.0);
    dress(w, "pipes-1", &kit::PIPES, [9.6, 0.0, -32.0], 0.0);
    dress(w, "ladder-0", &kit::LADDER, [-10.2, 4.9, -32.0], 0.0);
    dress(w, "ladder-1", &kit::LADDER, [east, 1.5, 10.6], -90.0);
    dress(w, "pole-0", &kit::POLE, [-5.2, 0.0, 11.0], 0.0);
    dress(w, "pole-1", &kit::POLE, [4.9, 0.0, -16.2], 180.0);
    dress(w, "pole-2", &kit::POLE, [-5.2, 0.0, -13.9], 0.0);
    dress(w, "scaffold", &kit::SCAFFOLD, [-3.3, 0.0, -20.5], 90.0);
    dress(w, "elevator", &kit::ELEVATOR, [15.0, 0.0, -36.0], 0.0);
    dress(w, "viaduct-west", &kit::VIADUCT, [-12.0, 0.0, 19.0], 0.0);
    dress(w, "viaduct-east", &kit::VIADUCT, [12.0, 0.0, 19.0], 0.0);
    // A block of flats in the gap past the viaduct, seen from the high roofs.
    dress(w, "facade", &kit::FACADE, [-1.0, 0.0, 30.5], 180.0);
}

/// What stands on the roofs.
fn roofs(w: &mut World) {
    let roof = |name: &str| named(name).roof;
    dress(w, "hvac-west", &kit::HVAC, [-13.3, roof("west-low"), -6.4], 0.0);
    dress(w, "garden", &kit::GARDEN, [14.6, roof("east-low"), -7.4], -90.0);
    lamp(w, "garden-light", Vec3::new(12.0, roof("east-low") + 2.2, -7.4), WARM, 7000.0, 9.0);
    dress(w, "hvac-east", &kit::HVAC, [13.3, roof("east-high"), -25.6], 180.0);
    dress(w, "zip-east", &kit::ZIP_ANCHOR, [16.2, roof("east-high"), -19.8], 90.0);
    dress(w, "crane", &kit::CRANE, [-13.6, roof("west-high"), -22.4], 0.0);
    lamp(w, "crane-light", Vec3::new(-11.0, roof("west-high") + 5.5, -20.0), WHITE, 16000.0, 14.0);
    dress(w, "zip-west", &kit::ZIP_ANCHOR, [-8.0, roof("west-start"), 2.0], -90.0);
    dress(w, "billboard", &kit::BILLBOARD, [13.0, roof("east-start"), 6.0], -90.0);
    lamp(w, "billboard-light", Vec3::new(9.5, roof("east-start") + 3.6, 6.0), WHITE, 30000.0, 20.0);
    dress(w, "pole-roof-0", &kit::POLE, [-10.5, roof("north"), -42.6], 90.0);
    dress(w, "pole-roof-1", &kit::POLE, [10.5, roof("north"), -42.6], 90.0);
    dress(w, "pipes-roof", &kit::PIPES, [-15.0, roof("west-start"), 0.4], 180.0);
    // Cool lamps on the roofs of the route, so an edge shows against the dark.
    for (i, at) in [
        [-8.0, 4.6 + 2.6, -11.5],
        [8.5, 5.6 + 2.6, -3.0],
        [16.0, 8.4 + 2.8, -28.5],
        [-7.5, 10.0 + 2.8, -29.0],
        [-9.0, 11.4 + 3.0, -34.0],
        [9.0, 11.4 + 3.0, -34.0],
    ]
    .into_iter()
    .enumerate()
    {
        lamp(w, &format!("roof-light-{i}"), Vec3::from_array(at), WHITE, 9000.0, 11.0);
    }
}

/// Rain that falls round the body, and a layer of low cloud that drifts over
/// the roofs. The rain is 1,300 streaks at most and the cloud 60 puffs; five
/// vents add 18 puffs of steam each.
fn weather(w: &mut World) {
    w.spawn_named(
        "rain",
        (
            Transform {
                position: START + Vec3::Y * RAIN_HEIGHT,
                // Upside down: the emitter throws along its own +Y.
                rotation: Quat::from_rotation_x(std::f32::consts::PI),
                ..Transform::default()
            },
            Emitter {
                shape: Spawn::Box(Vec3::new(34.0, 1.0, 34.0)),
                rate: 1900.0,
                lifetime: 0.68,
                speed: 21.0,
                spread: 0.03,
                gravity: Vec3::new(1.2, 6.0, 0.0),
                size: [0.022, 0.022],
                color: [[0.62, 0.72, 0.95, 0.5], [0.62, 0.72, 0.95, 0.35]],
                additive: true,
                bound: [-18.0, -1.0, -18.0, 18.0, 16.0, 18.0],
                seed: 11,
                ..Emitter::default()
            },
            ParticleLook {
                stretch: 0.035,
                ..ParticleLook::default()
            },
            WorldSpace::default(),
            Ambient,
        ),
    );
    // The cloud fills its box in the first seconds: each puff appears in place,
    // drifts east and thins out as it ages.
    w.spawn_named(
        "clouds",
        (
            Transform {
                position: Vec3::new(-10.0, 52.0, -14.0),
                // The emitter throws along its own +Y: lay it over to drift east.
                rotation: Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                ..Transform::default()
            },
            Emitter {
                shape: Spawn::Box(Vec3::new(14.0, 230.0, 230.0)),
                rate: 2.5,
                lifetime: 24.0,
                speed: 2.2,
                spread: 0.1,
                gravity: Vec3::ZERO,
                size: [44.0, 62.0],
                color: [[0.24, 0.19, 0.36, 0.55], [0.13, 0.2, 0.3, 0.0]],
                additive: false,
                bound: [-40.0, -150.0, -130.0, 40.0, 190.0, 130.0],
                seed: 5,
                ..Emitter::default()
            },
            ParticleLook {
                texture: "cloud.tex".into(),
                ..ParticleLook::default()
            },
            Ambient,
        ),
    );
    // Steam from the kitchens and the roof machines.
    for (i, at) in [
        [-4.9, 2.9, -6.2],
        [4.9, 2.9, 4.2],
        [4.9, 2.9, -22.4],
        [-12.0, 6.6, -6.0],
        [11.6, 10.4, -25.4],
    ]
    .into_iter()
    .enumerate()
    {
        w.spawn_named(
            &format!("steam-{i}"),
            (
                Transform::at(at[0], at[1], at[2]),
                Emitter {
                    shape: Spawn::Sphere(0.25),
                    rate: 5.0,
                    lifetime: 3.6,
                    speed: 0.9,
                    spread: 0.35,
                    gravity: Vec3::new(0.25, 0.5, 0.0),
                    drag: 0.5,
                    size: [0.5, 2.6],
                    color: [[0.78, 0.76, 0.86, 0.2], [0.7, 0.72, 0.85, 0.0]],
                    additive: false,
                    bound: [-3.0, -0.5, -3.0, 4.0, 6.0, 3.0],
                    seed: 20 + i as u64,
                    ..Emitter::default()
                },
                ParticleLook {
                    texture: "cloud.tex".into(),
                    soft: 0.4,
                    ..ParticleLook::default()
                },
                Ambient,
            ),
        );
    }
}

/// The rain starts this far above the body's feet.
const RAIN_HEIGHT: f32 = 14.0;

/// Move the rain with the body, advance the route and publish the HUD. Call once
/// per tick, after the body has moved.
pub fn tick(w: &mut World) {
    let feet = w.require::<Transform>("player").position - Vec3::Y * body::HALF;
    {
        let mut rain = w.require_mut::<Transform>("rain");
        rain.position = Vec3::new(feet.x, feet.y + RAIN_HEIGHT, feet.z);
    }
    let (step, started) = {
        let state = w.resource::<CityState>();
        (state.step as usize, state.started)
    };
    let mut now = step;
    if let Some(goal) = ROUTE.get(step) {
        let to = Vec3::from_array(goal.at);
        let flat = Vec3::new(feet.x - to.x, 0.0, feet.z - to.z).length();
        if flat < REACH && (feet.y - to.y).abs() < 1.2 {
            now = step + 1;
            w.resource_mut::<CityState>().step = now as u32;
            match ROUTE.get(now) {
                Some(next) => {
                    let mut beacon = w.require_mut::<Transform>("beacon");
                    beacon.position = Vec3::from_array(next.at) + Vec3::Y * 1.3;
                }
                None => {
                    w.require_mut::<Transform>("beacon").position.y = -40.0;
                    let tick = w.tick();
                    w.resource_mut::<CityState>().finished_at = tick;
                    w.emit("race-finished");
                }
            }
        }
    }
    let finished = now >= ROUTE.len();
    let seconds = (w.tick().saturating_sub(started) as f32 / w.hz() as f32).min(999.9);
    let (move_x, move_y) = {
        let stick = w.resource::<crate::TouchStick>();
        (stick.x, stick.y)
    };
    w.publish_record(&crate::Hud {
        mode: "city".into(),
        race: "Neon Rain".into(),
        subtitle: ROUTE
            .get(now)
            .map_or("You reached the sign", |s| s.ask)
            .into(),
        seconds: if finished {
            let state = w.resource::<CityState>();
            (state.finished_at.saturating_sub(state.started) as f32 / w.hz() as f32).min(999.9)
        } else {
            seconds
        },
        finished,
        round: 0,
        enemies: 0,
        hits: 0,
        phase: format!("{} / {}", now.min(ROUTE.len()), ROUTE.len()),
        move_x,
        move_y,
    });
}
