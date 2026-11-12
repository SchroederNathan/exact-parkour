//! The player's body: an upright capsule that the physics controller moves through
//! the world, and what that body is doing.
//!
//! `Action` is the whole story of the body at one tick. Each variant owns the data
//! of its move, and `step` is the only place one becomes another.
use crate::air;
use crate::climb::{self, Climb};
use crate::flow::{self, Flow};
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
/// On its feet the body's turn starts and stops gently: its rate changes by at
/// most this much, rad/s².
const SPIN: f32 = 400.0;
/// On its feet the body faces where its velocity will be when the stick has
/// pushed it this much further, m/s. An about-face so starts while the body
/// still brakes, and the body is side-on as it stops.
const FACE_AHEAD: f32 = 4.5;
/// How far the body leans when it speeds up as hard as it can, radians, and how
/// fast the lean follows what pushes it, per second: a spring with no bounce.
const TILT: f32 = 0.42;
const BALANCE: f32 = 14.0;
/// A change of ground speed under this, m/s², is none.
const STEADY: f32 = 0.5;
/// Of a lean. How far the hips go toward it, as a share of where a body that
/// tipped stiff about its feet would have them. How far they sink, metres per
/// radian. The shares of it that the pelvis, the spine and the chest turn by,
/// and the share the neck turns back.
const HIPS_OUT: f32 = 0.5;
const HIPS_SINK: f32 = 0.25;
const TRUNK: [f32; 3] = [0.5, 0.3, 0.2];
const NECK_BACK: f32 = 0.5;
/// How far the head turns toward a turn that the body has still to make, as a
/// share of that turn and at most, radians; and how fast it does, per second.
const LOOK: f32 = 0.5;
const LOOK_MOST: f32 = 0.5;
const LOOK_RATE: f32 = 16.0;
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
/// Seconds the body takes to swing in under an edge its hands caught, and the
/// time by which nothing is left of how it came. What is left before that dies
/// away at `CATCH` per second.
pub const SETTLE: f32 = 0.25;
const SWUNG_IN: f32 = 0.38;
/// How far from its shoulder a hand goes toward an edge it cannot reach, metres.
const ARM: f32 = 0.5;
const CATCH: f32 = 14.0;
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
const HAND_STEP: f32 = 0.28;

/// A vault carries the capsule's centre, which is at the hips, this far above the
/// wall's top, metres: an arm's length under the shoulder of a body that leans on
/// one hand. Over a wall too low for that it stays this far under a stand.
const VAULT_OVER: f32 = 0.35;
const VAULT_DIP: f32 = 0.15;
/// How far to the left of its course a vaulting body puts its hand down, metres.
const VAULT_HAND: f32 = 0.42;

/// Landings, by downward speed at touchdown in m/s. Each is deeper the harder it
/// is. Under `SOFT` the knees give. From there to `ROLL` the body kneels into the
/// landing and runs on out of it. From `ROLL` to `HARD` it must roll, or crouch
/// to its hands if it has no ground speed to roll with, and the recovery grows
/// with the speed. At `HARD` and above nothing saves it: the body falls.
pub const SOFT: f32 = 4.2;
pub const ROLL: f32 = 6.3;
pub const HARD: f32 = 9.0;
/// Ground speed a roll needs, m/s.
const ROLL_SPEED: f32 = 1.2;

/// How fast a body whose feet are past an edge goes off it, m/s.
const SLIP: f32 = 1.2;
/// A slide: the slowest ground speed it starts from, m/s, and how long one lasts
/// on level ground, seconds. The ground takes `SLIDE_DRAG` of its speed each
/// second, and it never goes faster than `SLIDE_TOP`, m/s.
pub const SLIDE_MIN: f32 = 2.5;
pub const SLIDE: f32 = 0.8;
pub const SLIDE_DRAG: f32 = 0.2;
pub const SLIDE_TOP: f32 = 10.0;
/// How far through its pose a slide rides, 0 to 1. It waits there while the
/// slope pulls it on at `SLIDE_PULL` m/s² or more: a slope of about 4 degrees.
pub const SLIDE_RIDE: f32 = 0.5;
const SLIDE_PULL: f32 = 0.6;
/// The stick leans a sliding body: all the way when it points this far off the
/// heading, radians. The lean follows as a spring with no bounce at `LEAN` rad/s,
/// never faster than `LEAN_MOST` a second: half a second from one side to the
/// other. The stick and a full lean together turn the slide at `SLIDE_TURN` rad/s.
const SLIDE_AIM: f32 = 0.6;
const LEAN: f32 = 12.0;
const LEAN_MOST: f32 = 4.0;
pub const SLIDE_TURN: f32 = 2.2;
/// How fast the speed a slide left the body with, above what its legs run, dies
/// away on its feet, m/s².
const MOMENTUM_LOSS: f32 = 1.5;
/// A jump off an edge the body hangs from: along the edge and up, m/s, and how
/// long after it the hands still look for the wall it left, seconds.
const LEAP_ALONG: f32 = 4.0;
const LEAP_UP: f32 = 4.2;
const LEAP: f32 = 0.9;
/// Seconds a hanging body swings its legs back before it leaps.
const WIND_UP: f32 = 0.12;
/// How far a body that peeks past the end of an edge swings out, metres, and how
/// fast it leans there and back, per second.
const PEEK: f32 = 0.22;
const PEEK_RATE: f32 = 4.0;

/// A thing that hits the body with at least this much momentum, in kg·m/s, knocks
/// it down. A lighter hit only shoves it. A 30 kg ball at 7 m/s is 210.
pub const KNOCK: f32 = 200.0;
/// The body's mass for what a hit does to its speed, kg.
const MASS: f32 = 80.0;
/// Seconds to rise from the ground, and the first part of it, in which the limbs
/// gather from wherever the fall left them.
const RISE: f32 = 1.8;
pub const GATHER: f32 = 0.4;

/// A vault's fixed course of the capsule's centre through three points.
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
}

