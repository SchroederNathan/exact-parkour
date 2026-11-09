# Rooftop Rumble

Rooftop Rumble is a small Exact game prototype. The current slice is a flat platform with a jointed stick man who walks, runs, sprints and jumps.

## Controls

| | Keyboard | Gamepad | Touch |
|---|---|---|---|
| Move | WASD or arrows | Left stick | Move stick (a partial push walks) |
| Sprint | Hold Shift | Hold L3 or R1 | Sprint toggle |
| Jump | Space | South button | Jump |

Desktop and web keep the playfield clear. Coarse-pointer devices show the move stick and native glass Pause, Sprint and Jump buttons.

## Code

- `logic/src/lib.rs`: the world, the input actions and the tick. The player uses Exact's `Character` controller.
- `logic/src/stickman.rs`: the skeleton, the jump and landing clips, and the Animator.
- `logic/src/gait.rs`: the gait clips. Each foot follows a path that is still on the ground during its stance; a two-bone solve turns the path into joint turns. `PACES` holds six gaits from a shuffle (0.8 m/s) to a sprint (6.2 m/s).
- `logic/src/locomotion.rs`: drives the Animator each tick. The stride shape follows ground speed; the cycle clock advances by the amount that keeps the planted foot in place.
- `app.contract`: the title screen and the touch controls.
- `logic/tests/sim.rs`: hostless tests. They measure how far planted feet slide and how far an ankle moves in one tick.

A native glass button cannot be a child of the canvas, and only a canvas button can hold an action. So Jump posts a message and Sprint is a toggle passed as the `sprint` argument.

## Commands

```sh
bun exact.mjs contract build app.contract --json
bun exact.mjs test-rust
bun exact.mjs test web
```

Run the web loop with `bun exact.mjs web`. Build for the iOS simulator with `bun exact.mjs ios --run`.

## Install on a phone

```sh
scripts/phone.sh 00008160-00093DAE0EC0000A
```

The script signs with the EAS ad hoc profile and distribution certificate in `release/credentials/` (not in git). `release/` is the EAS project `@exponathan/exact-beat-em-up`. To add a phone, register it with `eas device:create`, then run `eas credentials:configure-build -p ios -e device` and download the credentials again with `eas credentials -p ios`. The certificate is in the keychain `exact-beat-em-up-signing`.
