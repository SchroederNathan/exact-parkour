//! Gait clips solved from where the feet are, not from how the joints swing.
//!
//! Each foot follows a path in the body's side view. On the ground it moves back at
//! exactly the body's speed, so it is still in the world; near the end of its stance
//! the heel peels up about the toe; in the air it leaves and arrives at that same
//! backward speed, so it neither skates before a step nor drags after one. The hips
//! ride at the height the planted legs can reach, and fly a ballistic arc when no leg
//! is planted. A two-bone solve turns each ankle position into hip and knee turns.
use exact_game::asset::{Clip, Interpolation, Track, TrackPath};
use exact_game::rig::Rig;
use exact_game::*;
use std::f32::consts::{PI, TAU};

/// Samples per cycle. Every duty below is a whole number of them, as are the two
/// legs' phases, so each touchdown and lift-off falls exactly on a key.
const SAMPLES: usize = 48;
/// Stance progress at which the heel starts to peel.
const HEEL_OFF: f32 = 0.55;
/// The furthest a leg extends, as a share of its full length; it never locks.
const EXTENT: f32 = 0.995;
const GRAVITY: f32 = 9.81;
/// How late, as a share of the cycle, each part of the upper body follows the one
/// that moves it.
pub const ELBOW_LAG: f32 = 0.09;
const WRIST_LAG: f32 = 0.17;
const CHEST_LAG: f32 = 0.04;
/// The shares of a pace's lean that the pelvis, the spine and the chest take.
const LEAN: [f32; 3] = [0.3, 0.45, 0.25];
/// How far the pelvis drops on the side of the leg that swings, radians, at a
/// walk's arm swing.
const LIST: f32 = 0.055;

/// One gait, authored for `speed` m/s.
#[derive(Clone, Copy)]
pub struct Pace {
    pub speed: f32,
    /// Metres per cycle (two steps).
    pub stride: f32,
    /// Samples of the cycle each foot is planted; under half, the body flies.
    planted: usize,
    /// Share of the planted travel that lies ahead of the hip at touchdown.
    front: f32,
    /// Peak height of the swinging ankle, metres.
    lift: f32,
    /// Toe-down pitch the foot reaches at lift-off, radians.
    heel: f32,
    /// How far the hips sit below a straight-legged stand at mid-stance, metres.
    sink: f32,
    /// Arm swing as a share of a walk's.
    arms: f32,
    /// Elbow bend, radians.
    elbow: f32,
    /// Forward lean of the trunk, radians: the pelvis, the spine and the chest
    /// each take a share of it.
    lean: f32,
}
impl Pace {
    /// Seconds per cycle.
    pub fn period(&self) -> f32 {
        self.stride / self.speed
    }
    /// The clip's name in the model.
    pub fn clip(&self) -> String {
        format!("gait-{}", (self.speed * 10.0) as u32)
    }
    fn duty(&self) -> f32 {
        self.planted as f32 / SAMPLES as f32
    }
}

#[allow(clippy::too_many_arguments)]
const fn pace(
    speed: f32,
    stride: f32,
    planted: usize,
    front: f32,
    lift: f32,
    heel: f32,
    sink: f32,
    arms: f32,
    elbow: f32,
    lean: f32,
) -> Pace {
    Pace {
        speed,
        stride,
        planted,
        front,
        lift,
        heel,
        sink,
        arms,
        elbow,
        lean,
    }
}

/// Top ground speed without sprint, m/s.
pub const RUN: f32 = 3.4;
/// Top ground speed with sprint held, m/s.
pub const SPRINT: f32 = 6.2;

/// Shuffle, walk, jog, run, fast run, sprint. Neighbours differ a little in every
/// value, so a blend of two is itself a sound gait. The distance a planted foot
/// travels under the body stays near 0.9 m from the walk up: speed comes from
/// cadence and from flight, as it does for a runner.
pub const PACES: [Pace; 6] = [
    pace(0.8, 0.90, 30, 0.45, 0.05, 0.45, 0.012, 0.35, 0.25, 0.02),
    pace(1.5, 1.36, 28, 0.42, 0.09, 0.65, 0.020, 0.60, 0.30, 0.04),
    pace(2.4, 1.80, 23, 0.36, 0.16, 0.85, 0.040, 0.80, 0.85, 0.08),
    pace(RUN, 2.30, 19, 0.32, 0.24, 0.95, 0.055, 1.00, 1.15, 0.14),
    pace(4.8, 2.95, 15, 0.30, 0.36, 1.00, 0.060, 1.40, 1.18, 0.24),
    pace(SPRINT, 3.50, 12, 0.30, 0.50, 1.00, 0.065, 1.75, 1.20, 0.33),
];

