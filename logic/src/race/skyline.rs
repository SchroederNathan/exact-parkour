//! Skyline Sprint: the first race. A downtown office district in the morning.
//!
//! The route runs north in a gentle S. It starts on Meridian House, 36 m up, loses
//! height roof by roof, and climbs once more to a helipad on the podium of Beacon
//! Tower. The tower stands at the far end and is in view from the start.
//!
//! Each move appears once where a mistake costs nothing. Then it appears over a
//! gap. Then it is joined to another move.
//!
//! The beats, in order:
//!
//!  1. Meridian House (36 m). The start. A wide roof and one orange wall in its
//!     middle. Teaches the vault. A player can also walk round the wall.
//!  2. The first alley, 2 m, between level roofs. Teaches the jump. A run crosses it.
//!  3. The Annex (36 m). A long straight run to a slide gate in a screen wall.
//!     Teaches the slide. Checkpoint 1 is behind the gate.
//!  4. The Print Works, the Glasshouse, the Counting House and the Corner House
//!     (35, 34, 33, 32 m). Four sprint jumps of 2.5, 3, 3.5 and 4 m, each 1 m down,
//!     on a line that bends east. The third roof is short, so the last two jumps
//!     come close together. Tests the jump. It is the first rhythm of the level.
//!     Checkpoint 2 is half way.
//!  5. The pipe bridge. The avenue is 14 m wide and no jump crosses it. A yellow
//!     deck does. Teaches the beam. Parapets beside it show the way on, and
//!     checkpoint 3 is at its mouth.
//!  6. The Exchange hall (32 m) to its north wing (29.4 m). The roof ends and the
//!     runner drops 2.6 m into a roll that keeps the speed.
//!  7. The kicker. Checkpoint 4 is at its foot. A sand ramp goes up over a stair
//!     head, and a sprint jump from its top crosses a 4 m alley to the Chancery
//!     (29.9 m). Joins the ramp and the jump. It is the biggest air of the level.
//!     Checkpoint 5 is at the landing.
//!  8. The Chancery. An orange wall, and 6 m after it the edge: vault, then jump
//!     4 m to the Custom House (28.9 m). Joins the vault and the jump.
//!  9. The Custom House. A second slide gate close behind the landing. Checkpoint 6.
//!     Then the teal wall of the attic, 2.4 m: the one climb.
//! 10. The attic (31.3 m) to the podium of Beacon Tower (30.3 m). A sprint jump of
//!     4 m, then a wide stair to the helipad (31.8 m) and the finish. The reward.
//!
//! The fast line: on the Chancery, east of the orange wall, a sprint jump of 4 m
//! between level surfaces reaches a yellow pad on the corner of the Custom House.
//! A catwalk 1.2 m wide goes up from it to the attic. It passes the slide gate and
//! the climb, and it misses checkpoint 6. A runner who slips off it to the west
//! lands on the Custom House and goes on by the safe way.
//!
//! No gap needs a sprint. A runner who only runs crosses each one by a landing or
//! by a catch of the far edge, and `logic/tests/run_skyline.rs` proves it.
//!
//! Neighbours that are off the route are either 6 m or more above the roofs next
//! to them, or under the fall line.
use super::kit::{Level, Look};

/// The height and the thickness of a parapet.
const PARAPET: [f32; 2] = [0.6, 0.4];

// A parapet whose centre line goes from `a` to `b` on the map, along X or along Z.
// Give the line 0.25 m inside the edge of the roof.
fn parapet(level: &mut Level, name: &str, a: [f32; 2], b: [f32; 2], roof: f32) {
    let middle = [(a[0] + b[0]) / 2.0, roof, (a[1] + b[1]) / 2.0];
    let size = if a[0] == b[0] {
        [PARAPET[1], PARAPET[0], (b[1] - a[1]).abs()]
    } else {
        [(b[0] - a[0]).abs(), PARAPET[0], PARAPET[1]]
    };
    level.on(name, middle, size, Look::Trim);
}

