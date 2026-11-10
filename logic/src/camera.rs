//! An over-the-shoulder camera for a phone held in two thumbs.
//!
//! The camera is a boom behind a point beside the runner's chest. The view axis is
//! parallel to the boom, so the screen centre looks past the right shoulder and never
//! at the body. Five numbers place the eye (`Framing`). Two authored framings, one for
//! portrait and one for landscape, are blended by the canvas aspect.
//!
//! Two actors turn the view, and each one owns its own number. The player's finger
//! writes `Heading::frame`. The automatic follow writes `Heading::swing`. The view
//! yaw is never stored: it is `frame + swing`. Movement reads `frame` alone, so the
//! follow cannot bend a held stick.
//!
//! Shake is rotation only and is applied after collision, so it cannot move the
//! lens into a wall.
//!
//! Call `input` before `body::step` and `present` last in the tick.
use exact_game::*;
use exact_game_physics::{self as physics, Shape};

use crate::body::{self, Act, Action, Motion, Parkour};
use crate::gait::{RUN, SPRINT};
use crate::park;

/// Every tuning number of the camera, with its unit. Tests read these, so a retune
/// does not break a test that states a relation.
pub mod tune {
    use super::Framing;

    // `height` m above the feet, `boom` m, `shoulder` m to the right, `elevation` rad
    // (positive: eye above the anchor, view looks down), `fov` vertical degrees.
    // Both put the eye 1.62 m above the feet: height + boom * sin(elevation).
    /// 932 x 430 points. Horizontal field of view 93 degrees.
    pub const LANDSCAPE: Framing = Framing {
        height: 1.25,
        boom: 2.90,
        shoulder: 0.60,
        elevation: 0.128,
        fov: 52.0,
    };
    /// 430 x 932 points. Horizontal field of view 37 degrees.
    pub const PORTRAIT: Framing = Framing {
        height: 1.44,
        boom: 3.40,
        shoulder: 0.42,
        elevation: 0.053,
        fov: 72.0,
    };
    /// Canvas width / height at and below which the framing is `PORTRAIT`, and at and
    /// above which it is `LANDSCAPE`. Between them the two blend (a tablet, a fold).
    pub const ASPECT_PORTRAIT: f32 = 0.75;
    pub const ASPECT_LANDSCAPE: f32 = 1.50;
    /// Near plane, m. The lens sphere (`LENS`) contains the near plane's corners:
    /// the furthest is 0.09 m from the eye (landscape, 61 degrees, aspect 2.17).
    pub const NEAR: f32 = 0.05;

    /// Time constants, s, with which each framing number reaches its target.
    pub const HEIGHT_LAG: f32 = 0.14;
    pub const BOOM_LAG: f32 = 0.30;
    pub const SHOULDER_LAG: f32 = 0.25;
    pub const ELEVATION_LAG: f32 = 0.28;
    pub const FOV_LAG: f32 = 0.35;

    /// Radians of yaw per point of horizontal drag: 0.34 degrees, a quarter turn in 262 points.
    pub const YAW_PER_POINT: f32 = 0.0060;
    /// Radians of tilt per point of vertical drag: 0.22 degrees. Lower than yaw.
    pub const TILT_PER_POINT: f32 = 0.0038;
    /// Points a contact must travel before it turns the view. A tap does not take the camera.
    pub const TAP_SLOP: f32 = 6.0;
    /// Time constant, s, with which a drag reaches the view. It spreads a touch sample
    /// over the simulation ticks between two samples. The total turn is conserved.
    pub const TOUCH_LAG: f32 = 0.022;
    /// Limits of the eye's elevation, rad: 26 degrees up from below, 60 degrees down from above.
    pub const ELEVATION_MIN: f32 = -0.45;
    pub const ELEVATION_MAX: f32 = 1.05;
    /// Seconds after the finger lifts in which the automatic follow stays off.
    pub const LOOK_HOLD: f32 = 1.0;
    /// Seconds in which the follow's authority returns from 0 to 1 after `LOOK_HOLD`.
    pub const HANDBACK: f32 = 0.6;
    /// Time constant, s, with which the player's tilt returns to the framing's while moving.
    pub const TILT_RETURN: f32 = 0.9;

    /// A stick direction inside this angle of the one the frame was read under, rad
    /// (15 degrees), steers by its own change and keeps the frame. Outside it the
    /// stick is read against the screen again.
    pub const RELATCH_CONE: f32 = 0.26;

    /// Planar speed, m/s, under which the view does not turn behind the travel.
    pub const FOLLOW_SPEED: f32 = 0.8;
    /// Time constant, s, of the turn at a run and at a sprint.
    pub const FOLLOW_TIME_RUN: f32 = 0.45;
    pub const FOLLOW_TIME_SPRINT: f32 = 0.32;
    /// Fastest automatic turn, rad/s (160 degrees/s), and its acceleration, rad/s².
    pub const FOLLOW_RATE_MAX: f32 = 2.8;
    pub const FOLLOW_ACCEL: f32 = 10.0;
    /// An error larger than this, rad, is a reversal: the view always swings the
    /// `REVERSAL_WAY` (-1: clockwise from above).
    pub const REVERSAL: f32 = 2.8;
    pub const REVERSAL_WAY: f32 = -1.0;
    // The rate can always fall as fast as the proportional target falls, so the
    // turn never passes its target.
    const _: () = assert!(FOLLOW_ACCEL > FOLLOW_RATE_MAX / FOLLOW_TIME_SPRINT);
    /// A hang turns the view this far along the wall, rad, at full shimmy speed or
    /// a full lean.
    pub const HANG_PEEK: f32 = 0.30;

