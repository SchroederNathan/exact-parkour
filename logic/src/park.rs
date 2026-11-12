//! The obstacle park: static blocks from one table, and the ramp's rolling objects.
use exact_game::*;
use exact_game_physics::{Body, Collider};

/// Collision layers. The body climbs and vaults only what is `SOLID`; a roller is a
/// `PROP`, which blocks and hits the body but is never a ledge.
pub const SOLID: u32 = 1;
pub const PROP: u32 = 2;
/// The player's own capsule, which no probe of the park should find.
pub const BODY: u32 = 4;

/// One static box. `at` is its centre and `size` its full extents, metres. `yaw` and
/// `pitch` are degrees about Y, then about the box's own X.
pub struct Block {
    pub name: &'static str,
    pub at: [f32; 3],
    pub size: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub color: &'static str,
}

const fn block(name: &'static str, at: [f32; 3], size: [f32; 3], color: &'static str) -> Block {
    Block {
        name,
        at,
        size,
        yaw: 0.0,
        pitch: 0.0,
        color,
    }
}
const fn turned(mut b: Block, yaw: f32) -> Block {
    b.yaw = yaw;
    b
}
const fn pitched(mut b: Block, pitch: f32) -> Block {
    b.pitch = pitch;
    b
}

// The traversal blocks stay strongly separated by hue, but share the skyline's
// ink-and-steel base so the neon edge strips read as intentional wayfinding.
const VAULT: &str = "#263453";
const LEDGE: &str = "#1d4e5d";
const TOWER: &str = "#1a2340";
const GAP: &str = "#302657";
const RAMP: &str = "#24344d";
const STEP: &str = "#34415e";

const CITY_BODY: &str = "#11152b";
const CITY_EDGE: &str = "#20264a";
const CITY_GLASS: &str = "#28345b";
const NEON_PINK: [f32; 3] = [5.8, 0.08, 2.4];
const NEON_CYAN: [f32; 3] = [0.08, 5.2, 6.4];
const NEON_VIOLET: [f32; 3] = [2.6, 0.12, 6.2];
const NEON_AMBER: [f32; 3] = [6.0, 1.8, 0.08];

struct CityTower {
    name: &'static str,
    at: [f32; 3],
    size: [f32; 3],
    accent: [f32; 3],
}

const SKYLINE: &[CityTower] = &[
    CityTower {
        name: "tower-neon-east",
        at: [24.0, 8.5, -20.0],
        size: [7.0, 17.0, 7.0],
        accent: NEON_CYAN,
    },
    CityTower {
        name: "tower-neon-west",
        at: [-24.0, 11.0, -22.0],
        size: [8.0, 22.0, 8.0],
        accent: NEON_PINK,
    },
    CityTower {
        name: "tower-neon-north",
        at: [2.0, 7.0, -26.0],
        size: [11.0, 14.0, 6.0],
        accent: NEON_VIOLET,
    },
    CityTower {
        name: "tower-neon-south",
        at: [-23.0, 6.5, 16.0],
        size: [9.0, 13.0, 9.0],
        accent: NEON_AMBER,
    },
    CityTower {
        name: "tower-neon-southeast",
        at: [23.0, 5.5, 13.0],
        size: [6.0, 11.0, 10.0],
        accent: NEON_CYAN,
    },
];

/// Slope of the ramp, degrees. Its high end is to the north (-Z).
pub const RAMP_PITCH: f32 = 18.0;
/// Where a roller appears: above the ramp's high end.
pub const RAMP_TOP: Vec3 = Vec3::new(14.0, 4.9, -13.0);

