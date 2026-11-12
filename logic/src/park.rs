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

#[derive(Clone, Copy)]
enum RooftopProp {
    AirHandler,
    CrateStack,
    BarrelCluster,
    PipeRack,
    CableSpool,
    Debris,
}

#[derive(Clone, Copy)]
struct PropPlacement {
    name: &'static str,
    at: [f32; 3],
    yaw: f32,
    accent: [f32; 3],
    kind: RooftopProp,
}

const ROOFTOP_PROPS: &[PropPlacement] = &[
    PropPlacement {
        name: "prop-crates-gate",
        at: [-4.1, 0.42, -1.2],
        yaw: 0.12,
        accent: NEON_AMBER,
        kind: RooftopProp::CrateStack,
    },
    PropPlacement {
        name: "prop-pipes-gate",
        at: [4.3, 0.25, -1.8],
        yaw: 1.57,
        accent: NEON_CYAN,
        kind: RooftopProp::PipeRack,
    },
    PropPlacement {
        name: "prop-barrels-gate",
        at: [-5.2, 0.44, 2.1],
        yaw: -0.18,
        accent: NEON_PINK,
        kind: RooftopProp::BarrelCluster,
    },
    PropPlacement {
        name: "prop-air-handler-gate",
        at: [5.6, 0.72, 2.4],
        yaw: -0.28,
        accent: NEON_VIOLET,
        kind: RooftopProp::AirHandler,
    },
    PropPlacement { name: "prop-air-handler-east", at: [10.5, 0.72, 5.3], yaw: 0.18, accent: NEON_CYAN, kind: RooftopProp::AirHandler },
    PropPlacement { name: "prop-air-handler-west", at: [-10.5, 0.72, 3.2], yaw: -0.24, accent: NEON_PINK, kind: RooftopProp::AirHandler },
    PropPlacement { name: "prop-crates-north", at: [11.5, 0.45, -2.6], yaw: -0.12, accent: NEON_AMBER, kind: RooftopProp::CrateStack },
    PropPlacement { name: "prop-crates-south", at: [-10.5, 0.45, 10.8], yaw: 0.32, accent: NEON_VIOLET, kind: RooftopProp::CrateStack },
    PropPlacement { name: "prop-barrels-west", at: [-22.0, 0.42, 5.4], yaw: 0.0, accent: NEON_PINK, kind: RooftopProp::BarrelCluster },
    PropPlacement { name: "prop-barrels-east", at: [21.0, 0.42, -3.8], yaw: 0.0, accent: NEON_CYAN, kind: RooftopProp::BarrelCluster },
    PropPlacement { name: "prop-pipes-north", at: [-5.5, 0.28, -2.0], yaw: 0.0, accent: NEON_AMBER, kind: RooftopProp::PipeRack },
    PropPlacement { name: "prop-pipes-south", at: [4.0, 0.28, 11.5], yaw: 1.57, accent: NEON_VIOLET, kind: RooftopProp::PipeRack },
    PropPlacement { name: "prop-spool-east", at: [19.0, 0.48, 9.5], yaw: 0.25, accent: NEON_CYAN, kind: RooftopProp::CableSpool },
    PropPlacement { name: "prop-spool-west", at: [-19.5, 0.48, -3.0], yaw: -0.2, accent: NEON_PINK, kind: RooftopProp::CableSpool },
    PropPlacement { name: "prop-debris-north", at: [8.3, 0.14, -17.0], yaw: 0.4, accent: NEON_CYAN, kind: RooftopProp::Debris },
    PropPlacement { name: "prop-debris-south", at: [-7.0, 0.14, 16.5], yaw: -0.35, accent: NEON_AMBER, kind: RooftopProp::Debris },
];

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
    let mut tower_material = Material::hex(CITY_BODY).metallic(0.82).rough(0.24);
    tower_material.grid_spacing = 1.6;
    w.spawn_named(
        tower.name,
        (
            Transform::at(x, y, z),
            Mesh::cuboid(Vec3::new(width, height, depth)),
            tower_material,
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

fn local_offset(origin: Vec3, yaw: f32, offset: Vec3) -> Vec3 {
    origin + Quat::from_rotation_y(yaw) * offset
}

fn prop_transform(origin: Vec3, yaw: f32, offset: Vec3, rotation: Quat) -> Transform {
    Transform {
        position: local_offset(origin, yaw, offset),
        rotation: Quat::from_rotation_y(yaw) * rotation,
        ..Transform::default()
    }
}

fn prop_piece(
    w: &mut World,
    name: String,
    origin: Vec3,
    yaw: f32,
    offset: Vec3,
    mesh: Mesh,
    material: Material,
) {
    w.spawn_named(name.as_str(), (prop_transform(origin, yaw, offset, Quat::IDENTITY), mesh, material));
}

fn prop_piece_rotated(
    w: &mut World,
    name: String,
    origin: Vec3,
    yaw: f32,
    offset: Vec3,
    rotation: Quat,
    mesh: Mesh,
    material: Material,
) {
    w.spawn_named(name.as_str(), (prop_transform(origin, yaw, offset, rotation), mesh, material));
}

fn spawn_rooftop_prop(w: &mut World, placement: &PropPlacement) {
    let origin = Vec3::from_array(placement.at);
    let yaw = placement.yaw;
    let accent = placement.accent;
    let steel = Material::hex(CITY_EDGE).metallic(0.88).rough(0.24);
    let dark = Material::hex(CITY_BODY).metallic(0.72).rough(0.36);
    let rubber = Material::hex("#141823").metallic(0.15).rough(0.72);
    match placement.kind {
        RooftopProp::AirHandler => {
            prop_piece(w, format!("{}-base", placement.name), origin, yaw, Vec3::new(0.0, 0.5, 0.0), Mesh::cuboid(Vec3::new(2.6, 1.0, 1.8)), dark.clone());
            prop_piece(w, format!("{}-top", placement.name), origin, yaw, Vec3::new(0.0, 1.08, 0.0), Mesh::cuboid(Vec3::new(2.25, 0.12, 1.48)), steel.clone());
            for (index, x) in [-0.78, 0.0, 0.78].into_iter().enumerate() {
                prop_piece_rotated(w, format!("{}-fan-{index}", placement.name), origin, yaw, Vec3::new(x, 1.2, -0.01), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2), Mesh::cylinder(0.33, 0.1), steel.clone());
                prop_piece(w, format!("{}-fan-ring-{index}", placement.name), origin, yaw, Vec3::new(x, 1.27, -0.01), Mesh::torus(0.27, 0.035), accent_material(accent));
                for fin in [-0.18, 0.0, 0.18] {
                    prop_piece(w, format!("{}-fin-{index}-{fin}", placement.name), origin, yaw, Vec3::new(x + fin, 0.98, 0.77), Mesh::cuboid(Vec3::new(0.055, 0.64, 0.06)), steel.clone());
                }
            }
            prop_piece_rotated(w, format!("{}-duct", placement.name), origin, yaw, Vec3::new(-1.4, 0.82, 0.35), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), Mesh::cylinder(0.15, 1.6), steel.clone());
            prop_piece(w, format!("{}-warning", placement.name), origin, yaw, Vec3::new(0.0, 0.63, -0.93), Mesh::cuboid(Vec3::new(1.0, 0.16, 0.035)), accent_material(NEON_AMBER));
        }
        RooftopProp::CrateStack => {
            for (index, offset) in [Vec3::new(-0.58, 0.34, 0.0), Vec3::new(0.54, 0.34, 0.1), Vec3::new(-0.04, 1.0, -0.05)].into_iter().enumerate() {
                prop_piece_rotated(w, format!("{}-crate-{index}", placement.name), origin, yaw, offset, Quat::from_rotation_y((index as f32 - 1.0) * 0.11), Mesh::cuboid(Vec3::new(1.0, 0.68, 0.86)), Material::hex("#3a4658").metallic(0.62).rough(0.4));
                prop_piece(w, format!("{}-crate-band-{index}", placement.name), origin, yaw, offset + Vec3::new(0.0, 0.02, -0.44), Mesh::cuboid(Vec3::new(0.8, 0.15, 0.035)), accent_material(accent));
                for bolt_x in [-0.36, 0.36] {
                    prop_piece(w, format!("{}-bolt-{index}-{bolt_x}", placement.name), origin, yaw, offset + Vec3::new(bolt_x, 0.1, -0.47), Mesh::sphere(0.045), steel.clone());
                }
            }
        }
        RooftopProp::BarrelCluster => {
            for (index, offset) in [Vec3::new(-0.58, 0.48, 0.0), Vec3::new(0.55, 0.48, 0.08), Vec3::new(0.0, 0.48, 0.78)].into_iter().enumerate() {
                prop_piece(w, format!("{}-drum-{index}", placement.name), origin, yaw, offset, Mesh::cylinder(0.43, 0.94), Material::hex("#36475a").metallic(0.75).rough(0.32));
                for band_y in [0.22, 0.72] {
                    prop_piece(w, format!("{}-band-{index}-{band_y}", placement.name), origin, yaw, offset + Vec3::new(0.0, band_y - 0.47, 0.0), Mesh::torus(0.44, 0.035), steel.clone());
                }
                prop_piece(w, format!("{}-lid-{index}", placement.name), origin, yaw, offset + Vec3::new(0.0, 0.49, 0.0), Mesh::ring(0.1, 0.34), accent_material(accent));
            }
            prop_piece(w, format!("{}-spill", placement.name), origin, yaw, Vec3::new(0.06, 0.035, 0.12), Mesh::ring(0.2, 0.62), Material::glow([accent[0] * 0.28, accent[1] * 0.28, accent[2] * 0.28]).alpha(0.55));
        }
        RooftopProp::PipeRack => {
            for (index, _x) in [-0.72, 0.0, 0.72].into_iter().enumerate() {
                prop_piece_rotated(w, format!("{}-pipe-{index}", placement.name), origin, yaw, Vec3::new(0.0, 0.46 + index as f32 * 0.22, 0.0), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2), Mesh::cylinder(0.12, 2.4), steel.clone());
                prop_piece(w, format!("{}-pipe-cap-{index}", placement.name), origin, yaw, Vec3::new(-1.2, 0.46 + index as f32 * 0.22, 0.0), Mesh::torus(0.13, 0.03), accent_material(accent));
                prop_piece(w, format!("{}-pipe-tag-{index}", placement.name), origin, yaw, Vec3::new(0.22, 0.46 + index as f32 * 0.22, -0.14), Mesh::cuboid(Vec3::new(0.32, 0.12, 0.035)), accent_material(if index == 1 { NEON_AMBER } else { accent }));
                for side in [-1.0, 1.0] {
                    prop_piece(w, format!("{}-post-{index}-{side}", placement.name), origin, yaw, Vec3::new(side * 0.98, 0.34, 0.0), Mesh::cuboid(Vec3::new(0.1, 0.68, 0.1)), steel.clone());
                }
            }
        }
        RooftopProp::CableSpool => {
            prop_piece_rotated(w, format!("{}-drum", placement.name), origin, yaw, Vec3::new(0.0, 0.58, 0.0), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), Mesh::cylinder(0.53, 0.72), rubber.clone());
            for side in [-0.39, 0.39] {
                prop_piece_rotated(w, format!("{}-flange-{side}", placement.name), origin, yaw, Vec3::new(side, 0.58, 0.0), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), Mesh::cylinder(0.74, 0.1), steel.clone());
                prop_piece_rotated(w, format!("{}-rim-{side}", placement.name), origin, yaw, Vec3::new(side, 0.58, 0.0), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), Mesh::torus(0.66, 0.055), accent_material(accent));
            }
            for (index, offset) in [Vec3::new(0.0, 0.03, -0.82), Vec3::new(0.65, 0.035, -0.58), Vec3::new(-0.74, 0.035, -0.34)].into_iter().enumerate() {
                prop_piece(w, format!("{}-loose-cable-{index}", placement.name), origin, yaw, offset, Mesh::torus(0.35 + index as f32 * 0.06, 0.028), accent_material(accent));
            }
        }
        RooftopProp::Debris => {
            for (index, (offset, size)) in [
                (Vec3::new(-0.72, 0.11, 0.0), Vec3::new(0.9, 0.22, 0.45)),
                (Vec3::new(0.0, 0.17, 0.38), Vec3::new(0.52, 0.34, 0.62)),
                (Vec3::new(0.68, 0.08, -0.28), Vec3::new(0.72, 0.16, 0.32)),
                (Vec3::new(0.26, 0.08, -0.72), Vec3::new(0.3, 0.16, 0.86)),
            ].into_iter().enumerate() {
                prop_piece_rotated(w, format!("{}-scrap-{index}", placement.name), origin, yaw, offset, Quat::from_rotation_y(index as f32 * 0.7) * Quat::from_rotation_x(0.15 * index as f32), Mesh::cuboid(size), Material::hex(if index % 2 == 0 { "#38404d" } else { "#5a3b45" }).metallic(0.68).rough(0.48));
            }
            prop_piece(w, format!("{}-wet-patch", placement.name), origin, yaw, Vec3::new(0.0, 0.025, 0.0), Mesh::ring(0.18, 0.95), Material::glow([accent[0] * 0.16, accent[1] * 0.16, accent[2] * 0.16]).alpha(0.42));
        }
    }
}

