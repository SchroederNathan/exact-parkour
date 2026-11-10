//! Helix Heights: one turn around a core that is still being built, from a depot
//! roof at 18 m to a deck on the core at 47.5 m, straight above the start.
//!
//! The core is a slate box of 70 m that nobody climbs. The route goes round it
//! against the clock: east along the south side, north up the east side, west
//! across the north side, south down the west side, and then back in along the jib
//! of a crane. Each side is higher than the one before it, and each has its own
//! verb: the vault, the climb, the sprint gap, the slide. The highest place is the
//! tip of the jib, 49.5 m.
//!
//! Slopes are one in four. The body keeps its speed on that slope and loses most
//! of it on a steeper one.
//!
//! The beats, in order. A checkpoint stands before each hard move.
//!
//!  1. Depot roof, 18 m. Run east. The core stands on the left, and the crane and
//!     the finish deck are high above. A pipe across the roof teaches the vault
//!     where a miss costs nothing. Wide and slow.
//!  2. The wheelbarrow ramp to the depot's upper roof, 20 m. The first metres of
//!     height, with no button. Checkpoint 1.
//!  3. The first gap: a sprint jump over a 3 m alley to the corner block, on the
//!     level. A jump from a run catches the far edge, so the first gap forgives.
//!  4. Corner block. Turn north. A pipe to vault, and 5 m after it a second 3 m
//!     gap: the vault is tested with a jump behind it. Checkpoint 2.
//!  5. East tower, a stack of terraces. A 1.4 m step to mantle, then a 2.5 m
//!     wall: jump, hang and climb. Slow and exact after the sprints.
//!     Checkpoint 3.
//!  6. The ledge. A plant room closes the top of the next wall, so the hands go
//!     west along the teal edge, out over the light well, to where the top is
//!     open. At most 3.5 m. A runner that lets go over the terrace lands on it.
//!     The one move of the level that is all exposure. Checkpoint 4.
//!  7. The first allowed exception: a sprint jump over a 3 m alley to a roof 2 m
//!     higher. The hands catch the teal edge. Hard, with a wide roof behind it.
//!  8. North-east block. A stair to its upper wing, 30.5 m, at full speed.
//!     Checkpoint 5.
//!  9. The frame of open floor slabs. A 3.5 m sprint gap onto the first slab and
//!     a girder of 1.2 m to the second, with the floor below 4.5 m down. Fast
//!     and narrow. Checkpoint 6.
//! 10. The long gap: 5 m at a sprint, which ends in a catch on the edge girder
//!     of the third slab. This is the second exception, and it is the fast line:
//!     a plank of 0.8 m beside it is the safe line, at a run.
//! 11. A ramp to the poured part of the next floor, 33 m. Checkpoint 7. A turn
//!     south onto a plank over the alley to the scaffold on the west tower. The
//!     view opens to the south: the tower with the crane is the next goal.
//! 12. Scaffold, 34.5 m. A slide gate with nothing behind it teaches the slide,
//!     and a stair goes up to 37.5 m. Checkpoint 8.
//! 13. A 2.5 m scaffold wall to climb, then a slide gate and, 4 m after it, a 3 m
//!     gap in the deck: the slide is joined to the sprint jump.
//! 14. The plank bridge: 12 m up over the street to the slab tower, 43 m.
//!     Checkpoint 9. A long roof to breathe on, with a pipe to vault and a screen
//!     with a bar to slide under: the two easy moves, joined at a sprint.
//! 15. The crane. Two walls of 2.5 m, one on the other: the ballast and the
//!     machine house. Checkpoint 10 at the foot of the jib, 48 m.
//! 16. The jib. A sprint of 19 m up a beam of 1.6 m to its tip. The depot roof
//!     where the run began is 31 m under the feet.
//! 17. The last jump: 5 m out and 2 m down onto the deck on the core, into a roll
//!     and through the finish. The core is the wall behind the roll.
use super::kit::{Level, Look};

// The colour of the level, for its checkpoint pads and its finish.
const THEME: &str = "#c77dff";

