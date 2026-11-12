//! Falls, hits and getting up. Every run checks the ragdoll on every tick: its
//! points stay finite, above the floor and the right distance apart.
use exact_beat_em_up_logic::body::{self, Action, Parkour, HALF, HARD, KNOCK};
use exact_beat_em_up_logic::ledge;
use exact_beat_em_up_logic::park;
use exact_beat_em_up_logic::ragdoll::{Ragdoll, JOINTS, REST};
use exact_beat_em_up_logic::stickman::{self, MODEL};
use exact_beat_em_up_logic::{ExactBeatEmUp, Options};
use exact_game::animation::{bind_pose, joint_matrix, sample};
use exact_game::{Pose, Sim, Transform, Vec3};
use exact_game_physics::{Body, CapsuleController};

type Game = Sim<ExactBeatEmUp>;

// A little over one 120 Hz tick: each `run` of it advances exactly one.
const TICK_MS: f64 = 8.334;

fn start() -> Game {
    let mut game = exact_game::sim_with_art!(Options {
        seed: 7,
        ..Options::default()
    })
    .unwrap();
    game.run(25.0);
    game
}

fn centre(game: &Game) -> Vec3 {
    game.get::<Transform>("player").unwrap().position
}
fn action(game: &Game) -> Action {
    game.get::<Parkour>("player").unwrap().action.clone()
}
fn doll(game: &Game) -> Ragdoll {
    game.get::<Ragdoll>("player").unwrap().clone()
}

// Stand the body at `at` (its feet), facing along `dir`.
fn place(game: &mut Game, at: Vec3, dir: Vec3) {
    let w = game.world_mut();
    w.require_mut::<Transform>("player").position = at + Vec3::Y * (HALF + 0.02);
    *w.require_mut::<Parkour>("player") = Parkour {
        yaw: dir.x.atan2(dir.z),
        ..Parkour::default()
    };
    let mut c = w.require_mut::<CapsuleController>("player");
    c.velocity = Vec3::ZERO;
    c.grounded = false;
    c.support = None;
}

// World positions of the visual's joints that the ragdoll's points ride.
fn joints(game: &Game) -> Vec<Vec3> {
    let w = game.world();
    let model = w.model(MODEL).unwrap();
    let pose = game.get::<Pose>("stickman").unwrap();
    let child = *game.get::<Transform>("stickman").unwrap();
    let root = centre(game) + child.position;
    JOINTS
        .iter()
        .map(|name| {
            let node = model.nodes.iter().position(|n| n.name == *name).unwrap() as u32;
            let local = joint_matrix(model, &pose.local, node).w_axis.truncate();
            root + child.rotation * local
        })
        .collect()
}

// What must hold on every tick while the body is down.
#[track_caller]
fn check(game: &Game, what: &str) {
    let at = centre(game);
    assert!(at.is_finite(), "{what}: capsule at {at}");
    assert!(at.x.abs() < 31.0 && at.z.abs() < 31.0, "{what}: left the park at {at}");
    let doll = doll(game);
    if !matches!(action(game), Action::Ragdoll) {
        return;
    }
    for (i, p) in doll.at.iter().enumerate() {
        assert!(p.is_finite() && doll.velocity[i].is_finite(), "{what}: point {i} at {p}");
        assert!(p.y > 0.0, "{what}: point {} under the floor at {p}", JOINTS[i]);
        assert!(doll.velocity[i].length() < 40.0, "{what}: point {i} at {} m/s", doll.velocity[i].length());
    }
    // The bones keep their lengths: thigh, shin, upper arm, forearm, neck.
    for (a, b) in [(5, 11), (11, 13), (6, 12), (12, 14), (3, 7), (7, 9), (4, 8), (8, 10), (1, 0)] {
        let rest = (REST[a] - REST[b]).length();
        let now = (doll.at[a] - doll.at[b]).length();
        assert!(
            (now - rest).abs() < 0.06,
            "{what}: {} to {} is {now} m, at rest {rest} m",
            JOINTS[a],
            JOINTS[b]
        );
    }
    // The stick man is drawn where the ragdoll is.
    for (i, joint) in joints(game).into_iter().enumerate() {
        let off = (joint - doll.at[i]).length();
        assert!(off < 0.08, "{what}: {} drawn {off} m from its point", JOINTS[i]);
    }
}

#[track_caller]
fn run(game: &mut Game, ms: f64, what: &str, until: impl Fn(&Game) -> bool) -> bool {
    for _ in 0..(ms / TICK_MS) as u32 {
        game.run(TICK_MS);
        check(game, what);
        if until(game) {
            return true;
        }
    }
    false
}

