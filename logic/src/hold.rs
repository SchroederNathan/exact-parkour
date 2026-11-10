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
/// Distance past its reach over which a limb lets go, metres.
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
    /// Whether a foot stands on what it holds, and so is flat.
    pub flat: bool,
    /// Where the elbow or knee bends to, from the middle of the limb, in the
    /// body's frame, if not the limb's usual way.
    pub bend: Option<Vec3>,
}
impl Hold {
    pub fn new(limb: Limb, at: Vec3) -> Self {
        Hold {
            limb,
            at,
            grip: 1.0,
            flat: limb.is_foot(),
            bend: None,
        }
    }
    pub fn bend(mut self, bend: Vec3) -> Self {
        self.bend = Some(bend);
        self
    }
    pub fn grip(mut self, grip: f32) -> Self {
        self.grip = grip;
        self
    }
    pub fn loose(mut self) -> Self {
        self.flat = false;
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
    hold.grip.clamp(0.0, 1.0) * (1.0 - smooth(beyond / SLACK))
}

/// Turn the limbs of the drawn body onto `holds`. Call after `animation::step`.
/// Returns how much of each hold its limb took.
///
/// A hold taken in part moves the wrist or ankle that part of the straight way
/// from where the pose has it to the hold, and the elbow or knee that part of the
/// way to its side. Blending the joints' angles instead would swing the limb
/// through an arc, and through the wall beside it.
pub fn apply(w: &mut World, holds: &[Hold]) -> Vec<f32> {
    if holds.is_empty() {
        return vec![];
    }
    let (root, turn) = frame(w);
    let Some(mut local) = w.get::<Pose>("stickman").map(|pose| pose.local.clone()) else {
        return vec![0.0; holds.len()];
    };
    if local.is_empty() {
        return vec![0.0; holds.len()];
    }
    let taken: Vec<f32> = {
        let model = w.model(MODEL).expect("the stick man's model is generated in setup");
        holds
            .iter()
            .map(|hold| {
                let chain = hold.limb.chain();
                let target = turn.inverse() * (hold.at - root);
                let weight = weight(model, &local, hold, target);
                if weight <= 0.0 {
                    return 0.0;
                }
                let [first, middle, tip] = joints(model, &local, hold.limb);
                let bend = (first + target) * 0.5 + hold.bend.unwrap_or(hold.limb.bend());
                let ik = Ik {
                    chain: chain.map(String::from),
                    target: tip.lerp(target, weight),
                    pole: middle.lerp(bend, weight),
                    weight: 1.0,
                };
                solve_ik(model, &mut local, &ik).expect("the limb's chain is in the rig");
                if hold.flat {
                    // A foot that stands is flat, whatever the shin does.
                    let shin = joint_matrix(model, &local, node(model, chain[1]))
                        .to_scale_rotation_translation()
                        .1;
                    let slot = node(model, chain[2]) as usize * 10 + 3;
                    let was = Quat::from_slice(&local[slot..slot + 4]);
                    let flat = was.slerp(shin.inverse(), weight).normalize();
                    local[slot..slot + 4].copy_from_slice(&flat.to_array());
                }
                weight
            })
            .collect()
    };
    let mut pose = w.require_mut::<Pose>("stickman");
    if pose.local != local {
        pose.local = local;
    }
    taken
}
