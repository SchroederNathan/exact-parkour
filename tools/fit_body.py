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

import bmesh
import bpy
import numpy as np
from mathutils.bvhtree import BVHTree

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
# How far the toes point out from straight ahead, radians.
TOE_OUT = math.radians(6.0)
# The heights between which a vertex goes from the foot's to the shin's, metres.
ANKLE = (0.065, 0.15)
# How far the collar bone comes down from the T-pose, radians.
SHRUG = math.radians(8.0)
# How far a body's own shoulders may be nearer the middle and lower than the
# game's joints, metres.
NARROWER = 0.03
LOWER = 0.02
# The knuckles' place along the hand, the turn of a fingertip toward the palm
# (radians), and how far the fingers close toward the middle one.
KNUCKLES = 0.45
CURL = math.radians(100.0)
CLOSE = 0.3
# Where the collar bone hands a vertex to the arm: from this far before the
# shoulder joint to this far past it, along the arm, metres.
SHOULDER = (-0.03, 0.09)
# The height of the jaw's line at the middle of the neck, metres, and its fall
# toward the front for each metre.
JAW = 1.60
JAW_SLOPE = 0.3


# The socks: how high they come up the leg, metres, their colour, the height of
# the darker band at the top, and how far along a foot the toe box starts and
# is complete.
SOCK = 0.21
SOCK_COLOUR = np.array([0.84, 0.83, 0.80])
SOCK_BAND = 0.03
TOE_BOX = (0.52, 0.68)
TOE_ROUND = 6
# How far under the cap the old toes go, metres.
TOE_UNDER = 0.004


def toe_box(points, triangles):
    """The round end of a sock over the toes of one foot.

    `points` are the foot's vertices and `triangles` its faces. Meshy's toes are
    separate lumps with open gaps between them, so no move of their vertices
    makes one surface. The sock's end is a new piece: the hull of the front of
    the foot, made round. Toward the instep it lies down on the foot, and the
    toes go just inside it. Returns the foot's vertices, the cap's vertices and
    the cap's faces.
    """
    heel, tip = points[:, 2].min(), points[:, 2].max()

    def along(z):
        return (z - heel) / (tip - heel)

    front = along(points[:, 2]) > TOE_BOX[0] - 0.08
    hull = bmesh.new()
    for point in points[front]:
        hull.verts.new(point)
    bmesh.ops.convex_hull(hull, input=hull.verts)
    # The hull has a corner at the tip of each toe. More, smaller faces and a few
    # passes of smoothing make it one round end. The sole stays flat on the ground.
    bmesh.ops.triangulate(hull, faces=hull.faces)
    bmesh.ops.subdivide_edges(hull, edges=hull.edges, cuts=1, use_grid_fill=True)
    bmesh.ops.triangulate(hull, faces=hull.faces)
    ground = points[:, 1].min()
    for _ in range(TOE_ROUND):
        bmesh.ops.smooth_vert(hull, verts=hull.verts, factor=0.5, use_axis_x=True, use_axis_y=True, use_axis_z=True)
        for vertex in hull.verts:
            vertex.co.y = max(vertex.co.y, ground)
    bmesh.ops.recalc_face_normals(hull, faces=hull.faces)
    round_end = BVHTree.FromBMesh(hull)
    skin = BVHTree.FromPolygons([p.tolist() for p in points], [list(t) for t in triangles])

    # The toes go under the cap, and the cap lies down on the foot behind them.
    middle = np.array([v.co[:] for v in hull.verts]).mean(axis=0)
    inner = points.copy()
    for index in np.nonzero(front)[0]:
        nearest = np.array(round_end.find_nearest(points[index].tolist())[0])
        under = nearest + TOE_UNDER * unit(middle - nearest)
        inner[index] += smoothstep(TOE_BOX[0], TOE_BOX[1], along(points[index][2])) * (under - points[index])
    hull.verts.ensure_lookup_table()
    cap = []
    for vertex in hull.verts:
        at = np.array(vertex.co[:])
        on_foot = np.array(skin.find_nearest(at.tolist())[0])
        cap.append(on_foot + smoothstep(TOE_BOX[0] - 0.04, TOE_BOX[1], along(at[2])) * (at - on_foot))
    faces = [[v.index for v in face.verts] for face in hull.faces]
    hull.free()
    return inner, np.array(cap), faces