/// A body that hangs from an edge by its hands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
pub struct Hang {
    /// The edge, at the point the body hangs under.
    pub ledge: Ledge,
    /// What the capsule is off its place under the edge, metres, and how fast
    /// that changes: what is left of where and how fast the body was when the
    /// hands caught the edge.
    pub off: Vec3,
    pub drift: Vec3,
    /// Seconds since the catch.
    pub time: f32,
    /// Metres shimmied along the edge, and the speed of it; to the left is positive.
    pub travel: f32,
    pub speed: f32,
    /// How far the body leans out past the end of the edge to look along the
    /// wall, -1 to 1; to the left is positive.
    pub lean: f32,
    /// How fast the body met the wall and how fast it was going along it when
    /// the hands caught the edge, m/s: what its legs swing with.
    pub pace: f32,
    pub swung: f32,
    /// How far the arms had gone up to the edge before the hands caught it, 0 to
    /// 1, and how far the body leaned along the wall as it came, -1 to 1.
    pub reach: f32,
    pub carried: f32,
    /// A leap along the wall that is winding up: the seconds until it goes, and
    /// the way it will go, +1 to the left.
    pub wind: f32,
    pub aim: f32,
    /// Whether the stick has stopped pushing at the wall since the catch. Until
    /// then it still holds the push of the run-up, which is not a wish to climb.
    pub live: bool,
}
/// A body that slides along the ground.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
pub struct Slide {
    /// How far through its pose, 0 to 1: down, riding, up.
    pub phase: f32,
    /// The way it goes, and how fast, m/s.
    pub heading: Vec3,
    pub speed: f32,
    /// How far the body leans into a turn, -1 to 1, and how fast that changes,
    /// per second; to the left is positive.
    pub lean: f32,
    pub tip: f32,
    /// The normal of the ground the body lies on, which follows the ground
    /// under it over a moment.
    pub ground: Vec3,
}
impl Slide {
    /// How much of the ride's pose the body is in, 0 to 1: none while it goes
    /// down and while it comes up.
    pub fn riding(&self) -> f32 {
        smooth((self.phase - 0.1) / 0.2) * (1.0 - smooth((self.phase - 0.74) / 0.14))
    }
    /// The height at `at` of the ground the body lies on, with the capsule's
    /// feet at `feet`. That is the ground under `at`, unless a wall's top or a
    /// drop is there: a hand or a foot then stays level with the body's own
    /// ground, and does not go up onto the wall or down into the drop.
    fn floor(&self, w: &World, feet: Vec3, at: Vec3) -> f32 {
        let across = self.ground.x * (at.x - feet.x) + self.ground.z * (at.z - feet.z);
        let lies = feet.y - across / self.ground.y.max(0.5);
        ledge::ground(w, Vec3::new(at.x, lies + 0.6, at.z), 1.2)
            .filter(|y| (y - lies).abs() < 0.35)
            .unwrap_or(lies)
    }
}
impl Hang {
    /// How far the body has swung in under the edge, 0 to 1.
    pub fn settled(&self) -> f32 {
        smooth(self.time / SETTLE)
    }
    /// The capsule's centre: under the edge, less what is left of the catch,
    /// which is all gone when the body has settled.
    pub fn centre(&self) -> Vec3 {
        self.ledge.hang() + self.off * (1.0 - smooth(self.time / SWUNG_IN))
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
    Climb(Climb),
    /// Rolling out of a hard landing along `heading`, from `speed` m/s. `strain`
    /// is how hard the landing was, 0 to 1.
    Roll {
        time: f32,
        span: f32,
        heading: Vec3,
        speed: f32,
        strain: f32,
    },
    /// Sliding along the ground on one hip.
    Slide(Slide),
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
    Slide,
}
impl Act {
    pub const ALL: [Act; 9] = [
        Act::Move,
        Act::Air,
        Act::Vault,
        Act::Hang,
        Act::Climb,
        Act::Roll,
        Act::Land,
        Act::Rise,
        Act::Slide,
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
            Act::Slide => "slide",
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
    /// How deep the body kneels out of this tick's landing or roll, 0 to 1; zero
    /// when it does not. `kneel_from` is how far into the kneel it starts, 0 to
    /// 1: a roll comes up out of the kneel's deepest moment.
    pub kneel: f32,
    pub kneel_from: f32,
    /// How ready the legs are for the ground that is coming, 0 to 1.
    pub brace: f32,
    /// Height of the wall a vault or climb is crossing, metres.
    pub height: f32,
    /// Whether a rising body lay on its back.
    pub supine: bool,
    /// The leg that leads in the air, +1 the left and -1 the right, and the
    /// ground speed the body left the ground with, m/s.
    pub lead: f32,
    pub stride: f32,
    /// Whether the pose changes at once this tick with no change of move: a
    /// jump from the air, or a kneel that a roll opens into.
    pub cut: bool,
    /// How far a sliding body leans into a turn, -1 to 1; to the left is positive.
    pub lean: f32,
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
    /// Seconds a slide press is still remembered.
    pub slide: f32,
    /// After a jump off an edge: the way into the wall it left, and the seconds
    /// the hands still look for that wall.
    pub wall: Vec3,
    pub leap: f32,
    /// Whether a finger that came from another touch button rests on Sprint.
    pub glide: bool,
    /// Seconds the legs are still slow after a landing they barely took.
    pub stagger: f32,
    /// In the air: an edge the hands could catch soon, and how far the arms have
    /// gone up to it, 0 to 1.
    pub ahead: Ledge,
    pub reach: f32,
    /// The leg that leads in the air, +1 the left and -1 the right: the other
    /// one pushed. The body lands on it. And the ground speed the body left the
    /// ground with, m/s, which is how long a stride its legs hold in the air.
    pub lead: f32,
    pub stride: f32,
    /// How ready the legs are for the ground that is coming, 0 to 1.
    pub brace: f32,
    /// In the air: a lower wall the body is about to meet, and how long it has
    /// been in sight, 0 to 1. The legs get out of its way before they are on it.
    pub low: Ledge,
    pub near: f32,
    /// Ground speed a slide left the body with, m/s. While it is more than the
    /// legs run, the body keeps it, and it dies away.
    pub momentum: f32,
    /// On its feet: how the body leans, as a vector on the ground. It points
    /// where the weight goes and is as long as the lean's angle, radians.
    /// `tipping` is how fast it changes, per second.
    pub tilt: Vec3,
    pub tipping: Vec3,
    /// How fast the body turns on its feet, rad/s; to the left is positive.
    pub spin: f32,
    /// How far the head is turned toward a turn the body has still to make,
    /// radians; to the left is positive.
    pub look: f32,
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
    slide: bool,
}

/// Read the controls, advance the action and move the capsule. Call before
/// `physics::step`.
pub fn step(w: &mut World, input: &Input) -> Motion {
    let dt = w.dt();
    let mut body = w.require::<Parkour>("player").clone();
    let before = planar(w.require::<CapsuleController>("player").velocity);
    let stood = matches!(body.action, Action::Ground);
    // A finger that slides from one touch button onto another presses the one it
    // arrives on. The app says so with a message, since the button the finger
    // went down on still holds the touch.
    let said = |what: &str| input.messages().iter().any(|m| m == what);
    if said("touch sprint on") {
        body.glide = true;
    }
    if said("touch sprint off") {
        body.glide = false;
    }
    let controls = Controls {
        wish: crate::camera::movement(w, planar(input.stick_xz("move"))).clamp_length_max(1.0),
        jump: input.pressed("jump") || said("touch jump"),
        sprint: input.held("sprint") || body.glide,
        slide: input.pressed("slide") || said("touch slide"),
    };
    body.buffer = if controls.jump {
        BUFFER
    } else {
        (body.buffer - dt).max(0.0)
    };
    body.slide = if controls.slide {
        BUFFER
    } else {
        (body.slide - dt).max(0.0)
    };
    body.regrab = (body.regrab - dt).max(0.0);
    body.leap = (body.leap - dt).max(0.0);
    body.stagger = (body.stagger - dt).max(0.0);

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
        Action::Vault(path) => vault(w, path),
        Action::Climb(climb) => climbing(w, climb),
        Action::Hang(hanging) => hang(w, &mut body, hanging, &controls),
        Action::Roll {
            time,
            span,
            heading,
            speed,
            strain,
        } => roll(w, time + dt, span, heading, speed, strain),
        Action::Slide(sliding) => slide(w, &mut body, &controls, sliding),
        Action::Land { time, span } => land(w, time + dt, span),
        Action::Ragdoll => limp(w, &mut body),
        Action::Rise {
            time,
            from,
            to,
            supine,
        } => rise(w, time + dt, from, to, supine),
    };
    // Into the air from a move that ends in the stride of a left foot's strike,
    // or from a hang, which is no stride.
    if matches!(action, Action::Air) && !matches!(body.action, Action::Ground | Action::Air) {
        body.lead = 1.0;
        body.stride = match body.action {
            Action::Hang(_) => 0.0,
            _ => planar(w.require::<CapsuleController>("player").velocity).length(),
        };
    }
    body.action = action;
    // The drawn body is under the capsule, or closer to the wall it climbs. A rise
    // puts it where the body will stand.
    if !matches!(body.action, Action::Rise { .. }) {
        let place = Vec3::new(0.0, -HALF - 0.01, 0.0) + hug(&body.action);
        if w.require::<Transform>("stickman").position != place {
            w.require_mut::<Transform>("stickman").position = place;
        }
    }

    let velocity = planar(w.require::<CapsuleController>("player").velocity);
    let on_feet = stood && matches!(body.action, Action::Ground);
    // On its feet the body leans the way it is pushed: forward as it speeds up,
    // back as it brakes, into a turn. Through an about-face the push keeps one
    // way in the world while the body comes round under it, so the lean goes
    // from the back over one side to the front and the weight never stops.
    // In any other move the lean it had dies away, twice as fast.
    if matches!(body.action, Action::Ragdoll | Action::Rise { .. }) {
        (body.tilt, body.tipping) = (Vec3::ZERO, Vec3::ZERO);
    } else {
        let (want, rate) = if on_feet {
            let push = ((velocity - before) / dt).clamp_length_max(ACCEL);
            // The capsule's speed is a few ulps off what it was asked for.
            let push = if push.length() < STEADY { Vec3::ZERO } else { push };
            (push * (TILT / ACCEL), BALANCE)
        } else {
            (Vec3::ZERO, 2.0 * BALANCE)
        };
        let pull = (want - body.tilt) * (rate * rate) - body.tipping * (2.0 * rate);
        body.tipping += pull * dt;
        body.tilt += body.tipping * dt;
        if want == Vec3::ZERO && body.tilt.length() < 1e-3 && body.tipping.length() < 1e-2 {
            (body.tilt, body.tipping) = (Vec3::ZERO, Vec3::ZERO);
        }
    }

