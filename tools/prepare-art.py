# Clean the raw Meshy models and write them into art/.
#
#   /Applications/Blender.app/Contents/MacOS/Blender -b --python tools/prepare-art.py -- <raw dir> art [name ...]
#
# A raw model is one .glb from Meshy: about one unit long, centred, with one 2K
# base colour image. For each model in SPECS this script:
#
# - scales it to metres and puts its origin on the ground, under its centre
#   (a `wall` model has its origin on its back face instead);
# - ticks Backface Culling and sets one roughness for the wet look;
# - makes a glow image from the bright, warm or pink parts of the base colour;
# - shrinks the images to the size in SPECS;
# - writes art/<name>.glb, and art/<name>-far.glb with a fraction of the triangles
#   and a 256 pixel base colour, for `ModelLod`;
# - prints the flat tops of the model (`DECK` lines), in the game's axes, which
#   `logic/src/kit.rs` uses to place its colliders.
#
# The front of every model faces +Z in the game, and its length is along X.
#
# Image sizes are small on purpose. The bake writes each image three times (RGBA8,
# BC and ASTC), and the web build refuses an asset tree over about 380 MB.
import bpy, bmesh, sys, os, math
import numpy as np
from mathutils import Vector, Matrix

# name: (metres along `axis`, axis, base colour size, glow rule, wall-mounted, far ratio)
SPECS = {
    "awning":      (3.2,  "x", 1024, "warm", True,  0.25),
    "balcony":     (3.6,  "x", 1024, "warm", True,  0.25),
    "billboard":   (8.0,  "x", 1024, "white", False, 0.25),
    "crane":       (10.0, "y", 1024, None,   False, 0.2),
    "elevator":    (16.0, "z", 1024, "warm", False, 0.2),
    "facade":      (12.0, "z", 1024, "warm", False, 0.2),
    "fire-escape": (5.4,  "x", 1024, None,   False, 0.2),
    "garden":      (9.0,  "x", 1024, "warm", False, 0.2),
    "hvac":        (9.0,  "x", 1024, None,   False, 0.2),
    "ladder":      (6.5,  "z", 1024, None,   True,  0.25),
    "neon":        (7.0,  "x", 1024, "pink", False, 0.25),
    "pipes":       (5.0,  "z", 1024, None,   True,  0.2),
    "pole":        (8.0,  "z", 1024, None,   False, 0.25),
    "scaffold":    (9.0,  "z", 1024, None,   False, 0.2),
    "skybridge":   (14.0, "x", 1024, "warm", False, 0.2),
    "stall":       (5.0,  "x", 1024, "warm", False, 0.2),
    "viaduct":     (24.0, "x", 1024, None,   False, 0.2),
    "walkway":     (14.0, "x", 1024, "warm", False, 0.2),
    "zip-anchor":  (2.6,  "z", 1024, None,   False, 0.25),
}
ROUGHNESS = 0.42
GLOW_STRENGTH = 4.0
GLOW_SIZE = 512
GLOW_LEAST = 0.0008
FAR_SIZE = 256


def bounds(obj):
    pts = [obj.matrix_world @ v.co for v in obj.data.vertices]
    lo = Vector((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts)))
    hi = Vector((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts)))
    return lo, hi


def glow_image(base, rule):
    """The parts of the base colour that are a light, on black."""
    w, h = base.size
    px = np.empty(w * h * 4, dtype=np.float32)
    base.pixels.foreach_get(px)
    px = px.reshape(h, w, 4)
    r, g, b = px[..., 0], px[..., 1], px[..., 2]
    if rule == "pink":
        mask = (r > 0.55) & (b > 0.3) & (g < 0.62 * r)
    elif rule == "white":
        mask = (r > 0.72) & (g > 0.7) & (b > 0.62)
    else:  # warm: lamps, lanterns, lit windows
        mask = ((r > 0.82) & (g > 0.5) & (b < 0.8 * r)) | ((r > 0.9) & (g > 0.88) & (b > 0.8))
    out = np.zeros_like(px)
    out[..., 3] = 1.0
    out[..., :3] = px[..., :3] * mask[..., None]
    img = bpy.data.images.new("glow", w, h, alpha=False)
    img.pixels.foreach_set(out.reshape(-1))
    img.scale(GLOW_SIZE, GLOW_SIZE)
    img.pack()
    return img, float(mask.mean())


