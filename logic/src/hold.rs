//! What the hands and feet hold on to.
//!
//! A move that uses a wall knows where its edge is. Each tick the body names, for
//! each limb that touches the wall, the point in the world that limb holds. After
//! the Animator has posed the body, `apply` turns that arm or leg with the engine's
//! two-bone solver so its wrist or ankle is on the point.
//!
//! A limb holds its point only while it can reach it. Past its reach it lets go over
//! the next `SLACK` metres, so a hand comes off an edge because the shoulder left it
//! behind, and goes down on a wall because the shoulder arrived.
use crate::stickman::MODEL;
use exact_game::animation::{joint_matrix, solve_ik};
use exact_game::asset::Model;
use exact_game::*;

/// Height of a wrist above the surface its palm is on, and of an ankle above the
/// surface its sole is on, metres.
pub const WRIST: f32 = 0.04;
pub const ANKLE: f32 = 0.1;
/// Distance past its reach over which a limb lets go, metres, unless its hold
/// says another.
const SLACK: f32 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limb {
    HandL,
    HandR,
    FootL,
    FootR,
}

impl Limb {
    pub const ALL: [Limb; 4] = [Limb::HandL, Limb::HandR, Limb::FootL, Limb::FootR];

    /// The limb's three joints, from the body out; the last is the one that holds.
    pub fn chain(self) -> [&'static str; 3] {
        match self {
            Limb::HandL => ["upper_arm_l", "forearm_l", "hand_l"],
            Limb::HandR => ["upper_arm_r", "forearm_r", "hand_r"],
            Limb::FootL => ["thigh_l", "shin_l", "foot_l"],
            Limb::FootR => ["thigh_r", "shin_r", "foot_r"],
        }
    }
    fn is_foot(self) -> bool {
        matches!(self, Limb::FootL | Limb::FootR)
    }
    // +1 on the body's left, -1 on its right.
    fn side(self) -> f32 {
        match self {
            Limb::HandL | Limb::FootL => 1.0,
            Limb::HandR | Limb::FootR => -1.0,
        }
    }
    // Where the middle joint bends to, from the middle of the limb, in the body's
    // frame: an elbow goes out and back, a knee forward and up.
    fn bend(self) -> Vec3 {
        if self.is_foot() {
            Vec3::new(self.side() * 0.1, 0.3, 0.8)
        } else {
            Vec3::new(self.side() * 0.35, 0.0, -0.5)
        }
    }
}

/// One limb and the point in the world its wrist or ankle is held at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hold {
    pub limb: Limb,
    pub at: Vec3,
    /// How firmly, 0 to 1: a move eases a hold in and out with this.
    pub grip: f32,
    /// How much a foot is flat on what it holds, 0 to 1: all of it when it
    /// stands there, none when it only passes.
    pub flat: f32,
    /// Where the elbow or knee bends to, from the middle of the limb, in the
    /// body's frame, if not the limb's usual way.
    pub bend: Option<Vec3>,
    /// Distance past its reach over which the limb lets go, metres. Within it
    /// the limb goes part of the way to the hold.
    pub slack: f32,
    /// Whether the limb is put where the hold has it with no catching up: for
    /// a hold whose own reach already brings the limb to it in time.
    pub firm: bool,
}
impl Hold {
    pub fn new(limb: Limb, at: Vec3) -> Self {
        Hold {
            limb,
            at,
            grip: 1.0,
            flat: if limb.is_foot() { 1.0 } else { 0.0 },
            bend: None,
            slack: SLACK,
            firm: false,
        }
    }
    pub fn firm(mut self) -> Self {
        self.firm = true;
        self
    }
    pub fn slack(mut self, slack: f32) -> Self {
        self.slack = slack;
        self
    }
    pub fn bend(mut self, bend: Vec3) -> Self {
        self.bend = Some(bend);
        self
    }
    pub fn grip(mut self, grip: f32) -> Self {
        self.grip = grip;
        self
    }
    pub fn loose(self) -> Self {
        self.flat(0.0)
    }
    pub fn flat(mut self, flat: f32) -> Self {
        self.flat = flat.clamp(0.0, 1.0);
        self
    }
}

fn node(model: &Model, name: &str) -> u32 {
    model
        .nodes
        .iter()
        .position(|n| n.name == name)
        .unwrap_or_else(|| panic!("hold: no joint `{name}`")) as u32
}

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