/// The park. The player starts at the origin facing north (-Z).
///
/// - `vault-*`: walls to vault, 0.5 to 1.1 m high; one is turned 35 degrees, and
///   `mantle` is too deep to vault, so the body climbs onto it.
/// - `ledge-*`: walls to grab and climb, 1.6 to 2.8 m high; one is turned 30 degrees.
/// - `tower-*`: four roofs at 2.0, 4.2, 6.5 and 9.0 m, each a grab above the last.
///   The drops from them cover every landing: a roll, a fall and a hard fall.
/// - `gap-*`: roofs at 1.2 m with gaps of 1.5, 2.5 and 3.5 m, then one 1.4 m higher
///   across a 2.5 m gap, which is a jump to a ledge.
/// - `stair-*`: steps of 0.15 m, which the capsule walks up. It does not walk up
///   0.25 m, and 0.2 m only sometimes: its round foot meets the edge too high.
/// - `ramp`: the slope the rollers come down, between two rails.
pub const BLOCKS: &[Block] = &[
    block("vault-low", [-6.0, 0.25, -6.0], [3.0, 0.5, 0.6], VAULT),
    block("vault-mid", [-2.0, 0.4, -6.0], [3.0, 0.8, 0.6], VAULT),
    block("vault-high", [2.0, 0.55, -6.0], [3.0, 1.1, 0.6], VAULT),
    block("mantle", [6.5, 0.5, -6.9], [3.0, 1.0, 2.4], VAULT),
    turned(
        block("vault-turned", [-10.5, 0.4, -6.0], [3.0, 0.8, 0.6], VAULT),
        35.0,
    ),
    block("ledge-low", [-8.0, 0.8, -13.5], [4.0, 1.6, 3.0], LEDGE),
    block("ledge-mid", [-3.0, 1.1, -13.5], [4.0, 2.2, 3.0], LEDGE),
    block("ledge-high", [2.0, 1.4, -13.5], [4.0, 2.8, 3.0], LEDGE),
    turned(
        block("ledge-turned", [7.5, 1.1, -14.0], [4.0, 2.2, 3.0], LEDGE),
        30.0,
    ),
    block("tower-1", [-18.0, 1.0, -12.0], [5.0, 2.0, 5.0], TOWER),
    block("tower-2", [-18.0, 2.1, -17.0], [5.0, 4.2, 5.0], TOWER),
    block("tower-3", [-23.0, 3.25, -17.0], [5.0, 6.5, 5.0], TOWER),
    block("tower-4", [-23.0, 4.5, -22.0], [5.0, 9.0, 5.0], TOWER),
    block("gap-1", [-14.0, 0.6, 4.0], [3.0, 1.2, 3.0], GAP),
    block("gap-2", [-14.0, 0.6, 8.5], [3.0, 1.2, 3.0], GAP),
    block("gap-3", [-14.0, 0.6, 14.0], [3.0, 1.2, 3.0], GAP),
    block("gap-4", [-14.0, 0.6, 20.5], [3.0, 1.2, 3.0], GAP),
    block("gap-5", [-14.0, 1.3, 26.0], [3.0, 2.6, 3.0], GAP),
    block("stair-1", [6.0, 0.075, 4.0], [2.4, 0.15, 0.5], STEP),
    block("stair-2", [6.0, 0.15, 4.5], [2.4, 0.3, 0.5], STEP),
    block("stair-3", [6.0, 0.225, 5.0], [2.4, 0.45, 0.5], STEP),
    block("stair-4", [6.0, 0.3, 5.5], [2.4, 0.6, 0.5], STEP),
    block("stair-5", [6.0, 0.375, 6.0], [2.4, 0.75, 0.5], STEP),
    block("stair-6", [6.0, 0.45, 6.5], [2.4, 0.9, 0.5], STEP),
    block("stair-top", [6.0, 0.525, 8.0], [2.4, 1.05, 2.5], STEP),
    pitched(
        block("ramp", [14.0, 1.75, -8.0], [5.0, 0.4, 12.6], RAMP),
        RAMP_PITCH,
    ),
    pitched(
        block("ramp-rail-west", [11.4, 2.15, -8.0], [0.2, 1.0, 12.6], RAMP),
        RAMP_PITCH,
    ),
    pitched(
        block("ramp-rail-east", [16.6, 2.15, -8.0], [0.2, 1.0, 12.6], RAMP),
        RAMP_PITCH,
    ),
    block("ramp-back", [14.0, 2.6, -14.6], [5.4, 5.2, 0.4], RAMP),
];

