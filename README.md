# Rooftop Rumble

Rooftop Rumble is a small Exact game prototype. A runner runs, vaults, climbs, rolls and falls through an obstacle park. The player and every object share one Rapier physics world with real gravity.

The start screen shows two runners, Ash and Ember. The chosen runner turns in a small view of the world, and goes into the level that the player picks. Both runners are generated bodies on the same skeleton, so they move the same. [Generating assets](docs/asset-generation.md) describes how a body is made. Without a choice (`character=0`, which the tests use) the body is an articulated wooden mannequin.

## Controls

| | Keyboard | Gamepad | Touch |
|---|---|---|---|
| Move | WASD or arrows | Left stick | Move stick (a partial push walks) |
| Sprint | Hold Shift | Hold L3 or R1 | Hold Sprint |
| Jump, vault, climb | Space | South button | Jump |
| Slide | C or Left Control | East button | Slide |

The camera is behind the runner's right shoulder, at eye height. The runner stands left of the screen centre, clear of the move pad and the buttons. Portrait and landscape each have their own framing, and a rotated phone eases from one to the other.

Swipe or click-drag the scene to look around. A drag of the same length always turns the view by the same angle. The first 6 points of a touch do not turn the view, so a tap does not take the camera. The move pad and the action buttons can be held while another finger pans the scene.

The view turns behind your direction of travel while the stick is pushed. After a swipe the view stays yours for one second, then the follow returns gently. A held stick direction keeps a straight path while the view turns. A stick that moves more than 15 degrees is read against the screen again. A swipe turns the path of a held stick with the view.

The camera trails the runner a little at speed and widens its view in a sprint. It stays almost still in a jump, tilts down in a fall, lowers in a slide and looks down on a hang from behind. Next to a wall it moves in behind the runner, and it shortens before the lens can enter a wall. A slow handheld drift, a nod with each footfall and a short rattle on a hard landing turn the view. They never move it. Every number is in `camera::tune` in `logic/src/camera.rs`.

Desktop and web keep the playfield clear. Coarse-pointer devices show a circular move pad, round Sprint, Slide and Jump buttons, and a Pause button. The controls have translucent backgrounds and visible outlines. The move pad's thumb follows the actual analog input, stays inside its circular range, and recenters when released. On a screen under 600 points wide the three round buttons stand in a column. One finger can slide between the three round buttons: it presses the one it arrives on, and Sprint stays held while a finger that went down on it visits Jump or Slide. The stick and the round buttons are the same distance from the screen's edge. The world draws under the notch and the home indicator; the controls keep clear of them.

## Running

The gait changes with the speed, from a shuffle to a sprint. In a sprint the trunk leans forward from the pelvis, the arms drive far forward and far back, and the heel of the free leg comes up to the hip. At every pace the pelvis drops on the side of the leg that swings, and the shoulders tip the other way.

The body leans the way it is pushed. It leans forward when it speeds up, back when it brakes and into a turn. The hips go that way and sink, and the trunk leans over them. The feet stay where the gait put them, so a planted foot does not slide. A body that starts drives off a leg behind it. A body that brakes lands on a leg in front of it. The lean follows the push as a spring with no bounce, and it is at most 0.42 rad.

An about-face is one motion. The push points the new way from the first tick, so the lean goes from the back, over one side, to the front. The body starts its turn while it still brakes and is side-on when it stops. It turns away from the foot that is next on the ground, so that foot is on the outside and pushes the body off. The head turns into the turn first. The controls and the speeds are the same as before: the lean changes only the drawn body.

## Moves

Jump is the one parkour button. What it does depends on what is in front of the body.

| Situation | Result |
|---|---|
| A wall up to 1.15 m high and 1.2 m deep | Vault over it, at any approach angle up to 45 degrees. The body leans on its left hand, which is on the wall's top |
| A deeper or taller wall, up to 1.5 m | Climb onto it |
| In the air at an edge 1.45 to 2.1 m above the feet | Grab the edge and hang. The hands catch it while the body still goes up at up to 3 m/s |
| In the air at a lower edge | Pull straight onto it, as soon as the jump cannot clear the edge |

A jump starts from the gait. The foot that came down last pushes, and the other leg leads. The body lands on the leading foot and the run goes on from that foot's strike. Two jumps in a row lead with different legs. The speed at take-off sets the pose in the air: a tuck on both legs from a stand, a stride from a run, a wide split from a sprint. In the last fifth of a second before the ground the leading foot reaches for the place where it will strike.

A climb is one course from a hang to a stand: pull up, press the hips up over the edge, put one foot on the top, rock over it and stand. It takes 1.08 s from a hang. A body that climbs from the ground or catches a lower edge is already part of the way up. It joins the course at its own height, so its pose fits the height of the wall. What the body is off the course when it joins, and how fast, dies away as a spring: a run at the wall carries the body to it, and a fall onto an edge sags and comes back. The catch of a hang works the same way. The leg that led the jump steps onto the top first.