    /// Time constant, s, of the focus in the ground plane, and the furthest it trails, m.
    pub const FOCUS_LAG: f32 = 0.08;
    pub const FOCUS_LEASH: f32 = 0.55;
    /// Angular frequency, rad/s, of the critically damped vertical spring, by stance.
    /// In the air it is `LIFT_JUMP` while the body is above the focus and
    /// `LIFT_FALL` while it is below.
    pub const LIFT_FEET: f32 = 16.0;
    pub const LIFT_JUMP: f32 = 1.0;
    pub const LIFT_FALL: f32 = 12.0;
    pub const LIFT_HANG: f32 = 10.0;
    pub const LIFT_LIMP: f32 = 6.0;
    /// The body may rise this far above the focus, and drop this far below it, m.
    pub const LEASH_UP: f32 = 0.45;
    pub const LEASH_DOWN: f32 = 0.15;
    /// A touchdown pushes the focus down: m/s of focus speed per m/s of impact, and
    /// the most, m/s. It adds to the fall speed the focus brings in on its leash.
    pub const DIP: f32 = 0.30;
    pub const DIP_MAX: f32 = 3.2;
    /// Time constant, s, of the camera's estimate of the body's velocity.
    pub const VELOCITY_LAG: f32 = 0.10;

    /// Degrees of field of view added at `SPRINT` speed; nothing at `RUN` and below.
    pub const SPRINT_FOV: f32 = 6.0;
    /// In the air: metres of boom added, and the downward tilt, rad, that a fall of
    /// `FALL_FAST` m/s adds; nothing under `FALL_SLOW`.
    pub const AIR_BOOM: f32 = 0.15;
    pub const FALL_TILT: f32 = 0.22;
    pub const FALL_SLOW: f32 = 3.0;
    pub const FALL_FAST: f32 = 9.0;
    /// A slide lowers the anchor, m, and widens the view, degrees.
    pub const SLIDE_DROP: f32 = 0.45;
    pub const SLIDE_FOV: f32 = 3.0;
    /// A hang: anchor up, m; boom out, m; tilt down, rad; share of the shoulder kept.
    pub const HANG_RISE: f32 = 0.10;
    pub const HANG_BOOM: f32 = 0.50;
    pub const HANG_TILT: f32 = 0.28;
    pub const HANG_SHOULDER: f32 = 0.6;
    /// A limp body (ragdoll, and the first half of the rise): boom out, m; tilt down,
    /// rad; share of the shoulder kept (none: the tumble is centred).
    pub const LIMP_BOOM: f32 = 0.90;
    pub const LIMP_TILT: f32 = 0.20;
    pub const LIMP_SHOULDER: f32 = 0.0;
    /// As a blocked boom shortens from `CROWD_FAR` to `CROWD_NEAR`, m, the anchor
    /// rises by up to `CROWD_LIFT`, m, so the eye goes over the head and not into it.
    pub const CROWD_FAR: f32 = 1.2;
    pub const CROWD_NEAR: f32 = 0.5;
    pub const CROWD_LIFT: f32 = 0.35;

    /// Scales all three layers of the shake. One number to calm the camera.
    pub const SHAKE: f32 = 1.0;
    /// Handheld drift at a stand, rad per axis (0.13 degrees), its two frequencies,
    /// Hz, and how many times larger it is at `SPRINT`.
    pub const HANDHELD: f32 = 0.0022;
    pub const HANDHELD_HZ: [f32; 2] = [0.23, 0.37];
    pub const HANDHELD_RUN: f32 = 2.0;
    /// Footfall, locked to the gait cycle: pitch nod per step and roll per stride,
    /// rad at `SPRINT` (0.29 and 0.34 degrees), and how late in the cycle the nod
    /// is deepest after a foot strikes.
    pub const STEP_PITCH: f32 = 0.0050;
    pub const STEP_ROLL: f32 = 0.0060;
    pub const STEP_LATE: f32 = 0.06;
    /// Seconds for the footfall layer to fade in and out.
    pub const STEP_FADE: f32 = 0.12;
    /// Trauma: amplitudes at trauma 1, rad (1.5, 0.9, 1.25 degrees), the rattle's
    /// frequency, Hz, and how much trauma is lost per second. Amplitude is trauma².
    pub const TRAUMA_PITCH: f32 = 0.026;
    pub const TRAUMA_YAW: f32 = 0.016;
    pub const TRAUMA_ROLL: f32 = 0.022;
    pub const RATTLE_HZ: f32 = 13.0;
    pub const TRAUMA_DECAY: f32 = 1.8;
    /// A touchdown slower than `IMPACT_QUIET` m/s adds no trauma; one of
    /// `IMPACT_FULL` m/s (`body::HARD`) adds 1. A knock-down adds `KNOCK_TRAUMA`.
    pub const IMPACT_QUIET: f32 = 2.5;
    pub const IMPACT_FULL: f32 = 9.0;
    pub const KNOCK_TRAUMA: f32 = 0.8;