// The drawn body's place and turn in the world.
fn frame(w: &World) -> (Vec3, Quat) {
    let child = *w.require::<Transform>("stickman");
    (
        w.require::<Transform>("player").position + child.position,
        child.rotation,
    )
}

/// Where the joint `name` of the drawn body is in the world.
pub fn joint(w: &World, name: &str) -> Vec3 {
    point(w, name, Vec3::ZERO)
}

/// Where a point `offset` from the joint `name`, in that joint's own frame, is in
/// the world.
pub fn point(w: &World, name: &str, offset: Vec3) -> Vec3 {
    let (root, turn) = frame(w);
    let model = w.model(MODEL).expect("the stick man's model is generated in setup");
    let pose = w.require::<Pose>("stickman");
    let local = joint_matrix(model, &pose.local, node(model, name));
    root + turn * local.transform_point3(offset)
}

/// Turn the joint `name` of the drawn body by `by`, about its parent's axes.
pub fn turn(w: &mut World, name: &str, by: Quat) {
    let slot = {
        let model = w.model(MODEL).expect("the stick man's model is generated in setup");
        node(model, name) as usize * 10 + 3
    };
    let Some(mut pose) = w.get_mut::<Pose>("stickman") else {
        return;
    };
    if pose.local.len() < slot + 4 {
        return;
    }
    let turned = (by * Quat::from_slice(&pose.local[slot..slot + 4])).normalize();
    pose.local[slot..slot + 4].copy_from_slice(&turned.to_array());
}

/// Turn the joint `name` of the drawn body by `by`, about the world's axes.
pub fn turn_in_world(w: &mut World, name: &str, by: Quat) {
    let (_, facing) = frame(w);
    let parent = {
        let model = w.model(MODEL).expect("the stick man's model is generated in setup");
        let Some(pose) = w.get::<Pose>("stickman") else {
            return;
        };
        match model.nodes[node(model, name) as usize].parent {
            Some(parent) if !pose.local.is_empty() => {
                joint_matrix(model, &pose.local, parent).to_scale_rotation_translation().1
            }
            _ => Quat::IDENTITY,
        }
    };
    let frame = facing * parent;
    turn(w, name, frame.inverse() * by * frame);
}

/// Move the joint `name` of the drawn body by `by`, in its parent's frame.
pub fn shift(w: &mut World, name: &str, by: Vec3) {
    let slot = {
        let model = w.model(MODEL).expect("the stick man's model is generated in setup");
        node(model, name) as usize * 10
    };
    let Some(mut pose) = w.get_mut::<Pose>("stickman") else {
        return;
    };
    if pose.local.len() < slot + 3 {
        return;
    }
    let moved = Vec3::from_slice(&pose.local[slot..slot + 3]) + by;
    pose.local[slot..slot + 3].copy_from_slice(&moved.to_array());
}

// The limb's three joints as they are posed, in the body's frame.
fn joints(model: &Model, local: &[f32], limb: Limb) -> [Vec3; 3] {
    limb.chain()
        .map(|name| joint_matrix(model, local, node(model, name)).w_axis.truncate())
}

/// How much of `hold` the limb takes, 0 to 1, with `target` in the body's frame:
/// all of it within reach, none `SLACK` beyond.
fn weight(model: &Model, local: &[f32], hold: &Hold, target: Vec3) -> f32 {
    let [a, b, c] = joints(model, local, hold.limb);
    let reach = a.distance(b) + b.distance(c);
    let beyond = a.distance(target) - reach;
    hold.grip.clamp(0.0, 1.0) * (1.0 - smooth(beyond / hold.slack))
}

/// Where each hand and foot was put on the last ticks, so that none of them goes
/// anywhere in one tick.
///
/// A hold can take a limb at once, let it go at once, or move to another place
/// on the wall. The limb then is where it was, and goes on as it went, and what
/// it is off its new place dies away as a spring at rest would. The pose under
/// the holds does not jump: `flow` sees to that.
#[derive(Default, Clone, Component)]
pub struct Limbs {
    /// For each limb of `Limb::ALL`, twenty floats. Where its wrist or ankle
    /// was put last tick and the tick before, what it is off its place and how
    /// fast that changes, all in the body's frame. Then where its hold was in
    /// the world last tick and the tick before, how much of the hold it took,
    /// and how many ticks in a row it has had a hold, to two.
    state: Vec<f32>,
    /// The tick of that, and whether the tick before it is known too.
    tick: u64,
    moving: bool,
}

