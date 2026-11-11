//! The route of the scripted runner through The Crown, the upper district of
//! Gravity Cascade: from the level's start to the first handoff roof.
#![allow(dead_code, unused_imports)]
use crate::bot::Step::{self, *};

/// Where the runner stands when the part starts: its feet and its yaw in degrees
/// (180 faces north). `None` is the level's own start.
pub const ENTRY: Option<[f32; 4]> = None;
/// Where the part ends: the first handoff roof.
pub const EXIT: [f32; 3] = [0.0, 86.0, 60.0];

/// The steps of the part. Every Jump and Slide is pressed `early` metres before
/// its designed point; a negative `early` is late.
pub fn route(_early: f32) -> Vec<Step> {
    Vec::new()
}