// From the fall to back on the feet: the body goes limp, comes to rest, rises, and
// ends standing somewhere clear with the visual back under the capsule.
#[track_caller]
fn recovers(game: &mut Game, what: &str) {
    let down = run(game, 6000.0, what, |g| matches!(action(g), Action::Ragdoll));
    assert!(down, "{what}: never fell, doing {:?}", action(game));
    let rising = run(game, 8000.0, what, |g| matches!(action(g), Action::Rise { .. }));
    assert!(rising, "{what}: never came to rest");
    // Something may knock it down again while it rises; it still ends on its feet.
    let up = run(game, 25_000.0, what, |g| matches!(action(g), Action::Ground));
    assert!(up, "{what}: never stood up, doing {:?}", action(game));
    // Take away whatever is still rolling, then look at where it stands.
    let rollers: Vec<_> = {
        let w = game.world();
        w.query::<&park::Roller>().iter().map(|(e, _)| e).collect()
    };
    for e in rollers {
        game.world_mut().despawn(e);
    }
    assert!(
        ledge::clear(game.world(), centre(game) + Vec3::Y * 0.03),
        "{what}: stood up inside something at {}",
        centre(game)
    );
    let child = game.get::<Transform>("stickman").unwrap().position;
    assert!((child - Vec3::new(0.0, -HALF - 0.01, 0.0)).length() < 1e-4, "{what}: visual at {child}");
    assert!(!doll(game).active(), "{what}: the ragdoll is still out");
    // And it can walk away, some way or other.
    let from = centre(game);
    let free = ["KeyS", "KeyW", "KeyD", "KeyA"].into_iter().any(|key| {
        game.key_down(key);
        game.run(500.0);
        game.key_up(key);
        let speed = game.get::<CapsuleController>("player").unwrap().velocity.length();
        game.run(300.0);
        speed > 1.5
    });
    assert!(free, "{what}: cannot walk after rising at {from}, doing {:?}", action(game));
}

#[test]
fn the_points_rest_where_the_rig_has_its_joints() {
    let rig = stickman::rig(body::HEIGHT);
    for (name, rest) in JOINTS.iter().zip(REST) {
        let bone = rig.bones().iter().find(|b| b.name == *name).unwrap();
        assert!((bone.head - rest).length() < 1e-5, "{name}: rig {} and REST {rest}", bone.head);
    }
}

#[test]
fn a_fall_from_too_high_cannot_be_saved() {
    // The controller snaps to the ground from 0.3 m up, so a drop lands with the
    // speed of one that much shorter: 4.2 m is the hardest roll, not a fall.
    for (roof, height) in [("tower-3", 6.5 - 0.35), ("tower-4", 9.0 - 0.35)] {
        for running in [false, true] {
            let what = format!("fall from {roof}, running {running}");
            assert!((2.0 * body::GRAVITY * height as f32).sqrt() >= HARD);
            let mut game = start();
            let block = park::named(roof);
            // Off the west side, where the ground is clear below both roofs.
            let lip = block.at[0] - block.size[0] / 2.0;
            if running {
                place(&mut game, Vec3::new(lip + 1.5, block.top(), block.at[2]), -Vec3::X);
                game.run(40.0);
                game.key_down("KeyA");
            } else {
                place(&mut game, Vec3::new(lip - 0.4, block.top(), block.at[2]), -Vec3::X);
            }
            recovers(&mut game, &what);
        }
    }
}

#[test]
fn a_fall_just_under_the_limit_is_still_rolled_out() {
    let mut game = start();
    // The second roof, 4.2 m up: the highest a roll still saves.
    let roof = park::named("tower-2");
    let lip = roof.at[0] + roof.size[0] / 2.0;
    place(&mut game, Vec3::new(lip - 1.0, roof.top(), roof.at[2]), Vec3::X);
    game.run(40.0);
    game.key_down("KeyD");
    let off = run(&mut game, 2000.0, "4.2 m", |g| matches!(action(g), Action::Air));
    assert!(off);
    let landed = run(&mut game, 3000.0, "4.2 m", |g| !matches!(action(g), Action::Air));
    assert!(landed);
    assert!(matches!(action(&game), Action::Roll { .. }), "{:?}", action(&game));
}

// Send roller `kind` at the body at `speed` m/s from the north.
fn hit(game: &mut Game, kind: u32, speed: f32) -> f32 {
    let target = centre(game);
    let from = Vec3::new(target.x, 0.7, target.z - 1.3);
    let e = park::release(game.world_mut(), kind, from, Vec3::Z * speed);
    game.world().get::<Body>(e).unwrap().mass * speed
}

#[test]
fn a_heavy_hit_knocks_the_body_down_and_a_light_one_shoves_it() {
    // (kind, speed): a ball, a barrel, a crate and the heavy ball.
    for (kind, speed) in [(0, 9.0), (1, 6.0), (2, 4.0), (3, 5.0), (0, 3.0), (3, 0.6)] {
        let mut game = start();
        // Open ground, away from the ramp's own rollers.
        place(&mut game, Vec3::new(0.0, 0.0, 12.0), Vec3::Z);
        game.run(100.0);
        let momentum = hit(&mut game, kind, speed);
        let what = format!("hit by roller {kind} at {speed} m/s ({momentum} kg·m/s)");
        if momentum >= KNOCK * 1.1 {
            recovers(&mut game, &what);
        } else if momentum <= KNOCK * 0.9 {
            let fell = run(&mut game, 2000.0, &what, |g| matches!(action(g), Action::Ragdoll));
            assert!(!fell, "{what}: knocked down by a light hit");
            let v = game.get::<CapsuleController>("player").unwrap().velocity;
            let at = centre(&game);
            assert!(at.z > 12.05 || v.z > 0.0, "{what}: not shoved, at {at}");
            assert!(matches!(action(&game), Action::Ground), "{:?}", action(&game));
        }
    }
}

