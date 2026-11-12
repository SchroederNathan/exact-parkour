use exact_game::*;
use exact_game_physics::Collider;

use crate::body;
use crate::park;

#[derive(Clone, Copy)]
pub struct RaceBlock {
    pub name: &'static str,
    pub at: [f32; 3],
    pub size: [f32; 3],
    pub color: &'static str,
    pub yaw: f32,
}

#[derive(Clone, Copy)]
pub struct RaceDefinition {
    pub id: u8,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub theme: &'static str,
    pub start: [f32; 3],
    pub finish: [f32; 3],
    pub blocks: &'static [RaceBlock],
}

const SKYLINE_BLOCKS: &[RaceBlock] = &[
    RaceBlock {
        name: "skyline-step-1",
        at: [-1.5, 0.35, -5.0],
        size: [2.4, 0.7, 2.0],
        color: "#164f63",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-step-2",
        at: [2.0, 1.1, -8.0],
        size: [2.4, 1.8, 2.0],
        color: "#164f63",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-rail",
        at: [5.5, 1.35, -11.5],
        size: [5.0, 0.35, 0.45],
        color: "#6d3b20",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-tower",
        at: [5.5, 2.7, -15.5],
        size: [3.2, 5.4, 3.2],
        color: "#233d64",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-bridge",
        at: [1.5, 3.6, -20.5],
        size: [9.0, 0.45, 2.2],
        color: "#6d3b20",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-slide",
        at: [-5.0, 3.0, -22.5],
        size: [2.4, 1.1, 0.7],
        color: "#5c274e",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-ledge",
        at: [-9.5, 4.4, -25.0],
        size: [3.4, 3.0, 2.6],
        color: "#233d64",
        yaw: 0.0,
    },
];

const SWITCHBACK_BLOCKS: &[RaceBlock] = &[
    RaceBlock {
        name: "switchback-low",
        at: [0.0, 0.3, -5.5],
        size: [7.0, 0.6, 1.1],
        color: "#563047",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-turn",
        at: [5.0, 1.0, -9.0],
        size: [2.0, 2.0, 5.0],
        color: "#174c5c",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-high",
        at: [0.5, 2.0, -12.5],
        size: [7.0, 2.0, 1.1],
        color: "#174c5c",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-gap",
        at: [-5.0, 2.6, -16.5],
        size: [2.8, 1.2, 2.8],
        color: "#35275e",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-gap-2",
        at: [-10.5, 3.4, -16.5],
        size: [2.8, 2.8, 2.8],
        color: "#35275e",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-wall",
        at: [-14.5, 4.4, -12.0],
        size: [1.0, 4.0, 7.0],
        color: "#563047",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-landing",
        at: [-14.5, 5.6, -3.0],
        size: [5.0, 2.4, 3.0],
        color: "#174c5c",
        yaw: 0.0,
    },
];

const HELIX_BLOCKS: &[RaceBlock] = &[
    RaceBlock {
        name: "helix-pad",
        at: [0.0, 0.4, -4.0],
        size: [4.0, 0.8, 4.0],
        color: "#35275e",
        yaw: 0.0,
    },
    RaceBlock {
        name: "helix-east",
        at: [5.0, 1.3, -7.0],
        size: [5.0, 1.0, 2.0],
        color: "#164f63",
        yaw: -18.0,
    },
    RaceBlock {
        name: "helix-north",
        at: [7.0, 2.5, -12.0],
        size: [2.0, 1.2, 6.0],
        color: "#164f63",
        yaw: 0.0,
    },
    RaceBlock {
        name: "helix-west",
        at: [1.5, 3.5, -16.5],
        size: [9.0, 0.8, 2.0],
        color: "#164f63",
        yaw: 14.0,
    },
    RaceBlock {
        name: "helix-drop",
        at: [-5.0, 2.4, -19.5],
        size: [3.0, 2.0, 3.0],
        color: "#5c274e",
        yaw: 0.0,
    },
    RaceBlock {
        name: "helix-rail",
        at: [-8.5, 2.8, -23.0],
        size: [0.6, 1.8, 5.0],
        color: "#6d3b20",
        yaw: 0.0,
    },
    RaceBlock {
        name: "helix-finish",
        at: [-11.0, 4.0, -28.0],
        size: [5.0, 2.4, 3.0],
        color: "#35275e",
        yaw: 0.0,
    },
];

