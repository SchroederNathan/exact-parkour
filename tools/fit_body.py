"""Fit a Meshy rigged body to the game's skeleton. Run it inside Blender.

    import runpy
    runpy.run_path("tools/fit_body.py")["fit"]("rigged.glb", "art/body_x.glb", "body_x")

Meshy rigs a body in a T-pose on its own 24 joints. The game animates one
skeleton, the one `logic/src/stickman.rs` builds, whose arms hang at rest and
whose joints rest unrotated. This script:

1. imports the rigged GLB and scales the body to the game's height,
2. stretches each arm along its length until the elbow and the wrist are as
   far from the shoulder as the game's,
3. swings the arms down to the game's rest pose (dual quaternion blending, so
   the shoulder keeps its volume),
4. adds each Meshy joint's weights to the game joint that moves that part,
5. builds the game's joints as an armature and exports one GLB.

The trunk and the legs keep their shape: the game's joints are put inside them.
"""

import math

import bpy
import numpy as np

HEIGHT = 1.8
TEXTURE = 1024

# The joints of `stickman::rig`, at a height of 1.8 m: +X is the body's left,
# +Y is up, +Z is forward. The order is the order of the rig's bones.
JOINTS = [
    ("root", None, (0.0, 0.0, 0.0)),
    ("hips", "root", (0.0, 0.96, 0.0)),
    ("spine", "hips", (0.0, 1.1, 0.0)),
    ("chest", "spine", (0.0, 1.32, 0.0)),
    ("neck", "chest", (0.0, 1.48, 0.0)),
    ("head", "neck", (0.0, 1.68, 0.0)),
]
for _side, _x in (("l", 1.0), ("r", -1.0)):
    JOINTS += [
        (f"thigh_{_side}", "hips", (0.09 * _x, 0.98, 0.0)),
        (f"shin_{_side}", f"thigh_{_side}", (0.09 * _x, 0.55, 0.0)),
        (f"foot_{_side}", f"shin_{_side}", (0.09 * _x, 0.10, -0.03)),
        (f"upper_arm_{_side}", "chest", (0.19 * _x, 1.44, 0.0)),
        (f"forearm_{_side}", f"upper_arm_{_side}", (0.23 * _x, 1.17, 0.0)),
        (f"hand_{_side}", f"forearm_{_side}", (0.24 * _x, 0.91, 0.02)),
    ]
NAMES = [name for name, _, _ in JOINTS]
AT = {name: np.array(at) * (HEIGHT / 1.8) for name, _, at in JOINTS}

# Which game joint moves the part that each Meshy joint moves.
FROM_MESHY = {
    "Hips": "hips",
    "Spine02": "spine",
    "Spine01": "spine",
    "Spine": "chest",
    "neck": "neck",
    "Head": "head",
    "head_end": "head",
    "headfront": "head",
}
for _meshy, _side in (("Left", "l"), ("Right", "r")):
    FROM_MESHY.update(
        {
            f"{_meshy}Shoulder": "chest",
            f"{_meshy}Arm": f"upper_arm_{_side}",
            f"{_meshy}ForeArm": f"forearm_{_side}",
            f"{_meshy}Hand": f"hand_{_side}",
            f"{_meshy}UpLeg": f"thigh_{_side}",
            f"{_meshy}Leg": f"shin_{_side}",
            f"{_meshy}Foot": f"foot_{_side}",
            f"{_meshy}ToeBase": f"foot_{_side}",
        }
    )

# The middle of the mannequin's foot, front to back.
FOOT_MIDDLE = 0.052


def unit(v):
    return v / np.linalg.norm(v)


def turn_onto(a, b):
    """The smallest rotation that turns the direction `a` onto `b`."""
    a, b = unit(a), unit(b)
    axis = np.cross(a, b)
    sin, cos = np.linalg.norm(axis), float(np.dot(a, b))
    if sin < 1e-9:
        return np.eye(3)
    k = axis / sin
    cross = np.array([[0, -k[2], k[1]], [k[2], 0, -k[0]], [-k[1], k[0], 0]])
    return np.eye(3) + sin * cross + (1 - cos) * (cross @ cross)


def stretch(axis, by):
    """Scale by `by` along `axis`, and not across it."""
    axis = unit(axis)
    return np.eye(3) + (by - 1.0) * np.outer(axis, axis)


def quaternion(m):
    """(w, x, y, z) of a rotation matrix, with w positive."""
    w = math.sqrt(max(0.0, 1.0 + m[0, 0] + m[1, 1] + m[2, 2])) / 2.0
    if w < 1e-6:
        raise ValueError("a half turn has no smallest rotation")
    return np.array(
        [w, (m[2, 1] - m[1, 2]) / (4 * w), (m[0, 2] - m[2, 0]) / (4 * w), (m[1, 0] - m[0, 1]) / (4 * w)]
    )


