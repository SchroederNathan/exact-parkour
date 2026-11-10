//! Switchback Run: an old town block of brick and slate, packed around narrow
//! alleys and one courtyard. A clock tower stands in the courtyard and carries the
//! finish. The route runs east, west, east and west again, and each leg is one row
//! of roofs further north. It goes three quarters of the way round the tower, and
//! then winds up the tower's own steps to the finish.
//!
//! The block, from south to north. Each row is a line of houses from west to east:
//!
//! - Row A (z 18 to 30): low houses, 18 to 19.5 m. Leg 1 runs east on them.
//! - Row B (z 4 to 15): tall houses, 26.6 m, with one low yard roof between two
//!   of them, the notch. Leg 2 runs west on them.
//! - The courtyard band (z -15 to 1.5): the west house, the courtyard with the
//!   tower, the east house.
//! - Row C (z -30 to -18): terraces that step down from 28.4 to 22.7 m. Leg 3
//!   runs east on them. Leg 4 comes back west over the east house to the tower.
//!
//! The houses with pitched roofs are too tall to reach. They close the block and
//! say where the route is not.
//!
//! The beats, in the order the player meets them:
//!
//!  1. Start, house A1. A sprint jump over a 2.5 m alley between level roofs.
//!     Teaches the jump where the roofs are wide. The spire of the tower is ahead
//!     on the left for the whole leg.
//!  2. House A2. A fire wall crosses the roof, and its low orange part is the way
//!     over. Teaches the vault. A bad one costs nothing.
//!  3. A2 to A3. A 3 m alley to a roof 0.5 m higher. Tests the jump: it must be
//!     pressed near the edge.
//!  4. House A3. A doorway in a fire wall with a red bar. Teaches the slide. A
//!     runner on its feet stops at the bar and loses only time. Checkpoint 1.
//!  5. A3 to A4. A 2.5 m alley to a roof 1 m higher: the hands catch the edge.
//!     Teaches the catch, with the checkpoint a few metres behind.
//!  6. House A4, the first fold. The runner turns north, mantles the orange face
//!     of the dock and jumps from its top into the notch, 2.5 m away and 0.8 m
//!     lower, under the yellow beam it will cross later. Teaches the mantle, and
//!     joins a turn, a mantle and a jump. Checkpoint 2.
//!  7. The fire escape. Three teal faces of 2.2 m each in the corner of the
//!     notch: climb east, turn, climb north, turn, climb east onto house B5.
//!     Tests the climb three times, with a turn to find after each. Slow and
//!     exact after the speed of leg 1. Checkpoint 3.
//!  8. The beam. Leg 2 starts with a turn back west and 8 m of beam over the
//!     notch. The route crosses its own path, and the player looks down on where
//!     it was. Teaches the beam: this one is 1.4 m wide, and the later ones are
//!     narrower.
//!  9. House B3, the corridor between two attics: slide, vault, slide. Joins the
//!     moves of leg 1 into one rhythm. Fast, and no fall is possible: a rest.
//! 10. B3 to B2. A 3 m alley, then a wall ahead: the runner turns north straight
//!     after the landing. Checkpoint 4.
//! 11. The second fold, the hard beat. A 2.5 m jump at a teal edge on the tall
//!     west house. A wall stands on the edge, so the runner hangs, moves left by
//!     the hands to the corner where the roof is open, and climbs. A runner who
//!     aims the jump at the corner itself climbs at once. Checkpoint 5.
//! 12. The west house. Two planks of 1.2 m meet on a chimney stack in the middle
//!     of the light well, with a turn on the stack. Then a 3 m jump over the
//!     north alley onto row C, and a turn east. Tests the beam, and joins it to
//!     a jump. Checkpoint 6.
//! 13. Leg 3, the terraces. A 1.2 m step down. A vault straight into a jump that
//!     drops 2 m over a 3 m alley and ends in a roll. Checkpoint 7. A vault and
//!     at once a 2.5 m step down, with a second roll. A slide through a fire
//!     wall. The fastest part of the level, and all of it goes down.
//! 14. The shortcut. From the third terrace a 6 m jump south, over the alley and
//!     the yard of the east house, reaches the plant room 1 m lower: the hands
//!     catch its edge. It needs a sprint, and it cuts the third fold: 6 s. A
//!     runner who falls short falls to the street and starts again at
//!     checkpoint 7, which is beside the take-off.
//! 15. The third fold. A jump south over the north alley at the one place where
//!     the tall range of the east house leaves it open, and a turn west straight
//!     after the landing.
//! 16. Leg 4. A long stair up to the plant room. Checkpoint 8. A plank of 1 m on
//!     a slant over the courtyard to the balcony of the tower.
//! 17. The tower. Four steps wind round the shaft, anticlockwise: mantle, mantle,
//!     a teal face of 2.2 m to climb, mantle. The finish is on the terrace at
//!     the top, 6 m above the balcony, with the whole route below.
//!
//! The fall lines: every roof under a leg is under that leg's fall line, so a
//! runner who drops onto an earlier leg starts again at its checkpoint. No roof
//! that a runner can reach is without a way on.
use super::kit::{Level, Look};

