# Cyberpunk art source

`tools/cyberpunk_blender.py` is the source generator for a modular skyline diorama and a masked humanoid reference sculpt. It runs headlessly in Blender 3.6+ and writes `art/cyberpunk_city.glb` plus `art/textures/neon_grid.png`:

```sh
blender --background --python tools/cyberpunk_blender.py -- --output art/cyberpunk_city.glb
```

The Exact bake accepts the generated GLB's standard meshes, UVs, materials and PNG texture. Blender is not installed in the current build container, so the script is kept as a reproducible source artifact and is syntax-checked with Python here; the Rust game continues to use its deterministic procedural assets until the GLB is baked.