// Heights of the roofs on the route, metres, in the order the runner meets them.
const DEPOT: f32 = 18.0;
const DEPOT_UPPER: f32 = 20.0;
const CORNER: f32 = 20.0;
const TERRACE_A: f32 = 20.0;
const TERRACE_B: f32 = 21.4;
const TERRACE_C: f32 = 23.9;
const TERRACE_D: f32 = 26.4;
const NORTH_EAST: f32 = 28.4;
const SLABS: f32 = 30.5;
const SLAB_UPPER: f32 = 33.0;
const SCAFFOLD_1: f32 = 34.5;
const SCAFFOLD_2: f32 = 37.5;
const SCAFFOLD_3: f32 = 40.0;
const TOWER: f32 = 43.0;
const BALLAST: f32 = 45.5;
const CRANE: f32 = 48.0;
const JIB_TIP: f32 = 49.5;
const FINISH: f32 = 47.5;

// The middle of the scaffold lane on the west tower, on X.
const LANE: f32 = -41.5;

pub fn level() -> Level {
    let mut level = Level::new(
        "Helix Heights",
        "Climbs, girders, scaffolds and a crane jib",
        THEME,
    );
    core(&mut level);
    south(&mut level);
    east(&mut level);
    north(&mut level);
    west(&mut level);
    summit(&mut level);
    level
}

// A parapet on a roof edge that the route does not use: `along` is its length on
// X or on Z, by `on_x`.
fn parapet(level: &mut Level, name: &str, at: [f32; 3], along: f32, on_x: bool) {
    let size = if on_x { [along, 0.5, 0.3] } else { [0.3, 0.5, along] };
    level.on(name, at, size, Look::Trim);
}

// A thing that is drawn and not touched: scaffold poles, struts, rebar.
fn drawn(level: &mut Level, name: &str, at: [f32; 3], size: [f32; 3], look: Look) {
    level.on(name, at, size, look).scenery();
}

// The core: the tower in the middle, which the route goes round and never touches
// until the finish deck on its south face.
fn core(level: &mut Level) {
    level.building("core", [0.0, 0.0], [18.0, 18.0], 66.0).look(Look::Slate);
    level.on("core-top", [0.0, 66.0, 0.0], [14.0, 4.0, 14.0], Look::Slate);
    // The hoist that takes material up its east face.
    drawn(level, "core-hoist", [10.2, 0.0, -4.5], [2.2, 62.0, 2.2], Look::Trim);
    // The lines of its floors, and the rebar of the floor that is not poured.
    for (index, height) in [24.0, 36.0, 54.0, 62.0].into_iter().enumerate() {
        drawn(level, &format!("core-band-{index}"), [0.0, height, 0.0], [18.5, 0.5, 18.5], Look::Trim);
    }
    for (index, [x, z]) in [[-6.0, -6.0], [6.0, -6.0], [-6.0, 6.0], [6.0, 6.0], [0.0, 0.0]].into_iter().enumerate() {
        drawn(level, &format!("core-rebar-{index}"), [x, 70.0, z], [0.3, 3.0, 0.3], Look::Trim);
    }
    // Scaffold decks bolted to the faces the route does not use.
    for (index, [x, z, w, d, top]) in [
        [10.5, 0.0, 3.0, 12.0, 40.0],
        [0.0, -10.5, 12.0, 3.0, 46.0],
        [-10.5, 0.0, 3.0, 12.0, 56.0],
    ]
    .into_iter()
    .enumerate()
    {
        level.deck(&format!("core-deck-{index}"), [x, top, z], [w, 0.4, d], Look::Trim).scenery();
    }
}

