//! The player's body: an upright capsule that the physics controller moves through
//! the world, and what that body is doing.
//!
//! `Action` is the whole story of the body at one tick. Each variant owns the data
//! of its move, and `step` is the only place one becomes another.
use crate::gait::{RUN, SPRINT};
use crate::hold::{self, Hold, Limb};
use crate::ledge::{self, Ledge};
use crate::park::PROP;
use crate::ragdoll::{self, Ragdoll};
use exact_game::motion::{Gravity, Jump, Move};
use exact_game::*;
use exact_game_physics::{self as physics, CapsuleController, Shape};

/// The capsule: total height and radius in metres. Its centre is `HALF` above the feet.
pub const HEIGHT: f32 = 1.8;
pub const RADIUS: f32 = 0.3;
pub const HALF: f32 = HEIGHT / 2.0;
/// Real gravity, shared with every rigid body in the park, m/s².
pub const GRAVITY: f32 = 9.81;
/// Height of a standing jump, metres.
pub const JUMP: f32 = 0.75;
/// Ground acceleration, m/s²: top speed in about a tenth of a second, and a change
/// of direction as fast.
const ACCEL: f32 = 40.0;
/// Braking with the stick released, m/s²: a run stops within a stride, which is the
/// distance the legs need to close into a stand without sliding.
const BRAKE: f32 = 24.0;
/// Steering in the air, m/s²: enough to correct a jump, not to redo it.
const AIR_ACCEL: f32 = 9.0;
/// The body turns to face where it is going: by this share of the angle left per
/// second, and never faster than `TURN_MAX` radians per second, so an about-face is
/// a quick spin and not a flip.
const TURN: f32 = 18.0;
const TURN_MAX: f32 = 14.0;
/// A jump still starts this long after the feet left an edge, and a press this
/// long before they touch down is kept for the landing, seconds.
const COYOTE: f32 = 0.1;
const BUFFER: f32 = 0.12;
/// After letting go of a ledge the hands do not grab again for this long, seconds.
const REGRAB: f32 = 0.4;

/// The tallest wall the body vaults or climbs from the ground, and the deepest top
/// it vaults across, metres. A deeper or taller one it climbs onto.
pub const VAULT_HIGH: f32 = 1.15;
pub const VAULT_DEEP: f32 = 1.2;
pub const CLIMB_HIGH: f32 = 1.5;
/// The lowest wall worth a move; a lower one is a step.
const VAULT_LOW: f32 = 0.35;
/// The band above the feet in which airborne hands catch an edge, metres.
const GRAB_LOW: f32 = 1.45;
const GRAB_HIGH: f32 = 2.1;
/// Seconds the body takes to swing in under an edge its hands caught.
pub const SETTLE: f32 = 0.25;
/// How much closer to the wall than the capsule the drawn body hangs, metres: the
/// capsule is as thick as the body with its arms, and a hanging body is flat.
const HUG: f32 = 0.2;
/// Hanging, the stick counts as forward or back within 45 degrees of the wall's
/// normal; the rest of the circle is a shimmy.
const SECTOR: f32 = 0.7;
/// Shimmy speed along an edge, m/s, and how fast it starts and stops, m/s².
pub const SHIMMY: f32 = 0.8;
const SHIMMY_ACCEL: f32 = 6.0;
/// Half the distance between hanging hands, and how far one hand moves in a
/// shimmy step, metres.
const HANDS: f32 = 0.2;
const HAND_STEP: f32 = 0.3;

/// A vault carries the capsule's centre, which is at the hips, this far above the
/// wall's top, metres: an arm's length under the shoulder of a body that leans on
/// one hand. Over a wall too low for that it stays this far under a stand.
const VAULT_OVER: f32 = 0.35;
const VAULT_DIP: f32 = 0.15;
/// How far to the left of its course a vaulting body puts its hand down, metres.
const VAULT_HAND: f32 = 0.42;

/// Landings, by downward speed at touchdown in m/s. Under `SOFT` the knees take it.
/// From there to `HARD` the body must roll, or crouch to its hands if it has no
/// ground speed to roll with, and the recovery grows with the speed. At `HARD` and
/// above nothing saves it: the body falls.
pub const SOFT: f32 = 5.0;
pub const HARD: f32 = 9.0;
/// Ground speed a roll needs, m/s.
const ROLL_SPEED: f32 = 1.2;

/// A thing that hits the body with at least this much momentum, in kg·m/s, knocks
/// it down. A lighter hit only shoves it. A 30 kg ball at 7 m/s is 210.
pub const KNOCK: f32 = 200.0;
/// The body's mass for what a hit does to its speed, kg.
const MASS: f32 = 80.0;
/// Seconds to rise from the ground, and the first part of it, in which the limbs
/// gather from wherever the fall left them.
const RISE: f32 = 1.8;
pub const GATHER: f32 = 0.4;

