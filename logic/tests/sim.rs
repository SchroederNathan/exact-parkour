use exact_beat_em_up_logic::locomotion::Stride;
use exact_beat_em_up_logic::gait::{RUN, SPRINT};
use exact_beat_em_up_logic::stickman::{self, MODEL};
use exact_beat_em_up_logic::{body, ExactBeatEmUp, Options};
use exact_game::animation::joint_matrix;
use exact_game::{Animator, Pose, Sim, Transform, Vec3};
use exact_game_physics::CapsuleController;

// A little over one 120 Hz tick: each `run` of it advances exactly one.
const TICK_MS: f64 = 8.334;

fn start() -> Sim<ExactBeatEmUp> {
    let mut game = Sim::new(Options {
        seed: 7,
        ..Options::default()
    })
    .unwrap();
    // A few ticks, so the first pose exists.
    game.run(25.0);
    game
}

fn velocity(game: &Sim<ExactBeatEmUp>) -> Vec3 {
    game.get::<CapsuleController>("player").unwrap().velocity
}

fn planar_speed(game: &Sim<ExactBeatEmUp>) -> f32 {
    let velocity = velocity(game);
    Vec3::new(velocity.x, 0.0, velocity.z).length()
}

// The collision solver returns the speed it achieved, a few ulps from the request.
#[track_caller]
fn assert_speed(game: &Sim<ExactBeatEmUp>, expected: f32) {
    let speed = planar_speed(game);
    assert!((speed - expected).abs() < 1e-3, "speed {speed}, expected {expected}");
}

// Height of the feet above the ground plane.
fn feet_height(game: &Sim<ExactBeatEmUp>) -> f32 {
    game.global_position("player").unwrap().y - body::HALF
}

// The visual's world pose: the upright capsule's position and the child's turn.
fn visual(game: &Sim<ExactBeatEmUp>) -> Transform {
    let child = *game.get::<Transform>("stickman").unwrap();
    Transform {
        position: game.global_position("player").unwrap() + child.position,
        ..child
    }
}

// World positions of a point on each foot: its ankle joint, or its toe.
fn feet(game: &Sim<ExactBeatEmUp>, toe: bool) -> [Vec3; 2] {
    let w = game.world();
    let model = w.model(MODEL).unwrap();
    let pose = game.get::<Pose>("stickman").unwrap();
    let body = visual(game);
    let rig = stickman::rig(1.8);
    ["foot_l", "foot_r"].map(|name| {
        let node = model.nodes.iter().position(|n| n.name == name).unwrap() as u32;
        let bone = rig.bones().iter().find(|b| b.name == name).unwrap();
        let offset = if toe { bone.tail - bone.head } else { Vec3::ZERO };
        let local = joint_matrix(model, &pose.local, node).transform_point3(offset);
        body.position + body.rotation * local
    })
}
fn ankles(game: &Sim<ExactBeatEmUp>) -> [Vec3; 2] {
    feet(game, false)
}

// Hold `keys` for `ms`, and report how many metres planted feet slid along the
// ground, and how many the body went. A foot is planted while its toe is at rest
// height: flat on the ground, and after that while the heel peels up about it. The
// toe's first few millimetres of lift-off count too, so a clean gait reads a little
// above zero.
fn slip(game: &mut Sim<ExactBeatEmUp>, keys: &[&str], ms: f64) -> (f32, f32) {
    const PLANTED: f32 = 0.044;
    for key in keys {
        game.key_down(key);
    }
    let (mut slid, mut travelled) = (0.0, 0.0);
    let mut before = (feet(game, true), game.global_position("player").unwrap());
    let ticks = (ms * 120.0 / 1000.0) as u32;
    for _ in 0..ticks {
        game.run(TICK_MS);
        let now = (feet(game, true), game.global_position("player").unwrap());
        travelled += (now.1 - before.1).length();
        let planted = (0..2)
            .filter(|&i| before.0[i].y < PLANTED && now.0[i].y < PLANTED)
            .map(|i| {
                let d = now.0[i] - before.0[i];
                Vec3::new(d.x, 0.0, d.z).length()
            })
            .fold(f32::INFINITY, f32::min);
        if planted.is_finite() {
            slid += planted;
        }
        before = now;
    }
    for key in keys {
        game.key_up(key);
    }
    (slid, travelled)
}

#[test]
fn movement_accelerates_brakes_and_jumps() {
    let mut game = start();
    game.hold("KeyW", 100.0);
    let moving = game.global_position("player").unwrap();
    assert!(moving.z < -0.05);
    assert!(velocity(&game).z < 0.0);

    game.run(200.0);
    let released = game.global_position("player").unwrap();
    assert!(released.z < moving.z);
    let stopped = game.global_position("player").unwrap();
    game.run(100.0);
    assert_eq!(game.global_position("player").unwrap(), stopped);

    game.tap("Space");
    game.run(100.0);
    assert!(feet_height(&game) > 0.1);
}

