# Neon Holdout

The combat level is a circular rooftop holdout. The player keeps full parkour movement while enemies arrive from four staggered spawn pads. Each round adds two enemies and trims the spawn interval, so the pressure rises without changing the rules.

The combat state is explicit. `Ready` starts the pistols low. `Raising` lifts both pistols over 0.35 seconds. `Firing` alternates left and right shots every 0.16 seconds. `RoundClear` gives a short reset between waves. `Defeated` freezes enemy pursuit after the third contact.

An enemy is a lightweight stickman-shaped capsule with one health point and a three-contact attack budget on the player. Enemies seek the player until they are within grab distance, then stay attached and apply one contact every 0.65 seconds. The player auto-aims at the nearest living enemy. Shots use bright, short-lived tracer meshes and a muzzle flash at the active pistol.

Round counts are four, six, eight, and ten enemies. Spawn points rotate around the player, so each round asks for a different movement route. The HUD publishes the round, remaining enemies, and player hearts for Contract to render.