// A stair head: a small room with a sloped roof. `door` is +1 when its high side
// faces south and -1 when it faces north.
fn stair_head(level: &mut Level, name: &str, at: [f32; 2], roof: f32, door: f32) {
    level.on(name, [at[0], roof, at[1]], [3.6, 2.2, 4.6], Look::Trim);
    level.ramp(
        &format!("{name}-roof"),
        [at[0], roof + 3.0, at[1] + door * 2.5],
        [at[0], roof + 2.3, at[1] - door * 2.5],
        4.0,
        0.25,
        Look::Trim,
    );
}

// A water tank on four legs.
fn tank(level: &mut Level, name: &str, at: [f32; 2], roof: f32) {
    for (leg, [x, z]) in [[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]].into_iter().enumerate() {
        level.on(
            &format!("{name}-leg-{leg}"),
            [at[0] + x, roof, at[1] + z],
            [0.3, 1.6, 0.3],
            Look::Trim,
        );
    }
    level.on(&format!("{name}-drum"), [at[0], roof + 1.6, at[1]], [2.8, 2.4, 2.8], Look::Trim);
    level.on(&format!("{name}-lid"), [at[0], roof + 4.0, at[1]], [3.2, 0.2, 3.2], Look::Trim);
}

// A row of `count` air units that goes north from `at`, one each `step` metres.
// A unit near the route has a fan on it; a far one is a plain box.
fn units(level: &mut Level, name: &str, at: [f32; 2], roof: f32, count: usize, step: f32, near: bool) {
    for unit in 0..count {
        let z = at[1] - unit as f32 * step;
        level.on(&format!("{name}-{unit}"), [at[0], roof, z], [2.2, 1.4, 1.8], Look::Trim);
        if near {
            level.on(&format!("{name}-{unit}-fan"), [at[0], roof + 1.4, z], [1.2, 0.2, 1.2], Look::Slate);
        }
    }
}

// A skylight: a kerb with glass on it.
fn skylight(level: &mut Level, name: &str, at: [f32; 2], size: [f32; 2], roof: f32) {
    level.on(name, [at[0], roof, at[1]], [size[0], 0.45, size[1]], Look::Trim);
    level.on(
        &format!("{name}-glass"),
        [at[0], roof + 0.45, at[1]],
        [size[0] - 0.5, 0.2, size[1] - 0.5],
        Look::Glass,
    );
}

// Bands round a tower at the heights `at`: the lines of its floors. Nothing
// touches them.
fn bands(level: &mut Level, name: &str, middle: [f32; 2], size: [f32; 2], at: &[f32]) {
    for (band, &height) in at.iter().enumerate() {
        level
            .on(
                &format!("{name}-band-{band}"),
                [middle[0], height, middle[1]],
                [size[0] + 0.5, 0.5, size[1] + 0.5],
                Look::Trim,
            )
            .scenery();
    }
}

// A mast that nothing touches.
fn mast(level: &mut Level, name: &str, at: [f32; 2], base: f32, tall: f32) {
    level.on(name, [at[0], base, at[1]], [0.3, tall, 0.3], Look::Trim).scenery();
}

pub fn level() -> Level {
    let mut level = Level::new(
        "Skyline Sprint",
        "Office roofs, a pipe bridge and a helipad",
        "#ff4f9a",
    );
    south_block(&mut level);
    avenue(&mut level);
    north_block(&mut level);
    beacon_tower(&mut level);
    neighbours(&mut level);
    level
}

