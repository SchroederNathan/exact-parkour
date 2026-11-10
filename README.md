# Rooftop Rumble

Rooftop Rumble is a small Exact game prototype. A jointed stick man runs, vaults, climbs, rolls and falls through an obstacle park. The player and every object share one Rapier physics world with real gravity.

## Controls

| | Keyboard | Gamepad | Touch |
|---|---|---|---|
| Move | WASD or arrows | Left stick | Move stick (a partial push walks) |
| Sprint | Hold Shift | Hold L3 or R1 | Hold Sprint |
| Jump, vault, climb | Space | South button | Jump |

Desktop and web keep the playfield clear. Coarse-pointer devices show the move stick, round Sprint and Jump buttons, and a native glass Pause button. The world draws under the notch and the home indicator; the controls keep clear of them.

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
| Along the wall | Shimmy that way, hand over hand, as far as the edge goes |
| Toward the wall, or Jump | Climb onto the top |
| Away from the wall | Let go |

Landings depend on the downward speed at touchdown.

| Speed | Drop | Result |
|---|---|---|
| Below 5 m/s | Up to about 1.6 m | The knees take it |
| 5 to 9 m/s | Up to about 4.4 m | A roll if the body is moving, a crouch to the hands if it is not. The recovery is longer for a harder landing |
| 9 m/s and above | Higher | The body falls as a ragdoll, comes to rest, and gets up |

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

`bun exact.mjs test-rust` runs 31 hostless tests and the determinism lints.

- `logic/tests/parkour.rs`: every move in the park, checked on every tick. 56 vaults and 20 grabs across approach angles and speeds, climbs, a shimmy to both ends of a wall, the roof gaps, drops and stairs. On every tick of a vault, a hang and a climb no bone of the drawn body is in a wall. A vault's hand is on the wall's top, and hanging hands are within 1 cm of the edge.
- `logic/tests/ragdoll.rs`: falls from 6.5 and 9 m, four kinds of roller, a hit while hanging, and 56 throws into walls, corners, roofs and the ramp's lane. Each ends on its feet. Every tick checks the points, the bone lengths and the drawn pose.
- `logic/tests/sim.rs`: movement, the gait's planted feet, the arm swing and the rollers.

To see one move tick by tick, run the ignored `trace` test. It prints where the hips, shoulders and holds are and which bone, if any, is in the wall:

```sh
BLOCK=ledge-high UP=1 cargo test --manifest-path .shells/Cargo.toml -p exact-beat-em-up-logic --test parkour trace -- --ignored --nocapture
```

`.cargo/config.toml` turns off debug assertions for the vendored `rapier3d` package only. Its island check panics when the player's kinematic capsule moves in the tick in which a rigid body starts a contact.

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
scripts/phone.sh 00008160-00093DAE0EC0000A
```

The script signs with the EAS ad hoc profile and distribution certificate in `release/credentials/` (not in git). `release/` is the EAS project `@exponathan/exact-beat-em-up`. To add a phone, register it with `eas device:create`, then run `eas credentials:configure-build -p ios -e device` and download the credentials again with `eas credentials -p ios`. The certificate is in the keychain `exact-beat-em-up-signing`.

## TestFlight

```sh
scripts/testflight.sh 2
```

The argument is the build number; raise it for each upload. EAS Build cannot build an Exact app, so the script builds the signed `.ipa` here and `eas submit` uploads it. The App Store Connect app is 6832787049, named "Rooftop Rumble (6c5aef)" there because "Rooftop Rumble" was taken. Apple processes the build after the upload. Check it with `eas submit:status -p ios --profile production --json --non-interactive` in `release/`: `processingState` is `VALID` when it is ready.