/// A fixed course of the capsule's centre through three points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
pub struct Path {
    pub from: Vec3,
    /// The point the course passes at its middle.
    pub over: Vec3,
    pub to: Vec3,
    /// Seconds flown, of `span`.
    pub time: f32,
    pub span: f32,
    /// Velocity the body leaves the course with, m/s.
    pub exit: Vec3,
    /// Height of the wall above the feet at the start, metres.
    pub height: f32,
    /// The edge the move uses.
    pub ledge: Ledge,
    /// Where on the wall's top a vault's hand goes down, if `held`.
    pub hand: Vec3,
    pub held: bool,
    /// A climb out of a hang: how close to the wall the drawn body starts, and how
    /// far the hang had shimmied, which is where its hands are.
    pub hug: f32,
    pub travel: f32,
}

/// A body that hangs from an edge by its hands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
pub struct Hang {
    /// The edge, at the point the body hangs under.
    pub ledge: Ledge,
    /// Where the capsule's centre was when the hands caught the edge.
    pub caught: Vec3,
    /// Seconds since the catch.
    pub time: f32,
    /// Metres shimmied along the edge, and the speed of it; to the left is positive.
    pub travel: f32,
    pub speed: f32,
    /// Whether the stick has stopped pushing at the wall since the catch. Until
    /// then it still holds the push of the run-up, which is not a wish to climb.
    pub live: bool,
}
impl Hang {
    /// How far the body has swung in under the edge, 0 to 1.
    pub fn settled(&self) -> f32 {
        smooth(self.time / SETTLE)
    }
}
impl Path {
    pub fn progress(&self) -> f32 {
        (self.time / self.span).clamp(0.0, 1.0)
    }
    // A quadratic curve whose middle is `over`.
    fn at(&self, u: f32) -> Vec3 {
        let control = self.over * 2.0 - (self.from + self.to) * 0.5;
        self.from * ((1.0 - u) * (1.0 - u)) + control * (2.0 * u * (1.0 - u)) + self.to * (u * u)
    }
}

/// What the body is doing.
#[derive(Clone, Debug, Default, PartialEq, Data)]
pub enum Action {
    /// On its feet, under control.
    #[default]
    Ground,
    /// Jumping or falling, with a little steering.
    Air,
    /// Crossing a low wall, a hand on its top.
    Vault(Path),
    /// Hanging from an edge by the hands, still or in a shimmy.
    Hang(Hang),
    /// Pulling up onto a top, from a hang, from the ground or out of the air.
    Climb(Path),
    /// Rolling out of a hard landing along `heading`, from `speed` m/s.
    Roll {
        time: f32,
        span: f32,
        heading: Vec3,
        speed: f32,
    },
    /// Crouched to the hands after a hard landing with nowhere to roll.
    Land { time: f32, span: f32 },
    /// Down and limp: the ragdoll has the body.
    Ragdoll,
    /// Getting up from where the ragdoll came to rest. The capsule goes from
    /// `from` to the standing place `to`.
    Rise {
        time: f32,
        from: Vec3,
        to: Vec3,
        /// Whether the body lay on its back.
        supine: bool,
    },
}

/// The animation each action plays, as the Animator's `act` parameter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Act {
    #[default]
    Move,
    Air,
    Vault,
    Hang,
    Climb,
    Roll,
    Land,
    Rise,
}
impl Act {
    pub const ALL: [Act; 8] = [
        Act::Move,
        Act::Air,
        Act::Vault,
        Act::Hang,
        Act::Climb,
        Act::Roll,
        Act::Land,
        Act::Rise,
    ];
    /// The Animator state's name.
    pub fn state(self) -> &'static str {
        match self {
            Act::Move => "move",
            Act::Air => "air",
            Act::Vault => "vault",
            Act::Hang => "hang",
            Act::Climb => "climb",
            Act::Roll => "roll",
            Act::Land => "land",
            Act::Rise => "rise",
        }
    }
    pub fn code(self) -> f32 {
        self as u8 as f32
    }
}

/// What the animation needs to know about this tick's motion.
#[derive(Clone, Copy, Debug, Default)]
pub struct Motion {
    pub act: Act,
    /// How far through a move with a beginning and an end, 0 to 1.
    pub progress: f32,
    /// Ground speed, m/s.
    pub speed: f32,
    /// Vertical velocity, m/s.
    pub rising: f32,
    pub grounded: bool,
    /// Downward speed of a touchdown this tick, m/s; zero on every other tick.
    pub impact: f32,
    /// Height of the wall a vault or climb is crossing, metres.
    pub height: f32,
    /// Whether a rising body lay on its back.
    pub supine: bool,
}

/// The body's own state, beside the controller's.
#[derive(Clone, Debug, Default, Component)]
pub struct Parkour {
    pub action: Action,
    /// Heading about Y in radians; zero faces +Z. The visual turns toward it.
    pub yaw: f32,
    /// Seconds since the feet were last on the ground.
    pub airtime: f32,
    /// Fastest downward speed of this time in the air, m/s.
    pub fall: f32,
    /// Seconds a jump press is still remembered.
    pub buffer: f32,
    /// Seconds until the hands may grab again.
    pub regrab: f32,
}