impl Block {
    pub fn pose(&self) -> Transform {
        let turn = |degrees: f32| degrees * (std::f32::consts::PI / 180.0);
        Transform {
            position: Vec3::from_array(self.at),
            rotation: Quat::from_rotation_y(turn(self.yaw))
                * Quat::from_rotation_x(turn(self.pitch)),
            ..Transform::default()
        }
    }
    /// Height of the top face of an unpitched block.
    pub fn top(&self) -> f32 {
        self.at[1] + self.size[1] / 2.0
    }
}

/// The block named `name`; a misspelled name is a bug in the caller.
pub fn named(name: &str) -> &'static Block {
    BLOCKS
        .iter()
        .find(|b| b.name == name)
        .unwrap_or_else(|| panic!("park: no block `{name}`"))
}

fn accent_material(accent: [f32; 3]) -> Material {
    Material::glow(accent)
}

fn spawn_city_tower(w: &mut World, tower: &CityTower) {
    let [x, y, z] = tower.at;
    let [width, height, depth] = tower.size;
    w.spawn_named(
        tower.name,
        (
            Transform::at(x, y, z),
            Mesh::cuboid(Vec3::new(width, height, depth)),
            Material::hex(CITY_BODY).metallic(0.82).rough(0.24),
        ),
    );
    w.spawn_named(
        format!("{}-cap", tower.name),
        (
            Transform::at(x, y + height * 0.51, z),
            Mesh::cuboid(Vec3::new(width * 1.08, 0.12, depth * 1.08)),
            Material::hex(CITY_EDGE).metallic(0.7).rough(0.18),
        ),
    );
    for (edge, offset) in [("left", -1.0), ("right", 1.0)] {
        w.spawn_named(
            format!("{}-edge-{edge}", tower.name),
            (
                Transform::at(x + offset * width * 0.48, y, z - depth * 0.5 - 0.035),
                Mesh::cuboid(Vec3::new(0.09, height * 0.98, 0.07)),
                accent_material(tower.accent),
            ),
        );
    }
    let floors = (height / 1.45).floor().max(3.0) as usize;
    for floor in 0..floors {
        let fy = y - height * 0.42 + floor as f32 * 1.45;
        let tone = if floor % 3 == 0 {
            tower.accent
        } else if floor % 3 == 1 {
            NEON_CYAN
        } else {
            NEON_PINK
        };
        for (side, offset) in [("front", -1.0), ("back", 1.0)] {
            w.spawn_named(
                format!("{}-window-{floor}-{side}", tower.name),
                (
                    Transform::at(x, fy, z + offset * depth * 0.5 + offset * 0.035),
                    Mesh::cuboid(Vec3::new(width * 0.72, 0.14, 0.035)),
                    accent_material(tone),
                ),
            );
        }
    }
    let antenna_height = 2.0 + (height * 0.12).min(2.8);
    w.spawn_named(
        format!("{}-antenna", tower.name),
        (
            Transform::at(x, y + height * 0.58 + antenna_height * 0.5, z),
            Mesh::cylinder(0.055, antenna_height),
            accent_material(tower.accent),
        ),
    );
    w.spawn_named(
        format!("{}-halo", tower.name),
        (
            Transform::at(x, y + height * 0.58 + antenna_height, z),
            Mesh::torus(0.42, 0.045),
            accent_material(tower.accent),
        ),
    );
    w.spawn_named(
        format!("{}-beacon", tower.name),
        (
            Transform::at(x, y + height * 0.62 + antenna_height + 0.18, z),
            Mesh::sphere(0.13),
            accent_material(tower.accent),
            PointLight {
                color: tower.accent,
                intensity: 80.0,
                range: 12.0,
            },
        ),
    );
}

fn spawn_rooftop_detail(
    w: &mut World,
    name: String,
    position: Vec3,
    mesh: Mesh,
    material: Material,
) {
    w.spawn_named(
        name.as_str(),
        (
            Transform::at(position.x, position.y, position.z),
            mesh,
            material,
        ),
    );
}