On a wall the legs keep out of it. A knee or a foot that the pose has in the wall moves out by the nearer way, through the face or over the top, and that way changes without a step, so a leg near the edge does not flick in and out. In the air the legs start to keep out of a wall as it comes into sight.

A hanging body stays on the edge until the stick stops pushing at the wall. After that:

| Stick | Result |
|---|---|
| Along the wall | Shimmy that way, hand over hand, as far as the edge goes. At the end of the edge the body leans out past it on one hand |
| Toward the wall, or Jump with no aim | Climb onto the top |
| Away from the wall | Let go |
| Along the wall, and Jump | Leap that way and up, and catch the next edge on the wall if there is one |
| Away from the wall, and Jump | Jump off the wall, backward |

A body stands on its feet: with no ground within a foot's width under it, it slips off the edge it is past.

A slide starts from 2.5 m/s or more. On level ground it lasts 0.8 s, and the ground takes 20 percent of its speed each second. On a slope, gravity adds to its speed downhill and takes from it uphill. The fastest slide is 10 m/s. While a slope of about 4 degrees or more pulls the body on, the slide lasts to the end of that slope, and the body lies along the slope.

The stick leans the body into a turn. The stick and the lean together turn the slide at up to 2.2 rad/s.

| Lean | Pose |
|---|---|
| None | The body sits back and a little to the left, the left hand on the ground behind it |
| To the left | The body is further over the left hand, and the right arm is higher |
| To the right | The body is on its right side, the right hand on the ground, the left arm out |

The weight goes from one side to the other in half a second, and one hand comes off the ground while the other goes down.

A slide ends in a run, and Jump cuts it short. Speed that a slope gave the body above the speed of its legs stays with it after the slide. That speed decreases by 1.5 m/s² on the ground and does not decrease in the air.

Landings depend on the downward speed at touchdown, and each is deeper the harder it is.

| Speed | Drop | Result |
|---|---|---|
| Below 4.2 m/s | Up to about 0.9 m | The knees give |
| 4.2 to 6.3 m/s | Up to about 2 m | The body kneels into the landing and runs on. The deepest kneel shortens the next stride |
| 6.3 to 9 m/s | Up to about 4.1 m | A roll if the body is moving, a crouch to the hands if it is not. A harder roll is longer, slower out, and comes up through a deeper kneel |
| 9 m/s and above | Higher | The body falls as a ragdoll, comes to rest, and gets up |

In the air the pose follows the vertical speed.

No move fades in from a frozen pose. At a change of move each joint keeps the turn and the speed it had, and the difference from the new pose dies away as a spring. A hold that takes a hand or a foot, lets it go, or moves along the wall does the same for that limb. No wrist or ankle moves more than 0.1 m in one tick beyond the body's own motion.

A prop that hits the body with 200 kg·m/s or more knocks it down. A lighter hit shoves it. The ramp releases a ball, a barrel, a crate or a heavy ball every 4.5 seconds.

## Code

- `logic/src/lib.rs`: setup and the tick.
- `logic/src/body.rs`: the capsule and `Action`, the one state for what the body does. `step` is the only place an action becomes another.
- `logic/src/air.rs`: the three clips of the air, the leg that leads and the mirror that swaps the sides of a pose.
- `logic/src/climb.rs`: the climb's course and the spring that brings a body onto it.
- `logic/src/flow.rs`: carries the pose of one move into the next.
- `logic/src/ledge.rs`: finds the top edge of a wall with the physics queries. The vault, the climb, the grab and the shimmy all use it.
- `logic/src/hold.rs`: what the hands and feet hold. A move names the points on the wall, and after the Animator has posed the body the engine's two-bone solver puts each wrist and ankle on its point. A limb holds only what it can reach, and goes nowhere in one tick.
- `logic/src/park.rs`: the park as one table of blocks, and the ramp's rollers.
- `logic/src/ragdoll.rs`: fifteen points held together by constraints. The engine's physics has no joints, so the ragdoll is solved here and collides through the physics queries.
- `logic/src/rise.rs`: the two get-ups. Each is a list of stages that place the hips, wrists and ankles; the engine's two-bone solver turns the limbs to reach them, so a planted hand or foot does not move.
- `logic/src/gait.rs`, `logic/src/locomotion.rs`: the gait clips and the clock that keeps planted feet still. `body::step` keeps the lean and `body::balance` lays it over the pose after the holds.
- `logic/src/stickman.rs`: the skeleton, the other clips and the Animator, which has one state per action. `dressed` puts a body from `art/` on the skeleton in place of the mannequin.
- `logic/src/mannequin.rs`: the shaped mannequin mesh: rounded chest and pelvis, tapered limbs, ball joints, hands and feet. It keeps the existing skeleton and hand sockets; rigid limb weights retain the carved shapes while the torso blends across the spine.
- `logic/src/debug.rs`: `goto`, `camera` and `roller` messages for setting up a scene from an agent.
- `app.contract`: the title screen with the choice of runner, and the touch controls.
- `art/body_a.glb`, `art/body_b.glb`: the two runners. `tools/fit_body.py` makes such a file from a Meshy rig, in Blender.