/// Feet position of a capsule centre.
pub fn feet(centre: Vec3) -> Vec3 {
    centre - Vec3::Y * HALF
}

/// An angle brought into (-π, π].
pub fn wrap_angle(a: f32) -> f32 {
    let turns = a / std::f32::consts::TAU;
    (turns - math::floor(turns + 0.5)) * std::f32::consts::TAU
}

fn planar(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}
fn heading(yaw: f32) -> Vec3 {
    let (sin, cos) = math::sin_cos(yaw);
    Vec3::new(sin, 0.0, cos)
}
fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

struct Controls {
    wish: Vec3,
    jump: bool,
    sprint: bool,
}

/// Read the controls, advance the action and move the capsule. Call before
/// `physics::step`.
pub fn step(w: &mut World, input: &Input) -> Motion {
    let dt = w.dt();
    let controls = Controls {
        wish: planar(input.stick_xz("move")).clamp_length_max(1.0),
        jump: input.pressed("jump"),
        sprint: input.held("sprint"),
    };
    let mut body = w.require::<Parkour>("player").clone();
    body.buffer = if controls.jump {
        BUFFER
    } else {
        (body.buffer - dt).max(0.0)
    };
    body.regrab = (body.regrab - dt).max(0.0);

    // A hit comes before anything the body meant to do.
    let hit = if matches!(body.action, Action::Ragdoll) {
        None
    } else {
        struck(w)
    };
    if let Some(Hit::Knock(kick)) = hit {
        let velocity = w.require::<CapsuleController>("player").velocity;
        fall_down(w, &mut body, velocity, kick);
    } else if let (Some(Hit::Shove(push)), Action::Ground | Action::Air) = (hit, &body.action) {
        w.require_mut::<CapsuleController>("player").velocity += push;
    }

    let (action, mut motion, face) = match body.action.clone() {
        Action::Ground | Action::Air => free(w, &mut body, &controls),
        Action::Vault(path) => follow(w, path, Act::Vault, Action::Vault),
        Action::Climb(path) => follow(w, path, Act::Climb, Action::Climb),
        Action::Hang(hanging) => hang(w, &mut body, hanging, &controls),
        Action::Roll {
            time,
            span,
            heading,
            speed,
        } => roll(w, time + dt, span, heading, speed),
        Action::Land { time, span } => land(w, time + dt, span),
        Action::Ragdoll => limp(w, &mut body),
        Action::Rise {
            time,
            from,
            to,
            supine,
        } => rise(w, time + dt, from, to, supine),
    };
    body.action = action;
    // The drawn body is under the capsule, or closer to the wall it climbs. A rise
    // puts it where the body will stand.
    if !matches!(body.action, Action::Rise { .. }) {
        let place = Vec3::new(0.0, -HALF - 0.01, 0.0) + hug(&body.action);
        if w.require::<Transform>("stickman").position != place {
            w.require_mut::<Transform>("stickman").position = place;
        }
    }

    // Turn toward where the body is going, or where its move points it.
    let target = face.or_else(|| {
        let v = planar(w.require::<CapsuleController>("player").velocity);
        (v.length() > 0.02).then(|| math::atan2(v.x, v.z))
    });
    if let Some(target) = target {
        let left = wrap_angle(target - body.yaw);
        let turn = (left.abs() * TURN).min(TURN_MAX) * dt;
        body.yaw = wrap_angle(body.yaw + left.clamp(-turn, turn));
    }
    let facing = Quat::from_rotation_y(body.yaw);
    if w.require::<Transform>("stickman").rotation != facing {
        w.require_mut::<Transform>("stickman").rotation = facing;
    }
    motion.act = match body.action {
        Action::Ground => Act::Move,
        Action::Air => Act::Air,
        Action::Vault(_) => Act::Vault,
        Action::Hang(_) => Act::Hang,
        Action::Climb(_) => Act::Climb,
        Action::Roll { .. } => Act::Roll,
        Action::Land { .. } => Act::Land,
        Action::Ragdoll => Act::Air,
        Action::Rise { .. } => Act::Rise,
    };
    *w.require_mut::<Parkour>("player") = body;
    motion
}

type Outcome = (Action, Motion, Option<f32>);

