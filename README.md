# Rooftop Rumble

Rooftop Rumble is a small Exact game prototype. A jointed stick man runs, vaults, climbs, rolls and falls through an obstacle park. The player and every object share one Rapier physics world with real gravity.

## Controls

| | Keyboard | Gamepad | Touch |
|---|---|---|---|
| Move | WASD or arrows | Left stick | Move stick (a partial push walks) |
| Sprint | Hold Shift | Hold L3 or R1 | Hold Sprint |
| Jump, vault, climb | Space | South button | Jump |
| Slide | C or Left Control | East button | Slide |

Desktop and web keep the playfield clear. Coarse-pointer devices show the move stick, round Sprint, Slide and Jump buttons, and a Pause button, each drawn as the platform's own glass. On a screen under 600 points wide the three round buttons stand in a column. One finger can slide between the three round buttons: it presses the one it arrives on, and Sprint stays held while a finger that went down on it visits Jump or Slide. The stick and the round buttons are the same distance from the screen's edge. The world draws under the notch and the home indicator; the controls keep clear of them.

## Moves

Jump is the one parkour button. What it does depends on what is in front of the body.

| Situation | Result |
|---|---|
| A wall up to 1.15 m high and 1.2 m deep | Vault over it, at any approach angle up to 45 degrees. The body leans on its left hand, which is on the wall's top |
| A deeper or taller wall, up to 1.5 m | Climb onto it |
| In the air at an edge 1.45 to 2.1 m above the feet | Grab the edge and hang |
| In the air at a lower edge | Pull straight onto it |

A hanging body stays on the edge until the stick stops pushing at the wall. After that:

| Stick | Result |
|---|---|
| Along the wall | Shimmy that way, hand over hand, as far as the edge goes. At the end of the edge the body leans out past it on one hand |
| Toward the wall, or Jump with no aim | Climb onto the top |
| Away from the wall | Let go |
| Along the wall, and Jump | Leap that way and up, and catch the next edge on the wall if there is one |
| Away from the wall, and Jump | Jump off the wall, backward |

A body stands on its feet: with no ground within a foot's width under it, it slips off the edge it is past.

A slide starts from 2.5 m/s or more. It lasts 0.8 s, keeps 85 percent of the speed, steers a little, and ends in a run. Jump cuts it short.

Landings depend on the downward speed at touchdown, and each is deeper the harder it is.

| Speed | Drop | Result |
|---|---|---|
| Below 4.2 m/s | Up to about 0.9 m | The knees give |
| 4.2 to 6.3 m/s | Up to about 2 m | The body kneels into the landing and runs on. The deepest kneel shortens the next stride |
| 6.3 to 9 m/s | Up to about 4.1 m | A roll if the body is moving, a crouch to the hands if it is not. A harder roll is longer, slower out, and comes up through a deeper kneel |
| 9 m/s and above | Higher | The body falls as a ragdoll, comes to rest, and gets up |

In the air the pose follows the vertical speed, and the legs unlock in the last fifth of a second before the ground.

A prop that hits the body with 200 kg·m/s or more knocks it down. A lighter hit shoves it. The ramp releases a ball, a barrel, a crate or a heavy ball every 4.5 seconds.

## Code

- `logic/src/lib.rs`: setup and the tick.
- `logic/src/body.rs`: the capsule and `Action`, the one state for what the body does. `step` is the only place an action becomes another.
- `logic/src/ledge.rs`: finds the top edge of a wall with the physics queries. The vault, the climb, the grab and the shimmy all use it.
- `logic/src/hold.rs`: what the hands and feet hold. A move names the points on the wall, and after the Animator has posed the body the engine's two-bone solver puts each wrist and ankle on its point. A limb holds only what it can reach.
- `logic/src/park.rs`: the park as one table of blocks, and the ramp's rollers.
- `logic/src/ragdoll.rs`: fifteen points held together by constraints. The engine's physics has no joints, so the ragdoll is solved here and collides through the physics queries.
- `logic/src/rise.rs`: the two get-ups. Each is a list of stages that place the hips, wrists and ankles; the engine's two-bone solver turns the limbs to reach them, so a planted hand or foot does not move.
- `logic/src/gait.rs`, `logic/src/locomotion.rs`: the gait clips and the clock that keeps planted feet still.
- `logic/src/stickman.rs`: the skeleton, the other clips and the Animator, which has one state per action.
- `logic/src/debug.rs`: `goto`, `camera` and `roller` messages for setting up a scene from an agent.
- `app.contract`: the title screen and the touch controls.

## Tests

`bun exact.mjs test-rust` runs 36 hostless tests and the determinism lints.

- `logic/tests/parkour.rs`: every move in the park, checked on every tick. 56 vaults and 20 grabs across approach angles and speeds, climbs, a shimmy to both ends of a wall with the lean at each, leaps from the end of an edge, slides, the landing depths, the touch messages, the roof gaps, drops and stairs. On every tick of a vault, a hang and a climb no bone of the drawn body is in a wall. A vault's hand is on the wall's top, and hanging hands are within 1 cm of the edge.
- `logic/tests/ragdoll.rs`: falls from 6.5 and 9 m, four kinds of roller, a hit while hanging, and 56 throws into walls, corners, roofs and the ramp's lane. Each ends on its feet. Every tick checks the points, the bone lengths and the drawn pose.
- `logic/tests/sim.rs`: movement, the gait's planted feet, the arm swing and the rollers.

To see one move tick by tick, run the ignored `trace` test. It prints where the hips, shoulders and holds are and which bone, if any, is in the wall:

```sh
BLOCK=ledge-high UP=1 cargo test --manifest-path .shells/Cargo.toml -p exact-beat-em-up-logic --test parkour trace -- --ignored --nocapture
```

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