// The south side: the depot, where the run begins, and the corner block.
fn south(level: &mut Level) {
    level.building("depot", [-5.0, 22.0], [34.0, 16.0], DEPOT).look(Look::Brick);
    level.on("depot-upper", [5.5, DEPOT, 22.0], [13.0, DEPOT_UPPER - DEPOT, 16.0], Look::Brick);
    level.on("depot-pipe", [-13.5, DEPOT, 22.0], [0.5, 0.9, 16.0], Look::Vault);
    level.ramp("depot-ramp", [-9.0, DEPOT, 22.0], [-1.0, DEPOT_UPPER, 22.0], 3.0, 0.3, Look::Ramp);
    // Roof furniture, clear of the line along Z = 22.
    parapet(level, "depot-parapet-west", [-21.85, DEPOT, 22.0], 16.0, false);
    parapet(level, "depot-parapet-north", [-11.4, DEPOT, 14.15], 20.6, true);
    parapet(level, "depot-parapet-south", [-11.4, DEPOT, 29.85], 20.6, true);
    parapet(level, "depot-upper-parapet-north", [5.5, DEPOT_UPPER, 14.15], 12.4, true);
    parapet(level, "depot-upper-parapet-south", [5.5, DEPOT_UPPER, 29.85], 12.4, true);
    level.on("depot-skylight-1", [-17.0, DEPOT, 16.6], [5.0, 0.6, 2.4], Look::Glass);
    level.on("depot-skylight-2", [-7.0, DEPOT, 16.6], [5.0, 0.6, 2.4], Look::Glass);
    level.on("depot-skylight-3", [-7.0, DEPOT, 27.4], [5.0, 0.6, 2.4], Look::Glass);
    level.on("depot-stair-head", [-18.5, DEPOT, 27.0], [4.0, 2.6, 3.0], Look::Brick);
    level.on("depot-vent-1", [6.0, DEPOT_UPPER, 16.5], [2.0, 1.3, 2.0], Look::Trim);
    level.on("depot-vent-2", [9.5, DEPOT_UPPER, 27.5], [2.5, 1.0, 1.6], Look::Trim);
    level.on("depot-tank", [5.5, DEPOT_UPPER, 27.3], [3.0, 2.2, 3.0], Look::Trim);

    level.fall_below(14.0);
    level.checkpoint([-19.5, DEPOT, 22.0], [-10.0, 22.0]);
    level.fall_below(16.0);
    level.checkpoint([2.5, DEPOT_UPPER, 22.0], [12.0, 22.0]);

    // The corner block, over a 3 m alley. The route comes in from the west and
    // leaves to the north.
    level.building("corner", [26.0, 22.0], [22.0, 20.0], CORNER);
    level.on("corner-pipe", [29.1, CORNER, 18.0], [15.2, 0.9, 0.5], Look::Vault);
    level.on("corner-stair-head", [19.5, CORNER, 15.5], [4.0, 2.6, 5.5], Look::Concrete);
    parapet(level, "corner-parapet-south", [26.0, CORNER, 31.85], 22.0, true);
    parapet(level, "corner-parapet-east", [36.85, CORNER, 22.0], 19.4, false);
    level.on("corner-vent-1", [20.0, CORNER, 28.5], [2.4, 1.2, 2.4], Look::Trim);
    level.on("corner-vent-2", [24.5, CORNER, 28.5], [2.4, 1.2, 2.4], Look::Trim);
    level.on("corner-tank", [33.5, CORNER, 28.0], [3.0, 2.4, 3.0], Look::Trim);
}