// On the ground or in the air, under the player's control.
fn free(w: &mut World, body: &mut Parkour, controls: &Controls) -> Outcome {
    let dt = w.dt();
    let centre = w.require::<Transform>("player").position;
    let (was_grounded, before) = {
        let c = w.require::<CapsuleController>("player");
        (c.grounded, c.velocity)
    };
    let may_jump = was_grounded || body.airtime < COYOTE;

    // A jump at a wall is a vault or a climb, if the wall allows one.
    if body.buffer > 0.0 && may_jump {
        body.buffer = 0.0;
        let dir = if controls.wish.length() > 0.2 {
            controls.wish.normalize()
        } else {
            heading(body.yaw)
        };
        if let Some(next) = over_wall(w, centre, dir, planar(before).length()) {
            let (path, act) = match &next {
                Action::Vault(p) => (*p, Act::Vault),
                Action::Climb(p) => (*p, Act::Climb),
                _ => unreachable!("over_wall returns a vault or a climb"),
            };
            release(w);
            return (next, scripted(act, &path), Some(yaw_of(path.exit, dir)));
        }
        let mut c = w.require_mut::<CapsuleController>("player");
        Jump {
            height: JUMP,
            gravity: GRAVITY,
        }
        .start(&mut c.velocity);
        c.grounded = false;
        body.airtime = COYOTE;
    }

    let velocity = {
        let mut c = w.require_mut::<CapsuleController>("player");
        let speed = if controls.sprint { SPRINT } else { RUN };
        if c.grounded {
            Move {
                speed,
                accel: ACCEL,
                brake: BRAKE,
            }
            .step(&mut c.velocity, controls.wish, dt);
        } else if controls.wish != Vec3::ZERO {
            // In the air a release keeps the momentum.
            Move {
                speed: speed.max(planar(c.velocity).length()),
                accel: AIR_ACCEL,
                brake: 0.0,
            }
            .step(&mut c.velocity, controls.wish, dt);
        }
        Gravity(GRAVITY).step(&mut c.velocity, dt);
        c.velocity
    };
    let moved = physics::capsule(w, "player").step(velocity);
    let speed = planar(moved.displacement).length() / dt;
    let mut motion = Motion {
        speed,
        rising: velocity.y,
        grounded: moved.grounded,
        ..Motion::default()
    };

    if moved.grounded {
        let impact = if was_grounded { 0.0 } else { body.fall };
        body.airtime = 0.0;
        body.fall = 0.0;
        motion.impact = impact;
        return (touchdown(w, body, impact, speed), motion, None);
    }
    body.airtime += dt;
    body.fall = body.fall.max(-velocity.y);

    // In the air the hands catch an edge the body is moving at. It looks where the
    // stick points, where the body is going, and where it faces: a body that met
    // the wall on a slant slides along it, and only the stick still points in.
    if body.regrab > 0.0 {
        return (Action::Air, motion, None);
    }
    let centre = w.require::<Transform>("player").position;
    let toward = |into: Vec3| planar(velocity).dot(into) > 0.3 || controls.wish.dot(into) > 0.3;
    let looks = [controls.wish, planar(velocity), heading(body.yaw)]
        .map(|v| v.normalize_or_zero())
        .into_iter()
        .filter(|v| *v != Vec3::ZERO);
    for dir in looks {
        if velocity.y < 2.0 {
            let grab = ledge::probe(w, centre, dir, 0.45, GRAB_LOW, GRAB_HIGH)
                .filter(|l| toward(l.inward()) && ledge::clear(w, l.hang()));
            if let Some(ledge) = grab {
                release(w);
                body.fall = 0.0;
                let yaw = yaw_of(ledge.inward(), dir);
                let hanging = Hang {
                    ledge,
                    caught: centre,
                    live: controls.wish.dot(ledge.inward()) < 0.3,
                    ..Hang::default()
                };
                return (Action::Hang(hanging), Motion::default(), Some(yaw));
            }
        }
        // A lower edge it pulls straight onto: a jump that fell a little short.
        if velocity.y < 1.0 {
            let catch = ledge::probe(w, centre, dir, 0.4, VAULT_LOW, GRAB_LOW)
                .filter(|l| l.standable && toward(l.inward()));
            if let Some(ledge) = catch {
                release(w);
                body.fall = 0.0;
                let path = climb(centre, &ledge, 0.0, 0.0);
                let yaw = yaw_of(ledge.inward(), dir);
                return (Action::Climb(path), scripted(Act::Climb, &path), Some(yaw));
            }
        }
    }
    (Action::Air, motion, None)
}

// The move a jump at the wall ahead becomes, if there is such a wall.
fn over_wall(w: &World, centre: Vec3, dir: Vec3, speed: f32) -> Option<Action> {
    let ledge = ledge::probe(w, centre, dir, 1.2, VAULT_LOW, CLIMB_HIGH)?;
    let square = dir.dot(ledge.inward());
    if square < 0.6 {
        return None;
    }
    let height = ledge.edge.y - (centre.y - HALF);
    if height <= VAULT_HIGH && ledge.depth <= VAULT_DEEP {
        // Straight on along the heading, so a slanted approach crosses on a slant.
        let across = ledge.depth / square;
        let top = ledge.edge.y;
        let middle = ledge.edge + dir * (across * 0.5);
        let over = Vec3::new(
            middle.x,
            (top + VAULT_OVER).max(centre.y - VAULT_DIP),
            middle.z,
        );
        // The hand goes down on the top beside the course, or nearer it where the
        // wall ends.
        let hand = [VAULT_HAND, VAULT_HAND * 0.5]
            .into_iter()
            .map(|side| middle + ledge.left() * (side / square))
            .find(|at| {
                ledge::ground(w, *at + Vec3::Y * 0.2, 0.3).is_some_and(|y| (y - top).abs() < 0.03)
            });
        let beyond = ledge.edge + dir * (across + RADIUS + 0.45);
        let floor = ledge::ground(w, beyond + Vec3::Y * 0.3, 3.0);
        let to = Vec3::new(
            beyond.x,
            floor.map_or(over.y - 0.5, |y| y + HALF + 0.02),
            beyond.z,
        );
        if ledge::clear(w, to) {
            let pace = speed.clamp(3.0, SPRINT);
            let length = (over - centre).length() + (to - over).length();
            return Some(Action::Vault(Path {
                from: centre,
                over,
                to,
                time: 0.0,
                span: (length / pace).clamp(0.4, 0.75),
                exit: dir * pace,
                height,
                ledge,
                hand: hand.unwrap_or_default() + Vec3::Y * hold::WRIST,
                held: hand.is_some(),
                ..Path::default()
            }));
        }
    }
    ledge
        .standable
        .then(|| Action::Climb(climb(centre, &ledge, 0.0, 0.0)))
}

