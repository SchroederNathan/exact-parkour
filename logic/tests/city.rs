//! Neon Rain: the district's model file is what the code builds, every step of the route is
//! within what the body can climb and jump, and the body stands and runs on the
//! pieces the route crosses.
use exact_beat_em_up_logic::body::{self, Action, Parkour, HALF};
use exact_beat_em_up_logic::city::{self, CityState, ROUTE};
use exact_beat_em_up_logic::kit;
use exact_beat_em_up_logic::{ExactBeatEmUp, Options, CITY_MODE};
use exact_game::{InputEvent, PointerPhase, Sim, Transform, Vec3};
use exact_game_physics::CapsuleController;

type Game = Sim<ExactBeatEmUp>;

fn start() -> Game {
    let mut game: Game = Sim::new(Options {
        seed: 7,
        mode: CITY_MODE,
        ..Options::default()
    })
    .unwrap();
    game.run(25.0);
    game
}

fn feet(game: &Game) -> Vec3 {
    body::feet(game.get::<Transform>("player").unwrap().position)
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
        let yaw = exact_beat_em_up_logic::camera::view(game.world()).yaw;
        let d = exact_game::Quat::from_rotation_y(-yaw) * dir.normalize() * 60.0;
        game.input(event(PointerPhase::Down, 100.0, 100.0));
        game.input(event(PointerPhase::Move, 100.0 + d.x, 100.0 + d.z));
    }
}

#[test]
fn the_district_builds_and_the_body_stands_on_the_street() {
    let mut game = start();
    game.run(500.0);
    let at = feet(&game);
    assert!(at.y.abs() < 0.05, "feet on the street: {at}");
    assert!((at - city::START).length() < 0.2, "at the start: {at}");
    assert!(game.get::<Transform>("sign-goal").unwrap().position.z < -30.0);
}

/// The heights the body gains on the route, each against the move that gains it.
#[test]
fn every_step_of_the_route_is_within_reach() {
    let roof = |name: &str| city::named(name).roof;
    // A jump from a stand catches an edge up to 2.8 m above the ground the feet
    // left: the park's highest ledge. A climb from the ground takes 1.5 m.
    const CATCH: f32 = 2.8;
    let rises = [
        ("street to crate", city::CRATE.y, body::CLIMB_HIGH),
        ("crate to stall roof", kit::STALL_ROOF - city::CRATE.y, CATCH),
        ("stall roof to west-low", roof("west-low") - kit::STALL_ROOF, CATCH),
        ("west-low to walkway deck", city::WALKWAY_AT.y + kit::WALKWAY_DECK - roof("west-low"), body::CLIMB_HIGH),
        ("fire escape middle to top", kit::ESCAPE_TOP - kit::ESCAPE_MID, body::CLIMB_HIGH),
        ("east-high to skybridge roof", city::SKYBRIDGE_AT.y + kit::SKYBRIDGE_TOP - roof("east-high"), body::CLIMB_HIGH),
        ("skybridge roof to west-high", roof("west-high") - city::SKYBRIDGE_AT.y - kit::SKYBRIDGE_TOP, body::CLIMB_HIGH),
        // The park's jump to a ledge: 1.4 m up across 2.5 m.
        ("west-high to north", roof("north") - roof("west-high"), 1.4),
    ];
    for (what, rise, most) in rises {
        assert!(rise > 0.0 && rise <= most + 1e-3, "{what}: {rise} m, at most {most}");
    }
    // The pieces meet the roofs they join.
    let level = |a: f32, b: f32| (a - b).abs() < 0.06;
    assert!(level(city::WALKWAY_AT.y + kit::WALKWAY_DECK, roof("east-low")));
    assert!(level(city::ESCAPE_AT.y + kit::ESCAPE_LOW, roof("east-low")));
    assert!(level(city::ESCAPE_AT.y + kit::ESCAPE_TOP, roof("east-high")));
    let alley = city::named("west-high").z[0] - city::named("north").z[1];
    assert!(alley > 0.0 && alley <= 2.5, "the alley to the north roof: {alley} m");
    // Every outer building is a wall to every roof of the route.
    let highest = roof("north");
    for b in city::BUILDINGS.iter().filter(|b| b.name.starts_with("outer")) {
        assert!(b.roof > highest + 2.5, "{} is too low", b.name);
    }
}