def paint_socks(mesh, image, depth):
    """Paint the texture a sock's colour wherever `depth` is positive.

    `depth` is, for each vertex, how far under the top of the sock it is, metres;
    it is negative above the top and for a vertex that is not on a leg. A texel
    takes the depth of its place in its triangle, so the top is a clean line.
    """
    width, height = image.size
    pixels = np.array(image.pixels[:], dtype=np.float32).reshape(height, width, 4)
    uvs = np.empty(len(mesh.loops) * 2)
    mesh.uv_layers.active.data.foreach_get("uv", uvs)
    uvs = uvs.reshape(-1, 2) * (width, height) - 0.5
    mesh.calc_loop_triangles()
    painted = np.zeros((height, width), dtype=bool)
    shade = pixels[:, :, :3] @ np.array([0.3, 0.59, 0.11])
    target = np.zeros((height, width), dtype=np.float32)
    for triangle in mesh.loop_triangles:
        deep = depth[list(triangle.vertices)]
        if deep.max() <= 0.0:
            continue
        a, b, c = uvs[list(triangle.loops)]
        low = np.floor(np.minimum(np.minimum(a, b), c)).astype(int) - 2
        high = np.ceil(np.maximum(np.maximum(a, b), c)).astype(int) + 2
        xs, ys = np.meshgrid(np.arange(low[0], high[0] + 1), np.arange(low[1], high[1] + 1))
        area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
        if abs(area) < 1e-9:
            continue
        u = ((xs - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (ys - a[1])) / area
        v = ((b[0] - a[0]) * (ys - a[1]) - (xs - a[0]) * (b[1] - a[1])) / area
        # Two texels past the edge, so that no skin shows at a seam of the texture.
        edge = 2.0 / math.sqrt(abs(area))
        inside = (u >= -edge) & (v >= -edge) & (u + v <= 1.0 + edge)
        value = deep[0] + u * (deep[1] - deep[0]) + v * (deep[2] - deep[0])
        hit = inside & (value > 0.0)
        xs, ys = xs[hit] % width, ys[hit] % height
        painted[ys, xs] = True
        target[ys, xs] = value[hit]
    if not painted.any():
        raise ValueError("no texel of the texture is on a sock")
    # The cloth keeps a little of the light and shade that the skin had there.
    light = shade[painted] / max(float(shade[painted].mean()), 1e-6)
    cloth = np.clip(0.94 + 0.06 * light, 0.92, 1.03)
    band = np.where(target[painted] < SOCK_BAND, 0.86, 1.0)
    pixels[painted, :3] = np.clip(SOCK_COLOUR * (cloth * band)[:, None], 0.0, 1.0)
    image.pixels.foreach_set(pixels.ravel())
    image.update()
    image.pack()
    return int(painted.sum())


def smoothstep(low, high, x):
    t = np.clip((x - low) / (high - low), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


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


def claim(blocks, block, name):
    """Give `block` the name, and take it from a block that an earlier run left."""
    other = blocks.get(name)
    if other is not None and other != block:
        other.name = name + ".old"
    block.name = name


def fit(source, out, name):
    scene = bpy.data.scenes.get(name) or bpy.data.scenes.new(name)
    bpy.context.window.scene = scene
    for o in list(scene.objects):
        bpy.data.objects.remove(o, do_unlink=True)
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

    # Meshy's weights, a column for each of its joints.
    parts = [g.name for g in body.vertex_groups]
    part = {n: i for i, n in enumerate(parts)}
    raw = np.zeros((count, len(parts)))
    for v in mesh.vertices:
        for g in v.groups:
            raw[v.index, g.group] += g.weight
    loose = int((raw.sum(axis=1) < 1e-6).sum())
    if loose:
        raise ValueError(f"{loose} vertices have no weight")
    raw /= raw.sum(axis=1, keepdims=True)

    def share(*names):
        return raw[:, [part[n] for n in names]].sum(axis=1)

    # To the game's height, with the soles on the ground.
    scale = HEIGHT / (at[:, 1].max() - at[:, 1].min())
    origin = np.array([0.0, at[:, 1].min(), 0.0])
    at = (at - origin) * scale
    meshy = {n: (p - origin) * scale for n, p in meshy.items()}
    print(f"{name}: scale {scale:.4f}, {count} vertices")

    # Everything from here to the swing happens in the T-pose.
    shaped = at.copy()
    sides = (("Left", "l", 1.0), ("Right", "r", -1.0))
    for meshy_side, side, outward in sides:
        # The legs stand under the game's hips: each leaning leg is made upright,
        # from the hip down, so the ankle is on the game's line. The foot goes
        # with the ankle.
        leg = share(f"{meshy_side}UpLeg", f"{meshy_side}Leg", f"{meshy_side}Foot", f"{meshy_side}ToeBase")
        hip, ankle = meshy[f"{meshy_side}UpLeg"], meshy[f"{meshy_side}Foot"].copy()
        down = np.clip((hip[1] - shaped[:, 1]) / (hip[1] - ankle[1]), 0.0, 1.0)
        lean = AT[f"foot_{side}"][0] - ankle[0]
        shaped[:, 0] += leg * down * lean
        ankle[0] += lean
        # The feet point ahead, with a few degrees of toe-out. The foot turns about
        # the ankle, and the turn fades out up the shin. The foot's line is from
        # the middle of the heel to the middle of the toes.
        sole = shaped[(leg > 0.5) & (shaped[:, 1] < 0.05)][:, [0, 2]]
        long = np.linalg.svd(sole - sole.mean(axis=0), full_matrices=False)[2][0]
        long = long if long[1] > 0 else -long
        far = (sole - sole.mean(axis=0)) @ long
        line = sole[far > np.quantile(far, 0.85)].mean(axis=0) - sole[far < np.quantile(far, 0.15)].mean(axis=0)
        toes_out = outward * math.atan2(line[0], line[1])
        by_height = 1.0 - smoothstep(ANKLE[0], ANKLE[1], shaped[:, 1])
        turn = -outward * (toes_out - TOE_OUT) * by_height * (leg > 0.5)
        x, z = shaped[:, 0] - ankle[0], shaped[:, 2] - ankle[2]
        shaped[:, 0] = ankle[0] + x * np.cos(turn) + z * np.sin(turn)
        shaped[:, 2] = ankle[2] - x * np.sin(turn) + z * np.cos(turn)
        print(
            f"  {side} leg: ankle moved {lean * 100:.1f} cm across, toes were {math.degrees(toes_out):.1f} degrees out"
            f" (long axis {math.degrees(outward * math.atan2(long[0], long[1])):.1f})"
        )

    # The feet where the mannequin's are, front to back, and the body with them.
    feet = share("LeftFoot", "LeftToeBase", "RightFoot", "RightToeBase") > 0.5
    forward = np.array([0.0, 0.0, (shaped[feet, 2].max() + shaped[feet, 2].min()) / 2.0 - FOOT_MIDDLE])
    shaped -= forward
    at = at - forward
    meshy = {n: p - forward for n, p in meshy.items()}

    turns = [np.eye(3) for _ in parts]
    moves = [np.zeros(3) for _ in parts]
    hangs = {}
    for meshy_side, side, outward in sides:
        upper, fore, hand = (f"{limb}_{side}" for limb in ("upper_arm", "forearm", "hand"))
        m_upper, m_fore, m_hand = (f"{meshy_side}{j}" for j in ("Arm", "ForeArm", "Hand"))
        collar, shoulder, elbow, wrist = (meshy[f"{meshy_side}{j}"] for j in ("Shoulder", "Arm", "ForeArm", "Hand"))
        along = [unit(elbow - shoulder), unit(wrist - elbow)]
        length = [np.linalg.norm(AT[fore] - AT[upper]), np.linalg.norm(AT[hand] - AT[fore])]
        by = [length[0] / np.linalg.norm(elbow - shoulder), length[1] / np.linalg.norm(wrist - elbow)]
        print(f"  {side} arm: upper x{by[0]:.3f}, fore x{by[1]:.3f}")
        # Meshy's step from the collar bone to the arm can be sharp, and the swing
        # then leaves a corner on the shoulder. The two share what they had between
        # them by how far along the arm a vertex is. Hair that lies on the shoulder
        # stays with the head.
        m_collar = f"{meshy_side}Shoulder"
        between = raw[:, part[m_collar]] + raw[:, part[m_upper]]
        to_arm = smoothstep(SHOULDER[0], SHOULDER[1], (at - shoulder) @ along[0])
        body_part = share("Head", "neck", "head_end", "headfront") < 0.3
        raw[body_part, part[m_upper]] = (between * to_arm)[body_part]
        raw[body_part, part[m_collar]] = (between * (1.0 - to_arm))[body_part]

        # Each part of the arm longer along its own line, the next part moved out with it.
        new_elbow = shoulder + along[0] * length[0]
        new_wrist = new_elbow + along[1] * length[1]
        moved = [
            (m_upper, lambda p: shoulder + (p - shoulder) @ stretch(along[0], by[0]).T),
            (m_fore, lambda p: new_elbow + (p - elbow) @ stretch(along[1], by[1]).T),
            (m_hand, lambda p: new_wrist + (p - wrist)),
        ]
        shaped += sum(share(joint)[:, None] * (to(at) - at) for joint, to in moved)

        # A hand at rest: the fingers curl toward the palm and close a little. The
        # palm faces down in the T-pose, so the fingers go round a line across the
        # hand, under the knuckles.
        grip = share(m_hand)
        reach = (shaped - new_wrist) @ along[1]
        tip = reach[grip > 0.5].max()
        knuckles = KNUCKLES * tip
        across = unit(np.cross(along[1], np.array([0.0, 1.0, 0.0])))
        back = np.cross(across, along[1])
        fingers = (grip > 0.5) & (reach > knuckles)
        middle = shaped[fingers].mean(axis=0)
        past = np.clip(reach - knuckles, 0.0, None)
        angle = CURL * past / (tip - knuckles)
        radius = (tip - knuckles) / CURL + (shaped - middle) @ back
        curled = (
            shaped
            + np.outer(radius * np.sin(angle) - past, along[1])
            + np.outer(radius * (np.cos(angle) - 1.0), back)
            - np.outer(CLOSE * past / (tip - knuckles) * ((shaped - middle) @ across), across)
        )
        shaped += grip[:, None] * (curled - shaped)

        # The collar bone comes down with the arm, which lowers the top of the
        # shoulder. Then the arm goes down to the rest pose: the upper arm about
        # the shoulder, the forearm and the hand with it and onto the game's line.
        drop = turn_onto(np.array([1.0, 0.0, 0.0]), np.array([math.cos(SHRUG), -outward * math.sin(SHRUG), 0.0]))
        swing = turn_onto(along[0], AT[fore] - AT[upper])
        bend = turn_onto(swing @ along[1], AT[hand] - AT[fore]) @ swing
        turns[part[m_collar]] = drop
        moves[part[m_collar]] = collar - drop @ collar
        # A body keeps its own shoulders when they are a little narrower or lower
        # than the game's: the whole arm hangs that much nearer or lower than its
        # joints. An arm turns mostly about a line across the body, and a joint
        # that is off along that line turns the arm the same.
        own = collar + drop @ (shoulder - collar) - AT[upper]
        own = np.array([outward * np.clip(outward * own[0], -NARROWER, 0.0), np.clip(own[1], -LOWER, 0.0), 0.0])
        print(f"  {side} shoulder: {-outward * own[0] * 100:.1f} cm nearer and {-own[1] * 100:.1f} cm lower than the joint")
        for joint, to, turn, pivot in ((m_upper, upper, swing, shoulder), (m_fore, fore, bend, new_elbow), (m_hand, hand, bend, new_wrist)):
            turns[part[joint]] = turn
            moves[part[joint]] = AT[to] + own - turn @ pivot
        hangs[side] = own

    posed = blend_rigid(shaped, raw, turns, moves)

    # Socks hide the toes, which Meshy makes from too few triangles to look right.
    mesh.calc_loop_triangles()
    corners = np.empty(len(mesh.loop_triangles) * 3, dtype=np.int64)
    mesh.loop_triangles.foreach_get("vertices", corners)
    corners = corners.reshape(-1, 3)
    legs, caps = {}, {}
    for meshy_side, side, outward in sides:
        leg = share(f"{meshy_side}UpLeg", f"{meshy_side}Leg", f"{meshy_side}Foot", f"{meshy_side}ToeBase") > 0.5
        legs[side] = leg
        foot = leg & (posed[:, 1] < ANKLE[0] + 0.03)
        number = np.full(count, -1)
        number[foot] = np.arange(int(foot.sum()))
        triangles = number[corners[foot[corners].all(axis=1)]]
        posed[foot], cap, faces = toe_box(posed[foot], triangles)
        caps[side] = (cap, faces)
    in_sock = np.where(legs["l"] | legs["r"], SOCK - posed[:, 1], -1.0)

    # The game's weights: each Meshy joint's go to the game joint that moves that part.
    column = {n: i for i, n in enumerate(NAMES)}
    weights = np.zeros((count, len(NAMES)))
    for joint, index in part.items():
        weights[:, column[FROM_MESHY[joint]]] += raw[:, index]

    # Then each hinge is put where the game's joint is. Two neighbours share what
    # they had between them by how far a vertex is past the joint.
    def hinge(before, after, past, width):
        total = weights[:, column[before]] + weights[:, column[after]]
        to_after = smoothstep(-width, width, past)
        weights[:, column[after]] = total * to_after
        weights[:, column[before]] = total * (1.0 - to_after)

    for side in ("l", "r"):
        thigh, shin, foot = (f"{limb}_{side}" for limb in ("thigh", "shin", "foot"))
        upper, fore, hand = (f"{limb}_{side}" for limb in ("upper_arm", "forearm", "hand"))
        hinge(thigh, shin, (posed - AT[shin]) @ unit(AT[shin] - AT[thigh]), 0.06)
        # All of the foot follows the foot, the heel too. It gives way to the shin
        # over the ankle.
        hinge(shin, foot, (ANKLE[0] + ANKLE[1]) / 2.0 - posed[:, 1], (ANKLE[1] - ANKLE[0]) / 2.0)
        hinge(upper, fore, (posed - AT[fore] - hangs[side]) @ unit(AT[fore] - AT[upper]), 0.05)
        hinge(fore, hand, (posed - AT[hand] - hangs[side]) @ unit(AT[hand] - AT[fore]), 0.03)
    # Meshy gives the whole neck to the head. The head is what is above the line
    # of the jaw, which is lower at the chin than at the back of the skull.
    jaw = unit(np.array([0.0, 1.0, JAW_SLOPE]))
    hinge("neck", "head", (posed - np.array([0.0, JAW, 0.0])) @ jaw, 0.025)

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
    claim(bpy.data.armatures, data, name + "_rig")
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
    claim(bpy.data.meshes, new, name)
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
    # The caps of the socks: each follows its foot, and all of a cap takes the
    # colour of one texel of the sock.
    sock_at = int(np.argmax(np.where((in_sock > SOCK_BAND + 0.02) & (in_sock < SOCK - 0.08), posed[:, 2], -np.inf)))
    whole = bmesh.new()
    whole.from_mesh(new)
    uv = whole.loops.layers.uv.verify()
    follow = whole.verts.layers.deform.verify()
    whole.verts.ensure_lookup_table()
    colour = whole.verts[sock_at].link_loops[0][uv].uv.copy()
    for side, (cap, faces) in caps.items():
        made = []
        for at in cap:
            vertex = whole.verts.new(blender(at))
            vertex[follow][groups[column[f"foot_{side}"]].index] = 1.0
            made.append(vertex)
        for face in faces:
            added = whole.faces.new([made[i] for i in face])
            added.smooth = True
            added.material_index = 0
            for loop in added.loops:
                loop[uv].uv = colour
    whole.to_mesh(new)
    whole.free()
    new.update()

    fitted.parent = rig
    fitted.modifiers.new("rig", "ARMATURE").object = rig

    for material in new.materials:
        claim(bpy.data.materials, material, name)
        material.use_backface_culling = True
        for node in material.node_tree.nodes:
            if node.type == "BSDF_PRINCIPLED":
                node.inputs["Metallic"].default_value = 0.0
                node.inputs["Roughness"].default_value = 0.8
            if node.type == "TEX_IMAGE" and node.image:
                claim(bpy.data.images, node.image, name)
                if max(node.image.size) > TEXTURE:
                    node.image.scale(TEXTURE, TEXTURE)
                    node.image.pack()
                # The caps have no texels of their own.
                depth = np.concatenate([in_sock, np.full(len(new.vertices) - count, -1.0)])
                texels = paint_socks(new, node.image, depth)
                print(f"  socks: {texels} texels")

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