// The course from `centre` up the wall's face and onto its top.
fn climb(centre: Vec3, ledge: &Ledge, hug: f32, travel: f32) -> Path {
    let to = ledge.stand();
    let height = ledge.edge.y - (centre.y - HALF);
    Path {
        from: centre,
        // Clear of the face, with the centre, which is at the hips, just over the
        // edge: the height of a body that has pressed up onto straight arms. A
        // body that starts higher than that does not come down to it.
        over: ledge.edge
            + ledge.out * (RADIUS * 0.8)
            + Vec3::Y * (centre.y - ledge.edge.y).clamp(0.2, HALF),
        to,
        time: 0.0,
        span: 0.45 + 0.3 * height.clamp(0.0, 2.1),
        exit: Vec3::ZERO,
        height,
        ledge: *ledge,
        hug,
        travel,
        ..Path::default()
    }
}

// Take the body off the controller's ground for a scripted move.
fn release(w: &mut World) {
    let mut c = w.require_mut::<CapsuleController>("player");
    c.velocity = Vec3::ZERO;
    c.grounded = false;
    c.support = None;
}

fn yaw_of(v: Vec3, fallback: Vec3) -> f32 {
    let v = if planar(v).length() > 1e-3 { v } else { fallback };
    math::atan2(v.x, v.z)
}

fn scripted(act: Act, path: &Path) -> Motion {
    Motion {
        act,
        progress: path.progress(),
        height: path.height,
        ..Motion::default()
    }
}

// Fly a vault or a climb along its course, then hand the body back.
fn follow(w: &mut World, mut path: Path, act: Act, again: fn(Path) -> Action) -> Outcome {
    path.time += w.dt();
    let u = path.progress();
    // A climb eases in and out; a vault keeps its pace.
    let along = if act == Act::Climb { smooth(u) } else { u };
    w.require_mut::<Transform>("player").position = path.at(along);
    let face = Some(yaw_of(path.exit, path.to - path.from));
    if u < 1.0 {
        return (again(path), scripted(act, &path), face);
    }
    let mut c = w.require_mut::<CapsuleController>("player");
    c.velocity = path.exit;
    // A climb ends standing; a vault ends a step above the ground and lands.
    c.grounded = act == Act::Climb;
    let next = if c.grounded {
        Action::Ground
    } else {
        Action::Air
    };
    (next, Motion::default(), face)
}

// Hanging: still, along the edge, up and over, or let go.
fn hang(w: &mut World, body: &mut Parkour, mut hanging: Hang, controls: &Controls) -> Outcome {
    let dt = w.dt();
    let ledge = hanging.ledge;
    let face = Some(yaw_of(ledge.inward(), ledge.inward()));
    hanging.time += dt;
    hanging.live |= controls.wish.dot(ledge.inward()) < 0.3;
    let wish = if hanging.live {
        controls.wish
    } else {
        Vec3::ZERO
    };
    let up = body.buffer > 0.0 || wish.dot(ledge.inward()) > SECTOR;
    let away = wish.dot(ledge.out) > SECTOR;
    if up && ledge.standable {
        body.buffer = 0.0;
        let centre = w.require::<Transform>("player").position;
        let path = climb(centre, &ledge, HUG * hanging.settled(), hanging.travel);
        return (Action::Climb(path), scripted(Act::Climb, &path), face);
    }
    if away || (up && !ledge.standable) {
        body.buffer = 0.0;
        body.regrab = REGRAB;
        body.airtime = COYOTE;
        w.require_mut::<CapsuleController>("player").velocity = ledge.out * 1.2;
        return (Action::Air, Motion::default(), face);
    }
    // Along the edge, hand over hand, as far as the edge goes on under the hand
    // that leads and the body has room under it.
    let side = wish.dot(ledge.left());
    let want = if side.abs() > 0.3 && hanging.time >= SETTLE {
        side.signum() * SHIMMY
    } else {
        0.0
    };
    let change = SHIMMY_ACCEL * dt;
    hanging.speed += (want - hanging.speed).clamp(-change, change);
    if hanging.speed != 0.0 {
        let step = hanging.speed * dt;
        let lead = step.signum() * (HANDS + HAND_STEP * 0.5);
        let next = ledge::along(w, &ledge, step).filter(|next| {
            ledge::along(w, &ledge, step + lead).is_some() && ledge::clear(w, next.hang())
        });
        match next {
            Some(next) => {
                hanging.ledge = next;
                hanging.travel += step;
            }
            None => hanging.speed = 0.0,
        }
    }
    w.require_mut::<Transform>("player").position =
        hanging.caught.lerp(hanging.ledge.hang(), hanging.settled());
    let motion = Motion {
        act: Act::Hang,
        speed: hanging.speed.abs(),
        ..Motion::default()
    };
    (Action::Hang(hanging), motion, face)
}