    // Turn toward where the body is going, or where its move points it.
    let steered = face.is_none() && on_feet;
    let target = face.or_else(|| {
        let v = if steered {
            velocity + controls.wish * FACE_AHEAD
        } else {
            velocity
        };
        (v.length() > 0.02).then(|| math::atan2(v.x, v.z))
    });
    let mut look = 0.0;
    if let Some(target) = target {
        let mut left = wrap_angle(target - body.yaw);
        if steered {
            // Straight back is as near by the left as by the right. The body
            // turns away from the foot that is next on the ground, which is the
            // one it brakes on: that foot is then on the outside of the turn.
            if left.abs() > std::f32::consts::PI - 0.05 {
                let cycle = w.get::<Pose>("stickman").map_or(0.0, |pose| pose.phase);
                left = left.abs() * air::lead(cycle + 0.15);
            }
            let want = (left * TURN).clamp(-TURN_MAX, TURN_MAX);
            let change = SPIN * dt;
            body.spin += (want - body.spin).clamp(-change, change);
            let turn = body.spin * dt;
            let turn = if turn * left > 0.0 && turn.abs() > left.abs() {
                left
            } else {
                turn
            };
            body.yaw = wrap_angle(body.yaw + turn);
            look = (LOOK * wrap_angle(target - body.yaw)).clamp(-LOOK_MOST, LOOK_MOST);
        } else {
            let turn = (left.abs() * TURN).min(TURN_MAX) * dt;
            body.yaw = wrap_angle(body.yaw + left.clamp(-turn, turn));
            body.spin = 0.0;
        }
    } else {
        body.spin = 0.0;
    }
    // The head goes first into a turn.
    body.look += (look - body.look) * (1.0 - math::exp(-LOOK_RATE * dt));
    if look == 0.0 && body.look.abs() < 1e-4 {
        body.look = 0.0;
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
        Action::Slide(_) => Act::Slide,
        Action::Land { .. } => Act::Land,
        Action::Ragdoll => Act::Air,
        Action::Rise { .. } => Act::Rise,
    };
    motion.lead = body.lead;
    motion.stride = body.stride;
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
    let mut jumped = false;

    // A jump at a wall is a vault or a climb, if the wall allows one.
    if body.buffer > 0.0 && may_jump {
        body.buffer = 0.0;
        let dir = if controls.wish.length() > 0.2 {
            controls.wish.normalize()
        } else {
            heading(body.yaw)
        };
        let first = step_first(w);
        if let Some(next) = over_wall(w, centre, dir, before, first) {
            let (motion, face) = match &next {
                Action::Vault(path) => (scripted(path), yaw_of(path.exit, dir)),
                Action::Climb(climb) => (climb.motion(), yaw_of(climb.ledge.inward(), dir)),
                _ => unreachable!("over_wall returns a vault or a climb"),
            };
            release(w);
            return (next, motion, Some(face));
        }
        let mut c = w.require_mut::<CapsuleController>("player");
        Jump {
            height: JUMP,
            gravity: GRAVITY,
        }
        .start(&mut c.velocity);
        c.grounded = false;
        body.airtime = COYOTE;
        jumped = true;
    }

    // A slide takes the speed the body has.
    if body.slide > 0.0 && was_grounded && planar(before).length() >= SLIDE_MIN {
        body.slide = 0.0;
        let next = Slide {
            heading: planar(before).normalize(),
            speed: planar(before).length(),
            ground: ledge::slope(w, feet(centre) + Vec3::Y * 0.3, 0.8).unwrap_or(Vec3::Y),
            ..Slide::default()
        };
        let motion = Motion {
            act: Act::Slide,
            speed: next.speed,
            grounded: true,
            ..Motion::default()
        };
        return (Action::Slide(next), motion, None);
    }

    let velocity = {
        let mut c = w.require_mut::<CapsuleController>("player");
        let speed = if controls.sprint { SPRINT } else { RUN };
        // The stride after a landing the legs barely took is short.
        let speed = if body.stagger > 0.0 { speed * 0.7 } else { speed };
        // A body that a slide left faster than its legs run keeps that speed
        // while the stick is pushed, and loses it slowly on its feet.
        let pushed = controls.wish.length();
        body.momentum = if pushed > 0.0 {
            body.momentum.min(planar(c.velocity).length())
        } else {
            0.0
        };
        if c.grounded {
            body.momentum = (body.momentum - MOMENTUM_LOSS * dt).max(0.0);
        }
        let speed = if pushed > 0.0 {
            speed.max(body.momentum / pushed)
        } else {
            speed
        };
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
    // The body stands on its feet, not on the rim of its capsule. With its feet
    // past an edge it is off that edge, and goes on out until the rim is too.
    let slip = if moved.grounded {
        ledge::overhang(w, w.require::<Transform>("player").position)
    } else {
        None
    };
    if let Some(away) = slip {
        let mut c = w.require_mut::<CapsuleController>("player");
        c.grounded = false;
        c.support = None;
        let out = planar(c.velocity).dot(away);
        if out < SLIP {
            c.velocity += away * (SLIP - out);
        }
    }
    let grounded = moved.grounded && slip.is_none();
    // Off the ground, by a jump or past an edge: the leg that is not the one on
    // the ground leads, and the legs keep the stride of the speed they had.
    if was_grounded && !grounded {
        let cycle = w.get::<Pose>("stickman").map_or(0.0, |pose| pose.phase);
        body.lead = air::lead(cycle);
        body.stride = planar(before).length();
    }
    let mut motion = Motion {
        speed,
        rising: velocity.y,
        grounded,
        cut: jumped,
        ..Motion::default()
    };

    body.brace = 0.0;
    if grounded {
        body.reach = 0.0;
        body.near = 0.0;
        let impact = if was_grounded { 0.0 } else { body.fall };
        body.airtime = 0.0;
        body.fall = 0.0;
        body.leap = 0.0;
        motion.impact = impact;
        if (SOFT..ROLL).contains(&impact) {
            motion.kneel = 0.5 + 0.5 * (impact - SOFT) / (ROLL - SOFT);
            if motion.kneel > 0.8 {
                body.stagger = 0.25;
            }
        }
        return (touchdown(w, body, impact, speed), motion, None);
    }
    body.airtime += dt;
    body.fall = body.fall.max(-velocity.y);
    // The legs get ready in the last fifth of a second before the ground.
    if velocity.y < -1.0 {
        let feet = w.require::<Transform>("player").position - Vec3::Y * HALF;
        if let Some(floor) = ledge::ground(w, feet + Vec3::Y * 0.1, 3.0) {
            let seconds = (feet.y - floor).max(0.0) / -velocity.y;
            motion.brace = 1.0 - (seconds / 0.2).clamp(0.0, 1.0);
            body.brace = motion.brace;
        }
    }

    // In the air the hands catch an edge the body is moving at. It looks where the
    // stick points, where the body is going, and where it faces: a body that met
    // the wall on a slant slides along it, and only the stick still points in.
    // A body that jumped off an edge along its wall still faces that wall, and
    // its hands look for the next edge on it.
    let wall = if body.leap > 0.0 {
        body.wall
    } else {
        Vec3::ZERO
    };
    let facing = (wall != Vec3::ZERO).then(|| yaw_of(wall, wall));
    if body.regrab > 0.0 {
        return (Action::Air, motion, facing);
    }
    let centre = w.require::<Transform>("player").position;
    let toward = |into: Vec3| {
        planar(velocity).dot(into) > 0.3 || controls.wish.dot(into) > 0.3 || wall.dot(into) > 0.8
    };
    let looks = [wall, controls.wish, planar(velocity), heading(body.yaw)]
        .map(|v| v.normalize_or_zero())
        .into_iter()
        .filter(|v| *v != Vec3::ZERO);
    // The arms go up for an edge before the hands are on it.
    // A body that leapt along a wall looks a quarter second ahead of itself.
    let from = if wall != Vec3::ZERO {
        centre + planar(velocity) * 0.25
    } else {
        centre
    };
    let coming = looks.clone().find_map(|dir| {
        ledge::probe(w, from, dir, 1.0, GRAB_LOW - 0.3, GRAB_HIGH + 0.6)
            .filter(|l| toward(l.inward()))
    });
    let change = dt / 0.15;
    match coming {
        Some(ledge) => {
            body.ahead = ledge;
            body.reach = (body.reach + change).min(1.0);
        }
        None => body.reach = (body.reach - change).max(0.0),
    }
    // A lower wall is not for the hands, but the legs must not go into it.
    let low = looks.clone().find_map(|dir| {
        ledge::probe(w, from, dir, 1.0, VAULT_LOW, GRAB_LOW - 0.3).filter(|l| toward(l.inward()))
    });
    match low {
        Some(ledge) => {
            body.low = ledge;
            body.near = (body.near + change).min(1.0);
        }
        None => body.near = (body.near - change).max(0.0),
    }
    for dir in looks {
        if velocity.y < 3.0 {
            let grab = ledge::probe(w, centre, dir, 0.45, GRAB_LOW, GRAB_HIGH)
                .filter(|l| toward(l.inward()) && ledge::clear(w, l.hang()));
            if let Some(ledge) = grab {
                // Under both hands, not only under the body: near the end of a
                // wall the body hangs a little further along it.
                let ledge = seated(w, ledge);
                release(w);
                let carried = if body.leap > 0.0 {
                    (velocity.dot(ledge.left()) / LEAP_ALONG).clamp(-1.0, 1.0) * smooth(body.leap / 0.3)
                } else {
                    0.0
                };
                let hanging = Hang {
                    reach: body.reach,
                    carried,
                    ledge,
                    off: centre - ledge.hang(),
                    drift: velocity,
                    live: controls.wish.dot(ledge.inward()) < 0.3,
                    pace: velocity.dot(ledge.inward()).max(0.0),
                    swung: velocity.dot(ledge.left()),
                    ..Hang::default()
                };
                body.fall = 0.0;
                body.reach = 0.0;
                let yaw = yaw_of(ledge.inward(), dir);
                return (Action::Hang(hanging), Motion::default(), Some(yaw));
            }
        }
        // A lower edge it pulls straight onto: a jump that fell a little short,
        // or one that is still going up and will not get over the edge.
        {
            let peak = centre.y - HALF + velocity.y.max(0.0) * velocity.y.max(0.0) / (2.0 * GRAVITY);
            let catch = ledge::probe(w, centre, dir, 0.4, VAULT_LOW, GRAB_LOW).filter(|l| {
                l.standable && toward(l.inward()) && (velocity.y < 1.0 || peak < l.edge.y + 0.05)
            });
            if let Some(ledge) = catch {
                release(w);
                body.fall = 0.0;
                // The leg that leads the jump is the one that steps up.
                let climb = Climb {
                    reach: body.reach,
                    ..Climb::join(centre, velocity, &ledge, body.lead)
                };
                body.reach = 0.0;
                let yaw = yaw_of(ledge.inward(), dir);
                return (Action::Climb(climb), climb.motion(), Some(yaw));
            }
        }
    }
    (Action::Air, motion, facing)
}

// The place nearest `ledge` on its edge where both hands of a hanging body have
// the edge under them and the body has room.
fn seated(w: &World, ledge: Ledge) -> Ledge {
    let hands = HANDS + 0.03;
    [0.0, 0.05, -0.05, 0.1, -0.1, 0.15, -0.15, 0.2, -0.2, 0.25, -0.25, 0.3, -0.3]
        .into_iter()
        .filter_map(|aside| ledge::along(w, &ledge, aside))
        .find(|at| {
            ledge::along(w, at, hands).is_some()
                && ledge::along(w, at, -hands).is_some()
                && ledge::clear(w, at.hang())
        })
        .unwrap_or(ledge)
}

// The move a jump at the wall ahead becomes, if there is such a wall.
fn over_wall(w: &World, centre: Vec3, dir: Vec3, velocity: Vec3, first: f32) -> Option<Action> {
    let speed = planar(velocity).length();
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
            }));
        }
    }
    ledge
        .standable
        .then(|| Action::Climb(Climb::join(centre, velocity, &ledge, first)))
}

