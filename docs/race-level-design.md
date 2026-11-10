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
- A runner that hangs can move along the edge at 0.8 m/s, and can jump along it or off it.
- A drop of up to 0.9 m needs no recovery. Up to 2 m the runner kneels and runs on. Up to about 3.5 m it rolls. A drop of 4 m or more throws the runner down, and it needs 3.5 s to get up.
- A slide starts at 2.5 m/s or more. It covers about 2.5 m from a run and 4.6 m from a sprint. Under a bar the slide goes on until the runner is clear.
- A slope up to 50 degrees is walked. A step up to 0.15 m is walked.
- A slope slows a sprint. Up a ramp or a stair of 12 degrees the runner sprints at 5.9 m/s, at 14 degrees 5.3 m/s, at 17 degrees 3.8 m/s and at 21 degrees 2.5 m/s. A stair that must keep the speed has a run of 4.5 times its rise.
- A sprint landing on a roof 1.5 m lower can roll. The roll takes 0.7 s and leaves the runner at 3.4 m/s. Steps of 1 m in a run of jumps keep the speed.

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
12. A level has at most 400 pieces. One stair is about 7 pieces for each metre of rise.

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

`LEVEL` is 0 for Skyline Sprint, 1 for Switchback Run and 2 for Helix Heights. The comment at the top of `tools/map.py` has the other arguments: a trail, the angle of the view, and a crop.

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
| `Jump` | Presses Jump. Put it after a `Run` or `Sprint` to a point 0.3 to 0.5 m before an edge, or 0.6 to 0.9 m before a wall |
| `Slide` | Presses Slide. Put it after a `Sprint` to a point 2 m before a gate |
| `Hang` | Waits until the hands hold an edge |
| `Shimmy(x, z)` | On an edge, moves the hands toward the point |
| `Hold(seconds)`, `Wait(seconds)` | Keeps the stick, or lets it go, for a time |

`logic/tests/envelope.rs` has a short course with every kind of piece and the route that runs it.

**Check all levels.** `bun exact.mjs test-rust` also checks that each checkpoint and each finish is a place to stand.

**See it in the game.** `bun exact.mjs agent web --size 900x420 "tap play" "clock +600" "screenshot out.png"` shows the first level. `"type world message goto <x> <y> <z> <yaw>"` moves the runner.