// How far from under the capsule the drawn body is, toward the wall it is on.
fn hug(action: &Action) -> Vec3 {
    match action {
        Action::Hang(hanging) => hanging.ledge.inward() * (HUG * hanging.settled()),
        // A climb leaves the wall's face as the chest comes over the edge.
        Action::Climb(path) => {
            path.ledge.inward() * (path.hug * (1.0 - smooth(path.progress() / 0.5)))
        }
        _ => Vec3::ZERO,
    }
}

// The two points on an edge where the hands of a body that has shimmied `travel`
// metres are. A step of the shimmy moves one hand and then the other: the hand on
// the side the body is going to moves first.
fn grips(ledge: &Ledge, travel: f32) -> [Hold; 2] {
    let over = ledge.edge + ledge.inward() * 0.02 + Vec3::Y * hold::WRIST;
    let cycle = travel / HAND_STEP;
    let hand = |limb: Limb, side: f32, cycle: f32| {
        let whole = math::floor(cycle);
        // The first half of a cycle carries the hand; the second leaves it planted.
        let swing = ((cycle - whole) * 2.0).min(1.0);
        let along = side * HANDS + HAND_STEP * (whole + smooth(swing)) - travel;
        let lift = 0.06 * math::sin(std::f32::consts::PI * swing);
        Hold::new(limb, over + ledge.left() * along + Vec3::Y * lift)
    };
    [
        hand(Limb::HandL, 1.0, cycle),
        hand(Limb::HandR, -1.0, cycle - 0.5),
    ]
}

/// What the hands and feet hold this tick: the edge of a hang, the edge and then
/// the top of a climb, the top of a wall under a vault.
pub fn holds(w: &World) -> Vec<Hold> {
    let centre = w.require::<Transform>("player").position;
    match &w.require::<Parkour>("player").action {
        Action::Hang(hanging) => grips(&hanging.ledge, hanging.travel).to_vec(),
        Action::Vault(path) if path.held => vec![Hold::new(Limb::HandL, path.hand)],
        Action::Climb(path) => {
            let ledge = &path.ledge;
            // How far the feet's level is above the edge; under it is negative.
            let level = centre.y - HALF - ledge.edge.y;
            // Where the feet will stand at the end of the climb.
            let stand = path.to - Vec3::Y * (HALF + 0.01);
            // A foot comes up outside the wall's face to over its top, knee out to
            // the side, and then goes in onto the top, where it stands. It starts
            // when the feet's level is `from` metres from the edge and is down
            // at `to`, and no sooner than the climb is well begun.
            let step = |limb: Limb, side: f32, from: f32, to: f32| {
                let gate = smooth((level - from) / (to - from)).min(smooth(path.progress() / 0.45));
                let beside = ledge.left() * (side * 0.09);
                let lifted = ledge.edge + ledge.out * 0.32 + beside + Vec3::Y * 0.2;
                let down = stand + beside - ledge.inward() * 0.03 + Vec3::Y * hold::ANKLE;
                let carry = smooth((gate - 0.5) / 0.5);
                let knee = Vec3::new(side * 0.8, 0.3, 0.2).lerp(Vec3::new(side * 0.1, 0.3, 0.8), carry);
                Hold::new(limb, lifted.lerp(down, carry))
                    .grip(smooth(gate / 0.5))
                    .bend(knee)
            };
            let mut holds = grips(ledge, path.travel).to_vec();
            // One foot goes onto the top as the hips reach it, and the body stands
            // up over that foot. The other follows when the body is over the top.
            holds.push(step(Limb::FootR, -1.0, -1.3, -0.7));
            holds.push(step(Limb::FootL, 1.0, -0.9, -0.3));
            holds
        }
        _ => vec![],
    }
}

// What a touchdown at `impact` m/s downward and `speed` m/s over the ground becomes.
fn touchdown(w: &mut World, body: &mut Parkour, impact: f32, speed: f32) -> Action {
    if impact < SOFT {
        return Action::Ground;
    }
    if impact >= HARD {
        // The legs cannot take it. The body goes down the way it was moving, or
        // forward onto its hands if it came straight down.
        let velocity = planar(w.require::<CapsuleController>("player").velocity);
        let tip = if speed > 0.5 {
            velocity.normalize()
        } else {
            heading(body.yaw)
        };
        fall_down(w, body, velocity - Vec3::Y * (impact * 0.5), tip * 1.5);
        return Action::Ragdoll;
    }
    // How far into the band between a soft landing and a fall, 0 to 1.
    let strain = ((impact - SOFT) / (HARD - SOFT)).clamp(0.0, 1.0);
    let velocity = planar(w.require::<CapsuleController>("player").velocity);
    if speed >= ROLL_SPEED {
        Action::Roll {
            time: 0.0,
            span: 0.55 + 0.35 * strain,
            heading: velocity.normalize(),
            speed: velocity.length(),
        }
    } else {
        Action::Land {
            time: 0.0,
            span: 0.45 + 0.65 * strain,
        }
    }
}