// The east side: a tower of terraces, each a climb above the last, and the block
// in the north-east corner.
fn east(level: &mut Level) {
    level.building("terrace-a", [30.0, 6.0], [18.0, 6.0], TERRACE_A).look(Look::Glass);
    level.building("terrace-b", [30.0, 0.0], [18.0, 6.0], TERRACE_B).look(Look::Glass);
    // The third terrace is narrower: a light well opens on its west side, with a
    // low roof at its foot that is under the fall line.
    level.building("terrace-c", [33.25, -6.5], [11.5, 7.0], TERRACE_C).look(Look::Glass);
    level.building("terrace-well", [24.25, -6.5], [6.5, 7.0], 12.0).look(Look::Glass);
    level.building("terrace-d", [30.0, -16.0], [18.0, 12.0], TERRACE_D).look(Look::Glass);
    // The edges to climb, each a teal step in front of its wall.
    level.deck("terrace-b-edge", [32.0, TERRACE_B, 3.2], [8.0, TERRACE_B - TERRACE_A, 0.4], Look::Climb);
    level.deck("terrace-c-edge", [33.0, TERRACE_C, -2.8], [8.0, TERRACE_C - TERRACE_B, 0.4], Look::Climb);
    // The ledge: it runs on west past the end of the terrace below, over the well.
    // It ends 2.7 m east of the well, so the longest way along it is 3.5 m.
    level.deck("terrace-d-edge", [26.6, TERRACE_D, -9.8], [7.2, TERRACE_D - TERRACE_C, 0.4], Look::Climb);
    // The plant room stands on the edge and closes the top above the terrace.
    level.on("terrace-plant", [33.0, TERRACE_D, -12.0], [12.0, 4.0, 4.0], Look::Slate);
    level.on("terrace-plant-vent", [33.0, TERRACE_D + 4.0, -12.0], [3.0, 0.8, 2.0], Look::Trim);
    parapet(level, "terrace-a-parapet", [38.85, TERRACE_A, 6.0], 6.0, false);
    parapet(level, "terrace-b-parapet", [38.85, TERRACE_B, 0.0], 6.0, false);
    parapet(level, "terrace-c-parapet", [38.85, TERRACE_C, -6.5], 7.0, false);
    parapet(level, "terrace-d-parapet", [38.85, TERRACE_D, -18.0], 8.0, false);
    level.on("terrace-a-vent", [37.0, TERRACE_A, 5.0], [1.6, 1.0, 1.6], Look::Trim);
    level.on("terrace-b-planter", [24.0, TERRACE_B, 0.0], [3.0, 0.7, 3.0], Look::Trim);
    level.on("terrace-d-skylight", [34.0, TERRACE_D, -18.5], [5.0, 0.6, 3.0], Look::Glass);
    level.on("terrace-d-mast", [37.5, TERRACE_D, -20.5], [0.4, 6.0, 0.4], Look::Trim);

    level.fall_below(TERRACE_A - 2.5);
    level.checkpoint([32.0, TERRACE_A, 7.6], [32.0, 0.0]);
    // The checkpoint before the ledge. The terraces below it are under its line.
    level.fall_below(TERRACE_C - 2.0);
    level.checkpoint([33.0, TERRACE_C, -5.0], [28.6, -9.0]);
    level.fall_below(TERRACE_D - 2.0);
    level.checkpoint([24.0, TERRACE_D, -13.5], [24.0, -25.0]);

    // The north-east block, over a 3 m alley and 2 m higher: the hands catch it.
    level.building("north-east", [23.5, -34.2], [31.0, 17.6], NORTH_EAST).look(Look::Brick);
    level.deck("north-east-edge", [24.0, NORTH_EAST, -25.2], [8.0, 2.5, 0.4], Look::Climb);
    level.on("north-east-wing", [10.8, NORTH_EAST, -34.0], [5.6, SLABS - NORTH_EAST, 12.0], Look::Brick);
    // A stair of one in four: the runner keeps its speed on it.
    level.stairs("north-east-stair", [22.0, NORTH_EAST, -34.0], [13.6, SLABS, -34.0], 3.0, Look::Ramp);
    parapet(level, "north-east-parapet-east", [38.85, NORTH_EAST, -34.2], 17.6, false);
    parapet(level, "north-east-parapet-north", [26.5, NORTH_EAST, -42.85], 24.4, true);
    level.on("north-east-stair-head", [34.0, NORTH_EAST, -39.0], [4.5, 2.6, 4.0], Look::Brick);
    level.on("north-east-tank", [34.5, NORTH_EAST, -29.5], [3.2, 2.6, 3.2], Look::Trim);
    level.on("north-east-vent", [27.0, NORTH_EAST, -40.0], [2.4, 1.1, 2.4], Look::Trim);

    level.fall_below(SLABS - 2.6);
    level.checkpoint([12.4, SLABS, -34.0], [8.0, -34.0]);
}