// Heights of the roofs, metres.
const A1: f32 = 18.0;
const A3: f32 = 18.5;
const A4: f32 = 19.5;
const NOTCH: f32 = 20.0;
/// One flight of the fire escape.
const FLIGHT: f32 = 2.2;
/// Row B: three flights above the notch.
const B: f32 = NOTCH + 3.0 * FLIGHT;
/// The edge on the west house, and the roof of its wing.
const WING: f32 = B + 1.8;
const C2: f32 = WING - 1.2;
const C3: f32 = C2 - 2.0;
const C4: f32 = C3 - 2.5;
/// The east house is level with the last terrace.
const EAST: f32 = C4;
/// The plant room on the east house and the balcony of the tower.
const GALLERY: f32 = EAST + 1.5;

/// A wall to mantle: taller than a vault, and a Jump at it climbs onto it.
const MANTLE: f32 = 1.3;
/// A rail on an edge: too tall to vault.
const RAIL: f32 = 1.25;
/// The finish, on the terrace of the tower.
const FINISH: f32 = GALLERY + 3.0 * MANTLE + FLIGHT;

/// A fire wall is taller than a runner can catch.
const FIRE_WALL: f32 = 3.2;

// A low wall on the edge of a roof, off the route.
fn parapet(level: &mut Level, name: &str, x: [f32; 2], z: [f32; 2], base: f32) {
    level.on(
        name,
        [(x[0] + x[1]) / 2.0, base, (z[0] + z[1]) / 2.0],
        [x[1] - x[0], 0.6, z[1] - z[0]],
        Look::Trim,
    );
}

// A brick stack with a cap.
fn chimney(level: &mut Level, name: &str, at: [f32; 2], base: f32, tall: f32) {
    level
        .on(&format!("{name}-stack"), [at[0], base, at[1]], [0.9, tall, 0.9], Look::Brick);
    level
        .on(&format!("{name}-cap"), [at[0], base + tall, at[1]], [1.2, 0.2, 1.2], Look::Trim)
        .scenery();
}

// The head of a stair: a small house on a roof, with a cap.
fn stair_head(level: &mut Level, name: &str, at: [f32; 2], size: [f32; 2], base: f32) {
    level.on(
        &format!("{name}-walls"),
        [at[0], base, at[1]],
        [size[0], 2.5, size[1]],
        Look::Concrete,
    );
    level
        .on(
            &format!("{name}-cap"),
            [at[0], base + 2.5, at[1]],
            [size[0] + 0.4, 0.2, size[1] + 0.4],
            Look::Trim,
        )
        .scenery();
}

// A water tank on a frame.
fn tank(level: &mut Level, name: &str, at: [f32; 2], base: f32) {
    level.on(&format!("{name}-frame"), [at[0], base, at[1]], [1.8, 1.0, 1.8], Look::Trim);
    level
        .on(&format!("{name}-drum"), [at[0], base + 1.0, at[1]], [2.4, 2.0, 2.4], Look::Slate);
    level
        .on(&format!("{name}-lid"), [at[0], base + 3.0, at[1]], [2.7, 0.2, 2.7], Look::Trim)
        .scenery();
}

fn vent(level: &mut Level, name: &str, at: [f32; 2], base: f32) {
    level.on(name, [at[0], base, at[1]], [0.9, 0.8, 0.9], Look::Trim);
}

fn skylight(level: &mut Level, name: &str, at: [f32; 2], size: [f32; 2], base: f32) {
    level.on(name, [at[0], base, at[1]], [size[0], 0.5, size[1]], Look::Glass);
}

