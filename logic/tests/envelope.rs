//! What the runner can cross: the widest gap it jumps for each change of height,
//! at a run and at a sprint. A level is designed inside this table.
mod bot;

use bot::{action, feet, steer, Game, TICK_MS};
use exact_beat_em_up_logic::body::Action;
use exact_beat_em_up_logic::race::kit::{Level, Look};
use exact_beat_em_up_logic::race::{self, yard, YARD};
use exact_game::Vec3;

// The yard's roof ends at this Z in the north.
const LIP: f32 = -20.0;

// Run north off the yard's roof at a roof `rise` higher across `gap` metres, and
// jump `early` metres before the lip. Returns how the runner arrived.
fn cross(gap: f32, rise: f32, sprint: bool, early: f32) -> Option<&'static str> {
    let mut game: Game = bot::start(YARD);
    let mut level = Level::new("", "", "#ffffff");
    let top = yard::ROOF + rise;
    level.building("far", [0.0, LIP - gap - 5.0], [8.0, 10.0], top);
    for piece in &level.pieces {
        race::spawn(game.world_mut(), piece, "#ffffff");
    }
    game.post("goto 0 20 -8 180");
    game.run(100.0);
    steer(&mut game, -Vec3::Z);
    if sprint {
        game.key_down("ShiftLeft");
    }
    let mut jumped = false;
    let mut hung = false;
    let mut hardest = "feet";
    for _ in 0..(8000.0 / TICK_MS) as u32 {
        game.run(TICK_MS);
        let at = feet(&game);
        if !jumped && at.z < LIP + early {
            game.tap("Space");
            jumped = true;
        }
        match action(&game) {
            Action::Hang(_) | Action::Climb(_) => hung = true,
            Action::Roll { .. } => hardest = "roll",
            Action::Ragdoll => hardest = "fall",
            _ => {}
        }
        if at.y < yard::ROOF + rise.min(0.0) - 1.0 {
            return None;
        }
        let across = at.z < LIP - gap - 0.3 && (at.y - top).abs() < 0.05;
        if across && matches!(action(&game), Action::Ground) {
            return Some(if hung { "climb" } else { hardest });
        }
    }
    None
}

#[test]
#[ignore]
fn envelope() {
    // One letter for each gap: f lands on its feet, r rolls out of the landing,
    // c catches the far edge and climbs, X falls hard and lies there, - falls short.
    let gaps: Vec<f32> = (1..=18).map(|i| i as f32 * 0.5).collect();
    let header: Vec<String> = gaps.iter().map(|g| format!("{g:>4.1}")).collect();
    println!("rise  pace   gap:{}", header.join(""));
    for rise in [-6.0, -5.0, -4.0, -3.0, -2.0, -1.0, 0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 2.8, 3.0] {
        for sprint in [false, true] {
            let row: Vec<String> = gaps
                .iter()
                .map(|&gap| {
                    let mark = match cross(gap, rise, sprint, 0.35) {
                        Some("feet") => "f",
                        Some("roll") => "r",
                        Some("climb") => "c",
                        Some(_) => "X",
                        None => "-",
                    };
                    format!("{mark:>4}")
                })
                .collect();
            let pace = if sprint { "sprint" } else { "run" };
            println!("{rise:5.1} {pace:7}    {}", row.join(""));
        }
    }
}

// The widest gap must not depend on a perfect press: a jump pressed a little early
// still crosses a gap one metre under the widest.
#[test]
fn a_sprint_jump_crosses_four_metres_between_level_roofs() {
    for early in [0.2, 0.35, 0.8] {
        assert!(cross(3.5, 0.0, true, early).is_some(), "pressed {early} m early");
    }
    assert!(cross(2.0, 0.0, false, 0.35).is_some());
    assert_eq!(cross(2.0, 2.0, true, 0.35), Some("climb"));
}