    /// Radius of the sphere swept for the lens, m, and the gap it keeps, m.
    pub const LENS: f32 = 0.20;
    pub const SKIN: f32 = 0.04;

    /// A body that moves further than this in one tick, m, has been put somewhere: a cut.
    pub const CUT: f32 = 3.0;
}
use tune::*;

/// What the camera shows this tick. Tests and tools read it. The game does not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// Yaw of the view about Y, rad. Zero looks along -Z.
    pub yaw: f32,
    /// Elevation of the eye above the anchor, rad.
    pub elevation: f32,
    /// Length of the boom after collision, m.
    pub boom: f32,
    /// Offset of the anchor to the right after collision, m.
    pub shoulder: f32,
    /// Vertical field of view, degrees.
    pub fov: f32,
}

/// Register the camera's saved state. Call from `Game::register`.
pub fn register(w: &mut World) {
    w.register_resource::<Rig>();
}

/// Spawn the camera behind the player's right shoulder. The player must exist.
pub fn setup(w: &mut World) {
    w.insert_resource(Rig {
        follow: Follow {
            authority: 1.0,
            ..Follow::default()
        },
        cut: true,
        ..Rig::default()
    });
    w.spawn_named(
        "camera",
        (
            Transform::default(),
            Camera {
                fov_y_degrees: LANDSCAPE.fov,
                near: NEAR,
                ..Camera::default()
            },
        ),
    );
    present(w, Motion::default());
}

/// Read the finger and the stick. Call before `body::step`.
///
/// A contact on the scene turns the view and the movement frame together. Control
/// contacts (the move pad, the buttons) never appear here: the engine keeps them
/// out of `Input::pointer`.
pub fn input(w: &mut World, input: &Input) {
    let drag = scene_drag(input);
    let stick = input.stick_xz("move");
    let direction = if stick.length_squared() > 1e-4 {
        stick.normalize()
    } else {
        Vec3::ZERO
    };
    let viewport = input.viewport();
    let dt = w.dt();
    let mut rig = w.resource_mut::<Rig>();
    if rig.viewport != viewport {
        // The host's first report is not a rotation of the phone: place, do not ease.
        rig.cut |= rig.viewport == Vec2::ZERO;
        rig.viewport = viewport;
    }
    rig.heading.latch(direction);
    let turn = rig.look.drag(drag, dt);
    if rig.inspect.is_some() && rig.look.travel > TAP_SLOP {
        rig.inspect = None;
        rig.cut = true;
    }
    rig.heading.frame = math::wrap_angle(rig.heading.frame + turn.x);
    let base = Framing::framed(viewport).elevation;
    rig.look
        .tilt_by(turn.y, ELEVATION_MIN - base, ELEVATION_MAX - base);
}

/// The world direction of a stick vector. `body::step` calls it.
///
/// It uses the movement frame, which the automatic follow never turns.
pub fn movement(w: &World, stick: Vec3) -> Vec3 {
    Quat::from_rotation_y(w.resource::<Rig>().heading.frame) * stick
}

/// Follow the body and write the camera's `Transform` and field of view. Call last
/// in the tick, after `body::present`.
pub fn present(w: &mut World, motion: Motion) {
    let dt = w.dt();
    let subject = Subject::read(w, motion);
    let shot = w.resource_mut::<Rig>().shoot(w, &subject, dt);
    let camera = w.named("camera").expect("camera::setup spawns the camera");
    if shot.cut {
        // A cut must not be interpolated by the renderer.
        w.teleport(camera, shot.pose);
    } else {
        *w.require_mut::<Transform>(camera) = shot.pose;
    }
    let mut lens = w.require_mut::<Camera>(camera);
    if lens.fov_y_degrees != shot.fov {
        lens.fov_y_degrees = shot.fov;
    }
}

/// The `camera <x> <y> <z>` debug command: hold the eye that far from the body's
/// centre, looking at it. `Vec3::ZERO`, or a drag on the scene, returns to the rig.
/// Movement stays relative to this view.
pub fn set_offset(w: &mut World, offset: Vec3) {
    let mut rig = w.resource_mut::<Rig>();
    rig.cut = true;
    if offset == Vec3::ZERO {
        rig.inspect = None;
    } else {
        rig.inspect = Some(offset);
        rig.heading = Heading {
            frame: math::atan2(offset.x, offset.z),
            ..Heading::default()
        };
    }
}

/// What the camera shows this tick.
pub fn view(w: &World) -> View {
    let rig = w.resource::<Rig>();
    View {
        yaw: rig.heading.view(),
        elevation: rig.elevation(),
        boom: rig.framing.boom,
        shoulder: rig.framing.shoulder,
        fov: rig.framing.fov,
    }
}