// A pitched roof of slate on a house too tall to reach: two slabs that meet at a
// ridge `rise` above the walls, and two boxes that fill the gables under them.
// The ridge runs along X when `east_west` is set, and along Z otherwise.
fn gable(level: &mut Level, name: &str, x: [f32; 2], z: [f32; 2], base: f32, rise: f32, east_west: bool) {
    let (mid_x, mid_z) = ((x[0] + x[1]) / 2.0, (z[0] + z[1]) / 2.0);
    let (wide, deep) = (x[1] - x[0], z[1] - z[0]);
    for (step, share) in [(0.0, 0.62), (1.0, 0.3)] {
        let size = if east_west {
            [wide, rise / 3.0, deep * share]
        } else {
            [wide * share, rise / 3.0, deep]
        };
        level
            .on(
                &format!("{name}-gable-{step}"),
                [mid_x, base + step * rise / 3.0, mid_z],
                size,
                Look::Brick,
            )
            .scenery();
    }
    let ridge = [mid_x, base + rise, mid_z];
    let (eaves, width) = if east_west {
        ([[mid_x, base, z[0]], [mid_x, base, z[1]]], wide + 0.6)
    } else {
        ([[x[0], base, mid_z], [x[1], base, mid_z]], deep + 0.6)
    };
    for (side, eave) in ["a", "b"].into_iter().zip(eaves) {
        level
            .ramp(&format!("{name}-roof-{side}"), eave, ridge, width, 0.3, Look::Slate)
            .scenery();
    }
}

// A fire wall across a roof at `x`, from `z[0]` to `z[1]`, with a doorway and a
// slide gate at `door`. The path goes through it along X.
fn fire_wall_with_gate(level: &mut Level, name: &str, x: f32, z: [f32; 2], door: f32, base: f32) {
    let (near, far) = (door - 1.4, door + 1.4);
    level.on(
        &format!("{name}-north"),
        [x, base, (z[0] + near) / 2.0],
        [0.5, FIRE_WALL, near - z[0]],
        Look::Trim,
    );
    level.on(
        &format!("{name}-south"),
        [x, base, (far + z[1]) / 2.0],
        [0.5, FIRE_WALL, z[1] - far],
        Look::Trim,
    );
    level.gate(&format!("{name}-gate"), [x, base, door], 2.0, 90.0);
}

// A fire wall across a roof at `x`, from `z[0]` to `z[1]`, whose part from
// `low[0]` to `low[1]` is low enough to vault.
fn fire_wall_with_vault(level: &mut Level, name: &str, x: f32, z: [f32; 2], low: [f32; 2], base: f32) {
    level.on(
        &format!("{name}-north"),
        [x, base, (z[0] + low[0]) / 2.0],
        [0.5, FIRE_WALL, low[0] - z[0]],
        Look::Trim,
    );
    level.on(
        &format!("{name}-south"),
        [x, base, (low[1] + z[1]) / 2.0],
        [0.5, FIRE_WALL, z[1] - low[1]],
        Look::Trim,
    );
    level.on(
        &format!("{name}-vault"),
        [x, base, (low[0] + low[1]) / 2.0],
        [0.5, 0.9, low[1] - low[0]],
        Look::Vault,
    );
}

pub fn level() -> Level {
    let mut level = Level::new(
        "Switchback Run",
        "Old town roofs, a fire escape and a clock tower",
        "#b4e83a",
    );
    row_a(&mut level);
    row_b(&mut level);
    west_house(&mut level);
    tower(&mut level);
    east_house(&mut level);
    row_c(&mut level);
    route(&mut level);
    level
}

