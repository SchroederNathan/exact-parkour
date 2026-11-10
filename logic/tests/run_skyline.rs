//! Skyline Sprint, run from the start to the finish by the scripted runner.
mod bot;

use bot::Step::*;

#[test]
fn a_runner_finishes_the_level() {
    bot::finish(0, &[Run(0.0, -17.5)]);
}
