//! What a player sees of the camera: where the runner stands on the screen, how a
//! finger turns the view, and how the view follows the run.
use exact_beat_em_up_logic::body::{self, Action, Parkour, HEIGHT, RADIUS};
use exact_beat_em_up_logic::camera::{self, tune};
use exact_beat_em_up_logic::{gait, park, ExactBeatEmUp, Options, ARENA_MODE};
use exact_game::*;
use exact_game_physics::{self as physics, CapsuleController, Collider, Shape};

type Game = Sim<ExactBeatEmUp>;

// A little over one 120 Hz tick: each `run` of it advances exactly one.
const TICK_MS: f64 = 8.334;
const LANDSCAPE: Vec2 = Vec2::new(932.0, 430.0);
const PORTRAIT: Vec2 = Vec2::new(430.0, 932.0);

fn start() -> Game {
    let mut sim = exact_game::sim_with_art!(Options {
        seed: 7,
        mode: ARENA_MODE,
        ..Options::default()
    })
    .unwrap();
    sim.run(25.0);
    sim
}

// The obstacle park, where `start` is open ground.
fn start_in_the_park() -> Game {
    let mut sim = exact_game::sim_with_art!(Options {
        seed: 7,
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

fn centre(sim: &Game) -> Vec3 {
    sim.get::<Transform>("player").unwrap().position
}

fn action(sim: &Game) -> Action {
    sim.get::<Parkour>("player").unwrap().action.clone()
}

// Advance until `done`, for `ms` at most. Returns whether it came.
fn run_until(sim: &mut Game, ms: f64, done: impl Fn(&Game) -> bool) -> bool {
    for _ in 0..(ms / TICK_MS) as u32 {
        sim.run(TICK_MS);
        if done(sim) {
            return true;
        }
    }
    false
}

// A solid box, as the park's blocks are.
fn wall(sim: &mut Game, at: Vec3, half: Vec3) -> Entity {
    sim.world_mut().spawn((
        Transform::at(at.x, at.y, at.z),
        Collider {
            shape: Shape::Box { half },
            layer: park::SOLID,
            ..Collider::default()
        },
    ))
}

// Whether the lens, with the near plane inside it, is out of everything solid.
fn lens_is_clear(sim: &Game) -> bool {
    let eye = eye(sim);
    physics::overlap(
        sim.world(),
        &Shape::Sphere { radius: 0.15 },
        Transform::at(eye.x, eye.y, eye.z),
        park::SOLID,
    )
    .is_empty()
}

// The camera's rotation without the view's own turn and tilt: what shakes it.
fn shake(sim: &Game) -> Quat {
    let view = camera::view(sim.world());
    let steady = Quat::from_rotation_y(view.yaw) * Quat::from_rotation_x(-view.elevation);
    steady.inverse() * sim.get::<Transform>("camera").unwrap().rotation
}

// How far the shake turns the view axis, degrees.
fn shaken(sim: &Game) -> f32 {
    (shake(sim) * Vec3::Z).angle_between(Vec3::Z).to_degrees()
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

// The way a view of this yaw looks, level.
fn ahead(yaw: f32) -> Vec3 {
    Quat::from_rotation_y(yaw) * -Vec3::Z
}

// A thumb on the move pad, `x` to the right and `z` down the screen, of a full push.
fn stick(sim: &mut Game, phase: PointerPhase, x: f32, z: f32) {
    sim.input(InputEvent::Control {
        name: "move".into(),
        id: 1,
        phase,
        x: 100.0 + 60.0 * x,
        y: 100.0 + 60.0 * z,
        at_ms: 0.0,
        player: 0,
    });
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
        touch(
            sim,
            PointerPhase::Move,
            dx / ticks as f32,
            dy / ticks as f32,
        );
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
        assert!(
            now.distance(before) < 0.05,
            "the eye jumped {before} to {now}"
        );
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
            (
                framing.boom,
                framing.shoulder,
                framing.elevation,
                framing.fov
            )
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
    sim.key_down("KeyD");
    sim.run(300.0);
    drag(&mut sim, tune::TAP_SLOP - 1.0, 0.0, 2);
    let tapped = yaw(&sim);
    sim.run(200.0);
    assert!(
        (yaw(&sim) - tapped).abs() > 0.05,
        "a tap stopped the follow"
    );
    sim.key_up("KeyD");
    sim.run(1000.0);

    touch(&mut sim, PointerPhase::Down, 0.0, 0.0);
    let from = yaw(&sim);
    let mut before = from;
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
    let turned = math::wrap_angle(yaw(&sim) - from);
    assert!(
        (turned + (40.0 - tune::TAP_SLOP) * tune::YAW_PER_POINT).abs() < 1e-4,
        "the points under the slop are dropped: {turned}"
    );
}

#[test]
fn hover_and_action_controls_do_not_turn_but_a_second_finger_can() {
    let mut sim = start();
    touch(&mut sim, PointerPhase::Move, 90.0, 20.0);
    sim.run(100.0);
    assert_eq!(yaw(&sim), 0.0, "a hover turned the view");

    stick(&mut sim, PointerPhase::Down, 0.0, 0.0);
    stick(&mut sim, PointerPhase::Move, 1.0, 0.0);
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

    // Far past the limit, and back at once: the finger does not lift or wait.
    touch(&mut sim, PointerPhase::Down, 0.0, 0.0);
    slide(&mut sim, 0.0, 10_000.0, 20);
    assert_eq!(elevation(&sim), tune::ELEVATION_MAX);
    slide(&mut sim, 0.0, -20.0, 4);
    touch(&mut sim, PointerPhase::Up, 0.0, 0.0);
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
        (later.yaw - stopped.yaw).abs() < 1e-3
            && (later.elevation - stopped.elevation).abs() < 1e-3,
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
        assert!(
            held.distance(offset) < 1e-4,
            "the eye is {held} from the body"
        );
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

#[test]
fn a_held_stick_keeps_a_straight_path_while_the_view_swings_behind() {
    for (key, direction) in [
        ("KeyW", -Vec3::Z),
        ("KeyS", Vec3::Z),
        ("KeyA", -Vec3::X),
        ("KeyD", Vec3::X),
    ] {
        let mut sim = start();
        let from = feet(&sim);
        sim.key_down(key);
        let mut before = yaw(&sim);
        for _ in 0..360 {
            sim.run(TICK_MS);
            let now = yaw(&sim);
            assert!(
                math::wrap_angle(now - before).abs() <= tune::FOLLOW_RATE_MAX / 120.0 * 1.01,
                "{key}: the view snapped"
            );
            before = now;
        }
        let walked = (feet(&sim) - from) * Vec3::new(1.0, 0.0, 1.0);
        assert!(walked.dot(direction) > 6.0, "{key}: {walked}");
        assert!(
            walked.cross(direction).length() < 0.01,
            "{key}: the path curved: {walked}"
        );
        assert!(
            ahead(before).dot(direction) > 8.0f32.to_radians().cos(),
            "{key}: the view looks along {}",
            ahead(before)
        );
    }
}

#[test]
fn the_follow_turns_without_a_step_and_does_not_pass_the_travel() {
    let mut sim = start();
    let behind_the_run = -std::f32::consts::FRAC_PI_2;
    let (mut before, mut rate) = (yaw(&sim), 0.0);
    let mut turn = |sim: &mut Game, what: &str| {
        sim.run(TICK_MS);
        let now = yaw(sim);
        let moved = math::wrap_angle(now - before);
        assert!(
            (moved - rate).abs() <= tune::FOLLOW_ACCEL / (120.0 * 120.0) * 1.05,
            "{what}: the turn rate stepped from {rate} to {moved} rad per tick"
        );
        (before, rate) = (now, moved);
        now
    };
    sim.key_down("KeyD");
    for _ in 0..300 {
        let now = turn(&mut sim, "running");
        assert!(
            now > behind_the_run - 1e-3,
            "the view passed the travel: {now}"
        );
    }
    sim.key_up("KeyD");
    sim.key_down("KeyA");
    for _ in 0..120 {
        turn(&mut sim, "turning back");
    }
    sim.key_up("KeyA");
    for _ in 0..120 {
        turn(&mut sim, "stopping");
    }
}

#[test]
fn the_view_rests_when_the_stick_is_released() {
    let mut sim = start();
    sim.hold("KeyD", 400.0);
    sim.run(400.0);
    let resting = yaw(&sim);
    assert!(
        resting < -0.05 && resting > -1.2,
        "the view was part of the way behind the run: {resting}"
    );
    sim.run(2000.0);
    assert_eq!(yaw(&sim), resting, "the view moved with the stick at rest");
}

#[test]
fn a_wobbling_thumb_keeps_the_line() {
    let mut sim = start();
    let from = feet(&sim);
    stick(&mut sim, PointerPhase::Down, 0.0, 0.0);
    for tick in 0..360 {
        let seconds = tick as f32 / 120.0;
        let wobble = 6.0f32.to_radians() * (std::f32::consts::TAU * 2.0 * seconds).sin();
        stick(&mut sim, PointerPhase::Move, wobble.cos(), wobble.sin());
        sim.run(TICK_MS);
        let walked = feet(&sim) - from;
        assert!(
            walked.z.abs() < 0.1,
            "the path left its line after {seconds} s: {walked}"
        );
    }
    assert!(feet(&sim).x - from.x > 6.0);
    assert!(
        ahead(yaw(&sim)).dot(Vec3::X) > 0.9,
        "the view swung behind the run"
    );
}

#[test]
fn a_new_direction_is_read_against_the_screen() {
    let mut sim = start();
    sim.hold("KeyD", 2500.0);
    sim.key_down("KeyW");
    let view = ahead(yaw(&sim));
    assert!(
        view.dot(Vec3::X) > 0.9,
        "the view is behind the run: {view}"
    );
    sim.run(200.0);
    let from = feet(&sim);
    sim.run(500.0);
    let walked = (feet(&sim) - from).normalize();
    assert!(
        walked.dot(view) > 0.999,
        "forward went {walked}, and the view looks {view}"
    );
}

#[test]
fn a_finger_owns_the_view_then_the_follow_eases_back() {
    let mut sim = start();
    sim.key_down("KeyD");
    sim.run(450.0);
    touch(&mut sim, PointerPhase::Down, 0.0, 0.0);
    slide(&mut sim, 120.0, 0.0, 4);
    sim.run(500.0);
    let manual = yaw(&sim);
    sim.run(1000.0);
    assert_eq!(yaw(&sim), manual, "a held finger owns the view");
    touch(&mut sim, PointerPhase::Up, 0.0, 0.0);
    sim.run(f64::from(tune::LOOK_HOLD) * 1000.0 - 50.0);
    assert_eq!(
        yaw(&sim),
        manual,
        "the view is the player's after the release"
    );
    sim.run(2500.0);
    let from = feet(&sim);
    sim.run(500.0);
    let travel = ((feet(&sim) - from) * Vec3::new(1.0, 0.0, 1.0)).normalize();
    assert!(
        ahead(yaw(&sim)).dot(travel) > 8.0f32.to_radians().cos(),
        "the follow did not come back behind the run"
    );
}

#[test]
fn tilt_goes_home_only_while_moving() {
    let mut sim = start();
    let elevation = |sim: &Game| camera::view(sim.world()).elevation;
    let level = elevation(&sim);
    drag(&mut sim, 0.0, 100.0, 10);
    sim.run(500.0);
    let tilted = elevation(&sim);
    assert!(tilted > level + 0.2);
    sim.run(3000.0);
    assert_eq!(
        elevation(&sim),
        tilted,
        "a player who stands and looks keeps the view"
    );
    sim.hold("KeyW", 6000.0);
    assert!(
        (elevation(&sim) - level).abs() < 0.02,
        "the tilt stayed at {} during a run",
        elevation(&sim)
    );
}

#[test]
fn the_runner_falls_back_at_a_sprint_and_returns_at_a_stand() {
    let mut sim = start();
    let back = |sim: &Game| ((eye(sim) - feet(sim)) * Vec3::new(1.0, 0.0, 1.0)).length();
    let standing = back(&sim);
    sim.key_down("ShiftLeft");
    sim.key_down("KeyW");
    sim.run(2000.0);
    let trailing = back(&sim) - standing;
    assert!(
        trailing > 0.8 * gait::SPRINT * tune::FOCUS_LAG && trailing < tune::FOCUS_LEASH + 1e-3,
        "the eye is {trailing} m further back at a sprint"
    );
    sim.key_up("KeyW");
    sim.key_up("ShiftLeft");
    sim.run(1500.0);
    assert!((back(&sim) - standing).abs() < 0.01);
}

#[test]
fn a_jump_does_not_bob_the_view() {
    let mut sim = start();
    let (eye_rest, body_rest) = (eye(&sim).y, centre(&sim).y);
    sim.tap("Space");
    let (mut eye_rise, mut body_rise) = (0.0f32, 0.0f32);
    for _ in 0..120 {
        sim.run(TICK_MS);
        eye_rise = eye_rise.max(eye(&sim).y - eye_rest);
        body_rise = body_rise.max(centre(&sim).y - body_rest);
    }
    assert!(body_rise > 0.9 * body::JUMP, "the jump rose {body_rise} m");
    assert!(
        eye_rise < body_rise / 2.0,
        "the eye rose {eye_rise} m with a jump of {body_rise} m"
    );
}

#[test]
fn a_fall_tilts_the_view_down_and_keeps_the_feet_in_frame() {
    for viewport in [LANDSCAPE, PORTRAIT] {
        let mut sim = start_on(viewport);
        let level = camera::view(sim.world()).elevation;
        sim.post("goto 0 3.5 0 180");
        let mut fast = 0;
        for _ in 0..240 {
            sim.run(TICK_MS);
            let falling = -sim.get::<CapsuleController>("player").unwrap().velocity.y;
            if action(&sim) != Action::Air || falling < 6.0 {
                continue;
            }
            fast += 1;
            let elevation = camera::view(sim.world()).elevation;
            assert!(
                elevation > level + 0.02,
                "{viewport}: the view is level in a fall of {falling} m/s"
            );
            let at = screen(&sim, viewport, feet(&sim));
            assert!(
                at.cmpgt(Vec2::ZERO).all() && at.cmplt(viewport).all(),
                "{viewport}: the feet are off the screen at {at}"
            );
        }
        assert!(fast > 10, "{viewport}: the fall never reached 6 m/s");
    }
}

#[test]
fn a_landing_dips_the_rig_and_it_recovers() {
    let mut sim = start();
    // The height of the point the boom hangs from.
    let anchor = |sim: &Game| {
        let view = camera::view(sim.world());
        eye(sim).y - view.boom * view.elevation.sin()
    };
    let rest = anchor(&sim);
    sim.post("goto 0 2 0 180");
    assert!(run_until(&mut sim, 500.0, |sim| action(sim) == Action::Air));
    assert!(run_until(&mut sim, 2000.0, |sim| action(sim) != Action::Air));
    let mut dip = 0.0f32;
    for _ in 0..60 {
        sim.run(TICK_MS);
        dip = dip.max(rest - anchor(&sim));
    }
    assert!(
        dip > 0.01 && dip < 0.08,
        "the landing dipped the rig {dip} m"
    );
    assert!(
        (anchor(&sim) - rest).abs() < 0.005,
        "the rig is not back half a second after the landing"
    );
}

#[test]
fn a_sprint_widens_the_view() {
    let mut sim = start();
    let fov = |sim: &Game| sim.get::<Camera>("camera").unwrap().fov_y_degrees;
    sim.key_down("KeyW");
    sim.run(2000.0);
    let running = fov(&sim);
    assert!(
        (running - tune::LANDSCAPE.fov).abs() < 0.05,
        "a run changed the field of view to {running}"
    );
    sim.key_down("ShiftLeft");
    sim.run(2500.0);
    let sprinting = fov(&sim);
    assert!(
        sprinting > running + 1.0 && (sprinting - running - tune::SPRINT_FOV).abs() < 0.1,
        "the field of view is {sprinting} at a sprint and {running} at a run"
    );
}

#[test]
fn a_slide_lowers_the_eye_and_it_comes_back() {
    let mut sim = start();
    let above = |sim: &Game| eye(sim).y - feet(sim).y;
    let standing = above(&sim);
    sim.key_down("KeyW");
    sim.run(800.0);
    sim.tap("KeyC");
    sim.run(250.0);
    assert!(matches!(action(&sim), Action::Slide(_)));
    let lowered = standing - above(&sim);
    assert!(
        lowered > 0.6 * tune::SLIDE_DROP && lowered < tune::SLIDE_DROP,
        "a slide lowered the eye {lowered} m"
    );
    sim.run(2000.0);
    assert!((above(&sim) - standing).abs() < 0.02);
}

#[test]
fn a_hang_is_seen_from_behind_and_above_and_a_shimmy_turns_the_view() {
    let mut sim = start_in_the_park();
    // Three metres south of the south face of `ledge-mid`, which is at z = -12.
    sim.post("goto -3 0 -9 180");
    sim.run(100.0);
    let level = camera::view(sim.world()).elevation;
    sim.key_down("KeyW");
    assert!(run_until(&mut sim, 4000.0, |sim| feet(sim).z - RADIUS
        + 12.0
        < 0.35));
    sim.tap("Space");
    let hanging = |sim: &Game| matches!(action(sim), Action::Hang(_));
    assert!(run_until(&mut sim, 900.0, hanging), "{:?}", action(&sim));
    sim.key_up("KeyW");
    sim.run(400.0);
    drag(&mut sim, 200.0, 0.0, 10);
    sim.run(4000.0);
    assert!(hanging(&sim));
    let view = camera::view(sim.world());
    assert!(
        ahead(view.yaw).dot(-Vec3::Z) > 20.0f32.to_radians().cos(),
        "the view is not behind the hanging body: it looks along {}",
        ahead(view.yaw)
    );
    assert!(
        view.elevation > level + 0.8 * tune::HANG_TILT,
        "the view does not look down on the edge: {view:?}"
    );

    sim.key_down("KeyA");
    sim.run(1200.0);
    assert!(hanging(&sim));
    let peek = math::wrap_angle(yaw(&sim) - view.yaw);
    assert!(
        peek > 0.5 * tune::HANG_PEEK && peek < 1.1 * tune::HANG_PEEK,
        "a shimmy to the left turned the view {peek} rad to the left"
    );
}

#[test]
fn a_ragdoll_is_centred_and_the_view_does_not_chase_the_tumble() {
    let mut sim = start();
    // Face east, and send the heaviest roller at the body's back.
    sim.post("goto 0 0 0 90");
    sim.run(100.0);
    sim.post("roller 3 6");
    sim.key_down("KeyW");
    assert!(run_until(&mut sim, 2000.0, |sim| action(sim) == Action::Ragdoll));
    let from = centre(&sim);
    sim.run(1000.0);
    let tumbled = centre(&sim) - from;
    assert!(tumbled.x > 1.0, "the hit sent the body {tumbled}");
    let view = camera::view(sim.world());
    assert!(
        view.yaw.abs() < 0.05,
        "the view chased the tumble to yaw {}",
        view.yaw
    );
    assert!(view.shoulder < 0.1, "the tumble is off centre: {view:?}");
    assert!(
        view.boom > tune::LANDSCAPE.boom + 0.5 * tune::LIMP_BOOM,
        "{view:?}"
    );
}

#[test]
fn a_teleport_is_a_cut() {
    let mut sim = start();
    let offset = eye(&sim) - centre(&sim);
    sim.key_down("KeyD");
    sim.run(1500.0);
    sim.post("goto 15 0 -20 180");
    sim.run(TICK_MS);
    assert!(centre(&sim).distance(Vec3::new(15.0, body::HALF, -20.0)) < 0.1);
    let placed = Quat::from_rotation_y(yaw(&sim)) * offset;
    assert!(
        (eye(&sim) - centre(&sim)).distance(placed) < 0.05,
        "the camera swept after the body: it is {} from it",
        eye(&sim) - centre(&sim)
    );
}

#[test]
fn a_wall_beside_the_runner_closes_the_shoulder_and_keeps_the_boom() {
    let mut sim = start();
    // Its face is 0.5 m to the right of the body's axis.
    wall(&mut sim, Vec3::new(0.7, 2.0, 0.0), Vec3::new(0.2, 2.0, 6.0));
    sim.run(1000.0);
    let view = camera::view(sim.world());
    assert!(
        view.shoulder < 0.5 - tune::LENS + 1e-3,
        "the lens does not fit between the body and the wall: {view:?}"
    );
    assert!(view.boom > 0.95 * tune::LANDSCAPE.boom, "{view:?}");
    assert!(eye(&sim).x < 0.5 && lens_is_clear(&sim));
}

#[test]
fn a_wall_behind_the_runner_shortens_the_boom_at_once_and_it_lengthens_slowly() {
    let mut sim = start();
    let open = camera::view(sim.world()).boom;
    // Its face is 1.8 m behind the body's axis.
    let behind = wall(&mut sim, Vec3::new(0.0, 2.0, 2.0), Vec3::new(4.0, 2.0, 0.2));
    sim.run(TICK_MS);
    assert!(
        eye(&sim).z < 1.8 && lens_is_clear(&sim),
        "the lens was in the wall for a tick: {}",
        eye(&sim)
    );
    sim.run(500.0);
    assert!(eye(&sim).z < 1.8 && lens_is_clear(&sim));
    sim.world_mut().despawn(behind);
    let mut before = camera::view(sim.world()).boom;
    for _ in 0..480 {
        sim.run(TICK_MS);
        let now = camera::view(sim.world()).boom;
        assert!(
            now >= before && now - before < 0.05,
            "the boom went from {before} to {now} m in a tick"
        );
        before = now;
    }
    assert!(
        (before - open).abs() < 0.01,
        "the boom came back to {before} m"
    );
}

#[test]
fn a_short_boom_lifts_the_eye_over_the_head() {
    let mut sim = start();
    // Its face is 0.9 m behind the body's axis.
    wall(&mut sim, Vec3::new(0.0, 2.0, 1.1), Vec3::new(4.0, 2.0, 0.2));
    sim.run(1500.0);
    let view = camera::view(sim.world());
    assert!(view.boom < tune::CROWD_FAR, "{view:?}");
    let unlifted = tune::LANDSCAPE.height + view.boom * view.elevation.sin();
    let lifted = eye(&sim).y - feet(&sim).y - unlifted;
    assert!(
        lifted > 0.1 && lifted < tune::CROWD_LIFT + 1e-3,
        "a boom of {} m lifted the eye {lifted} m",
        view.boom
    );
    assert!(lens_is_clear(&sim));
}

#[test]
fn the_lens_stays_out_of_solid_geometry() {
    // Feet positions: beside a wall, in a concave corner, on a roof's edge, under
    // the ramp against its back wall, and on the stairs.
    let spots = [
        [2.0, 0.0, -11.5],
        [-21.0, 0.0, -14.0],
        [-16.0, 2.0, -10.0],
        [14.0, 0.0, -13.6],
        [6.0, 0.45, 6.0],
    ];
    // A full turn of the view, in points.
    let around = std::f32::consts::TAU / tune::YAW_PER_POINT;
    for [x, y, z] in spots {
        let mut sim = start_in_the_park();
        sim.post(format!("goto {x} {y} {z} 180"));
        sim.run(200.0);
        touch(&mut sim, PointerPhase::Down, 0.0, 0.0);
        // Level, from below and from above.
        for tilt in [0.0, -400.0, 800.0] {
            slide(&mut sim, 0.0, tilt, 20);
            for step in 0..240 {
                slide(&mut sim, around / 240.0, 0.0, 1);
                assert!(
                    lens_is_clear(&sim),
                    "at {x} {y} {z}, tilt {tilt}, step {step}: the lens is in a wall at {}",
                    eye(&sim)
                );
            }
        }
    }
}

#[test]
fn the_shake_at_a_stand_is_subtle_and_never_moves_the_eye() {
    let mut sim = start();
    let from = eye(&sim);
    let (mut least, mut most) = (f32::MAX, 0.0f32);
    for _ in 0..600 {
        sim.run(TICK_MS);
        assert!(eye(&sim).distance(from) < 0.001, "the shake moved the eye");
        least = least.min(shaken(&sim));
        most = most.max(shaken(&sim));
    }
    assert!(
        most > 0.05 && most < 0.4,
        "the view wanders {most} degrees at a stand"
    );
    assert!(most - least > 0.05, "the view is turned, and does not move");
}

#[test]
fn the_nod_is_locked_to_the_footfalls() {
    let mut sim = start();
    sim.key_down("ShiftLeft");
    sim.key_down("KeyW");
    sim.run(1000.0);
    // The mean pitch of the shake in twenty slices of a step. A step is half a
    // gait cycle. The slow handheld drift averages out of every slice.
    let mut slices = [(0.0f32, 0u32); 20];
    for _ in 0..480 {
        sim.run(TICK_MS);
        let phase = sim.get::<Pose>("stickman").unwrap().phase;
        let pitch = (shake(&sim) * -Vec3::Z).y.asin();
        let slice = &mut slices[((phase * 2.0).fract() * 20.0) as usize % 20];
        *slice = (slice.0 + pitch, slice.1 + 1);
    }
    let means = slices.map(|(sum, count)| sum / count as f32);
    let deepest = (0..20)
        .min_by(|&a, &b| means[a].total_cmp(&means[b]))
        .unwrap();
    let highest = means.iter().copied().fold(f32::MIN, f32::max);
    assert!(
        highest - means[deepest] > tune::STEP_PITCH,
        "the view does not nod with the steps: {means:?}"
    );
    // Slice `n` is `n / 40` of a cycle after a foot strikes.
    let after_strike = (deepest as f32 + 0.5) / 40.0;
    assert!(
        (after_strike - tune::STEP_LATE).abs() < 0.03,
        "the nod is deepest {after_strike} of a cycle after a strike"
    );
}

#[test]
fn a_hard_landing_shakes_the_view_and_the_shake_ends() {
    let mut sim = start();
    sim.post("goto 0 3.5 0 180");
    assert!(run_until(&mut sim, 500.0, |sim| action(sim) == Action::Air));
    assert!(run_until(&mut sim, 2000.0, |sim| action(sim) != Action::Air));
    assert_ne!(action(&sim), Action::Ragdoll);
    let mut most = 0.0f32;
    for _ in 0..36 {
        sim.run(TICK_MS);
        most = most.max(shaken(&sim));
    }
    assert!(
        most > 0.5 && most < 2.0,
        "the landing shook the view {most} degrees"
    );
    sim.run(500.0);
    for _ in 0..60 {
        sim.run(TICK_MS);
        assert!(shaken(&sim) < 0.4, "the view still shakes after 0.8 s");
    }
}

#[test]
fn a_knock_down_shakes_the_view() {
    let mut sim = start();
    sim.post("goto 0 0 0 90");
    sim.run(100.0);
    sim.post("roller 3 6");
    assert!(run_until(&mut sim, 2000.0, |sim| action(sim) == Action::Ragdoll));
    let mut most = 0.0f32;
    for _ in 0..24 {
        sim.run(TICK_MS);
        most = most.max(shaken(&sim));
    }
    assert!(
        most > 0.5 && most < 2.0,
        "the knock shook the view {most} degrees"
    );
}