## Tests

`bun exact.mjs test-rust` runs 91 hostless tests and the determinism lints. The tests load the baked `art/` with `exact_game::sim_with_art!`.

- `logic/tests/mannequin.rs`: closed mesh topology, normals, skin weights, height scaling and compatibility with the animation skeleton and hand sockets.

- `logic/tests/bodies.rs`: each body in `art/` has the rig's joints at the rig's places, one mesh inside the vertex budget with valid weights, and no double-sided material. The player runs the same course in each body. The start screen's view keeps the body in place while the eye goes round it.

- `logic/tests/parkour.rs`: every move in the park, checked on every tick. 56 vaults and 20 grabs across approach angles and speeds, climbs, a shimmy to both ends of a wall with the lean at each, leaps from the end of an edge, slides on level ground, through a turn to each side, and down and up the ramp, the landing depths, the touch messages, the roof gaps, drops and stairs, and the lean of a start, a stop and an about-face at a run and at a sprint. On every tick of a vault, a hang and a climb no bone of the drawn body is in a wall. A vault's hand is on the wall's top, and hanging hands are within 1 cm of the edge. One test runs 16 jumps, catches and climbs and fails if a wrist or an ankle moves more than 0.1 m in one tick. Another runs 16 catches and climbs and fails if a joint shakes on the wall: if the change of its speed turns round from one tick to the next by more than 0.075 m.
- `logic/tests/ragdoll.rs`: falls from 6.5 and 9 m, four kinds of roller, a hit while hanging, and 56 throws into walls, corners, roofs and the ramp's lane. Each ends on its feet. Every tick checks the points, the bone lengths and the drawn pose.
- `logic/tests/sim.rs`: movement, the gait's planted feet, the arm swing and the rollers.

To see one move tick by tick, run the ignored `trace` test. It prints where the hips, shoulders and holds are and which bone, if any, is in the wall:

```sh
BLOCK=ledge-high UP=1 cargo test --manifest-path .shells/Cargo.toml -p exact-beat-em-up-logic --test parkour trace -- --ignored --nocapture
```

To see one move as a film strip, run the ignored `film` test and draw its output. Each frame shows the body from the side and from behind:

```sh
SCENE=wall BLOCK=ledge-mid UP=110 RELEASE=1 FILM=/tmp/film.jsonl cargo test --manifest-path .shells/Cargo.toml -p exact-beat-em-up-logic --test parkour film -- --ignored
python3 tools/film.py /tmp/film.jsonl /tmp/film.png 0 250 4 8
```

`SCENE` is `wall`, `jumps`, `gap`, `slide` or `ramp`. The test's comment lists the other variables.

`.cargo/config.toml` turns off debug assertions for the vendored `rapier3d` package only. Its island check panics when the player's kinematic capsule moves in the tick in which a rigid body starts a contact.

The framework checkout is at `/private/tmp/exact-framework-20261109`, which a restart of the Mac removes. To restore it:

```sh
git clone --no-checkout <path to your exact2 repository> /private/tmp/exact-framework-20261109
git -C /private/tmp/exact-framework-20261109 checkout --detach 94b8671f7
(cd /private/tmp/exact-framework-20261109 && bun install --frozen-lockfile)
```

## Commands

```sh
bun exact.mjs contract build app.contract --json
bun exact.mjs test-rust
bun exact.mjs test web
```

Run the web loop with `bun exact.mjs web`. Build for the iOS simulator with `bun exact.mjs ios --run`.

To look at one move from an agent:

```sh
bun exact.mjs agent web "tap play" "type world message camera 4.6 0.4 0.9" \
  "type world message goto -3 0 -10.5 180" "type world key KeyW down" "clock +260" \
  "type world key Space" "screenshot climb.png over 1680 every 140"
```

## Install on a phone

```sh
scripts/phone.sh "<phone name or UDID>"
```

The script signs with the EAS ad hoc profile and distribution certificate in `release/credentials/` (not in git). `release/` is the EAS project. To add a phone, register it with `eas device:create`, then run `eas credentials:configure-build -p ios -e device` and download the credentials again with `eas credentials -p ios`. The certificate is in the keychain `exact-beat-em-up-signing`.

## TestFlight

```sh
scripts/testflight.sh 2
```

The argument is the build number; raise it for each upload. EAS Build cannot build an Exact app, so the script builds the signed `.ipa` here and `eas submit` uploads it. Apple processes the build after the upload. Check it with `eas submit:status -p ios --profile production --json --non-interactive` in `release/`: `processingState` is `VALID` when it is ready.