// Row A, z 18 to 30: leg 1 runs east along z = 24.
fn row_a(level: &mut Level) {
    // A1, the start.
    level.building("a1", [-30.25, 24.0], [11.5, 12.0], A1).look(Look::Brick);
    parapet(level, "a1-west", [-36.0, -35.7], [18.0, 30.0], A1);
    parapet(level, "a1-south", [-35.7, -24.5], [29.7, 30.0], A1);
    parapet(level, "a1-north", [-35.7, -28.0], [18.0, 18.3], A1);
    stair_head(level, "a1-stair", [-33.6, 20.2], [2.6, 2.4], A1);
    chimney(level, "a1-chimney-1", [-27.0, 28.6], A1, 2.2);
    chimney(level, "a1-chimney-2", [-29.2, 28.6], A1, 1.7);

    // A2: the fire wall with a low part.
    level.building("a2", [-15.75, 23.0], [12.5, 10.0], A1).look(Look::Slate);
    fire_wall_with_vault(level, "a2-wall", -16.0, [18.0, 28.0], [20.8, 25.8], A1);
    parapet(level, "a2-south-west", [-22.0, -16.25], [27.7, 28.0], A1);
    parapet(level, "a2-south-east", [-15.75, -9.5], [27.7, 28.0], A1);
    parapet(level, "a2-north-west", [-21.0, -16.25], [18.0, 18.3], A1);
    parapet(level, "a2-north-east", [-15.75, -10.5], [18.0, 18.3], A1);
    vent(level, "a2-vent-1", [-12.6, 20.0], A1);
    vent(level, "a2-vent-2", [-11.3, 20.0], A1);
    skylight(level, "a2-skylight", [-19.4, 26.2], [2.6, 1.4], A1);

    // A3: the fire wall with a doorway.
    level.building("a3", [-0.25, 24.5], [12.5, 13.0], A3);
    fire_wall_with_gate(level, "a3-wall", -1.5, [18.0, 31.0], 24.8, A3);
    parapet(level, "a3-south-west", [-6.5, -1.75], [30.7, 31.0], A3);
    parapet(level, "a3-south-east", [-1.25, 6.0], [30.7, 31.0], A3);
    parapet(level, "a3-north-west", [-6.5, -1.75], [18.0, 18.3], A3);
    parapet(level, "a3-north-east", [-1.25, 6.0], [18.0, 18.3], A3);
    chimney(level, "a3-chimney-1", [-4.6, 20.0], A3, 2.0);
    chimney(level, "a3-chimney-2", [-4.6, 29.2], A3, 2.4);
    skylight(level, "a3-skylight", [3.4, 28.8], [2.4, 1.4], A3);

    // A4: the turn north. Its north edge is open where the notch is.
    level.building("a4", [15.25, 24.0], [13.5, 12.0], A4).look(Look::Brick);
    parapet(level, "a4-south", [8.5, 22.0], [29.7, 30.0], A4);
    parapet(level, "a4-east", [21.7, 22.0], [21.6, 29.7], A4);
    parapet(level, "a4-north", [8.5, 13.6], [18.0, 18.3], A4);
    // The dock: a wall to mantle, and the jump into the notch goes from its top.
    // It stands half a metre out over the alley.
    level.on("a4-dock", [18.0, A4, 19.45], [7.4, MANTLE, 3.9], Look::Concrete);
    level.on("a4-dock-face", [18.0, A4, 21.5], [7.4, MANTLE, 0.2], Look::Vault);
    stair_head(level, "a4-stair", [19.6, 26.9], [2.8, 2.6], A4);
    vent(level, "a4-vent", [11.2, 28.2], A4);
    chimney(level, "a4-chimney", [10.2, 19.6], A4, 2.2);

    // A5, the corner house: too tall to reach.
    level.building("a5", [30.25, 23.0], [11.5, 11.0], 26.5).look(Look::Slate);
    gable(level, "a5", [24.5, 36.0], [17.5, 28.5], 26.5, 3.0, true);
    chimney(level, "a5-chimney", [27.0, 20.4], 26.5, 3.4);
}