// The north side: the frame of a building with its floors half poured. The route
// crosses three slabs at 30.5 m, by a gap, a girder and a long gap, and goes up a
// ramp to the part of the next floor that is poured.
fn north(level: &mut Level) {
    level.deck("slab-1", [0.5, SLABS, -34.0], [8.0, 0.4, 10.0], Look::Concrete);
    level.deck("slab-girder", [-7.5, SLABS, -31.5], [8.0, 0.4, 1.2], Look::Beam);
    level.deck("slab-2", [-15.5, SLABS, -33.0], [8.0, 0.4, 10.0], Look::Concrete);
    level.deck("slab-3", [-35.7, SLABS, -33.0], [21.6, 0.4, 12.0], Look::Concrete);
    // The edge girder of the third slab, which the hands catch after the long gap.
    level.deck("slab-3-edge", [-24.7, SLABS, -33.0], [0.4, 1.2, 12.0], Look::Climb);
    // The safe way beside the long gap: a plank to walk, slower than the jump.
    level.deck("slab-plank", [-22.0, SLABS, -36.4], [5.0, 0.25, 0.8], Look::Beam);
    level.on("slab-upper", [-42.75, SLABS, -33.0], [7.5, SLAB_UPPER - SLABS, 12.0], Look::Concrete);
    level.ramp("slab-ramp", [-29.0, SLABS, -33.0], [-39.0, SLAB_UPPER, -33.0], 2.4, 0.25, Look::Ramp);
    // The floors below, which are under the fall line, and the columns.
    level.deck("slab-low-1", [-21.0, 26.0, -33.5], [51.0, 0.4, 13.0], Look::Concrete);
    level.deck("slab-low-2", [-21.0, 22.0, -33.5], [51.0, 0.4, 13.0], Look::Concrete);
    let columns = [
        (4.0, 31.2),
        (-3.0, 31.2),
        (-12.0, 31.2),
        (-19.0, 31.2),
        (-25.6, 33.5),
        (-36.0, 33.5),
        (-45.9, 35.0),
    ];
    for (index, (x, top)) in columns.into_iter().enumerate() {
        for (row, z) in [(0, -38.4), (1, -28.6)] {
            level.building(&format!("column-{index}-{row}"), [x, z], [0.7, 0.7], top);
            drawn(level, &format!("rebar-{index}-{row}"), [x, top, z], [0.15, 1.0, 0.15], Look::Trim);
        }
    }
    // What a site leaves on a floor, off the line.
    level.on("slab-1-pallet", [2.5, SLABS, -37.0], [1.6, 1.0, 1.6], Look::Trim);
    level.on("slab-2-pallet", [-17.0, SLABS, -29.6], [2.0, 0.8, 1.4], Look::Trim);
    level.on("slab-3-cabin", [-32.0, SLABS, -37.3], [6.0, 2.6, 2.6], Look::Trim);
    level.on("slab-3-stack", [-28.0, SLABS, -28.6], [2.4, 1.2, 1.4], Look::Trim);

    // The checkpoint before the long gap, with the second slab as its run-up.
    level.fall_below(SLABS - 1.6);
    level.checkpoint([-13.0, SLABS, -32.6], [-24.0, -33.0]);
    // The checkpoint after it, before the plank. The slabs are under its line.
    level.fall_below(SLAB_UPPER - 2.0);
    level.checkpoint([-42.0, SLAB_UPPER, -32.0], [LANE, -24.0]);
}

