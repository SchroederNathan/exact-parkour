//! Helix Heights, run from the start to the finish by the scripted runner.
mod bot;

use bot::Step::{self, *};

// The route. `early` moves every press of Jump that many metres back along the
// way the runner comes; a negative number moves it on. `slide` does the same for
// every press of Slide. `safe` takes the plank beside the long gap.
fn route(early: f32, slide: f32, safe: bool) -> Vec<Step> {
    let e = early;
    let s = slide;
    // The middle of the scaffold lane.
    let lane = -41.5;
    // 10. The long gap, or the plank beside it.
    let long_gap = if safe {
        vec![Run(-18.5, -36.4), Run(-25.5, -36.4), Sprint(-28.5, -33.0)]
    } else {
        vec![Sprint(-19.1 + e, -33.0), Jump, Sprint(-26.5, -33.0)]
    };
    let before = vec![
        // 1. The depot: vault the pipe.
        Run(-14.5 - e, 22.0),
        Jump,
        // 2. The ramp to the upper roof.
        Run(-9.0, 22.0),
        Run(2.5, 22.0),
        // 3. The first gap, to the corner block.
        Sprint(11.6 - e, 22.0),
        Jump,
        Sprint(17.5, 22.0),
        // 4. Turn north, vault the pipe, jump to the east tower.
        Sprint(27.0, 22.0),
        Run(32.0, 19.0 + e),
        Jump,
        Sprint(32.0, 12.4 + e),
        Jump,
        Sprint(32.0, 8.0),
        // 5. The step to mantle and the wall to climb.
        Run(32.0, 4.2 + e),
        Jump,
        Run(33.0, 1.0),
        Run(33.0, -1.8 + e),
        Jump,
        Run(33.0, -4.5),
        // 6. The ledge: hang, move west over the well, climb.
        Run(28.6, -7.0),
        Run(28.6, -8.8 + e),
        Jump,
        Hang,
        Shimmy(26.2, -9.26),
        Run(26.2, -12.5),
        // 7. The jump to the roof 2 m higher.
        Run(24.0, -15.0),
        Sprint(24.0, -21.6 + e),
        Jump,
        Sprint(24.0, -27.0),
        // 8. The stair to the wing.
        Run(22.4, -34.0),
        Sprint(12.0, -34.0),
        // 9. The gap to the first slab, and the girder.
        Sprint(8.4 + e, -34.0),
        Jump,
        Sprint(2.5, -34.0),
        Sprint(-3.0, -31.5),
        Sprint(-12.0, -31.5),
    ];
    let after = vec![
        // 11. The ramp on the slab and the plank to the scaffold.
        Sprint(-39.4, -33.0),
        Sprint(lane, -27.0),
        Sprint(lane, -21.0),
        // 12. The first gate and the stair.
        Sprint(lane, -18.5 - s),
        Slide,
        Sprint(lane, -11.5),
        Sprint(lane, 2.0),
        // 13. The scaffold wall, the second gate and the gap in the deck.
        Run(lane, 6.2 - e),
        Jump,
        Run(lane, 8.5),
        Sprint(lane, 10.5 - s),
        Slide,
        Sprint(lane, 17.6 - e),
        Jump,
        Sprint(lane, 23.0),
        // 14. The bridge and the roof of the slab tower.
        Sprint(lane, 37.5),
        Sprint(-38.0, 42.5),
        Sprint(-27.0 - e, 44.0),
        Jump,
        Sprint(-20.0 - s, 44.0),
        Slide,
        Sprint(-13.0, 44.0),
        // 15. The crane: the ballast and the machine house.
        Run(-10.2 - e, 44.0),
        Jump,
        Run(-8.2, 44.0),
        Run(-6.8 - e, 44.0),
        Jump,
        Run(-4.0, 44.0),
        // 16. The jib.
        Run(0.0, 43.0),
        Sprint(0.0, 22.35 + e),
        Jump,
        // 17. The deck on the core.
        Sprint(0.0, 12.5),
    ];
    [before, long_gap, after].concat()
}

#[test]
fn a_runner_finishes_the_level() {
    let time = bot::finish(2, &route(0.0, 0.0, false));
    assert!((70.0..=100.0).contains(&time), "the run took {time:.1} s");
}

// The level does not need one exact script: every Jump pressed 0.3 m early, or
// 0.15 m late, still finishes, and so does every Slide.
#[test]
fn every_jump_pressed_early_still_finishes() {
    bot::finish(2, &route(0.3, 0.0, false));
}

#[test]
fn every_jump_pressed_late_still_finishes() {
    bot::finish(2, &route(-0.15, 0.0, false));
}

#[test]
fn every_slide_pressed_early_still_finishes() {
    bot::finish(2, &route(0.0, 0.3, false));
}

#[test]
fn every_slide_pressed_late_still_finishes() {
    bot::finish(2, &route(0.0, -0.15, false));
}

// The plank beside the long gap is a way on: slower, with no catch.
#[test]
fn the_safe_line_beside_the_long_gap_finishes() {
    bot::finish(2, &route(0.0, 0.0, true));
}

// For tuning: `EARLY=0.5 SLIDE=-0.2 cargo test ... probe -- --ignored --nocapture`
// runs the route with those shifts and prints the log. `SAFE=1` takes the plank.
#[test]
#[ignore]
fn probe() {
    let read = |name: &str| std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    bot::finish(2, &route(read("EARLY"), read("SLIDE"), std::env::var("SAFE").is_ok()));
}
