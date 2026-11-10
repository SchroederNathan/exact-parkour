//! Skyline Sprint, run from the start to the finish by the scripted runner.
mod bot;

use bot::Step::{self, *};

/// The run time of the scripted runner on this level, seconds.
const TARGET: std::ops::RangeInclusive<f32> = 40.0..=58.0;

// The safe line from the start to checkpoint 5 on the Chancery. `gap` moves the
// point before each jump over a gap back along the route, metres, and `wall` does
// the same before each wall: the margin tests use them.
fn to_the_chancery(gap: f32, wall: f32) -> Vec<Step> {
    vec![
        // 1. Meridian House: the vault in the middle of the roof.
        Sprint(0.0, 127.95 + wall),
        Jump,
        // 2. The first alley, 2 m.
        Sprint(0.0, 115.4 + gap),
        Jump,
        // 3. The Annex: the long run to the slide gate, and checkpoint 1.
        Sprint(0.0, 99.0),
        Slide,
        Sprint(0.0, 89.0),
        // 4. Four roofs down and east: 2.5, 3, 3.5 and 4 m. Checkpoint 2
        // is on the second.
        Sprint(0.0, 83.4 + gap),
        Jump,
        Sprint(3.5, 66.9 + gap),
        Jump,
        Sprint(7.0, 51.4 + gap),
        Jump,
        Sprint(9.5, 38.9 + gap),
        Jump,
        // 5. Checkpoint 3 and the pipe bridge over the avenue.
        Sprint(12.5, 24.0),
        Sprint(13.0, 16.0),
        Sprint(13.0, -9.0),
        // 6. The drop off the hall and the roll.
        Sprint(12.5, -16.0),
        // 7. Checkpoint 4 and the kicker: up the ramp and off the stair head.
        // Checkpoint 5 is at the landing.
        Sprint(10.0, -23.2),
        Sprint(10.0, -35.6 + gap),
        Jump,
        Sprint(9.5, -44.5),
    ]
}

// The safe line: every beat of the level.
fn safe(gap: f32, wall: f32) -> Vec<Step> {
    let mut route = to_the_chancery(gap, wall);
    route.extend([
        // 8. The vault and the jump after it.
        Sprint(5.0, -52.0),
        Sprint(5.0, -54.55 + wall),
        Jump,
        Sprint(4.5, -61.6 + gap),
        Jump,
        // 9. The second gate, checkpoint 6 and the climb to the attic.
        Sprint(2.0, -73.0),
        Slide,
        Sprint(2.0, -79.5),
        Sprint(2.0, -83.0 + wall),
        Jump,
        Sprint(2.0, -86.0),
        // 10. The last jump, the stair and the helipad.
        Sprint(2.0, -91.6 + gap),
        Jump,
        Sprint(0.0, -101.5),
        Sprint(0.0, -115.5),
    ]);
    route
}

// The fast line: past the end of the vault wall, over to the yellow pad, and up
// the catwalk to the attic.
fn fast(gap: f32) -> Vec<Step> {
    let mut route = to_the_chancery(gap, 0.0);
    route.extend([
        Sprint(11.4, -55.0),
        Sprint(11.4, -61.6 + gap),
        Jump,
        Sprint(10.9, -68.5),
        Sprint(10.6, -70.5),
        Sprint(10.6, -84.5),
        Sprint(9.0, -88.0),
        Sprint(8.5, -91.6 + gap),
        Jump,
        Sprint(1.5, -101.5),
        Sprint(0.0, -115.5),
    ]);
    route
}

#[test]
fn a_runner_finishes_the_level() {
    let time = bot::finish(0, &safe(0.0, 0.0));
    assert!(TARGET.contains(&time), "the run took {time:.1} s, outside {TARGET:?}");
}

// No jump needs an exact press: the runner still finishes when it presses Jump
// 0.3 m earlier or 0.15 m later at every gap, and at every wall. The last case
// presses Jump with the body against each wall.
#[test]
fn the_level_forgives_an_early_and_a_late_press() {
    for (gap, wall) in [(0.3, 0.0), (-0.15, 0.0), (0.0, 0.3), (0.0, -0.15), (0.3, 0.3), (-0.15, -0.15), (0.0, -0.35)] {
        println!("gaps {gap} m early, walls {wall} m early");
        bot::finish(0, &safe(gap, wall));
    }
}

#[test]
fn the_fast_line_is_faster() {
    let safe = bot::finish(0, &safe(0.0, 0.0));
    let fast_time = bot::finish(0, &fast(0.0));
    assert!(fast_time < safe - 1.0, "the fast line took {fast_time:.1} s and the safe line {safe:.1} s");
    for gap in [0.3, -0.15] {
        bot::finish(0, &fast(gap));
    }
}

// The first level asks for no sprint: a runner who only runs crosses every gap,
// by a landing or by a catch of the far edge.
#[test]
fn a_runner_who_never_sprints_still_finishes() {
    let route: Vec<Step> = safe(0.0, 0.0)
        .into_iter()
        .map(|step| match step {
            Sprint(x, z) => Run(x, z),
            other => other,
        })
        .collect();
    bot::finish(0, &route);
}