pub const ROUTES: [RaceDefinition; 3] = [
    RaceDefinition {
        id: 0,
        title: "Skyline Sprint",
        subtitle: "Rooftop stairs, a rail slide, and a high bridge",
        theme: "#00d7ed",
        start: [0.0, 0.0, 0.0],
        finish: [-9.5, 5.9, -25.0],
        blocks: SKYLINE_BLOCKS,
    },
    RaceDefinition {
        id: 1,
        title: "Switchback Run",
        subtitle: "Vault the spine, shimmy the wall, leap the split",
        theme: "#00e8c5",
        start: [0.0, 0.0, 0.0],
        finish: [-14.5, 6.9, -3.0],
        blocks: SWITCHBACK_BLOCKS,
    },
    RaceDefinition {
        id: 2,
        title: "Helix Heights",
        subtitle: "A climbing spiral with a fast final drop",
        theme: "#b34dff",
        start: [0.0, 0.0, 0.0],
        finish: [-11.0, 5.2, -28.0],
        blocks: HELIX_BLOCKS,
    },
];

#[derive(Default, Resource)]
pub struct RaceState {
    pub selected: u8,
    pub started: u64,
    pub finished: bool,
}

pub fn definition(id: u8) -> &'static RaceDefinition {
    &ROUTES[(id as usize).min(ROUTES.len() - 1)]
}

pub fn start_position(id: u8) -> Vec3 {
    let route = definition(id);
    Vec3::from_array(route.start)
}

fn route_offset(at: Vec3, yaw: f32, offset: Vec3) -> Vec3 {
    at + Quat::from_rotation_y(yaw) * offset
}

fn route_piece(
    w: &mut World,
    name: String,
    at: Vec3,
    yaw: f32,
    offset: Vec3,
    rotation: Quat,
    mesh: Mesh,
    material: Material,
) {
    w.spawn_named(
        name.as_str(),
        (
            Transform {
                position: route_offset(at, yaw, offset),
                rotation: Quat::from_rotation_y(yaw) * rotation,
                ..Transform::default()
            },
            mesh,
            material,
        ),
    );
}

