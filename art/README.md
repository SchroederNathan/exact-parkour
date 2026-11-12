# Cyberpunk art source

`tools/cyberpunk_blender.py` is the source generator for a modular skyline diorama and a masked humanoid reference sculpt. It runs headlessly in Blender 3.6+ and writes `art/cyberpunk_city.glb` plus `art/textures/neon_grid.png`:

```sh
blender --background --python tools/cyberpunk_blender.py -- --output art/cyberpunk_city.glb
```

The checked-in GLB was generated with Blender 5.2.3 and inspected through the local Blender MCP server. It contains 284 mesh objects, 28,384 triangles, eight materials, and the packed 256 × 256 neon texture. The runtime game keeps its deterministic procedural scene and uses the same generator as an art source and review artifact.
