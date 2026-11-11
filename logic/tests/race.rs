mod bot;

use exact_beat_em_up_logic::body::HALF;
use exact_beat_em_up_logic::ledge;
use exact_beat_em_up_logic::race::{self, kit::Kind};
use exact_game::Vec3;

#[test]
fn every_race_has_a_name_a_route_and_a_finish() {
    let mut titles = Vec::new();
    for id in 0..race::COUNT {
        let level = race::level(id);
        assert!(!level.title.is_empty() && !level.subtitle.is_empty());
        assert!(level.checkpoints.len() >= 2, "{}: no checkpoints", level.title);
        assert!(
            level.pieces.iter().any(|p| p.kind == Kind::Solid),
            "{}: nothing to stand on",
            level.title
        );
        titles.push(level.title);
    }
    titles.dedup();
    assert_eq!(titles.len(), race::COUNT as usize, "two races share a name");
}

// A checkpoint is a place to stand: on a surface, clear of every wall, above the
// height that counts as a fall. The finish is too.
#[test]
fn every_checkpoint_is_a_place_to_stand() {
    for id in 0..race::COUNT {
        let level = race::level(id);
        let game = bot::start(id);
        let w = game.world();
        let mut names: Vec<&str> = level.pieces.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        for pair in names.windows(2) {
            assert_ne!(pair[0], pair[1], "{}: two pieces are named {}", level.title, pair[0]);
        }
        assert!(level.pieces.len() <= 900, "{}: {} pieces", level.title, level.pieces.len());
        let spots = level
            .checkpoints
            .iter()
            .enumerate()
            .map(|(i, c)| (format!("checkpoint {i}"), c.at, c.floor))
            .chain([("the finish".to_string(), level.finish, f32::MIN)]);
        for (what, at, floor) in spots {
            let what = format!("{}: {what} at {at:?}", level.title);
            let at = Vec3::from_array(at);
            let ground = ledge::ground(w, at + Vec3::Y * 0.5, 1.0);
            assert!(
                ground.is_some_and(|y| (y - at.y).abs() < 0.05),
                "{what}: the surface under it is at {ground:?}"
            );
            assert!(ledge::clear(w, at + Vec3::Y * (HALF + 0.03)), "{what}: inside a wall");
            assert!(floor < at.y - 1.5, "{what}: its fall line {floor} is too near");
            assert!(floor == f32::MIN || floor >= 3.0, "{what}: its fall line {floor} is under the street");
        }
    }
}

// A runner who leaves the roofs comes back to the last checkpoint it reached, on
// its feet, and the race goes on.
#[test]
fn a_runner_who_falls_starts_again_at_the_last_checkpoint() {
    for id in 0..race::COUNT {
        let level = race::level(id);
        let mut game = bot::start(id);
        let second = level.checkpoints[1];
        game.post(format!("checkpoint 1"));
        game.run(300.0);
        // Reaching it by any way counts.
        assert_eq!(bot::race(&game).checkpoint, 1, "{}", level.title);
        // Far out over the street, level with the roofs.
        game.post(format!("goto {} {} {} 0", second.at[0] + 300.0, second.at[1], second.at[2]));
        let back = (0..1200).any(|_| {
            game.run(bot::TICK_MS);
            bot::race(&game).falls == 1
        });
        assert!(back, "{}: the runner fell to {} and stayed", level.title, bot::feet(&game));
        game.run(600.0);
        let at = bot::feet(&game);
        let home = Vec3::from_array(second.at);
        assert!((at - home).length() < 0.3, "{}: back at {at}, not {home}", level.title);
        assert_eq!(bot::doing(&game), "ground", "{}", level.title);
        assert!(!bot::race(&game).finished);
    }
}

// Write one level as JSON for `tools/map.py`: `LEVEL=0 OUT=/tmp/level.json`.
#[test]
#[ignore]
fn dump() {
    let id: u8 = std::env::var("LEVEL").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let out = std::env::var("OUT").unwrap_or_else(|_| "/tmp/level.json".into());
    let level = race::level(id);
    let list = |v: &[f32]| {
        let parts: Vec<String> = v.iter().map(|n| format!("{n}")).collect();
        format!("[{}]", parts.join(","))
    };
    let pieces: Vec<String> = level
        .pieces
        .iter()
        .map(|p| {
            format!(
                r#"{{"name":"{}","at":{},"size":{},"yaw":{},"pitch":{},"look":"{:?}","kind":"{:?}"}}"#,
                p.name,
                list(&p.at),
                list(&p.size),
                p.yaw,
                p.pitch,
                p.look,
                p.kind
            )
        })
        .collect();
    let checkpoints: Vec<String> = level
        .checkpoints
        .iter()
        .map(|c| format!(r#"{{"at":{},"toward":{},"floor":{}}}"#, list(&c.at), list(&c.toward), c.floor))
        .collect();
    let json = format!(
        r#"{{"title":"{}","pieces":[{}],"checkpoints":[{}],"finish":{}}}"#,
        level.title,
        pieces.join(","),
        checkpoints.join(","),
        list(&level.finish)
    );
    std::fs::write(&out, json).unwrap();
    println!("{} pieces, written to {out}", level.pieces.len());
}
