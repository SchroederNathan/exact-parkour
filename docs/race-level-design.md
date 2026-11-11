# Race level design

A race is a run across the roofs of a city block: from the start, through the checkpoints, to the finish gate. The street is far below. A runner who falls off the roofs starts again at the last checkpoint, and the clock keeps running.

Each level is one file in `logic/src/race/` with one function, `pub fn level() -> Level`. The kit in `logic/src/race/kit.rs` builds it from boxes. A level has no textures: its colours are its language.

## The world

- Lengths are metres. Y is up. North is -Z, east is +X. A yaw is degrees about Y.
- The street is the plane Y = 0. A building is a box from the street to its roof.
- The runner is 1.8 m tall and 0.6 m wide. It runs at 3.4 m/s and sprints at 6.2 m/s.
- The far city around a level is made automatically from the level's bounds. `backdrop` adds a landmark by hand.

## The kit

| Call | What it makes |
|---|---|
| `level.building(name, [x, z], [width, depth], roof)` | A building from the street to a flat roof at height `roof` |
| `level.on(name, [x, base, z], [width, height, depth], look)` | A box that stands on the height `base`: a wall, a vent, a step, an upper storey |
| `level.deck(name, [x, top, z], [width, thick, depth], look)` | A box whose top is at the height `top`: a plank, a bridge, a balcony |
| `level.ramp(name, from, to, width, thick, look)` | A slab from point to point. Level points make a plank at any angle; other points make a ramp |
| `level.stairs(name, from, to, width, look)` | Drawn steps over an unseen ramp, so no step stops the runner. Its slope slows a sprint as a ramp's does |
| `level.gate(name, [x, base, z], span, yaw)` | A slide gate: a bar between two posts. A slide goes under it; a run does not pass |
| `level.backdrop(name, [x, z], [width, depth], roof)` | A far building that nothing touches |
| `level.fall_below(y)` | The fall line for the checkpoints added after this call |
| `level.checkpoint([x, y, z], [toward x, toward z])` | A checkpoint. The first one is the start |
| `level.finish_at([x, y, z])` | The finish gate |

`building`, `on`, `deck`, `ramp` and `backdrop` return the piece, so `.look(Look::Brick)` and `.turned(20.0)` can follow. A level file is Rust: a loop or a helper function is fine. Piece names must be unique in a level.

## The colours

| Look | Meaning |
|---|---|
| `Concrete`, `Slate`, `Brick`, `Glass` | Building bodies. Vary them so that each roof is easy to name |
| `Trim` | Roof furniture off the route: parapets, vents, tanks |
| `Vault` (orange) | A wall to vault or mantle |
| `Climb` (teal) | A wall or an edge to grab and climb |
| `Beam` (yellow) | A plank, a beam or a catwalk to run along |
| `Ramp` (sand) | A ramp or a stair |
| `Danger` (red) | A slide bar. `gate` sets it |
| `Accent` | The level's own colour: checkpoint pads and the finish. Use it for nothing else |

Use one colour for one meaning. A player must be able to read the next move from the colour of the piece in front.

## What the runner can do

Measured by the ignored `envelope` test in `logic/tests/envelope.rs`. A gap is the clear distance between two roof edges. Jump is pressed 0.35 m before the edge.

| Height of the far roof | Run: lands on its feet | Run: catches the edge | Sprint: lands on its feet | Sprint: catches the edge |
|---|---|---|---|---|
| 3 m lower | up to 4.0 m (rolls) | 4.5 m | 2.0 to 7.5 m (rolls) | 8.0 to 8.5 m |
| 2 m lower | up to 3.5 m (rolls) | 4.0 to 4.5 m | up to 6.5 m (rolls) | 7.0 to 8.0 m |
| 1 m lower | up to 3.0 m | 3.5 to 4.0 m | up to 5.5 m | 6.0 to 7.0 m |
| level | up to 2.0 m | 2.5 to 3.5 m | up to 4.5 m | 5.0 to 6.0 m |
| 0.5 m higher | 1.5 m | up to 3.5 m | 1.5 to 3.5 m | up to 6.0 m |
| 1.0 m higher | never | up to 3.0 m | never | up to 5.5 m |
| 1.5 m higher | never | up to 2.5 m | never | up to 5.0 m |
| 2.0 m higher | never | 1.0 to 2.0 m | never | 2.0 to 4.0 m |
| 2.5 m higher or more | never | never | never | never |

A runner that catches an edge climbs onto the roof in about 1.1 s. A catch is slower than a landing, and safer.

Other moves, from the README:

- A wall on the runner's own surface, 0.35 to 1.15 m high and up to 1.2 m deep: Jump vaults it.
- A wall up to 1.5 m high that is deeper: Jump climbs onto it.
- A wall up to about 2.8 m high: Jump at it, the hands catch its edge, and the runner climbs. A wall of 3 m or more stops the runner.
- A runner that hangs can move along the edge at 0.8 m/s, and can jump along it or off it. Where the top has no room to stand, a push at the wall does nothing: the runner stays on the edge and must move along it.
- The table is for a jump from one roof to another. A wall of 2.2 to 2.8 m on the runner's own surface, or across a gap of up to about 2.5 m, is caught by the hands.
- A checkpoint or the finish counts when the feet are within 2.2 m of it on the map and within 1.6 m of its height. Keep a later checkpoint further than that from an earlier part of the route.
- A drop of up to 0.9 m needs no recovery. Up to 2 m the runner kneels and runs on. Up to about 3.5 m it rolls. A drop of 4 m or more throws the runner down, and it needs 3.5 s to get up.
- A slide starts at 2.5 m/s or more. It covers about 2.5 m from a run and 4.6 m from a sprint. Under a bar the slide goes on until the runner is clear.
- A slope up to 50 degrees is walked. A step up to 0.15 m is walked.
- A slope slows a sprint. Up a ramp or a stair of 12 degrees the runner sprints at 5.9 m/s, at 14 degrees 5.3 m/s, at 17 degrees 3.8 m/s and at 21 degrees 2.5 m/s. A stair that must keep the speed has a run of 4.5 times its rise.
- A sprint landing on a roof 1.5 m lower can roll. The roll takes 0.7 s and leaves the runner at 3.4 m/s. Steps of 1 m in a run of jumps keep the speed.

## Slides, ramps and speed

A slide down a ramp gives the runner more speed than its legs do. Measured by the ignored `slide_envelope` and `valley_envelope` tests in `logic/tests/envelope.rs`. The runner sprints onto the ramp and presses Slide 1 m down it.

Speed at the foot of the ramp, m/s. The fastest slide is 10 m/s:

| Slope | Drop 2 m | Drop 4 m | Drop 6 m | Drop 8 m | Drop 12 m |
|---|---|---|---|---|---|
| 6 degrees | 5.7 | 5.4 | 5.2 | slower | slower |
| 10 degrees | 6.8 | 7.2 | 7.5 | 7.8 | 8.0 |
| 15 degrees | 7.3 | 8.1 | 8.8 | 9.3 | 10.0 |
| 20 to 30 degrees | 7.4 | 8.5 | 9.4 | 10.0 | 10.0 |

A sprint down the same ramps with no slide stays at 6.2 m/s.

- **On level ground the speed goes.** After a foot speed of 10 m/s the runner has 9.5 m/s after 3 m of level roof, 8.9 after 6 m, 8.2 after 10 m and 7.3 after 15 m.
- **A jump keeps the speed, and so does the air.** Jump cuts the slide and the runner leaves the lip at its slide speed.
- **A fast jump is a long jump.** Gap crossed with a landing on the feet, Jump pressed 0.35 m before the lip:

| Speed at the lip | Level roofs | 1 m lower | 2 m lower (rolls) | 3 m lower (rolls) |
|---|---|---|---|---|
| 6.2 m/s (a sprint) | 4.5 m | 5.5 m | 6.5 m | 7.5 m |
| 6.8 m/s | 5 m | 6 m | 7 m | 8 m |
| 7.9 m/s | 5 m | 7 m | 8 m | 9 m |
| 9.5 m/s | 7 m | 9 m | 10 m | 11 m |

  One or two metres more than these ends in a catch of the far edge. A roof 1 m higher is always a catch, up to 6 m at 7.9 m/s and 8 m at 9.5 m/s.
- **A soft landing keeps the speed.** After a jump at 9.5 m/s onto a roof that is level or up to 1 m lower, the runner still has 9 m/s. A landing 1.5 m lower or more is a roll, and a roll leaves the runner at sprint speed. So a chain of slides and jumps that keeps its speed steps down 1 m or less at each jump, and loses its height on the ramps.
- **A slide ends when the body leaves the ground.** After a landing the player presses Slide again. Slide needs 2.5 m/s and feet on the ground.
- **A slide ends at something it hits.** A wall across a chute stops it.
- **The stick leans and turns a slide** at up to 2.2 rad/s. At 10 m/s that is a turn of 4.5 m radius at the very best. Build a bend in a chute with a radius of 12 m or more, from segments that each turn 15 degrees or less.
- **Slide speed does not help uphill.** A ramp up after a slide is as slow as it is from a sprint: 10 degrees up 4 m takes 3.9 s, 20 degrees up 4 m takes 4.2 s. The fast way up is a wall: a climb of 2.5 m takes about 1.1 s. Use ramps to go down and walls to go up.
- **A slide passes under a bar.** A bar across a chute is a thing to slide under at speed, and it stops a runner who came down the chute on its feet.

## Rules