// A leg in the side view. Angles are measured from straight down, forward positive.
struct Leg {
    bones: [u32; 3],
    phase: f32,
    /// Hip to knee and knee to ankle, metres.
    upper: f32,
    lower: f32,
    /// Rest angles of those two segments.
    rest: [f32; 2],
    /// Hip height above the ankle at rest.
    stand: f32,
    /// The toe from the ankle at rest: (up, forward).
    toe: (f32, f32),
}

// Where an ankle is: metres ahead of the hip, metres above its rest height, and the
// foot's toe-down pitch.
#[derive(Clone, Copy)]
struct Ankle {
    ahead: f32,
    up: f32,
    pitch: f32,
}

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}
fn wrap(x: f32) -> f32 {
    x - math::floor(x)
}
fn forward_angle(v: Vec3) -> f32 {
    math::atan2(v.z, -v.y)
}

impl Leg {
    fn new(rig: &Rig, side: &str, phase: f32) -> Self {
        let find = |name: &str| {
            let name = format!("{name}_{side}");
            rig.bones()
                .iter()
                .position(|b| b.name == name)
                .unwrap_or_else(|| panic!("gait: no bone `{name}`"))
        };
        let bones = [find("thigh"), find("shin"), find("foot")];
        let [hip, knee, ankle] = bones.map(|b| rig.bones()[b].head);
        let toe = rig.bones()[bones[2]].tail - ankle;
        Self {
            bones: bones.map(|b| b as u32),
            phase,
            upper: (knee - hip).length(),
            lower: (ankle - knee).length(),
            rest: [forward_angle(knee - hip), forward_angle(ankle - knee)],
            stand: hip.y - ankle.y,
            toe: (toe.y, toe.z),
        }
    }

    // The ankle `u` of the way through its stance: flat and still on the ground,
    // then rolling up and forward about the toe, which stays where it is.
    fn planted(&self, p: &Pace, u: f32) -> Ankle {
        let travel = p.duty() * p.stride;
        let flat = travel * (p.front - u);
        let pitch = p.heel * smooth((u - HEEL_OFF) / (1.0 - HEEL_OFF));
        let (sin, cos) = math::sin_cos(pitch);
        let (up, ahead) = self.toe;
        Ankle {
            ahead: flat + ahead - (up * sin + ahead * cos),
            up: up - (up * cos - ahead * sin),
            pitch,
        }
    }

    // The ankle `u` of the way through its swing, from lift-off to touchdown. A
    // Hermite curve whose end slopes are the ground's speed past the body: the foot
    // kicks back as it leaves and is already moving back as it lands.
    fn swinging(&self, p: &Pace, u: f32) -> Ankle {
        let (from, to) = (self.planted(p, 1.0), self.planted(p, 0.0));
        let span = to.ahead - from.ahead;
        let slope = -(p.stride * (1.0 - p.duty())).min(3.0 * span);
        let ahead = from.ahead + span * smooth(u) + slope * u * (1.0 - u) * (1.0 - 2.0 * u);
        let arc = math::sin(PI * u) * (1.0 + 0.3 * math::cos(PI * u));
        let toes_up = 0.22 * math::sin(PI * ((u - 0.5) / 0.5).max(0.0));
        Ankle {
            ahead,
            up: from.up * (1.0 - u) * (1.0 - u) + p.lift * arc,
            pitch: from.pitch * (1.0 - smooth(u / 0.6)) - toes_up,
        }
    }

    // `Some` while planted. `cycle` is the gait's phase; the leg's own starts at touchdown.
    fn ankle(&self, p: &Pace, cycle: f32) -> (Ankle, bool) {
        let c = wrap(cycle - self.phase);
        let duty = p.duty();
        if c < duty {
            (self.planted(p, c / duty), true)
        } else {
            (self.swinging(p, (c - duty) / (1.0 - duty)), false)
        }
    }

    // How far the hips must sit below a stand for this ankle to be in reach.
    fn needs(&self, ankle: Ankle) -> f32 {
        let reach = (self.upper + self.lower) * EXTENT;
        self.stand - ankle.up - math::sqrt((reach * reach - ankle.ahead * ankle.ahead).max(0.0))
    }

    // Turns of the thigh, shin and foot about X that put the ankle there, with the
    // hips `drop` metres below a stand. The knee always bends forward.
    fn solve(&self, ankle: Ankle, drop: f32) -> [f32; 3] {
        let (a, b) = (self.upper, self.lower);
        let below = self.stand - drop - ankle.up;
        let distance = math::sqrt(ankle.ahead * ankle.ahead + below * below)
            .clamp((a - b).abs() + 0.01, (a + b) * 0.999);
        let line = math::atan2(ankle.ahead, below);
        let corner = |near: f32, far: f32| {
            math::acos(
                ((near * near + distance * distance - far * far) / (2.0 * near * distance))
                    .clamp(-1.0, 1.0),
            )
        };
        let thigh = line + corner(a, b);
        let shin = line - corner(b, a);
        // A turn about X swings a hanging bone backward, so each is the rest angle
        // less the wanted one, less the turns already above it.
        let thigh_turn = self.rest[0] - thigh;
        let shin_turn = self.rest[1] - shin - thigh_turn;
        [thigh_turn, shin_turn, ankle.pitch - thigh_turn - shin_turn]
    }
}