fn spawn_city_decorations(w: &mut World) {
    for tower in SKYLINE {
        spawn_city_tower(w, tower);
    }

    for (index, x, z, accent) in [
        (0, -18.0, 5.0, NEON_CYAN),
        (1, -8.0, 17.0, NEON_PINK),
        (2, 15.0, 6.0, NEON_VIOLET),
        (3, 18.0, -2.0, NEON_AMBER),
    ] {
        let name = format!("vent-bank-{index}");
        spawn_rooftop_detail(
            w,
            format!("{name}-base"),
            Vec3::new(x, 0.55, z),
            Mesh::cuboid(Vec3::new(2.8, 1.1, 1.8)),
            Material::hex(CITY_GLASS).metallic(0.72).rough(0.3),
        );
        for vent in [-0.85, 0.0, 0.85] {
            spawn_rooftop_detail(
                w,
                format!("{name}-fan-{vent}"),
                Vec3::new(x + vent, 1.18, z),
                Mesh::cylinder(0.34, 0.08),
                Material::hex(CITY_EDGE).metallic(0.86).rough(0.2),
            );
            spawn_rooftop_detail(
                w,
                format!("{name}-fan-glow-{vent}"),
                Vec3::new(x + vent, 1.24, z),
                Mesh::torus(0.28, 0.035),
                accent_material(accent),
            );
        }
        w.spawn_named(
            format!("{name}-lamp"),
            (
                Transform::at(x, 1.8, z - 0.78),
                Mesh::sphere(0.08),
                accent_material(accent),
                PointLight {
                    color: accent,
                    intensity: 38.0,
                    range: 5.0,
                },
            ),
        );
    }

    for (index, x, z, yaw, accent) in [
        (0, -11.0, -1.8, 0.0, NEON_PINK),
        (1, 11.0, -4.0, 0.4, NEON_CYAN),
        (2, -3.0, 11.0, -0.35, NEON_VIOLET),
    ] {
        let pose = Transform {
            position: Vec3::new(x, 2.3, z),
            rotation: Quat::from_rotation_y(yaw),
            ..Transform::default()
        };
        w.spawn_named(
            format!("holo-sign-{index}"),
            (
                pose,
                Mesh::cuboid(Vec3::new(3.8, 1.6, 0.08)),
                accent_material(accent).alpha(0.72),
            ),
        );
        w.spawn_named(
            format!("holo-sign-{index}-frame"),
            (
                Transform {
                    position: Vec3::new(x, 2.3, z - 0.06),
                    rotation: Quat::from_rotation_y(yaw),
                    ..Transform::default()
                },
                Mesh::cuboid(Vec3::new(4.05, 1.84, 0.04)),
                accent_material(accent),
            ),
        );
        w.spawn_named(
            format!("holo-sign-{index}-light"),
            (
                Transform::at(x, 2.0, z + 0.4),
                PointLight {
                    color: accent,
                    intensity: 65.0,
                    range: 8.0,
                },
            ),
        );
    }

    for (index, x, z, accent) in [
        (0, -27.0, -2.0, NEON_CYAN),
        (1, 27.0, 4.0, NEON_PINK),
        (2, 2.0, 26.0, NEON_AMBER),
    ] {
        spawn_rooftop_detail(
            w,
            format!("antenna-mast-{index}"),
            Vec3::new(x, 3.8, z),
            Mesh::cylinder(0.08, 7.0),
            Material::hex(CITY_EDGE).metallic(0.9).rough(0.16),
        );
        spawn_rooftop_detail(
            w,
            format!("antenna-ring-{index}"),
            Vec3::new(x, 6.9, z),
            Mesh::torus(0.48, 0.055),
            accent_material(accent),
        );
        w.spawn_named(
            format!("antenna-light-{index}"),
            (
                Transform::at(x, 7.2, z),
                Mesh::sphere(0.14),
                accent_material(accent),
                PointLight {
                    color: accent,
                    intensity: 50.0,
                    range: 8.0,
                },
            ),
        );
    }
}

/// A thing that rolls down the ramp. `spawned` is the tick it appeared.
#[derive(Default, Component)]
pub struct Roller {
    pub spawned: u64,
}

