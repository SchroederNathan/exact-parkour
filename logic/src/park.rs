//! The obstacle park: static blocks from one table, and the ramp's rolling objects.
use exact_game::*;
use exact_game_physics::{Body, Collider};

/// Collision layers. The body climbs and vaults only what is `SOLID`; a roller is a
/// `PROP`, which blocks and hits the body but is never a ledge.
pub const SOLID: u32 = 1;
pub const PROP: u32 = 2;
/// The player's own capsule, which no probe of the park should find.
pub const BODY: u32 = 4;
/// A bar across a path. It stops a body on its feet, and a sliding body goes under
/// it. It is not `SOLID`, so it has no edge to vault or grab.
pub const BAR: u32 = 8;

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

const VAULT: &str = "#c9793a";
const LEDGE: &str = "#3f8f8a";
const TOWER: &str = "#6b7f99";
const GAP: &str = "#7d6aa8";
const RAMP: &str = "#8a8f6a";
const STEP: &str = "#9a8f7c";

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
    }
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
    release(w, released, RAMP_TOP + Vec3::X * lane(released), Vec3::Z * 1.5);
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
