//! Getting up from the ground, drawn from where the hands and feet are.
//!
//! A rise is a list of stages. Each says where the hips are and how they are
//! pitched, how the spine bends, and where each wrist and ankle is, in the frame of
//! the place the body will stand. Between stages the values ease from one to the
//! next, and the engine's two-bone solver turns each arm and leg to reach its
//! target. A hand or a foot that has the same target in two stages does not move
//! between them: it is planted, and the body moves over it.
use exact_game::animation::{bind_pose, joint_matrix, solve_ik};
use exact_game::asset::{Clip, Interpolation, Model, Track, TrackPath};
use exact_game::rig::Rig;
use exact_game::*;

/// Samples in a baked rise.
const SAMPLES: usize = 48;
/// Height of a wrist and of an ankle that rest on the floor, metres.
const WRIST: f32 = 0.04;
const ANKLE: f32 = 0.1;
/// Height of the ankle of a foot that stands on its toes, and that foot's pitch.
const TOES: f32 = 0.17;
const TOE_PITCH: f32 = 1.1;

/// One moment of a rise. `x` is to the body's left and `z` is the way it will face.
#[derive(Clone, Copy)]
struct Stage {
    /// How far through the rise, 0 to 1.
    at: f32,
    /// The hips: height, distance ahead of the standing place, and pitch about X
    /// (forward is positive; a body on its front is at +1.5).
    hips: [f32; 3],
    /// Bend of the spine and of the neck about X.
    spine: f32,
    neck: f32,
    /// Left wrist and left ankle; the right ones mirror them unless given.
    hand: [f32; 3],
    foot: [f32; 3],
    hand_r: Option<[f32; 3]>,
    foot_r: Option<[f32; 3]>,
    /// Pitch of the left and right foot, toes down positive.
    toes: [f32; 2],
}

const fn stage(at: f32, hips: [f32; 3], spine: f32, hand: [f32; 3], foot: [f32; 3]) -> Stage {
    Stage {
        at,
        hips,
        spine,
        neck: 0.0,
        hand,
        foot,
        hand_r: None,
        foot_r: None,
        toes: [0.0, 0.0],
    }
}

impl Stage {
    const fn neck(mut self, neck: f32) -> Self {
        self.neck = neck;
        self
    }
    const fn right_hand(mut self, at: [f32; 3]) -> Self {
        self.hand_r = Some(at);
        self
    }
    const fn right_foot(mut self, at: [f32; 3]) -> Self {
        self.foot_r = Some(at);
        self
    }
    const fn on_toes(mut self, left: f32, right: f32) -> Self {
        self.toes = [left, right];
        self
    }
    // Wrist and ankle targets, left then right.
    fn limbs(&self) -> [[Vec3; 2]; 2] {
        let mirror = |p: [f32; 3]| [-p[0], p[1], p[2]];
        let hand_r = self.hand_r.unwrap_or(mirror(self.hand));
        let foot_r = self.foot_r.unwrap_or(mirror(self.foot));
        [
            [Vec3::from_array(self.hand), Vec3::from_array(hand_r)],
            [Vec3::from_array(self.foot), Vec3::from_array(foot_r)],
        ]
    }
}

// Where the standing body has its wrists and ankles.
const HANG: [f32; 3] = [0.24, 0.91, 0.02];
const STAND: [f32; 3] = [0.09, ANKLE, -0.03];

/// How far behind the standing place the hips of a lying body are, metres. The
/// body rises onto its feet, which are not where its hips lay.
pub const PRONE_BACK: f32 = 0.25;
pub const SUPINE_BACK: f32 = 0.45;