// Beats 1 to 4: Meridian House, the Annex and the four roofs of the first rhythm.
fn south_block(level: &mut Level) {
    // Beat 1. Meridian House: the start, and the first vault in the middle of
    // the roof.
    let roof = 36.0;
    level.building("meridian", [0.0, 129.0], [28.0, 28.0], roof);
    // The fall line of a checkpoint is under every roof a runner can reach
    // before the next checkpoint that it cannot miss.
    level.fall_below(33.0);
    level.checkpoint([0.0, roof, 138.0], [0.0, 113.0]);
    level.on("meridian-vault", [0.0, roof, 127.0], [10.0, 0.9, 0.5], Look::Vault);
    parapet(level, "meridian-south", [-14.0, 142.75], [14.0, 142.75], roof);
    parapet(level, "meridian-west", [-13.75, 115.0], [-13.75, 143.0], roof);
    parapet(level, "meridian-east", [13.75, 115.0], [13.75, 143.0], roof);
    // The north edge is open where the Annex stands across the alley.
    parapet(level, "meridian-north-west", [-14.0, 115.25], [-10.5, 115.25], roof);
    parapet(level, "meridian-north-east", [10.5, 115.25], [14.0, 115.25], roof);
    stair_head(level, "meridian-stairs", [-9.5, 137.0], roof, -1.0);
    tank(level, "meridian-tank", [9.5, 137.5], roof);
    units(level, "meridian-air", [10.5, 129.0], roof, 3, 3.6, true);
    skylight(level, "meridian-light", [-9.5, 124.0], [4.0, 8.0], roof);

    // Beats 2 and 3. The alley is 2 m. The Annex: a long run to the first gate.
    level.building("annex", [0.0, 98.0], [20.0, 30.0], roof).look(Look::Brick);
    level.gate("annex-gate", [0.0, roof, 97.0], 2.4, 0.0);
    level.on("annex-screen-west", [-5.8, roof, 97.0], [8.4, 3.0, 0.4], Look::Trim);
    level.on("annex-screen-east", [5.8, roof, 97.0], [8.4, 3.0, 0.4], Look::Trim);
    // The gate lets no runner miss this one. The next can be missed, so the
    // fall line is under all four roofs of the rhythm.
    level.fall_below(30.5);
    level.checkpoint([0.0, roof, 89.0], [2.0, 73.0]);
    parapet(level, "annex-west", [-9.75, 83.0], [-9.75, 113.0], roof);
    parapet(level, "annex-east", [9.75, 83.0], [9.75, 113.0], roof);
    // The plant yard behind the screen, and skylights before it.
    units(level, "annex-air-west", [-6.5, 94.0], roof, 3, 3.2, true);
    units(level, "annex-air-east", [6.5, 94.0], roof, 2, 3.2, true);
    skylight(level, "annex-light-west", [-6.5, 105.0], [3.0, 7.0], roof);
    skylight(level, "annex-light-east", [6.5, 105.0], [3.0, 7.0], roof);

    // Beat 4. Four roofs, each 1 m lower, across 2.5, 3, 3.5 and 4 m. The third
    // roof is short: the last two jumps come close together. Each roof reaches
    // west of the line, so a runner who does not bend east still lands.
    let roof = 35.0;
    level.building("printworks", [4.0, 73.5], [20.0, 14.0], roof).look(Look::Slate);
    parapet(level, "printworks-west", [-5.75, 66.5], [-5.75, 80.5], roof);
    parapet(level, "printworks-east", [13.75, 66.5], [13.75, 80.5], roof);
    skylight(level, "printworks-light-a", [10.5, 76.5], [4.0, 3.0], roof);
    skylight(level, "printworks-light-b", [10.5, 71.0], [4.0, 3.0], roof);
    units(level, "printworks-air", [-3.6, 74.5], roof, 1, 3.0, true);

    let roof = 34.0;
    level.building("glasshouse", [8.5, 57.25], [23.0, 12.5], roof).look(Look::Glass);
    level.checkpoint([5.0, roof, 58.5], [7.0, 51.0]);
    parapet(level, "glasshouse-west", [-2.75, 51.0], [-2.75, 63.5], roof);
    parapet(level, "glasshouse-east", [19.75, 51.0], [19.75, 63.5], roof);
    units(level, "glasshouse-air", [17.0, 61.0], roof, 3, 3.4, true);
    mast(level, "glasshouse-mast", [18.5, 52.2], roof, 7.0);

    let roof = 33.0;
    level.building("countinghouse", [10.5, 43.0], [23.0, 9.0], roof).look(Look::Brick);
    parapet(level, "countinghouse-west", [-0.75, 38.5], [-0.75, 47.5], roof);
    parapet(level, "countinghouse-east", [21.75, 38.5], [21.75, 47.5], roof);
    skylight(level, "countinghouse-light", [17.0, 43.0], [4.0, 5.0], roof);

    let roof = 32.0;
    level.building("cornerhouse", [14.0, 24.25], [28.0, 20.5], roof);
    // At the mouth of the bridge, where every runner passes.
    level.fall_below(27.0);
    level.checkpoint([13.0, roof, 16.0], [13.0, 0.0]);
    parapet(level, "cornerhouse-west", [0.25, 14.0], [0.25, 34.5], roof);
    parapet(level, "cornerhouse-east", [27.75, 14.0], [27.75, 34.5], roof);
    // The avenue side is closed but for the pipe bridge.
    parapet(level, "cornerhouse-north-west", [0.0, 14.25], [12.0, 14.25], roof);
    parapet(level, "cornerhouse-north-east", [14.0, 14.25], [28.0, 14.25], roof);
    stair_head(level, "cornerhouse-stairs", [23.0, 28.0], roof, 1.0);
    tank(level, "cornerhouse-tank", [22.0, 18.5], roof);
    skylight(level, "cornerhouse-light", [4.5, 22.0], [3.0, 6.0], roof);
}