1. Design inside the table, and leave a margin: a required sprint jump is at most 4.5 m between level roofs. Use a gap that needs a catch only as an optional fast line, or as the one hard move of a section.
2. No required drop of more than 3 m.
3. No piece on the route with a height between 0.16 and 0.34 m. It stops the runner and gives no move.
4. No parapet or kerb on an edge the route jumps from or lands on. A runner vaults a parapet at the edge and falls into the gap.
5. A beam on the route is at least 1.0 m wide for a sprint and 0.8 m wide for a run.
6. A slide gate needs 4 m of straight path before it and 3 m clear after it. Close the path beside its posts with a wall or a roof edge.
7. An edge to hang from is the top of a wall at least 1 m tall. Keep a move along an edge under 4 m.
8. A checkpoint stands on a surface, at least 1 m from any wall, with 4 m of run-up toward the next obstacle. The time between two checkpoints is at most 15 s.
9. A checkpoint counts when the runner reaches it, also when an earlier one was missed. The finish needs no checkpoint. So a fast line can pass a checkpoint, and the fall line of the checkpoint before it must be under every roof of that fast line.
10. Set `fall_below` a little under the lowest surface of the route in that part. A lower roof that is off the route must be under the fall line, so no runner is left on a roof with no way on.
11. Keep 3 m clear above the route. The camera is behind the runner's right shoulder.
12. A level has at most 900 pieces. One stair is about 7 pieces for each metre of rise.
13. A beam that the scripted runner must cross a quarter of a metre off its middle is at least 1.4 m wide. A 1.0 m beam passed that check only by a hair, and failed it after the slide changed.

## What makes a level good

- **A route the player can read.** From each checkpoint the player sees the next one, or a landmark on the way to it. The beacon of light stands over the next checkpoint. The finish is visible from the start when the layout allows it.
- **Teach, test, twist.** A move appears first where a mistake costs nothing. Then it appears over a gap. Then it is joined to another move.
- **Rhythm.** Fast parts and exact parts take turns. After a hard part there is a wide roof and a checkpoint.
- **A city, not a set of boxes in the sky.** Buildings have different footprints, heights and looks. Roofs carry what real roofs carry: stair heads, vents, tanks, skylights, pipes, signs. An alley is 2 to 5 m wide. A street is wider than a jump, and something crosses it: a plank, a crane, a pipe bridge.
- **A second line.** Where there is room, a harder and faster way stands beside the safe way.
- **No blind jump.** The player sees where it lands before it leaves the ground.

## How to build and check a level

Run `bun exact.mjs test-rust` once in a new checkout. It writes `.shells/`, which `cargo` needs. Put cargo on the path with `export PATH="$HOME/.cargo/bin:$PATH"`.

**Draw the level.** The ignored `dump` test writes a level as JSON, and `tools/map.py` draws a plan and a view from it:

```sh
LEVEL=0 OUT=/tmp/level.json cargo test --manifest-path .shells/Cargo.toml -p exact-beat-em-up-logic --test race dump -- --ignored
python3 tools/map.py /tmp/level.json /tmp/level
```

`LEVEL` is 0 for Skyline Sprint, 1 for Switchback Run, 2 for Helix Heights and 3 for Gravity Cascade. The comment at the top of `tools/map.py` has the other arguments: a trail, the angle of the view, and a crop.

**Prove the level.** Each level has a test, `logic/tests/run_<level>.rs`, with a route for the runner in `logic/tests/bot/mod.rs`. The runner pushes the stick toward points on the map and presses Jump and Slide, as a player does. The test passes when the runner reaches the finish with no fall:

```sh
cargo test --manifest-path .shells/Cargo.toml -p exact-beat-em-up-logic --test run_skyline -- --nocapture
```

`TRACE=1` prints where the runner is ten times each second. `TRAIL=/tmp/trail.txt` writes its path, and `tools/map.py` draws the path on the plan.

A route is a list of steps:

| Step | What the runner does |
|---|---|
| `Run(x, z)` | Runs toward the point. Done when it stands at the point or past it. It climbs an edge it catches on the way |
| `Sprint(x, z)` | The same, with Sprint held |
| `Glide(x, z)` | The same as `Sprint`, and it is also done while the runner slides. Use it down a ramp and to a lip that the runner leaves in a slide |
| `Jump` | Presses Jump. Put it after a `Run` or `Sprint` to a point 0.3 to 0.5 m before an edge, or 0.6 to 0.9 m before a wall |
| `Slide` | Presses Slide. Put it after a `Sprint` to a point 2 m before a gate |
| `Hang` | Waits until the hands hold an edge |
| `Shimmy(x, z)` | On an edge, moves the hands toward the point |
| `Hold(seconds)`, `Wait(seconds)` | Keeps the stick, or lets it go, for a time |

`logic/tests/envelope.rs` has a short course with every kind of piece and the route that runs it.

**A level in districts.** Gravity Cascade is three districts in `logic/src/race/cascade/`, joined at two handoff roofs that `cascade/mod.rs` builds. Each district has its route in `logic/tests/cascade/<district>.rs` and its own test, `logic/tests/run_cascade_<district>.rs`, which crosses the district alone with `bot::cross`. `logic/tests/run_cascade.rs` runs the three routes as one, from the start to the finish.

**Check all levels.** `bun exact.mjs test-rust` also checks that each checkpoint and each finish is a place to stand.

**See it in the game.** `bun exact.mjs agent web --size 900x420 "tap play" "clock +600" "screenshot out.png"` shows the first level. `"type world message goto <x> <y> <z> <yaw>"` moves the runner.
