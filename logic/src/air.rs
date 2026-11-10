//! The body in the air.
//!
//! A jump from a run is one long stride: the foot on the ground pushes, the other
//! leg leads, and the body lands on the leading foot and runs on. So the pose in
//! the air depends on three things. The vertical speed says how far through the
//! jump the body is. The ground speed at take-off says how long the stride is: a
//! jump from a stand is a tuck on both legs, one from a sprint is a wide split.
//! The foot that pushed says which leg leads.
//!
//! The clips here are drawn with the left leg leading. `mirror` swaps the sides of
//! a pose for a jump off the left foot.
use crate::gait::{RUN, SPRINT};
use crate::stickman::{keyed, Key, MODEL};
use exact_game::asset::Clip;
use exact_game::rig::Rig;
use exact_game::*;

/// The vertical speed of each moment of the clips, m/s, from the first to the last:
/// leaving the ground, rising, the top, coming down, a real drop, a fall too long
/// to be easy.
const LIFT: [f32; 6] = [3.8, 1.5, 0.0, -2.5, -5.0, -8.5];

/// The clips, and the ground speed at take-off each is drawn for, m/s.
pub const STRIDES: [(f32, &str); 3] = [(0.0, "air-stand"), (RUN, "air-run"), (SPRINT, "air-sprint")];

/// One moment in the air. A leg is the turns of its thigh, shin and foot about X:
/// a negative thigh is forward, a positive shin bends the knee, a positive foot
/// points the toes. An arm is how far it is lifted out from the side, how far
/// forward, and the elbow's bend. `lead` is the left leg and `with` the arm on
/// its side; that arm is back when the leg is forward, as in a stride.
struct Moment {
    spine: f32,
    neck: f32,
    lead: [f32; 3],
    trail: [f32; 3],
    with: [f32; 3],
    against: [f32; 3],
}

fn clip(rig: &Rig, name: &str, moments: [Moment; 6]) -> Clip {
    let joints: Vec<Vec<(&str, [f32; 3])>> = moments
        .iter()
        .map(|m| {
            let arm = |a: [f32; 3], side: f32| ([-a[1], 0.0, side * a[0]], [a[2], 0.0, 0.0]);
            let (upper_l, fore_l) = arm(m.with, 1.0);
            let (upper_r, fore_r) = arm(m.against, -1.0);
            vec![
                ("spine", [m.spine, 0.0, 0.0]),
                ("neck", [m.neck, 0.0, 0.0]),
                ("thigh_l", [m.lead[0], 0.0, 0.0]),
                ("shin_l", [m.lead[1], 0.0, 0.0]),
                ("foot_l", [m.lead[2], 0.0, 0.0]),
                ("thigh_r", [m.trail[0], 0.0, 0.0]),
                ("shin_r", [m.trail[1], 0.0, 0.0]),
                ("foot_r", [m.trail[2], 0.0, 0.0]),
                ("upper_arm_l", upper_l),
                ("forearm_l", fore_l),
                ("upper_arm_r", upper_r),
                ("forearm_r", fore_r),
            ]
        })
        .collect();
    let keys: Vec<Key> = joints
        .iter()
        .enumerate()
        .map(|(i, joints)| Key {
            at: i as f32 / 5.0,
            drop: 0.0,
            joints,
        })
        .collect();
    keyed(rig, name, 1.0, &keys)
}

// A fall too long to be easy looks the same from any start: the knees come up,
// the chest goes forward, and the arms are thrown high and back.
const PLUNGE: Moment = Moment {
    spine: 0.3,
    neck: -0.25,
    lead: [-0.75, 1.0, 0.0],
    trail: [-0.3, 0.85, 0.0],
    with: [1.6, -0.15, -0.3],
    against: [1.6, -0.15, -0.3],
};