def multiply(a, b):
    aw, ax, ay, az = a[..., 0], a[..., 1], a[..., 2], a[..., 3]
    bw, bx, by, bz = b[..., 0], b[..., 1], b[..., 2], b[..., 3]
    return np.stack(
        [
            aw * bw - ax * bx - ay * by - az * bz,
            aw * bx + ax * bw + ay * bz - az * by,
            aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw,
        ],
        axis=-1,
    )


def conjugate(q):
    return q * np.array([1.0, -1.0, -1.0, -1.0])


def blend_rigid(points, weights, turns, moves):
    """Move each point by the blend of the rigid moves `p -> turns[j] @ p + moves[j]`.

    The blend is of dual quaternions: a point between two joints goes round an
    arc, where a blend of matrices takes it straight across and loses volume.
    """
    real = np.array([quaternion(t) for t in turns])
    shift = np.concatenate([np.zeros((len(moves), 1)), np.array(moves)], axis=1)
    dual = 0.5 * multiply(shift, real)
    q = weights @ real
    d = weights @ dual
    norm = np.linalg.norm(q, axis=1, keepdims=True)
    q, d = q / norm, d / norm
    move = 2.0 * multiply(d, conjugate(q))[:, 1:]
    p = np.concatenate([np.zeros((len(points), 1)), points], axis=1)
    return multiply(multiply(q, p), conjugate(q))[:, 1:] + move