// Fill the gaps of a cyclic series (the flights, when no leg is planted) with a
// straight line between the values either side, less a ballistic arc: the hips
// rise and fall under gravity for as long as the gap lasts.
fn fly(held: &[Option<f32>], seconds_per_sample: f32) -> Vec<f32> {
    let n = held.len() as isize;
    let at = |i: isize| held[i.rem_euclid(n) as usize];
    (0..n)
        .map(|i| {
            if let Some(v) = at(i) {
                return v;
            }
            let (mut before, mut after) = (i, i);
            while at(before).is_none() && i - before < n {
                before -= 1;
            }
            while at(after).is_none() && after - i < n {
                after += 1;
            }
            let (Some(a), Some(b)) = (at(before), at(after)) else {
                return 0.0;
            };
            let u = (i - before) as f32 / (after - before) as f32;
            let gap = (after - before) as f32 * seconds_per_sample;
            a + (b - a) * u - 0.5 * GRAVITY * (u * gap) * ((1.0 - u) * gap)
        })
        .collect()
}

fn turns(node: u32, times: &[f32], at: impl Fn(usize) -> Quat) -> Track {
    Track {
        node,
        path: TrackPath::Rotation,
        interpolation: Interpolation::Linear,
        times: times.to_vec(),
        values: (0..times.len()).flat_map(|i| at(i).to_array()).collect(),
    }
}