const ROOFTOP_SURFACES: &[(&str, [f32; 3], [f32; 2], [f32; 3])] = &[
    ("service-yard-north", [-2.0, 0.035, -1.5], [11.0, 4.5], NEON_CYAN),
    ("service-yard-south", [3.0, 0.035, 10.0], [10.0, 4.0], NEON_VIOLET),
    ("service-yard-west", [-18.0, 0.035, 2.0], [7.0, 8.0], NEON_PINK),
];

fn spawn_rooftop_surfaces(w: &mut World) {
    for (name, at, size, accent) in ROOFTOP_SURFACES {
        let [x, y, z] = *at;
        let [width, depth] = *size;
        let mut deck_material = Material::hex("#1b2230").metallic(0.58).rough(0.5);
        deck_material.grid_spacing = 0.9;
        w.spawn_named(
            *name,
            (
                Transform::at(x, y, z),
                Mesh::cuboid(Vec3::new(width, 0.06, depth)),
                deck_material,
            ),
        );
        for (index, seam) in [-0.42, 0.0, 0.42].into_iter().enumerate() {
            w.spawn_named(
                format!("{name}-seam-x-{index}"),
                (
                    Transform::at(x + seam * width, y + 0.045, z),
                    Mesh::cuboid(Vec3::new(0.035, 0.018, depth * 0.92)),
                    Material::hex(CITY_EDGE).metallic(0.8).rough(0.24),
                ),
            );
        }
        for (index, seam) in [-0.36, 0.0, 0.36].into_iter().enumerate() {
            w.spawn_named(
                format!("{name}-seam-z-{index}"),
                (
                    Transform::at(x, y + 0.047, z + seam * depth),
                    Mesh::cuboid(Vec3::new(width * 0.92, 0.018, 0.035)),
                    Material::hex(CITY_EDGE).metallic(0.8).rough(0.24),
                ),
            );
        }
        for (index, ox) in [-0.32, 0.32].into_iter().enumerate() {
            w.spawn_named(
                format!("{name}-drain-{index}"),
                (
                    Transform::at(x + ox * width, y + 0.06, z + depth * 0.25),
                    Mesh::ring(0.17, 0.34),
                    Material::hex("#0b0e16").metallic(0.9).rough(0.2),
                ),
            );
            for rib in [-0.1, 0.0, 0.1] {
                w.spawn_named(
                    format!("{name}-drain-rib-{index}-{rib}"),
                    (
                        Transform::at(x + ox * width + rib, y + 0.066, z + depth * 0.25),
                        Mesh::cuboid(Vec3::new(0.025, 0.012, 0.48)),
                        Material::hex(CITY_EDGE).metallic(0.85).rough(0.24),
                    ),
                );
            }
        }
        w.spawn_named(
            format!("{name}-puddle"),
            (
                Transform::at(x + width * 0.18, y + 0.061, z - depth * 0.18),
                Mesh::ring(0.24, 1.15),
                Material::glow([accent[0] * 0.11, accent[1] * 0.11, accent[2] * 0.11]).alpha(0.5),
            ),
        );
    }
}