const ROW: usize = 20;
/// A hold that is this much further on in one tick than it was going, metres,
/// has gone somewhere at once: a hold that goes with the body turns and stops
/// with it, which is less than this. So has a hold that a limb takes so much
/// more or less of that it moves the limb this far, metres.
const SNAP: f32 = 0.07;
const TAKEN: f32 = 0.09;
/// How fast what a limb is off its place dies away, per second, and the fastest
/// it is taken to have been going, m/s.
const CATCH_UP: f32 = 30.0;
const FASTEST: f32 = 7.0;

/// Turn the limbs of the drawn body onto `holds`. Call after `animation::step`,
/// once a tick with every hold of that tick.
/// Returns how much of each hold its limb took.
///
/// A hold taken in part moves the wrist or ankle that part of the straight way
/// from where the pose has it to the hold, and the elbow or knee that part of the
/// way to its side. Blending the joints' angles instead would swing the limb
/// through an arc, and through the wall beside it.
pub fn apply(w: &mut World, holds: &[Hold]) -> Vec<f32> {
    let dt = w.dt();
    let tick = w.tick();
    let (root, turn) = frame(w);
    let Some(mut local) = w.get::<Pose>("stickman").map(|pose| pose.local.clone()) else {
        return vec![0.0; holds.len()];
    };
    if local.is_empty() {
        return vec![0.0; holds.len()];
    }
    let mut limbs = w.get::<Limbs>("stickman").map(|l| l.clone()).unwrap_or_default();
    if limbs.state.len() != ROW * 4 {
        limbs.state = vec![0.0; ROW * 4];
        limbs.tick = 0;
    }
    // What is known of the last tick, and of the one before it.
    let (known, moving) = (limbs.tick + 1 == tick, limbs.tick + 1 == tick && limbs.moving);
    let mut taken = vec![0.0; holds.len()];
    {
        let model = w.model(MODEL).expect("the stick man's model is generated in setup");
        for (index, limb) in Limb::ALL.into_iter().enumerate() {
            let chain = limb.chain();
            let [first, middle, tip] = joints(model, &local, limb);
            let hold = holds.iter().position(|hold| hold.limb == limb);
            // Where the hold has the wrist or ankle, and the elbow or knee.
            let (want, pole, flat, held) = match hold {
                Some(i) => {
                    let hold = &holds[i];
                    let target = turn.inverse() * (hold.at - root);
                    let weight = weight(model, &local, hold, target);
                    taken[i] = weight;
                    let bend = (first + target) * 0.5 + hold.bend.unwrap_or(limb.bend());
                    let want = tip.lerp(target, weight);
                    (want, middle.lerp(bend, weight), weight * hold.flat, Some((hold.at, weight)))
                }
                None => (tip, middle, 0.0, None),
            };
            let row = &mut limbs.state[index * ROW..(index + 1) * ROW];
            let (was, before) = (Vec3::from_slice(&row[..3]), Vec3::from_slice(&row[3..6]));
            let (mut off, mut drift) = (Vec3::from_slice(&row[6..9]), Vec3::from_slice(&row[9..12]));
            let (place, place_before) = (Vec3::from_slice(&row[12..15]), Vec3::from_slice(&row[15..18]));
            let (took, run) = (row[18], if known { row[19] } else { 0.0 });
            let (weight, place_now) = held.map_or((0.0, place), |(at, weight)| (weight, at));
            if !known {
                (off, drift) = (Vec3::ZERO, Vec3::ZERO);
            } else {
                // The hold went somewhere at once, or took or left the limb at
                // once: the limb starts from where it was, at the speed it had.
                let jumped = match run {
                    _ if held.is_none() || weight < 0.02 => false,
                    r if r >= 2.0 => (place_now - (place * 2.0 - place_before)).length() > SNAP,
                    r if r >= 1.0 => (place_now - place).length() > SNAP * 2.0,
                    _ => false,
                };
                let lever = held.map_or(was, |(at, _)| turn.inverse() * (at - root)) - tip;
                let firm = hold.is_some_and(|i| holds[i].firm);
                if firm {
                    (off, drift) = (Vec3::ZERO, Vec3::ZERO);
                } else if jumped || (weight - took).abs() * lever.length() > TAKEN {
                    off = was - want;
                    let going = if moving { (was - before) / dt } else { Vec3::ZERO };
                    drift = going.clamp_length_max(FASTEST);
                }
                let decay = math::exp(-CATCH_UP * dt);
                let pull = drift + off * CATCH_UP;
                (off, drift) = ((off + pull * dt) * decay, (drift - pull * (CATCH_UP * dt)) * decay);
                if off.length() < 5e-4 {
                    (off, drift) = (Vec3::ZERO, Vec3::ZERO);
                }
            }
            row[15..18].copy_from_slice(&place.to_array());
            row[12..15].copy_from_slice(&place_now.to_array());
            row[18] = weight;
            row[19] = if held.is_some() { (run + 1.0).min(2.0) } else { 0.0 };
            let at = want + off;
            row[3..6].copy_from_slice(&was.to_array());
            row[..3].copy_from_slice(&at.to_array());
            row[6..9].copy_from_slice(&off.to_array());
            row[9..12].copy_from_slice(&drift.to_array());
            if at == tip {
                continue;
            }
            let ik = Ik {
                chain: chain.map(String::from),
                target: at,
                pole,
                weight: 1.0,
            };
            solve_ik(model, &mut local, &ik).expect("the limb's chain is in the rig");
            if flat > 0.0 {
                // A foot that stands is flat, whatever the shin does.
                let shin = joint_matrix(model, &local, node(model, chain[1]))
                    .to_scale_rotation_translation()
                    .1;
                let slot = node(model, chain[2]) as usize * 10 + 3;
                let was = Quat::from_slice(&local[slot..slot + 4]);
                let flat = was.slerp(shin.inverse(), flat).normalize();
                local[slot..slot + 4].copy_from_slice(&flat.to_array());
            }
        }
    }
    limbs.moving = known;
    limbs.tick = tick;
    if let Some(mut kept) = w.get_mut::<Limbs>("stickman") {
        *kept = limbs;
    }
    let mut pose = w.require_mut::<Pose>("stickman");
    if pose.local != local {
        pose.local = local;
    }
    taken
}