/// A looping, in-place gait cycle of `pace.period()` seconds for the stick man's rig.
pub fn clip(rig: &Rig, pace: &Pace) -> Clip {
    let node = |name: &str| {
        rig.bones()
            .iter()
            .position(|b| b.name == name)
            .unwrap_or_else(|| panic!("gait: no bone `{name}`"))
    };
    let period = pace.period();
    let times: Vec<f32> = (0..=SAMPLES)
        .map(|i| period * i as f32 / SAMPLES as f32)
        .collect();
    let cycle = |i: usize| (i % SAMPLES) as f32 / SAMPLES as f32;
    let legs = [Leg::new(rig, "l", 0.0), Leg::new(rig, "r", 0.5)];

    // The hips: low enough for every planted leg, never above the mid-stance sink.
    let held: Vec<Option<f32>> = (0..SAMPLES)
        .map(|i| {
            legs.iter()
                .filter_map(|leg| {
                    let (ankle, planted) = leg.ankle(pace, cycle(i));
                    planted.then(|| leg.needs(ankle))
                })
                .reduce(f32::max)
                .map(|need| need.max(pace.sink))
        })
        .collect();
    let drops = fly(&held, period / SAMPLES as f32);
    let drop = |i: usize| drops[i % SAMPLES];

    let hips = node("hips");
    let rest = rig.bones()[hips].head
        - rig.bones()[hips]
            .parent
            .map_or(Vec3::ZERO, |p| rig.bones()[p].head);
    let mut tracks = vec![Track {
        node: hips as u32,
        path: TrackPath::Translation,
        interpolation: Interpolation::Linear,
        times: times.clone(),
        values: (0..times.len())
            .flat_map(|i| (rest - Vec3::Y * drop(i)).to_array())
            .collect(),
    }];
    // The pelvis tips forward with the lean, and drops on the side of the leg that
    // swings: a leg's side is highest in the middle of the time that leg stands
    // alone. The thighs undo both turns, so the feet stay where the solve put them.
    let pitch = LEAN[0] * pace.lean;
    let list = |i: usize| {
        LIST * (0.4 + 0.45 * pace.arms) * math::cos(TAU * (cycle(i) - 0.5 * pace.duty()))
    };
    for leg in &legs {
        for (joint, &bone) in leg.bones.iter().enumerate() {
            tracks.push(turns(bone, &times, |i| {
                let (ankle, _) = leg.ankle(pace, cycle(i));
                let turn = leg.solve(ankle, drop(i))[joint];
                if joint == 0 {
                    Quat::from_rotation_z(-list(i)) * Quat::from_rotation_x(turn - pitch)
                } else {
                    Quat::from_rotation_x(turn)
                }
            }));
        }
    }

    // Each arm swings against the leg on its side: furthest forward as the opposite
    // foot lands. The swing travels down the arm as a wave. The elbow follows the
    // shoulder late and the wrist follows the elbow later, so the forearm whips
    // through and the hand trails. The hand crosses toward the midline in front and
    // the elbow flares out behind, as a runner's does.
    let swing = |i: usize, phase: f32| math::cos(TAU * (cycle(i) - phase));
    for (side, out, phase) in [("l", 1.0, 0.5), ("r", -1.0, 0.0)] {
        tracks.push(turns(
            node(&format!("upper_arm_{side}")) as u32,
            &times,
            |i| {
                let s = swing(i, phase);
                // Further back than forward: the elbow drives the stride.
                let reach = if s > 0.0 { 0.42 } else { 0.56 };
                let across = if s > 0.0 { -0.14 * s } else { -0.10 * s };
                Quat::from_rotation_z(out * (0.05 + pace.arms * across))
                    * Quat::from_rotation_x(-reach * pace.arms * s)
            },
        ));
        tracks.push(turns(node(&format!("forearm_{side}")) as u32, &times, |i| {
            let late = swing(i, phase + ELBOW_LAG);
            Quat::from_rotation_x(-pace.elbow - 0.45 * pace.arms * (0.5 + 0.5 * late))
        }));
        tracks.push(turns(node(&format!("hand_{side}")) as u32, &times, |i| {
            Quat::from_rotation_x(-0.3 * pace.arms * swing(i, phase + WRIST_LAG))
        }));
    }
    // The hips turn with the stride. The trunk leans in over them and turns back
    // against them, the chest turns further, and the shoulders tip the other way
    // from the pelvis: down on the side of the planted foot. The neck undoes most
    // of it, so the head keeps level and faces the way the body goes.
    let twist = |i: usize| pace.arms * swing(i, 0.0);
    tracks.push(turns(hips as u32, &times, |i| {
        Quat::from_rotation_y(0.08 * twist(i))
            * Quat::from_rotation_x(pitch)
            * Quat::from_rotation_z(list(i))
    }));
    tracks.push(turns(node("spine") as u32, &times, |i| {
        Quat::from_rotation_x(LEAN[1] * pace.lean)
            * Quat::from_rotation_y(-0.1 * twist(i))
            * Quat::from_rotation_z(-0.6 * list(i))
    }));
    tracks.push(turns(node("chest") as u32, &times, |i| {
        Quat::from_rotation_x(LEAN[2] * pace.lean)
            * Quat::from_rotation_y(-0.07 * pace.arms * swing(i, CHEST_LAG))
            * Quat::from_rotation_z(-0.8 * list(i))
    }));
    tracks.push(turns(node("neck") as u32, &times, |i| {
        Quat::from_rotation_y(0.02 * twist(i) + 0.07 * pace.arms * swing(i, CHEST_LAG))
            * Quat::from_rotation_x(-0.6 * pace.lean)
            * Quat::from_rotation_z(0.4 * list(i))
    }));
    Clip {
        name: pace.clip(),
        tracks,
        markers: vec![],
    }
}

/// Where the blend at `pace` (m/s) has each foot on the ground at cycle phase
/// `cycle`: metres ahead of the hip, and the share of its stance still to come.
/// `None` for a foot that any clip of the blend has in the air. Left, then right.
pub fn planted(pace: f32, cycle: f32) -> [Option<(f32, f32)>; 2] {
    let (first, last) = (&PACES[0], &PACES[PACES.len() - 1]);
    // The stance below the slowest gait keeps both feet under the hips.
    let knots: [(Option<&Pace>, f32); 2] = if pace <= first.speed {
        let k = (pace / first.speed).max(0.0);
        [(None, 1.0 - k), (Some(first), k)]
    } else if pace >= last.speed {
        [(Some(last), 1.0), (None, 0.0)]
    } else {
        let hi = PACES.partition_point(|p| p.speed <= pace);
        let (a, b) = (&PACES[hi - 1], &PACES[hi]);
        let k = (pace - a.speed) / (b.speed - a.speed);
        [(Some(a), 1.0 - k), (Some(b), k)]
    };
    [0.0, 0.5].map(|phase| {
        let c = wrap(cycle - phase);
        let (mut ahead, mut left) = (0.0, 1.0f32);
        for (knot, weight) in knots {
            let Some(p) = knot.filter(|_| weight > 0.0) else {
                continue;
            };
            let u = c / p.duty();
            if u >= 1.0 {
                return None;
            }
            ahead += weight * p.duty() * p.stride * (p.front - u);
            left = left.min(1.0 - u);
        }
        Some((ahead, left))
    })
}

/// The phase to stand at: the left foot planted directly under the hip in the
/// slowest gait and the right mid-swing, so the first step from a stand is a step.
pub fn ready() -> f32 {
    PACES[0].front * PACES[0].duty()
}