#[test]
fn a_gate_stops_a_runner_and_passes_a_slide() {
    for slide in [false, true] {
        let mut game: Game = bot::start(YARD);
        let mut level = Level::new("", "", "#ffffff");
        level.gate("gate", [0.0, yard::ROOF, 0.0], 2.0, 0.0);
        // Walls beside the posts close the roof.
        level.on("wall-west", [-3.9, yard::ROOF, 0.0], [5.0, 2.6, 0.4], Look::Trim);
        level.on("wall-east", [3.9, yard::ROOF, 0.0], [5.0, 2.6, 0.4], Look::Trim);
        for piece in &level.pieces {
            race::spawn(game.world_mut(), piece, "#ffffff");
        }
        game.post("goto 0 20 8 180");
        game.run(100.0);
        steer(&mut game, -Vec3::Z);
        game.key_down("ShiftLeft");
        let mut slid = false;
        let mut lowest = f32::MAX;
        for _ in 0..(4000.0 / TICK_MS) as u32 {
            game.run(TICK_MS);
            let at = feet(&game);
            if slide && !slid && at.z < 2.5 {
                game.tap("KeyC");
                slid = true;
            }
            if at.z.abs() < 0.3 {
                let hips = exact_beat_em_up_logic::hold::joint(game.world(), "hips").y;
                let head = exact_beat_em_up_logic::hold::joint(game.world(), "head").y;
                lowest = lowest.min(hips.max(head) - yard::ROOF);
                assert!(
                    matches!(action(&game), Action::Slide { .. }),
                    "under the bar doing {:?}",
                    action(&game)
                );
            }
        }
        let at = feet(&game);
        if slide {
            assert!(at.z < -3.0, "the slide did not pass the gate: {at}");
            assert!(matches!(action(&game), Action::Ground));
            assert!(lowest < race::kit::BAR_CLEAR, "the head was {lowest} m up under the bar");
        } else {
            assert!(at.z > 0.3, "the runner went through the bar: {at}");
        }
    }
}

// Every kind of piece the kit builds, in one course on the yard: a stair, a
// ramp, a wall to vault, a wall to climb, a drop and a slide gate.
#[test]
fn a_runner_finishes_a_course_of_every_piece() {
    use bot::Step::*;
    let roof = yard::ROOF;
    let mut game: Game = bot::start(YARD);
    let mut level = Level::new("", "", "#ffffff");
    level.stairs("stair", [0.0, roof, 14.0], [0.0, roof + 1.5, 10.0], 3.0, Look::Ramp);
    level.on("landing", [0.0, roof, 8.0], [6.0, 1.5, 4.0], Look::Concrete);
    level.ramp("ramp", [0.0, roof + 1.5, 6.0], [0.0, roof, 2.0], 3.0, 0.3, Look::Ramp);
    level.on("vault", [0.0, roof, -2.0], [6.0, 0.9, 0.5], Look::Vault);
    level.on("climb", [0.0, roof, -8.0], [8.0, 2.6, 3.0], Look::Climb);
    level.gate("gate", [0.0, roof, -14.0], 2.0, 0.0);
    level.on("wall-west", [-3.9, roof, -14.0], [5.0, 2.6, 0.4], Look::Trim);
    level.on("wall-east", [3.9, roof, -14.0], [5.0, 2.6, 0.4], Look::Trim);
    for piece in &level.pieces {
        race::spawn(game.world_mut(), piece, "#ffffff");
    }
    let route = [
        Run(0.0, 14.3),
        Run(0.0, 8.0),
        Run(0.0, 1.0),
        Run(0.0, -1.1),
        Jump,
        Run(0.0, -4.5),
        Run(0.0, -5.9),
        Jump,
        Run(0.0, -9.0),
        Run(0.0, -11.0),
        Sprint(0.0, -11.6),
        Slide,
        Sprint(0.0, -16.5),
        Run(0.0, -18.8),
    ];
    let log = bot::follow(&mut game, &route).unwrap_or_else(|why| panic!("{why}"));
    game.run(100.0);
    println!("{}", log.join("\n"));
    assert!(bot::race(&game).finished, "ended at {}", feet(&game));
    assert_eq!(bot::race(&game).checkpoint, 1);
}