#[test]
fn the_body_stands_at_the_end_of_every_step_and_the_route_ends_at_the_sign() {
    let mut game = start();
    for (i, step) in ROUTE.iter().enumerate() {
        assert_eq!(game.world().resource::<CityState>().step as usize, i);
        let at = Vec3::from_array(step.at);
        place(&mut game, at + Vec3::Y * 0.2, Vec3::NEG_Z);
        game.run(700.0);
        let stood = feet(&game);
        assert!((stood.y - at.y).abs() < 0.08, "step {i} `{}`: feet at {stood}", step.ask);
    }
    assert_eq!(game.world().resource::<CityState>().step as usize, ROUTE.len());
    let shown = |game: &Game, key: &str| game.world().published(key).unwrap();
    assert_eq!(shown(&game, "finished").as_bool(), Some(true));
    assert_eq!(shown(&game, "mode").as_str(), Some("city"));
    let seconds = shown(&game, "seconds").as_number();
    game.run(500.0);
    assert_eq!(shown(&game, "seconds").as_number(), seconds, "the clock stops at the sign");
}

#[test]
fn the_body_runs_through_the_walkway_onto_the_east_roof() {
    let mut game = start();
    let deck = city::WALKWAY_AT.y + kit::WALKWAY_DECK;
    place(&mut game, Vec3::new(-6.0, deck + 0.1, city::WALKWAY_AT.z), Vec3::X);
    game.run(300.0);
    steer(&mut game, Vec3::X);
    for _ in 0..120 {
        game.run(50.0);
        let at = feet(&game);
        assert!((at.y - deck).abs() < 0.12, "on the deck: {at}");
        assert!((at.z - city::WALKWAY_AT.z).abs() < 1.0, "between the rails: {at}");
        if at.x > 7.4 {
            return;
        }
    }
    panic!("the body did not cross: {}", feet(&game));
}

#[test]
fn the_body_walks_up_the_fire_escape_to_its_middle_landing() {
    let mut game = start();
    let low = city::ESCAPE_AT + Vec3::new(-1.9, kit::ESCAPE_LOW, 1.15);
    place(&mut game, low + Vec3::Y * 0.1, Vec3::X);
    game.run(300.0);
    assert!((feet(&game).y - low.y).abs() < 0.06, "on the low landing: {}", feet(&game));
    steer(&mut game, Vec3::X);
    let mid = city::ESCAPE_AT.y + kit::ESCAPE_MID;
    for _ in 0..80 {
        game.run(50.0);
        let at = feet(&game);
        if (at.y - mid).abs() < 0.06 && at.x > city::ESCAPE_AT.x + 1.2 {
            return;
        }
    }
    panic!("the body did not reach the middle landing: {}", feet(&game));
}

#[test]
fn the_body_runs_the_roof_of_the_skybridge() {
    let mut game = start();
    let top = city::SKYBRIDGE_AT.y + kit::SKYBRIDGE_TOP;
    place(&mut game, Vec3::new(5.5, top + 0.1, city::SKYBRIDGE_AT.z), Vec3::NEG_X);
    game.run(300.0);
    steer(&mut game, Vec3::NEG_X);
    for _ in 0..120 {
        game.run(50.0);
        let at = feet(&game);
        assert!((at.y - top).abs() < 0.12, "on the roof of the bridge: {at}");
        if at.x < -5.5 {
            return;
        }
    }
    panic!("the body did not cross: {}", feet(&game));
}

/// `art/district.gltf` is made from `city::district()`. After a change to the
/// city's tables, write it again:
///
/// ```sh
/// EXPORT=1 cargo test --manifest-path .shells/Cargo.toml -p exact-beat-em-up-logic --test city district
/// ```
#[test]
fn the_district_file_is_what_the_code_builds() {
    let art = concat!(env!("CARGO_MANIFEST_DIR"), "/../art");
    let (json, bin) = kit::gltf(&city::district().model(), "district");
    if std::env::var("EXPORT").is_ok() {
        std::fs::write(format!("{art}/district.gltf"), &json).unwrap();
        std::fs::write(format!("{art}/district.bin"), &bin).unwrap();
    }
    let on_disk = std::fs::read_to_string(format!("{art}/district.gltf")).unwrap_or_default();
    assert!(on_disk == json, "art/district.gltf is stale: run this test with EXPORT=1");
    let on_disk = std::fs::read(format!("{art}/district.bin")).unwrap_or_default();
    assert!(on_disk == bin, "art/district.bin is stale: run this test with EXPORT=1");
}

fn action(game: &Game) -> Action {
    game.get::<Parkour>("player").unwrap().action.clone()
}