// Rolling: the heading is fixed and the speed bleeds to a walk.
fn roll(w: &mut World, time: f32, span: f32, heading: Vec3, speed: f32) -> Outcome {
    let u = (time / span).clamp(0.0, 1.0);
    let pace = math::lerp(speed.max(2.5), speed.min(2.0), smooth(u));
    let velocity = {
        let mut c = w.require_mut::<CapsuleController>("player");
        c.velocity = Vec3::new(heading.x * pace, c.velocity.y, heading.z * pace);
        Gravity(GRAVITY).step(&mut c.velocity, w.dt());
        c.velocity
    };
    let moved = physics::capsule(w, "player").step(velocity);
    let motion = Motion {
        act: Act::Roll,
        progress: u,
        speed: pace,
        rising: velocity.y,
        grounded: moved.grounded,
        ..Motion::default()
    };
    let next = if u < 1.0 {
        Action::Roll {
            time,
            span,
            heading,
            speed,
        }
    } else if moved.grounded {
        Action::Ground
    } else {
        Action::Air
    };
    (next, motion, Some(yaw_of(heading, heading)))
}

// Crouched after a hard landing: the body brakes and rises.
fn land(w: &mut World, time: f32, span: f32) -> Outcome {
    let dt = w.dt();
    let velocity = {
        let mut c = w.require_mut::<CapsuleController>("player");
        Move {
            speed: 0.0,
            accel: BRAKE,
            brake: BRAKE,
        }
        .step(&mut c.velocity, Vec3::ZERO, dt);
        Gravity(GRAVITY).step(&mut c.velocity, dt);
        c.velocity
    };
    let moved = physics::capsule(w, "player").step(velocity);
    let u = (time / span).clamp(0.0, 1.0);
    let motion = Motion {
        act: Act::Land,
        progress: u,
        grounded: moved.grounded,
        ..Motion::default()
    };
    let next = if u < 1.0 {
        Action::Land { time, span }
    } else {
        Action::Ground
    };
    (next, motion, None)
}

/// What a moving thing did to the body this tick.
#[derive(Clone, Copy, Debug)]
enum Hit {
    /// Added to the body's velocity, m/s.
    Shove(Vec3),
    /// Knocked down, the chest thrown at this velocity, m/s.
    Knock(Vec3),
}

