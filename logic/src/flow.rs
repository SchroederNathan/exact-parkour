//! Carries the pose of one move into the next.
//!
//! Each move has its own poses, so a change of move is a change of pose in one
//! tick. `carry` knows what was shown. At a change it measures how far every joint
//! is from where the new move has it, and how fast that joint was turning, and
//! then lets the difference die away as a spring at rest would. The limbs go on
//! the way they were going and arrive at the new move: nothing stops and nothing
//! jumps. A fade from a frozen pose does both.
use exact_game::*;

/// How fast a joint may be taken to turn at a change of move, rad/s, and to shift,
/// m/s. A faster one is a pose that was itself in a change.
const TURN_MAX: f32 = 12.0;
const SHIFT_MAX: f32 = 3.0;
/// A difference smaller than this is none, radians and metres.
const NONE: f32 = 1e-4;

/// What is left of the pose the body had before its last change of move.
#[derive(Default, Clone, Component)]
pub struct Flow {
    /// The pose shown two ticks ago, ten floats per node.
    before: Vec<f32>,
    /// For each node, what it is still off the pose its move asks for: a turn
    /// as a rotation vector and its rate, then a shift and its rate.
    off: Vec<f32>,
    /// How fast that dies away, per second: about 1.7 over this is the time to
    /// half, and 4 over it the time to a tenth.
    rate: f32,
    /// The move shown last tick, as `Act::code`.
    pub act: f32,
}

// The turn `q` as a rotation vector: its axis, as long as its angle in radians.
fn log(q: Quat) -> Vec3 {
    let q = if q.w < 0.0 { -q } else { q };
    let v = Vec3::new(q.x, q.y, q.z);
    let sin = v.length();
    if sin < 1e-6 {
        return v * 2.0;
    }
    v * (2.0 * math::atan2(sin, q.w) / sin)
}

fn exp(r: Vec3) -> Quat {
    let angle = r.length();
    if angle < 1e-6 {
        return Quat::from_xyzw(r.x * 0.5, r.y * 0.5, r.z * 0.5, 1.0).normalize();
    }
    let (sin, cos) = math::sin_cos(angle * 0.5);
    let v = r * (sin / angle);
    Quat::from_xyzw(v.x, v.y, v.z, cos)
}

fn turn_of(pose: &[f32], node: usize) -> Quat {
    Quat::from_slice(&pose[node * 10 + 3..node * 10 + 7])
}
fn shift_of(pose: &[f32], node: usize) -> Vec3 {
    Vec3::from_slice(&pose[node * 10..node * 10 + 3])
}

/// Forget the pose before: what the Animator has is shown as it is.
pub fn settle(w: &mut World, act: f32) {
    let Some(mut flow) = w.get_mut::<Flow>("stickman") else {
        return;
    };
    flow.off.clear();
    flow.before.clear();
    flow.act = act;
}

/// Lay what is left of the last pose over the one the Animator and the trunk's
/// own motion made this tick. Call before the holds, which then reach from where
/// this left the shoulders and hips. With `cut`, the pose changed its move this
/// tick: start again from what was shown, and die away at that rate.
pub fn carry(w: &mut World, act: f32, cut: Option<f32>) {
    let dt = w.dt();
    let Some((mut local, shown)) = w
        .get::<Pose>("stickman")
        .map(|pose| (pose.local.clone(), pose.previous.clone()))
    else {
        return;
    };
    if local.is_empty() || shown.len() != local.len() {
        return;
    }
    let Some(mut flow) = w.get::<Flow>("stickman").map(|f| f.clone()) else {
        return;
    };
    let nodes = local.len() / 10;
    if flow.off.len() != nodes * 12 {
        flow.off = vec![0.0; nodes * 12];
    }
    if let Some(rate) = cut {
        flow.rate = rate;
        let moving = flow.before.len() == local.len();
        for node in 0..nodes {
            let to = turn_of(&local, node).inverse();
            let turn = log(turn_of(&shown, node) * to);
            let shift = shift_of(&shown, node) - shift_of(&local, node);
            // How fast the shown pose was going, which the new one takes over.
            let (turning, shifting) = if moving {
                (
                    (turn - log(turn_of(&flow.before, node) * to)) / dt,
                    (shift_of(&shown, node) - shift_of(&flow.before, node)) / dt,
                )
            } else {
                (Vec3::ZERO, Vec3::ZERO)
            };
            let turning = if turning.length() > TURN_MAX * 3.0 {
                Vec3::ZERO
            } else {
                turning.clamp_length_max(TURN_MAX)
            };
            let row = &mut flow.off[node * 12..node * 12 + 12];
            row[..3].copy_from_slice(&turn.to_array());
            row[3..6].copy_from_slice(&turning.to_array());
            row[6..9].copy_from_slice(&shift.to_array());
            row[9..].copy_from_slice(&shifting.clamp_length_max(SHIFT_MAX).to_array());
        }
    }
    // A spring with no bounce, exactly: what is off goes to nothing and does
    // not pass it unless it came fast.
    let rate = flow.rate.max(1.0);
    let decay = math::exp(-rate * dt);
    let mut any = false;
    for node in 0..nodes {
        let row = &mut flow.off[node * 12..node * 12 + 12];
        for part in 0..2 {
            let at = part * 6;
            let x = Vec3::from_slice(&row[at..at + 3]);
            let v = Vec3::from_slice(&row[at + 3..at + 6]);
            if x == Vec3::ZERO && v == Vec3::ZERO {
                continue;
            }
            let pull = v + x * rate;
            let (mut x, mut v) = ((x + pull * dt) * decay, (v - pull * (rate * dt)) * decay);
            if x.length() < NONE && v.length() < NONE * rate {
                (x, v) = (Vec3::ZERO, Vec3::ZERO);
            }
            row[at..at + 3].copy_from_slice(&x.to_array());
            row[at + 3..at + 6].copy_from_slice(&v.to_array());
            if x == Vec3::ZERO {
                continue;
            }
            any = true;
            if part == 0 {
                let turned = (exp(x) * turn_of(&local, node)).normalize();
                local[node * 10 + 3..node * 10 + 7].copy_from_slice(&turned.to_array());
            } else {
                let moved = shift_of(&local, node) + x;
                local[node * 10..node * 10 + 3].copy_from_slice(&moved.to_array());
            }
        }
    }
    flow.before = shown;
    flow.act = act;
    *w.require_mut::<Flow>("stickman") = flow;
    if any {
        w.require_mut::<Pose>("stickman").local = local;
    }
}
