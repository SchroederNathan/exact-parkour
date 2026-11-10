//! The test yard: one long roof with nothing on it. A test stands the runner on
//! it and builds what it measures.
use super::kit::Level;

/// The height of the yard's roof.
pub const ROOF: f32 = 20.0;

pub fn level() -> Level {
    let mut level = Level::new("Test Yard", "One roof", "#e0b040");
    level.building("yard", [0.0, 0.0], [12.0, 40.0], ROOF);
    level.fall_below(4.0);
    level.checkpoint([0.0, ROOF, 18.0], [0.0, 0.0]);
    level.checkpoint([0.0, ROOF, -18.0], [0.0, -30.0]);
    level.finish_at([0.0, ROOF, -19.0]);
    level
}
