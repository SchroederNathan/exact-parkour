//! The mannequin's skeleton, clips and Animator.
//!
//! Locomotion is one blend along ground speed: a still stance at zero, then the
//! family of gaits in `gait::PACES`.
use exact_game::animation::{Layer, Layers, Param};
use exact_game::asset::{Clip, Interpolation, Model, Track, TrackPath};
use crate::body::Act;
use crate::gait::{self, PACES};
use exact_game::rig::{Limb, Rig};
use exact_game::*;

/// Asset name of the generated model.
pub const MODEL: &str = "stick-man.model";
/// Seconds of the landing crouch.
pub const LAND_SECONDS: f32 = 0.3;
/// Index of the landing layer in the player's `Layers`.
pub const LAND_LAYER: usize = 1;
/// Seconds of the kneel out of a harder landing, and its layer.
pub const KNEEL_SECONDS: f32 = 0.55;
pub const KNEEL_LAYER: usize = 2;
/// The layer of the brace for a landing.
pub const BRACE_LAYER: usize = 3;

pub fn rig(height: f32) -> Rig {
    let h = height / 1.8;
    let v = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) * h;
    let (skin, shirt, dark, shoe) = (
        [0.96, 0.72, 0.38, 1.0],
        [0.12, 0.48, 0.72, 1.0],
        [0.07, 0.08, 0.12, 1.0],
        [0.12, 0.1, 0.08, 1.0],
    );
    let mut rig = Rig::new();
    rig.sides(8)
        .bone("root", None, Vec3::ZERO, Vec3::ZERO, [0.0, 0.0], shirt)
        .bone(
            "hips",
            Some("root"),
            v(0.0, 0.96, 0.0),
            v(0.0, 1.1, 0.0),
            [0.075 * h, 0.07 * h],
            dark,
        )
        .bone(
            "spine",
            Some("hips"),
            v(0.0, 1.1, 0.0),
            v(0.0, 1.32, 0.0),
            [0.045 * h, 0.04 * h],
            shirt,
        )
        .bone(
            "chest",
            Some("spine"),
            v(0.0, 1.32, 0.0),
            v(0.0, 1.48, 0.0),
            [0.05 * h, 0.045 * h],
            shirt,
        )
        .bone(
            "neck",
            Some("chest"),
            v(0.0, 1.48, 0.0),
            v(0.0, 1.62, 0.0),
            [0.028 * h, 0.026 * h],
            skin,
        )
        .bone(
            "head",
            Some("neck"),
            v(0.0, 1.68, 0.0),
            v(0.0, 1.68, 0.0),
            [0.11 * h, 0.11 * h],
            skin,
        );

    for (side, x) in [("l", 1.0), ("r", -1.0)] {
        let n = |bone: &str| format!("{bone}_{side}");
        rig.bone(
            &n("thigh"),
            Some("hips"),
            v(0.09 * x, 0.98, 0.0),
            v(0.09 * x, 0.55, 0.0),
            [0.044 * h, 0.034 * h],
            dark,
        )
        .bone(
            &n("shin"),
            Some(&n("thigh")),
            v(0.09 * x, 0.55, 0.0),
            v(0.09 * x, 0.1, 0.0),
            [0.034 * h, 0.024 * h],
            dark,
        )
        .bone(
            &n("foot"),
            Some(&n("shin")),
            v(0.09 * x, 0.1, -0.03),
            v(0.09 * x, 0.04, 0.16),
            [0.026 * h, 0.02 * h],
            shoe,
        )
        .bone(
            &n("upper_arm"),
            Some("chest"),
            v(0.19 * x, 1.44, 0.0),
            v(0.23 * x, 1.17, 0.0),
            [0.036 * h, 0.028 * h],
            shirt,
        )
        .bone(
            &n("forearm"),
            Some(&n("upper_arm")),
            v(0.23 * x, 1.17, 0.0),
            v(0.24 * x, 0.91, 0.02),
            [0.029 * h, 0.021 * h],
            skin,
        )
        .bone(
            &n("hand"),
            Some(&n("forearm")),
            v(0.24 * x, 0.91, 0.02),
            v(0.24 * x, 0.83, 0.03),
            [0.026 * h, 0.02 * h],
            skin,
        );
        let phase = if x > 0.0 { 0.0 } else { 0.5 };
        rig.limb(Limb::Leg, &[&n("thigh"), &n("shin"), &n("foot")], phase)
            .limb(
                Limb::Arm,
                &[&n("upper_arm"), &n("forearm"), &n("hand")],
                phase + 0.5,
            );
    }
    rig.spine(&["hips", "spine", "chest", "neck", "head"]);
    rig
}

fn node(rig: &Rig, name: &str) -> u32 {
    rig.bones()
        .iter()
        .position(|bone| bone.name == name)
        .unwrap_or_else(|| panic!("stick man has no bone `{name}`")) as u32
}

fn rotation(node: u32, times: &[f32], at: impl Fn(f32) -> Quat) -> Track {
    Track {
        node,
        path: TrackPath::Rotation,
        interpolation: Interpolation::Linear,
        times: times.to_vec(),
        values: times.iter().flat_map(|&t| at(t).to_array()).collect(),
    }
}

// The hips' rest translation under the root, lowered by `drop(t)` metres.
fn hips(rig: &Rig, times: &[f32], drop: impl Fn(f32) -> f32) -> Track {
    let rest = rig.bones()[node(rig, "hips") as usize].head;
    Track {
        node: node(rig, "hips"),
        path: TrackPath::Translation,
        interpolation: Interpolation::Linear,
        times: times.to_vec(),
        values: times
            .iter()
            .flat_map(|&t| (rest - Vec3::Y * drop(t)).to_array())
            .collect(),
    }
}

// A held pose `seconds` long: rotations about each named joint from rest.
fn held(rig: &Rig, name: &str, seconds: f32, joints: &[(&str, Quat)]) -> Clip {
    let times = [0.0, seconds];
    Clip {
        name: name.into(),
        tracks: joints
            .iter()
            .map(|&(bone, turn)| rotation(node(rig, bone), &times, |_| turn))
            .collect(),
        markers: vec![],
    }
}