// The foot that steps first onto a top the body climbs from the ground: the one
// that is not on the ground, by the gait's cycle. +1 the left, -1 the right.
fn step_first(w: &World) -> f32 {
    air::lead(w.get::<Pose>("stickman").map_or(0.0, |pose| pose.phase))
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

fn scripted(path: &Path) -> Motion {
    Motion {
        act: Act::Vault,
        progress: path.progress(),
        height: path.height,
        // A vault leaves at a run, so the stride stays open through it.
        speed: path.exit.length(),
        ..Motion::default()
    }
}

// Fly a vault along its course at its pace, then hand the body back a step
// above the ground, where it lands.
fn vault(w: &mut World, mut path: Path) -> Outcome {
    path.time += w.dt();
    let u = path.progress();
    w.require_mut::<Transform>("player").position = path.at(u);
    let face = Some(yaw_of(path.exit, path.to - path.from));
    if u < 1.0 {
        return (Action::Vault(path), scripted(&path), face);
    }
    let mut c = w.require_mut::<CapsuleController>("player");
    c.velocity = path.exit;
    c.grounded = false;
    (Action::Air, Motion::default(), face)
}

// Go on up a climb, then stand the body on the top, walking in.
fn climbing(w: &mut World, mut climb: Climb) -> Outcome {
    let (at, exit) = climb.step(w.dt());
    w.require_mut::<Transform>("player").position = at;
    let face = Some(yaw_of(climb.ledge.inward(), climb.ledge.inward()));
    let Some(exit) = exit else {
        return (Action::Climb(climb), climb.motion(), face);
    };
    let mut c = w.require_mut::<CapsuleController>("player");
    c.velocity = exit;
    c.grounded = true;
    (Action::Ground, Motion::default(), face)
}

// Hanging: still, along the edge, up and over, off it in a jump, or let go.
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
    let jump = body.buffer > 0.0;
    let side = wish.dot(ledge.left());
    let away = wish.dot(ledge.out) > SECTOR;
    let centre = w.require::<Transform>("player").position;
    // A jump goes where the stick aims. Along the wall it is a leap for the next
    // edge on it: the legs swing back first, and then the body leaves from where
    // it had leaned to.
    if jump && side.abs() > 0.5 && hanging.wind == 0.0 {
        body.buffer = 0.0;
        hanging.wind = WIND_UP;
        hanging.aim = side.signum();
    }
    if hanging.wind > 0.0 {
        hanging.wind -= dt;
    }
    if hanging.aim != 0.0 && hanging.wind <= 0.0 {
        let along = ledge.left() * hanging.aim;
        let from = centre + ledge.left() * (PEEK * hanging.lean);
        if ledge::clear(w, from) {
            w.require_mut::<Transform>("player").position = from;
        }
        body.buffer = 0.0;
        body.regrab = 0.2;
        body.airtime = COYOTE;
        body.wall = ledge.inward();
        body.leap = LEAP;
        w.require_mut::<CapsuleController>("player").velocity =
            along * LEAP_ALONG + Vec3::Y * LEAP_UP + ledge.out * 0.3;
        return (Action::Air, Motion::default(), face);
    }
    // Away from the wall it is a jump off it, backward.
    if jump && away && hanging.aim == 0.0 {
        body.buffer = 0.0;
        body.regrab = REGRAB;
        body.airtime = COYOTE;
        w.require_mut::<CapsuleController>("player").velocity = ledge.out * 3.5 + Vec3::Y * 3.8;
        return (Action::Air, Motion::default(), Some(yaw_of(ledge.out, ledge.out)));
    }
    let winding = hanging.aim != 0.0;
    let up = !winding && (jump || wish.dot(ledge.inward()) > SECTOR);
    let away = away && !winding;
    if up && ledge.standable {
        body.buffer = 0.0;
        let left = 1.0 - smooth(hanging.time / SWUNG_IN);
        let climb = Climb {
            hug: HUG * hanging.settled(),
            travel: hanging.travel,
            lean: hanging.lean,
            ..Climb::from_hang(centre, hanging.drift * left, &ledge, body.lead)
        };
        return (Action::Climb(climb), climb.motion(), face);
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
    let pushing = side.abs() > 0.3 && hanging.time >= SETTLE && !winding;
    let want = if pushing { side.signum() * SHIMMY } else { 0.0 };
    let change = SHIMMY_ACCEL * dt;
    hanging.speed += (want - hanging.speed).clamp(-change, change);
    let mut ended = false;
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
            None => {
                hanging.speed = 0.0;
                ended = true;
            }
        }
    }
    // Where the edge ends and the stick still pushes, the body leans out past
    // the end to look along the wall.
    let lean = if pushing && ended { side.signum() } else { 0.0 };
    let change = PEEK_RATE * dt;
    hanging.lean += (lean - hanging.lean).clamp(-change, change);
    // What is left of how the body came dies away. The wall stops what is left
    // of a run at it: the body goes no further in.
    (hanging.off, hanging.drift) = climb::settle(hanging.off, hanging.drift, CATCH, dt);
    if hanging.off.dot(ledge.inward()) > 0.0 {
        hanging.drift -= ledge.inward() * hanging.drift.dot(ledge.inward()).max(0.0);
    }
    w.require_mut::<Transform>("player").position = hanging.centre();
    let motion = Motion {
        act: Act::Hang,
        ..Motion::default()
    };
    (Action::Hang(hanging), motion, face)
}