// Row B, z 4 to 15: leg 2 runs west along z = 12.3.
fn row_b(level: &mut Level) {
    // B5, the top of the fire escape. A strip of it closes the notch in the north.
    level.building("b5", [29.0, 9.5], [14.0, 11.0], B).look(Look::Brick);
    level.building("b5-strip", [18.0, 4.75], [8.0, 1.5], B).look(Look::Brick);
    parapet(level, "b5-east", [35.7, 36.0], [4.0, 15.0], B);
    parapet(level, "b5-south", [23.0, 35.7], [14.7, 15.0], B);
    parapet(level, "b5-north", [14.0, 35.7], [4.0, 4.3], B);
    stair_head(level, "b5-stair", [32.6, 6.6], [2.8, 2.6], B);
    tank(level, "b5-tank", [33.0, 12.4], B);
    chimney(level, "b5-chimney-1", [29.0, 13.6], B, 2.4);
    chimney(level, "b5-chimney-2", [27.2, 13.6], B, 1.8);

    // The notch: a low yard roof between B3 and B5.
    level.building("notch", [18.0, 10.25], [8.0, 9.5], NOTCH).look(Look::Slate);
    vent(level, "notch-vent-1", [14.8, 7.0], NOTCH);
    vent(level, "notch-vent-2", [14.8, 8.3], NOTCH);

    // The fire escape, in the north-east corner of the notch. Each flight is a
    // box with a teal face on the side the runner climbs.
    level.on("escape-1", [20.925, NOTCH, 9.8], [2.15, FLIGHT, 3.2], Look::Concrete);
    level.on("escape-1-face", [19.725, NOTCH, 9.8], [0.25, FLIGHT, 3.2], Look::Climb);
    level.on("escape-2", [20.8, NOTCH, 6.725], [2.4, 2.0 * FLIGHT, 2.45], Look::Concrete);
    level.on("escape-2-face", [20.8, NOTCH, 8.075], [2.4, 2.0 * FLIGHT, 0.25], Look::Climb);
    level.on(
        "escape-3-face",
        [21.875, NOTCH + 2.0 * FLIGHT, 6.725],
        [0.25, FLIGHT, 2.45],
        Look::Climb,
    );

    // The beam over the notch: the route crosses its own path here.
    level.deck("notch-beam", [18.0, B, 12.3], [8.0, 0.3, 1.4], Look::Beam);

    // B3: the corridor between two attics.
    level.building("b3", [2.5, 9.5], [23.0, 11.0], B);
    level.on("b3-attic-north", [3.5, B, 7.95], [14.0, 4.0, 5.9], Look::Slate);
    level.on("b3-attic-south", [3.5, B, 14.35], [14.0, 4.0, 1.3], Look::Slate);
    level.gate("b3-gate-1", [7.0, B, 12.3], 2.0, 90.0);
    level.on("b3-vault", [2.5, B, 12.3], [0.5, 0.9, 2.8], Look::Vault);
    level.gate("b3-gate-2", [-2.5, B, 12.3], 2.0, 90.0);
    parapet(level, "b3-north", [-9.0, 14.0], [4.0, 4.3], B);
    parapet(level, "b3-south-west", [-9.0, -3.5], [14.7, 15.0], B);
    parapet(level, "b3-south-east", [10.5, 14.0], [14.7, 15.0], B);
    vent(level, "b3-vent-1", [-7.6, 6.2], B);
    vent(level, "b3-vent-2", [12.4, 6.0], B);
    gable(level, "b3-attic", [-3.5, 10.5], [5.0, 10.9], B + 4.0, 1.8, true);
    chimney(level, "b3-chimney-1", [0.0, 6.4], B + 4.0, 2.6);
    chimney(level, "b3-chimney-2", [6.0, 6.4], B + 4.0, 2.6);

    // B2: a lane beside an upper storey. The storey's wall turns the runner north.
    level.building("b2", [-18.0, 9.5], [12.0, 11.0], B).look(Look::Slate);
    level.on("b2-storey", [-19.5, B, 9.5], [9.0, 4.8, 11.0], Look::Brick);
    gable(level, "b2-storey", [-24.0, -15.0], [4.0, 15.0], B + 4.8, 2.4, false);
    parapet(level, "b2-south", [-15.0, -12.0], [14.7, 15.0], B);
    chimney(level, "b2-chimney-1", [-22.4, 6.5], B + 4.8, 2.6);
    chimney(level, "b2-chimney-2", [-22.4, 12.5], B + 4.8, 2.6);

    // B1, the west end of the row: too tall to reach.
    level.building("b1", [-31.25, 9.5], [9.5, 11.0], 30.5).look(Look::Brick);
    gable(level, "b1", [-36.0, -26.5], [4.0, 15.0], 30.5, 2.6, true);
    chimney(level, "b1-chimney", [-29.0, 6.4], 30.5, 3.2);
}

// The west house, z -15 to 1.2: a tall body, and a lower wing with a light well.
fn west_house(level: &mut Level) {
    level.building("west-body", [-12.5, -6.9], [5.0, 16.2], 32.0);
    // The edge to hang from. Under the body's wall there is no room to stand; on
    // the wing, west of x = -15, there is.
    level.deck("west-edge", [-14.75, WING, 1.35], [6.5, 1.2, 0.3], Look::Climb);
    level.building("wing-south", [-21.5, -1.4], [13.0, 5.2], WING).look(Look::Brick);
    level.building("wing-north", [-21.5, -13.0], [13.0, 4.0], WING).look(Look::Brick);
    // Two planks over the light well, by way of a chimney stack in the middle of
    // it. They are a little proud of the roofs they lie on.
    level.building("well-stack", [-24.5, -7.5], [2.4, 2.4], WING).look(Look::Slate);
    level.ramp(
        "wing-plank-1",
        [-21.81, WING + 0.03, -3.77],
        [-24.09, WING + 0.03, -6.53],
        1.2,
        0.3,
        Look::Beam,
    );
    level.ramp(
        "wing-plank-2",
        [-24.09, WING + 0.03, -8.47],
        [-21.81, WING + 0.03, -11.23],
        1.2,
        0.3,
        Look::Beam,
    );
    parapet(level, "wing-south-west", [-28.0, -27.7], [-4.0, 1.2], WING);
    parapet(level, "wing-north-west", [-28.0, -27.7], [-15.0, -11.0], WING);
    stair_head(level, "wing-stair", [-25.8, -0.6], [2.6, 2.4], WING);
    chimney(level, "wing-chimney", [-16.4, -13.2], WING, 2.2);
    vent(level, "wing-vent", [-26.4, -13.4], WING);
    gable(level, "west-body", [-15.0, -10.0], [-15.0, 1.2], 32.0, 2.0, false);
    chimney(level, "west-body-chimney-1", [-11.2, -3.0], 32.0, 2.6);
    chimney(level, "west-body-chimney-2", [-11.2, -11.0], 32.0, 2.6);

    // The house at the west end of the band: too tall to reach.
    level.building("west-end", [-33.25, -6.75], [5.5, 16.5], 31.5).look(Look::Slate);
    gable(level, "west-end", [-36.0, -30.5], [-15.0, 1.5], 31.5, 2.2, false);
    chimney(level, "west-end-chimney", [-34.8, -2.0], 31.5, 2.8);
}

