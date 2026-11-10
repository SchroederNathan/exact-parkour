//! Scenarios in the park: every move, from several directions and speeds. Each run
//! checks the body on every tick, so a pass means the whole course was sound and
//! not only its end.
use exact_beat_em_up_logic::body::{self, Action, Parkour, HALF, RADIUS};
use exact_beat_em_up_logic::{hold, ledge};
use exact_beat_em_up_logic::park::{self, Block};
use exact_beat_em_up_logic::{ExactBeatEmUp, Options};
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
        let d = dir.normalize() * 60.0;
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
            approach(&mut game, &turned_block, slant, 3.0, 0.5, false);
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
fn a_drop_of_two_metres_is_rolled_out_or_crouched_out() {
    // Running off: a roll that keeps the body moving, then back on its feet.
    let mut game = start();
    let landing = drop_from(&mut game, "tower-1", Some(false));
    assert!(matches!(landing, Action::Roll { .. }), "{landing:?}");
    let rolled_from = feet(&game);
    let up = run(&mut game, 2000.0, "roll", |g| matches!(action(g), Action::Ground));
    assert!(up);
    assert!((feet(&game) - rolled_from).length() > 1.0, "the roll covers ground");

    // Stepping off with no speed: down to the hands, then up, in place.
    let mut game = start();
    let landing = drop_from(&mut game, "tower-1", None);
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
    approach(&mut game, block, slant, 3.0, near, false);
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
                if before < 0.0 && over < 0.0 {
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
