//! The climb onto a top: one course from a hang to a stand, which a body joins
//! where it is.
//!
//! A body that hangs from the edge starts the course at its beginning: it pulls
//! up, presses up over the edge, puts a foot on the top and stands. A body that
//! meets a lower wall from the ground, or catches a lower edge out of the air,
//! is already part of the way up. It joins the course at its own height, so its
//! pose is the pose of a body that high on the wall, whatever way it came.
//!
//! The body is rarely on the course when it joins, and it is moving. What it is
//! off the course, and how fast, dies away as a spring at rest would: a run at
//! the wall carries the body to it, and a fall onto an edge sags and comes back.
use crate::body::{Act, Motion, HALF, RADIUS};
use crate::ledge::{Ledge, HANG_REACH};
use crate::stickman::through;
use exact_game::*;

/// Seconds of the whole course, and the moments on it: the chin is at the edge,
/// the arms have pressed the hips up over it, the body crouches on the top.
pub const SPAN: f32 = 1.08;
pub const PULLED: f32 = 0.3;
pub const PRESSED: f32 = 0.62;
pub const CROUCHED: f32 = 0.86;

// The capsule's centre at each moment: metres out from the wall's face, and up
// from its edge. The first is a hang and the last a stand.
const TIMES: [f32; 7] = [0.0, 0.1, PULLED, PRESSED, CROUCHED, 0.98, SPAN];
const OUT: [f32; 7] = [RADIUS + 0.04, RADIUS + 0.04, 0.32, 0.24, -0.25, -0.38, -(RADIUS + 0.15)];
const UP: [f32; 7] = [HALF - HANG_REACH, HALF - HANG_REACH + 0.03, -0.5, 0.2, 0.62, 0.88, HALF + 0.02];

/// How fast what the body is off the course dies away, per second.
const RATE: f32 = 13.0;
/// The body leaves the climb walking in at this speed, m/s.
const EXIT: f32 = 1.2;

/// A body on its way up onto a top.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
pub struct Climb {
    /// The edge it climbs.
    pub ledge: Ledge,
    /// Seconds along the course, and where the body joined it.
    pub time: f32,
    pub joined: f32,
    /// What the capsule is off the course, metres, and how fast that changes.
    pub off: Vec3,
    pub drift: Vec3,
    /// Height of the edge above the feet when the climb began, metres.
    pub height: f32,
    /// The foot that steps onto the top first: +1 the left, -1 the right.
    pub first: f32,
    /// Out of a hang: how close to the wall the drawn body starts, how far the
    /// hang had shimmied, which is where its hands are, and how far it leaned
    /// past the end of the edge.
    pub hug: f32,
    pub travel: f32,
    pub lean: f32,
}

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// Where the capsule's centre is `time` seconds along the course up `ledge`.
pub fn course(ledge: &Ledge, time: f32) -> Vec3 {
    let time = time.clamp(0.0, SPAN);
    ledge.edge + ledge.out * through(&TIMES, &OUT, time) + Vec3::Y * through(&TIMES, &UP, time)
}

// The moment of the course at which the centre is `up` metres over the edge. A
// body that is nearly up still has a moment's climb to do.
fn moment(up: f32) -> f32 {
    let (mut low, mut high) = (0.0, SPAN - 0.24);
    for _ in 0..24 {
        let middle = (low + high) * 0.5;
        if through(&TIMES, &UP, middle) < up {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low + high) * 0.5
}

impl Climb {
    /// A climb by a body whose capsule is centred at `centre` and moves at
    /// `velocity`, from the ground or out of the air.
    pub fn join(centre: Vec3, velocity: Vec3, ledge: &Ledge, first: f32) -> Self {
        let joined = moment(centre.y - ledge.edge.y);
        let going = (course(ledge, joined + 0.01) - course(ledge, joined - 0.01)) / 0.02;
        Climb {
            ledge: *ledge,
            time: joined,
            joined,
            off: centre - course(ledge, joined),
            drift: velocity - going,
            height: ledge.edge.y - (centre.y - HALF),
            first,
            ..Climb::default()
        }
    }

    /// A climb out of a hang at `centre`.
    pub fn from_hang(centre: Vec3, drift: Vec3, ledge: &Ledge, first: f32) -> Self {
        Climb {
            ledge: *ledge,
            off: centre - course(ledge, 0.0),
            drift,
            height: HANG_REACH,
            first,
            ..Climb::default()
        }
    }

    pub fn progress(&self) -> f32 {
        (self.time / SPAN).clamp(0.0, 1.0)
    }

    /// What is left of the lean a climb out of a hang began with: it straightens
    /// first.
    pub fn leaning(&self) -> f32 {
        self.lean * (1.0 - smooth(self.time / 0.3))
    }

    pub fn motion(&self) -> Motion {
        Motion {
            act: Act::Climb,
            progress: self.progress(),
            height: self.height,
            speed: EXIT,
            ..Motion::default()
        }
    }

    /// Go on for `dt` seconds. Returns where the capsule's centre is, and the
    /// velocity the body leaves with when the climb is over.
    pub fn step(&mut self, dt: f32) -> (Vec3, Option<Vec3>) {
        self.time += dt;
        // A body that joins late has less time to get onto the course.
        let rate = RATE.max(4.5 / (SPAN - self.joined));
        (self.off, self.drift) = settle(self.off, self.drift, rate, dt);
        // The wall stops what is left of a run at it.
        let inward = self.ledge.inward();
        let into = self.off.dot(inward);
        if into > 0.0 {
            self.off -= inward * into;
            self.drift -= inward * self.drift.dot(inward).max(0.0);
        }
        let at = course(&self.ledge, self.time) + self.off;
        (at, (self.time >= SPAN).then(|| inward * EXIT))
    }
}

/// One step of a spring with no bounce that takes `off` to nothing: what is left
/// of it and of its rate after `dt` seconds.
pub fn settle(off: Vec3, drift: Vec3, rate: f32, dt: f32) -> (Vec3, Vec3) {
    let decay = math::exp(-rate * dt);
    let pull = drift + off * rate;
    let (off, drift) = ((off + pull * dt) * decay, (drift - pull * (rate * dt)) * decay);
    if off.length() < 1e-4 && drift.length() < 1e-3 {
        (Vec3::ZERO, Vec3::ZERO)
    } else {
        (off, drift)
    }
}