#[test]
fn a_hit_while_hanging_takes_the_body_off_the_wall() {
    let mut game = start();
    let wall = park::named("ledge-mid");
    // Hanging on the south face.
    let face = wall.at[2] + wall.size[2] / 2.0;
    place(&mut game, Vec3::new(wall.at[0], 0.0, face + 0.4), -Vec3::Z);
    game.run(60.0);
    game.key_down("KeyW");
    game.tap("Space");
    let hanging = run(&mut game, 1500.0, "hang", |g| matches!(action(g), Action::Hang(_)));
    assert!(hanging, "{:?}", action(&game));
    game.key_up("KeyW");
    // A barrel along the wall, at the body's height.
    let at = centre(&game);
    park::release(game.world_mut(), 1, at + Vec3::X * 2.0, -Vec3::X * 7.0);
    recovers(&mut game, "hit while hanging");
}

#[test]
fn the_body_gets_up_wherever_it_lands() {
    // Thrown from many places in many directions: against walls, onto roofs, into
    // corners and across the ramp's lane.
    let spots = [
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(-2.0, 0.0, -5.0),
        Vec3::new(-3.0, 0.0, -11.5),
        Vec3::new(-18.0, 2.0, -12.0),
        Vec3::new(14.0, 0.0, 0.5),
        Vec3::new(28.5, 0.0, 28.5),
        Vec3::new(6.0, 1.05, 8.0),
    ];
    for (n, spot) in spots.into_iter().enumerate() {
        for turn in 0..8 {
            let angle = turn as f32 * std::f32::consts::FRAC_PI_4;
            let dir = Vec3::new(angle.sin(), 0.0, angle.cos());
            let what = format!("thrown from spot {n} toward {turn}/8");
            let mut game = start();
            place(&mut game, spot, dir);
            game.run(60.0);
            // A hard shove from behind by the heavy ball.
            let at = centre(&game);
            park::release(game.world_mut(), 3, at - dir * 1.0, dir * 7.0);
            recovers(&mut game, &what);
        }
    }
}

#[test]
fn the_rise_keeps_the_body_above_the_floor_and_its_hands_on_it() {
    let rig = stickman::rig(body::HEIGHT);
    let model = stickman::model(&rig);
    let rest = bind_pose(&model);
    let at = |clip: &str, phase: f32, name: &str| {
        let clip = model.clips.iter().find(|c| c.name == clip).unwrap();
        let mut local = vec![];
        sample(clip, phase * clip.duration(), &rest, &mut local);
        let node = model.nodes.iter().position(|n| n.name == name).unwrap() as u32;
        joint_matrix(&model, &local, node).w_axis.truncate()
    };
    for clip in ["rise-prone", "rise-supine"] {
        for step in 0..=100 {
            let phase = step as f32 / 100.0;
            for name in JOINTS {
                let p = at(clip, phase, name);
                assert!(p.y > -0.03, "{clip} at {phase}: {name} at {p}");
            }
        }
        // It ends standing.
        assert!((at(clip, 1.0, "head").y - 1.68).abs() < 0.01);
        assert!(at(clip, 1.0, "foot_l").y < 0.12);
    }
    // On the front the hands press the floor beside the chest, and do not move,
    // while the chest and then the hips come up.
    let planted = at("rise-prone", 0.2, "hand_l");
    for phase in [0.25, 0.4, 0.55, 0.72] {
        for hand in ["hand_l", "hand_r"] {
            let p = at("rise-prone", phase, hand);
            assert!(p.y < 0.08, "rise-prone at {phase}: {hand} is {} m up", p.y);
        }
        let moved = (at("rise-prone", phase, "hand_l") - planted).length();
        assert!(moved < 0.03, "rise-prone at {phase}: the hand slid {moved} m");
    }
    assert!(at("rise-prone", 0.4, "neck").y > at("rise-prone", 0.1, "neck").y + 0.15);
    // On the back the hands are on the floor behind the hips while the body sits
    // up and the feet come in, and the feet then stay where they stand.
    for phase in [0.2, 0.3, 0.4] {
        for hand in ["hand_l", "hand_r"] {
            let p = at("rise-supine", phase, hand);
            assert!(p.y < 0.1, "rise-supine at {phase}: {hand} is {} m up", p.y);
        }
    }
    let stood = at("rise-supine", 0.62, "foot_l");
    for phase in [0.7, 0.8, 0.9, 1.0] {
        let moved = (at("rise-supine", phase, "foot_l") - stood).length();
        assert!(moved < 0.03, "rise-supine at {phase}: the foot slid {moved} m");
    }
}

