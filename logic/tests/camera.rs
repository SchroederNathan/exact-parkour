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

fn yaw(sim: &Game) -> f32 {
    camera::view(sim.world()).yaw
}

// A finger on the scene. A touch contact reports no mouse button.
fn touch(sim: &mut Game, phase: PointerPhase, dx: f32, dy: f32) {
    sim.input(InputEvent::Pointer {
        id: 9,
        phase,
        x: 200.0,
        y: 300.0,
        dx,
        dy,
        buttons: 0,
        at_ms: 0.0,
        player: 0,
    });
}

// Move the finger that is down by `dx`, `dy` points in `ticks` equal steps.
fn slide(sim: &mut Game, dx: f32, dy: f32, ticks: u32) {
    for _ in 0..ticks {
        touch(sim, PointerPhase::Move, dx / ticks as f32, dy / ticks as f32);
        sim.run(TICK_MS);
    }
}

// One whole contact: down, across, up.
fn drag(sim: &mut Game, dx: f32, dy: f32, ticks: u32) {
    touch(sim, PointerPhase::Down, 0.0, 0.0);
    slide(sim, dx, dy, ticks);
    touch(sim, PointerPhase::Up, 0.0, 0.0);
    sim.run(TICK_MS);
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

#[test]
fn a_drag_turns_the_view_by_exactly_its_length() {
    let mut sim = start();
    touch(&mut sim, PointerPhase::Down, 0.0, 0.0);
    slide(&mut sim, 2.0 * tune::TAP_SLOP, 0.0, 4);
    sim.run(500.0);
    let from = yaw(&sim);
    let turned = |sim: &Game| math::wrap_angle(yaw(sim) - from);

    slide(&mut sim, 200.0, 0.0, 120);
    sim.run(500.0);
    let slow = turned(&sim);
    assert!(
        (slow + 200.0 * tune::YAW_PER_POINT).abs() < 1e-3,
        "a swipe to the right looks right: {slow}"
    );
    slide(&mut sim, 200.0, 0.0, 6);
    sim.run(500.0);
    assert!(
        (turned(&sim) - 2.0 * slow).abs() < 1e-3,
        "a fast drag turns as far as a slow one"
    );
    slide(&mut sim, -400.0, 0.0, 60);
    sim.run(500.0);
    assert!(
        turned(&sim).abs() < 1e-3,
        "a finger back at its start has the view back at its start"
    );
}

#[test]
fn a_tap_does_not_take_the_view_and_passing_the_slop_does_not_jump() {
    let mut sim = start();
    drag(&mut sim, tune::TAP_SLOP - 1.0, 0.0, 2);
    sim.run(500.0);
    assert_eq!(yaw(&sim), 0.0, "a tap turned the view");

    touch(&mut sim, PointerPhase::Down, 0.0, 0.0);
    let mut before = yaw(&sim);
    for _ in 0..40 {
        slide(&mut sim, 1.0, 0.0, 1);
        let now = yaw(&sim);
        assert!(
            (now - before).abs() <= tune::YAW_PER_POINT * 1.001,
            "a point of drag turned the view by {} points",
            (now - before).abs() / tune::YAW_PER_POINT
        );
        before = now;
    }
    sim.run(500.0);
    assert!(
        (yaw(&sim) + (40.0 - tune::TAP_SLOP) * tune::YAW_PER_POINT).abs() < 1e-4,
        "the points under the slop are dropped: {}",
        yaw(&sim)
    );
}

#[test]
fn hover_and_action_controls_do_not_turn_but_a_second_finger_can() {
    let mut sim = start();
    touch(&mut sim, PointerPhase::Move, 90.0, 20.0);
    sim.run(100.0);
    assert_eq!(yaw(&sim), 0.0, "a hover turned the view");

    for (phase, x) in [(PointerPhase::Down, 40.0), (PointerPhase::Move, 400.0)] {
        sim.input(InputEvent::Control {
            name: "move".into(),
            id: 1,
            phase,
            x,
            y: 40.0,
            at_ms: 0.0,
            player: 0,
        });
    }
    sim.run(TICK_MS);
    assert_eq!(yaw(&sim), 0.0, "a finger on the move pad turned the view");
    let from = yaw(&sim);
    drag(&mut sim, 160.0, 0.0, 4);
    sim.run(500.0);
    let turned = math::wrap_angle(yaw(&sim) - from);
    assert!(
        (turned + (160.0 - tune::TAP_SLOP) * tune::YAW_PER_POINT).abs() < 1e-3,
        "a second finger on the scene turns the view: {turned}"
    );

    let from = yaw(&sim);
    touch(&mut sim, PointerPhase::Down, 0.0, 0.0);
    touch(&mut sim, PointerPhase::Move, 200.0, 0.0);
    touch(&mut sim, PointerPhase::Cancel, 0.0, 0.0);
    sim.run(100.0);
    assert_eq!(yaw(&sim), from, "a cancelled contact turned the view");
}

#[test]
fn tilt_stops_at_its_limits_and_leaves_them_at_once() {
    let mut sim = start();
    let elevation = |sim: &Game| camera::view(sim.world()).elevation;
    let level = elevation(&sim);
    drag(&mut sim, 0.0, 40.0, 8);
    sim.run(500.0);
    assert!(elevation(&sim) > level, "a swipe down looks down");

    drag(&mut sim, 0.0, 10_000.0, 20);
    sim.run(500.0);
    assert_eq!(elevation(&sim), tune::ELEVATION_MAX);
    drag(&mut sim, 0.0, -(tune::TAP_SLOP + 20.0), 4);
    sim.run(500.0);
    assert!(
        (elevation(&sim) - tune::ELEVATION_MAX + 20.0 * tune::TILT_PER_POINT).abs() < 1e-3,
        "the overshoot wound up: {}",
        elevation(&sim)
    );
    drag(&mut sim, 0.0, -20_000.0, 20);
    sim.run(500.0);
    assert_eq!(elevation(&sim), tune::ELEVATION_MIN);
    assert!(sim.get::<Transform>("camera").unwrap().rotation.is_finite());
}

#[test]
fn the_view_stops_with_the_finger() {
    let mut sim = start();
    drag(&mut sim, 300.0, 60.0, 30);
    sim.run(100.0);
    let stopped = camera::view(sim.world());
    assert!(
        (stopped.yaw + (300.0 - tune::TAP_SLOP) * tune::YAW_PER_POINT).abs() < 0.02,
        "{stopped:?}"
    );
    sim.run(1000.0);
    let later = camera::view(sim.world());
    assert!(
        (later.yaw - stopped.yaw).abs() < 1e-3 && (later.elevation - stopped.elevation).abs() < 1e-3,
        "the view kept turning: {stopped:?} to {later:?}"
    );
}

#[test]
fn a_drag_steers_a_held_stick() {
    let mut sim = start();
    sim.key_down("KeyW");
    sim.run(300.0);
    let quarter = std::f32::consts::FRAC_PI_2 / tune::YAW_PER_POINT;
    drag(&mut sim, quarter + tune::TAP_SLOP, 0.0, 30);
    sim.run(500.0);
    let from = feet(&sim);
    sim.run(500.0);
    let walked = feet(&sim) - from;
    assert!(
        walked.x > 0.5 && walked.z.abs() < 0.01,
        "forward is the way the view looks: {walked}"
    );
}

#[test]
fn a_save_resumes_the_same_camera() {
    let mut sim = start_on(PORTRAIT);
    drag(&mut sim, 180.0, 45.0, 10);
    sim.hold("KeyW", 150.0);
    touch(&mut sim, PointerPhase::Down, 0.0, 0.0);
    slide(&mut sim, 50.0, -10.0, 3);
    let saved = sim.save().unwrap();
    let mut restored = start();
    restored.restore(&saved).unwrap();
    for game in [&mut sim, &mut restored] {
        slide(game, 30.0, 0.0, 3);
        game.hold("KeyD", 200.0);
    }
    assert_eq!(
        sim.get::<Transform>("camera"),
        restored.get::<Transform>("camera")
    );
    assert_eq!(sim.world().hash(), restored.world().hash());
}

#[test]
fn the_debug_camera_command_holds_a_fixed_offset_until_a_drag() {
    let mut sim = start();
    let offset = Vec3::new(4.6, 0.4, 0.9);
    let centre = |sim: &Game| sim.get::<Transform>("player").unwrap().position;
    sim.post("camera 4.6 0.4 0.9");
    sim.run(TICK_MS);
    let from = centre(&sim);
    sim.key_down("KeyW");
    for _ in 0..60 {
        sim.run(TICK_MS);
        let held = eye(&sim) - centre(&sim);
        assert!(held.distance(offset) < 1e-4, "the eye is {held} from the body");
    }
    sim.key_up("KeyW");
    let pose = *sim.get::<Transform>("camera").unwrap();
    let to_body = (centre(&sim) - pose.position).normalize();
    assert!((pose.rotation * -Vec3::Z).dot(to_body) > 0.9999);
    let walked = (centre(&sim) - from) * Vec3::new(1.0, 0.0, 1.0);
    let away = -(offset * Vec3::new(1.0, 0.0, 1.0)).normalize();
    assert!(
        walked.length() > 0.5 && walked.normalize().dot(away) > 0.999,
        "forward is away from the debug camera: {walked}"
    );

    drag(&mut sim, 40.0, 0.0, 4);
    sim.run(500.0);
    let [.., right, _] = runner(&sim, LANDSCAPE);
    assert!(
        right < LANDSCAPE.x / 2.0,
        "a drag gives the camera back to the rig"
    );
}