/// Run the body to `to` (its feet) as a player would: push the stick at it, press
/// Jump when a wall stops the body below the place it wants, and press Jump again
/// on a hang to climb. With `lip`, press Jump once when the feet pass that Z going
/// north: the jump over a gap.
#[track_caller]
fn leg(game: &mut Game, what: &str, to: Vec3, lip: Option<f32>) {
    let mut stopped = 0.0;
    let mut since_jump = 1000.0;
    let mut leapt = false;
    for _ in 0..(12_000 / 25) {
        let at = feet(game);
        let flat = Vec3::new(to.x - at.x, 0.0, to.z - at.z);
        let doing = action(game);
        if flat.length() < 0.5 && (at.y - to.y).abs() < 0.12 && matches!(doing, Action::Ground) {
            return;
        }
        steer(game, flat);
        let speed = {
            let v = game.get::<CapsuleController>("player").unwrap().velocity;
            Vec3::new(v.x, 0.0, v.z).length()
        };
        stopped = if speed < 0.7 { stopped + 25.0 } else { 0.0 };
        since_jump += 25.0;
        let wants_up = to.y > at.y + 0.3;
        let jump = match doing {
            Action::Hang(_) => since_jump > 350.0,
            Action::Ground => {
                (wants_up && stopped > 150.0 && since_jump > 500.0)
                    || lip.is_some_and(|z| !leapt && at.z < z)
            }
            _ => false,
        };
        if jump {
            leapt |= lip.is_some();
            game.tap("Space");
            since_jump = 0.0;
        }
        game.run(25.0);
    }
    panic!("{what}: at {} doing {:?}, wanted {to}", feet(game), action(game));
}

/// The whole route with the stick and the Jump button only, from the street to
/// the sign. Each leg ends where the next begins.
#[test]
fn the_route_can_be_run_from_the_street_to_the_sign() {
    let mut game = start();
    game.run(200.0);
    let roof = |name: &str| city::named(name).roof;
    let deck = city::WALKWAY_AT.y + kit::WALKWAY_DECK;
    let bridge = city::SKYBRIDGE_AT.y + kit::SKYBRIDGE_TOP;
    let escape = city::ESCAPE_AT;
    let legs: &[(&str, Vec3, Option<f32>)] = &[
        ("into the street", Vec3::new(-1.5, 0.0, 3.0), None),
        ("to the crate", Vec3::new(-4.3, 0.0, -0.5), None),
        ("onto the crate", Vec3::new(city::CRATE_AT.x, city::CRATE.y, city::CRATE_AT.z), None),
        ("onto the stall", Vec3::new(-4.7, kit::STALL_ROOF, -4.2), None),
        ("onto west-low", Vec3::from_array(ROUTE[0].at), None),
        ("to the walkway", Vec3::new(-7.8, roof("west-low"), city::WALKWAY_AT.z), None),
        ("onto its deck", Vec3::new(-5.4, deck, city::WALKWAY_AT.z), None),
        ("through the walkway", Vec3::from_array(ROUTE[1].at), None),
        ("to the fire escape", escape + Vec3::new(-1.9, kit::ESCAPE_LOW, 0.9), None),
        ("up its steps", escape + Vec3::new(1.9, kit::ESCAPE_MID, 1.15), None),
        ("onto its top landing", escape + Vec3::new(1.4, kit::ESCAPE_TOP, -0.9), None),
        ("onto east-high", Vec3::from_array(ROUTE[2].at), None),
        ("to the skybridge", Vec3::new(8.3, roof("east-high"), city::SKYBRIDGE_AT.z), None),
        ("onto its roof", Vec3::new(5.4, bridge, city::SKYBRIDGE_AT.z), None),
        ("over the skybridge", Vec3::new(-5.6, bridge, city::SKYBRIDGE_AT.z), None),
        ("onto west-high", Vec3::from_array(ROUTE[3].at), None),
        ("to the alley", Vec3::new(-7.5, roof("west-high"), -27.4), None),
        ("over the alley", Vec3::new(-7.5, roof("north"), -33.6), Some(city::named("west-high").z[0] + 0.4)),
        ("to the sign", Vec3::from_array(ROUTE[4].at), None),
    ];
    for (i, (what, to, lip)) in legs.iter().enumerate() {
        if lip.is_some() {
            game.key_down("ShiftLeft");
        }
        leg(&mut game, &format!("leg {i}, {what}"), *to, *lip);
        if lip.is_some() {
            game.key_up("ShiftLeft");
        }
    }
    game.run(100.0);
    assert_eq!(game.world().resource::<CityState>().step as usize, ROUTE.len());
    let seconds = game.world().published("seconds").unwrap().as_number().unwrap();
    assert!(seconds < 120.0, "the route took {seconds} s");
}