// A leg bone hangs down, so a negative turn about X swings it forward; an arm's
// positive turn about Z lifts the left one outward and the right one inward.
fn x(angle: f32) -> Quat {
    Quat::from_rotation_x(angle)
}
fn out(side: f32, angle: f32, forward: f32) -> Quat {
    Quat::from_rotation_z(side * angle) * Quat::from_rotation_x(-forward)
}

// An additive 3 s loop: the chest rises and settles, the head nods against it,
// and the arms float out and back a moment after the chest moves them.
fn breath(rig: &Rig) -> Clip {
    const PERIOD: f32 = 3.0;
    let times: Vec<f32> = (0..=24).map(|i| PERIOD * i as f32 / 24.0).collect();
    let wave = |t: f32, late: f32| math::sin(std::f32::consts::TAU * (t / PERIOD - late));
    let mut tracks = vec![
        rotation(node(rig, "spine"), &times, |t| x(-0.025 * wave(t, 0.0))),
        rotation(node(rig, "chest"), &times, |t| x(-0.015 * wave(t, 0.0))),
        rotation(node(rig, "neck"), &times, |t| x(0.02 * wave(t, 0.05))),
    ];
    for (side, away) in [("l", 1.0), ("r", -1.0)] {
        let bone = |name: &str| node(rig, &format!("{name}_{side}"));
        tracks.push(rotation(bone("upper_arm"), &times, move |t| {
            out(away, 0.025 * wave(t, 0.08), 0.0)
        }));
        tracks.push(rotation(bone("forearm"), &times, move |t| {
            x(-0.03 * (0.5 + 0.5 * wave(t, 0.16)))
        }));
    }
    Clip {
        name: "breath".into(),
        tracks,
        markers: vec![],
    }
}

// The poses of a body in the air, by its vertical speed. `AIR` gives each its
// speed in m/s, fastest upward first. The legs are a stride throughout, left leg
// leading, so a landing is on the left foot and the run goes on from there.

// Leaving the ground: the push leg long behind, the lead knee driving up, the
// arms as in a stride.
fn launch(rig: &Rig) -> Clip {
    held(
        rig,
        "launch",
        1.0,
        &[
            ("spine", x(0.1)),
            ("thigh_l", x(-0.9)),
            ("shin_l", x(1.4)),
            ("thigh_r", x(0.5)),
            ("shin_r", x(0.15)),
            ("foot_r", x(0.8)),
            ("upper_arm_l", out(1.0, 0.3, -0.6)),
            ("forearm_l", x(-0.9)),
            ("upper_arm_r", out(-1.0, 0.25, 1.1)),
            ("forearm_r", x(-1.2)),
        ],
    )
}

// Rising: the lead knee high, the push leg folding in behind, the chest open.
fn rise(rig: &Rig) -> Clip {
    held(
        rig,
        "rise",
        1.0,
        &[
            ("spine", x(-0.06)),
            ("thigh_l", x(-0.95)),
            ("shin_l", x(1.35)),
            ("foot_l", x(0.2)),
            ("thigh_r", x(0.4)),
            ("shin_r", x(0.8)),
            ("foot_r", x(0.55)),
            ("upper_arm_l", out(1.0, 0.35, -0.5)),
            ("forearm_l", x(-0.7)),
            ("upper_arm_r", out(-1.0, 0.5, 0.9)),
            ("forearm_r", x(-1.2)),
        ],
    )
}

// The top of the jump: both knees up, the arms out to the sides.
fn float(rig: &Rig) -> Clip {
    held(
        rig,
        "float",
        1.0,
        &[
            ("spine", x(0.05)),
            ("thigh_l", x(-1.0)),
            ("shin_l", x(1.3)),
            ("thigh_r", x(-0.35)),
            ("shin_r", x(1.1)),
            ("foot_r", x(0.3)),
            ("upper_arm_l", out(1.0, 0.8, 0.25)),
            ("forearm_l", x(-0.6)),
            ("upper_arm_r", out(-1.0, 0.8, 0.25)),
            ("forearm_r", x(-0.6)),
        ],
    )
}

// Coming down: the lead leg opens toward the ground, the other trails.
fn fall(rig: &Rig) -> Clip {
    held(
        rig,
        "fall",
        1.0,
        &[
            ("spine", x(0.12)),
            ("thigh_l", x(-0.6)),
            ("shin_l", x(0.45)),
            ("thigh_r", x(0.15)),
            ("shin_r", x(0.7)),
            ("foot_r", x(0.4)),
            ("upper_arm_l", out(1.0, 0.7, -0.3)),
            ("forearm_l", x(-0.5)),
            ("upper_arm_r", out(-1.0, 0.6, 0.4)),
            ("forearm_r", x(-0.7)),
        ],
    )
}

// A real drop: both legs reach down, a little bent, and the arms go wide.
fn reach(rig: &Rig) -> Clip {
    held(
        rig,
        "reach",
        1.0,
        &[
            ("spine", x(0.2)),
            ("thigh_l", x(-0.5)),
            ("shin_l", x(0.5)),
            ("foot_l", x(0.25)),
            ("thigh_r", x(-0.05)),
            ("shin_r", x(0.55)),
            ("foot_r", x(0.25)),
            ("upper_arm_l", out(1.0, 1.2, 0.15)),
            ("forearm_l", x(-0.45)),
            ("upper_arm_r", out(-1.0, 1.2, 0.15)),
            ("forearm_r", x(-0.45)),
        ],
    )
}

// A fall too long to be easy: the knees come up, the chest goes forward, and the
// arms are thrown high and back.
fn plunge(rig: &Rig) -> Clip {
    held(
        rig,
        "plunge",
        1.0,
        &[
            ("spine", x(0.3)),
            ("neck", x(-0.25)),
            ("thigh_l", x(-0.75)),
            ("shin_l", x(1.0)),
            ("thigh_r", x(-0.3)),
            ("shin_r", x(0.85)),
            ("upper_arm_l", out(1.0, 1.6, -0.15)),
            ("forearm_l", x(-0.3)),
            ("upper_arm_r", out(-1.0, 1.6, -0.15)),
            ("forearm_r", x(-0.3)),
        ],
    )
}

