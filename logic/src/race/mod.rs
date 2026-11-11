//! The races: a level is built from its pieces, and a runner goes from the start
//! through the checkpoints to the finish gate. A runner who falls off the roofs
//! starts again at the last checkpoint.
use exact_game::*;
use exact_game_physics::{CapsuleController, Collider};

use crate::body::{self, Parkour};
use crate::park;

pub mod cascade;
pub mod city;
pub mod helix;
pub mod kit;
pub mod skyline;
pub mod switchback;
pub mod yard;

use kit::{Checkpoint, Kind, Level, Look, Piece};

/// How many races the menus offer.
pub const COUNT: u8 = 4;
/// The id of the test yard: one roof, which a test builds on.
pub const YARD: u8 = 255;

/// The level with this id. An id past the last race is the test yard.
pub fn level(id: u8) -> Level {
    match id {
        0 => skyline::level(),
        1 => switchback::level(),
        2 => helix::level(),
        3 => cascade::level(),
        _ => yard::level(),
    }
}

/// How close the feet come to a checkpoint or the finish to reach it, metres on
/// the map, and how far above or below it they may be.
const REACH: f32 = 2.2;
const REACH_UP: f32 = 1.6;
/// Seconds the view stays on a fall before the runner starts again.
const FALL_HOLD: f32 = 0.35;

#[derive(Clone, Debug, Default, Resource)]
pub struct RaceState {
    pub selected: u8,
    pub started: u64,
    pub finished: bool,
    /// The tick the runner reached the finish.
    pub ended: u64,
    /// The checkpoint a fall returns to.
    pub checkpoint: u32,
    pub falls: u32,
    /// Ticks the runner has been under the floor.
    pub falling: u32,
}

/// The route of the race in play. The pieces are in the world; the tick needs
/// only the checkpoints and the finish.
#[derive(Clone, Debug, Default, Resource)]
pub struct Course {
    pub title: String,
    pub subtitle: String,
    pub checkpoints: Vec<[f32; 6]>,
    pub finish: Vec3,
}

impl Course {
    fn of(level: &Level) -> Self {
        Self {
            title: level.title.into(),
            subtitle: level.subtitle.into(),
            checkpoints: level
                .checkpoints
                .iter()
                .map(|c| [c.at[0], c.at[1], c.at[2], c.yaw(), c.floor, 0.0])
                .collect(),
            finish: Vec3::from_array(level.finish),
        }
    }
    fn checkpoint(&self, index: u32) -> (Vec3, f32, f32) {
        let c = self.checkpoints[(index as usize).min(self.checkpoints.len() - 1)];
        (Vec3::new(c[0], c[1], c[2]), c[3], c[4])
    }
}

/// Put one piece into the world.
pub fn spawn(w: &mut World, piece: &Piece, theme: &str) -> Entity {
    let size = Vec3::from_array(piece.size);
    let material = piece.look.material(theme);
    match piece.kind {
        // Scenery shares one mesh, so the renderer draws the far city in few calls.
        Kind::Scenery => w.spawn_named(
            &piece.name,
            (
                Transform {
                    scale: size,
                    ..piece.pose()
                },
                Mesh::cube(1.0),
                material,
            ),
        ),
        Kind::Unseen => w.spawn_named(
            &piece.name,
            (
                piece.pose(),
                Collider {
                    layer: park::SOLID,
                    ..Collider::of(&Mesh::cuboid(size))
                },
            ),
        ),
        Kind::Solid | Kind::Bar => {
            let mesh = Mesh::cuboid(size);
            let layer = if piece.kind == Kind::Bar {
                park::BAR
            } else {
                park::SOLID
            };
            w.spawn_named(
                &piece.name,
                (
                    piece.pose(),
                    Collider {
                        layer,
                        ..Collider::of(&mesh)
                    },
                    mesh,
                    material,
                ),
            )
        }
    }
}

/// Where the runner starts, at the feet, and the yaw the body faces.
pub fn start(id: u8) -> (Vec3, f32) {
    let level = level(id);
    let first = level.checkpoints.first().copied().unwrap_or(Checkpoint {
        at: [0.0; 3],
        toward: [0.0, -1.0],
        floor: 0.0,
    });
    (Vec3::from_array(first.at), first.yaw())
}

fn glow(theme: &str, gain: f32) -> Material {
    let [r, g, b] = Color::hex(theme).0;
    Material::rgb(r * 0.2, g * 0.2, b * 0.2).emissive(r * gain, g * gain, b * gain)
}

