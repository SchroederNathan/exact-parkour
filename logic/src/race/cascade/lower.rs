//! The Sluice, the lower district of Gravity Cascade. A placeholder until the district
//! is designed: it builds nothing. Its zone is z from -190 to -67, x from -85 to 85.
use crate::race::kit::Level;

/// The fall line of the handoff roof this district starts on: the height under
/// which a runner who has not yet reached the district's first checkpoint has
/// fallen.
pub const ENTRY_FALL: f32 = 46.0;

pub fn build(_level: &mut Level) {}
