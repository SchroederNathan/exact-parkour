//! Switchback Run. A placeholder until the level is designed: one roof.
use super::kit::Level;

pub fn level() -> Level {
    let mut level = Level::new("Switchback Run", "One roof", "#e0b040");
    level.building("roof", [0.0, 0.0], [12.0, 40.0], 20.0);
    level.fall_below(4.0);
    level.checkpoint([0.0, 20.0, 18.0], [0.0, 0.0]);
    level.checkpoint([0.0, 20.0, -16.0], [0.0, -30.0]);
    level.finish_at([0.0, 20.0, -18.0]);
    level
}