// The clock tower in the courtyard. Four steps wind round its shaft from the
// balcony where the plank lands, by the north and the west, to the terrace in
// the south that carries the finish.
fn tower(level: &mut Level) {
    let base = GALLERY;
    level.building("tower-base", [0.0, -5.0], [8.0, 8.0], base).look(Look::Brick);
    level.deck("tower-balcony", [5.5, base, -7.75], [3.0, 0.4, 2.5], Look::Concrete);
    level
        .on("tower-bracket", [5.0, base - 1.4, -7.75], [2.0, 1.0, 0.4], Look::Trim)
        .scenery();
    // Step 1, north: a wall to mantle.
    level.on("tower-step-1", [-0.1, base, -7.75], [2.8, MANTLE, 2.5], Look::Concrete);
    level.on("tower-step-1-face", [1.4, base, -7.75], [0.2, MANTLE, 2.5], Look::Vault);
    // Step 2, north-west: a second mantle.
    level.on("tower-step-2", [-2.85, base, -7.75], [2.3, 2.0 * MANTLE, 2.5], Look::Concrete);
    level.on("tower-step-2-face", [-1.6, base, -7.75], [0.2, 2.0 * MANTLE, 2.5], Look::Vault);
    // Step 3, west: a face to climb.
    let third = 2.0 * MANTLE + FLIGHT;
    level.on("tower-step-3", [-2.75, base, -4.9], [2.5, third, 2.8], Look::Concrete);
    level.on("tower-step-3-face", [-2.75, base, -6.4], [2.5, third, 0.2], Look::Climb);
    // The terrace, south and east of the shaft: the last mantle, and the finish.
    let terrace = third + MANTLE;
    level.on("tower-terrace-west", [-2.75, base, -2.15], [2.5, terrace, 2.3], Look::Concrete);
    level.on("tower-terrace-face", [-2.75, base, -3.4], [2.5, terrace, 0.2], Look::Vault);
    level.on("tower-terrace-south", [1.25, base, -2.25], [5.5, terrace, 2.5], Look::Concrete);
    level.on("tower-terrace-east", [2.75, base, -5.0], [2.5, terrace, 3.0], Look::Concrete);
    // The shaft, the clock and the spire.
    level.on("tower-shaft", [0.0, base, -5.0], [3.0, 12.4, 3.0], Look::Brick);
    let top = base + 12.4;
    level.on("tower-cornice", [0.0, top, -5.0], [4.0, 0.5, 4.0], Look::Trim).scenery();
    level.on("tower-spire-1", [0.0, top + 0.5, -5.0], [2.8, 2.4, 2.8], Look::Slate).scenery();
    level.on("tower-spire-2", [0.0, top + 2.9, -5.0], [1.9, 2.4, 1.9], Look::Slate).scenery();
    level.on("tower-spire-3", [0.0, top + 5.3, -5.0], [1.0, 2.8, 1.0], Look::Slate).scenery();
    level.on("tower-finial", [0.0, top + 8.1, -5.0], [0.3, 1.8, 0.3], Look::Trim).scenery();
    for (name, x, z, size) in [
        ("east", 1.56, -5.0, [0.12, 1.7, 1.7]),
        ("west", -1.56, -5.0, [0.12, 1.7, 1.7]),
        ("south", 0.0, -3.44, [1.7, 1.7, 0.12]),
        ("north", 0.0, -6.56, [1.7, 1.7, 0.12]),
    ] {
        level
            .on(&format!("tower-clock-{name}"), [x, base + 9.6, z], size, Look::Concrete)
            .scenery();
    }
    // Rails on the outer edges. A rail is too tall to vault and too thin to
    // stand on, so a Jump at it is only a jump.
    let rail = |level: &mut Level, name: &str, x: [f32; 2], z: [f32; 2], on: f32| {
        level.on(
            &format!("tower-rail-{name}"),
            [(x[0] + x[1]) / 2.0, base + on, (z[0] + z[1]) / 2.0],
            [x[1] - x[0], RAIL, z[1] - z[0]],
            Look::Trim,
        );
    };
    rail(level, "balcony-north", [4.0, 7.0], [-9.0, -8.85], 0.0);
    rail(level, "balcony-south", [4.0, 7.0], [-6.65, -6.5], 0.0);
    rail(level, "arrival-north", [1.5, 4.0], [-9.0, -8.85], 0.0);
    rail(level, "step-1-north", [-1.5, 1.5], [-9.0, -8.85], MANTLE);
    rail(level, "step-2-north", [-4.0, -1.5], [-9.0, -8.85], 2.0 * MANTLE);
    rail(level, "step-2-west", [-4.0, -3.85], [-8.85, -6.5], 2.0 * MANTLE);
    rail(level, "step-3-west", [-4.0, -3.85], [-6.5, -3.5], third);
    rail(level, "terrace-west", [-4.0, -3.85], [-3.5, -1.0], terrace);
    rail(level, "terrace-south", [-3.85, 4.0], [-1.15, -1.0], terrace);
    rail(level, "terrace-east", [3.85, 4.0], [-6.5, -1.15], terrace);
}