/// The three clips of the air, each one second long. The game sets the phase.
pub fn clips(rig: &Rig) -> Vec<Clip> {
    vec![
        // From a stand: both feet push, the arms are thrown up, the knees tuck
        // at the top and the legs open again for the ground.
        clip(
            rig,
            "air-stand",
            [
                Moment {
                    spine: -0.06,
                    neck: 0.0,
                    lead: [0.02, 0.1, 0.7],
                    trail: [0.08, 0.12, 0.7],
                    with: [0.25, 2.3, -0.35],
                    against: [0.25, 2.3, -0.35],
                },
                Moment {
                    spine: -0.02,
                    neck: 0.0,
                    lead: [-0.45, 0.85, 0.5],
                    trail: [-0.3, 0.75, 0.5],
                    with: [0.35, 2.5, -0.3],
                    against: [0.35, 2.5, -0.3],
                },
                Moment {
                    spine: 0.08,
                    neck: 0.0,
                    lead: [-0.95, 1.4, 0.2],
                    trail: [-0.75, 1.3, 0.2],
                    with: [0.6, 1.3, -0.5],
                    against: [0.6, 1.3, -0.5],
                },
                Moment {
                    spine: 0.1,
                    neck: 0.0,
                    lead: [-0.45, 0.6, 0.0],
                    trail: [-0.3, 0.55, 0.0],
                    with: [0.8, 0.1, -0.5],
                    against: [0.8, 0.1, -0.5],
                },
                Moment {
                    spine: 0.2,
                    neck: 0.0,
                    lead: [-0.5, 0.5, 0.25],
                    trail: [-0.05, 0.55, 0.25],
                    with: [1.2, 0.15, -0.45],
                    against: [1.2, 0.15, -0.45],
                },
                PLUNGE,
            ],
        ),
        // From a run: the push leg is long behind and the lead knee drives up,
        // the trailing heel folds in, the lead leg opens toward the ground and
        // the body lands on it as in a stride.
        clip(
            rig,
            "air-run",
            [
                Moment {
                    spine: 0.14,
                    neck: -0.05,
                    lead: [-1.0, 1.5, 0.1],
                    trail: [0.55, 0.2, 0.8],
                    with: [0.3, -0.7, -1.0],
                    against: [0.2, 1.0, -1.3],
                },
                Moment {
                    spine: 0.0,
                    neck: 0.0,
                    lead: [-1.05, 1.3, 0.2],
                    trail: [0.45, 0.95, 0.6],
                    with: [0.5, -0.45, -0.8],
                    against: [0.45, 0.9, -1.1],
                },
                Moment {
                    spine: 0.04,
                    neck: 0.0,
                    lead: [-0.9, 0.7, 0.0],
                    trail: [-0.2, 1.6, 0.4],
                    with: [0.75, -0.1, -0.6],
                    against: [0.7, 0.4, -0.7],
                },
                Moment {
                    spine: 0.14,
                    neck: -0.05,
                    lead: [-0.55, 0.3, -0.15],
                    trail: [0.15, 1.1, 0.4],
                    with: [0.6, -0.35, -0.6],
                    against: [0.5, 0.5, -0.8],
                },
                Moment {
                    spine: 0.2,
                    neck: -0.1,
                    lead: [-0.45, 0.5, 0.2],
                    trail: [-0.1, 0.6, 0.25],
                    with: [1.15, 0.1, -0.45],
                    against: [1.15, 0.2, -0.45],
                },
                PLUNGE,
            ],
        ),
        // From a sprint: the same stride, split wider, the chest further forward
        // and the arms driving harder.
        clip(
            rig,
            "air-sprint",
            [
                Moment {
                    spine: 0.24,
                    neck: -0.12,
                    lead: [-1.25, 1.35, 0.1],
                    trail: [0.85, 0.25, 0.9],
                    with: [0.35, -0.95, -1.2],
                    against: [0.15, 1.3, -1.5],
                },
                Moment {
                    spine: 0.1,
                    neck: -0.05,
                    lead: [-1.3, 1.0, 0.2],
                    trail: [0.8, 1.0, 0.7],
                    with: [0.5, -0.7, -0.9],
                    against: [0.4, 1.2, -1.2],
                },
                Moment {
                    spine: 0.08,
                    neck: 0.0,
                    lead: [-1.15, 0.5, 0.0],
                    trail: [0.1, 1.75, 0.5],
                    with: [0.7, -0.3, -0.7],
                    against: [0.65, 0.7, -0.8],
                },
                Moment {
                    spine: 0.2,
                    neck: -0.08,
                    lead: [-0.75, 0.35, -0.2],
                    trail: [0.3, 1.3, 0.4],
                    with: [0.55, -0.5, -0.8],
                    against: [0.45, 0.7, -1.0],
                },
                Moment {
                    spine: 0.26,
                    neck: -0.12,
                    lead: [-0.55, 0.5, 0.2],
                    trail: [0.0, 0.8, 0.3],
                    with: [1.1, 0.0, -0.5],
                    against: [1.1, 0.3, -0.5],
                },
                PLUNGE,
            ],
        ),
    ]
}

/// The phase of the air clips at a vertical speed of `rising` m/s.
pub fn phase(rising: f32) -> f32 {
    let last = LIFT.len() - 1;
    if rising >= LIFT[0] {
        return 0.0;
    }
    if rising <= LIFT[last] {
        return 0.999;
    }
    // The first moment slower than this speed, and the one before it.
    let i = LIFT.partition_point(|&v| v > rising).clamp(1, last);
    let k = (LIFT[i - 1] - rising) / (LIFT[i - 1] - LIFT[i]);
    ((i - 1) as f32 + k) / last as f32
}

/// The leg that leads a jump from the gait's cycle at `phase`: +1 the left, -1
/// the right. The leg that came down last is the one that pushes.
pub fn lead(phase: f32) -> f32 {
    let cycle = phase - math::floor(phase);
    if cycle < 0.5 {
        -1.0
    } else {
        1.0
    }
}

/// The gait's phase at which the leading foot strikes the ground.
pub fn strike(lead: f32) -> f32 {
    if lead < 0.0 {
        0.5
    } else {
        0.0
    }
}

/// Swap the left and right sides of the drawn body's pose.
pub fn mirror(w: &mut World) {
    let pairs: Vec<(usize, Option<usize>)> = {
        let model = w.model(MODEL).expect("the stick man's model is generated in setup");
        let find = |name: &str| model.nodes.iter().position(|n| n.name == name);
        model
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| match node.name.strip_suffix("_l") {
                Some(stem) => Some((i, Some(find(&format!("{stem}_r"))?))),
                None if node.name.ends_with("_r") => None,
                None => Some((i, None)),
            })
            .collect()
    };
    let Some(mut pose) = w.get_mut::<Pose>("stickman") else {
        return;
    };
    // A turn seen in a mirror keeps its part about X and reverses the others.
    let flip = |row: &mut [f32]| {
        row[4] = -row[4];
        row[5] = -row[5];
    };
    for (left, right) in pairs {
        if left * 10 + 10 > pose.local.len() {
            continue;
        }
        match right {
            None => flip(&mut pose.local[left * 10..left * 10 + 10]),
            Some(right) => {
                for slot in 3..7 {
                    pose.local.swap(left * 10 + slot, right * 10 + slot);
                }
                flip(&mut pose.local[left * 10..left * 10 + 10]);
                flip(&mut pose.local[right * 10..right * 10 + 10]);
            }
        }
    }
}
