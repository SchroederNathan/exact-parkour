//! What a player sees of the camera: where the runner stands on the screen, how a
//! finger turns the view, and how the view follows the run.
use exact_beat_em_up_logic::body::{self, HEIGHT, RADIUS};
use exact_beat_em_up_logic::camera::{self, tune};
use exact_beat_em_up_logic::{ExactBeatEmUp, Options, ARENA_MODE};
use exact_game::*;

type Game = Sim<ExactBeatEmUp>;

// A little over one 120 Hz tick: each `run` of it advances exactly one.
const TICK_MS: f64 = 8.334;
const LANDSCAPE: Vec2 = Vec2::new(932.0, 430.0);
const PORTRAIT: Vec2 = Vec2::new(430.0, 932.0);

fn start() -> Game {
    let mut sim = Sim::new(Options {
        seed: 7,
        mode: ARENA_MODE,
        ..Options::default()
    })
    .unwrap();
    sim.run(25.0);
    sim
}

fn start_on(viewport: Vec2) -> Game {
    let mut sim = start();
    sim.viewport(viewport.x, viewport.y);
    sim.run(TICK_MS);
    sim
}

fn eye(sim: &Game) -> Vec3 {
    sim.get::<Transform>("camera").unwrap().position
}

fn feet(sim: &Game) -> Vec3 {
    body::feet(sim.get::<Transform>("player").unwrap().position)
}

// Where a world point is on a canvas of `viewport` points, origin at the top left.
#[track_caller]
fn screen(sim: &Game, viewport: Vec2, point: Vec3) -> Vec2 {
    let pose = *sim.get::<Transform>("camera").unwrap();
    let lens = *sim.get::<Camera>("camera").unwrap();
    assert!(
        (point - pose.position).dot(pose.rotation * -Vec3::Z) > lens.near,
        "{point} is behind the lens"
    );
    let view = Mat4::from_rotation_translation(pose.rotation, pose.position).inverse();
    let ndc = (lens.matrix(viewport) * view).project_point3(point);
    Vec2::new(
        (ndc.x + 1.0) * 0.5 * viewport.x,
        (1.0 - ndc.y) * 0.5 * viewport.y,
    )
}

// The standing body's box on the canvas: [left, top, right, bottom] in points.
#[track_caller]
fn runner(sim: &Game, viewport: Vec2) -> [f32; 4] {
    let right = sim.get::<Transform>("camera").unwrap().rotation * Vec3::X;
    let feet = feet(sim);
    let corners = [feet, feet + Vec3::Y * HEIGHT]
        .into_iter()
        .flat_map(|at| [at - right * RADIUS, at + right * RADIUS])
        .map(|at| screen(sim, viewport, at));
    corners.fold(
        [f32::MAX, f32::MAX, f32::MIN, f32::MIN],
        |[left, top, right, bottom], at| {
            [
                left.min(at.x),
                top.min(at.y),
                right.max(at.x),
                bottom.max(at.y),
            ]
        },
    )
}

// The move pad and the action buttons of `app.contract` on a coarse pointer with
// no safe-area inset: [left, top, right, bottom] in points.
fn controls(viewport: Vec2) -> [[f32; 4]; 2] {
    let (w, h) = (viewport.x, viewport.y);
    let pad = [24.0, h - 136.0, 136.0, h - 24.0];
    let buttons = if w < 600.0 {
        [w - 120.0, h - 304.0, w - 24.0, h - 24.0]
    } else {
        [w - 212.0, h - 196.0, w - 24.0, h - 24.0]
    };
    [pad, buttons]
}

#[test]
fn the_runner_stands_whole_and_left_of_the_screen_centre() {
    for viewport in [LANDSCAPE, PORTRAIT] {
        let sim = start_on(viewport);
        let [left, top, right, bottom] = runner(&sim, viewport);
        assert!(
            right < viewport.x / 2.0,
            "{viewport}: the centre looks at the body, which ends at x {right}"
        );
        assert!(
            left > 0.0 && top > 0.0 && bottom < viewport.y,
            "{viewport}: the body is cut off: {left} {top} {bottom}"
        );
    }
}

#[test]
fn the_runner_is_clear_of_the_touch_controls() {
    for viewport in [LANDSCAPE, PORTRAIT] {
        let sim = start_on(viewport);
        let [left, top, right, bottom] = runner(&sim, viewport);
        for [l, t, r, b] in controls(viewport) {
            assert!(
                right < l || left > r || bottom < t || top > b,
                "{viewport}: the body [{left} {top} {right} {bottom}] is under a control [{l} {t} {r} {b}]"
            );
        }
    }
}

#[test]
fn the_runner_is_larger_and_the_eye_lower_than_under_the_old_orbit() {
    // The orbit this camera replaced: 4.8 m from the body, 64 degrees of vertical
    // field of view, the eye 2.11 m above the feet.
    let old_share = HEIGHT / (2.0 * 4.8 * 32.0f32.to_radians().tan());
    for viewport in [LANDSCAPE, PORTRAIT] {
        let sim = start_on(viewport);
        let [_, top, _, bottom] = runner(&sim, viewport);
        let share = (bottom - top) / viewport.y;
        assert!(share > old_share, "{viewport}: {share} of the height");
        let above = eye(&sim).y - feet(&sim).y;
        assert!(
            above > HEIGHT / 2.0 && above < HEIGHT,
            "{viewport}: the eye is {above} m above the feet"
        );
    }
}

#[test]
fn a_rotated_phone_reframes_without_a_jump() {
    let mut sim = start_on(LANDSCAPE);
    sim.viewport(PORTRAIT.x, PORTRAIT.y);
    let mut before = eye(&sim);
    for _ in 0..360 {
        sim.run(TICK_MS);
        let now = eye(&sim);
        assert!(now.distance(before) < 0.05, "the eye jumped {before} to {now}");
        before = now;
    }
    let view = camera::view(sim.world());
    assert!((view.boom - tune::PORTRAIT.boom).abs() < 0.01, "{view:?}");
    assert!((view.fov - tune::PORTRAIT.fov).abs() < 0.1, "{view:?}");
}

#[test]
fn an_unknown_viewport_is_landscape_and_the_first_report_places_the_framing() {
    #[track_caller]
    fn assert_framed(sim: &Game, framing: camera::Framing) {
        let view = camera::view(sim.world());
        assert_eq!(
            (view.boom, view.shoulder, view.elevation, view.fov),
            (framing.boom, framing.shoulder, framing.elevation, framing.fov)
        );
    }
    let mut sim = start();
    assert_framed(&sim, tune::LANDSCAPE);
    sim.viewport(PORTRAIT.x, PORTRAIT.y);
    sim.run(TICK_MS);
    assert_framed(&sim, tune::PORTRAIT);
}
