//! The body as a ragdoll: fifteen points held together by constraints, falling
//! under gravity and colliding with the park through the physics queries.
//!
//! The engine's physics has rigid bodies but no joints, so the ragdoll is solved
//! here, by position-based dynamics. Each tick the points fall and move, the
//! constraints pull them back into the shape of a body, and each point is swept
//! against the world as a small sphere. The stick man's bones are then turned to
//! lie between the points.
use crate::body::{GRAVITY, HALF};
use crate::park::{PROP, SOLID};
use crate::stickman::MODEL;
use exact_game::animation::joint_matrix;
use exact_game::*;
use exact_game_physics::{self as physics, Body, Shape};

/// The points, by the joint of the rig each one rides.
pub const JOINTS: [&str; 15] = [
    "head",
    "neck",
    "hips",
    "upper_arm_l",
    "upper_arm_r",
    "thigh_l",
    "thigh_r",
    "forearm_l",
    "forearm_r",
    "hand_l",
    "hand_r",
    "shin_l",
    "shin_r",
    "foot_l",
    "foot_r",
];
const HEAD: usize = 0;
const CHEST: usize = 1;
const HIPS: usize = 2;
const SHOULDER: [usize; 2] = [3, 4];
const HIP: [usize; 2] = [5, 6];
const ELBOW: [usize; 2] = [7, 8];
const HAND: [usize; 2] = [9, 10];
const KNEE: [usize; 2] = [11, 12];
const FOOT: [usize; 2] = [13, 14];

/// Where each point is in the rig's rest pose, metres; a test holds this to the rig.
pub const REST: [Vec3; 15] = [
    Vec3::new(0.0, 1.68, 0.0),
    Vec3::new(0.0, 1.48, 0.0),
    Vec3::new(0.0, 0.96, 0.0),
    Vec3::new(0.19, 1.44, 0.0),
    Vec3::new(-0.19, 1.44, 0.0),
    Vec3::new(0.09, 0.98, 0.0),
    Vec3::new(-0.09, 0.98, 0.0),
    Vec3::new(0.23, 1.17, 0.0),
    Vec3::new(-0.23, 1.17, 0.0),
    Vec3::new(0.24, 0.91, 0.02),
    Vec3::new(-0.24, 0.91, 0.02),
    Vec3::new(0.09, 0.55, 0.0),
    Vec3::new(-0.09, 0.55, 0.0),
    Vec3::new(0.09, 0.1, -0.03),
    Vec3::new(-0.09, 0.1, -0.03),
];
/// Radius of the sphere each point collides as, metres.
const RADIUS: [f32; 15] = [
    0.11, 0.07, 0.09, 0.05, 0.05, 0.07, 0.07, 0.04, 0.04, 0.035, 0.035, 0.05, 0.05, 0.05, 0.05,
];
/// Share of the body's mass each point carries; the trunk is heavy and the hands light.
const MASS: [f32; 15] = [
    5.0, 12.0, 14.0, 4.0, 4.0, 7.0, 7.0, 2.0, 2.0, 1.0, 1.0, 4.5, 4.5, 2.0, 2.0,
];

// The trunk is one rigid piece: every pair of its six points keeps its distance.
const TRUNK: [usize; 6] = [CHEST, HIPS, SHOULDER[0], SHOULDER[1], HIP[0], HIP[1]];
// The bones outside the trunk.
const BONES: [(usize, usize); 9] = [
    (CHEST, HEAD),
    (SHOULDER[0], ELBOW[0]),
    (ELBOW[0], HAND[0]),
    (SHOULDER[1], ELBOW[1]),
    (ELBOW[1], HAND[1]),
    (HIP[0], KNEE[0]),
    (KNEE[0], FOOT[0]),
    (HIP[1], KNEE[1]),
    (KNEE[1], FOOT[1]),
];
// Pairs that may come no nearer than this, and pairs that may go no further: the
// joints' limits, as distances.
const APART: [(usize, usize, f32); 14] = [
    (HIP[0], FOOT[0], 0.3),
    (HIP[1], FOOT[1], 0.3),
    (SHOULDER[0], HAND[0], 0.2),
    (SHOULDER[1], HAND[1], 0.2),
    (KNEE[0], KNEE[1], 0.11),
    (FOOT[0], FOOT[1], 0.11),
    (KNEE[0], CHEST, 0.3),
    (KNEE[1], CHEST, 0.3),
    (FOOT[0], HIPS, 0.3),
    (FOOT[1], HIPS, 0.3),
    (HAND[0], HEAD, 0.14),
    (HAND[1], HEAD, 0.14),
    (HEAD, SHOULDER[0], 0.22),
    (HEAD, SHOULDER[1], 0.22),
];
const WITHIN: [(usize, usize, f32); 4] = [
    (KNEE[0], KNEE[1], 0.75),
    (FOOT[0], FOOT[1], 1.2),
    (HEAD, SHOULDER[0], 0.36),
    (HEAD, SHOULDER[1], 0.36),
];