// Beat 5: the pipe bridge over the avenue, from z = 14 to z = 0.
fn avenue(level: &mut Level) {
    let top = 32.0;
    level.deck("bridge", [13.0, top, 7.0], [1.8, 0.4, 14.0], Look::Beam);
    // Its pipes, a little under the deck, and the trestle that carries it.
    for (side, x) in [("west", 11.75), ("east", 14.25)] {
        level
            .deck(&format!("bridge-pipe-{side}"), [x, top - 0.25, 7.0], [0.45, 0.45, 14.0], Look::Trim)
            .scenery();
        level
            .deck(&format!("bridge-leg-{side}"), [x, top - 0.7, 7.0], [0.5, top - 0.7, 0.5], Look::Trim)
            .scenery();
    }
    level
        .deck("bridge-tie", [13.0, top - 0.7, 7.0], [3.4, 0.4, 0.6], Look::Trim)
        .scenery();
}

// Beats 6 to 9: the Exchange, the Chancery and the Custom House.
fn north_block(level: &mut Level) {
    // Beat 6. The Exchange: the hall, and the drop to the north wing.
    let hall = 32.0;
    let roof = 29.4;
    level.building("exchange-hall", [15.0, -5.0], [26.0, 10.0], hall).look(Look::Slate);
    level.building("exchange", [15.0, -23.0], [26.0, 26.0], roof).look(Look::Slate);
    parapet(level, "hall-south-west", [2.0, -0.25], [12.0, -0.25], hall);
    parapet(level, "hall-south-east", [14.0, -0.25], [28.0, -0.25], hall);
    parapet(level, "hall-west", [2.25, -10.0], [2.25, 0.0], hall);
    parapet(level, "hall-east", [27.75, -10.0], [27.75, 0.0], hall);
    stair_head(level, "hall-stairs", [23.5, -5.0], hall, -1.0);
    skylight(level, "hall-light", [6.0, -5.0], [3.0, 5.0], hall);

    // Beat 7. The kicker: a ramp over a stair head at the north edge. The
    // checkpoint is at the foot of the ramp, where every runner passes.
    level.checkpoint([10.0, roof, -23.2], [10.0, -36.0]);
    level.ramp("kicker", [10.0, roof, -24.0], [10.0, roof + 1.5, -31.0], 4.0, 0.3, Look::Ramp);
    level.on("kicker-head", [10.0, roof, -33.5], [6.0, 1.5, 5.0], Look::Ramp);
    // The north edge is closed beside the stair head: the ramp is the way on.
    parapet(level, "exchange-north-west", [2.0, -35.75], [7.0, -35.75], roof);
    parapet(level, "exchange-north-east", [13.0, -35.75], [28.0, -35.75], roof);
    parapet(level, "exchange-west", [2.25, -36.0], [2.25, -10.0], roof);
    parapet(level, "exchange-east", [27.75, -36.0], [27.75, -10.0], roof);
    units(level, "exchange-air", [22.5, -15.5], roof, 4, 3.4, true);
    tank(level, "exchange-tank", [22.0, -31.0], roof);
    skylight(level, "exchange-light", [5.0, -20.0], [3.0, 7.0], roof);

    // Beat 8. The Chancery: a vault, and the edge 6 m after it.
    let roof = 29.9;
    level.building("chancery", [3.0, -51.0], [30.0, 22.0], roof).look(Look::Brick);
    level.checkpoint([9.5, roof, -44.5], [4.5, -62.0]);
    level.on("chancery-vault", [4.0, roof, -55.5], [12.0, 0.9, 0.5], Look::Vault);
    parapet(level, "chancery-south", [-12.0, -40.25], [2.0, -40.25], roof);
    parapet(level, "chancery-north", [-12.0, -61.75], [-4.0, -61.75], roof);
    parapet(level, "chancery-west", [-11.75, -62.0], [-11.75, -40.0], roof);
    parapet(level, "chancery-east", [17.75, -62.0], [17.75, -40.0], roof);
    stair_head(level, "chancery-stairs", [-7.0, -46.5], roof, 1.0);
    units(level, "chancery-air", [-7.5, -53.0], roof, 2, 3.4, true);
    skylight(level, "chancery-light", [15.0, -50.0], [2.6, 5.0], roof);

    // Beat 9. The Custom House: the second gate, then the climb to the attic.
    let roof = 28.9;
    let attic = 31.3;
    level.building("customhouse", [-3.0, -75.0], [26.0, 18.0], roof);
    level.building("attic", [-3.0, -88.0], [26.0, 8.0], attic);
    level.gate("customhouse-gate", [2.0, roof, -75.0], 2.4, 0.0);
    level.on("customhouse-screen-west", [-7.8, roof, -75.0], [16.4, 3.0, 0.4], Look::Trim);
    level.on("customhouse-screen-east", [6.65, roof, -75.0], [6.1, 3.0, 0.4], Look::Trim);
    level.checkpoint([2.0, roof, -79.5], [2.0, -92.0]);
    // The face of the attic where the route meets it.
    level.on("attic-climb", [1.0, roof, -83.85], [12.0, attic - roof, 0.3], Look::Climb);
    parapet(level, "customhouse-south", [-16.0, -66.25], [-4.0, -66.25], roof);
    parapet(level, "customhouse-west", [-15.75, -84.0], [-15.75, -66.0], roof);
    parapet(level, "attic-west", [-15.75, -92.0], [-15.75, -84.0], attic);
    parapet(level, "attic-north", [-16.0, -91.75], [-6.0, -91.75], attic);
    units(level, "customhouse-air", [-10.0, -79.0], roof, 2, 3.0, true);
    skylight(level, "customhouse-light", [-10.0, -70.5], [4.0, 5.0], roof);
    units(level, "attic-air", [-11.5, -86.5], attic, 1, 3.0, true);
    mast(level, "attic-mast", [-13.5, -90.0], attic, 6.0);

    // The fast line: a pad on the south-east corner, level with the Chancery,
    // and a catwalk up the east wall to the attic.
    let pad = 29.9;
    level.deck("gantry-pad", [11.7, pad, -67.75], [3.4, 0.4, 3.5], Look::Beam);
    level.ramp("gantry-walk", [10.6, pad, -69.5], [10.6, attic, -84.0], 1.2, 0.3, Look::Beam);
    level.deck("gantry-top", [10.8, attic, -86.5], [1.6, 0.3, 5.0], Look::Beam);
    for (post, z) in [-66.4, -75.0, -83.6, -88.6].into_iter().enumerate() {
        let top = if z > -70.0 { pad - 0.4 } else { pad + (attic - pad) * (-69.5 - z) / 14.5 - 0.4 };
        let top = top.min(attic - 0.3);
        level
            .deck(&format!("gantry-post-{post}"), [11.0, top, z], [0.3, top, 0.3], Look::Trim)
            .scenery();
    }
}