#[test]
fn top_speed_arrives_within_a_tenth_of_a_second() {
    let mut game = start();
    game.hold("KeyD", 100.0);
    assert_speed(&game, RUN);
}

#[test]
fn shift_sprints_and_releasing_it_returns_to_a_run() {
    let mut game = start();
    game.key_down("KeyW");
    game.run(400.0);
    assert_speed(&game, RUN);
    game.key_down("ShiftLeft");
    game.run(400.0);
    assert_speed(&game, SPRINT);
    game.key_up("ShiftLeft");
    game.run(400.0);
    assert_speed(&game, RUN);
}

#[test]
fn the_touch_buttons_hold_their_actions() {
    use exact_game::{InputEvent, PointerPhase};
    let control = |name: &str, phase| InputEvent::Control {
        name: name.into(),
        id: 2,
        phase,
        x: 0.0,
        y: 0.0,
        at_ms: 0.0,
        player: 0,
    };
    let mut game = start();
    game.input(control("jump", PointerPhase::Down));
    game.input(control("jump", PointerPhase::Up));
    game.run(100.0);
    assert!(feet_height(&game) > 0.1);

    // Sprint lasts exactly as long as the finger is down.
    let mut game = start();
    game.key_down("KeyW");
    game.input(control("sprint", PointerPhase::Down));
    game.run(400.0);
    assert_speed(&game, SPRINT);
    game.input(control("sprint", PointerPhase::Up));
    game.run(400.0);
    assert_speed(&game, RUN);
}

#[test]
fn diagonal_input_keeps_planar_speed_bounded() {
    let mut game = start();
    game.key_down("KeyW");
    game.key_down("KeyD");
    game.run(1000.0);
    assert!(planar_speed(&game) <= RUN + 0.0001);
}

#[test]
fn the_body_faces_where_it_is_going() {
    for (key, heading) in [
        ("KeyW", -Vec3::Z),
        ("KeyS", Vec3::Z),
        ("KeyA", -Vec3::X),
        ("KeyD", Vec3::X),
    ] {
        let mut game = start();
        game.hold(key, 500.0);
        // The rig's rest pose faces +Z.
        let facing = game.get::<Transform>("stickman").unwrap().rotation * Vec3::Z;
        assert!(facing.dot(heading) > 0.99, "{key}: facing {facing}");
    }
}

#[test]
fn planted_feet_stay_planted_at_every_pace() {
    // South of the start the park is open ground.
    for keys in [&["KeyS"][..], &["KeyS", "ShiftLeft"][..]] {
        let mut game = start();
        game.world_mut().require_mut::<body::Parkour>("player").yaw = 0.0;
        game.run(25.0);
        // From a stand through full acceleration: the first step pushes off a
        // planted foot.
        let (slid, went) = slip(&mut game, keys, 300.0);
        assert!(slid < 0.2, "{keys:?}: starting slid {slid} m over {went} m");
        // At speed.
        let (slid, went) = slip(&mut game, keys, 1800.0);
        assert!(slid < 0.07 * went, "{keys:?}: slid {slid} m over {went} m");
        // Letting go: the body brakes within a stride and the legs close into the
        // stance with one step; the foot left out in front comes back under the hips.
        let (slid, went) = slip(&mut game, &[], 700.0);
        assert!(slid < 0.3, "{keys:?}: stopping slid {slid} m over {went} m");
        assert_eq!(game.get::<Stride>("stickman").unwrap().pace, 0.0);
    }
}

#[test]
fn a_touch_stick_walks_at_every_speed_without_sliding() {
    use exact_game::{InputEvent, PointerPhase};
    let control = |phase, y: f32| InputEvent::Control {
        name: "move".into(),
        id: 1,
        phase,
        x: 48.0,
        y,
        at_ms: 0.0,
        player: 0,
    };
    // The stick is unit-clamped at 60 points from where the finger landed.
    for points in [6.0, 12.0, 20.0, 30.0, 42.0, 54.0] {
        let mut game = start();
        game.input(control(PointerPhase::Down, 48.0));
        game.input(control(PointerPhase::Move, 48.0 - points));
        game.run(600.0);
        let speed = planar_speed(&game);
        assert!(speed > 0.0 && speed < RUN, "stick at {points}: {speed} m/s");
        let (slid, went) = slip(&mut game, &[], 2500.0);
        assert!(
            slid < 0.07 * went,
            "at {speed} m/s planted feet slid {slid} m over {went} m"
        );
    }
}

