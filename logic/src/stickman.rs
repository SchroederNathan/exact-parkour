//! The stick man: his skeleton, his clips and the Animator that plays them.
//!
//! Locomotion is one blend along ground speed: a still stance at zero, then the
//! family of gaits in `gait::PACES`.
use exact_game::animation::{Layer, Layers, Param};
use exact_game::asset::{Clip, Interpolation, Model, Track, TrackPath};
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

/// The skinned model with every clip the Animator and its layers name.
pub fn model(rig: &Rig) -> Model {
    let mut clips = vec![
        // Rest, as long as the slowest gait so the blend between them keeps its period.
        held(rig, "stand", PACES[0].period(), &[("hips", Quat::IDENTITY)]),
        rig.idle("breath"),
        rise(rig),
        fall(rig),
        land(rig),
    ];
    clips.extend(PACES.iter().map(|p| gait::clip(rig, p)));
    rig.model(clips)
}

/// `move` blends the stance and the gaits along `pace` (m/s); `air` blends falling
/// into rising along `lift` (vertical m/s). The flag `air` moves between them, each
/// way through a short fade from the pose the body was in.
pub fn animator() -> Animator {
    let mut knots = vec![(0.0, "stand".to_string())];
    knots.extend(PACES.iter().map(|p| (p.speed, p.clip())));
    let locomotion = Blend {
        clips: knots,
        ..Blend::default()
    }
    .parameter("pace");
    let airborne = |value: bool| Condition::Arg("air".into(), Cmp::Eq, Param::Flag(value));
    let mut animator = Animator::new([
        State::blend("move", locomotion)
            .fade(0.14)
            .to("air", airborne(true)),
        State::blend(
            "air",
            Blend::across([(-2.5, "fall"), (2.5, "rise")]).parameter("lift"),
        )
        .fade(0.1)
        .to("move", airborne(false)),
    ]);
    animator.set("pace", 0.0);
    animator.set("lift", 0.0);
    animator.set("air", false);
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