// The east house, z -15 to 1: leg 4 runs west on it, up the stair to the plant
// room and over the plank to the tower.
fn east_house(level: &mut Level) {
    // The low roof, and its corner in the north-east where the runner lands.
    level.building("east", [24.0, -5.45], [24.0, 12.9], EAST).look(Look::Brick);
    level.building("east-corner", [32.5, -13.45], [7.0, 3.1], EAST).look(Look::Brick);
    // The tall range on its north side: no way over from row C but at the corner.
    level.building("east-range", [23.75, -13.45], [10.5, 3.1], EAST + 5.7).look(Look::Slate);
    gable(level, "east-range", [18.5, 29.0], [-15.0, -11.9], EAST + 5.7, 1.4, true);
    chimney(level, "east-range-chimney-1", [21.0, -14.2], EAST + 5.7, 2.0);
    chimney(level, "east-range-chimney-2", [26.5, -14.2], EAST + 5.7, 2.0);
    // The plant room. Its north face stands over the yard, and the shortcut from
    // row C lands on its north edge.
    level.on("plant-room", [15.25, EAST, -10.0], [6.5, GALLERY - EAST, 3.8], Look::Concrete);
    // A long stair: a slope of more than about 14 degrees slows a sprint.
    level.stairs(
        "plant-stair",
        [27.0, EAST, -10.0],
        [18.5, GALLERY, -10.0],
        2.4,
        Look::Ramp,
    );
    // The plank to the tower, a little proud of the plant room and the gallery.
    level.ramp(
        "tower-plank",
        [12.27, GALLERY + 0.03, -10.12],
        [6.73, GALLERY + 0.03, -7.63],
        1.0,
        0.3,
        Look::Beam,
    );
    parapet(level, "east-east", [35.7, 36.0], [-15.0, 1.0], EAST);
    parapet(level, "east-north", [29.0, 30.8], [-15.0, -14.7], EAST);
    vent(level, "east-corner-vent", [29.9, -13.6], EAST);
    parapet(level, "east-south", [12.0, 35.7], [0.7, 1.0], EAST);
    parapet(level, "east-west", [12.0, 12.3], [-7.6, 0.7], EAST);
    skylight(level, "east-skylight-1", [32.0, -3.0], [2.6, 1.6], EAST);
    skylight(level, "east-skylight-2", [20.0, -3.4], [2.6, 1.6], EAST);
    stair_head(level, "east-stair", [15.0, -1.6], [2.8, 2.6], EAST);
    vent(level, "east-vent-1", [22.6, -4.6], EAST);
    vent(level, "east-vent-2", [23.9, -4.6], EAST);
    chimney(level, "east-chimney-1", [31.0, -0.6], EAST, 2.2);
    chimney(level, "east-chimney-2", [33.0, -0.6], EAST, 1.8);
}