// Slide down a ramp of `slope` degrees that loses `drop` metres, cross `runout`
// metres of level roof, and jump at the lip over `gap` metres to a roof `rise`
// higher. Returns the speed at the foot of the ramp, the speed at the lip, how
// the runner arrived, if it did, and its speed a quarter of a second later. With `slide` false the runner sprints down.
fn launch(slope: f32, drop: f32, runout: f32, gap: f32, rise: f32, slide: bool) -> (f32, f32, Option<&'static str>, f32) {
    let roof = yard::ROOF;
    let mut game: Game = bot::start(YARD);
    let mut level = Level::new("", "", "#ffffff");
    let foot = LIP + runout;
    let head = foot + drop / slope.to_radians().tan();
    level.on("deck", [0.0, roof, head + 4.0], [6.0, drop, 8.0], Look::Concrete);
    level.ramp("chute", [0.0, roof + drop, head], [0.0, roof, foot], 4.0, 0.3, Look::Ramp);
    let top = roof + rise;
    level.building("far", [0.0, LIP - gap - 12.0], [10.0, 24.0], top);
    for piece in &level.pieces {
        race::spawn(game.world_mut(), piece, "#ffffff");
    }
    game.post(format!("goto 0 {} {} 180", roof + drop, head + 6.0));
    game.run(100.0);
    steer(&mut game, -Vec3::Z);
    game.key_down("ShiftLeft");
    let (mut slid, mut jumped, mut hung) = (false, false, false);
    let (mut at_foot, mut at_lip) = (0.0, 0.0);
    let mut hardest = "feet";
    let speed = |g: &Game| {
        let v = g.get::<exact_game_physics::CapsuleController>("player").unwrap().velocity;
        Vec3::new(v.x, 0.0, v.z).length()
    };
    for _ in 0..(12000.0 / TICK_MS) as u32 {
        game.run(TICK_MS);
        let at = feet(&game);
        if slide && !slid && at.z < head - 1.0 {
            game.tap("KeyC");
            slid = true;
        }
        if at_foot == 0.0 && at.z < foot {
            at_foot = speed(&game);
        }
        if !jumped && at.z < LIP + 0.35 {
            at_lip = speed(&game);
            game.tap("Space");
            jumped = true;
        }
        match action(&game) {
            Action::Hang(_) | Action::Climb(_) => hung = true,
            Action::Roll { .. } => hardest = "roll",
            Action::Ragdoll => hardest = "fall",
            _ => {}
        }
        if jumped && at.y < roof + rise.min(0.0) - 1.0 {
            return (at_foot, at_lip, None, 0.0);
        }
        let across = at.z < LIP - gap - 0.3 && (at.y - top).abs() < 0.05;
        if across && matches!(action(&game), Action::Ground | Action::Slide(_)) {
            game.run(250.0);
            return (at_foot, at_lip, Some(if hung { "climb" } else { hardest }), speed(&game));
        }
    }
    (at_foot, at_lip, None, 0.0)
}

// What a slide down a ramp gives: the speed at its foot, and the gap that speed
// crosses. Run it with `--ignored --nocapture`.
#[test]
#[ignore]
fn slide_envelope() {
    println!("speed at the foot of a ramp, m/s (sprint in, slide pressed 1 m down the ramp)");
    println!("slope   drop:    2 m    4 m    6 m    8 m   12 m   | sprint down, no slide: 4 m");
    for slope in [6.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0] {
        let row: Vec<String> = [2.0, 4.0, 6.0, 8.0, 12.0]
            .iter()
            .map(|&drop| format!("{:6.1}", launch(slope, drop, 3.0, 2.0, 0.0, true).0))
            .collect();
        let plain = launch(slope, 4.0, 3.0, 2.0, 0.0, false).0;
        println!("{slope:5.0}        {}   | {plain:6.1}", row.join(" "));
    }
    println!("\nspeed at the lip after a level run-out, from a 20 degree ramp that drops 8 m");
    for runout in [1.0, 3.0, 6.0, 10.0, 15.0] {
        let (foot, lip, _, _) = launch(20.0, 8.0, runout, 2.0, 0.0, true);
        println!("run-out {runout:4.0} m: foot {foot:4.1}  lip {lip:4.1}");
    }
    println!("\nspeed a quarter of a second after the landing, from a lip speed of 9.5 m/s, over a 5 m gap");
    for rise in [-3.0, -2.0, -1.5, -1.0, -0.5, 0.0] {
        let (_, _, how, after) = launch(20.0, 8.0, 3.0, 5.0, rise, true);
        println!("rise {rise:5.1}: {how:?} then {after:4.1} m/s");
    }
    println!("\ngaps after a slide (20 degree ramp, 3 m run-out). f feet, r roll, c catch, X hard fall, - short");
    let gaps: Vec<f32> = (2..=16).map(|i| i as f32).collect();
    let header: Vec<String> = gaps.iter().map(|g| format!("{g:>3.0}")).collect();
    println!("drop of ramp  rise  lip m/s  gap:{}", header.join(""));
    for drop in [2.0, 4.0, 8.0] {
        for rise in [-3.0, -2.0, -1.0, 0.0, 1.0, 2.0] {
            let mut lip = 0.0;
            let row: Vec<String> = gaps
                .iter()
                .map(|&gap| {
                    let (_, at_lip, how, _) = launch(20.0, drop, 3.0, gap, rise, true);
                    lip = at_lip;
                    let mark = match how {
                        Some("feet") => "f",
                        Some("roll") => "r",
                        Some("climb") => "c",
                        Some(_) => "X",
                        None => "-",
                    };
                    format!("{mark:>3}")
                })
                .collect();
            println!("{drop:8.0} m  {rise:5.1}  {lip:6.1}      {}", row.join(""));
        }
    }
}