const SUBSTEPS: u32 = 2;
const ITERATIONS: u32 = 8;
/// Share of the speed along a surface that one contact takes away, and of the
/// speed into it that bounces back.
const FRICTION: f32 = 0.35;
const BOUNCE: f32 = 0.1;
/// The body is at rest when no point is faster than `STILL` m/s for `REST_FOR`
/// seconds; after `LIMIT` seconds it gets up wherever it is.
const STILL: f32 = 0.45;
const REST_FOR: f32 = 0.45;
const LIMIT: f32 = 6.0;

/// The ragdoll's points; empty while the body is on its feet.
#[derive(Clone, Debug, Default, Component)]
pub struct Ragdoll {
    /// World position of each point, metres.
    pub at: Vec<Vec3>,
    /// Velocity of each point, m/s.
    pub velocity: Vec<Vec3>,
    /// Seconds the body has been still, and seconds since it fell.
    pub still: f32,
    pub age: f32,
}

// A surface a point touched this step: a point on it and its normal.
#[derive(Clone, Copy)]
struct Contact {
    point: Vec3,
    normal: Vec3,
    /// Velocity of the thing touched, m/s.
    carried: Vec3,
}

impl Ragdoll {
    pub fn active(&self) -> bool {
        !self.at.is_empty()
    }
    /// The trunk's frame: right, up and forward unit vectors.
    pub fn trunk(&self) -> [Vec3; 3] {
        let right = (self.at[HIP[0]] - self.at[HIP[1]]).normalize_or(Vec3::X);
        let spine = (self.at[CHEST] - self.at[HIPS]).normalize_or(Vec3::Y);
        let forward = right.cross(spine).normalize_or(Vec3::Z);
        [right, forward.cross(right), forward]
    }
    /// The trunk's turn from the rest pose.
    pub fn turn(&self) -> Quat {
        let [right, up, forward] = self.trunk();
        Quat::from_mat4(&Mat4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            forward.extend(0.0),
            Vec4::W,
        ))
    }
    /// Whether the chest faces the sky.
    pub fn face_up(&self) -> bool {
        self.trunk()[2].y > 0.0
    }
    /// The compass direction from the hips to the head, as a yaw.
    pub fn lying(&self, fallback: f32) -> f32 {
        let along = self.at[CHEST] - self.at[HIPS];
        if along.x * along.x + along.z * along.z < 1e-4 {
            fallback
        } else {
            math::atan2(along.x, along.z)
        }
    }
    pub fn hips(&self) -> Vec3 {
        self.at[HIPS]
    }
    /// The fastest point's speed, m/s.
    pub fn speed(&self) -> f32 {
        self.velocity.iter().map(|v| v.length()).fold(0.0, f32::max)
    }
}

fn distance_between(a: usize, b: usize) -> f32 {
    (REST[a] - REST[b]).length()
}

// Move two points along the line between them until they are `want` apart, each
// by the share its mass leaves it.
fn pull(at: &mut [Vec3], a: usize, b: usize, want: f32, stiffness: f32) {
    let delta = at[b] - at[a];
    let length = delta.length();
    if length < 1e-6 {
        return;
    }
    let (wa, wb) = (1.0 / MASS[a], 1.0 / MASS[b]);
    let correction = delta * ((length - want) / length * stiffness / (wa + wb));
    at[a] += correction * wa;
    at[b] -= correction * wb;
}

fn constrain(at: &mut [Vec3]) {
    for (i, &a) in TRUNK.iter().enumerate() {
        for &b in &TRUNK[i + 1..] {
            pull(at, a, b, distance_between(a, b), 1.0);
        }
    }
    for (a, b) in BONES {
        pull(at, a, b, distance_between(a, b), 1.0);
    }
    for (a, b, least) in APART {
        if (at[a] - at[b]).length() < least {
            pull(at, a, b, least, 1.0);
        }
    }
    for (a, b, most) in WITHIN {
        if (at[a] - at[b]).length() > most {
            pull(at, a, b, most, 1.0);
        }
    }
    // A knee bends forward of the line from hip to foot, and an elbow behind the
    // line from shoulder to hand, as seen from the trunk.
    let right = (at[HIP[0]] - at[HIP[1]]).normalize_or(Vec3::X);
    let spine = (at[CHEST] - at[HIPS]).normalize_or(Vec3::Y);
    let forward = right.cross(spine).normalize_or(Vec3::Z);
    for side in 0..2 {
        let middle = (at[HIP[side]] + at[FOOT[side]]) * 0.5;
        let behind = -(at[KNEE[side]] - middle).dot(forward);
        if behind > 0.0 {
            at[KNEE[side]] += forward * (behind * 0.6);
            at[FOOT[side]] -= forward * (behind * 0.3);
        }
        let middle = (at[SHOULDER[side]] + at[HAND[side]]) * 0.5;
        let ahead = (at[ELBOW[side]] - middle).dot(forward);
        if ahead > 0.0 {
            at[ELBOW[side]] -= forward * (ahead * 0.4);
            at[HAND[side]] += forward * (ahead * 0.2);
        }
    }
}

