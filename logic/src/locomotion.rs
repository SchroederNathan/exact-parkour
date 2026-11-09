//! Drives the stick man's Animator from what his body is doing.
//!
//! Two numbers are kept apart. The gait's shape (`Stride::pace`, m/s) follows ground
//! speed over a fraction of a second, so the legs open and close smoothly however
//! hard the body accelerates. The gait's clock follows the ground: each tick the
//! cycle advances by exactly what keeps the planted foot where it was put, counting
//! both the metres the body covered and how far the change of shape moved that foot.
//! From a stand that makes the first motion a push off the planted foot while the
//! other swings through; to a stand, the last step closes onto it.
use crate::gait::{self, PACES};
use crate::stickman::LAND_LAYER;
use exact_game::animation::Layers;
use exact_game::character::Contact;
use exact_game::*;

/// The gait the legs are shaped for, and how hard the body is coming down.
#[derive(Default, Component)]
pub struct Stride {
    /// Speed the legs are shaped for, m/s; trails ground speed.
    pub pace: f32,
    /// Downward speed of the last airborne tick, m/s.
    pub sink: f32,
}

// How `pace` closes on ground speed: a share of the gap per second, plus a floor
// that makes it arrive. Opening the stride is quicker than closing it.
const OPEN: f32 = 14.0;
const CLOSE: f32 = 9.0;
const ARRIVE: f32 = 6.0;
/// Cycles per second the legs never exceed, however short the stride.
const CADENCE_MAX: f32 = 2.6;
/// How fast (m/s) an opening stride may drag the planted foot over the ground.
/// Beyond it the stride waits for the body to cover the distance.
const DRAG: f32 = 0.15;
/// Ground speed (m/s) under which the body counts as stopped.
const STILL: f32 = 0.1;
/// Downward speed (m/s) of a full landing crouch.
const HARD_LANDING: f32 = 5.0;

/// The stride (metres per cycle) and period (seconds per cycle) of the blend at `pace`.
pub fn stride_at(pace: f32) -> (f32, f32) {
    let (first, last) = (PACES[0], PACES[PACES.len() - 1]);
    if pace <= first.speed {
        // Toward the stance the steps shorten and keep their rhythm.
        return (first.stride * (pace / first.speed).max(0.0), first.period());
    }
    if pace >= last.speed {
        return (last.stride, last.period());
    }
    let hi = PACES.partition_point(|p| p.speed <= pace);
    let (a, b) = (PACES[hi - 1], PACES[hi]);
    let k = (pace - a.speed) / (b.speed - a.speed);
    (
        math::lerp(a.stride, b.stride, k),
        math::lerp(a.period(), b.period(), k),
    )
}

// How far a planted foot moves, relative to the hips, when the blend's shape goes
// `from` one pace `to` another at the same phase. `None` in a flight, or when the
// change lifts every foot. An opening stride answers to the foot with more of its
// stance to come, which will carry the body longest. A closing one answers to the
// foot the change pulls furthest forward: the trailing foot, which the clock then
// carries to its lift-off so that it steps in and does not slide in.
fn shift(from: f32, to: f32, phase: f32) -> Option<f32> {
    let (a, b) = (gait::planted(from, phase), gait::planted(to, phase));
    let feet = (0..2).filter_map(|leg| Some((a[leg]?, b[leg]?)));
    if to < from {
        feet.map(|(a, b)| b.0 - a.0).reduce(f32::max)
    } else {
        feet.max_by(|x, y| x.1 .1.total_cmp(&y.1 .1))
            .map(|(a, b)| b.0 - a.0)
    }
}

// How far the cycle is from the next phase at which one foot is under the hips.
fn to_ready(phase: f32) -> f32 {
    let ahead = gait::ready() - phase;
    let ahead = ahead - math::floor(ahead * 2.0) * 0.5;
    ahead.clamp(0.0, 0.5)
}

/// Set `target`'s Animator and layers for this tick; call before `animation::step`.
/// `speed` is ground speed in m/s and `rising` vertical velocity in m/s.
pub fn drive(w: &mut World, target: &str, speed: f32, rising: f32, contact: Contact) {
    let dt = w.dt();
    let ground = speed * dt;
    // The phase the legs were last shown at; none before the first pose, and in the
    // air the feet answer to nothing.
    let phase = w
        .get::<Pose>(target)
        .map(|pose| pose.phase)
        .filter(|_| contact.grounded && !contact.landed);
    let (was, pace, sink) = {
        let mut stride = w.require_mut::<Stride>(target);
        let was = stride.pace;
        let gap = speed - was;
        let step = (gap.abs() * if gap > 0.0 { OPEN } else { CLOSE } + ARRIVE) * dt;
        let mut pace = if gap.abs() <= step {
            speed
        } else {
            was + step * gap.signum()
        };
        // An opening stride reaches back for ground the body has not covered yet.
        if let Some(back) = phase
            .filter(|_| pace > was)
            .and_then(|phase| shift(was, pace, phase))
            .map(|moved| -moved)
        {
            let paid = ground + DRAG * dt;
            if back > paid {
                pace = was + (pace - was) * paid / back;
            }
        }
        let sink = if contact.grounded {
            stride.sink
        } else {
            (-rising).max(0.0)
        };
        if stride.pace != pace || stride.sink != sink {
            stride.pace = pace;
            stride.sink = sink;
        }
        (was, pace, sink)
    };

    let (metres, period) = stride_at(pace);
    let most = CADENCE_MAX * dt;
    let mut advance = if metres <= 0.02 {
        0.0
    } else {
        match phase.and_then(|phase| shift(was, pace, phase)) {
            // The planted foot must end `ground` metres further back; the change of
            // shape already moved it `moved`, and the clock supplies the rest.
            Some(moved) => ((moved + ground) / metres).clamp(0.0, most),
            None => (ground / metres).min(most),
        }
    };
    // Closing into a stand from a stop: finish the step in hand, so the cycle
    // arrives where one foot is under the hips as the stride reaches nothing.
    if let Some(phase) = phase.filter(|_| speed < STILL && pace < was && pace < PACES[0].speed) {
        advance = advance.max(to_ready(phase) * (was - pace) / was).min(most);
    }
    {
        let mut animator = w.require_mut::<Animator>(target);
        animator.set("pace", pace);
        animator.set("lift", rising);
        animator.set("air", !contact.grounded);
        // The blend's clock runs `rate` times its own period.
        let state = animator
            .state_mut("move")
            .expect("the stick man's Animator has a `move` state");
        let rate = advance * period / dt;
        if state.speed != rate {
            state.speed = rate;
        }
    }
    // At a stand every phase looks the same, so this is the moment to pick the one
    // the next step should start from.
    if pace == 0.0 && contact.grounded {
        let ready = gait::ready();
        if let Some(mut pose) = w.get_mut::<Pose>(target) {
            if pose.phase != ready {
                pose.phase = ready;
            }
        }
    }
    if contact.landed {
        let mut layers = w.require_mut::<Layers>(target);
        let land = &mut layers.0[LAND_LAYER];
        land.weight = (sink / HARD_LANDING).clamp(0.35, 1.0);
        land.animation.time = 0.0;
    }
}