/// All of the camera's state. Saved and hashed: a restore resumes the same camera.
#[derive(Clone, Default, Resource)]
struct Rig {
    look: Look,
    heading: Heading,
    follow: Follow,
    track: Track,
    /// The eased framing, after collision. Not the target.
    framing: Framing,
    focus: Focus,
    shake: Shake,
    /// Canvas size in points as `input` last saw it; zero before the host reports one.
    viewport: Vec2,
    /// The next `present` places the rig instead of easing it.
    cut: bool,
    /// The debug command's fixed offset, when it is on.
    inspect: Option<Vec3>,
}

/// What `present` writes to the camera entity.
struct Shot {
    pose: Transform,
    fov: f32,
    cut: bool,
}

impl Rig {
    fn elevation(&self) -> f32 {
        (self.framing.elevation + self.look.tilt).clamp(ELEVATION_MIN, ELEVATION_MAX)
    }
    /// Snap everything that lags to where it is going. Keeps the heading and the tilt.
    fn place(&mut self, subject: &Subject) {
        self.framing = Framing::framed(self.viewport).staged(subject.stance, 0.0, f32::MAX);
        self.focus = Focus {
            at: subject.centre,
            lift: 0.0,
        };
        self.track = Track {
            centre: subject.centre,
            velocity: Vec3::ZERO,
            limp: subject.stance == Stance::Limp,
        };
        self.follow.rate = 0.0;
        self.shake.trauma = 0.0;
    }
    fn shoot(&mut self, w: &World, subject: &Subject, dt: f32) -> Shot {
        let cut = std::mem::take(&mut self.cut) || subject.centre.distance(self.track.centre) > CUT;
        if cut {
            self.place(subject);
        }
        if self.track.step(subject, dt) {
            self.shake.hit(KNOCK_TRAUMA);
        }
        if let Some(offset) = self.inspect {
            let basis = Basis::new(
                math::atan2(offset.x, offset.z),
                math::atan2(offset.y, offset.xz().length()),
            );
            return Shot {
                pose: Transform {
                    position: subject.centre + offset,
                    rotation: basis.rotation(),
                    ..Transform::default()
                },
                fov: self.framing.fov,
                cut,
            };
        }
        let pace = self.track.velocity.xz().length();
        let want = subject
            .stance
            .behind(self.track.velocity, self.heading.pushing());
        self.follow
            .step(&mut self.heading, want, self.look.owns(), pace, dt);
        self.look.settle(want.is_some(), self.follow.authority, dt);
        let target = Framing::framed(self.viewport).staged(subject.stance, pace, self.framing.boom);
        self.framing.ease(target, dt);
        if subject.impact > 0.0 {
            self.focus.lift -= (subject.impact * DIP).min(DIP_MAX);
            self.shake.hit(trauma_of(subject.impact));
        }
        self.focus.step(subject, dt);
        let basis = Basis::new(self.heading.view(), self.elevation());
        Shot {
            pose: Transform {
                position: clear(w, subject, &self.focus, &mut self.framing, &basis),
                rotation: basis.rotation() * self.shake.step(pace, subject.stride, dt),
                ..Transform::default()
            },
            fov: self.framing.fov,
            cut,
        }
    }
}

/// The player's hand on the view. Only `input` writes it, except that `settle`
/// lets the tilt go home.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
struct Look {
    /// Elevation the player added to the framing's, rad.
    tilt: f32,
    /// Turn the finger asked for that has not reached the view yet: (yaw, tilt), rad.
    pending: Vec2,
    /// Points this contact has travelled; zero with no contact.
    travel: f32,
    /// Seconds the view still belongs to the player. `LOOK_HOLD` while a finger turns it.
    hold: f32,
}

impl Look {
    /// Whether the player owns the view: a finger turns it, or it lifted less than
    /// `LOOK_HOLD` ago.
    fn owns(&self) -> bool {
        self.hold > 0.0
    }
    /// Take this tick's drag on the scene, in points. Returns the turn to apply
    /// now: (yaw, tilt), rad.
    fn drag(&mut self, drag: Option<Vec2>, dt: f32) -> Vec2 {
        match drag {
            Some(delta) => {
                let moved = delta.length();
                self.travel += moved;
                if self.travel > TAP_SLOP {
                    // Only the motion past the slop turns the view. The points
                    // under it are dropped, so passing the slop is not a jump.
                    let live = ((self.travel - TAP_SLOP) / moved).min(1.0);
                    // Swipe right looks right; swipe down looks down.
                    self.pending +=
                        Vec2::new(-delta.x * YAW_PER_POINT, delta.y * TILT_PER_POINT) * live;
                    self.hold = LOOK_HOLD;
                }
            }
            None => {
                self.travel = 0.0;
                self.hold = (self.hold - dt).max(0.0);
            }
        }
        // The last of the tail goes out whole, so the view comes to rest.
        let turn = if self.pending.length_squared() < 1e-10 {
            self.pending
        } else {
            self.pending * (1.0 - math::exp(-dt / TOUCH_LAG))
        };
        self.pending -= turn;
        turn
    }
    /// Add `by` to the tilt inside `[low, high]`. At a limit the pending tilt is
    /// dropped, so a drag back leaves the limit at once.
    fn tilt_by(&mut self, by: f32, low: f32, high: f32) {
        let tilt = self.tilt + by;
        self.tilt = tilt.clamp(low, high);
        if self.tilt != tilt {
            self.pending.y = 0.0;
        }
    }
    /// Let the tilt return to the framing's while the follow has a target, as far
    /// as the follow has authority.
    fn settle(&mut self, following: bool, authority: f32, dt: f32) {
        if following {
            let home = math::ease(self.tilt, 0.0, TILT_RETURN, dt);
            self.tilt = math::lerp(self.tilt, home, authority);
        }
    }
}