/// Build the level: its pieces, the far city, the checkpoint pads, the beacon
/// over the next checkpoint and the finish gate.
pub fn setup(w: &mut World, id: u8) {
    let level = level(id);
    for piece in &level.pieces {
        spawn(w, piece, level.theme);
    }
    for piece in city::around(&level) {
        spawn(w, &piece, level.theme);
    }
    for (index, checkpoint) in level.checkpoints.iter().enumerate().skip(1) {
        w.spawn_named(
            format!("race-checkpoint-{index}"),
            (
                Transform::at(checkpoint.at[0], checkpoint.at[1] + 0.03, checkpoint.at[2]),
                Mesh::cylinder(1.1, 0.05),
                Look::Accent.material(level.theme),
            ),
        );
    }
    let finish = Vec3::from_array(level.finish);
    // The gate stands across the way the runner comes from.
    let from = level
        .checkpoints
        .last()
        .map_or(finish + Vec3::Z, |c| Vec3::from_array(c.at));
    let yaw = math::atan2(from.x - finish.x, from.z - finish.z);
    w.spawn_named(
        "race-finish",
        (
            Transform {
                position: finish + Vec3::Y * 1.5,
                rotation: Quat::from_rotation_y(yaw)
                    * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ..Transform::default()
            },
            Mesh::torus(1.5, 0.12),
            glow(level.theme, 2.5),
        ),
    );
    w.spawn_named(
        "race-finish-pad",
        (
            Transform::at(finish.x, finish.y + 0.03, finish.z),
            Mesh::cylinder(1.5, 0.05),
            Look::Accent.material(level.theme),
        ),
    );
    // A thin column of light over where the runner goes next.
    w.spawn_named(
        "race-beacon",
        (
            Transform::default(),
            Mesh::cylinder(0.07, BEACON),
            glow(level.theme, 3.0),
        ),
    );
    w.insert_resource(Course::of(&level));
    w.insert_resource(RaceState {
        selected: id,
        started: w.tick(),
        ..RaceState::default()
    });
    aim_beacon(w);
}

const BEACON: f32 = 30.0;

// Stand the beacon over the next checkpoint, or over the finish after the last.
fn aim_beacon(w: &mut World) {
    let next = w.resource::<RaceState>().checkpoint + 1;
    let at = {
        let course = w.resource::<Course>();
        if (next as usize) < course.checkpoints.len() {
            course.checkpoint(next).0
        } else {
            course.finish
        }
    };
    w.require_mut::<Transform>("race-beacon").position = at + Vec3::Y * (BEACON / 2.0 + 3.2);
}

/// Stand the runner on a checkpoint, facing its way, with the view behind.
pub fn place(w: &mut World, checkpoint: u32) {
    let (at, yaw, _) = w.resource::<Course>().checkpoint(checkpoint);
    w.require_mut::<Transform>("player").position = at + Vec3::Y * (body::HALF + 0.02);
    w.require_mut::<Transform>("stickman").position = Vec3::new(0.0, -body::HALF - 0.01, 0.0);
    *w.require_mut::<Parkour>("player") = Parkour {
        yaw,
        ..Parkour::default()
    };
    crate::ragdoll::stop(w);
    {
        let mut c = w.require_mut::<CapsuleController>("player");
        c.velocity = Vec3::ZERO;
        c.grounded = false;
        c.support = None;
    }
    crate::camera::face(w, yaw);
}

/// Count the checkpoints, send a fallen runner back, and see the finish. Call
/// first in the tick, before the body moves.
pub fn tick(w: &mut World) {
    let feet = body::feet(w.require::<Transform>("player").position);
    let mut state = w.resource::<RaceState>().clone();
    let course = w.resource::<Course>().clone();
    let near = |at: Vec3| {
        (feet - at).xz().length() < REACH && (feet.y - at.y).abs() < REACH_UP
    };

    let mut reached = None;
    for index in (state.checkpoint as usize + 1)..course.checkpoints.len() {
        if near(course.checkpoint(index as u32).0) {
            reached = Some(index as u32);
        }
    }
    if let Some(index) = reached {
        state.checkpoint = index;
    }

    let (_, _, floor) = course.checkpoint(state.checkpoint);
    let mut fell = false;
    if feet.y < floor && !state.finished {
        state.falling += 1;
        if state.falling as f32 > FALL_HOLD * w.hz() as f32 {
            fell = true;
            state.falling = 0;
            state.falls += 1;
        }
    } else {
        state.falling = 0;
    }

    let newly_finished = !state.finished && near(course.finish);
    if newly_finished {
        state.finished = true;
        state.ended = w.tick();
    }
    let checkpoint = state.checkpoint;
    *w.resource_mut::<RaceState>() = state.clone();

    if fell {
        place(w, checkpoint);
        w.emit("race-fell");
    }
    if reached.is_some() {
        aim_beacon(w);
        w.emit("race-checkpoint");
    }
    if newly_finished {
        w.emit("race-finished");
    }

    let end = if state.finished { state.ended } else { w.tick() };
    let seconds = (end.saturating_sub(state.started) as f32 / w.hz() as f32).min(999.9);
    let (move_x, move_y) = {
        let stick = w.resource::<crate::TouchStick>();
        (stick.x, stick.y)
    };
    w.publish_record(&crate::Hud {
        mode: "race".into(),
        race: course.title.clone(),
        subtitle: course.subtitle.clone(),
        seconds,
        finished: state.finished,
        checkpoint: state.checkpoint,
        checkpoints: course.checkpoints.len().saturating_sub(1) as u32,
        falls: state.falls,
        round: 0,
        enemies: 0,
        hits: 0,
        phase: "racing".into(),
        move_x,
        move_y,
    });
}