// Sweep point `i` from `from` to `to` as a sphere. Returns where it may be and the
// surface it met, if any.
fn sweep(w: &World, i: usize, from: Vec3, to: Vec3) -> (Vec3, Option<Contact>) {
    let motion = to - from;
    let length = motion.length();
    let shape = Shape::Sphere { radius: RADIUS[i] };
    let hit = (length > 1e-5)
        .then(|| {
            physics::sweep(
                w,
                &shape,
                Transform::at(from.x, from.y, from.z),
                motion,
                SOLID | PROP,
            )
        })
        .flatten()
        .filter(|hit| hit.distance <= length && hit.normal.is_finite() && hit.normal != Vec3::ZERO);
    let Some(hit) = hit else {
        return (to, None);
    };
    let dir = motion / length;
    let stop = from + dir * (hit.distance - 0.002).max(0.0);
    // Slide the rest of the motion along the surface.
    let rest = to - stop;
    let slid = stop + rest - hit.normal * rest.dot(hit.normal).min(0.0);
    let carried = w
        .get::<Body>(hit.entity)
        .map_or(Vec3::ZERO, |body| body.velocity);
    (
        slid,
        Some(Contact {
            point: stop,
            normal: hit.normal,
            carried,
        }),
    )
}

/// Let the body go: the ragdoll starts in the pose the stick man is in, each point
/// moving at `velocity` m/s. `kick` is added at the chest and fades toward the
/// feet, which tips the body the way it was hit.
pub fn start(w: &mut World, velocity: Vec3, kick: Vec3) {
    let root = {
        let player = w.require::<Transform>("player").position;
        let child = *w.require::<Transform>("stickman");
        Transform {
            position: player + child.position,
            ..child
        }
    };
    let joints: Vec<Vec3> = {
        let model = w.model(MODEL).expect("the stick man's model is generated in setup");
        let pose = w.get::<Pose>("stickman");
        JOINTS
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let local = match &pose {
                    Some(pose) => {
                        let node = model.nodes.iter().position(|n| n.name == *name).unwrap();
                        joint_matrix(model, &pose.local, node as u32).w_axis.truncate()
                    }
                    None => REST[i],
                };
                root.position + root.rotation * local
            })
            .collect()
    };
    let lowest = joints.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let tall = (joints[HEAD].y - lowest).max(0.5);
    let velocities = joints
        .iter()
        .map(|p| velocity + kick * ((p.y - lowest) / tall).clamp(0.0, 1.0))
        .collect();
    *w.require_mut::<Ragdoll>("player") = Ragdoll {
        at: joints,
        velocity: velocities,
        still: 0.0,
        age: 0.0,
    };
}

/// Put the ragdoll away.
pub fn stop(w: &mut World) {
    *w.require_mut::<Ragdoll>("player") = Ragdoll::default();
}

/// Advance the ragdoll one tick and carry the capsule with its hips. Returns
/// whether the body has come to rest.
pub fn step(w: &mut World) -> bool {
    let dt = w.dt();
    let h = dt / SUBSTEPS as f32;
    let mut doll = w.require::<Ragdoll>("player").clone();
    for _ in 0..SUBSTEPS {
        let before = doll.at.clone();
        for (p, v) in doll.at.iter_mut().zip(doll.velocity.iter_mut()) {
            v.y -= GRAVITY * h;
            *p += *v * h;
        }
        // Sweep each point along its free motion, then let the constraints pull
        // the body back into shape without pushing a point through what it met.
        let mut contacts: [Option<Contact>; 15] = [None; 15];
        for i in 0..doll.at.len() {
            let (to, contact) = sweep(w, i, before[i], doll.at[i]);
            doll.at[i] = to;
            contacts[i] = contact;
        }
        for _ in 0..ITERATIONS {
            constrain(&mut doll.at);
            for i in 0..doll.at.len() {
                if let Some(c) = contacts[i] {
                    let depth = -(doll.at[i] - c.point).dot(c.normal);
                    if depth > 0.0 {
                        doll.at[i] += c.normal * depth;
                    }
                }
                // The park's floor is the last word: nothing goes under it.
                if doll.at[i].y < RADIUS[i] {
                    doll.at[i].y = RADIUS[i];
                    contacts[i].get_or_insert(Contact {
                        point: doll.at[i],
                        normal: Vec3::Y,
                        carried: Vec3::ZERO,
                    });
                }
            }
        }
        for i in 0..doll.at.len() {
            let mut v = (doll.at[i] - before[i]) / h;
            if let Some(c) = contacts[i] {
                // Relative to the surface: lose the speed into it, and some along it.
                let relative = v - c.carried;
                let into = relative.dot(c.normal);
                let along = relative - c.normal * into;
                v = c.carried + along * (1.0 - FRICTION) + c.normal * (-into * BOUNCE).max(0.0);
            }
            doll.velocity[i] = v;
        }
    }
    doll.age += dt;
    doll.still = if doll.speed() < STILL {
        doll.still + dt
    } else {
        0.0
    };
    let settled = doll.still >= REST_FOR || doll.age >= LIMIT;
    let hips = doll.hips();
    *w.require_mut::<Ragdoll>("player") = doll;
    let mut pose = w.require_mut::<Transform>("player");
    if pose.position != hips {
        pose.position = hips;
    }
    settled
}