/// This tick's motion, in points, of a contact on the scene: a finger, or a pressed
/// mouse. A flick that goes down and up inside one tick counts. A hover and a
/// cancelled contact do not.
fn scene_drag(input: &Input) -> Option<Vec2> {
    input
        .pointer()
        .filter(|p| {
            p.press_origin.is_some()
                && (p.down || input.pressed("orbit") || input.released("orbit"))
        })
        .map(|p| p.delta)
}

/// Where "stick up" points in the world, and how far the follow has turned the view
/// away from it. The view yaw is never stored: it is `frame + swing`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
struct Heading {
    /// Yaw of the movement frame, rad. `input` writes it: a drag, or a latch.
    frame: f32,
    /// Yaw the follow has added, rad. Only `Follow::step` and `latch` write it.
    swing: f32,
    /// Unit stick direction the frame was last read under; zero with the stick at rest.
    anchor: Vec3,
}

impl Heading {
    fn view(&self) -> f32 {
        math::wrap_angle(self.frame + self.swing)
    }
    fn pushing(&self) -> bool {
        self.anchor != Vec3::ZERO
    }
    /// Read the stick against the screen again when it rests, starts a push, or
    /// leaves `RELATCH_CONE` of the direction it was read under. Otherwise keep the
    /// frame: a held direction keeps a straight path while the view swings.
    fn latch(&mut self, direction: Vec3) {
        let outside = direction.dot(self.anchor) < math::cos(RELATCH_CONE);
        if direction == Vec3::ZERO || self.anchor == Vec3::ZERO || outside {
            self.frame = self.view();
            self.swing = 0.0;
            self.anchor = direction;
        }
    }
}

/// The automatic turn behind the travel. It writes `Heading::swing` and nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
struct Follow {
    /// Present turn rate, rad/s. It changes by at most `FOLLOW_ACCEL` per second,
    /// so the turn starts and stops gently.
    rate: f32,
    /// How much of the follow is in force, 0 to 1: 0 while the player owns the view,
    /// then up over `HANDBACK`.
    authority: f32,
}

impl Follow {
    /// Turn the view toward `want` (a view yaw, rad), or coast to rest without one.
    fn step(&mut self, heading: &mut Heading, want: Option<f32>, owned: bool, pace: f32, dt: f32) {
        if owned {
            self.rate = 0.0;
            self.authority = 0.0;
            return;
        }
        self.authority = (self.authority + dt / HANDBACK).min(1.0);
        let error = want.map(|yaw| {
            let error = math::wrap_angle(yaw - heading.view());
            // Near half a turn the shorter way flips with noise: always go one way.
            if error.abs() > REVERSAL && error * REVERSAL_WAY < 0.0 {
                error + std::f32::consts::TAU * REVERSAL_WAY
            } else {
                error
            }
        });
        let time = math::lerp(
            FOLLOW_TIME_RUN,
            FOLLOW_TIME_SPRINT,
            math::smoothstep(RUN, SPRINT, pace),
        );
        let wish = error.map_or(0.0, |e| (e / time).clamp(-FOLLOW_RATE_MAX, FOLLOW_RATE_MAX))
            * self.authority;
        self.rate += (wish - self.rate).clamp(-FOLLOW_ACCEL * dt, FOLLOW_ACCEL * dt);
        let mut step = self.rate * dt;
        if let Some(error) = error {
            if step * error > 0.0 && step.abs() > error.abs() {
                step = error;
                self.rate = 0.0;
            }
        }
        heading.swing = math::wrap_angle(heading.swing + step);
    }
}

/// What the body is doing, as far as the camera frames it differently. The camera's
/// only knowledge of `body::Action`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stance {
    /// On the ground or on a scripted course: run, vault, climb, roll, land, late rise.
    Feet,
    /// In the air. `fall` is the downward speed, m/s, zero while rising.
    Air {
        fall: f32,
    },
    Slide,
    /// Hanging from an edge. `facing` is the yaw of the way into the wall, rad
    /// (zero faces +Z, as `Parkour::yaw`). `along` is the shimmy speed over
    /// `body::SHIMMY` plus the lean past the edge's end, -1 to 1, left positive.
    Hang {
        facing: f32,
        along: f32,
    },
    /// The ragdoll has the body, or it has only begun to rise.
    Limp,
}