// From the front, head toward +Z. The hands come in beside the chest and press it
// up. The hips lift, the left foot steps through under the body, the hands leave
// the floor, and the right foot comes up beside the left.
const PRONE: &[Stage] = &[
    stage(0.0, [0.13, -PRONE_BACK, 1.5], 0.0, [0.34, WRIST, 0.55], [0.1, TOES, -1.1])
        .neck(-0.2)
        .on_toes(TOE_PITCH, TOE_PITCH),
    stage(0.18, [0.13, -PRONE_BACK, 1.5], 0.0, [0.3, WRIST, 0.2], [0.1, TOES, -1.1])
        .neck(-0.35)
        .on_toes(TOE_PITCH, TOE_PITCH),
    stage(0.4, [0.24, -PRONE_BACK, 1.3], -0.55, [0.3, WRIST, 0.2], [0.1, TOES, -1.1])
        .neck(-0.5)
        .on_toes(TOE_PITCH, TOE_PITCH),
    stage(0.58, [0.4, -0.36, 1.38], -0.15, [0.3, WRIST, 0.2], [0.1, TOES, -1.1])
        .neck(-0.4)
        .on_toes(TOE_PITCH, TOE_PITCH),
    stage(0.74, [0.4, -0.25, 1.3], -0.05, [0.3, WRIST, 0.2], STAND)
        .right_foot([-0.1, TOES, -1.0])
        .neck(-0.4)
        .on_toes(0.0, TOE_PITCH),
    stage(0.88, [0.66, -0.08, 0.5], 0.2, [0.26, 0.5, 0.25], STAND)
        .right_foot([-0.1, TOES, -0.45])
        .neck(-0.2)
        .on_toes(0.0, 0.7),
    stage(1.0, [0.96, 0.0, 0.0], 0.0, HANG, STAND),
];

// From the back, head toward -Z. The body sits up onto its hands, draws its feet in
// flat, presses on the floor to bring the hips forward over the feet, and stands.
const SUPINE: &[Stage] = &[
    stage(0.0, [0.12, -SUPINE_BACK, -1.5], 0.0, [0.5, WRIST, -0.75], [0.12, 0.08, 0.5]),
    stage(0.22, [0.12, -SUPINE_BACK, -0.75], 0.3, [0.3, WRIST, -0.8], [0.12, ANKLE, 0.25]).neck(0.3),
    stage(0.42, [0.13, -SUPINE_BACK, -0.35], 0.35, [0.3, WRIST, -0.6], [0.11, ANKLE, 0.02]),
    stage(0.6, [0.24, -0.22, 0.5], 0.35, [0.3, WRIST, -0.05], STAND).neck(-0.3),
    stage(0.74, [0.42, -0.06, 0.6], 0.3, [0.22, 0.3, 0.3], STAND).neck(-0.3),
    stage(0.88, [0.68, -0.02, 0.35], 0.2, [0.24, 0.62, 0.22], STAND).neck(-0.1),
    stage(1.0, [0.96, 0.0, 0.0], 0.0, HANG, STAND),
];

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

// The stage values at `u`, eased between the stages either side.
fn at(stages: &[Stage], u: f32) -> Stage {
    let next = stages
        .iter()
        .position(|s| s.at >= u)
        .unwrap_or(stages.len() - 1)
        .max(1);
    let (a, b) = (stages[next - 1], stages[next]);
    let k = smooth((u - a.at) / (b.at - a.at));
    let mix = |x: f32, y: f32| x + (y - x) * k;
    let mix3 = |x: [f32; 3], y: [f32; 3]| [mix(x[0], y[0]), mix(x[1], y[1]), mix(x[2], y[2])];
    let [[hand_la, hand_ra], [foot_la, foot_ra]] = a.limbs();
    let [[hand_lb, hand_rb], [foot_lb, foot_rb]] = b.limbs();
    Stage {
        at: u,
        hips: mix3(a.hips, b.hips),
        spine: mix(a.spine, b.spine),
        neck: mix(a.neck, b.neck),
        hand: hand_la.lerp(hand_lb, k).to_array(),
        foot: foot_la.lerp(foot_lb, k).to_array(),
        hand_r: Some(hand_ra.lerp(hand_rb, k).to_array()),
        foot_r: Some(foot_ra.lerp(foot_rb, k).to_array()),
        toes: [mix(a.toes[0], b.toes[0]), mix(a.toes[1], b.toes[1])],
    }
}

fn node(model: &Model, name: &str) -> usize {
    model
        .nodes
        .iter()
        .position(|n| n.name == name)
        .unwrap_or_else(|| panic!("rise: no joint `{name}`"))
}

