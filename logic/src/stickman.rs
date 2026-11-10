//! The stick man: his skeleton, his clips and the Animator that plays them.
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

// Rising: the lead knee drives up, the arms reach, the chest opens.
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
            ("thigh_r", x(0.12)),
            ("shin_r", x(0.45)),
            ("foot_r", x(0.55)),
            ("upper_arm_l", out(1.0, 0.35, -0.5)),
            ("forearm_l", x(-0.7)),
            ("upper_arm_r", out(-1.0, 0.5, 0.9)),
            ("forearm_r", x(-0.9)),
        ],
    )
}

// Falling: the legs reach for the ground, the arms spread for balance.
fn fall(rig: &Rig) -> Clip {
    held(
        rig,
        "fall",
        1.0,
        &[
            ("spine", x(0.1)),
            ("thigh_l", x(-0.4)),
            ("shin_l", x(0.5)),
            ("foot_l", x(0.3)),
            ("thigh_r", x(-0.08)),
            ("shin_r", x(0.38)),
            ("foot_r", x(0.3)),
            ("upper_arm_l", out(1.0, 1.0, 0.25)),
            ("forearm_l", x(-0.35)),
            ("upper_arm_r", out(-1.0, 1.0, 0.25)),
            ("forearm_r", x(-0.35)),
        ],
    )
}

