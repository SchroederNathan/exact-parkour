//! An over-the-shoulder camera for a phone held in two thumbs.
//!
//! The camera is a boom behind a point beside the runner's chest. The view axis is
//! parallel to the boom, so the screen centre looks past the right shoulder and never
//! at the body. Five numbers place the eye (`Framing`). Two authored framings, one for
//! portrait and one for landscape, are blended by the canvas aspect.
//!
//! The view yaw is never stored. It is `Heading::frame + Heading::swing`, and
//! movement reads `frame` alone.
//!
//! Call `input` before `body::step` and `present` last in the tick.
use exact_game::*;

use crate::body::{self, Motion};

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
    /// Near plane, m.
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
    /// Length of the boom, m.
    pub boom: f32,
    /// Offset of the anchor to the right, m.
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

/// Read the finger. Call before `body::step`.
///
/// A contact on the scene turns the view and the movement frame together. Control
/// contacts (the move pad, the buttons) never appear here: the engine keeps them
/// out of `Input::pointer`.
pub fn input(w: &mut World, input: &Input) {
    let drag = scene_drag(input);
    let viewport = input.viewport();
    let dt = w.dt();
    let mut rig = w.resource_mut::<Rig>();
    if rig.viewport != viewport {
        // The host's first report is not a rotation of the phone: place, do not ease.
        rig.cut |= rig.viewport == Vec2::ZERO;
        rig.viewport = viewport;
    }
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
pub fn movement(w: &World, stick: Vec3) -> Vec3 {
    Quat::from_rotation_y(w.resource::<Rig>().heading.frame) * stick
}

/// Follow the body and write the camera's `Transform` and field of view. Call last
/// in the tick, after `body::present`.
pub fn present(w: &mut World, _motion: Motion) {
    let dt = w.dt();
    let centre = w.require::<Transform>("player").position;
    let shot = w.resource_mut::<Rig>().shoot(centre, dt);
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
    /// The eased framing, not its target.
    framing: Framing,
    /// The body's centre last tick, m.
    centre: Vec3,
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
    fn shoot(&mut self, centre: Vec3, dt: f32) -> Shot {
        let cut = std::mem::take(&mut self.cut) || centre.distance(self.centre) > CUT;
        self.centre = centre;
        if let Some(offset) = self.inspect {
            let basis = Basis::new(
                math::atan2(offset.x, offset.z),
                math::atan2(offset.y, offset.xz().length()),
            );
            return Shot {
                pose: Transform {
                    position: centre + offset,
                    rotation: basis.rotation(),
                    ..Transform::default()
                },
                fov: self.framing.fov,
                cut,
            };
        }
        let target = Framing::framed(self.viewport);
        if cut {
            self.framing = target;
        }
        self.framing.ease(target, dt);
        let basis = Basis::new(self.heading.view(), self.elevation());
        let anchor = centre
            + Vec3::Y * (self.framing.height - body::HALF)
            + basis.right() * self.framing.shoulder;
        Shot {
            pose: Transform {
                position: anchor + basis.back() * self.framing.boom,
                rotation: basis.rotation(),
                ..Transform::default()
            },
            fov: self.framing.fov,
            cut,
        }
    }
}

/// The player's hand on the view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
struct Look {
    /// Elevation the player added to the framing's, rad.
    tilt: f32,
    /// Turn the finger asked for that has not reached the view yet: (yaw, tilt), rad.
    pending: Vec2,
    /// Points this contact has travelled; zero with no contact.
    travel: f32,
}

impl Look {
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
                }
            }
            None => self.travel = 0.0,
        }
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
/// away from it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
struct Heading {
    /// Yaw of the movement frame, rad.
    frame: f32,
    /// Yaw the follow has added, rad.
    swing: f32,
    /// Unit stick direction the frame was last read under; zero with the stick at rest.
    anchor: Vec3,
}

impl Heading {
    fn view(&self) -> f32 {
        math::wrap_angle(self.frame + self.swing)
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
    fn ease(&mut self, target: Framing, dt: f32) {
        self.height = math::ease(self.height, target.height, HEIGHT_LAG, dt);
        self.boom = math::ease(self.boom, target.boom, BOOM_LAG, dt);
        self.shoulder = math::ease(self.shoulder, target.shoulder, SHOULDER_LAG, dt);
        self.elevation = math::ease(self.elevation, target.elevation, ELEVATION_LAG, dt);
        self.fov = math::ease(self.fov, target.fov, FOV_LAG, dt);
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