def decks(obj, name):
    """Print the flat tops: upward faces grouped by height, in the game's axes."""
    mesh = obj.data
    mesh.calc_loop_triangles()
    bins = {}
    for t in mesh.loop_triangles:
        if t.normal.z < 0.85 or t.area < 1e-5:
            continue
        zs = [mesh.vertices[i].co.z for i in t.vertices]
        key = round(sum(zs) / 3 / 0.1)
        e = bins.setdefault(key, [0.0, 1e9, -1e9, 1e9, -1e9])
        e[0] += t.area
        for i in t.vertices:
            c = mesh.vertices[i].co
            e[1] = min(e[1], c.x); e[2] = max(e[2], c.x)
            e[3] = min(e[3], -c.y); e[4] = max(e[4], -c.y)
    for key in sorted(bins):
        area, x0, x1, z0, z1 = bins[key]
        if area >= 0.6:
            print(f"DECK {name} y={key * 0.1:.1f} area={area:.1f} x=[{x0:.2f},{x1:.2f}] z=[{z0:.2f},{z1:.2f}]")


def export(path):
    bpy.ops.export_scene.gltf(
        filepath=path, export_format="GLB", use_selection=False,
        export_image_format="JPEG", export_image_quality=88,
        export_yup=True, export_apply=True, export_animations=False,
        export_vertex_color="NONE", export_tangents=False,
        export_materials="EXPORT", export_cameras=False, export_lights=False,
    )


def prepare(raw, out, name):
    length, axis, size, rule, wall, far = SPECS[name]
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=os.path.join(raw, name + ".glb"))
    objs = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    for o in bpy.context.scene.objects:
        o.select_set(o.type == "MESH")
    bpy.context.view_layer.objects.active = objs[0]
    if len(objs) > 1:
        bpy.ops.object.join()
    obj = bpy.context.view_layer.objects.active
    obj.name = name
    # Unparent from the importer's root, keeping the world pose, then bake it in.
    world = obj.matrix_world.copy()
    obj.parent = None
    obj.matrix_world = world
    for o in list(bpy.context.scene.objects):
        if o is not obj:
            bpy.data.objects.remove(o)
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    lo, hi = bounds(obj)
    k = length / (hi - lo)["xyz".index(axis)]
    lo, hi = lo * k, hi * k
    # Blender +Y is the back of the model: the game's -Z.
    origin = Vector(((lo.x + hi.x) / 2, hi.y if wall else (lo.y + hi.y) / 2, lo.z))
    obj.data.transform(Matrix.Translation(-origin) @ Matrix.Scale(k, 4))
    obj.data.update()
    while obj.data.color_attributes:
        obj.data.color_attributes.remove(obj.data.color_attributes[0])
    for poly in obj.data.polygons:
        poly.use_smooth = True

    glow_share = 0.0
    for mat in obj.data.materials:
        mat.use_backface_culling = True
        nodes = mat.node_tree.nodes
        bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
        bsdf.inputs["Roughness"].default_value = ROUGHNESS
        bsdf.inputs["Metallic"].default_value = 0.0
        tex = next(n for n in nodes if n.type == "TEX_IMAGE")
        base = tex.image
        if rule:
            glow, glow_share = glow_image(base, rule)
            # A model with no lamp in its image gets no glow image.
            if glow_share >= GLOW_LEAST:
                node = nodes.new("ShaderNodeTexImage")
                node.image = glow
                mat.node_tree.links.new(node.outputs["Color"], bsdf.inputs["Emission Color"])
                bsdf.inputs["Emission Strength"].default_value = GLOW_STRENGTH
        if base.size[0] != size:
            base.scale(size, size)
            base.pack()

    lo, hi = bounds(obj)
    tris = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    print(f"MODEL {name} tris={tris} x=[{lo.x:.2f},{hi.x:.2f}] y=[{lo.z:.2f},{hi.z:.2f}] z=[{-hi.y:.2f},{-lo.y:.2f}] glow={glow_share:.3f}")
    decks(obj, name)
    export(os.path.join(out, name + ".glb"))

    mod = obj.modifiers.new("far", "DECIMATE")
    mod.ratio = far
    bpy.ops.object.modifier_apply(modifier=mod.name)
    for mat in obj.data.materials:
        base = next(n for n in mat.node_tree.nodes if n.type == "TEX_IMAGE" and n.image.name != "glow").image
        base.scale(FAR_SIZE, FAR_SIZE)
        base.pack()
    print(f"FAR {name} tris={sum(len(p.vertices) - 2 for p in obj.data.polygons)}")
    export(os.path.join(out, name + "-far.glb"))


args = sys.argv[sys.argv.index("--") + 1:]
raw, out, names = args[0], args[1], args[2:] or sorted(SPECS)
os.makedirs(out, exist_ok=True)
for name in names:
    prepare(raw, out, name)