// How far from under the capsule the drawn body is: toward the wall it is on,
// and out past the end of an edge it leans from.
fn hug(action: &Action) -> Vec3 {
    let out = |lean: f32| PEEK * lean.signum() * smooth(lean.abs());
    match action {
        Action::Hang(hanging) => {
            hanging.ledge.inward() * (HUG * hanging.settled())
                + hanging.ledge.left() * out(hanging.lean)
        }
        // A climb leaves the wall's face as the chest comes over the edge.
        Action::Climb(climb) => {
            climb.ledge.inward() * (climb.hug * (1.0 - smooth(climb.time / climb::PRESSED)))
                + climb.ledge.left() * out(climb.leaning())
        }
        _ => Vec3::ZERO,
    }
}

// The two points on an edge where the hands of a body that has shimmied `travel`
// metres are. A step of the shimmy moves one hand and then the other: the hand on
// the side the body is going to moves first.
//
// A body that leans out past the end of the edge hangs from the hand on that
// side, and the other lets go.
fn grips(ledge: &Ledge, travel: f32, lean: f32) -> [Hold; 2] {
    let over = ledge.edge + ledge.inward() * 0.02 + Vec3::Y * hold::WRIST;
    let cycle = travel / HAND_STEP;
    let hand = |limb: Limb, side: f32, cycle: f32| {
        let whole = math::floor(cycle);
        // The first half of a cycle carries the hand; the second leaves it planted.
        let swing = ((cycle - whole) * 2.0).min(1.0);
        let along = side * HANDS + HAND_STEP * (whole + smooth(swing)) - travel;
        let lift = 0.04 * math::sin(std::f32::consts::PI * swing);
        let lets_go = (-side * lean).max(0.0);
        Hold::new(limb, over + ledge.left() * along + Vec3::Y * lift)
            .grip(1.0 - smooth(lets_go / 0.6))
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
        // The hands close on the edge over the first moment of the catch.
        // Until the body has swung in under it, an edge can be further than
        // an arm is long: the hand is then as far toward it as the arm goes.
        Action::Hang(hanging) => grips(&hanging.ledge, hanging.travel, hanging.lean)
            .map(|hand| {
                // They close from as far as the arms had gone up to it.
                let reached = smooth(hanging.reach);
                let closed = reached + (1.0 - reached) * smooth(hanging.time / 0.1);
                let shoulder = hold::joint(w, hand.limb.chain()[0]);
                let to = hand.at - shoulder;
                let at = if hanging.time < SWUNG_IN && to.length() > ARM {
                    shoulder + to.normalize() * ARM
                } else {
                    hand.at
                };
                Hold { at, ..hand }.grip(hand.grip * closed)
            })
            .to_vec(),
        Action::Air => {
            let body = w.require::<Parkour>("player");
            let mut holds = vec![];
            // With an edge coming, each arm goes up toward where its hand will
            // be, as far as it reaches.
            if body.reach > 0.0 {
                holds.extend(grips(&body.ahead, 0.0, 0.0).map(|hand| {
                    let shoulder = hold::joint(w, hand.limb.chain()[0]);
                    let to = hand.at - shoulder;
                    let at = shoulder + to.normalize_or_zero() * to.length().min(ARM);
                    Hold::new(hand.limb, at).grip(smooth(body.reach))
                }));
            }
            // With the ground coming, the leading leg opens toward the place its
            // foot will strike, and has it as the body arrives. From a stand
            // both feet come down together.
            if body.brace > 0.0 {
                let speed = planar(w.require::<CapsuleController>("player").velocity).length();
                let feet = centre - Vec3::Y * HALF;
                let (facing, left) = (heading(body.yaw), Vec3::Y.cross(heading(body.yaw)));
                let lead = if body.lead < 0.0 { 1 } else { 0 };
                let strides = crate::gait::planted(speed, air::strike(body.lead));
                let both = 1.0 - smooth(speed / 1.5);
                for (leg, limb, side) in [(0, Limb::FootL, 1.0), (1, Limb::FootR, -1.0)] {
                    let grip = if leg == lead { 1.0 } else { both };
                    if grip == 0.0 {
                        continue;
                    }
                    let ahead = strides[leg].map_or(0.0, |(ahead, _)| ahead);
                    let at = feet + facing * ahead + left * (0.09 * side);
                    let floor = ledge::ground(w, at + Vec3::Y * 0.5, 2.5).unwrap_or(feet.y);
                    holds.push(
                        Hold::new(limb, Vec3::new(at.x, floor + hold::ANKLE, at.z))
                            .grip(grip * smooth(body.brace / 0.5))
                            .slack(0.45),
                    );
                }
            }
            holds
        }
        // A slide rides on the tucked foot and a hand behind, and the other
        // heel skims the ground ahead. Each is on the ground where it is, so a
        // slide down a slope or over a step still touches it. Beside a wall it
        // does not go up onto the wall.
        Action::Slide(slide) => {
            let u = slide.phase;
            let feet = centre - Vec3::Y * HALF;
            let left = Vec3::Y.cross(slide.heading);
            let on = |ahead: f32, side: f32, over: f32| {
                let at = feet + slide.heading * ahead + left * side;
                Vec3::new(at.x, slide.floor(w, feet, at) + over, at.z)
            };
            // The lead leg kicks out first, then the tucked foot is down, then
            // the hand. Out of the slide the body rises over the tucked foot,
            // which stays where it is on the ground while the body passes it.
            let rise = (u - 0.78).max(0.0) * SLIDE;
            let tucked = (0.34 - slide.speed * rise).max(-0.35);
            let long = 0.8 - 0.1 * smooth((u - 0.3) / 0.45);
            let between = |from: f32, full: f32, fade: f32, gone: f32| {
                smooth((u - from) / (full - from)) * (1.0 - smooth((u - fade) / (gone - fade)))
            };
            // The body leans into a turn, and its feet go to the outside of it.
            // The hand on the side it leans to is on the ground: the left one
            // also when it goes straight. As the weight goes over, one hand
            // comes off the ground and the other goes down.
            let lean = slide.lean * slide.riding();
            let (to_left, to_right) = (lean.max(0.0), (-lean).max(0.0));
            let down = between(0.1, 0.3, 0.7, 0.86);
            vec![
                // Each of these eases in and out by its own grip.
                Hold::new(Limb::FootR, on(tucked, -0.12 - 0.06 * lean, hold::ANKLE))
                    .grip(between(0.0, 0.16, 0.9, 1.0))
                    .firm(),
                Hold::new(Limb::FootL, on(long, 0.1 - 0.14 * lean, 0.15))
                    .grip(between(0.12, 0.3, 0.76, 0.92))
                    .loose()
                    .firm(),
                Hold::new(Limb::HandL, on(-0.46, 0.4 + 0.1 * to_left, hold::WRIST))
                    .grip(down * (1.0 - smooth(to_right / 0.8)))
                    .slack(0.6)
                    .bend(Vec3::new(0.5, 0.1, -0.6))
                    .firm(),
                Hold::new(Limb::HandR, on(-0.42, -0.42, hold::WRIST))
                    .grip(down * smooth(to_right / 0.8))
                    .slack(0.6)
                    .bend(Vec3::new(-0.5, 0.1, -0.6))
                    .firm(),
            ]
        }
        // The hand starts for the wall's top well before the shoulder is over it.
        Action::Vault(path) if path.held => vec![Hold::new(Limb::HandL, path.hand).slack(0.9).firm()],
        Action::Climb(climb) => {
            let ledge = &climb.ledge;
            let (t, joined) = (climb.time, climb.joined);
            let unfacing = Quat::from_rotation_y(-w.require::<Parkour>("player").yaw);
            // Where the feet will stand at the end of the climb, and the top's
            // level at the edge.
            let stand = climb::course(ledge, climb::SPAN) - Vec3::Y * (HALF + 0.01);
            let top = Vec3::new(ledge.edge.x, stand.y, ledge.edge.z);
            // A foot comes up beside the wall's face with its knee out to its
            // own side, swings over the edge in one arc and steps in onto the
            // top, as on a high stair. It goes from `from` to `to` seconds of
            // the course, and `onto` is where on the top it steps, in metres
            // from the edge. Before the swing the leg is where the pose has it.
            // The hold takes the foot whole when the swing starts, and the foot
            // catches up with it from where it was.
            // `under` is how far below the edge it starts from, and `aside` how
            // far out to its side the knee goes on the way.
            // A leg that `trails` hangs behind the body when its swing starts,
            // as the second leg of a climb out of a hang does, and its knee
            // comes round from there.
            let step = |side: f32, from: f32, to: f32, onto: f32, under: f32, aside: f32, trails: bool| {
                let q = ((t - from) / (to - from)).clamp(0.0, 1.0);
                let beside = ledge.left() * (side * 0.09);
                // Far enough out that the toes clear the face on the way up.
                let low = ledge.edge + ledge.out * 0.32 + beside - Vec3::Y * under;
                let over = ledge.edge + ledge.out * 0.2 + beside + Vec3::Y * 0.5;
                let down = top + ledge.inward() * onto + beside + Vec3::Y * hold::ANKLE;
                let s = smooth(q);
                let at = low * ((1.0 - s) * (1.0 - s)) + over * (2.0 * s * (1.0 - s)) + down * (s * s);
                // The knee comes up at its own side before it goes in.
                let up = unfacing * (ledge.left() * (side * aside) + ledge.inward() * (0.5 - aside))
                    + Vec3::Y * 0.7;
                let inside = unfacing * (ledge.left() * (side * 0.2) + ledge.inward() * 0.4)
                    + Vec3::Y * 0.8;
                // The toes point down while the foot comes up beside the wall,
                // and level as it comes over the edge.
                let behind = unfacing * (ledge.out * 0.5 + ledge.left() * (side * 0.15)) - Vec3::Y * 0.1;
                let up = if trails { behind.lerp(up, smooth(q / 0.45)) } else { up };
                let grip = if q > 0.0 { 1.0 } else { 0.0 };
                (at, grip, smooth((q - 0.55) / 0.3), up.lerp(inside, smooth((q - 0.4) / 0.6)))
            };
            // The first foot steps onto the top by the edge and takes the
            // weight. The second steps past it to where the body will stand,
            // and the first then closes up beside it. A body that joins the
            // course late has less time for each.
            let behind = 0.15;
            let ahead = (stand - top).dot(ledge.inward()) - 0.03;
            let from = joined.max(0.4);
            let to = (from + 0.22).max(0.72);
            // The second knee must be over the top before the hips are.
            let next = (from + 0.06).max(0.68);
            let last = (next + 0.22).max(0.94).min(climb::SPAN - 0.02);
            let (first, grip, flat, knee) = step(climb.first, from, to, behind, 0.5, 0.5, false);
            let closes = smooth((t - last) / (climb::SPAN - last));
            let closed = first
                + ledge.inward() * ((ahead - behind) * closes)
                + Vec3::Y * (0.06 * math::sin(std::f32::consts::PI * closes));
            // The hips are over the top by then: the second knee comes
            // through in front, from where the leg hangs.
            let (second, grip_2, flat_2, knee_2) = step(-climb.first, next, last, ahead, 0.2, 0.3, joined == 0.0);
            let (one, two) = if climb.first > 0.0 {
                (Limb::FootL, Limb::FootR)
            } else {
                (Limb::FootR, Limb::FootL)
            };
            let feet = [
                Hold::new(one, closed).grip(grip).flat(flat).bend(knee),
                Hold::new(two, second).grip(grip_2).flat(flat_2).bend(knee_2),
            ];
            // The hands hold the edge, roll in onto the top as the chest comes
            // over with the elbows coming up over them, press, and let go when
            // the feet have the weight. A body that joins from the ground or
            // the air puts them down as it arrives, and a top lower than the
            // hips needs no hands.
            let onto = ledge.inward() * (0.1 * smooth((t - 0.34) / 0.2));
            let press = smooth((t - climb::PULLED) / 0.25);
            // They go on from as far as they had gone up to the edge in the air.
            let reached = smooth(climb.reach);
            let arrives = if joined > 0.0 {
                reached + (1.0 - reached) * smooth((t - joined) / 0.14)
            } else {
                1.0
            };
            // Once on the edge they ease off by their own grip. Until then a
            // hand that was somewhere else catches up.
            let on = joined == 0.0 || t - joined > 0.25;
            let needed = smooth((climb.height - 0.6) / 0.25);
            let lets_go = 1.0 - smooth((t - 0.7) / 0.2);
            let mut holds: Vec<Hold> = grips(ledge, climb.travel, climb.leaning())
                .into_iter()
                .map(|hand| {
                    let side = if hand.limb == Limb::HandL { 1.0 } else { -1.0 };
                    let elbow = Vec3::new(side * 0.35, 0.0, -0.5)
                        .lerp(Vec3::new(side * 0.3, 0.3, -0.25), press);
                    let hold = Hold::new(hand.limb, hand.at + onto)
                        .grip(hand.grip * arrives * needed * lets_go)
                        .bend(elbow);
                    if on {
                        hold.firm()
                    } else {
                        hold
                    }
                })
                .collect();
            holds.extend(feet);
            holds
        }
        _ => vec![],
    }
}