/// The air poses and the vertical speed of each, m/s.
const AIR: [(f32, &str); 6] = [
    (-8.5, "plunge"),
    (-5.0, "reach"),
    (-2.5, "fall"),
    (0.0, "float"),
    (1.5, "rise"),
    (3.8, "launch"),
];

// An additive pose that the game dials in as the ground comes up: the knees and
// hips unlock and the feet level, so the legs are already bending when they
// touch. At its start it is nothing, at its end the whole brace.
fn brace(rig: &Rig) -> Clip {
    let times = [0.0, 1.0];
    let mut tracks = vec![rotation(node(rig, "spine"), &times, |t| x(0.12 * t))];
    for (side, away) in [("l", 1.0), ("r", -1.0)] {
        let bone = |name: &str| node(rig, &format!("{name}_{side}"));
        tracks.push(rotation(bone("thigh"), &times, |t| x(-0.3 * t)));
        tracks.push(rotation(bone("shin"), &times, |t| x(0.6 * t)));
        tracks.push(rotation(bone("foot"), &times, |t| x(-0.3 * t)));
        tracks.push(rotation(bone("upper_arm"), &times, move |t| out(away, -0.2 * t, 0.3 * t)));
    }
    Clip {
        name: "brace".into(),
        tracks,
        markers: vec![],
    }
}

// How far down a landing has the body at `u` of its length: it goes down from
// the first moment at the speed it arrived with, slows to its deepest at `deep`,
// and comes up slowly.
fn give(u: f32, deep: f32) -> f32 {
    if u < deep {
        let left = 1.0 - u / deep;
        1.0 - left * left
    } else {
        1.0 - smooth((u - deep) / (1.0 - deep))
    }
}

// How far the arms of a landing body are thrown forward at `u` of its length:
// back and down with the drop, then forward and up as the body rises.
fn swing(u: f32, deep: f32) -> f32 {
    let through = ((u - deep) / (1.0 - deep)).clamp(0.0, 1.0);
    -0.45 * give(u, deep) + 0.8 * math::sin(std::f32::consts::PI * through)
}

// An additive one-shot: the knees give on touchdown and push back up. The thigh,
// shin and foot turn by -a, 2a and -a, which keeps the foot flat under the hip
// while the hips come down by the height the bent leg lost.
fn land(rig: &Rig) -> Clip {
    const BEND: f32 = 0.5;
    let times: Vec<f32> = (0..=14).map(|i| LAND_SECONDS * i as f32 / 14.0).collect();
    let give = |t: f32| give(t / LAND_SECONDS, 0.25);
    let leg = {
        let bone = |name: &str| {
            let b = &rig.bones()[node(rig, name) as usize];
            (b.head - b.tail).length()
        };
        bone("thigh_l") + bone("shin_l")
    };
    let mut tracks = vec![
        hips(rig, &times, |t| leg * (1.0 - math::cos(BEND * give(t)))),
        rotation(node(rig, "spine"), &times, |t| x(0.22 * give(t))),
    ];
    for (side, away) in [("l", 1.0), ("r", -1.0)] {
        let bone = |name: &str| node(rig, &format!("{name}_{side}"));
        tracks.push(rotation(bone("thigh"), &times, |t| x(-BEND * give(t))));
        tracks.push(rotation(bone("shin"), &times, |t| x(2.0 * BEND * give(t))));
        tracks.push(rotation(bone("foot"), &times, |t| x(-BEND * give(t))));
        tracks.push(rotation(bone("upper_arm"), &times, move |t| {
            out(away, 0.25 * give(t), 0.5 * swing(t / LAND_SECONDS, 0.25))
        }));
        tracks.push(rotation(bone("forearm"), &times, |t| x(-0.3 * give(t))));
    }
    Clip {
        name: "land".into(),
        tracks,
        markers: vec![],
    }
}

// One moment of a move: how far through it is (0 to 1), how far the hips sit below a
// stand, and the turn of each joint that is not at rest.
struct Key {
    at: f32,
    drop: f32,
    joints: &'static [(&'static str, [f32; 3])],
}

// A turn from its angles about X, then Y, then Z of the turned bone.
fn turn([about_x, about_y, about_z]: [f32; 3]) -> Quat {
    Quat::from_rotation_z(about_z) * Quat::from_rotation_y(about_y) * Quat::from_rotation_x(about_x)
}

// A smooth curve through `values` at `times`, read at `t`: a cubic through each
// pair of neighbours whose slope at a key is the slope between the keys either
// side of it. A straight line from key to key would stop and turn at every key,
// which reads as a body moved from pose to pose.
fn through(times: &[f32], values: &[f32], t: f32) -> f32 {
    let last = times.len() - 1;
    let i = times.partition_point(|&k| k <= t).clamp(1, last);
    let slope = |k: usize| {
        let (a, b) = (k.saturating_sub(1), (k + 1).min(last));
        (values[b] - values[a]) / (times[b] - times[a])
    };
    let span = times[i] - times[i - 1];
    let u = ((t - times[i - 1]) / span).clamp(0.0, 1.0);
    let (u2, u3) = (u * u, u * u * u);
    (2.0 * u3 - 3.0 * u2 + 1.0) * values[i - 1]
        + (u3 - 2.0 * u2 + u) * span * slope(i - 1)
        + (-2.0 * u3 + 3.0 * u2) * values[i]
        + (u3 - u2) * span * slope(i)
}