fn set_turn(local: &mut [f32], node: usize, turn: Quat) {
    local[node * 10 + 3..node * 10 + 7].copy_from_slice(&turn.normalize().to_array());
}

// The local pose of every joint at one stage.
fn solve(model: &Model, rest: &[f32], stage: &Stage) -> Vec<f32> {
    let mut local = rest.to_vec();
    let hips = node(model, "hips");
    let [height, ahead, pitch] = stage.hips;
    local[hips * 10..hips * 10 + 3].copy_from_slice(&[0.0, height, ahead]);
    set_turn(&mut local, hips, Quat::from_rotation_x(pitch));
    set_turn(&mut local, node(model, "spine"), Quat::from_rotation_x(stage.spine));
    set_turn(&mut local, node(model, "neck"), Quat::from_rotation_x(stage.neck));
    let [hands, feet] = stage.limbs();
    for (side, out) in [("l", 1.0), ("r", -1.0)] {
        let i = if out > 0.0 { 0 } else { 1 };
        let chain = |a: &str, b: &str, c: &str| {
            [a, b, c].map(|bone| format!("{bone}_{side}"))
        };
        let joint = |local: &[f32], name: &str| {
            let n = node(model, &format!("{name}_{side}")) as u32;
            joint_matrix(model, local, n)
        };
        // An elbow points out, up and back of the line from shoulder to wrist; a
        // knee points forward of the line from hip to ankle.
        let shoulder = joint(&local, "upper_arm").w_axis.truncate();
        let arm = Ik {
            chain: chain("upper_arm", "forearm", "hand"),
            target: hands[i],
            pole: (shoulder + hands[i]) * 0.5 + Vec3::new(out * 0.5, 0.35, -0.35),
            weight: 1.0,
        };
        solve_ik(model, &mut local, &arm).expect("the arm's chain is in the rig");
        let hip = joint(&local, "thigh").w_axis.truncate();
        let leg = Ik {
            chain: chain("thigh", "shin", "foot"),
            target: feet[i],
            pole: (hip + feet[i]) * 0.5 + Vec3::new(out * 0.1, 0.1, 0.8),
            weight: 1.0,
        };
        solve_ik(model, &mut local, &leg).expect("the leg's chain is in the rig");
        // The foot keeps its own pitch whatever the shin does.
        let shin = joint(&local, "shin").to_scale_rotation_translation().1;
        let foot = node(model, &format!("foot_{side}"));
        set_turn(&mut local, foot, shin.inverse() * Quat::from_rotation_x(stage.toes[i]));
    }
    local
}

fn bake(rig: &Rig, name: &str, stages: &[Stage]) -> Clip {
    let model = rig.model([]);
    let rest = bind_pose(&model);
    let times: Vec<f32> = (0..=SAMPLES).map(|i| i as f32 / SAMPLES as f32).collect();
    let poses: Vec<Vec<f32>> = times
        .iter()
        .map(|&u| solve(&model, &rest, &at(stages, u)))
        .collect();
    let mut tracks = vec![];
    for (n, bone) in rig.bones().iter().enumerate() {
        if bone.name == "root" {
            continue;
        }
        tracks.push(Track {
            node: n as u32,
            path: TrackPath::Rotation,
            interpolation: Interpolation::Linear,
            times: times.clone(),
            values: poses.iter().flat_map(|p| p[n * 10 + 3..n * 10 + 7].to_vec()).collect(),
        });
    }
    let hips = node(&model, "hips");
    tracks.push(Track {
        node: hips as u32,
        path: TrackPath::Translation,
        interpolation: Interpolation::Linear,
        times,
        values: poses.iter().flat_map(|p| p[hips * 10..hips * 10 + 3].to_vec()).collect(),
    });
    Clip {
        name: name.into(),
        tracks,
        markers: vec![],
    }
}

/// The two rises, `rise-prone` and `rise-supine`, each one second long.
pub fn clips(rig: &Rig) -> [Clip; 2] {
    [bake(rig, "rise-prone", PRONE), bake(rig, "rise-supine", SUPINE)]
}