// A valley: slide down a 20 degree ramp that drops `drop`, cross 2 m of level
// roof, and go up a ramp of `slope` degrees that gains `gain`. Returns the
// seconds from the foot of the first ramp to the top of the second, and the
// speed there. `again` presses Slide a second time at the foot of the up ramp.
fn valley(drop: f32, slope: f32, gain: f32, slide: bool, again: bool) -> Option<(f32, f32)> {
    let roof = yard::ROOF;
    let mut game: Game = bot::start(YARD);
    let mut level = Level::new("", "", "#ffffff");
    let foot = 0.0;
    let head = foot + drop / 20.0_f32.to_radians().tan();
    let up_foot = foot - 2.0;
    let up_head = up_foot - gain / slope.to_radians().tan();
    level.on("deck", [0.0, roof, head + 4.0], [6.0, drop, 8.0], Look::Concrete);
    level.ramp("down", [0.0, roof + drop, head], [0.0, roof, foot], 4.0, 0.3, Look::Ramp);
    level.ramp("up", [0.0, roof, up_foot], [0.0, roof + gain, up_head], 4.0, 0.3, Look::Ramp);
    level.on("top", [0.0, roof, up_head - 5.0], [6.0, gain, 10.0], Look::Concrete);
    for piece in &level.pieces {
        race::spawn(game.world_mut(), piece, "#ffffff");
    }
    game.post(format!("goto 0 {} {} 180", roof + drop, head + 6.0));
    game.run(100.0);
    steer(&mut game, -Vec3::Z);
    game.key_down("ShiftLeft");
    let (mut slid, mut twice) = (false, false);
    let mut since = None;
    for tick in 0..(15000.0 / TICK_MS) as u32 {
        game.run(TICK_MS);
        let at = feet(&game);
        if slide && !slid && at.z < head - 1.0 {
            game.tap("KeyC");
            slid = true;
        }
        if again && !twice && at.z < up_foot - 0.5 {
            game.tap("KeyC");
            twice = true;
        }
        if since.is_none() && at.z < foot {
            since = Some(tick);
        }
        if at.z < up_head - 0.5 && (at.y - roof - gain).abs() < 0.1 {
            let v = game.get::<exact_game_physics::CapsuleController>("player").unwrap().velocity;
            return Some(((tick - since?) as f32 / 120.0, Vec3::new(v.x, 0.0, v.z).length()));
        }
    }
    None
}

#[test]
#[ignore]
fn valley_envelope() {
    println!("up a ramp after a slide down 8 m: seconds from foot to top, and the speed at the top");
    println!("slope  gain   sprint only     slide, then run   slide, and slide again on the up ramp");
    let show = |r: Option<(f32, f32)>| match r {
        Some((t, v)) => format!("{t:4.1} s {v:4.1} m/s"),
        None => "   never    ".into(),
    };
    for slope in [10.0, 15.0, 20.0, 30.0] {
        for gain in [2.0, 3.0, 4.0, 6.0] {
            println!(
                "{slope:4.0}  {gain:4.0}   {}   {}   {}",
                show(valley(8.0, slope, gain, false, false)),
                show(valley(8.0, slope, gain, true, false)),
                show(valley(8.0, slope, gain, true, true)),
            );
        }
    }
}