// What a touchdown at `impact` m/s downward and `speed` m/s over the ground becomes.
fn touchdown(w: &mut World, body: &mut Parkour, impact: f32, speed: f32) -> Action {
    // The knees take it, or the body kneels into it and runs on.
    if impact < ROLL {
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
    // How far into the band between a landing the legs take and a fall, 0 to 1.
    let strain = ((impact - ROLL) / (HARD - ROLL)).clamp(0.0, 1.0);
    let velocity = planar(w.require::<CapsuleController>("player").velocity);
    if speed >= ROLL_SPEED {
        Action::Roll {
            time: 0.0,
            span: 0.75 + 0.35 * strain,
            heading: velocity.normalize(),
            speed: velocity.length(),
            strain,
        }
    } else {
        Action::Land {
            time: 0.0,
            span: 0.45 + 0.65 * strain,
        }
    }
}

// Rolling: the heading is fixed, and the harder the landing, the more of the
// speed the roll costs.
fn roll(w: &mut World, time: f32, span: f32, heading: Vec3, speed: f32, strain: f32) -> Outcome {
    let u = (time / span).clamp(0.0, 1.0);
    let out = math::lerp(speed.min(RUN), speed.min(2.0), strain);
    let pace = math::lerp(speed.max(2.5), out, smooth(u));
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
        // A roll comes up through a kneel, deeper after a harder landing. The
        // kneel starts at its deepest as the ball opens, and rises with it.
        kneel: if (time - w.dt()) / span < 0.6 && u >= 0.6 {
            0.7 + 0.3 * strain
        } else {
            0.0
        },
        kneel_from: 0.22,
        cut: (time - w.dt()) / span < 0.6 && u >= 0.6,
        ..Motion::default()
    };
    let next = if u < 1.0 {
        Action::Roll {
            time,
            span,
            heading,
            speed,
            strain,
        }
    } else if moved.grounded {
        Action::Ground
    } else {
        Action::Air
    };
    (next, motion, Some(yaw_of(heading, heading)))
}