impl Stance {
    fn of(parkour: &Parkour, motion: &Motion) -> Stance {
        match &parkour.action {
            Action::Ground
            | Action::Vault(_)
            | Action::Climb(_)
            | Action::Roll { .. }
            | Action::Land { .. } => Stance::Feet,
            Action::Air => Stance::Air {
                fall: (-motion.rising).max(0.0),
            },
            Action::Slide { .. } => Stance::Slide,
            Action::Hang(hang) => {
                let inward = hang.ledge.inward();
                Stance::Hang {
                    facing: math::atan2(inward.x, inward.z),
                    along: (hang.speed / body::SHIMMY + hang.lean).clamp(-1.0, 1.0),
                }
            }
            Action::Ragdoll => Stance::Limp,
            Action::Rise { .. } if motion.progress < 0.5 => Stance::Limp,
            Action::Rise { .. } => Stance::Feet,
        }
    }
    /// The view yaw the follow wants, if this stance gives it one.
    fn behind(self, velocity: Vec3, pushing: bool) -> Option<f32> {
        let travel = || {
            (velocity.xz().length() > FOLLOW_SPEED).then(|| math::atan2(-velocity.x, -velocity.z))
        };
        match self {
            Stance::Hang { facing, along } => {
                Some(facing + std::f32::consts::PI + HANG_PEEK * along)
            }
            // Do not chase a tumble.
            Stance::Limp => None,
            Stance::Slide => travel(),
            Stance::Feet | Stance::Air { .. } => pushing.then(travel).flatten(),
        }
    }
    /// Angular frequency of the vertical spring, rad/s, for a body `above` m
    /// above the focus.
    fn lift(self, above: f32) -> f32 {
        match self {
            Stance::Feet | Stance::Slide => LIFT_FEET,
            // A body above the focus is in a jump. The view must not follow it
            // up, also not on its way down.
            Stance::Air { .. } if above > 0.0 => LIFT_JUMP,
            Stance::Air { .. } => LIFT_FALL,
            Stance::Hang { .. } => LIFT_HANG,
            Stance::Limp => LIFT_LIMP,
        }
    }
}

/// What the camera reads of the world at the top of `present`. Never stored.
#[derive(Clone, Copy, Debug)]
struct Subject {
    /// The capsule's centre, m. In a ragdoll this is the hips.
    centre: Vec3,
    stance: Stance,
    /// Vertical velocity, m/s.
    rising: f32,
    /// Downward speed of a touchdown this tick, m/s; zero otherwise.
    impact: f32,
    /// Phase of the gait cycle, 0 to 1, while the feet run on the ground. The left
    /// foot strikes at 0 and the right at 0.5 (`gait::planted`).
    stride: Option<f32>,
}

impl Subject {
    fn read(w: &World, motion: Motion) -> Subject {
        Subject {
            centre: w.require::<Transform>("player").position,
            stance: Stance::of(&w.require::<Parkour>("player"), &motion),
            rising: motion.rising,
            impact: motion.impact,
            stride: w
                .get::<Pose>("stickman")
                .filter(|_| motion.act == Act::Move && motion.grounded)
                .map(|pose| pose.phase),
        }
    }
}

/// What the camera remembers of the body between ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
struct Track {
    /// The capsule's centre last tick, m.
    centre: Vec3,
    /// Smoothed velocity from the centre's own motion, m/s. It is right on scripted
    /// courses and in a ragdoll, where the controller's velocity is not.
    velocity: Vec3,
    /// Whether the stance was `Limp` last tick.
    limp: bool,
}

impl Track {
    /// Take this tick's body. Returns whether it went limp this tick: a knock-down.
    fn step(&mut self, subject: &Subject, dt: f32) -> bool {
        let moved = subject.centre - self.centre;
        self.velocity = math::ease(self.velocity, moved / dt, VELOCITY_LAG, dt);
        self.centre = subject.centre;
        let was = std::mem::replace(&mut self.limp, subject.stance == Stance::Limp);
        self.limp && !was
    }
}

/// The five numbers that place the eye. The two authored framings, the target of a
/// tick and the eased state are all this one type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
pub struct Framing {
    /// Height of the anchor above the feet of a standing body, m. The anchor is
    /// `height - body::HALF` above the capsule's centre in every stance.
    pub height: f32,
    /// Distance from the anchor back to the eye, m.
    pub boom: f32,
    /// Distance of the anchor to the right of the body's axis, m.
    pub shoulder: f32,
    /// Elevation of the eye above the anchor, rad, before the player's tilt.
    pub elevation: f32,
    /// Vertical field of view, degrees.
    pub fov: f32,
}