// The west side: the scaffold on the outer face of the west tower. It goes up to
// the south in four decks, with a slide gate on the first and on the third.
fn west(level: &mut Level) {
    level.building("west-tower", [-33.0, 1.0], [14.0, 52.0], 47.0);
    level.on("west-tower-plant", [-33.0, 47.0, -8.0], [8.0, 3.0, 10.0], Look::Trim);
    level.on("west-tower-tank", [-33.0, 47.0, 14.0], [4.0, 3.0, 4.0], Look::Trim);
    // The plank from the upper slab over the alley.
    level.ramp("scaffold-plank", [LANE, SLAB_UPPER, -27.0], [LANE, SCAFFOLD_1, -21.0], 1.6, 0.25, Look::Ramp);
    level.deck("scaffold-1", [LANE, SCAFFOLD_1, -16.0], [3.0, 0.3, 10.0], Look::Beam);
    level.gate("scaffold-gate-1", [LANE, SCAFFOLD_1, -16.5], 2.2, 0.0);
    level.stairs("scaffold-stair", [LANE, SCAFFOLD_1, -11.0], [LANE, SCAFFOLD_2, 1.0], 3.0, Look::Ramp);
    level.deck("scaffold-2", [LANE, SCAFFOLD_2, 4.2], [3.0, 0.3, 6.4], Look::Beam);
    // The scaffold wall, a climb of 2.5 m, and the third deck with its gate.
    level.deck("scaffold-wall", [LANE, SCAFFOLD_3, 7.2], [3.0, SCAFFOLD_3 - SCAFFOLD_2 + 0.3, 0.4], Look::Climb);
    level.deck("scaffold-3", [LANE, SCAFFOLD_3, 12.7], [3.0, 0.3, 10.6], Look::Beam);
    level.gate("scaffold-gate-2", [LANE, SCAFFOLD_3, 12.5], 2.2, 0.0);
    // A bay with no deck, 4 m after the gate, and the last deck.
    level.deck("scaffold-4", [LANE, SCAFFOLD_3, 22.5], [3.0, 0.3, 3.0], Look::Beam);
    // The plank bridge: 12 m up over the street to the slab tower.
    level.ramp("scaffold-bridge", [LANE, SCAFFOLD_3, 24.0], [LANE, TOWER, 36.0], 1.6, 0.25, Look::Ramp);
    // Poles and ties under the decks.
    for (index, (z, top)) in [
        (-20.5, SCAFFOLD_1),
        (-11.5, SCAFFOLD_1),
        (-5.0, SCAFFOLD_1 + 1.5),
        (1.5, SCAFFOLD_2),
        (7.2, SCAFFOLD_2),
        (12.5, SCAFFOLD_3),
        (17.5, SCAFFOLD_3),
        (21.5, SCAFFOLD_3),
        (23.5, SCAFFOLD_3),
    ]
    .into_iter()
    .enumerate()
    {
        drawn(level, &format!("scaffold-pole-{index}"), [LANE - 1.4, top - 12.3, z], [0.14, 12.0, 0.14], Look::Trim);
        drawn(level, &format!("scaffold-tie-{index}"), [LANE, top - 0.5, z], [3.0, 0.14, 0.14], Look::Trim);
    }
    // The bays of the scaffold above the route, and the lines of the floors on
    // the wall beside it.
    for (index, (z, depth, top)) in [(-13.0, 14.0, 39.0), (3.0, 12.0, 44.0), (17.0, 14.0, 45.5)].into_iter().enumerate() {
        level.deck(&format!("scaffold-bay-{index}"), [LANE + 0.9, top, z], [1.2, 0.15, depth], Look::Trim).scenery();
    }
    for (index, height) in [36.0, 39.0, 42.0, 45.0].into_iter().enumerate() {
        drawn(level, &format!("west-tower-floor-{index}"), [-40.05, height, 1.0], [0.1, 0.3, 52.0], Look::Trim);
    }

    level.fall_below(SCAFFOLD_2 - 2.0);
    level.checkpoint([LANE, SCAFFOLD_2, 3.0], [LANE, 7.0]);
}