// Beat 10: the podium, the stair, the helipad and the tower behind it.
fn beacon_tower(level: &mut Level) {
    let roof = 30.3;
    let pad = 31.8;
    level.building("podium", [0.0, -111.0], [28.0, 30.0], roof).look(Look::Slate);
    // A stair steeper than 1 in 4.5 slows a sprint: this one is 1 in 4.7.
    level.stairs("helipad-stair", [0.0, roof, -101.5], [0.0, pad, -108.5], 6.0, Look::Ramp);
    level.on("helipad", [0.0, roof, -115.5], [14.0, pad - roof, 14.0], Look::Concrete);
    // The letter on the pad. It is paint: nothing touches it.
    level.on("helipad-h-west", [-1.6, pad, -115.5], [0.7, 0.02, 5.0], Look::Trim).scenery();
    level.on("helipad-h-east", [1.6, pad, -115.5], [0.7, 0.02, 5.0], Look::Trim).scenery();
    level.on("helipad-h-bar", [0.0, pad, -115.5], [2.5, 0.02, 0.7], Look::Trim).scenery();
    level.finish_at([0.0, pad, -115.5]);
    parapet(level, "podium-south", [-14.0, -96.25], [-8.0, -96.25], roof);
    parapet(level, "podium-west", [-13.75, -126.0], [-13.75, -96.0], roof);
    parapet(level, "podium-east", [13.75, -126.0], [13.75, -96.0], roof);
    units(level, "podium-air", [-10.5, -102.0], roof, 2, 3.4, true);
    skylight(level, "podium-light", [10.5, -113.0], [3.0, 8.0], roof);

    // The tower: the landmark of the level, with the level's colour on its face.
    level.building("beacon", [0.0, -137.0], [26.0, 22.0], 64.0).look(Look::Glass);
    level.on("beacon-crown", [0.0, 64.0, -137.0], [14.0, 5.0, 12.0], Look::Trim);
    mast(level, "beacon-mast", [0.0, -137.0], 69.0, 12.0);
    bands(level, "beacon", [0.0, -137.0], [26.0, 22.0], &[36.0, 46.0, 56.0]);
    level
        .on("beacon-sign", [0.0, 40.0, -125.8], [3.0, 22.0, 0.4], Look::Accent)
        .scenery();
}