fn spawn_route_dressing(w: &mut World, route: &RaceDefinition, signal: [f32; 3]) {
    for (index, block) in route.blocks.iter().enumerate() {
        let at = Vec3::from_array(block.at);
        let yaw = block.yaw.to_radians();
        let depth = block.size[2] * 0.5 + 0.05;
        let front = Vec3::new(0.0, block.size[1] * 0.2, -depth);
        route_piece(
            w,
            format!("{}-service-face", block.name),
            at,
            yaw,
            front,
            Quat::IDENTITY,
            Mesh::cuboid(Vec3::new((block.size[0] * 0.42).max(0.32), 0.28, 0.045)),
            Material::hex("#303a49").metallic(0.84).rough(0.28),
        );
        route_piece(
            w,
            format!("{}-service-strip", block.name),
            at,
            yaw,
            front + Vec3::new(0.0, 0.2, -0.035),
            Quat::IDENTITY,
            Mesh::cuboid(Vec3::new((block.size[0] * 0.34).max(0.24), 0.045, 0.018)),
            Material::glow(signal),
        );
        for (bolt, side) in [-1.0, 1.0].into_iter().enumerate() {
            route_piece(
                w,
                format!("{}-service-bolt-{bolt}", block.name),
                at,
                yaw,
                front + Vec3::new(side * (block.size[0] * 0.17).min(0.32), 0.0, -0.04),
                Quat::IDENTITY,
                Mesh::sphere(0.04),
                Material::hex("#b2beca").metallic(0.94).rough(0.18),
            );
        }
        if block.size[1] > 1.6 {
            for (rail, side) in [-1.0, 1.0].into_iter().enumerate() {
                route_piece(
                    w,
                    format!("{}-ladder-rail-{rail}", block.name),
                    at,
                    yaw,
                    Vec3::new(side * (block.size[0] * 0.32).min(0.65), block.size[1] * 0.5, -depth - 0.08),
                    Quat::IDENTITY,
                    Mesh::cuboid(Vec3::new(0.055, block.size[1] * 0.84, 0.055)),
                    Material::hex("#6c7785").metallic(0.86).rough(0.3),
                );
            }
            for rung in 0..4 {
                let y = -block.size[1] * 0.25 + rung as f32 * block.size[1] * 0.17;
                route_piece(
                    w,
                    format!("{}-ladder-rung-{rung}", block.name),
                    at,
                    yaw,
                    Vec3::new(0.0, y, -depth - 0.08),
                    Quat::IDENTITY,
                    Mesh::cuboid(Vec3::new((block.size[0] * 0.34).min(1.0), 0.045, 0.045)),
                    Material::hex("#6c7785").metallic(0.86).rough(0.3),
                );
            }
        }
        if index % 2 == 0 {
            route_piece(
                w,
                format!("{}-hazard-chevron", block.name),
                at,
                yaw,
                Vec3::new(block.size[0] * 0.28, 0.04, -block.size[2] * 0.52),
                Quat::from_rotation_z(0.2),
                Mesh::cuboid(Vec3::new(0.32, 0.12, 0.035)),
                Material::glow([5.5, 1.7, 0.08]),
            );
        }
    }

    let equipment: &[(Vec3, f32, f32, bool)] = match route.id {
        0 => &[
            (Vec3::new(-3.8, 0.22, -3.8), 0.0, 1.1, false),
            (Vec3::new(4.2, 0.28, -13.2), 0.4, 0.9, true),
            (Vec3::new(-3.6, 3.86, -20.7), -0.2, 0.85, false),
        ],
        1 => &[
            (Vec3::new(3.6, 0.22, -5.1), 0.2, 1.0, true),
            (Vec3::new(-7.6, 2.8, -15.0), -0.5, 0.8, false),
            (Vec3::new(-12.0, 5.0, -7.4), 0.0, 1.15, true),
        ],
        _ => &[
            (Vec3::new(2.2, 0.26, -3.0), -0.2, 1.0, false),
            (Vec3::new(5.7, 2.05, -10.0), 0.3, 0.9, true),
            (Vec3::new(-3.2, 3.95, -17.7), -0.2, 1.1, false),
        ],
    };
    for (index, (position, yaw, scale, lit)) in equipment.iter().enumerate() {
        let at = *position;
        let tone = if *lit { signal } else { [0.2, 0.35, 0.5] };
        route_piece(
            w,
            format!("route-equipment-{index}-body"),
            at,
            *yaw,
            Vec3::new(0.0, 0.38 * *scale, 0.0),
            Quat::IDENTITY,
            Mesh::cuboid(Vec3::new(1.2 * *scale, 0.76 * *scale, 0.86 * *scale)),
            Material::hex("#202a39").metallic(0.8).rough(0.34),
        );
        for fan in [-0.32, 0.32] {
            route_piece(
                w,
                format!("route-equipment-{index}-fan-{fan}"),
                at,
                *yaw,
                Vec3::new(fan * *scale, 0.82 * *scale, -0.02),
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                Mesh::cylinder(0.18 * *scale, 0.07),
                Material::hex("#697685").metallic(0.82).rough(0.25),
            );
            route_piece(
                w,
                format!("route-equipment-{index}-halo-{fan}"),
                at,
                *yaw,
                Vec3::new(fan * *scale, 0.87 * *scale, -0.02),
                Quat::IDENTITY,
                Mesh::torus(0.15 * *scale, 0.025),
                Material::glow(tone),
            );
        }
    }

    let debris = match route.id {
        0 => [Vec3::new(7.0, 0.08, -17.0), Vec3::new(-7.5, 4.0, -24.0)],
        1 => [Vec3::new(-2.5, 0.08, -14.0), Vec3::new(-16.0, 5.0, -7.0)],
        _ => [Vec3::new(7.7, 0.08, -14.0), Vec3::new(-10.5, 3.0, -25.0)],
    };
    for (index, at) in debris.into_iter().enumerate() {
        for (piece, offset) in [Vec3::new(-0.3, 0.08, 0.0), Vec3::new(0.28, 0.12, 0.18), Vec3::new(0.0, 0.07, -0.28)].into_iter().enumerate() {
            route_piece(
                w,
                format!("route-debris-{index}-{piece}"),
                at,
                index as f32 * 0.4,
                offset,
                Quat::from_rotation_y((piece as f32) * 0.7) * Quat::from_rotation_x(0.12 * piece as f32),
                Mesh::cuboid(Vec3::new(0.42 + piece as f32 * 0.12, 0.14 + piece as f32 * 0.06, 0.28 + piece as f32 * 0.11)),
                Material::hex(if piece == 1 { "#59404c" } else { "#3b4652" }).metallic(0.62).rough(0.48),
            );
        }
        route_piece(
            w,
            format!("route-debris-{index}-wet"),
            at,
            0.0,
            Vec3::new(0.0, 0.025, 0.0),
            Quat::IDENTITY,
            Mesh::ring(0.16, 0.7),
            Material::glow([signal[0] * 0.13, signal[1] * 0.13, signal[2] * 0.13]).alpha(0.42),
        );
    }
}

