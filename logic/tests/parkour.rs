//! Scenarios in the park: every move, from several directions and speeds. Each run
//! checks the body on every tick, so a pass means the whole course was sound and
//! not only its end.
use exact_beat_em_up_logic::body::{self, Action, Parkour, HALF, RADIUS};
use exact_beat_em_up_logic::{hold, ledge, stickman};
use exact_beat_em_up_logic::park::{self, Block};
use exact_beat_em_up_logic::{ExactBeatEmUp, Options};
use exact_game::animation::Layers;
use exact_game::{InputEvent, PointerPhase, Sim, Transform, Vec3};
use exact_game_physics::{self as physics, CapsuleController, Shape};

type Game = Sim<ExactBeatEmUp>;

// A little over one 120 Hz tick: each `run` of it advances exactly one.
const TICK_MS: f64 = 8.334;

fn start() -> Game {
    let mut game = Sim::new(Options {
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
fn feet(game: &Game) -> Vec3 {
    body::feet(centre(game))
}
fn action(game: &Game) -> Action {
    game.get::<Parkour>("player").unwrap().action.clone()
}
fn velocity(game: &Game) -> Vec3 {
    game.get::<CapsuleController>("player").unwrap().velocity
}

// Stand the body at `at` (its feet), facing along `dir`.
fn place(game: &mut Game, at: Vec3, dir: Vec3) {
    let w = game.world_mut();
    w.require_mut::<Transform>("player").position = at + Vec3::Y * (HALF + 0.02);
    let mut body = w.require_mut::<Parkour>("player");
    *body = Parkour {
        yaw: dir.x.atan2(dir.z),
        ..Parkour::default()
    };
    drop(body);
    let mut c = w.require_mut::<CapsuleController>("player");
    c.velocity = Vec3::ZERO;
    c.grounded = false;
    c.support = None;
}

// Push the stick fully along the world direction `dir`, as a thumb would.
fn steer(game: &mut Game, dir: Vec3) {
    let event = |phase, x: f32, y: f32| InputEvent::Control {
        name: "move".into(),
        id: 1,
        phase,
        x,
        y,
        at_ms: 0.0,
        player: 0,
    };
    game.input(event(PointerPhase::Up, 100.0, 100.0));
    if dir != Vec3::ZERO {
        let yaw = game.world().resource::<exact_beat_em_up_logic::camera::Orbit>().yaw;
        let d = exact_game::Quat::from_rotation_y(-yaw) * dir.normalize() * 60.0;
        game.input(event(PointerPhase::Down, 100.0, 100.0));
        game.input(event(PointerPhase::Move, 100.0 + d.x, 100.0 + d.z));
    }
}

// What must hold on every tick of every scenario.
#[track_caller]
fn check(game: &Game, what: &str) {
    let at = centre(game);
    let v = velocity(game);
    assert!(at.is_finite() && v.is_finite(), "{what}: not finite: {at} {v}");
    // A vault carries the centre at the hips, which are low over a low wall.
    let low = match action(game) {
        Action::Ragdoll | Action::Rise { .. } => 0.0,
        Action::Vault(_) => HALF - 0.2,
        _ => HALF - 0.05,
    };
    assert!(at.y > low, "{what}: under the ground at {at}");
    assert!(at.y > 0.0, "{what}: under the ground at {at}");
    assert!(at.x.abs() < 31.0 && at.z.abs() < 31.0, "{what}: left the park at {at}");
    // On a wall, no part of the drawn body is in it: a bone may graze a surface,
    // and no more.
    let on_wall = match action(game) {
        Action::Vault(_) => Some("vault"),
        Action::Hang(_) => Some("hang"),
        Action::Climb(_) => Some("climb"),
        _ => None,
    };
    // The first moment of a hang is the catch, in which the arms come from
    // wherever the jump had them.
    let catching = matches!(action(game), Action::Hang(h) if h.time < body::SETTLE);
    if let (Some(doing), Some((bone, depth)), false) = (on_wall, buried(game), catching) {
        assert!(depth < 0.05, "{what}: {bone} is {depth} m inside a wall in a {doing}");
    }
    // On its feet, the body has ground under its feet, and not only under the
    // rim of its capsule.
    if matches!(action(game), Action::Ground) {
        assert!(
            ledge::overhang(game.world(), at).is_none(),
            "{what}: stands past an edge at {at}"
        );
    }
    // Under its own control the body is never inside a wall.
    if matches!(action(game), Action::Ground | Action::Air) {
        assert!(
            ledge::clear(game.world(), at + Vec3::Y * 0.03),
            "{what}: inside something solid at {at}"
        );
    }
}

// Advance `ms`, checking every tick. Stops early when `until` holds; returns
// whether it did.
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

fn degrees(a: f32) -> f32 {
    a * std::f32::consts::PI / 180.0
}

// The face of an unpitched block that looks south (+Z) when its yaw is zero: a
// point at its middle on the ground, and its outward normal.
fn south_face(b: &Block) -> (Vec3, Vec3) {
    let yaw = degrees(b.yaw);
    let out = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let middle = Vec3::new(b.at[0], 0.0, b.at[2]) + out * (b.size[2] / 2.0);
    (middle, out)
}

// `v` turned by `a` radians about Y.
fn turned(v: Vec3, a: f32) -> Vec3 {
    Vec3::new(v.x * a.cos() + v.z * a.sin(), 0.0, -v.x * a.sin() + v.z * a.cos())
}

// Run at the middle of `block`'s south face from `back` metres away, `slant`
// degrees off square, and press jump `near` metres from the face.
fn approach(game: &mut Game, block: &Block, slant: f32, back: f32, near: f32, sprint: bool) {
    let (middle, out) = south_face(block);
    let dir = turned(-out, degrees(slant));
    place(game, middle - dir * back, dir);
    game.run(60.0);
    steer(game, dir);
    if sprint {
        game.key_down("ShiftLeft");
    }
    let what = format!("approach {} at {slant} degrees", block.name);
    let reached = run(game, 4000.0, &what, |g| {
        (feet(g) - middle).dot(out) - RADIUS < near
    });
    assert!(reached, "{what}: never reached the wall");
    game.tap("Space");
}

#[test]
fn the_stick_steers_in_world_directions() {
    for dir in [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z, Vec3::new(1.0, 0.0, 1.0)] {
        let mut game = start();
        steer(&mut game, dir);
        run(&mut game, 500.0, "steer", |_| false);
        let v = velocity(&game);
        assert!(v.normalize().dot(dir.normalize()) > 0.99, "{dir}: went {v}");
    }
}

// Run one case of a matrix and report why it failed, so a matrix test can name
// every failing case and not only the first.
fn case(what: &str, run: impl FnOnce() + std::panic::UnwindSafe) -> Option<String> {
    std::panic::catch_unwind(run).err().map(|cause| {
        let why = cause
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| cause.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        format!("{what}: {why}")
    })
}

#[track_caller]
fn all_pass(failures: Vec<String>, of: usize) {
    assert!(
        failures.is_empty(),
        "{} of {of} cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn a_jump_at_a_low_wall_vaults_it_from_any_side() {
    let mut failures = vec![];
    let mut cases = 0;
    for name in ["vault-low", "vault-mid", "vault-high", "vault-turned"] {
        let block = park::named(name);
        for slant in [-45.0, -30.0, -15.0, 0.0, 15.0, 30.0, 45.0] {
            for sprint in [false, true] {
                cases += 1;
                let what = format!("vault {name} at {slant} degrees, sprint {sprint}");
                failures.extend(case(&what.clone(), move || {
                    let mut game = start();
                    let (middle, out) = south_face(block);
                    approach(&mut game, block, slant, 4.0, 0.6, sprint);
                    let vaulting = run(&mut game, 100.0, &what, |g| {
                        matches!(action(g), Action::Vault(_))
                    });
                    assert!(vaulting, "no vault, doing {:?}", action(&game));
                    // Over the top, never under it, and down on the far side. On
                    // the way the left hand is down on the wall's top.
                    let mut planted = 0;
                    let mut over = false;
                    for _ in 0..180 {
                        game.run(TICK_MS);
                        check(&game, &what);
                        if let Action::Vault(path) = action(&game) {
                            assert!(path.held, "nowhere to put the hand");
                            let top = path.ledge.edge.y;
                            assert!((path.hand.y - top - hold::WRIST).abs() < 0.03);
                            let wrist = hold::joint(game.world(), "hand_l");
                            planted += ((wrist - path.hand).length() < 0.01) as u32;
                        }
                        if matches!(action(&game), Action::Ground) {
                            over = true;
                            break;
                        }
                    }
                    assert!(over, "stuck in {:?}", action(&game));
                    assert!(planted >= 6, "the hand was on the wall for {planted} ticks");
                    let beyond = -(feet(&game) - middle).dot(out);
                    assert!(
                        beyond > block.size[2] + 0.2,
                        "ended {beyond} m past the near face"
                    );
                    assert!(feet(&game).y < 0.06, "ended at {}", feet(&game));
                    // It comes out moving.
                    let v = velocity(&game);
                    assert!(v.length() > 2.0, "lost its speed: {v}");
                }));
            }
        }
    }
    all_pass(failures, cases);
}

#[test]
fn a_jump_at_a_deep_wall_climbs_onto_it() {
    for (name, face) in [("mantle", 0.0), ("gap-1", 0.0), ("gap-1", 90.0)] {
        let block = park::named(name);
        for slant in [-30.0, 0.0, 30.0] {
            let mut game = start();
            // `face` turns the approach to the side of the block that is open.
            let turned_block = Block {
                yaw: block.yaw + face,
                ..*block
            };
            // South of the first roof the next one is 1.5 m away: start in the gap.
            let back = if (name, face) == ("gap-1", 0.0) { 1.1 } else { 3.0 };
            approach(&mut game, &turned_block, slant, back, 0.5, false);
            let what = format!("climb {name} at {slant} degrees");
            let climbing = run(&mut game, 100.0, &what, |g| {
                matches!(action(g), Action::Climb(_))
            });
            assert!(climbing, "{what}: no climb, doing {:?}", action(&game));
            let up = run(&mut game, 2000.0, &what, |g| {
                matches!(action(g), Action::Ground)
            });
            assert!(up, "{what}: stuck in {:?}", action(&game));
            assert!(
                (feet(&game).y - block.top()).abs() < 0.05,
                "{what}: feet at {} and the top at {}",
                feet(&game).y,
                block.top()
            );
        }
    }
}

#[test]
fn a_jump_at_a_high_wall_grabs_its_edge_and_climbs() {
    let mut failures = vec![];
    let mut cases = 0;
    for name in ["ledge-low", "ledge-mid", "ledge-high", "ledge-turned"] {
        let block = park::named(name);
        for slant in [-35.0, -20.0, 0.0, 20.0, 35.0] {
            cases += 1;
            let what = format!("grab {name} at {slant} degrees");
            failures.extend(case(&what.clone(), move || {
                let mut game = start();
                approach(&mut game, block, slant, 3.0, 0.35, false);
                // The lowest of these walls is within a jump: the body pulls
                // straight onto it. The others it must hang from first.
                let caught = run(&mut game, 900.0, &what, |g| {
                    matches!(action(g), Action::Hang(_) | Action::Climb(_))
                });
                assert!(caught, "no grab, doing {:?}", action(&game));
                if matches!(action(&game), Action::Hang(_)) {
                    // It hangs still, with its hands at the edge.
                    steer(&mut game, Vec3::ZERO);
                    run(&mut game, 400.0, &what, |_| false);
                    let held = centre(&game);
                    run(&mut game, 500.0, &what, |_| false);
                    assert_eq!(centre(&game), held, "slid while hanging");
                    let hands = feet(&game).y + ledge::HANG_REACH;
                    assert!((hands - block.top()).abs() < 0.02, "hands at {hands}");
                    // Both hands are on the edge, a palm's thickness over it.
                    for name in ["hand_l", "hand_r"] {
                        let wrist = hold::joint(game.world(), name);
                        let over = wrist.y - block.top();
                        assert!((over - hold::WRIST).abs() < 0.015, "{name} is {over} m over the edge");
                    }
                    let holds = body::holds(game.world());
                    assert_eq!(holds.len(), 2);
                    for h in holds {
                        let wrist = hold::joint(game.world(), h.limb.chain()[2]);
                        assert!((wrist - h.at).length() < 0.015, "{:?} is off its hold", h.limb);
                    }
                } else {
                    assert_eq!(name, "ledge-low", "climbed without a hang");
                }
                game.tap("Space");
                let up = run(&mut game, 2500.0, &what, |g| {
                    matches!(action(g), Action::Ground)
                });
                assert!(up, "stuck in {:?}", action(&game));
                assert!(
                    (feet(&game).y - block.top()).abs() < 0.05,
                    "feet at {}",
                    feet(&game).y
                );
            }));
        }
    }
    all_pass(failures, cases);
}

#[test]
fn letting_go_of_an_edge_drops_to_the_ground() {
    let block = park::named("ledge-mid");
    let (_, out) = south_face(block);
    let mut game = start();
    approach(&mut game, block, 0.0, 3.0, 0.35, false);
    let hanging = run(&mut game, 900.0, "let go", |g| matches!(action(g), Action::Hang(_)));
    assert!(hanging);
    steer(&mut game, out);
    let down = run(&mut game, 2000.0, "let go", |g| {
        matches!(action(g), Action::Ground) && feet(g).y < 0.06
    });
    assert!(down, "still {:?} at {}", action(&game), feet(&game));
}

#[test]
fn the_roofs_are_a_run_of_jumps() {
    let mut game = start();
    let first = park::named("gap-1");
    place(
        &mut game,
        Vec3::new(first.at[0], first.top(), first.at[2] - 1.0),
        Vec3::Z,
    );
    game.run(60.0);
    steer(&mut game, Vec3::Z);
    game.key_down("ShiftLeft");
    // Jump at the end of each roof; the last gap ends on a higher roof, by its edge.
    for (from, to) in [("gap-1", "gap-2"), ("gap-2", "gap-3"), ("gap-3", "gap-4"), ("gap-4", "gap-5")] {
        let (from, to) = (park::named(from), park::named(to));
        let lip = from.at[2] + from.size[2] / 2.0;
        let what = format!("{} to {}", from.name, to.name);
        let at_lip = run(&mut game, 3000.0, &what, |g| feet(g).z > lip - 0.35);
        assert!(at_lip, "{what}: never reached the lip, at {}", feet(&game));
        game.tap("Space");
        let across = run(&mut game, 4000.0, &what, |g| {
            matches!(action(g), Action::Hang(_))
                || (matches!(action(g), Action::Ground)
                    && (feet(g).y - to.top()).abs() < 0.05
                    && feet(g).z > to.at[2] - to.size[2] / 2.0)
        });
        assert!(across, "{what}: ended {:?} at {}", action(&game), feet(&game));
        if matches!(action(&game), Action::Hang(_)) {
            game.tap("Space");
            let up = run(&mut game, 2500.0, &what, |g| matches!(action(g), Action::Ground));
            assert!(up, "{what}: did not climb");
            assert!((feet(&game).y - to.top()).abs() < 0.05);
        }
    }
}

#[test]
fn stairs_are_walked_not_jumped() {
    let mut game = start();
    let top = park::named("stair-top");
    place(&mut game, Vec3::new(6.0, 0.0, 2.0), Vec3::Z);
    game.run(60.0);
    steer(&mut game, Vec3::Z);
    let up = run(&mut game, 4000.0, "stairs", |g| {
        (feet(g).y - top.top()).abs() < 0.03 && feet(g).z > 7.2
    });
    assert!(up, "ended at {} doing {:?}", feet(&game), action(&game));
}

// Run off the south edge of a roof, or with no `pace` drop from just past it, and
// report the action the landing became.
fn drop_from(game: &mut Game, name: &str, pace: Option<bool>) -> Action {
    let roof = park::named(name);
    let edge = roof.at[2] + roof.size[2] / 2.0;
    let what = format!("drop from {name}");
    place(game, Vec3::new(roof.at[0], roof.top(), edge - 1.5), Vec3::Z);
    game.run(60.0);
    match pace {
        Some(sprint) => {
            steer(game, Vec3::Z);
            if sprint {
                game.key_down("ShiftLeft");
            }
        }
        // A standing drop: the body starts just past the lip with no speed.
        None => place(game, Vec3::new(roof.at[0], roof.top(), edge + 0.4), Vec3::Z),
    }
    let off = run(game, 4000.0, &what, |g| matches!(action(g), Action::Air));
    assert!(off, "{what}: never left the roof");
    let landed = run(game, 4000.0, &what, |g| !matches!(action(g), Action::Air));
    assert!(landed, "{what}: never landed");
    action(game)
}

#[test]
fn a_drop_from_a_roof_is_knelt_out_rolled_out_or_crouched_out() {
    // Running off a 2 m roof: the body kneels into the landing and runs on.
    let mut game = start();
    let landing = drop_from(&mut game, "tower-1", Some(false));
    assert!(matches!(landing, Action::Ground), "{landing:?}");
    run(&mut game, 300.0, "kneel", |_| false);
    assert!(velocity(&game).length() > 3.0, "stopped: {}", velocity(&game));

    // Running off a 2.8 m wall: a roll that keeps the body moving, then back on
    // its feet.
    let mut game = start();
    let landing = drop_from(&mut game, "ledge-high", Some(false));
    assert!(matches!(landing, Action::Roll { .. }), "{landing:?}");
    let rolled_from = feet(&game);
    let up = run(&mut game, 2000.0, "roll", |g| matches!(action(g), Action::Ground));
    assert!(up);
    assert!((feet(&game) - rolled_from).length() > 1.0, "the roll covers ground");

    // Stepping off it with no speed: down to the hands, then up, in place.
    let mut game = start();
    let landing = drop_from(&mut game, "ledge-high", None);
    assert!(matches!(landing, Action::Land { .. }), "{landing:?}");
    let up = run(&mut game, 2500.0, "crouch", |g| matches!(action(g), Action::Ground));
    assert!(up);
}

#[test]
fn a_short_hop_needs_no_recovery() {
    let mut game = start();
    game.tap("Space");
    run(&mut game, 300.0, "hop", |_| false);
    assert!(matches!(action(&game), Action::Air));
    let down = run(&mut game, 1500.0, "hop", |g| !matches!(action(g), Action::Air));
    assert!(down);
    assert!(matches!(action(&game), Action::Ground), "{:?}", action(&game));
}

// Jump at the middle of `block`'s south face and hang from its edge. The stick
// still pushes at the wall, as it does when a player runs and jumps at it.
fn hang_from(game: &mut Game, block: &Block) {
    approach(game, block, 0.0, 3.0, 0.35, false);
    let hanging = run(game, 900.0, "hang", |g| matches!(action(g), Action::Hang(_)));
    assert!(hanging, "no hang, doing {:?}", action(game));
}

// How far each wrist is from the point on the edge it holds.
fn off_the_edge(game: &Game) -> [f32; 2] {
    let holds = body::holds(game.world());
    [0, 1].map(|i| (hold::joint(game.world(), holds[i].limb.chain()[2]) - holds[i].at).length())
}

#[test]
fn a_body_that_jumps_to_an_edge_stays_on_it_until_a_new_push() {
    let block = park::named("ledge-mid");
    let (_, out) = south_face(block);
    let mut game = start();
    hang_from(&mut game, block);
    // The stick is still forward from the run-up. That is not a wish to climb.
    let left = run(&mut game, 2000.0, "stay", |g| !matches!(action(g), Action::Hang(_)));
    assert!(!left, "left the edge, doing {:?}", action(&game));
    // Let go of the stick and push forward again: now it climbs.
    steer(&mut game, Vec3::ZERO);
    run(&mut game, 100.0, "stay", |_| false);
    steer(&mut game, -out);
    let climbing = run(&mut game, 100.0, "push", |g| matches!(action(g), Action::Climb(_)));
    assert!(climbing, "no climb, doing {:?}", action(&game));
    let up = run(&mut game, 2500.0, "push", |g| matches!(action(g), Action::Ground));
    assert!(up);
    assert!((feet(&game).y - block.top()).abs() < 0.05, "feet at {}", feet(&game).y);
}

#[test]
fn a_hanging_body_shimmies_along_the_edge_to_its_ends() {
    let block = park::named("ledge-mid");
    let (middle, out) = south_face(block);
    // Along the edge to the left of a body that faces the wall.
    let left = Vec3::Y.cross(-out);
    let half = block.size[0] / 2.0;
    let along = |p: Vec3| (p - middle).dot(left);
    let mut game = start();
    hang_from(&mut game, block);
    steer(&mut game, Vec3::ZERO);
    run(&mut game, 400.0, "settle", |_| false);
    let level = centre(&game).y;

    for (side, name) in [(1.0, "left"), (-1.0, "right")] {
        let from = along(centre(&game));
        steer(&mut game, left * side);
        let what = format!("shimmy {name}");
        let mut was = body::holds(game.world());
        let mut wrists = was.clone().map_holds(&game);
        let mut still = 0.0f32;
        for tick in 0..900 {
            game.run(TICK_MS);
            check(&game, &what);
            let Action::Hang(hanging) = action(&game) else {
                panic!("{what}: left the edge, doing {:?}", action(&game));
            };
            // Level, flat against the wall, and both hands on the edge.
            let at = centre(&game);
            assert!((at.y - level).abs() < 1e-4, "{what}: rose or sank to {}", at.y);
            // A hand has let go when the body leans out past the end of the edge.
            let now = body::holds(game.world());
            for (i, off) in off_the_edge(&game).into_iter().enumerate() {
                let holds = now[i].grip == 1.0;
                assert!(!holds || off < 0.015, "{what}: hand {i} is {off} m off the edge at tick {tick}");
            }
            // A hand that holds the same place on the edge does not move.
            let wrists_now = now.clone().map_holds(&game);
            for i in 0..2 {
                if (now[i].at - was[i].at).length() < 1e-5 && now[i].grip == 1.0 && was[i].grip == 1.0 {
                    let slid = (wrists_now[i] - wrists[i]).length();
                    assert!(slid < 0.004, "{what}: a planted hand slid {slid} m");
                }
                // Neither hand is ever past the end of the wall.
                assert!(along(now[i].at).abs() < half, "{what}: a hand is off the wall's end");
            }
            (was, wrists) = (now, wrists_now);
            still = if hanging.speed == 0.0 { still + 1.0 } else { 0.0 };
            if tick > 60 && still > 30.0 {
                break;
            }
        }
        // It went as far as the wall goes, and stopped with its hands still on it.
        let to = along(centre(&game));
        assert!(still > 30.0, "{what}: never stopped");
        assert!((to - from) * side > 1.0, "{what}: went {} m", (to - from) * side);
        let short = half - to * side;
        assert!((0.3..0.6).contains(&short), "{what}: stopped {short} m from the end");
        // At the end, with the stick still pushed, it leans out to look along the
        // wall: it hangs from the hand on that side, and the other has let go.
        run(&mut game, 600.0, &what, |_| false);
        let Action::Hang(hanging) = action(&game) else {
            panic!("{what}: left the edge");
        };
        assert!((hanging.lean - side).abs() < 1e-3, "{what}: leans {}", hanging.lean);
        let holds = body::holds(game.world());
        let (lead, trail) = if side > 0.0 { (0, 1) } else { (1, 0) };
        assert_eq!(holds[lead].grip, 1.0);
        assert_eq!(holds[trail].grip, 0.0);
        assert!(off_the_edge(&game)[lead] < 0.02, "{what}: the hand it hangs from is off the edge");
        // The stick let go, it comes back under both hands.
        steer(&mut game, Vec3::ZERO);
        run(&mut game, 600.0, &what, |_| false);
        let Action::Hang(hanging) = action(&game) else {
            panic!("{what}: left the edge");
        };
        assert_eq!(hanging.lean, 0.0);
        for off in off_the_edge(&game) {
            assert!(off < 0.01, "{what}: a hand is {off} m off the edge after the lean");
        }
    }

    // From anywhere on the edge, forward climbs.
    steer(&mut game, -out);
    let up = run(&mut game, 3000.0, "climb", |g| matches!(action(g), Action::Ground));
    assert!(up, "doing {:?}", action(&game));
    assert!((feet(&game).y - block.top()).abs() < 0.05);
}

// The wrists that go with a list of holds.
trait Wrists {
    fn map_holds(self, game: &Game) -> Vec<Vec3>;
}
impl Wrists for Vec<hold::Hold> {
    fn map_holds(self, game: &Game) -> Vec<Vec3> {
        self.iter()
            .map(|h| hold::joint(game.world(), h.limb.chain()[2]))
            .collect()
    }
}

#[test]
fn a_jump_from_the_end_of_an_edge_goes_where_the_stick_aims() {
    let from = park::named("ledge-mid");
    let (middle, out) = south_face(from);
    let left = Vec3::Y.cross(-out);
    // To the right is a higher wall, a metre away: the body catches its edge.
    // To the left is a lower one: the body lands on its top.
    for (side, to, hangs) in [(-1.0, "ledge-high", true), (1.0, "ledge-low", false)] {
        let to = park::named(to);
        let what = format!("leap to {}", to.name);
        let mut game = start();
        hang_from(&mut game, from);
        steer(&mut game, Vec3::ZERO);
        run(&mut game, 400.0, &what, |_| false);
        steer(&mut game, left * side);
        let at_end = run(&mut game, 6000.0, &what, |g| {
            matches!(action(g), Action::Hang(h) if (h.lean - side).abs() < 1e-3)
        });
        assert!(at_end, "{what}: never leaned at the end");
        game.tap("Space");
        // The legs swing back first, then it goes.
        let off = run(&mut game, 300.0, &what, |g| matches!(action(g), Action::Air));
        assert!(off, "{what}: no jump, doing {:?}", action(&game));
        let there = run(&mut game, 2500.0, &what, |g| {
            matches!(action(g), Action::Hang(_))
                || (matches!(action(g), Action::Ground) && (feet(g).y - to.top()).abs() < 0.05)
        });
        assert!(there, "{what}: ended {:?} at {}", action(&game), feet(&game));
        assert_eq!(matches!(action(&game), Action::Hang(_)), hangs, "{what}");
        let across = (centre(&game) - middle).dot(left) * side;
        assert!(across > from.size[0] / 2.0 + 1.0, "{what}: got {across} m along the wall");
    }

    // Away from the wall, a jump leaves it backward, further than letting go.
    let mut game = start();
    hang_from(&mut game, from);
    steer(&mut game, Vec3::ZERO);
    run(&mut game, 400.0, "jump off", |_| false);
    steer(&mut game, out);
    game.tap("Space");
    let down = run(&mut game, 2500.0, "jump off", |g| !matches!(action(g), Action::Air | Action::Hang(_)));
    assert!(down);
    let back = (feet(&game) - middle).dot(out);
    assert!(back > 1.5, "landed {back} m from the wall");
}

#[test]
fn a_body_past_an_edge_is_off_it() {
    let roof = park::named("tower-1");
    let edge = roof.at[0] + roof.size[0] / 2.0;
    // Creeping east off the roof: the feet are never further past the edge than
    // a foot is wide while the body still stands.
    let mut game = start();
    place(&mut game, Vec3::new(edge - 0.8, roof.top(), roof.at[2]), Vec3::X);
    game.run(60.0);
    let event = |phase, x: f32| InputEvent::Control {
        name: "move".into(),
        id: 1,
        phase,
        x,
        y: 100.0,
        at_ms: 0.0,
        player: 0,
    };
    game.input(event(PointerPhase::Down, 100.0));
    game.input(event(PointerPhase::Move, 118.0));
    let mut furthest = f32::MIN;
    let down = (0..900).any(|_| {
        game.run(TICK_MS);
        check(&game, "creep");
        if matches!(action(&game), Action::Ground) && feet(&game).y > roof.top() - 0.05 {
            furthest = furthest.max(feet(&game).x - edge);
        }
        feet(&game).y < 0.1
    });
    assert!(down, "never came off the roof: {:?} at {}", action(&game), feet(&game));
    assert!(furthest < ledge::FOOTING + 0.03, "stood {furthest} m past the edge");
    assert!(furthest > -0.1, "left the roof {furthest} m before its edge");

    // Put down with its feet 0.2 m past the edge and no push: it does not rest
    // there on the rim of its capsule.
    let mut game = start();
    place(&mut game, Vec3::new(edge + 0.2, roof.top(), roof.at[2]), Vec3::X);
    let down = run(&mut game, 2500.0, "rim", |g| feet(g).y < 0.1);
    assert!(down, "rests past the edge: {:?} at {}", action(&game), feet(&game));
}

// Run north from the origin for `ms`, sprinting or not, then press `key`.
fn run_then(game: &mut Game, sprint: bool, ms: f64, key: &str) {
    place(game, Vec3::new(0.0, 0.0, 20.0), -Vec3::Z);
    game.run(60.0);
    steer(game, -Vec3::Z);
    if sprint {
        game.key_down("ShiftLeft");
    }
    run(game, ms, "run up", |_| false);
    game.tap(key);
}

#[test]
fn a_slide_keeps_the_body_moving_and_ends_in_a_run() {
    for sprint in [true, false] {
        let what = format!("slide, sprint {sprint}");
        let mut game = start();
        run_then(&mut game, sprint, 900.0, "KeyC");
        let before = velocity(&game).length();
        let sliding = run(&mut game, 100.0, &what, |g| matches!(action(g), Action::Slide { .. }));
        assert!(sliding, "{what}: no slide, doing {:?}", action(&game));
        let from = feet(&game);
        // Low on the ground the whole way, never slower than most of its speed,
        // with a hand on the ground and no part of it under the ground.
        let mut lowest = f32::MAX;
        let mut touched = 0;
        let over = (0..200).any(|_| {
            game.run(TICK_MS);
            check(&game, &what);
            if !matches!(action(&game), Action::Slide { .. }) {
                return true;
            }
            let w = game.world();
            assert!(velocity(&game).length() > before * 0.8, "{what}: slowed to {}", velocity(&game));
            lowest = lowest.min(hold::joint(w, "hips").y);
            touched += (hold::joint(w, "hand_l").y < hold::WRIST + 0.02) as u32;
            if let Some((bone, depth)) = buried(&game) {
                assert!(depth < 0.05, "{what}: {bone} is {depth} m under the ground");
            }
            false
        });
        assert!(over, "{what}: never ended");
        assert!(matches!(action(&game), Action::Ground), "{what}: ended {:?}", action(&game));
        assert!(lowest < 0.4, "{what}: the hips came down to {lowest} m only");
        assert!(touched > 20, "{what}: the hand was on the ground for {touched} ticks");
        let far = (feet(&game) - from).length();
        assert!(far > before * body::SLIDE * 0.85, "{what}: slid {far} m");
        // It runs on at the speed it had.
        run(&mut game, 400.0, &what, |_| false);
        assert!(velocity(&game).length() > before * 0.95, "{what}: {}", velocity(&game));
    }

    // Too slow to slide: nothing happens.
    let mut game = start();
    game.tap("KeyC");
    run(&mut game, 100.0, "still", |_| false);
    assert!(matches!(action(&game), Action::Ground));

    // A jump cuts a slide short.
    let mut game = start();
    run_then(&mut game, true, 900.0, "KeyC");
    run(&mut game, 250.0, "slide", |_| false);
    assert!(matches!(action(&game), Action::Slide { .. }));
    game.tap("Space");
    let up = run(&mut game, 100.0, "jump", |g| matches!(action(g), Action::Air));
    assert!(up, "doing {:?}", action(&game));
}

#[test]
fn a_finger_that_slides_between_touch_buttons_presses_each() {
    // The app posts these when a finger that went down on one button arrives on
    // another; the button it went down on still holds the touch.
    let mut game = start();
    run_then(&mut game, false, 600.0, "KeyQ");
    let run_speed = velocity(&game).length();
    game.post("touch sprint on");
    run(&mut game, 500.0, "glide", |_| false);
    assert!(velocity(&game).length() > run_speed + 2.0, "no sprint: {}", velocity(&game));
    game.post("touch slide");
    let sliding = run(&mut game, 100.0, "glide", |g| matches!(action(g), Action::Slide { .. }));
    assert!(sliding, "doing {:?}", action(&game));
    run(&mut game, 1200.0, "glide", |g| matches!(action(g), Action::Ground));
    game.post("touch jump");
    let up = run(&mut game, 100.0, "glide", |g| matches!(action(g), Action::Air));
    assert!(up, "doing {:?}", action(&game));
    game.post("touch sprint off");
    run(&mut game, 1500.0, "glide", |g| matches!(action(g), Action::Ground));
    run(&mut game, 600.0, "glide", |_| false);
    assert!((velocity(&game).length() - run_speed).abs() < 0.3, "still sprints: {}", velocity(&game));
}

// Drop from `height` metres over open ground at `pace` m/s, and report the
// landing: what the body is doing, how deep it kneels, and its speed after.
fn land_from(height: f32, pace: f32) -> (Action, f32, f32) {
    let mut game = start();
    place(&mut game, Vec3::new(0.0, height, 20.0), -Vec3::Z);
    game.world_mut().require_mut::<CapsuleController>("player").velocity = -Vec3::Z * pace;
    if pace > 0.0 {
        steer(&mut game, -Vec3::Z);
    }
    let what = format!("land from {height} m");
    let mut kneel = 0.0f32;
    let landed = (0..600).any(|_| {
        game.run(TICK_MS);
        check(&game, &what);
        let layers = game.get::<Layers>("stickman").unwrap();
        let layer = &layers.0[stickman::KNEEL_LAYER];
        if layer.animation.time < 0.05 {
            kneel = kneel.max(layer.weight);
        }
        !matches!(action(&game), Action::Air)
    });
    assert!(landed, "{what}: never landed");
    game.run(TICK_MS);
    let layers = game.get::<Layers>("stickman").unwrap();
    let layer = &layers.0[stickman::KNEEL_LAYER];
    if layer.animation.time < 0.05 {
        kneel = kneel.max(layer.weight);
    }
    (action(&game), kneel, velocity(&game).length())
}

#[test]
fn a_landing_is_as_deep_as_the_fall_is_high() {
    // A step down: the knees give, and the run goes on.
    let (doing, kneel, speed) = land_from(0.5, 3.4);
    assert!(matches!(doing, Action::Ground), "{doing:?}");
    assert_eq!(kneel, 0.0);
    assert!(speed > 3.0, "{speed}");
    // Higher: the body kneels into it, deeper from higher, and still runs on.
    let (doing, low, speed) = land_from(1.2, 3.4);
    assert!(matches!(doing, Action::Ground), "{doing:?}");
    assert!(speed > 3.0, "{speed}");
    let (doing, high, speed) = land_from(1.9, 3.4);
    assert!(matches!(doing, Action::Ground), "{doing:?}");
    assert!(speed > 3.0, "{speed}");
    assert!(0.5 < low && low < high && high <= 1.0, "kneels {low} then {high}");
    // Higher still: a roll, longer and slower out of it from higher.
    let (light, _, _) = land_from(2.5, 3.4);
    let (heavy, _, _) = land_from(3.9, 3.4);
    let (Action::Roll { span: a, strain: s, .. }, Action::Roll { span: b, strain: t, .. }) = (&light, &heavy) else {
        panic!("{light:?} and {heavy:?}");
    };
    assert!(a < b && s < t, "rolls {a} s and {b} s");
    // With no speed to roll with, a crouch to the hands.
    let (doing, _, _) = land_from(3.0, 0.0);
    assert!(matches!(doing, Action::Land { .. }), "{doing:?}");
    // Too high: nothing saves it.
    let (doing, _, _) = land_from(6.0, 3.4);
    assert!(matches!(doing, Action::Ragdoll), "{doing:?}");
}

// The drawn body's bones as sampled points, each with its bone's name.
const BONES: [(&str, &str); 13] = [
    ("hips", "spine"),
    ("spine", "chest"),
    ("chest", "neck"),
    ("thigh_l", "shin_l"),
    ("shin_l", "foot_l"),
    ("thigh_r", "shin_r"),
    ("shin_r", "foot_r"),
    ("upper_arm_l", "forearm_l"),
    ("forearm_l", "hand_l"),
    ("upper_arm_r", "forearm_r"),
    ("forearm_r", "hand_r"),
    ("foot_l", "toe"),
    ("foot_r", "toe"),
];
// From the ankle to the tip of the toes, in the foot's frame.
const TOE: Vec3 = Vec3::new(0.0, -0.06, 0.19);

fn bones(game: &Game) -> Vec<(&'static str, Vec3)> {
    let w = game.world();
    BONES
        .iter()
        .flat_map(|&(from, to)| {
            let a = hold::joint(w, from);
            let b = if to == "toe" {
                hold::point(w, from, TOE)
            } else {
                hold::joint(w, to)
            };
            (0..=4).map(move |i| (from, a.lerp(b, i as f32 / 4.0)))
        })
        .collect()
}

// How deep the drawn body is in something solid: the bone and the depth in metres
// of the sampled point that is furthest in, if any is.
fn buried(game: &Game) -> Option<(&'static str, f32)> {
    let w = game.world();
    bones(game)
        .into_iter()
        .filter_map(|(bone, at)| {
            let inside = |radius: f32| {
                let shape = Shape::Sphere { radius };
                !physics::overlap(w, &shape, Transform::at(at.x, at.y, at.z), park::SOLID).is_empty()
            };
            // The largest sphere about the point that is wholly inside is its depth.
            [0.12, 0.08, 0.05, 0.03, 0.015]
                .into_iter()
                .find(|&depth| {
                    let around = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
                    inside(0.002)
                        && around.iter().all(|d| {
                            let p = at + *d * depth;
                            let shape = Shape::Sphere { radius: 0.002 };
                            !physics::overlap(w, &shape, Transform::at(p.x, p.y, p.z), park::SOLID)
                                .is_empty()
                        })
                })
                .map(|depth| (bone, depth))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

// Prints one move tick by tick: where the drawn body is against the wall.
#[test]
#[ignore]
fn trace() {
    let name = std::env::var("BLOCK").unwrap_or("vault-mid".into());
    let slant: f32 = std::env::var("SLANT").map_or(0.0, |s| s.parse().unwrap());
    let every: usize = std::env::var("EVERY").map_or(4, |s| s.parse().unwrap());
    let block = park::named(&name);
    let mut game = start();
    let near: f32 = std::env::var("NEAR").map_or(0.6, |s| s.parse().unwrap());
    let back: f32 = std::env::var("BACK").map_or(3.0, |s| s.parse().unwrap());
    approach(&mut game, block, slant, back, near, std::env::var("SPRINT").is_ok());
    for tick in 0..260 {
        game.run(TICK_MS);
        if std::env::var("UP").is_ok() && tick == 120 {
            steer(&mut game, Vec3::ZERO);
            game.tap("Space");
        }
        let w = game.world();
        let j = |n: &str| hold::joint(w, n);
        let (u, kind) = match action(&game) {
            Action::Vault(p) => (p.progress(), "vault"),
            Action::Climb(p) => (p.progress(), "climb"),
            Action::Hang(h) => (h.time, "hang"),
            _ => (-1.0, ""),
        };
        if u < 0.0 || tick % every != 0 {
            continue;
        }
        if std::env::var("DEEP").is_ok() {
            let (middle, out) = south_face(block);
            for (bone, at) in bones(&game) {
                let before = (at - middle).dot(out);
                let over = at.y - block.top();
                if before < 0.0 && before > -block.size[2] && over < 0.0 {
                    println!("{tick:3} {kind} {u:.2} {bone} {before:+.3}/{over:+.3}");
                }
            }
            continue;
        }
        let top = block.top();
        let holds = body::holds(w);
        let off: Vec<String> = holds
            .iter()
            .map(|h| format!("{:.3}", (j(h.limb.chain()[2]) - h.at).length()))
            .collect();
        let (_, out) = south_face(block);
        let (middle, _) = south_face(block);
        let rel = |n: &str| {
            let p = j(n);
            format!("{n} {:+.2}/{:+.2}", (p - middle).dot(out), p.y - top)
        };
        println!(
            "{tick:3} {kind} {u:.2} hips {:+.2} shoulder {:+.2} holds {off:?} buried {:?} {} {} {} {} {}",
            j("hips").y - top,
            j("upper_arm_l").y - top,
            buried(&game),
            rel("hips"), rel("shin_l"), rel("foot_l"), rel("shin_r"), rel("foot_r"),
        );
    }
}