// A clip `seconds` long through `keys`. The Animator holds it still and the game
// sets its phase, so a move's pose follows the move and not the clock. A joint
// absent from a key is at rest there. Between keys every angle follows a smooth
// curve, sampled `SAMPLES` times.
fn keyed(rig: &Rig, name: &str, seconds: f32, keys: &[Key]) -> Clip {
    const SAMPLES: usize = 48;
    let at: Vec<f32> = keys.iter().map(|k| k.at).collect();
    let times: Vec<f32> = (0..=SAMPLES)
        .map(|i| seconds * i as f32 / SAMPLES as f32)
        .collect();
    let mut bones: Vec<&str> = keys
        .iter()
        .flat_map(|k| k.joints.iter().map(|j| j.0))
        .collect();
    bones.sort_unstable();
    bones.dedup();
    let mut tracks: Vec<Track> = bones
        .into_iter()
        .map(|bone| {
            let angles: Vec<[f32; 3]> = keys
                .iter()
                .map(|k| k.joints.iter().find(|j| j.0 == bone).map_or([0.0; 3], |j| j.1))
                .collect();
            let axis = |n: usize| angles.iter().map(|a| a[n]).collect::<Vec<f32>>();
            let (x, y, z) = (axis(0), axis(1), axis(2));
            rotation(node(rig, bone), &times, |t| {
                let u = t / seconds;
                turn([through(&at, &x, u), through(&at, &y, u), through(&at, &z, u)])
            })
        })
        .collect();
    let drops: Vec<f32> = keys.iter().map(|k| k.drop).collect();
    tracks.push(hips(rig, &times, |t| through(&at, &drops, t / seconds)));
    Clip {
        name: name.into(),
        tracks,
        markers: vec![],
    }
}

// Hanging by the hands: arms straight up, legs loose.
const HANGING: &[(&str, [f32; 3])] = &[
    ("upper_arm_l", [-3.0, 0.0, 0.08]),
    ("upper_arm_r", [-3.0, 0.0, -0.08]),
    ("forearm_l", [-0.1, 0.0, 0.0]),
    ("forearm_r", [-0.1, 0.0, 0.0]),
    ("thigh_l", [-0.15, 0.0, 0.0]),
    ("thigh_r", [0.05, 0.0, 0.0]),
    ("shin_l", [0.35, 0.0, 0.0]),
    ("shin_r", [0.3, 0.0, 0.0]),
    ("foot_l", [0.4, 0.0, 0.0]),
    ("foot_r", [0.4, 0.0, 0.0]),
    ("neck", [-0.25, 0.0, 0.0]),
];

fn hang(rig: &Rig) -> Clip {
    let key = |at| Key {
        at,
        drop: 0.0,
        joints: HANGING,
    };
    keyed(rig, "hang", 1.0, &[key(0.0), key(1.0)])
}