// The summit: the slab tower on the south side, the crane on its roof, the jib
// over the depot, and the deck on the core.
fn summit(level: &mut Level) {
    level.building("tower", [-19.0, 44.0], [62.0, 16.0], TOWER).look(Look::Glass);
    level.on("tower-pipe", [-26.0, TOWER, 44.0], [0.5, 0.9, 15.4], Look::Vault);
    parapet(level, "tower-parapet-south", [-19.0, TOWER, 51.85], 62.0, true);
    parapet(level, "tower-parapet-west", [-49.85, TOWER, 44.0], 15.4, false);
    parapet(level, "tower-parapet-east", [11.85, TOWER, 44.0], 15.4, false);
    parapet(level, "tower-parapet-north", [-18.0, TOWER, 36.15], 40.0, true);
    level.on("tower-stair-head", [-46.5, TOWER, 48.0], [5.0, 2.8, 5.0], Look::Concrete);
    level.on("tower-tank-1", [-35.0, TOWER, 49.0], [3.4, 2.8, 3.4], Look::Trim);
    level.on("tower-tank-2", [-33.0, TOWER, 38.6], [3.0, 2.2, 3.0], Look::Trim);
    // A screen across the roof with one way through: a bar to slide under, 7 m
    // after the pipe.
    level.gate("tower-gate", [-18.0, TOWER, 44.0], 2.2, 90.0);
    level.on("tower-screen-north", [-18.0, TOWER, 39.4], [0.4, 3.2, 6.2], Look::Trim);
    level.on("tower-screen-south", [-18.0, TOWER, 48.6], [0.4, 3.2, 6.2], Look::Trim);
    level.on("tower-vent-1", [-13.5, TOWER, 49.0], [2.6, 1.2, 2.6], Look::Trim);
    level.on("tower-vent-2", [-13.5, TOWER, 39.0], [2.6, 1.2, 2.6], Look::Trim);
    level.on("tower-mast", [9.5, TOWER, 49.5], [0.4, 7.0, 0.4], Look::Trim);
    level.on("tower-skylight", [8.0, TOWER, 40.0], [4.0, 0.6, 5.0], Look::Glass);

    level.fall_below(TOWER - 2.0);
    level.checkpoint([LANE, TOWER, 39.0], [-25.0, 44.0]);

    // The crane: the ballast, the machine house, the deck, the cab and the jib.
    level.on("crane-ballast", [-7.5, TOWER, 44.0], [3.0, BALLAST - TOWER, 4.0], Look::Concrete);
    level.on("crane-ballast-edge", [-9.2, TOWER, 44.0], [0.4, BALLAST - TOWER, 4.0], Look::Climb);
    level.on("crane-house", [-3.8, TOWER, 44.0], [3.6, CRANE - TOWER - 0.5, 6.0], Look::Trim);
    level.on("crane-house-edge", [-5.8, BALLAST, 44.0], [0.4, CRANE - BALLAST, 4.0], Look::Climb);
    level.on("crane-mast", [1.0, TOWER, 44.0], [2.6, CRANE - TOWER - 0.5, 2.6], Look::Trim);
    level.deck("crane-deck", [-0.8, CRANE, 44.0], [9.6, 0.5, 6.0], Look::Concrete);
    level.on("crane-cab", [3.0, CRANE, 45.5], [2.0, 2.2, 3.0], Look::Glass);
    // The jib goes up to its tip, which is the highest place of the route.
    level.ramp("crane-jib", [0.0, CRANE, 41.0], [0.0, JIB_TIP, 22.0], 1.6, 0.3, Look::Beam);
    level
        .ramp("crane-jib-truss", [0.0, CRANE - 0.3, 41.0], [0.0, JIB_TIP - 0.3, 22.0], 0.9, 0.8, Look::Trim)
        .scenery();
    level.deck("crane-counter", [0.0, CRANE, 50.5], [2.0, 0.6, 7.0], Look::Trim);
    level.on("crane-weight", [0.0, CRANE, 52.0], [2.6, 1.8, 3.0], Look::Concrete);
    // The head of the crane and the ties to the counter-jib.
    level.on("crane-head", [0.0, CRANE, 46.3], [1.0, 10.0, 1.0], Look::Trim);
    for (index, x) in [0.7, -0.7].into_iter().enumerate() {
        level
            .ramp(&format!("crane-tie-{index}"), [0.0, CRANE + 9.8, 46.6], [x, CRANE + 1.8, 53.0], 0.12, 0.12, Look::Trim)
            .scenery();
    }
    // The load hangs low over the depot, beside the first run: its cable leads
    // the eye from the start up to the jib and the deck.
    drawn(level, "crane-cable", [0.0, 26.0, 29.0], [0.15, CRANE + 0.9 - 26.0, 0.15], Look::Trim);
    drawn(level, "crane-hook", [0.0, 25.3, 29.0], [0.7, 0.7, 0.7], Look::Trim);
    drawn(level, "crane-load", [0.0, 24.1, 29.0], [2.2, 1.2, 1.6], Look::Concrete);

    level.fall_below(CRANE - 2.0);
    level.checkpoint([-1.0, CRANE, 44.0], [0.0, 30.0]);

    // The deck on the south face of the core, 5 m past the tip of the jib and 2 m
    // under it. The core is the wall behind the roll.
    level.deck("finish-deck", [0.0, FINISH, 13.0], [12.0, 0.4, 8.0], Look::Concrete);
    for (index, x) in [-5.0, 0.0, 5.0].into_iter().enumerate() {
        level
            .ramp(&format!("finish-strut-{index}"), [x, FINISH - 5.0, 9.1], [x, FINISH - 0.4, 16.0], 0.3, 0.3, Look::Trim)
            .scenery();
    }
    level.finish_at([0.0, FINISH, 12.2]);
}
