//! A runner that plays a race from a script, as a player would: it pushes the
//! stick toward a point on the map and presses Jump and Slide. A level's test gives
//! it the route, and the level is sound when the runner reaches the finish without
//! a fall.
#![allow(dead_code)]
use exact_beat_em_up_logic::body::{self, Action, Parkour};
use exact_beat_em_up_logic::race::RaceState;
use exact_beat_em_up_logic::{ExactBeatEmUp, Options, RACE_MODE};
use exact_game::{InputEvent, PointerPhase, Quat, Sim, Transform, Vec3};

pub type Game = Sim<ExactBeatEmUp>;

// A little over one 120 Hz tick: each `run` of it advances exactly one.
pub const TICK_MS: f64 = 8.334;

/// One thing the runner does. A point is `(x, z)` on the map.
#[derive(Clone, Copy, Debug)]
pub enum Step {
    /// Run toward the point. Done when the runner is on its feet at the point, or
    /// past it along the way it came. A runner that catches an edge on the way
    /// climbs it, because the stick still points at the wall.
    Run(f32, f32),
    /// The same, with Sprint held.
    Sprint(f32, f32),
    /// Press Jump. Takes no time: the next step goes on.
    Jump,
    /// Press Slide. Takes no time.
    Slide,
    /// Keep the stick where it is until the hands hold an edge.
    Hang,
    /// On an edge, push the stick toward the point, which moves the hands along
    /// the edge. Done when the runner is within 0.3 m of the point on the map.
    Shimmy(f32, f32),
    /// Keep the stick where it is for this many seconds.
    Hold(f32),
    /// Let the stick go for this many seconds.
    Wait(f32),
}

pub fn start(race: u8) -> Game {
    let mut game = Sim::new(Options {
        seed: 7,
        mode: RACE_MODE,
        race: race as u32,
        ..Options::default()
    })
    .unwrap();
    game.run(100.0);
    game
}

pub fn centre(game: &Game) -> Vec3 {
    game.get::<Transform>("player").unwrap().position
}
pub fn feet(game: &Game) -> Vec3 {
    body::feet(centre(game))
}
pub fn action(game: &Game) -> Action {
    game.get::<Parkour>("player").unwrap().action.clone()
}
pub fn race(game: &Game) -> RaceState {
    game.world().resource::<RaceState>().clone()
}

/// The name of what the body does, without the data of the move.
pub fn doing(game: &Game) -> &'static str {
    match action(game) {
        Action::Ground => "ground",
        Action::Air => "air",
        Action::Vault(_) => "vault",
        Action::Hang(_) => "hang",
        Action::Climb(_) => "climb",
        Action::Roll { .. } => "roll",
        Action::Slide { .. } => "slide",
        Action::Land { .. } => "land",
        Action::Ragdoll => "ragdoll",
        Action::Rise { .. } => "rise",
    }
}

/// Push the stick fully along the world direction `dir`, as a thumb would; zero
/// lets it go.
pub fn steer(game: &mut Game, dir: Vec3) {
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
        let d = Quat::from_rotation_y(-yaw) * dir.normalize() * 60.0;
        game.input(event(PointerPhase::Down, 100.0, 100.0));
        game.input(event(PointerPhase::Move, 100.0 + d.x, 100.0 + d.z));
    }
}

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

/// The longest one step may take, seconds.
const PATIENCE: f32 = 12.0;

/// Play `steps` from where the runner stands. `Ok` holds one line for each step:
/// when it ended and where. `Err` says which step the runner did not finish, or
/// where it fell.
pub fn follow(game: &mut Game, steps: &[Step]) -> Result<Vec<String>, String> {
    let mut trail = Vec::new();
    let result = walk(game, steps, &mut trail);
    // `TRAIL` names a file for the runner's path, which `tools/map.py` draws.
    if let Ok(path) = std::env::var("TRAIL") {
        let lines: Vec<String> = trail.iter().map(|p| format!("{} {} {}", p.x, p.y, p.z)).collect();
        std::fs::write(path, lines.join("\n")).unwrap();
    }
    result
}