def fit(source, out, name):
    scene = bpy.data.scenes.get(name) or bpy.data.scenes.new(name)
    bpy.context.window.scene = scene
    for o in list(scene.objects):
        bpy.data.objects.remove(o, do_unlink=True)
    # What an earlier run of this body left, so that the names stay the same.
    for blocks in (bpy.data.meshes, bpy.data.armatures, bpy.data.materials, bpy.data.images):
        for block in list(blocks):
            if block.name in (name, name + "_rig") and block.users == 0:
                blocks.remove(block)
    bpy.ops.import_scene.gltf(filepath=source, merge_vertices=True)
    armature = next(o for o in scene.objects if o.type == "ARMATURE")
    body = next(o for o in scene.objects if o.type == "MESH" and o.vertex_groups)
    mesh = body.data

    # Blender is Z up and the body faces -Y. The game is Y up and it faces +Z.
    def game(points):
        return np.stack([points[:, 0], points[:, 2], -points[:, 1]], axis=1)

    count = len(mesh.vertices)
    local = np.empty(count * 3)
    mesh.vertices.foreach_get("co", local)
    world = np.array(body.matrix_world)
    at = game(local.reshape(count, 3) @ world[:3, :3].T + world[:3, 3])
    bones = {b.name: np.array(armature.matrix_world @ b.head_local) for b in armature.data.bones}
    meshy = dict(zip(bones, game(np.array(list(bones.values())))))

    # The weights, a column for each game joint.
    column = {n: i for i, n in enumerate(NAMES)}
    group = {g.index: column[FROM_MESHY[g.name]] for g in body.vertex_groups}
    weights = np.zeros((count, len(NAMES)))
    for v in mesh.vertices:
        for g in v.groups:
            weights[v.index, group[g.group]] += g.weight
    loose = int((weights.sum(axis=1) < 1e-6).sum())
    if loose:
        raise ValueError(f"{loose} vertices have no weight")
    weights /= weights.sum(axis=1, keepdims=True)

    # To the game's height, with the soles on the ground and the feet where the
    # mannequin's are, front to back.
    scale = HEIGHT / (at[:, 1].max() - at[:, 1].min())
    feet = weights[:, column["foot_l"]] + weights[:, column["foot_r"]] > 0.5
    middle = (at[feet, 2].max() + at[feet, 2].min()) / 2.0
    origin = np.array([0.0, at[:, 1].min(), middle - FOOT_MIDDLE / scale])
    at = (at - origin) * scale
    meshy = {n: (p - origin) * scale for n, p in meshy.items()}
    print(f"{name}: scale {scale:.4f}, {count} vertices")

    turns = [np.eye(3) for _ in NAMES]
    moves = [np.zeros(3) for _ in NAMES]
    stretched = at.copy()
    for meshy_side, side in (("Left", "l"), ("Right", "r")):
        upper, fore, hand = (f"{part}_{side}" for part in ("upper_arm", "forearm", "hand"))
        shoulder, elbow, wrist = (meshy[f"{meshy_side}{j}"] for j in ("Arm", "ForeArm", "Hand"))
        along = [unit(elbow - shoulder), unit(wrist - elbow)]
        length = [np.linalg.norm(AT[fore] - AT[upper]), np.linalg.norm(AT[hand] - AT[fore])]
        by = [length[0] / np.linalg.norm(elbow - shoulder), length[1] / np.linalg.norm(wrist - elbow)]
        print(f"  {side} arm: upper x{by[0]:.3f}, fore x{by[1]:.3f}")
        # In the T-pose: each part longer along its own line, the next part moved out with it.
        new_elbow = shoulder + along[0] * length[0]
        new_wrist = new_elbow + along[1] * length[1]
        parts = [
            (upper, lambda p: shoulder + (p - shoulder) @ stretch(along[0], by[0]).T),
            (fore, lambda p: new_elbow + (p - elbow) @ stretch(along[1], by[1]).T),
            (hand, lambda p: new_wrist + (p - wrist)),
        ]
        for joint, moved in parts:
            w = weights[:, column[joint]][:, None]
            stretched += w * (moved(at) - at)
        # Then down to the rest pose: the upper arm about the shoulder, the
        # forearm and the hand with it and onto the game's forearm line.
        swing = turn_onto(along[0], AT[fore] - AT[upper])
        bend = turn_onto(swing @ along[1], AT[hand] - AT[fore]) @ swing
        for joint, turn, pivot in ((upper, swing, shoulder), (fore, bend, new_elbow), (hand, bend, new_wrist)):
            turns[column[joint]] = turn
            moves[column[joint]] = AT[joint] - turn @ pivot
    posed = blend_rigid(stretched, weights, turns, moves)

    # At most four joints for a vertex, which is what the engine skins with.
    order = np.argsort(-weights, axis=1)[:, :4]
    kept = np.take_along_axis(weights, order, axis=1)
    kept[kept < 0.01] = 0.0
    kept /= kept.sum(axis=1, keepdims=True)

    # The game's joints as bones. A bone that points up with no roll exports
    # with no rotation, which is how the game's joints rest.
    def blender(p):
        return (float(p[0]), float(-p[2]), float(p[1]))

    data = bpy.data.armatures.new(name + "_rig")
    rig = bpy.data.objects.new(name + "_rig", data)
    scene.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    for joint, parent, _ in JOINTS:
        bone = data.edit_bones.new(joint)
        head = blender(AT[joint])
        bone.head = head
        bone.tail = (head[0], head[1], head[2] + 0.05)
        bone.roll = 0.0
        if parent:
            bone.parent = data.edit_bones[parent]
            bone.use_connect = False
    bpy.ops.object.mode_set(mode="OBJECT")

    # The fitted mesh: the same faces, UVs and material, new places and weights.
    fitted = bpy.data.objects.new(name, mesh.copy())
    scene.collection.objects.link(fitted)
    new = fitted.data
    new.name = name
    new.vertices.foreach_set("co", np.array([blender(p) for p in posed]).ravel())
    for attribute in ("custom_normal", "sharp_edge", "sharp_face"):
        if attribute in new.attributes:
            new.attributes.remove(new.attributes[attribute])
    for attribute in list(new.color_attributes):
        new.color_attributes.remove(attribute)
    new.polygons.foreach_set("use_smooth", [True] * len(new.polygons))
    new.update()
    # The copy of the mesh carries Meshy's groups. Replace them with the game's.
    fitted.vertex_groups.clear()
    groups = [fitted.vertex_groups.new(name=joint) for joint in NAMES]
    for vertex in range(count):
        for slot in range(4):
            if kept[vertex, slot] > 0.0:
                groups[int(order[vertex, slot])].add([vertex], float(kept[vertex, slot]), "REPLACE")
    fitted.parent = rig
    fitted.modifiers.new("rig", "ARMATURE").object = rig

    for material in new.materials:
        material.name = name
        material.use_backface_culling = True
        for node in material.node_tree.nodes:
            if node.type == "BSDF_PRINCIPLED":
                node.inputs["Metallic"].default_value = 0.0
                node.inputs["Roughness"].default_value = 0.8
            if node.type == "TEX_IMAGE" and node.image:
                node.image.name = name
                if max(node.image.size) > TEXTURE:
                    node.image.scale(TEXTURE, TEXTURE)
                    node.image.pack()

    for o in list(scene.objects):
        if o not in (rig, fitted):
            bpy.data.objects.remove(o, do_unlink=True)
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    fitted.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=out,
        export_format="GLB",
        use_selection=True,
        use_active_scene=True,
        export_yup=True,
        export_apply=False,
        export_skins=True,
        export_animations=False,
        export_morph=False,
        export_tangents=False,
        export_vertex_color="NONE",
        export_image_format="AUTO",
    )
    low, high = posed.min(axis=0), posed.max(axis=0)
    print(f"  from {np.round(low, 3)} to {np.round(high, 3)}, wrote {out}")
    return fitted