impl Framing {
    /// The authored framing for a canvas of `viewport` points. A zero viewport
    /// (no host yet, a hostless test) is `LANDSCAPE`.
    fn framed(viewport: Vec2) -> Framing {
        if viewport.min_element() <= 0.0 {
            return LANDSCAPE;
        }
        let wide = math::smoothstep(ASPECT_PORTRAIT, ASPECT_LANDSCAPE, viewport.x / viewport.y);
        Framing {
            height: math::lerp(PORTRAIT.height, LANDSCAPE.height, wide),
            boom: math::lerp(PORTRAIT.boom, LANDSCAPE.boom, wide),
            shoulder: math::lerp(PORTRAIT.shoulder, LANDSCAPE.shoulder, wide),
            elevation: math::lerp(PORTRAIT.elevation, LANDSCAPE.elevation, wide),
            fov: math::lerp(PORTRAIT.fov, LANDSCAPE.fov, wide),
        }
    }
    /// This framing as `stance` changes it at `pace` m/s. `boom` is the present
    /// boom after collision, which lifts the anchor when it is short.
    fn staged(mut self, stance: Stance, pace: f32, boom: f32) -> Framing {
        self.fov += SPRINT_FOV * math::smoothstep(RUN, SPRINT, pace);
        match stance {
            Stance::Feet => {}
            Stance::Air { fall } => {
                self.boom += AIR_BOOM;
                self.elevation += FALL_TILT * math::smoothstep(FALL_SLOW, FALL_FAST, fall);
            }
            Stance::Slide => {
                self.height -= SLIDE_DROP;
                self.fov += SLIDE_FOV;
            }
            Stance::Hang { .. } => {
                self.height += HANG_RISE;
                self.boom += HANG_BOOM;
                self.elevation += HANG_TILT;
                self.shoulder *= HANG_SHOULDER;
            }
            Stance::Limp => {
                self.boom += LIMP_BOOM;
                self.elevation += LIMP_TILT;
                self.shoulder *= LIMP_SHOULDER;
            }
        }
        self.height += CROWD_LIFT * (1.0 - math::smoothstep(CROWD_NEAR, CROWD_FAR, boom));
        self
    }
    /// Move each number toward `target` with its own time constant.
    fn ease(&mut self, target: Framing, dt: f32) {
        self.height = math::ease(self.height, target.height, HEIGHT_LAG, dt);
        self.boom = math::ease(self.boom, target.boom, BOOM_LAG, dt);
        self.shoulder = math::ease(self.shoulder, target.shoulder, SHOULDER_LAG, dt);
        self.elevation = math::ease(self.elevation, target.elevation, ELEVATION_LAG, dt);
        self.fov = math::ease(self.fov, target.fov, FOV_LAG, dt);
    }
}

/// The lagged point the rig stands on: the capsule's centre, late.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
struct Focus {
    at: Vec3,
    /// Vertical speed of the focus, m/s: the state of its spring.
    lift: f32,
}

impl Focus {
    fn step(&mut self, subject: &Subject, dt: f32) {
        let centre = subject.centre;
        let behind = (self.at - centre) * Vec3::new(1.0, 0.0, 1.0);
        let behind = math::ease(behind, Vec3::ZERO, FOCUS_LAG, dt).clamp_length_max(FOCUS_LEASH);
        let omega = subject.stance.lift(centre.y - self.at.y);
        let (mut height, mut lift) = spring(self.at.y, self.lift, centre.y, omega, dt);
        if height < centre.y - LEASH_UP {
            height = centre.y - LEASH_UP;
        }
        // A falling body takes the focus down with it at its own speed, so the
        // landing arrives in the spring and not as a stop.
        if height > centre.y + LEASH_DOWN {
            height = centre.y + LEASH_DOWN;
            lift = lift.min(subject.rising);
        }
        self.at = Vec3::new(centre.x + behind.x, height, centre.z + behind.z);
        self.lift = lift;
    }
}

/// Camera shake as rotation: handheld drift, footfall, trauma.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
struct Shake {
    /// 0 to 1. Events add to it and it decays at `TRAUMA_DECAY` per second.
    trauma: f32,
    /// Phases of the two handheld waves, rad, wrapped at a full turn.
    drift: Vec2,
    /// Phase of the trauma rattle, rad, wrapped at a full turn.
    rattle: f32,
    /// Weight of the footfall layer, 0 to 1, faded over `STEP_FADE`.
    footfall: f32,
    /// The gait phase the footfall layer last saw. It fades out on this phase
    /// when the feet leave the ground.
    stride: f32,
}

