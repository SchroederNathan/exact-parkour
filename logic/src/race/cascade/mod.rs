//! Gravity Cascade: the long race. It loses about 100 m from the crown of the
//! city to the water, and its ramps are for sliding: a slide down a ramp gives the
//! runner more speed than its legs do, and that speed crosses a gap that a sprint
//! does not.
//!
//! The level is three districts in a row from south to north, each in its own
//! file, and two handoff roofs between them, which are here:
//!
//! 1. `upper`: The Crown, from the start at about 120 m down to the first handoff
//!    roof at 86 m.
//! 2. `middle`: The Works, from 86 m down to the second handoff roof at 52 m.
//! 3. `lower`: The Sluice, from 52 m down to the finish near 18 m.
//!
//! A district builds only inside its own zone, and it joins the next one only at
//! the handoff roof. Each has its own beat sheet.
use super::kit::{Level, Look};

pub mod lower;
pub mod middle;
pub mod upper;

/// Where the feet stand on the two handoff roofs. The upper district ends on the
/// first and the middle district starts there; the middle district ends on the
/// second and the lower district starts there.
pub const HANDOFF_1: [f32; 3] = [0.0, 86.0, 60.0];
pub const HANDOFF_2: [f32; 3] = [0.0, 52.0, -60.0];
/// The footprint of a handoff roof: `[width (X), depth (Z)]` about its point.
pub const HANDOFF_SIZE: [f32; 2] = [18.0, 12.0];

pub fn level() -> Level {
    let mut level = Level::new(
        "Gravity Cascade",
        "Ramps to slide, gaps to launch, 100 m to lose",
        "#3d7bff",
    );
    upper::build(&mut level);

    level
        .building("handoff-1", [HANDOFF_1[0], HANDOFF_1[2]], HANDOFF_SIZE, HANDOFF_1[1])
        .look(Look::Slate);
    level.fall_below(middle::ENTRY_FALL);
    level.checkpoint(HANDOFF_1, [HANDOFF_1[0], HANDOFF_1[2] - 20.0]);
    middle::build(&mut level);

    level
        .building("handoff-2", [HANDOFF_2[0], HANDOFF_2[2]], HANDOFF_SIZE, HANDOFF_2[1])
        .look(Look::Slate);
    level.fall_below(lower::ENTRY_FALL);
    level.checkpoint(HANDOFF_2, [HANDOFF_2[0], HANDOFF_2[2] - 20.0]);
    // The lower district puts the finish where its route ends.
    level.finish_at([HANDOFF_2[0], HANDOFF_2[1], HANDOFF_2[2] - 4.0]);
    lower::build(&mut level);
    level
}