// Row C, z -30 to -18: leg 3 runs east along z = -24, down the terraces.
fn row_c(level: &mut Level) {
    // C1: the landing from the west house, and the turn east.
    level.building("c1", [-25.5, -24.0], [21.0, 12.0], WING).look(Look::Slate);
    parapet(level, "c1-west", [-36.0, -35.7], [-30.0, -18.0], WING);
    parapet(level, "c1-north", [-35.7, -15.0], [-30.0, -29.7], WING);
    parapet(level, "c1-south", [-35.7, -29.0], [-18.3, -18.0], WING);
    stair_head(level, "c1-stair", [-24.4, -27.9], [3.0, 2.6], WING);
    tank(level, "c1-tank", [-32.6, -26.4], WING);
    chimney(level, "c1-chimney", [-31.0, -20.2], WING, 2.2);
    vent(level, "c1-vent", [-17.2, -28.4], WING);

    // C2: the fire wall with a low part, then the edge.
    level.building("c2", [-9.0, -24.0], [12.0, 12.0], C2).look(Look::Brick);
    fire_wall_with_vault(level, "c2-wall", -9.0, [-30.0, -18.0], [-26.5, -21.5], C2);
    parapet(level, "c2-north-west", [-15.0, -9.25], [-30.0, -29.7], C2);
    parapet(level, "c2-north-east", [-8.75, -3.0], [-30.0, -29.7], C2);
    chimney(level, "c2-chimney", [-13.2, -19.4], C2, 2.0);
    skylight(level, "c2-skylight", [-12.6, -27.6], [2.6, 1.6], C2);
    vent(level, "c2-vent", [-4.4, -28.4], C2);

    // C3: the landing of the long jump, the shortcut from its south edge, and a
    // fire wall near its east edge whose low part is the way down.
    level.building("c3", [8.5, -24.0], [17.0, 12.0], C3);
    fire_wall_with_vault(level, "c3-wall", 15.25, [-30.0, -18.0], [-26.5, -21.5], C3);
    parapet(level, "c3-north", [0.0, 15.0], [-30.0, -29.7], C3);
    parapet(level, "c3-south", [0.0, 11.0], [-18.3, -18.0], C3);
    skylight(level, "c3-skylight-1", [4.0, -28.0], [2.6, 1.6], C3);
    skylight(level, "c3-skylight-2", [8.4, -28.0], [2.6, 1.6], C3);
    chimney(level, "c3-chimney", [1.4, -19.6], C3, 2.2);
    vent(level, "c3-vent", [12.6, -28.6], C3);

    // C4: the last terrace, a doorway in a fire wall, and the turn south.
    level.building("c4", [26.5, -24.0], [19.0, 12.0], C4).look(Look::Slate);
    fire_wall_with_gate(level, "c4-wall", 29.0, [-30.0, -18.0], -24.0, C4);
    parapet(level, "c4-north-west", [17.0, 28.75], [-30.0, -29.7], C4);
    parapet(level, "c4-north-east", [29.25, 35.7], [-30.0, -29.7], C4);
    parapet(level, "c4-east", [35.7, 36.0], [-30.0, -18.0], C4);
    parapet(level, "c4-south", [17.25, 28.75], [-18.3, -18.0], C4);
    stair_head(level, "c4-stair", [33.4, -27.6], [2.8, 2.6], C4);
    chimney(level, "c4-chimney-1", [22.0, -19.6], C4, 2.2);
    chimney(level, "c4-chimney-2", [24.0, -19.6], C4, 1.7);
    vent(level, "c4-vent", [26.6, -28.4], C4);
}

// The checkpoints and their fall lines, and the finish.
fn route(level: &mut Level) {
    level.fall_below(A1 - 1.6);
    level.checkpoint([-33.0, A1, 24.0], [-20.0, 24.0]);
    level.fall_below(A3 - 1.6);
    level.checkpoint([1.8, A3, 24.6], [12.0, 24.0]);
    // The notch: house A4 is above this line, and a runner on it can jump back.
    level.fall_below(NOTCH - 1.6);
    level.checkpoint([16.2, NOTCH, 13.4], [19.0, 9.8]);
    // Leg 2: the notch, the east house and the gallery are under this line.
    level.fall_below(B - 1.6);
    level.checkpoint([26.2, B, 9.8], [22.0, 12.3]);
    level.checkpoint([-13.5, B, 10.4], [-13.5, 0.0]);
    // The west house and the first terrace: house B2 is under this line.
    level.fall_below(WING - 1.6);
    level.checkpoint([-18.0, WING, -1.2], [-22.0, -4.0]);
    // The terraces as far as the third: the gallery is under this line.
    level.fall_below(C3 - 0.3);
    level.checkpoint([-20.0, WING, -22.6], [-10.0, -24.0]);
    level.fall_below(EAST - 1.6);
    level.checkpoint([9.5, C3, -24.0], [20.0, -24.0]);
    level.checkpoint([16.0, GALLERY, -10.0], [8.0, -8.5]);
    level.finish_at([1.2, FINISH, -2.25]);
}