// An additive one-shot: the knees give on touchdown and push back up. The thigh,
// shin and foot turn by -a, 2a and -a, which keeps the foot flat under the hip
// while the hips come down by the height the bent leg lost.
fn land(rig: &Rig) -> Clip {
    const BEND: f32 = 0.5;
    let times: Vec<f32> = (0..=12).map(|i| LAND_SECONDS * i as f32 / 12.0).collect();
    let smooth = |u: f32| u * u * (3.0 - 2.0 * u);
    let give = move |t: f32| {
        let u = t / LAND_SECONDS;
        if u < 0.25 {
            smooth(u / 0.25)
        } else {
            1.0 - smooth((u - 0.25) / 0.75)
        }
    };
    let leg = {
        let bone = |name: &str| {
            let b = &rig.bones()[node(rig, name) as usize];
            (b.head - b.tail).length()
        };
        bone("thigh_l") + bone("shin_l")
    };
    let mut tracks = vec![
        hips(rig, &times, |t| {
            leg * (1.0 - math::cos(BEND * give(t)))
        }),
        rotation(node(rig, "spine"), &times, |t| x(0.22 * give(t))),
    ];
    for side in ["l", "r"] {
        let bone = |name: &str| node(rig, &format!("{name}_{side}"));
        tracks.push(rotation(bone("thigh"), &times, |t| x(-BEND * give(t))));
        tracks.push(rotation(bone("shin"), &times, |t| x(2.0 * BEND * give(t))));
        tracks.push(rotation(bone("foot"), &times, |t| x(-BEND * give(t))));
        tracks.push(rotation(bone("upper_arm"), &times, |t| {
            out(if side == "l" { 1.0 } else { -1.0 }, 0.3 * give(t), 0.0)
        }));
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

// A clip `seconds` long through `keys`. The Animator holds it still and the game
// sets its phase, so a move's pose follows the move and not the clock. A joint
// absent from a key is at rest there.
fn keyed(rig: &Rig, name: &str, seconds: f32, keys: &[Key]) -> Clip {
    let times: Vec<f32> = keys.iter().map(|k| k.at * seconds).collect();
    let mut bones: Vec<&str> = keys
        .iter()
        .flat_map(|k| k.joints.iter().map(|j| j.0))
        .collect();
    bones.sort_unstable();
    bones.dedup();
    let mut tracks: Vec<Track> = bones
        .into_iter()
        .map(|bone| {
            let at = |i: usize| {
                keys[i]
                    .joints
                    .iter()
                    .find(|j| j.0 == bone)
                    .map_or(Quat::IDENTITY, |j| turn(j.1))
            };
            Track {
                node: node(rig, bone),
                path: TrackPath::Rotation,
                interpolation: Interpolation::Linear,
                times: times.clone(),
                values: (0..keys.len()).flat_map(|i| at(i).to_array()).collect(),
            }
        })
        .collect();
    let rest = rig.bones()[node(rig, "hips") as usize].head;
    tracks.push(Track {
        node: node(rig, "hips"),
        path: TrackPath::Translation,
        interpolation: Interpolation::Linear,
        times,
        values: keys
            .iter()
            .flat_map(|k| (rest - Vec3::Y * k.drop).to_array())
            .collect(),
    });
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

// From the hang: pull the chest to the edge, press down on it, bring a knee up
// onto the top, and stand.
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
                    ("thigh_l", [-0.5, 0.0, 0.0]),
                    ("thigh_r", [-0.2, 0.0, 0.0]),
                    ("shin_l", [0.9, 0.0, 0.0]),
                    ("shin_r", [0.6, 0.0, 0.0]),
                    ("spine", [0.15, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.6,
                drop: 0.1,
                joints: &[
                    ("upper_arm_l", [0.5, 0.0, 0.25]),
                    ("upper_arm_r", [0.5, 0.0, -0.25]),
                    ("forearm_l", [-1.5, 0.0, 0.0]),
                    ("forearm_r", [-1.5, 0.0, 0.0]),
                    ("thigh_l", [-0.3, 0.0, 0.0]),
                    ("thigh_r", [-1.9, 0.0, 0.0]),
                    ("shin_l", [0.5, 0.0, 0.0]),
                    ("shin_r", [1.9, 0.0, 0.0]),
                    ("spine", [0.7, 0.0, 0.0]),
                    ("neck", [-0.4, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.82,
                drop: 0.42,
                joints: &[
                    ("upper_arm_l", [-0.7, 0.0, 0.1]),
                    ("upper_arm_r", [-0.7, 0.0, -0.1]),
                    ("forearm_l", [-0.6, 0.0, 0.0]),
                    ("forearm_r", [-0.6, 0.0, 0.0]),
                    ("thigh_l", [-1.0, 0.0, 0.0]),
                    ("thigh_r", [-1.0, 0.0, 0.0]),
                    ("shin_l", [2.0, 0.0, 0.0]),
                    ("shin_r", [2.0, 0.0, 0.0]),
                    ("foot_l", [-1.0, 0.0, 0.0]),
                    ("foot_r", [-1.0, 0.0, 0.0]),
                    ("spine", [0.55, 0.0, 0.0]),
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

// Over a low wall: the left hand goes down on the top, both knees come up and
// swing through beside it, and the legs reach for the far side.
fn vault(rig: &Rig) -> Clip {
    keyed(
        rig,
        "vault",
        1.0,
        &[
            Key {
                at: 0.0,
                drop: 0.05,
                joints: &[
                    ("spine", [0.3, 0.0, 0.0]),
                    ("upper_arm_l", [-1.1, 0.0, 0.0]),
                    ("forearm_l", [-0.3, 0.0, 0.0]),
                    ("upper_arm_r", [0.5, 0.0, 0.0]),
                    ("forearm_r", [-0.9, 0.0, 0.0]),
                    ("thigh_l", [0.4, 0.0, 0.0]),
                    ("shin_l", [0.5, 0.0, 0.0]),
                    ("thigh_r", [-0.9, 0.0, 0.0]),
                    ("shin_r", [1.2, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.3,
                drop: 0.15,
                joints: &[
                    ("hips", [0.0, 0.0, 0.2]),
                    ("spine", [0.5, 0.0, 0.0]),
                    ("upper_arm_l", [-0.6, 0.0, 0.25]),
                    ("forearm_l", [-0.1, 0.0, 0.0]),
                    ("upper_arm_r", [0.2, 0.0, -0.6]),
                    ("forearm_r", [-0.8, 0.0, 0.0]),
                    ("thigh_l", [-1.0, 0.0, 0.0]),
                    ("shin_l", [1.7, 0.0, 0.0]),
                    ("thigh_r", [-1.6, 0.0, 0.0]),
                    ("shin_r", [1.9, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.55,
                drop: 0.3,
                joints: &[
                    ("hips", [0.0, 0.0, 0.45]),
                    ("spine", [0.55, 0.0, -0.2]),
                    ("upper_arm_l", [0.1, 0.0, 0.5]),
                    ("forearm_l", [0.0, 0.0, 0.0]),
                    ("upper_arm_r", [-0.3, 0.0, -0.9]),
                    ("forearm_r", [-0.6, 0.0, 0.0]),
                    ("thigh_l", [-1.8, 0.0, 0.0]),
                    ("shin_l", [1.9, 0.0, 0.0]),
                    ("thigh_r", [-1.8, 0.0, 0.0]),
                    ("shin_r", [1.8, 0.0, 0.0]),
                    ("neck", [-0.3, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.8,
                drop: 0.12,
                joints: &[
                    ("hips", [0.0, 0.0, 0.15]),
                    ("spine", [0.25, 0.0, 0.0]),
                    ("upper_arm_l", [0.8, 0.0, 0.3]),
                    ("forearm_l", [-0.4, 0.0, 0.0]),
                    ("upper_arm_r", [-0.5, 0.0, -0.3]),
                    ("forearm_r", [-0.7, 0.0, 0.0]),
                    ("thigh_l", [-1.0, 0.0, 0.0]),
                    ("shin_l", [0.7, 0.0, 0.0]),
                    ("thigh_r", [-0.6, 0.0, 0.0]),
                    ("shin_r", [1.0, 0.0, 0.0]),
                ],
            },
            Key {
                at: 1.0,
                drop: 0.05,
                joints: &[
                    ("spine", [0.15, 0.0, 0.0]),
                    ("upper_arm_l", [0.3, 0.0, 0.0]),
                    ("forearm_l", [-0.7, 0.0, 0.0]),
                    ("upper_arm_r", [-0.3, 0.0, 0.0]),
                    ("forearm_r", [-0.7, 0.0, 0.0]),
                    ("thigh_l", [-0.5, 0.0, 0.0]),
                    ("shin_l", [0.4, 0.0, 0.0]),
                    ("thigh_r", [0.1, 0.0, 0.0]),
                    ("shin_r", [0.6, 0.0, 0.0]),
                ],
            },
        ],
    )
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

// A forward roll: the body curls into a ball that turns once about its middle,
// 0.45 m above the ground, and uncurls onto its feet.
fn roll(rig: &Rig) -> Clip {
    let times: Vec<f32> = (0..=32).map(|i| i as f32 / 32.0).collect();
    // How tightly the body is curled, and how far it has turned.
    let curl = |u: f32| smooth(u / 0.2) * (1.0 - smooth((u - 0.78) / 0.22));
    let spin = |u: f32| std::f32::consts::TAU * smooth((u - 0.08) / 0.8);
    let rest = rig.bones()[node(rig, "hips") as usize].head;
    // The hips ride the rim of the ball: its middle is in front of the belly.
    let hips_at = move |u: f32| {
        let ball = Vec3::new(0.0, 0.45, 0.0);
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
        rotation(node(rig, "hips"), &times, |u| x(spin(u))),
        rotation(node(rig, "spine"), &times, |u| x(0.55 * curl(u))),
        rotation(node(rig, "chest"), &times, |u| x(0.35 * curl(u))),
        rotation(node(rig, "neck"), &times, |u| x(0.5 * curl(u))),
    ];
    for (side, away) in [("l", 1.0), ("r", -1.0)] {
        let bone = |name: &str| node(rig, &format!("{name}_{side}"));
        tracks.push(rotation(bone("thigh"), &times, |u| x(-2.0 * curl(u))));
        tracks.push(rotation(bone("shin"), &times, |u| x(2.3 * curl(u))));
        tracks.push(rotation(bone("upper_arm"), &times, move |u| {
            out(away, 0.3 * curl(u), 1.1 * curl(u))
        }));
        tracks.push(rotation(bone("forearm"), &times, |u| x(-0.2 - 1.6 * curl(u))));
    }
    Clip {
        name: "roll".into(),
        tracks,
        markers: vec![],
    }
}

// Up from lying on the front, head toward +Z: the hands come in beside the chest
// and push it up, the knees come under, one foot steps through, and the body
// stands over it.
fn rise_prone(rig: &Rig) -> Clip {
    keyed(
        rig,
        "rise-prone",
        1.0,
        &[
            Key {
                at: 0.0,
                drop: 0.84,
                joints: &[
                    ("hips", [1.5, 0.0, 0.0]),
                    ("upper_arm_l", [-2.6, 0.0, 0.5]),
                    ("upper_arm_r", [-2.6, 0.0, -0.5]),
                    ("forearm_l", [-0.6, 0.0, 0.0]),
                    ("forearm_r", [-0.6, 0.0, 0.0]),
                    ("neck", [-0.3, 0.0, 0.0]),
                    ("foot_l", [0.8, 0.0, 0.0]),
                    ("foot_r", [0.8, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.2,
                drop: 0.84,
                joints: &[
                    ("hips", [1.5, 0.0, 0.0]),
                    ("upper_arm_l", [0.2, 0.0, 0.9]),
                    ("upper_arm_r", [0.2, 0.0, -0.9]),
                    ("forearm_l", [-2.3, 0.0, 0.0]),
                    ("forearm_r", [-2.3, 0.0, 0.0]),
                    ("neck", [-0.4, 0.0, 0.0]),
                    ("foot_l", [0.8, 0.0, 0.0]),
                    ("foot_r", [0.8, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.42,
                drop: 0.74,
                joints: &[
                    ("hips", [1.25, 0.0, 0.0]),
                    ("spine", [-0.45, 0.0, 0.0]),
                    ("upper_arm_l", [-1.0, 0.0, 0.2]),
                    ("upper_arm_r", [-1.0, 0.0, -0.2]),
                    ("forearm_l", [-0.25, 0.0, 0.0]),
                    ("forearm_r", [-0.25, 0.0, 0.0]),
                    ("neck", [-0.5, 0.0, 0.0]),
                    ("thigh_l", [-0.2, 0.0, 0.0]),
                    ("thigh_r", [-0.2, 0.0, 0.0]),
                    ("foot_l", [0.3, 0.0, 0.0]),
                    ("foot_r", [0.3, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.62,
                drop: 0.52,
                joints: &[
                    ("hips", [1.05, 0.0, 0.0]),
                    ("spine", [-0.1, 0.0, 0.0]),
                    ("upper_arm_l", [-1.25, 0.0, 0.15]),
                    ("upper_arm_r", [-1.25, 0.0, -0.15]),
                    ("forearm_l", [-0.15, 0.0, 0.0]),
                    ("forearm_r", [-0.15, 0.0, 0.0]),
                    ("neck", [-0.5, 0.0, 0.0]),
                    ("thigh_l", [-2.1, 0.0, 0.0]),
                    ("thigh_r", [-2.1, 0.0, 0.0]),
                    ("shin_l", [2.2, 0.0, 0.0]),
                    ("shin_r", [2.2, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.8,
                drop: 0.46,
                joints: &[
                    ("hips", [0.55, 0.0, 0.0]),
                    ("spine", [0.2, 0.0, 0.0]),
                    ("upper_arm_l", [-0.9, 0.0, 0.1]),
                    ("upper_arm_r", [-0.5, 0.0, -0.2]),
                    ("forearm_l", [-0.5, 0.0, 0.0]),
                    ("forearm_r", [-0.3, 0.0, 0.0]),
                    ("neck", [-0.3, 0.0, 0.0]),
                    ("thigh_l", [-2.0, 0.0, 0.0]),
                    ("shin_l", [1.5, 0.0, 0.0]),
                    ("thigh_r", [-0.6, 0.0, 0.0]),
                    ("shin_r", [1.9, 0.0, 0.0]),
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

// Up from lying on the back, head toward -Z: sit up onto the hands, draw the feet
// in, rock forward over them, and stand.
fn rise_supine(rig: &Rig) -> Clip {
    keyed(
        rig,
        "rise-supine",
        1.0,
        &[
            Key {
                at: 0.0,
                drop: 0.84,
                joints: &[
                    ("hips", [-1.5, 0.0, 0.0]),
                    ("upper_arm_l", [0.0, 0.0, 1.0]),
                    ("upper_arm_r", [0.0, 0.0, -1.0]),
                ],
            },
            Key {
                at: 0.22,
                drop: 0.83,
                joints: &[
                    ("hips", [-1.0, 0.0, 0.0]),
                    ("spine", [0.3, 0.0, 0.0]),
                    ("upper_arm_l", [1.0, 0.0, 0.35]),
                    ("upper_arm_r", [1.0, 0.0, -0.35]),
                    ("forearm_l", [-0.3, 0.0, 0.0]),
                    ("forearm_r", [-0.3, 0.0, 0.0]),
                    ("thigh_l", [-0.9, 0.0, 0.0]),
                    ("thigh_r", [-0.7, 0.0, 0.0]),
                    ("shin_l", [1.3, 0.0, 0.0]),
                    ("shin_r", [1.0, 0.0, 0.0]),
                    ("neck", [0.3, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.45,
                drop: 0.8,
                joints: &[
                    ("hips", [-0.35, 0.0, 0.0]),
                    ("spine", [0.35, 0.0, 0.0]),
                    ("upper_arm_l", [0.55, 0.0, 0.3]),
                    ("upper_arm_r", [0.55, 0.0, -0.3]),
                    ("forearm_l", [-0.15, 0.0, 0.0]),
                    ("forearm_r", [-0.15, 0.0, 0.0]),
                    ("thigh_l", [-1.75, 0.0, 0.0]),
                    ("thigh_r", [-1.75, 0.0, 0.0]),
                    ("shin_l", [2.2, 0.0, 0.0]),
                    ("shin_r", [2.2, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.66,
                drop: 0.5,
                joints: &[
                    ("hips", [0.55, 0.0, 0.0]),
                    ("spine", [0.3, 0.0, 0.0]),
                    ("upper_arm_l", [-1.1, 0.0, 0.1]),
                    ("upper_arm_r", [-1.1, 0.0, -0.1]),
                    ("forearm_l", [-0.3, 0.0, 0.0]),
                    ("forearm_r", [-0.3, 0.0, 0.0]),
                    ("thigh_l", [-2.3, 0.0, 0.0]),
                    ("thigh_r", [-2.3, 0.0, 0.0]),
                    ("shin_l", [2.4, 0.0, 0.0]),
                    ("shin_r", [2.4, 0.0, 0.0]),
                    ("foot_l", [-0.65, 0.0, 0.0]),
                    ("foot_r", [-0.65, 0.0, 0.0]),
                ],
            },
            Key {
                at: 0.85,
                drop: 0.28,
                joints: &[
                    ("hips", [0.3, 0.0, 0.0]),
                    ("spine", [0.2, 0.0, 0.0]),
                    ("upper_arm_l", [-0.6, 0.0, 0.05]),
                    ("upper_arm_r", [-0.6, 0.0, -0.05]),
                    ("forearm_l", [-0.4, 0.0, 0.0]),
                    ("forearm_r", [-0.4, 0.0, 0.0]),
                    ("thigh_l", [-1.3, 0.0, 0.0]),
                    ("thigh_r", [-1.3, 0.0, 0.0]),
                    ("shin_l", [1.6, 0.0, 0.0]),
                    ("shin_r", [1.6, 0.0, 0.0]),
                    ("foot_l", [-0.6, 0.0, 0.0]),
                    ("foot_r", [-0.6, 0.0, 0.0]),
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
        rise(rig),
        fall(rig),
        land(rig),
        hang(rig),
        climb(rig),
        vault(rig),
        crouch(rig),
        roll(rig),
        rise_prone(rig),
        rise_supine(rig),
    ];
    clips.extend(PACES.iter().map(|p| gait::clip(rig, p)));
    rig.model(clips)
}

/// One state per `Act`, and from each an edge to every other on the number `act`.
/// `move` blends the stance and the gaits along `pace` (m/s); `air` blends falling
/// into rising along `lift` (vertical m/s). Every other state holds its clip still
/// and the game sets its phase, so the pose follows the move's own progress. Each
/// change fades from the pose the body was in.
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
            Act::Move => State::blend(name, locomotion.clone()).fade(0.14),
            Act::Air => State::blend(
                name,
                Blend::across([(-2.5, "fall"), (2.5, "rise")]).parameter("lift"),
            )
            .fade(0.1),
            Act::Vault => State::clip(name, "vault").speed(0.0).fade(0.08),
            Act::Hang => State::clip(name, "hang").speed(0.0).fade(0.12),
            Act::Climb => State::clip(name, "climb").speed(0.0).fade(0.1),
            Act::Roll => State::clip(name, "roll").speed(0.0).fade(0.06),
            Act::Land => State::clip(name, "crouch").speed(0.0).fade(0.05),
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

/// Breathing over everything, and the landing crouch parked at its end until a
/// touchdown rewinds it.
pub fn layers() -> Layers {
    let mut land = Animation::play("land").once();
    land.time = LAND_SECONDS;
    Layers(vec![
        Layer::new(Animation::play("breath")).additive(),
        Layer::new(land).additive(),
    ])
}