// The rest of the district. A neighbour is far above the roofs of the route next
// to it, or far under the fall line there: no runner can stand on one and stay.
fn neighbours(level: &mut Level) {
    // West of the route, from south to north.
    level.building("west-tower", [-28.0, 128.0], [20.0, 30.0], 54.0).look(Look::Glass);
    level.on("west-tower-crown", [-28.0, 54.0, 128.0], [12.0, 4.0, 20.0], Look::Trim);
    mast(level, "west-tower-mast", [-28.0, 128.0], 58.0, 9.0);
    bands(level, "west-tower", [-28.0, 128.0], [20.0, 30.0], &[40.0, 47.0]);
    level.building("west-low", [-23.0, 96.0], [18.0, 26.0], 23.0).look(Look::Brick);
    stair_head(level, "west-low-stairs", [-20.0, 103.0], 23.0, 1.0);
    units(level, "west-low-air", [-26.0, 99.0], 23.0, 3, 4.0, false);
    level.building("west-mid", [-20.0, 63.0], [20.0, 32.0], 46.0).look(Look::Slate);
    level.on("west-mid-crown", [-21.0, 46.0, 63.0], [12.0, 3.0, 22.0], Look::Trim);
    bands(level, "west-mid", [-20.0, 63.0], [20.0, 32.0], &[39.0]);
    level.building("west-corner", [-15.0, 28.0], [20.0, 28.0], 23.0);
    stair_head(level, "west-corner-stairs", [-19.0, 35.0], 23.0, 1.0);
    skylight(level, "west-corner-light", [-11.0, 24.0], [5.0, 8.0], 23.0);
    level.building("west-glass", [-12.0, -17.0], [20.0, 34.0], 50.0).look(Look::Glass);
    level.on("west-glass-step", [-13.5, 50.0, -20.0], [13.0, 6.0, 22.0], Look::Glass);
    mast(level, "west-glass-mast", [-13.5, -20.0], 56.0, 8.0);
    bands(level, "west-glass", [-12.0, -17.0], [20.0, 34.0], &[36.0, 43.0]);
    level.building("west-brick", [-26.0, -50.0], [20.0, 24.0], 22.0).look(Look::Brick);
    tank(level, "west-brick-tank", [-22.0, -44.0], 22.0);
    skylight(level, "west-brick-light", [-28.0, -54.0], [6.0, 8.0], 22.0);
    level.building("west-slab", [-30.0, -80.0], [20.0, 28.0], 56.0).look(Look::Slate);
    level.on("west-slab-crown", [-30.0, 56.0, -80.0], [10.0, 4.0, 18.0], Look::Trim);
    bands(level, "west-slab", [-30.0, -80.0], [20.0, 28.0], &[35.0, 45.0]);
    level.building("west-annex", [-27.0, -111.0], [18.0, 26.0], 21.0);
    stair_head(level, "west-annex-stairs", [-24.0, -104.0], 21.0, 1.0);
    units(level, "west-annex-air", [-30.0, -108.0], 21.0, 3, 4.0, false);

    // East of the route, from south to north.
    level.building("east-low", [29.0, 130.0], [22.0, 26.0], 25.0).look(Look::Brick);
    tank(level, "east-low-tank", [24.0, 135.0], 25.0);
    skylight(level, "east-low-light", [32.0, 125.0], [8.0, 6.0], 25.0);
    level.building("east-tower", [24.0, 98.0], [20.0, 28.0], 58.0).look(Look::Slate);
    level.on("east-tower-step", [25.0, 58.0, 98.0], [14.0, 7.0, 18.0], Look::Slate);
    mast(level, "east-tower-mast", [25.0, 98.0], 65.0, 8.0);
    bands(level, "east-tower", [24.0, 98.0], [20.0, 28.0], &[40.0, 49.0]);
    level.building("east-mid", [34.0, 63.0], [20.0, 32.0], 22.0);
    stair_head(level, "east-mid-stairs", [30.0, 71.0], 22.0, 1.0);
    units(level, "east-mid-air", [38.0, 67.0], 22.0, 4, 4.0, false);
    level.building("east-corner", [40.0, 28.0], [16.0, 28.0], 42.0).look(Look::Glass);
    level.on("east-corner-crown", [40.0, 42.0, 28.0], [10.0, 3.0, 18.0], Look::Trim);
    bands(level, "east-corner", [40.0, 28.0], [16.0, 28.0], &[36.0]);
    // A building site with a tower crane, whose jib is over the kicker.
    level.building("east-site", [41.0, -17.0], [18.0, 30.0], 19.0);
    level.on("crane-mast", [41.0, 19.0, -27.0], [1.6, 39.0, 1.6], Look::Trim).scenery();
    level.on("crane-jib", [29.0, 56.0, -27.0], [42.0, 1.0, 1.2], Look::Trim).scenery();
    level.on("crane-cab", [41.0, 57.0, -27.0], [2.4, 2.4, 2.4], Look::Trim).scenery();
    level.on("crane-weight", [48.0, 54.0, -27.0], [3.0, 2.0, 2.0], Look::Trim).scenery();
    level.building("east-slate", [32.0, -53.0], [20.0, 22.0], 41.0).look(Look::Slate);
    level.on("east-slate-crown", [33.0, 41.0, -53.0], [12.0, 3.0, 14.0], Look::Trim);
    bands(level, "east-slate", [32.0, -53.0], [20.0, 22.0], &[34.0]);
    level.building("east-brick", [28.0, -82.0], [20.0, 24.0], 21.0).look(Look::Brick);
    stair_head(level, "east-brick-stairs", [24.0, -76.0], 21.0, 1.0);
    skylight(level, "east-brick-light", [31.0, -86.0], [8.0, 6.0], 21.0);
    level.building("east-annex", [29.0, -112.0], [22.0, 28.0], 22.0).look(Look::Brick);
    tank(level, "east-annex-tank", [24.0, -104.0], 22.0);
    units(level, "east-annex-air", [33.0, -108.0], 22.0, 3, 4.0, false);

    // Far towers that frame the view to the north and close the south.
    level.backdrop("far-north-west", [-62.0, -128.0], [22.0, 22.0], 82.0);
    level.backdrop("far-north-east", [60.0, -122.0], [20.0, 24.0], 74.0);
    level.backdrop("far-west", [-64.0, 30.0], [20.0, 26.0], 68.0);
    level.backdrop("far-east", [64.0, 96.0], [22.0, 22.0], 70.0);
}