/// Move the wrists and ankles of `holds` there, after `apply`: a correction of
/// this tick's pose, which `apply` then knows of.
pub fn correct(w: &mut World, holds: &[Hold]) {
    if holds.is_empty() {
        return;
    }
    let (root, turn) = frame(w);
    let Some(mut local) = w.get::<Pose>("stickman").map(|pose| pose.local.clone()) else {
        return;
    };
    let mut put = vec![];
    {
        let model = w.model(MODEL).expect("the stick man's model is generated in setup");
        for hold in holds {
            let chain = hold.limb.chain();
            let target = turn.inverse() * (hold.at - root);
            let weight = weight(model, &local, hold, target);
            if weight <= 0.0 {
                continue;
            }
            let [first, middle, tip] = joints(model, &local, hold.limb);
            let bend = (first + target) * 0.5 + hold.bend.unwrap_or(hold.limb.bend());
            let at = tip.lerp(target, weight);
            let ik = Ik {
                chain: chain.map(String::from),
                target: at,
                pole: middle.lerp(bend, weight),
                weight: 1.0,
            };
            solve_ik(model, &mut local, &ik).expect("the limb's chain is in the rig");
            if hold.flat > 0.0 {
                let shin = joint_matrix(model, &local, node(model, chain[1]))
                    .to_scale_rotation_translation()
                    .1;
                let slot = node(model, chain[2]) as usize * 10 + 3;
                let was = Quat::from_slice(&local[slot..slot + 4]);
                let flat = was.slerp(shin.inverse(), weight * hold.flat).normalize();
                local[slot..slot + 4].copy_from_slice(&flat.to_array());
            }
            put.push((hold.limb, at));
        }
    }
    if let Some(mut limbs) = w.get_mut::<Limbs>("stickman") {
        if limbs.state.len() == ROW * 4 {
            for (limb, at) in put {
                let index = Limb::ALL.iter().position(|l| *l == limb).unwrap();
                limbs.state[index * ROW..index * ROW + 3].copy_from_slice(&at.to_array());
            }
        }
    }
    let mut pose = w.require_mut::<Pose>("stickman");
    if pose.local != local {
        pose.local = local;
    }
}