pub fn setup(w: &mut World, id: u8) {
    let route = definition(id);
    let signal = match route.id {
        0 => [0.04, 4.8, 6.0],
        1 => [0.02, 5.6, 3.8],
        _ => [3.3, 0.08, 6.4],
    };
    for block in route.blocks {
        let mesh = Mesh::cuboid(Vec3::from_array(block.size));
        let rotation = Quat::from_rotation_y(block.yaw.to_radians());
        w.spawn_named(
            block.name,
            (
                Transform {
                    position: Vec3::from_array(block.at),
                    rotation,
                    ..Transform::default()
                },
                Collider {
                    layer: park::SOLID,
                    ..Collider::of(&mesh)
                },
                mesh,
                Material::hex(block.color),
            ),
        );
        w.spawn_named(
            format!("{}-signal", block.name),
            (
                Transform {
                    position: Vec3::new(
                        block.at[0],
                        block.at[1] + block.size[1] * 0.5 + 0.035,
                        block.at[2] - block.size[2] * 0.44,
                    ),
                    rotation,
                    ..Transform::default()
                },
                Mesh::cuboid(Vec3::new(block.size[0] * 0.84, 0.055, 0.07)),
                Material::glow(signal),
            ),
        );
    }
    spawn_route_dressing(w, route, signal);
    let finish = Vec3::from_array(route.finish);
    w.spawn_named(
        "race-finish",
        (
            Transform::at(finish.x, finish.y - body::HALF, finish.z),
            Mesh::torus(1.2, 0.12),
            Material::hex(route.theme),
        ),
    );
    w.insert_resource(RaceState {
        selected: route.id,
        started: w.tick(),
        finished: false,
    });
}

pub fn tick(w: &mut World) {
    let route = definition(w.resource::<RaceState>().selected);
    let player = w.require::<Transform>("player").position;
    let finish = Vec3::from_array(route.finish);
    let mut state = w.resource_mut::<RaceState>();
    let newly_finished = !state.finished && (player - finish).length() < 1.8;
    if newly_finished {
        state.finished = true;
    }
    let finished = state.finished;
    let started = state.started;
    drop(state);
    if newly_finished {
        w.emit("race-finished");
    }
    let seconds = (w.tick().saturating_sub(started) as f32 / w.hz() as f32).min(999.9);
    let (move_x, move_y) = {
        let stick = w.resource::<crate::TouchStick>();
        (stick.x, stick.y)
    };
    w.publish_record(&crate::Hud {
        mode: "race".into(),
        race: route.title.into(),
        subtitle: route.subtitle.into(),
        seconds,
        finished,
        round: 0,
        enemies: 0,
        hits: 0,
        phase: "racing".into(),
        move_x,
        move_y,
    });
}