fn walk(game: &mut Game, steps: &[Step], trail: &mut Vec<Vec3>) -> Result<Vec<String>, String> {
    let trace = std::env::var("TRACE").is_ok();
    let mut log = Vec::new();
    let mut ticks = 0u32;
    let mut pushed = Vec3::ZERO;
    let mut sprinting = false;
    let seconds = |ticks: u32| ticks as f32 / 120.0;
    for (index, step) in steps.iter().enumerate() {
        let from = flat(feet(game));
        let began = ticks;
        let target = match *step {
            Step::Run(x, z) | Step::Sprint(x, z) | Step::Shimmy(x, z) => Some(Vec3::new(x, 0.0, z)),
            _ => None,
        };
        match *step {
            Step::Jump => {
                game.tap("Space");
                log.push(format!("{index:3} {step:?} at {:.2}s {:.2}", seconds(ticks), feet(game)));
                continue;
            }
            Step::Slide => {
                game.tap("KeyC");
                log.push(format!("{index:3} {step:?} at {:.2}s {:.2}", seconds(ticks), feet(game)));
                continue;
            }
            Step::Sprint(..) if !sprinting => {
                game.key_down("ShiftLeft");
                sprinting = true;
            }
            Step::Run(..) if sprinting => {
                game.key_up("ShiftLeft");
                sprinting = false;
            }
            Step::Wait(_) => {
                steer(game, Vec3::ZERO);
                pushed = Vec3::ZERO;
            }
            _ => {}
        }
        let way = target.map(|t| (t - from).normalize_or_zero());
        loop {
            let at = flat(feet(game));
            let since = seconds(ticks - began);
            let done = match *step {
                Step::Run(..) | Step::Sprint(..) => {
                    let (target, way) = (target.unwrap(), way.unwrap());
                    let near = (at - target).length() < 0.15 || (at - target).dot(way) > -0.05;
                    near && matches!(action(game), Action::Ground)
                }
                Step::Hang => matches!(action(game), Action::Hang(_)),
                Step::Shimmy(..) => (at - target.unwrap()).length() < 0.3,
                Step::Hold(time) | Step::Wait(time) => since >= time,
                Step::Jump | Step::Slide => unreachable!(),
            };
            if done {
                break;
            }
            if since > PATIENCE {
                return Err(format!(
                    "step {index} {step:?}: not done after {PATIENCE} s; the runner is at {:.2} doing {}\n{}",
                    feet(game),
                    doing(game),
                    log.join("\n")
                ));
            }
            if let Some(target) = target {
                // The stick is pushed again only when the way to the point has
                // turned: a held stick keeps a straight path.
                let want = (target - at).normalize_or_zero();
                // A body that hangs stays on its edge until a new push: let the
                // stick go and push it again, as a player does.
                let hanging = matches!(action(game), Action::Hang(h) if h.time > 0.3);
                if hanging && pushed != Vec3::ZERO && ticks % 36 == 0 && matches!(step, Step::Run(..) | Step::Sprint(..)) {
                    steer(game, Vec3::ZERO);
                    pushed = Vec3::ZERO;
                } else if want != Vec3::ZERO && (pushed == Vec3::ZERO || want.dot(pushed) < 0.9986) {
                    steer(game, want);
                    pushed = want;
                }
            }
            let before = feet(game);
            game.run(TICK_MS);
            ticks += 1;
            let now = feet(game);
            if !now.is_finite() {
                return Err(format!("step {index} {step:?}: the runner is at {now}"));
            }
            if race(game).falls > 0 {
                return Err(format!(
                    "step {index} {step:?}: the runner fell; it was last at {before:.2} doing {}\n{}",
                    doing(game),
                    log.join("\n")
                ));
            }
            if ticks % 4 == 0 {
                trail.push(now);
            }
            if trace && ticks % 12 == 0 {
                eprintln!("{:6.2}s step {index:3} {:.2} {}", seconds(ticks), now, doing(game));
            }
        }
        log.push(format!("{index:3} {step:?} done at {:.2}s {:.2}", seconds(ticks), feet(game)));
    }
    Ok(log)
}

/// Play the whole of race `id` with `steps` and require the finish, with no fall.
/// Returns the time of the run, seconds.
#[track_caller]
pub fn finish(id: u8, steps: &[Step]) -> f32 {
    let mut game = start(id);
    let log = match follow(&mut game, steps) {
        Ok(log) => log,
        Err(why) => panic!("{why}"),
    };
    // The finish is seen on the tick after the feet reach it.
    game.run(100.0);
    let state = race(&game);
    assert!(
        state.finished,
        "the route ended at {:.2} and not in the finish gate\n{}",
        feet(&game),
        log.join("\n")
    );
    assert_eq!(state.falls, 0);
    let time = (state.ended - state.started) as f32 / 120.0;
    println!("{}\nfinished in {time:.1} s, at checkpoint {}", log.join("\n"), state.checkpoint);
    time
}