fn spawn_obstacle_dressing(w: &mut World) {
    for block in BLOCKS.iter().filter(|b| b.name.starts_with("vault") || b.name.starts_with("ledge")) {
        let yaw = block.yaw.to_radians();
        let front = local_offset(Vec3::from_array(block.at), yaw, Vec3::new(0.0, 0.0, -block.size[2] * 0.5 - 0.045));
        let tone = if block.name.starts_with("vault") { NEON_PINK } else { NEON_CYAN };
        w.spawn_named(
            format!("{}-access-panel", block.name),
            (
                Transform { position: front + Vec3::Y * (block.size[1] * 0.24), rotation: Quat::from_rotation_y(yaw), ..Transform::default() },
                Mesh::cuboid(Vec3::new(block.size[0] * 0.32, 0.22, 0.035)),
                Material::hex("#3a4350").metallic(0.82).rough(0.28),
            ),
        );
        for bolt in [-0.12, 0.12] {
            w.spawn_named(
                format!("{}-access-bolt-{bolt}", block.name),
                (
                    Transform { position: front + Vec3::Y * (block.size[1] * 0.24) + Quat::from_rotation_y(yaw) * Vec3::new(bolt, 0.0, -0.028), rotation: Quat::from_rotation_y(yaw), ..Transform::default() },
                    Mesh::sphere(0.035),
                    Material::glow(tone),
                ),
            );
        }
    }
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
    for placement in ROOFTOP_PROPS {
        spawn_rooftop_prop(w, placement);
    }
    spawn_rooftop_surfaces(w);
    spawn_obstacle_dressing(w);
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
