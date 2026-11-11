//! The lower district of Gravity Cascade, run by the scripted runner on its own.
mod bot;
#[path = "cascade/lower.rs"]
mod part;

// A placeholder until the district is designed. Replace it with tests that cross
// the district: `bot::cross(3, part::ENTRY, &part::route(0.0), part::EXIT)`.
#[test]
fn the_district_is_not_built_yet() {
    assert!(part::route(0.0).is_empty());
}