#[test]
fn legs_never_jump_between_ticks() {
    let mut game = start();
    let mut worst: f32 = 0.0;
    let mut before = ankles(&game);
    let mut step = |game: &mut Sim<ExactBeatEmUp>, ms: f64| {
        for _ in 0..(ms * 120.0 / 1000.0) as u32 {
            game.run(TICK_MS);
            let body = velocity(game).length() / 120.0;
            let now = ankles(game);
            for i in 0..2 {
                // Motion of the ankle beyond the body's own.
                let moved = ((now[i] - before[i]).length() - body).max(0.0);
                worst = worst.max(moved);
            }
            before = now;
        }
    };
    // Start, stop, start into a sprint, reverse, jump while moving, land, stop.
    game.key_down("KeyW");
    step(&mut game, 300.0);
    game.key_up("KeyW");
    step(&mut game, 400.0);
    game.key_down("KeyD");
    game.key_down("ShiftLeft");
    step(&mut game, 500.0);
    game.key_up("KeyD");
    game.key_down("KeyA");
    step(&mut game, 500.0);
    game.tap("Space");
    step(&mut game, 1500.0);
    game.key_up("KeyA");
    game.key_up("ShiftLeft");
    step(&mut game, 600.0);
    // A sprinting swing foot passes the body at about its speed (0.05 m a tick at
    // 120 Hz); a snap between poses would be several times that.
    assert!(worst < 0.09, "an ankle moved {worst} m in one tick");
}

#[test]
fn stopping_settles_into_the_stance_and_jumping_takes_to_the_air() {
    let mut game = start();
    game.hold("KeyW", 600.0);
    assert!(game.get::<Stride>("stickman").unwrap().pace > RUN - 0.01);
    game.run(160.0);
    // Braked to a stop already, the legs still closing.
    assert_speed(&game, 0.0);
    assert!(game.get::<Stride>("stickman").unwrap().pace > 0.5);
    game.run(400.0);
    assert_eq!(game.get::<Stride>("stickman").unwrap().pace, 0.0);

    game.tap("Space");
    game.run(100.0);
    assert_eq!(game.get::<Animator>("stickman").unwrap().state(), "air");
    game.run(1500.0);
    assert_eq!(game.get::<Animator>("stickman").unwrap().state(), "move");
}

#[test]
fn restoring_a_save_preserves_the_movement_path() {
    let mut game = start();
    game.hold("KeyW", 250.0);
    let saved = game.save().unwrap();
    let mut restored = start();
    restored.restore(&saved).unwrap();
    game.hold("KeyD", 400.0);
    restored.hold("KeyD", 400.0);
    assert_eq!(
        game.global_position("player"),
        restored.global_position("player")
    );
    assert_eq!(game.world().hash(), restored.world().hash());
}

#[test]
fn the_forearm_follows_the_upper_arm_late() {
    use exact_beat_em_up_logic::gait::PACES;
    use exact_game::animation::{bind_pose, sample};
    use exact_game::Quat;
    let rig = stickman::rig(1.8);
    let model = stickman::model(&rig);
    let rest = bind_pose(&model);
    let node = |name: &str| model.nodes.iter().position(|n| n.name == name).unwrap();
    for pace in PACES.iter() {
        let clip = model.clips.iter().find(|c| c.name == pace.clip()).unwrap();
        // The cycle phase at which a joint is turned furthest forward about X.
        let peak = |name: &str| {
            (0..200)
                .map(|i| {
                    let phase = i as f32 / 200.0;
                    let mut local = vec![];
                    sample(clip, phase * clip.duration(), &rest, &mut local);
                    let turn = Quat::from_slice(&local[node(name) * 10 + 3..][..4]);
                    (phase, turn.to_scaled_axis().x)
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap()
                .0
        };
        let lag = (peak("forearm_r") - peak("upper_arm_r")).rem_euclid(1.0);
        assert!(
            (0.04..0.2).contains(&lag),
            "{}: the elbow peaks {lag} of a cycle after the shoulder",
            pace.clip()
        );
    }
}

#[test]
fn rollers_come_down_the_ramp_and_leave() {
    use exact_beat_em_up_logic::park::{Roller, RAMP_TOP};
    use exact_game_physics::Body;
    let mut game = start();
    game.run(4000.0);
    let rollers = |game: &Sim<ExactBeatEmUp>| -> Vec<Vec3> {
        let w = game.world();
        w.query::<&Roller>()
            .iter()
            .map(|(e, _)| w.get::<Transform>(e).unwrap().position)
            .collect()
    };
    let now = rollers(&game);
    assert_eq!(now.len(), 2, "one roller every 2.6 s");
    // The first is well down the slope and between the rails.
    assert!(now.iter().any(|p| p.z > RAMP_TOP.z + 4.0 && p.y < RAMP_TOP.y - 1.5));
    assert!(now.iter().all(|p| (p.x - RAMP_TOP.x).abs() < 2.5), "{now:?}");
    game.run(30_000.0);
    let later = rollers(&game);
    assert!(later.len() <= 6, "rollers leave after their time: {}", later.len());
    assert!(later.iter().all(|p| p.is_finite() && p.y > -0.5), "{later:?}");
    let w = game.world();
    assert!(w.query::<&Body>().iter().all(|(_, b)| b.velocity.is_finite()));
}
