//! The route of the scripted runner through The Works, the middle district of
//! Gravity Cascade: from the first handoff roof to the second.
#![allow(dead_code, unused_imports)]
use crate::bot::Step::{self, *};

/// Where the runner stands when the part starts: its feet and its yaw in degrees
/// (180 faces north).
pub const ENTRY: Option<[f32; 4]> = Some([0.0, 86.0, 60.0, 180.0]);
/// Where the part ends: the second handoff roof.
pub const EXIT: [f32; 3] = [0.0, 52.0, -60.0];

/// The steps of the part. Every Jump and Slide is pressed `early` metres before
/// its designed point; a negative `early` is late.
pub fn route(_early: f32) -> Vec<Step> {
    Vec::new()
}