/// One bone's place in a pose: its node, and its turn in its parent's frame.
pub struct Bend {
    pub joint: &'static str,
    pub turn: Quat,
}

/// The pose that lays the stick man's bones along the ragdoll's points, for a
/// visual whose root is at `root` and turned `yaw` about Y: the hips' place in the
/// root's frame, and each bone's turn.
pub fn pose(doll: &Ragdoll, root: Vec3, yaw: f32) -> (Vec3, Vec<Bend>) {
    let facing = Quat::from_rotation_y(yaw);
    let trunk = doll.turn();
    let hips = facing.inverse() * (doll.at[HIPS] - root);
    let mut bends = vec![Bend {
        joint: "hips",
        turn: (facing.inverse() * trunk).normalize(),
    }];
    // A bone that rests along `rest` in its parent, turned to point from `a` to `b`.
    let mut aim = |joint: &'static str, parent: Quat, a: usize, b: usize, from: usize, to: usize| {
        let rest = (REST[to] - REST[from]).normalize();
        let want = parent.inverse() * (doll.at[b] - doll.at[a]).normalize_or(parent * rest);
        let turn = Quat::from_rotation_arc(rest, want);
        bends.push(Bend { joint, turn });
        (parent * turn).normalize()
    };
    aim("neck", trunk, CHEST, HEAD, CHEST, HEAD);
    let names = [
        ["upper_arm_l", "forearm_l", "thigh_l", "shin_l"],
        ["upper_arm_r", "forearm_r", "thigh_r", "shin_r"],
    ];
    for side in 0..2 {
        let [upper_arm, forearm, thigh, shin] = names[side];
        let arm = aim(upper_arm, trunk, SHOULDER[side], ELBOW[side], SHOULDER[side], ELBOW[side]);
        aim(forearm, arm, ELBOW[side], HAND[side], ELBOW[side], HAND[side]);
        let leg = aim(thigh, trunk, HIP[side], KNEE[side], HIP[side], KNEE[side]);
        aim(shin, leg, KNEE[side], FOOT[side], KNEE[side], FOOT[side]);
    }
    for joint in ["spine", "chest", "head", "hand_l", "hand_r", "foot_l", "foot_r"] {
        bends.push(Bend {
            joint,
            turn: Quat::IDENTITY,
        });
    }
    (hips, bends)
}

/// Write a pose into the stick man's `Pose`, mixed with what the Animator left
/// there: `weight` one is the given pose alone. Call after `animation::step`.
pub fn show(w: &mut World, hips: Vec3, bends: &[Bend], weight: f32) {
    let nodes: Vec<(usize, Quat)> = {
        let model = w.model(MODEL).expect("the stick man's model is generated in setup");
        bends
            .iter()
            .map(|bend| {
                let node = model.nodes.iter().position(|n| n.name == bend.joint).unwrap();
                (node, bend.turn)
            })
            .collect()
    };
    let Some(mut pose) = w.get_mut::<Pose>("stickman") else {
        return;
    };
    for (i, (node, turn)) in nodes.into_iter().enumerate() {
        let slot = &mut pose.local[node * 10..node * 10 + 10];
        let was = Quat::from_slice(&slot[3..7]);
        slot[3..7].copy_from_slice(&was.slerp(turn, weight).normalize().to_array());
        // The first bend is the hips, which also carry the body's place.
        if i == 0 {
            let place = Vec3::from_slice(&slot[0..3]).lerp(hips, weight);
            slot[0..3].copy_from_slice(&place.to_array());
        }
    }
}

/// Where the visual's root is while the ragdoll runs: under the capsule, which
/// rides the hips.
pub fn root(w: &World) -> Vec3 {
    w.require::<Transform>("player").position - Vec3::Y * (HALF + 0.01)
}
