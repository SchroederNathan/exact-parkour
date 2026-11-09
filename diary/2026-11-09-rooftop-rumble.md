# Rooftop Rumble platform slice

## Rough

The first loop is a flat platform where a stick man can walk, run, jump, and stop. Web and desktop show the platform without a gameplay HUD. Coarse-pointer devices get a move stick and glass pause and jump controls.

## Lean in

The player is a typed `Character` with explicit acceleration, braking, speed, jump, gravity, and platform bounds. A named bone hierarchy drives a generated skinned model. Idle, walk, and run clips blend from planar ground speed. The follow camera sits above and behind the player with a short lag.

## Needed

Combat, enemies, attacks, hit reactions, and ragdoll physics are intentionally deferred. The rig keeps stable joints and parent links so physics can replace procedural animation later.

## Checkpoints

- Contract compiles with no diagnostics.
- Hostless movement, braking, diagonal speed, jump, and save-restore tests pass.
- Web build and Contract tests pass.
- Web screenshots show a clear desktop platform and coarse-pointer controls.
- Native iOS build completes. Device inspection remains pending because the selected simulator is held by another agent-device session.