// Sliding: the body goes on the way it was going. The ground takes a share of
// its speed each second, and a slope gives it speed or takes it as gravity does,
// along the slope's own fall. The stick leans the body, and the lean turns the
// slide. A jump ends it.
fn slide(w: &mut World, body: &mut Parkour, controls: &Controls, mut slide: Slide) -> Outcome {
    let dt = w.dt();
    // How far off the heading the stick points; to the left is positive.
    let aim = if controls.wish.length() > 0.3 {
        let (to, from) = (controls.wish, slide.heading);
        wrap_angle(math::atan2(to.x, to.z) - math::atan2(from.x, from.z))
    } else {
        0.0
    };
    // The weight goes over to that side as fast as a body's can. The slide
    // turns half with the stick and half with the lean, so it answers at once
    // and turns hardest when the weight is over, but not past where the stick
    // points.
    let want = (aim / SLIDE_AIM).clamp(-1.0, 1.0);
    let pull = (want - slide.lean) * LEAN * LEAN - slide.tip * 2.0 * LEAN;
    slide.tip = (slide.tip + pull * dt).clamp(-LEAN_MOST, LEAN_MOST);
    slide.lean = (slide.lean + slide.tip * dt).clamp(-1.0, 1.0);
    let turn = 0.5 * (want + slide.lean) * SLIDE_TURN * dt;
    let turn = if turn * aim > 0.0 {
        turn.clamp(-aim.abs(), aim.abs())
    } else {
        turn
    };
    // On a slope, the part of gravity that lies along the ground.
    let under = feet(w.require::<Transform>("player").position);
    let normal = ledge::slope(w, under + Vec3::Y * 0.3, 0.8).unwrap_or(Vec3::Y);
    let fall = planar(normal) * (GRAVITY * normal.y);
    let going = Quat::from_rotation_y(turn) * slide.heading * slide.speed + fall * dt;
    let going = (going * math::exp(-SLIDE_DRAG * dt)).clamp_length_max(SLIDE_TOP);
    let velocity = {
        let mut c = w.require_mut::<CapsuleController>("player");
        c.velocity = Vec3::new(going.x, c.velocity.y, going.z);
        Gravity(GRAVITY).step(&mut c.velocity, dt);
        c.velocity
    };
    let moved = physics::capsule(w, "player").step(velocity);
    let went = planar(moved.displacement).length() / dt;
    slide.speed = going.length();
    slide.heading = going.normalize_or(slide.heading);
    slide.ground = slide.ground.lerp(normal, 1.0 - math::exp(-12.0 * dt)).normalize_or(Vec3::Y);
    // A slope that pulls the body on keeps it down: the ride lasts as long as
    // the slope does. A body that has begun to come up comes up.
    let pulled = moved.grounded && fall.dot(slide.heading) >= SLIDE_PULL;
    let next = slide.phase + dt / SLIDE;
    slide.phase = if pulled && slide.phase <= SLIDE_RIDE {
        next.min(SLIDE_RIDE)
    } else {
        next
    };
    let motion = Motion {
        act: Act::Slide,
        progress: slide.phase.min(1.0),
        speed: went,
        rising: velocity.y,
        grounded: moved.grounded,
        lean: slide.lean,
        ..Motion::default()
    };
    let face = Some(yaw_of(slide.heading, slide.heading));
    // Off an edge, over, stopped by something, or cut short by a jump, which
    // the ground takes. What the slide gave the body above the speed of its
    // legs, it leaves with.
    let over = slide.phase >= 1.0 || went < 1.0 || body.buffer > 0.0;
    if !moved.grounded || over {
        let legs = if controls.sprint { SPRINT } else { RUN };
        body.momentum = if went > legs { went } else { 0.0 };
        let next = if moved.grounded { Action::Ground } else { Action::Air };
        return (next, motion, face);
    }
    (Action::Slide(slide), motion, face)
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

// What hangs from the hands moves. The whole body swings from the catch, harder
// the faster it came, and the knees fold as the feet come to the wall. In a
// shimmy the hips roll from hand to hand and the legs follow, one after the
// other. Before a leap the legs swing back. At the end of an edge the body
// rolls about the shoulder of the hand it hangs from, so the head goes out past
// the end and the legs go the other way; the head turns to look, and the arm
// that let go hangs out for balance.
fn sway(w: &mut World, hanging: &Hang) {
    let t = hanging.time;
    let swing = (0.2 + 0.07 * hanging.pace).min(0.55) * math::exp(-2.2 * t) * math::cos(7.0 * t)
        + 0.03 * math::sin(3.1 * t);
    let fold = swing.min(0.0);
    let k = hanging.speed / SHIMMY;
    let step = std::f32::consts::TAU * hanging.travel / HAND_STEP;
    let wind = if hanging.aim != 0.0 {
        hanging.aim * smooth(1.0 - hanging.wind.max(0.0) / WIND_UP)
    } else {
        0.0
    };
    let along = 0.11 * hanging.swung * math::exp(-2.5 * t) * math::sin(6.0 * t);
    // What is left of the lean a leap arrived with.
    let carried = hanging.carried * (1.0 - smooth(t / 0.3));
    let aside = -0.2 * k - 0.4 * wind + along - 0.25 * carried;
    let lean = hanging.lean;
    let roll = 0.4 * lean * (1.0 + 0.06 * math::sin(1.8 * t))
        + 0.05 * k * math::sin(step)
        + 0.3 * carried;
    if roll != 0.0 {
        // About the shoulder of the hand on that side, which stays where it is.
        let shoulder = Vec3::new(0.19 * roll.signum(), 0.48, 0.0);
        let turn = Quat::from_rotation_z(-roll);
        hold::shift(w, "hips", shoulder - turn * shoulder);
        hold::turn(w, "hips", turn);
    }
    hold::turn(w, "hips", Quat::from_rotation_x(0.35 * swing));
    for (leg, late, share) in [("l", 0.0, 0.6), ("r", 1.2, 1.2)] {
        let own = aside * share + 0.08 * k * math::sin(step - late);
        let thigh = Quat::from_rotation_z(own)
            * Quat::from_rotation_x(swing.max(0.0) + 1.3 * fold);
        hold::turn(w, &format!("thigh_{leg}"), thigh);
        let knee = -2.4 * fold + 0.25 * k.abs() * share + 0.4 * wind.abs();
        hold::turn(w, &format!("shin_{leg}"), Quat::from_rotation_x(knee));
    }
    if lean != 0.0 {
        hold::turn(
            w,
            "neck",
            Quat::from_rotation_z(-0.2 * lean) * Quat::from_rotation_y(0.7 * lean),
        );
        let side = if lean > 0.0 { "r" } else { "l" };
        hold::turn(w, &format!("upper_arm_{side}"), Quat::from_rotation_z(1.9 * lean));
        hold::turn(w, &format!("forearm_{side}"), Quat::from_rotation_x(-0.5 * lean.abs()));
    }
}

// A sliding body lies on the ground it slides on: it tips with the slope, about
// its hips, which are next to the ground. Faster than a sprint it lies further
// back.
fn recline(w: &mut World, slide: &Slide, yaw: f32) {
    let ground = Quat::from_rotation_y(-yaw) * slide.ground;
    let tip = Quat::from_rotation_arc(Vec3::Y, ground.normalize_or(Vec3::Y));
    let back = 0.25 * smooth((slide.speed - SPRINT) / (SLIDE_TOP - SPRINT));
    let lie = Quat::IDENTITY.slerp(tip * Quat::from_rotation_x(-back), slide.riding());
    hold::turn(w, "hips", lie);
}

// A body on its feet puts its weight where it is pushed. The hips go that way
// and sink, the trunk leans on over them, and the head stays up and looks into
// a turn. Returns how far the hips went, metres.
fn lean(w: &mut World, body: &Parkour) -> f32 {
    if body.look != 0.0 {
        hold::turn(w, "neck", Quat::from_rotation_y(body.look));
    }
    let angle = body.tilt.length();
    if angle < 1e-4 {
        return 0.0;
    }
    let toward = body.tilt / angle;
    let way = Quat::from_rotation_y(-body.yaw) * toward;
    let height = hold::joint(w, "hips").y - feet(w.require::<Transform>("player").position).y;
    let out = way * (height * math::sin(angle) * HIPS_OUT) - Vec3::Y * (HIPS_SINK * angle);
    hold::shift(w, "hips", out);
    hold::turn(w, "hips", Quat::from_axis_angle(Vec3::Y.cross(way), angle * TRUNK[0]));
    let axis = Vec3::Y.cross(toward);
    hold::turn_in_world(w, "spine", Quat::from_axis_angle(axis, angle * TRUNK[1]));
    hold::turn_in_world(w, "chest", Quat::from_axis_angle(axis, angle * TRUNK[2]));
    hold::turn_in_world(w, "neck", Quat::from_axis_angle(axis, -angle * NECK_BACK));
    out.length()
}

// The lean of a body that is not on a wall, laid over the pose its move and its
// holds made. On the ground the feet stay where the gait has them, so the legs
// reach from the hips to the ground: a body that speeds up drives off a leg that
// is behind it, one that brakes lands on a leg that is out in front, and one
// that turns stands on the leg that is on the outside. In another move the lean
// dies away, and the hands stay where that move has them too.
fn balance(w: &mut World, body: &Parkour) {
    if body.tilt == Vec3::ZERO && body.look == 0.0 {
        return;
    }
    let kept: &[Limb] = match body.action {
        Action::Ground => &[Limb::FootL, Limb::FootR],
        _ => &Limb::ALL,
    };
    let placed = hold::placed(w, kept);
    let out = lean(w, body);
    hold::replace(w, &placed, out);
}

// A body that has leapt along a wall leans the way it goes, and its legs trail.
fn lunge(w: &mut World, body: &Parkour) {
    let along = w.require::<CapsuleController>("player").velocity.dot(Vec3::Y.cross(body.wall));
    let lean = (along / LEAP_ALONG).clamp(-1.0, 1.0) * smooth(body.leap / 0.3);
    hold::turn(w, "hips", Quat::from_rotation_z(-0.3 * lean));
    hold::turn(w, "thigh_l", Quat::from_rotation_z(-0.35 * lean));
    hold::turn(w, "thigh_r", Quat::from_rotation_z(-0.15 * lean));
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

// Where a foot the pose has too low must be lifted to. `need` is the height a
// point of the body must have to be `over` metres clear of what is under it.
fn lifted(w: &World, need: &dyn Fn(Vec3, f32) -> f32) -> Vec<Hold> {
    // From the ankle to the tip of the toes, in the foot's frame.
    const TOE: Vec3 = Vec3::new(0.0, -0.06, 0.19);
    [(Limb::FootL, "foot_l"), (Limb::FootR, "foot_r")]
        .into_iter()
        .filter_map(|(limb, name)| {
            let ankle = hold::joint(w, name);
            let toe = hold::point(w, name, TOE);
            let short = (need(ankle, hold::ANKLE + 0.04) - ankle.y).max(need(toe, 0.05) - toe.y);
            // A foot that hangs toes down over what it must clear also levels.
            (short > 0.0)
                .then(|| Hold::new(limb, ankle + Vec3::Y * short).flat(smooth(short / 0.06)))
        })
        .collect()
}

// How far, and which way, a point of a leg must move to be out of a wall: `clear`
// metres out from its face, or `over` metres above its top, whichever is nearer.
// Nothing for a point that is already one of the two.
//
// The way out changes from the face to the top without a step. A point deep in
// the wall but just under its top goes up a little, and not out a long way, so
// nothing moves at once when it comes over the edge.
fn out_of_wall(ledge: &Ledge, at: Vec3, clear: f32, over: f32) -> Vec3 {
    let out = clear - (at - ledge.edge).dot(ledge.out);
    let up = ledge.edge.y + over - at.y;
    if out <= 0.0 || up <= 0.0 {
        return Vec3::ZERO;
    }
    let (out4, up4) = (out * out * out * out, up * up * up * up);
    let way = ledge.out * (out * up4 / (out4 + up4)) + Vec3::Y * (up * out4 / (out4 + up4));
    // It starts from nothing with no step in its rate either, so a point that
    // comes to the wall is slowed and not stopped.
    let near = out.min(up);
    way * (near / (near + SOFT_WALL))
}

/// The depth into a wall's clearance over which a leg is slowed, metres.
const SOFT_WALL: f32 = 0.03;

// Turn a thigh whose knee the pose has in the wall until the knee is out of it.
fn knees_out(w: &mut World, ledge: &Ledge, strength: f32) {
    for _ in 0..2 {
        for side in ["l", "r"] {
            let hip = hold::joint(w, &format!("thigh_{side}"));
            let knee = hold::joint(w, &format!("shin_{side}"));
            let way = out_of_wall(ledge, knee, 0.09, 0.04) * strength;
            let axis = (knee - hip).cross(way);
            if axis.length() > 1e-6 {
                let turn = Quat::from_axis_angle(axis.normalize(), (way.length() / 0.43).min(0.8));
                hold::turn_in_world(w, &format!("thigh_{side}"), turn);
            }
        }
    }
}

// Where a foot the pose has in the wall must be moved to.
fn kept_out(w: &World, ledge: &Ledge, strength: f32) -> Vec<Hold> {
    const TOE: Vec3 = Vec3::new(0.0, -0.06, 0.19);
    [(Limb::FootL, "foot_l"), (Limb::FootR, "foot_r")]
        .into_iter()
        .filter_map(|(limb, name)| {
            let ankle = hold::joint(w, name);
            let heel = out_of_wall(ledge, ankle, 0.1, 0.07);
            let toe = out_of_wall(ledge, hold::point(w, name, TOE), 0.04, 0.03);
            let way = if toe.length() > heel.length() { toe } else { heel } * strength;
            (way.length() > 1e-4).then(|| Hold::new(limb, ankle + way).loose())
        })
        .collect()
}

// How fast the pose of the move before dies away in each move, per second. A
// move that must have a hand or a foot somewhere at once takes over sooner.
fn takes_over(act: Act) -> f32 {
    match act {
        Act::Move => 18.0,
        Act::Air => 13.0,
        Act::Hang | Act::Climb => 14.0,
        Act::Slide => 16.0,
        Act::Vault | Act::Roll => 22.0,
        Act::Land | Act::Rise => 24.0,
    }
}

/// Lay the ragdoll's pose over the Animator's, or carry the last move's pose into
/// this one and put the hands and feet on what they hold. Call after
/// `animation::step`.
pub fn present(w: &mut World, motion: Motion) {
    let body = w.require::<Parkour>("player").clone();
    let (action, yaw) = (body.action.clone(), body.yaw);
    let act = motion.act;
    let weight = match action {
        Action::Ragdoll => 1.0,
        // The limbs gather from where they lay into the first of the rise.
        Action::Rise { time, .. } if time < GATHER => 1.0 - smooth(time / GATHER),
        _ => {
            // The trunk and the free limbs first, then the holds, which reach
            // from wherever that left the shoulders and hips.
            // The air's clips lead with the left leg, and the climb's step
            // up with the right foot first.
            let mirrored = match &action {
                Action::Air => body.lead < 0.0,
                Action::Climb(climb) => climb.first > 0.0,
                _ => false,
            };
            if mirrored {
                air::mirror(w);
            }
            match &action {
                Action::Hang(hanging) => sway(w, hanging),
                Action::Air if body.leap > 0.0 => lunge(w, &body),
                Action::Slide(slide) => recline(w, slide, yaw),
                _ => {}
            }
            let was = w.get::<Flow>("stickman").map_or(act.code(), |flow| flow.act);
            let cut = was != act.code() || motion.cut;
            flow::carry(w, act.code(), cut.then(|| takes_over(act)));
            // The wall the body is on, or is about to be on, and how much the
            // legs keep out of it: all the way on it, and more and more as it
            // comes.
            let wall = match &action {
                Action::Hang(hanging) => Some((hanging.ledge, 1.0)),
                Action::Climb(climb) => Some((climb.ledge, 1.0)),
                Action::Air if body.reach > 0.0 => Some((body.ahead, smooth(body.reach))),
                Action::Air if body.near > 0.0 => Some((body.low, smooth(body.near))),
                _ => None,
            };
            // On a wall what is left of a lean moves the trunk only, and the
            // holds reach from there.
            let on_wall = matches!(action, Action::Vault(_) | Action::Climb(_) | Action::Hang(_));
            if on_wall {
                lean(w, &body);
            }
            if let Some((wall, strength)) = &wall {
                knees_out(w, wall, *strength);
            }
            let holds = holds(w);
            hold::apply(w, &holds);
            // The feet go over a wall the body vaults, and stay out of the ground
            // it slides on. Lifting an ankle turns its foot, which can put the
            // toes low again, so a second pass lifts what the first left short.
            for _ in 0..2 {
                let floor = |at: Vec3, over: f32| {
                    ledge::ground(w, at + Vec3::Y * 0.6, 1.5).map_or(f32::MIN, |y| y + over - 0.04)
                };
                let lifted = match &action {
                    Action::Vault(path) => lifted(w, &|at, over| {
                        clearance(&path.ledge, at, over).max(floor(at, over))
                    }),
                    Action::Slide(slide) => {
                        let feet = feet(w.require::<Transform>("player").position);
                        lifted(w, &|at, over| slide.floor(w, feet, at) + over - 0.04)
                    }
                    // On a wall the feet stay out of its face.
                    _ => match &wall {
                        Some((wall, strength)) => kept_out(w, wall, *strength),
                        None => break,
                    },
                };
                hold::correct(w, &lifted);
            }
            if !on_wall {
                balance(w, &body);
            }
            return;
        }
    };
    flow::settle(w, act.code());
    let doll = w.require::<Ragdoll>("player").clone();
    if !doll.active() {
        return;
    }
    let root = w.require::<Transform>("player").position + w.require::<Transform>("stickman").position;
    let (hips, bends) = ragdoll::pose(&doll, root, yaw);
    ragdoll::show(w, hips, &bends, weight);
}
