//! Switchback Run, run from the start to the finish by the scripted runner.
mod bot;

use bot::Step::{self, *};

// The route is in three parts, so that the shortcut can take the place of the
// third fold. In each part `early` moves every take-off point this many metres
// back along the runner's way, to prove that no jump needs an exact press, and
// `side` moves the runner's line on every beam this many metres off its middle.

/// From the start to the third terrace of row C, where the shortcut leaves.
fn to_the_third_terrace(early: f32, side: f32) -> Vec<Step> {
    let (e, s) = (early, side);
    vec![
        // Leg 1, east on row A.
        Sprint(-24.9 - e, 24.0),
        Jump,
        Sprint(-16.95 - e, 23.6),
        Jump,
        Sprint(-9.9 - e, 24.0),
        Jump,
        Sprint(-3.5 - e, 24.8),
        Slide,
        Sprint(5.6 - e, 24.4),
        Jump,
        Sprint(11.0, 24.4),
        Sprint(16.4, 24.6),
        // The first fold: onto the dock, north into the notch, and up the fire
        // escape.
        Sprint(17.0, 22.4 + e),
        Jump,
        Sprint(17.0, 17.9 + e),
        Jump,
        Sprint(17.2, 12.6),
        Run(18.8 - e, 10.0),
        Jump,
        Run(20.9, 9.9),
        Run(20.9, 9.0 + e),
        Jump,
        Run(20.9, 6.9),
        Run(21.0 - e, 6.9),
        Jump,
        Run(23.5, 6.9),
        // Leg 2, west on row B: the beam and the corridor.
        Sprint(25.6, 7.8),
        Sprint(27.0, 10.2),
        Sprint(25.8, 12.3 + s),
        Sprint(12.0, 12.3 + s),
        Sprint(9.0 + e, 12.3),
        Slide,
        Sprint(3.45 + e, 12.3),
        Jump,
        Sprint(-0.5 + e, 12.3),
        Slide,
        Sprint(-8.6 + e, 12.3),
        Jump,
        Run(-13.4, 12.0),
        // The second fold: the edge on the west house. The runner hangs, waits a
        // moment as a player does, and moves left by the hands to the open roof.
        Sprint(-13.5, 4.4 + e),
        Jump,
        Hang,
        Wait(0.5),
        Shimmy(-15.9, 1.84),
        Run(-16.1, -1.0),
        // The planks over the light well, and the north alley.
        Sprint(-22.0 + s, -4.2),
        Sprint(-23.9 + s, -6.6),
        Sprint(-23.9 + s, -8.5),
        Sprint(-22.0 + s, -11.2),
        Sprint(-21.8, -14.6 + e),
        Jump,
        Sprint(-21.6, -20.0),
        // Leg 3, east on row C: the terraces.
        Sprint(-19.0, -23.4),
        Sprint(-9.95 - e, -24.0),
        Jump,
        Sprint(-3.4 - e, -24.0),
        Jump,
        Sprint(9.0, -24.0),
    ]
}

/// The safe way from the third terrace to the plant room: down the last terrace,
/// through the fire wall, south over the alley and up the stair.
fn round_the_corner(early: f32) -> Vec<Step> {
    let e = early;
    vec![
        Sprint(14.3 - e, -24.0),
        Jump,
        Sprint(27.0 - e, -24.0),
        Slide,
        Sprint(32.6, -24.2),
        Sprint(33.2, -18.4 - e),
        Jump,
        Sprint(33.2, -13.8),
        Sprint(30.5, -10.0),
        Sprint(18.0, -10.0),
    ]
}

/// The shortcut from the third terrace to the plant room: one jump of 6 m over
/// the alley and the yard.
fn over_the_yard(early: f32) -> Vec<Step> {
    vec![
        Sprint(12.8, -23.0),
        Sprint(13.4, -18.4 - early),
        Jump,
        Sprint(14.4, -10.2),
    ]
}

/// From the plant room over the plank, and round the tower by its steps.
fn up_the_tower(early: f32, side: f32) -> Vec<Step> {
    let (e, s) = (early, side);
    vec![
        Sprint(12.3, -10.1 + s),
        Sprint(6.8, -7.7 + s),
        Run(2.3 + e, -7.75),
        Jump,
        Run(-0.8 + e, -7.75),
        Jump,
        Run(-2.8, -7.75),
        Run(-2.8, -7.3 - e),
        Jump,
        Run(-2.8, -4.3 - e),
        Jump,
        Run(-2.7, -2.5),
        Run(0.6, -2.25),
    ]
}

fn route(early: f32, side: f32) -> Vec<Step> {
    [to_the_third_terrace(early, side), round_the_corner(early), up_the_tower(early, side)].concat()
}

#[test]
fn a_runner_finishes_the_level() {
    let time = bot::finish(1, &route(0.0, 0.0));
    assert!((55.0..=80.0).contains(&time), "the run took {time:.1} s");
}

// No jump of the route needs an exact press: the runner still finishes when every
// take-off is 0.3 m early, and when every one is 0.15 m late.
#[test]
fn the_route_has_a_margin_at_every_take_off() {
    for early in [0.3, 0.15, -0.15] {
        println!("every take-off {early} m early");
        bot::finish(1, &route(early, 0.0));
    }
}

// No beam needs an exact line: the runner still finishes a quarter of a metre to
// either side of the middle of every beam.
#[test]
fn the_beams_forgive_a_line_off_the_middle() {
    for side in [0.25, -0.25] {
        println!("{side} m off the middle of every beam");
        bot::finish(1, &route(0.0, side));
    }
}

// The shortcut is a real second line: it finishes, and it is faster.
#[test]
fn the_shortcut_cuts_the_third_fold() {
    let safe = bot::finish(1, &route(0.0, 0.0));
    let steps = [to_the_third_terrace(0.0, 0.0), over_the_yard(0.0), up_the_tower(0.0, 0.0)].concat();
    let fast = bot::finish(1, &steps);
    assert!(fast < safe - 3.0, "the shortcut took {fast:.1} s and the safe way {safe:.1} s");
}