/// The ramp's schedule: how many rollers it has released.
#[derive(Default, Resource)]
pub struct Ramp {
    pub released: u32,
}

/// Seconds between rollers, and how long each lives. A body that a roller knocked
/// down needs about 3.5 s to come to rest and get up, so it is on its feet before
/// the next one arrives.
pub const EVERY: f32 = 4.5;
const LIFE: f32 = 12.0;

struct Kind {
    mesh: fn() -> Mesh,
    mass: f32,
    color: &'static str,
    /// Lying on its side, so a cylinder rolls.
    on_side: bool,
}
const KINDS: [Kind; 4] = [
    Kind {
        mesh: || Mesh::sphere(0.35),
        mass: 30.0,
        color: "#d9534f",
        on_side: false,
    },
    Kind {
        mesh: || Mesh::cylinder(0.4, 0.9),
        mass: 70.0,
        color: "#e0a03a",
        on_side: true,
    },
    Kind {
        mesh: || Mesh::cube(0.55),
        mass: 20.0,
        color: "#b98a5a",
        on_side: false,
    },
    Kind {
        mesh: || Mesh::sphere(0.6),
        mass: 220.0,
        color: "#8f3b3b",
        on_side: false,
    },
];

/// Build the static park.
pub fn setup(w: &mut World) {
    for b in BLOCKS {
        let mesh = Mesh::cuboid(Vec3::from_array(b.size));
        w.spawn_named(
            b.name,
            (
                b.pose(),
                Collider {
                    layer: SOLID,
                    ..Collider::of(&mesh)
                },
                mesh,
                Material::hex(b.color),
            ),
        );
        if b.pitch == 0.0 {
            let [width, _, depth] = b.size;
            w.spawn_named(
                format!("{}-neon-edge", b.name),
                (
                    Transform::at(b.at[0], b.top() + 0.035, b.at[2] - depth * 0.44),
                    Mesh::cuboid(Vec3::new(width * 0.86, 0.055, 0.07)),
                    accent_material(if b.name.starts_with("vault") {
                        NEON_PINK
                    } else if b.name.starts_with("ledge") {
                        NEON_CYAN
                    } else {
                        NEON_VIOLET
                    }),
                ),
            );
        }
    }
    spawn_city_decorations(w);
    w.insert_resource(Ramp::default());
}

/// Release the next roller when it is due, and remove the ones that have lived
/// their time. Call once per tick, before `physics::step`.
pub fn tick(w: &mut World) {
    let now = w.tick();
    let hz = 1.0 / w.dt();
    let old: Vec<Entity> = w
        .query::<&Roller>()
        .iter()
        .filter(|(_, r)| (now - r.spawned) as f32 > LIFE * hz)
        .map(|(e, _)| e)
        .collect();
    for e in old {
        w.despawn(e);
    }
    let released = w.resource::<Ramp>().released;
    if (now as f32) < released as f32 * EVERY * hz {
        return;
    }
    w.resource_mut::<Ramp>().released = released + 1;
    release(
        w,
        released,
        RAMP_TOP + Vec3::X * lane(released),
        Vec3::Z * 1.5,
    );
}

// Rollers take the ramp's three lanes in turn.
fn lane(index: u32) -> f32 {
    [0.0, -1.3, 1.3][(index % 3) as usize]
}

/// Put roller number `index` into the world at `at`, moving at `velocity`.
pub fn release(w: &mut World, index: u32, at: Vec3, velocity: Vec3) -> Entity {
    let kind = &KINDS[(index % KINDS.len() as u32) as usize];
    let mesh = (kind.mesh)();
    let rotation = if kind.on_side {
        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
    } else {
        Quat::IDENTITY
    };
    let now = w.tick();
    w.spawn((
        Transform {
            position: at,
            rotation,
            ..Transform::default()
        },
        Collider {
            layer: PROP,
            friction: 0.8,
            bounce: 0.15,
            ..Collider::of(&mesh)
        },
        Body {
            mass: kind.mass,
            velocity,
            ..Body::default()
        },
        mesh,
        Material::hex(kind.color),
        Roller { spawned: now },
    ))
}