// From the hang: pull up until the hips are at the edge, lean the chest in over
// the top, and stand up on it. The holds put the hands on the edge and step the
// feet onto the top; these keys are the trunk, and the limbs between holds.
fn climb(rig: &Rig) -> Clip {
    keyed(
        rig,
        "climb",
        1.0,
        &[
            Key {
                at: 0.0,
                drop: 0.0,
                joints: HANGING,
            },
            Key {
                at: 0.3,
                drop: 0.0,
                joints: &[
                    ("upper_arm_l", [-2.3, 0.0, 0.3]),
                    ("upper_arm_r", [-2.3, 0.0, -0.3]),
                    ("forearm_l", [-1.7, 0.0, 0.0]),
                    ("forearm_r", [-1.7, 0.0, 0.0]),
                    ("thigh_l", [-0.25, 0.0, 0.0]),
                    ("thigh_r", [-0.35, 0.0, 0.0]),
                    ("shin_l", [0.7, 0.0, 0.0]),
                    ("shin_r", [0.8, 0.0, 0.0]),
                    // The chest is already going in over the edge.
                    ("hips", [0.2, 0.0, 0.0]),
                    ("spine", [0.35, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.5,
                drop: 0.0,
                joints: &[
                    ("upper_arm_l", [0.3, 0.0, 0.25]),
                    ("upper_arm_r", [0.3, 0.0, -0.25]),
                    ("forearm_l", [-0.4, 0.0, 0.0]),
                    ("forearm_r", [-0.4, 0.0, 0.0]),
                    ("thigh_l", [-0.1, 0.0, 0.0]),
                    ("shin_l", [0.5, 0.0, 0.0]),
                    ("thigh_r", [-0.2, 0.0, 0.0]),
                    ("shin_r", [0.7, 0.0, 0.0]),
                    // Folded over the edge, the shoulders low enough to press on it.
                    ("hips", [0.4, 0.0, 0.0]),
                    ("spine", [0.5, 0.0, 0.0]),
                    ("neck", [-0.6, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.64,
                drop: 0.2,
                joints: &[
                    ("upper_arm_l", [-0.5, 0.0, 0.2]),
                    ("upper_arm_r", [-0.5, 0.0, -0.2]),
                    ("forearm_l", [-0.7, 0.0, 0.0]),
                    ("forearm_r", [-0.7, 0.0, 0.0]),
                    ("thigh_l", [-0.1, 0.0, 0.0]),
                    ("shin_l", [0.7, 0.0, 0.0]),
                    ("thigh_r", [-1.0, 0.0, 0.0]),
                    ("shin_r", [1.5, 0.0, 0.0]),
                    ("hips", [0.2, 0.0, 0.0]),
                    ("spine", [0.5, 0.0, 0.0]),
                    ("neck", [-0.5, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.8,
                drop: 0.12,
                joints: &[
                    ("upper_arm_l", [-0.3, 0.0, 0.1]),
                    ("upper_arm_r", [-0.3, 0.0, -0.1]),
                    ("forearm_l", [-0.5, 0.0, 0.0]),
                    ("forearm_r", [-0.5, 0.0, 0.0]),
                    ("thigh_l", [-0.5, 0.0, 0.0]),
                    ("thigh_r", [-0.5, 0.0, 0.0]),
                    ("shin_l", [1.0, 0.0, 0.0]),
                    ("shin_r", [1.0, 0.0, 0.0]),
                    ("spine", [0.3, 0.0, 0.0]),
                    ("neck", [-0.2, 0.0, 0.0]),
                ],
            },
            Key {
                at: 1.0,
                drop: 0.0,
                joints: &[],
            },
        ],
    )
}

// Over a low wall, leaning on the left hand: the body tips toward that hand until
// the shoulder is over it, the legs swing up and across on the other side, level
// with the hips, and come down ahead. The course carries the hips a forearm above
// the wall's top, and the hold puts the hand on it; these keys only say how the
// body is arranged around those two.
fn vault(rig: &Rig) -> Clip {
    keyed(
        rig,
        "vault",
        1.0,
        &[
            // The last stride, gathered: the hips low over a bent push leg, the chest
            // turned to the hand that reaches for the wall, the other arm back.
            Key {
                at: 0.0,
                drop: 0.1,
                joints: &[
                    ("spine", [0.35, 0.35, 0.0]),
                    ("neck", [-0.15, -0.2, 0.0]),
                    ("upper_arm_l", [-1.2, 0.0, 0.1]),
                    ("forearm_l", [-0.3, 0.0, 0.0]),
                    ("upper_arm_r", [0.7, 0.0, -0.3]),
                    ("forearm_r", [-0.9, 0.0, 0.0]),
                    ("thigh_l", [0.1, 0.0, 0.0]),
                    ("shin_l", [0.7, 0.0, 0.0]),
                    ("thigh_r", [-0.5, 0.0, -0.4]),
                    ("shin_r", [0.9, 0.0, 0.0]),
                ],
            },
            // The push: that leg long behind, on its toes, the lead knee up and out.
            Key {
                at: 0.1,
                drop: 0.0,
                joints: &[
                    ("hips", [0.15, 0.15, -0.15]),
                    ("spine", [0.3, 0.35, 0.0]),
                    ("neck", [-0.15, -0.2, 0.1]),
                    ("upper_arm_l", [-1.3, 0.0, 0.2]),
                    ("forearm_l", [-0.2, 0.0, 0.0]),
                    ("upper_arm_r", [0.6, 0.0, -0.4]),
                    ("forearm_r", [-0.7, 0.0, 0.0]),
                    ("thigh_l", [0.45, 0.0, 0.0]),
                    ("shin_l", [0.1, 0.0, 0.0]),
                    ("foot_l", [0.5, 0.0, 0.0]),
                    ("thigh_r", [-0.9, 0.0, -0.6]),
                    ("shin_r", [1.2, 0.0, 0.0]),
                ],
            },
            // Off the ground: the lead leg is up at the side, the push leg still trails,
            // the pelvis turns after the lead leg, and the free arm starts its swing.
            Key {
                at: 0.28,
                drop: 0.0,
                joints: &[
                    ("hips", [0.2, 0.3, -0.55]),
                    ("spine", [0.25, 0.45, -0.1]),
                    ("neck", [-0.1, -0.3, 0.5]),
                    ("upper_arm_l", [-0.7, 0.0, 0.3]),
                    ("forearm_l", [-0.15, 0.0, 0.0]),
                    ("upper_arm_r", [0.4, 0.0, -0.8]),
                    ("forearm_r", [-0.6, 0.0, 0.0]),
                    ("thigh_l", [0.1, 0.0, -0.6]),
                    ("shin_l", [0.9, 0.0, 0.0]),
                    ("thigh_r", [-0.5, 0.0, -1.1]),
                    ("shin_r", [0.3, 0.0, 0.0]),
                ],
            },
            // Over the top on the hand: the lead leg long across the wall, the other
            // tucked behind it, the free arm high, the head up and looking ahead.
            Key {
                at: 0.5,
                drop: 0.0,
                joints: &[
                    ("hips", [0.25, 0.15, -1.02]),
                    ("spine", [0.15, 0.1, -0.1]),
                    ("neck", [-0.3, -0.15, 0.7]),
                    ("upper_arm_l", [0.0, 0.0, 0.5]),
                    ("forearm_l", [-0.1, 0.0, 0.0]),
                    ("upper_arm_r", [-0.2, 0.0, -1.5]),
                    ("forearm_r", [-0.2, 0.0, 0.0]),
                    ("thigh_l", [-0.6, 0.0, -0.55]),
                    ("shin_l", [1.5, 0.0, 0.0]),
                    ("thigh_r", [-1.3, 0.0, -0.35]),
                    ("shin_r", [0.3, 0.0, 0.0]),
                ],
            },
            // Past it: the lead leg reaches down, the tucked knee drives through, the
            // hand that was on the wall folds in, and the free arm comes down.
            Key {
                at: 0.72,
                drop: 0.0,
                joints: &[
                    ("hips", [0.1, -0.15, -0.5]),
                    ("spine", [0.2, -0.3, -0.05]),
                    ("neck", [0.1, 0.15, 0.2]),
                    ("upper_arm_l", [0.5, 0.0, 0.3]),
                    ("forearm_l", [-0.8, 0.0, 0.0]),
                    ("upper_arm_r", [-0.7, 0.0, -0.8]),
                    ("forearm_r", [-0.4, 0.0, 0.0]),
                    ("thigh_l", [-1.1, 0.0, -0.3]),
                    ("shin_l", [1.5, 0.0, 0.0]),
                    ("thigh_r", [-0.7, 0.0, -0.1]),
                    ("shin_r", [0.55, 0.0, 0.0]),
                ],
            },
            // Into the run's stride as the left foot strikes, the hips a little
            // low, the arms opposite the legs.
            Key {
                at: 1.0,
                drop: 0.09,
                joints: &[
                    ("spine", [0.3, -0.1, 0.0]),
                    ("upper_arm_l", [-0.5, 0.0, 0.05]),
                    ("forearm_l", [-0.9, 0.0, 0.0]),
                    ("upper_arm_r", [0.4, 0.0, -0.15]),
                    ("forearm_r", [-0.9, 0.0, 0.0]),
                    ("thigh_l", [-0.55, 0.0, 0.0]),
                    ("shin_l", [0.45, 0.0, 0.0]),
                    ("thigh_r", [0.15, 0.0, 0.0]),
                    ("shin_r", [1.0, 0.0, 0.0]),
                ],
            },
        ],
    )
}

// A slide on the right foot and the left hand: the body sits back and to the
// left, low over the ground, the left leg long in front, the right arm out for
// balance. The holds put the foot, the heel and the hand on the ground; these
// keys are the trunk and the limbs between holds.
fn slide(rig: &Rig) -> Clip {
    keyed(
        rig,
        "slide",
        1.0,
        &[
            Key {
                at: 0.0,
                drop: 0.06,
                joints: &[
                    ("spine", [0.25, 0.0, 0.0]),
                    ("upper_arm_l", [0.5, 0.0, 0.2]),
                    ("forearm_l", [-0.6, 0.0, 0.0]),
                    ("upper_arm_r", [-0.6, 0.0, -0.2]),
                    ("forearm_r", [-0.9, 0.0, 0.0]),
                    ("thigh_l", [-0.35, 0.0, 0.0]),
                    ("shin_l", [1.1, 0.0, 0.0]),
                    ("thigh_r", [0.2, 0.0, 0.0]),
                    ("shin_r", [0.9, 0.0, 0.0]),
                ],
            },
            // The kick: the lead leg goes out long while the hips are still high.
            Key {
                at: 0.18,
                drop: 0.2,
                joints: &[
                    ("hips", [-0.25, 0.0, -0.08]),
                    ("spine", [0.3, 0.0, 0.0]),
                    ("upper_arm_l", [0.7, 0.0, 0.35]),
                    ("forearm_l", [-0.5, 0.0, 0.0]),
                    ("upper_arm_r", [-0.9, 0.0, -0.4]),
                    ("forearm_r", [-1.1, 0.0, 0.0]),
                    ("thigh_l", [-1.05, 0.0, 0.0]),
                    ("shin_l", [0.2, 0.0, 0.0]),
                    ("thigh_r", [-0.2, 0.0, 0.0]),
                    ("shin_r", [1.3, 0.0, 0.0]),
                ],
            },
            // Down: the hips drop a little past where they will ride, the trunk well back.
            Key {
                at: 0.3,
                drop: 0.74,
                joints: &[
                    ("hips", [-1.05, 0.0, -0.36]),
                    ("spine", [0.4, 0.1, 0.1]),
                    ("neck", [0.55, -0.1, 0.1]),
                    ("upper_arm_l", [0.9, 0.0, 0.5]),
                    ("forearm_l", [-0.5, 0.0, 0.0]),
                    ("upper_arm_r", [-0.5, 0.0, -1.0]),
                    ("forearm_r", [-1.2, 0.0, 0.0]),
                    ("thigh_l", [-0.7, 0.0, 0.0]),
                    ("shin_l", [0.2, 0.0, 0.0]),
                    ("thigh_r", [-1.3, 0.0, -0.1]),
                    ("shin_r", [1.9, 0.0, 0.0]),
                ],
            },
            // Riding: the trunk starts to come forward and the free arm swings out.
            Key {
                at: 0.5,
                drop: 0.7,
                joints: &[
                    ("hips", [-0.9, 0.0, -0.36]),
                    ("spine", [0.35, 0.15, 0.1]),
                    ("neck", [0.45, -0.1, 0.1]),
                    ("upper_arm_l", [0.9, 0.0, 0.5]),
                    ("forearm_l", [-0.5, 0.0, 0.0]),
                    ("upper_arm_r", [-0.2, 0.0, -1.2]),
                    ("forearm_r", [-0.9, 0.0, 0.0]),
                    ("thigh_l", [-0.65, 0.0, 0.0]),
                    ("shin_l", [0.15, 0.0, 0.0]),
                    ("thigh_r", [-1.3, 0.0, -0.1]),
                    ("shin_r", [1.9, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.76,
                drop: 0.52,
                joints: &[
                    ("hips", [-0.6, 0.0, -0.2]),
                    ("spine", [0.45, 0.05, 0.05]),
                    ("neck", [0.25, 0.0, 0.05]),
                    ("upper_arm_l", [0.7, 0.0, 0.45]),
                    ("forearm_l", [-0.5, 0.0, 0.0]),
                    ("upper_arm_r", [-0.9, 0.0, -0.5]),
                    ("forearm_r", [-1.2, 0.0, 0.0]),
                    ("thigh_l", [-0.7, 0.0, 0.0]),
                    ("shin_l", [0.4, 0.0, 0.0]),
                    ("thigh_r", [-1.3, 0.0, -0.1]),
                    ("shin_r", [1.9, 0.0, 0.0]),
                ],
            },
            // Up: the chest goes forward over the tucked foot before the hips rise.
            Key {
                at: 0.88,
                drop: 0.28,
                joints: &[
                    ("hips", [0.15, 0.0, -0.05]),
                    ("spine", [0.5, 0.0, 0.0]),
                    ("neck", [-0.25, 0.0, 0.0]),
                    ("upper_arm_l", [0.8, 0.0, 0.3]),
                    ("forearm_l", [-0.8, 0.0, 0.0]),
                    ("upper_arm_r", [-0.8, 0.0, -0.3]),
                    ("forearm_r", [-1.3, 0.0, 0.0]),
                    ("thigh_l", [-1.1, 0.0, 0.0]),
                    ("shin_l", [0.35, 0.0, 0.0]),
                    ("thigh_r", [-1.2, 0.0, -0.05]),
                    ("shin_r", [1.9, 0.0, 0.0]),
                ],
            },
            // The sprint's stride as the left foot strikes, where the run takes over.
            Key {
                at: 1.0,
                drop: 0.07,
                joints: &[
                    ("spine", [0.26, 0.0, 0.0]),
                    ("upper_arm_l", [0.75, 0.0, 0.1]),
                    ("forearm_l", [-1.35, 0.0, 0.0]),
                    ("upper_arm_r", [-0.57, 0.0, -0.1]),
                    ("forearm_r", [-1.8, 0.0, 0.0]),
                    ("thigh_l", [-0.55, 0.0, 0.0]),
                    ("shin_l", [0.45, 0.0, 0.0]),
                    ("thigh_r", [0.0, 0.0, 0.0]),
                    ("shin_r", [1.7, 0.0, 0.0]),
                ],
            },
        ],
    )
}

// An additive one-shot, deeper and longer than `land`: the body kneels into a
// landing its knees alone cannot take, a hand's width from the ground, the chest
// over the knees and the arms thrown forward, and comes up out of it. It plays
// over the run, so the legs go on striding under it.
fn kneel(rig: &Rig) -> Clip {
    const BEND: f32 = 0.8;
    const DEEP: f32 = 0.22;
    let times: Vec<f32> = (0..=24).map(|i| KNEEL_SECONDS * i as f32 / 24.0).collect();
    let give = |t: f32| give(t / KNEEL_SECONDS, DEEP);
    let leg = {
        let bone = |name: &str| {
            let b = &rig.bones()[node(rig, name) as usize];
            (b.head - b.tail).length()
        };
        bone("thigh_l") + bone("shin_l")
    };
    let mut tracks = vec![
        hips(rig, &times, |t| leg * (1.0 - math::cos(BEND * give(t)))),
        rotation(node(rig, "spine"), &times, |t| x(0.4 * give(t))),
        rotation(node(rig, "chest"), &times, |t| x(0.2 * give(t))),
        rotation(node(rig, "neck"), &times, |t| x(-0.4 * give(t))),
    ];
    // The left knee goes lower than the right: a kneel, not a squat. The arms go
    // down with the drop, the left a moment before the right, and are thrown
    // forward as the body comes up.
    for (side, away, deep, late) in [("l", 1.0, 1.05, 0.0), ("r", -1.0, 0.95, 0.03)] {
        let bone = |name: &str| node(rig, &format!("{name}_{side}"));
        let bend = BEND * deep;
        tracks.push(rotation(bone("thigh"), &times, move |t| x(-bend * give(t))));
        tracks.push(rotation(bone("shin"), &times, move |t| x(2.0 * bend * give(t))));
        tracks.push(rotation(bone("foot"), &times, move |t| x(-bend * give(t))));
        tracks.push(rotation(bone("upper_arm"), &times, move |t| {
            let u = ((t - late) / KNEEL_SECONDS).max(0.0);
            out(away, 0.3 * give(t), swing(u, DEEP))
        }));
        tracks.push(rotation(bone("forearm"), &times, move |t| {
            let u = ((t - late) / KNEEL_SECONDS).max(0.0);
            x(-0.25 * give(t) - 0.5 * swing(u, DEEP).max(0.0))
        }));
    }
    Clip {
        name: "kneel".into(),
        tracks,
        markers: vec![],
    }
}

// A hard landing with nowhere to roll: down to the hands, then up. The thigh, shin
// and foot turn by -a, 2a and -a, which keeps the foot flat under the hip while the
// hips come down by the height the bent leg lost.
fn crouch(rig: &Rig) -> Clip {
    let leg = {
        let bone = |name: &str| {
            let b = &rig.bones()[node(rig, name) as usize];
            (b.head - b.tail).length()
        };
        bone("thigh_l") + bone("shin_l")
    };
    // How deep the knees are bent through the move, 0 to 1: down fast, up slowly.
    let depth = |u: f32| {
        if u < 0.18 {
            smooth(u / 0.18)
        } else {
            1.0 - smooth((u - 0.3).max(0.0) / 0.7)
        }
    };
    const BEND: f32 = 1.15;
    let times: Vec<f32> = (0..=20).map(|i| i as f32 / 20.0).collect();
    let mut tracks = vec![
        hips(rig, &times, |u| leg * (1.0 - math::cos(BEND * depth(u)))),
        rotation(node(rig, "spine"), &times, |u| x(0.75 * depth(u))),
        rotation(node(rig, "neck"), &times, |u| x(-0.5 * depth(u))),
    ];
    for (side, away) in [("l", 1.0), ("r", -1.0)] {
        let bone = |name: &str| node(rig, &format!("{name}_{side}"));
        tracks.push(rotation(bone("thigh"), &times, |u| x(-BEND * depth(u))));
        tracks.push(rotation(bone("shin"), &times, |u| x(2.0 * BEND * depth(u))));
        tracks.push(rotation(bone("foot"), &times, |u| x(-BEND * depth(u))));
        // The hands go down to the ground in front of the feet.
        tracks.push(rotation(bone("upper_arm"), &times, move |u| {
            out(away, 0.25 * depth(u), 0.95 * depth(u))
        }));
        tracks.push(rotation(bone("forearm"), &times, |u| x(-0.2 - 0.25 * depth(u))));
    }
    Clip {
        name: "crouch".into(),
        tracks,
        markers: vec![],
    }
}

// A forward roll over the right shoulder: the hands go down first and take the
// ground, the head tucks under, the body curls into a ball that turns once about
// its middle and uncurls onto its feet, the left leg a moment before the right.
fn roll(rig: &Rig) -> Clip {
    let times: Vec<f32> = (0..=40).map(|i| i as f32 / 40.0).collect();
    // How tightly the body is curled, and how far it has turned.
    // The turn is done before the legs open, and they open slowly.
    let curl = |u: f32| smooth(u / 0.2) * (1.0 - smooth((u - 0.6) / 0.4));
    let spin = |u: f32| std::f32::consts::TAU * smooth((u - 0.06) / 0.62);
    // The arms reach for the ground before the body is a ball.
    let reach = |u: f32| smooth(u / 0.08) * (1.0 - smooth((u - 0.14) / 0.16));
    let rest = rig.bones()[node(rig, "hips") as usize].head;
    // The hips ride the rim of the ball: its middle is in front of the belly, and
    // lower while the body is upside down, when it rolls on its back.
    let hips_at = move |u: f32| {
        let over = math::sin(0.5 * spin(u));
        let ball = Vec3::new(0.0, 0.45 - 0.22 * over * over, 0.0);
        let rim = Quat::from_rotation_x(spin(u)) * Vec3::new(0.0, 0.25, 0.2);
        rest.lerp(ball - rim, curl(u))
    };
    let mut tracks = vec![
        Track {
            node: node(rig, "hips"),
            path: TrackPath::Translation,
            interpolation: Interpolation::Linear,
            times: times.clone(),
            values: times.iter().flat_map(|&u| hips_at(u).to_array()).collect(),
        },
        // Turned a little onto the right side, so that shoulder leads.
        rotation(node(rig, "hips"), &times, |u| {
            x(spin(u)) * Quat::from_rotation_z(0.3 * curl(u))
        }),
        rotation(node(rig, "spine"), &times, |u| {
            x(0.8 * curl(u)) * Quat::from_rotation_y(-0.25 * curl(u))
        }),
        rotation(node(rig, "chest"), &times, |u| x(0.55 * curl(u))),
        rotation(node(rig, "neck"), &times, |u| {
            x(0.7 * curl(u)) * Quat::from_rotation_z(0.4 * curl(u))
        }),
    ];
    for (side, away, late) in [("l", 1.0, 0.0), ("r", -1.0, 0.04)] {
        let bone = |name: &str| node(rig, &format!("{name}_{side}"));
        tracks.push(rotation(bone("thigh"), &times, move |u| x(-2.0 * curl(u - late))));
        tracks.push(rotation(bone("shin"), &times, move |u| x(2.3 * curl(u - late))));
        tracks.push(rotation(bone("upper_arm"), &times, move |u| {
            out(away, 0.3 * curl(u), 1.5 * reach(u) + 1.1 * curl(u) * (1.0 - reach(u)))
        }));
        tracks.push(rotation(bone("forearm"), &times, move |u| {
            x(-0.3 * reach(u) - (0.2 + 1.6 * curl(u)) * (1.0 - reach(u)))
        }));
    }
    Clip {
        name: "roll".into(),
        tracks,
        markers: vec![],
    }
}

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// The skinned model with every clip the Animator and its layers name.
pub fn model(rig: &Rig) -> Model {
    let mut clips = vec![
        // A relaxed stand, as long as the slowest gait so the blend between them
        // keeps its period: elbows a little bent, arms a little off the body.
        held(
            rig,
            "stand",
            PACES[0].period(),
            &[
                ("hips", Quat::IDENTITY),
                ("upper_arm_l", out(1.0, 0.06, 0.0)),
                ("upper_arm_r", out(-1.0, 0.06, 0.0)),
                ("forearm_l", x(-0.2)),
                ("forearm_r", x(-0.2)),
                ("hand_l", x(-0.12)),
                ("hand_r", x(-0.12)),
            ],
        ),
        breath(rig),
        launch(rig),
        rise(rig),
        float(rig),
        fall(rig),
        reach(rig),
        plunge(rig),
        brace(rig),
        land(rig),
        hang(rig),
        climb(rig),
        vault(rig),
        crouch(rig),
        roll(rig),
        slide(rig),
        kneel(rig),
    ];
    clips.extend(crate::rise::clips(rig));
    clips.extend(PACES.iter().map(|p| gait::clip(rig, p)));
    let mut model = rig.model(clips);
    crate::mannequin::skin(&mut model, rig);
    model
}

/// One state per `Act`, and from each an edge to every other on the number `act`.
/// `move` blends the stance and the gaits along `pace` (m/s); `air` blends falling
/// into rising along `lift` (vertical m/s). Every other state holds its clip still
/// and the game sets its phase, so the pose follows the move's own progress. No
/// state fades in: `flow` carries the pose of one move into the next.
pub fn animator() -> Animator {
    let mut knots = vec![(0.0, "stand".to_string())];
    knots.extend(PACES.iter().map(|p| (p.speed, p.clip())));
    let locomotion = Blend {
        clips: knots,
        ..Blend::default()
    }
    .parameter("pace");
    let states = Act::ALL.map(|act| {
        let name = act.state();
        let state = match act {
            Act::Move => State::blend(name, locomotion.clone()),
            Act::Air => State::blend(name, Blend::across(AIR).parameter("lift")),
            Act::Vault => State::clip(name, "vault").speed(0.0),
            Act::Hang => State::clip(name, "hang").speed(0.0),
            Act::Climb => State::clip(name, "climb").speed(0.0),
            Act::Roll => State::clip(name, "roll").speed(0.0),
            Act::Land => State::clip(name, "crouch").speed(0.0),
            Act::Slide => State::clip(name, "slide").speed(0.0),
            Act::Rise => State::blend(
                name,
                Blend::across([(0.0, "rise-prone"), (1.0, "rise-supine")]).parameter("supine"),
            )
            .speed(0.0),
        };
        Act::ALL
            .into_iter()
            .filter(|other| *other != act)
            .fold(state, |state, other| {
                let is = Condition::Arg("act".into(), Cmp::Eq, Param::Number(other.code()));
                state.to(other.state(), is)
            })
    });
    let mut animator = Animator::new(states);
    animator.set("pace", 0.0);
    animator.set("lift", 0.0);
    animator.set("act", Act::Move.code());
    animator.set("supine", 0.0);
    animator
}

/// Breathing over everything, the two landings, each parked at its end until a
/// touchdown rewinds it, and the brace, which the game dials in before one.
pub fn layers() -> Layers {
    let mut land = Animation::play("land").once();
    land.time = LAND_SECONDS;
    let mut kneel = Animation::play("kneel").once();
    kneel.time = KNEEL_SECONDS;
    let mut brace = Layer::new(Animation::play("brace").once()).additive();
    brace.weight = 0.0;
    Layers(vec![
        Layer::new(Animation::play("breath")).additive(),
        Layer::new(land).additive(),
        Layer::new(kneel).additive(),
        brace,
    ])
}
