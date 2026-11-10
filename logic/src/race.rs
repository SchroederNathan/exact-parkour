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
        color: "#2d8192",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-step-2",
        at: [2.0, 1.1, -8.0],
        size: [2.4, 1.8, 2.0],
        color: "#2d8192",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-rail",
        at: [5.5, 1.35, -11.5],
        size: [5.0, 0.35, 0.45],
        color: "#e1a44b",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-tower",
        at: [5.5, 2.7, -15.5],
        size: [3.2, 5.4, 3.2],
        color: "#536d9a",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-bridge",
        at: [1.5, 3.6, -20.5],
        size: [9.0, 0.45, 2.2],
        color: "#e1a44b",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-slide",
        at: [-5.0, 3.0, -22.5],
        size: [2.4, 1.1, 0.7],
        color: "#d56b4c",
        yaw: 0.0,
    },
    RaceBlock {
        name: "skyline-ledge",
        at: [-9.5, 4.4, -25.0],
        size: [3.4, 3.0, 2.6],
        color: "#536d9a",
        yaw: 0.0,
    },
];

const SWITCHBACK_BLOCKS: &[RaceBlock] = &[
    RaceBlock {
        name: "switchback-low",
        at: [0.0, 0.3, -5.5],
        size: [7.0, 0.6, 1.1],
        color: "#af6d3d",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-turn",
        at: [5.0, 1.0, -9.0],
        size: [2.0, 2.0, 5.0],
        color: "#438d86",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-high",
        at: [0.5, 2.0, -12.5],
        size: [7.0, 2.0, 1.1],
        color: "#438d86",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-gap",
        at: [-5.0, 2.6, -16.5],
        size: [2.8, 1.2, 2.8],
        color: "#755fa1",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-gap-2",
        at: [-10.5, 3.4, -16.5],
        size: [2.8, 2.8, 2.8],
        color: "#755fa1",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-wall",
        at: [-14.5, 4.4, -12.0],
        size: [1.0, 4.0, 7.0],
        color: "#af6d3d",
        yaw: 0.0,
    },
    RaceBlock {
        name: "switchback-landing",
        at: [-14.5, 5.6, -3.0],
        size: [5.0, 2.4, 3.0],
        color: "#438d86",
        yaw: 0.0,
    },
];

const HELIX_BLOCKS: &[RaceBlock] = &[
    RaceBlock {
        name: "helix-pad",
        at: [0.0, 0.4, -4.0],
        size: [4.0, 0.8, 4.0],
        color: "#6a5d9a",
        yaw: 0.0,
    },
    RaceBlock {
        name: "helix-east",
        at: [5.0, 1.3, -7.0],
        size: [5.0, 1.0, 2.0],
        color: "#3b8792",
        yaw: -18.0,
    },
    RaceBlock {
        name: "helix-north",
        at: [7.0, 2.5, -12.0],
        size: [2.0, 1.2, 6.0],
        color: "#3b8792",
        yaw: 0.0,
    },
    RaceBlock {
        name: "helix-west",
        at: [1.5, 3.5, -16.5],
        size: [9.0, 0.8, 2.0],
        color: "#3b8792",
        yaw: 14.0,
    },
    RaceBlock {
        name: "helix-drop",
        at: [-5.0, 2.4, -19.5],
        size: [3.0, 2.0, 3.0],
        color: "#b56c42",
        yaw: 0.0,
    },
    RaceBlock {
        name: "helix-rail",
        at: [-8.5, 2.8, -23.0],
        size: [0.6, 1.8, 5.0],
        color: "#e1a44b",
        yaw: 0.0,
    },
    RaceBlock {
        name: "helix-finish",
        at: [-11.0, 4.0, -28.0],
        size: [5.0, 2.4, 3.0],
        color: "#6a5d9a",
        yaw: 0.0,
    },
];

pub const ROUTES: [RaceDefinition; 3] = [
    RaceDefinition {
        id: 0,
        title: "Skyline Sprint",
        subtitle: "Rooftop stairs, a rail slide, and a high bridge",
        theme: "#2d8192",
        start: [0.0, 0.0, 0.0],
        finish: [-9.5, 5.9, -25.0],
        blocks: SKYLINE_BLOCKS,
    },
    RaceDefinition {
        id: 1,
        title: "Switchback Run",
        subtitle: "Vault the spine, shimmy the wall, leap the split",
        theme: "#438d86",
        start: [0.0, 0.0, 0.0],
        finish: [-14.5, 6.9, -3.0],
        blocks: SWITCHBACK_BLOCKS,
    },
    RaceDefinition {
        id: 2,
        title: "Helix Heights",
        subtitle: "A climbing spiral with a fast final drop",
        theme: "#6a5d9a",
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

pub fn setup(w: &mut World, id: u8) {
    let route = definition(id);
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
    }
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