impl Shake {
    fn hit(&mut self, trauma: f32) {
        self.trauma = (self.trauma + trauma).min(1.0);
    }
    /// Advance and return this tick's rotation, to apply after the view's own.
    fn step(&mut self, pace: f32, stride: Option<f32>, dt: f32) -> Quat {
        use std::f32::consts::TAU;
        self.drift.x = (self.drift.x + TAU * HANDHELD_HZ[0] * dt) % TAU;
        self.drift.y = (self.drift.y + TAU * HANDHELD_HZ[1] * dt) % TAU;
        self.rattle = (self.rattle + TAU * RATTLE_HZ * dt) % TAU;
        self.trauma = (self.trauma - TRAUMA_DECAY * dt).max(0.0);
        let fade = dt / STEP_FADE;
        self.footfall = match stride {
            Some(phase) => {
                self.stride = phase;
                (self.footfall + fade).min(1.0)
            }
            None => (self.footfall - fade).max(0.0),
        };
        let fast = (pace / SPRINT).min(1.0);
        let (a, b) = (self.drift.x, self.drift.y);

        let hand = HANDHELD * (1.0 + (HANDHELD_RUN - 1.0) * fast);
        let mut pitch = hand * (0.6 * math::sin(a) + 0.4 * math::sin(2.0 * b + 1.3));
        let mut yaw = hand * (0.6 * math::sin(b) + 0.4 * math::sin(2.0 * a + 2.1));
        let mut roll = hand * 0.6 * math::sin(a + b);

        // Two steps per cycle: a nod after each strike, and a roll onto the
        // planted side.
        let step = self.footfall * fast;
        pitch -= STEP_PITCH * step * math::cos(2.0 * TAU * (self.stride - STEP_LATE));
        roll += STEP_ROLL * step * math::sin(TAU * self.stride);

        // A wrapped phase takes whole multiples only, so the three axes use the
        // rattle's first three harmonics.
        let rattle = self.trauma * self.trauma;
        pitch += TRAUMA_PITCH * rattle * math::sin(self.rattle);
        roll += TRAUMA_ROLL * rattle * math::sin(2.0 * self.rattle + 0.5);
        yaw += TRAUMA_YAW * rattle * math::sin(3.0 * self.rattle + 1.0);

        Quat::from_rotation_y(yaw * SHAKE)
            * Quat::from_rotation_x(pitch * SHAKE)
            * Quat::from_rotation_z(roll * SHAKE)
    }
}

/// The view's axes for a yaw and an elevation.
#[derive(Clone, Copy, Debug)]
struct Basis {
    turn: Quat,
    tilt: Quat,
}

impl Basis {
    fn new(yaw: f32, elevation: f32) -> Basis {
        Basis {
            turn: Quat::from_rotation_y(yaw),
            tilt: Quat::from_rotation_x(-elevation),
        }
    }
    /// To the right of the view, level.
    fn right(&self) -> Vec3 {
        self.turn * Vec3::X
    }
    /// From the anchor back to the eye.
    fn back(&self) -> Vec3 {
        self.turn * (self.tilt * Vec3::Z)
    }
    /// The camera looks along `-back()`: parallel to the boom, never at the body.
    fn rotation(&self) -> Quat {
        self.turn * self.tilt
    }
}

/// One step of a critically damped spring toward `to`. Exact for any `dt`; a
/// spring at rest never passes a still target.
fn spring(at: f32, speed: f32, to: f32, omega: f32, dt: f32) -> (f32, f32) {
    let decay = math::exp(-omega * dt);
    let from = at - to;
    let travel = (speed + omega * from) * dt;
    (
        to + (from + travel) * decay,
        (speed - omega * travel) * decay,
    )
}

/// Trauma a touchdown of `impact` m/s adds.
fn trauma_of(impact: f32) -> f32 {
    ((impact - IMPACT_QUIET) / (IMPACT_FULL - IMPACT_QUIET)).clamp(0.0, 1.0)
}

/// Keep the lens out of solid geometry and return the eye. Shortens
/// `framing.shoulder` and `framing.boom` at once when blocked; `Framing::ease`
/// lengthens them again slowly.
///
/// A wall beside the runner ends the first leg early: the eye slides in behind the
/// body and keeps its distance. Only a wall behind the runner shortens the boom.
fn clear(
    w: &World,
    subject: &Subject,
    focus: &Focus,
    framing: &mut Framing,
    basis: &Basis,
) -> Vec3 {
    let lift = framing.height - body::HALF;
    // The sweeps start inside the capsule, which the body keeps clear of solids.
    let reach = body::HALF - body::RADIUS;
    let origin = subject.centre + Vec3::Y * lift.clamp(-reach, reach);
    // The lag of the focus is part of this leg, so a lagging focus cannot start
    // the boom behind a wall.
    let anchor = focus.at + Vec3::Y * lift + basis.right() * framing.shoulder;
    let share = free(w, origin, anchor - origin);
    framing.shoulder *= share;
    let anchor = origin + (anchor - origin) * share;
    framing.boom *= free(w, anchor, basis.back() * framing.boom);
    anchor + basis.back() * framing.boom
}

/// The share, 0 to 1, of `travel` that a lens sphere at `from` can move.
fn free(w: &World, from: Vec3, travel: Vec3) -> f32 {
    let length = travel.length();
    let lens = Shape::Sphere { radius: LENS };
    let pose = Transform::at(from.x, from.y, from.z);
    let reach = match physics::sweep(w, &lens, pose, travel, park::SOLID) {
        None => return 1.0,
        // A sweep that starts in contact reports zero (a ragdoll's hips against a
        // wall). A ray from the same point says how far the lens centre may go.
        Some(hit) if hit.distance < 1e-3 => {
            match physics::raycast(w, from, travel, length + LENS, park::SOLID) {
                None => return 1.0,
                Some(hit) => hit.distance - LENS,
            }
        }
        Some(hit) => hit.distance,
    };
    ((reach - SKIN) / length).clamp(0.0, 1.0)
}