// The hardest hit from the props that touch the capsule, by the momentum each
// brings toward the body.
fn struck(w: &World) -> Option<Hit> {
    let centre = w.require::<Transform>("player").position;
    let own = planar(w.require::<CapsuleController>("player").velocity);
    let shape = Shape::Capsule {
        radius: RADIUS + 0.05,
        height: HEIGHT,
    };
    let touching = physics::overlap(w, &shape, Transform::at(centre.x, centre.y, centre.z), PROP);
    touching
        .into_iter()
        .filter_map(|e| {
            let prop = w.get::<physics::Body>(e)?;
            let at = w.get::<Transform>(e)?.position;
            let toward = planar(centre - at).normalize_or_zero();
            let closing = (prop.velocity - own).dot(toward);
            (closing > 0.5).then(|| (prop.mass * closing, toward))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(momentum, toward)| {
            if momentum >= KNOCK {
                Hit::Knock(toward * (momentum / MASS).clamp(2.5, 9.0) + Vec3::Y * 1.5)
            } else {
                Hit::Shove(toward * (momentum / MASS).min(4.0))
            }
        })
}

// Hand the body to the ragdoll, moving at `velocity` with `kick` at the chest.
fn fall_down(w: &mut World, body: &mut Parkour, velocity: Vec3, kick: Vec3) {
    ragdoll::start(w, velocity, kick);
    release(w);
    body.action = Action::Ragdoll;
    body.fall = 0.0;
    body.buffer = 0.0;
    // A body that was rising has its visual off the usual place under the capsule.
    w.require_mut::<Transform>("stickman").position = Vec3::new(0.0, -HALF - 0.01, 0.0);
}

// Limp until the ragdoll rests, then find where to stand and start to rise.
fn limp(w: &mut World, body: &mut Parkour) -> Outcome {
    if !ragdoll::step(w) {
        return (Action::Ragdoll, Motion::default(), None);
    }
    let doll = w.require::<Ragdoll>("player").clone();
    let hips = doll.hips();
    let supine = doll.face_up();
    // The rise is drawn with the head toward +Z for a body on its front, and
    // toward -Z for one on its back.
    let lying = doll.lying(body.yaw);
    body.yaw = if supine {
        wrap_angle(lying + std::f32::consts::PI)
    } else {
        lying
    };
    // It stands up onto its feet, which is ahead of where its hips lay.
    let ahead = if supine {
        crate::rise::SUPINE_BACK
    } else {
        crate::rise::PRONE_BACK
    };
    let spot = hips + heading(body.yaw) * ahead;
    let floor = ledge::ground(w, hips + Vec3::Y * 0.4, 3.0).unwrap_or(hips.y - 0.1);
    let standing = |x: f32, z: f32| Vec3::new(spot.x + x, floor + HALF + 0.01, spot.z + z);
    // There, or at the nearest place around it that is clear. A place clear of
    // walls but not of a prop is second best: the body can push a prop away.
    let ring = |layers: u32| {
        [0.0f32, 0.4, 0.8, 1.2, 1.6]
            .into_iter()
            .flat_map(|far| {
                (0..8).map(move |i| {
                    let (sin, cos) = math::sin_cos(i as f32 * std::f32::consts::FRAC_PI_4);
                    (far * sin, far * cos)
                })
            })
            .map(|(x, z)| standing(x, z))
            .find(|at| ledge::clear_of(w, *at + Vec3::Y * 0.02, layers))
    };
    let to = ring(crate::park::SOLID | PROP)
        .or_else(|| ring(crate::park::SOLID))
        .unwrap_or(standing(0.0, 0.0));
    let next = Action::Rise {
        time: 0.0,
        from: hips,
        to,
        supine,
    };
    let motion = Motion {
        supine,
        ..Motion::default()
    };
    (next, motion, None)
}

// Rising: the visual's root is at the standing place from the first moment, so
// planted hands and feet stay where they are, and the capsule comes to it.
fn rise(w: &mut World, time: f32, from: Vec3, to: Vec3, supine: bool) -> Outcome {
    let u = (time / RISE).clamp(0.0, 1.0);
    let centre = from.lerp(to, smooth(u));
    w.require_mut::<Transform>("player").position = centre;
    let root = to - Vec3::Y * (HALF + 0.01);
    w.require_mut::<Transform>("stickman").position = root - centre;
    let motion = Motion {
        act: Act::Rise,
        progress: u,
        supine,
        ..Motion::default()
    };
    if u < 1.0 {
        let next = Action::Rise {
            time,
            from,
            to,
            supine,
        };
        return (next, motion, None);
    }
    ragdoll::stop(w);
    release(w);
    (Action::Ground, motion, None)
}

/// How high a point of a vaulting body must be to pass the wall: over the top by
/// `over` metres between the wall's faces, and down from there at `STEEP` metres
/// per metre before the near face and after the far one. The feet follow this, so
/// they go over the wall wherever along it and from whatever side they meet it.
fn clearance(ledge: &Ledge, at: Vec3, over: f32) -> f32 {
    const STEEP: f32 = 3.0;
    let before = (at - ledge.edge).dot(ledge.out);
    let outside = before.max(-ledge.depth - before).max(0.0);
    ledge.edge.y + over - STEEP * outside
}

// Where the feet of a vaulting body must be lifted to, for each foot the pose
// has too low.
fn lifted(w: &World, path: &Path) -> Vec<Hold> {
    // From the ankle to the tip of the toes, in the foot's frame.
    const TOE: Vec3 = Vec3::new(0.0, -0.06, 0.19);
    [(Limb::FootL, "foot_l"), (Limb::FootR, "foot_r")]
        .into_iter()
        .filter_map(|(limb, name)| {
            let ankle = hold::joint(w, name);
            let toe = hold::point(w, name, TOE);
            let short = (clearance(&path.ledge, ankle, hold::ANKLE + 0.04) - ankle.y)
                .max(clearance(&path.ledge, toe, 0.05) - toe.y);
            (short > 0.0).then(|| Hold::new(limb, ankle + Vec3::Y * short).loose())
        })
        .collect()
}

/// Lay the ragdoll's pose over the Animator's, or put the hands and feet on what
/// they hold. Call after `animation::step`.
pub fn present(w: &mut World) {
    let (action, yaw) = {
        let body = w.require::<Parkour>("player");
        (body.action.clone(), body.yaw)
    };
    let weight = match action {
        Action::Ragdoll => 1.0,
        // The limbs gather from where they lay into the first of the rise.
        Action::Rise { time, .. } if time < GATHER => 1.0 - smooth(time / GATHER),
        _ => {
            let holds = holds(w);
            hold::apply(w, &holds);
            if let Action::Vault(path) = action {
                let lifted = lifted(w, &path);
                hold::apply(w, &lifted);
            }
            return;
        }
    };
    let doll = w.require::<Ragdoll>("player").clone();
    if !doll.active() {
        return;
    }
    let root = w.require::<Transform>("player").position + w.require::<Transform>("stickman").position;
    let (hips, bends) = ragdoll::pose(&doll, root, yaw);
    ragdoll::show(w, hips, &bends, weight);
}
