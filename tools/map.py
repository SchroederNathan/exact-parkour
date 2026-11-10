#!/usr/bin/env python3
"""Draw a race level from the file the `dump` test writes: a plan and a view.

    LEVEL=0 OUT=/tmp/level.json cargo test --manifest-path .shells/Cargo.toml \\
        -p exact-beat-em-up-logic --test race dump -- --ignored
    python3 tools/map.py /tmp/level.json /tmp/level [trail.txt] [yaw] [x0 z0 x1 z1]

It writes `/tmp/level-plan.png`, the level from above with north (-Z) up, and
`/tmp/level-view.png`, the level from the south-east and above. `yaw` turns the
view, degrees (default 35). The four numbers crop both pictures to that box on the
map. `trail.txt` holds the runner's path, one `x y z` per line, which the route
test writes when `TRAIL` names a file; pass `-` for none.

On the plan a number in a piece is the height of its top, a ring with a number is
a checkpoint (0 is the start) and the ring with F is the finish. Scenery is left
out.
"""
import json, math, struct, sys, zlib

LOOKS = {
    "Concrete": (143, 152, 156), "Slate": (89, 100, 109), "Brick": (150, 112, 92),
    "Glass": (95, 130, 150), "Trim": (69, 77, 85), "Vault": (217, 130, 43),
    "Climb": (47, 156, 149), "Beam": (227, 179, 65), "Ramp": (179, 161, 122),
    "Danger": (208, 71, 63), "Accent": (230, 80, 200), "Haze": (190, 200, 208),
}
FONT = {
    "0": "111101101101111", "1": "010110010010111", "2": "111001111100111",
    "3": "111001111001111", "4": "101101111001001", "5": "111100111001111",
    "6": "111100111101111", "7": "111001001001001", "8": "111101111101111",
    "9": "111101111001111", ".": "000000000000010", "-": "000000111000000",
    " ": "000000000000000", "F": "111100111100100", "N": "101111111111101",
    "x": "000101010101000", "z": "000111001100111",
}

def png(path, width, height, pixels):
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        raw += pixels[y * width * 3:(y + 1) * width * 3]
    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 6)))
        f.write(chunk(b"IEND", b""))

class Canvas:
    def __init__(self, width, height, back=(236, 240, 243)):
        self.w, self.h = width, height
        self.px = bytearray(bytes(back) * (width * height))
        self.depth = None
    def set(self, x, y, color):
        if 0 <= x < self.w and 0 <= y < self.h:
            i = (y * self.w + x) * 3
            self.px[i:i + 3] = bytes(color)
    def dot(self, x, y, r, color):
        for yy in range(int(y - r), int(y + r) + 1):
            for xx in range(int(x - r), int(x + r) + 1):
                if (xx - x) ** 2 + (yy - y) ** 2 <= r * r:
                    self.set(xx, yy, color)
    def ring(self, x, y, r, color, thick=1.5):
        for yy in range(int(y - r - thick), int(y + r + thick) + 1):
            for xx in range(int(x - r - thick), int(x + r + thick) + 1):
                if abs(math.hypot(xx - x, yy - y) - r) <= thick:
                    self.set(xx, yy, color)
    def line(self, a, b, color, r=0.6):
        steps = int(max(abs(b[0] - a[0]), abs(b[1] - a[1]))) + 1
        for s in range(steps + 1):
            k = s / steps
            self.dot(a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, r, color)
    def text(self, x, y, words, color=(20, 24, 28), size=2):
        x = int(x)
        y = int(y)
        for ch in words:
            bits = FONT.get(ch, FONT[" "])
            for i, bit in enumerate(bits):
                if bit == "1":
                    for dy in range(size):
                        for dx in range(size):
                            self.set(x + (i % 3) * size + dx, y + (i // 3) * size + dy, color)
            x += 4 * size
    def polygon(self, points, color, depths=None):
        """Fill a convex polygon. With `depths`, one per point, a pixel is drawn
        only where it is nearer than what is there (larger is nearer)."""
        ys = [p[1] for p in points]
        top, bottom = max(int(math.ceil(min(ys))), 0), min(int(math.floor(max(ys))), self.h - 1)
        n = len(points)
        color = bytes(color)
        for y in range(top, bottom + 1):
            hits = []
            for i in range(n):
                (x0, y0), (x1, y1) = points[i], points[(i + 1) % n]
                if (y0 <= y < y1) or (y1 <= y < y0):
                    k = (y - y0) / (y1 - y0)
                    d = depths[i] + (depths[(i + 1) % n] - depths[i]) * k if depths else 0.0
                    hits.append((x0 + (x1 - x0) * k, d))
            if len(hits) < 2:
                continue
            hits.sort()
            (xa, da), (xb, db) = hits[0], hits[-1]
            a, b = max(int(math.ceil(xa)), 0), min(int(math.floor(xb)), self.w - 1)
            if b < a:
                continue
            if depths is None:
                i = (y * self.w + a) * 3
                self.px[i:i + (b - a + 1) * 3] = color * (b - a + 1)
                continue
            span = (xb - xa) or 1.0
            row = y * self.w
            for x in range(a, b + 1):
                d = da + (db - da) * (x - xa) / span
                if d > self.depth[row + x]:
                    self.depth[row + x] = d
                    self.px[(row + x) * 3:(row + x) * 3 + 3] = color
    def save(self, path):
        png(path, self.w, self.h, self.px)

def corners(piece):
    """The eight corners of a piece in the world."""
    yaw, pitch = math.radians(piece["yaw"]), math.radians(piece["pitch"])
    hx, hy, hz = (s / 2 for s in piece["size"])
    out = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            for sz in (-1, 1):
                x, y, z = sx * hx, sy * hy, sz * hz
                # About the box's own X, then about Y.
                y, z = y * math.cos(pitch) - z * math.sin(pitch), y * math.sin(pitch) + z * math.cos(pitch)
                x, z = x * math.cos(yaw) + z * math.sin(yaw), -x * math.sin(yaw) + z * math.cos(yaw)
                out.append((piece["at"][0] + x, piece["at"][1] + y, piece["at"][2] + z))
    return out

# Corner indices of the six faces, and the sign and axis of each face's normal.
FACES = [((0, 1, 3, 2), 0, -1), ((4, 6, 7, 5), 0, 1), ((0, 4, 5, 1), 1, -1),
         ((2, 3, 7, 6), 1, 1), ((0, 2, 6, 4), 2, -1), ((1, 5, 7, 3), 2, 1)]

def shade(color, k):
    return tuple(max(0, min(255, int(c * k))) for c in color)

def drawn(piece):
    # The far city and the unseen ramp under a stair are left out.
    return piece["kind"] != "Unseen" and piece["look"] != "Haze"

def plan(level, trail, crop, path):
    solid = [p for p in level["pieces"] if drawn(p)]
    if crop:
        x0, z0, x1, z1 = crop
    else:
        xs = [c[0] for p in solid for c in corners(p)]
        zs = [c[2] for p in solid for c in corners(p)]
        x0, z0, x1, z1 = min(xs) - 4, min(zs) - 4, max(xs) + 4, max(zs) + 4
    scale = min(1500 / (x1 - x0), 1500 / (z1 - z0), 40)
    canvas = Canvas(int((x1 - x0) * scale) + 60, int((z1 - z0) * scale) + 60)
    at = lambda x, z: (30 + (x - x0) * scale, 30 + (z - z0) * scale)
    step = 10 if scale < 14 else 5
    for g in range(int(math.floor(x0 / step)) * step, int(x1) + 1, step):
        canvas.line(at(g, z0), at(g, z1), (212, 218, 223), 0.4)
        canvas.text(at(g, z0)[0] + 2, 4, str(g), (120, 128, 135))
    for g in range(int(math.floor(z0 / step)) * step, int(z1) + 1, step):
        canvas.line(at(x0, g), at(x1, g), (212, 218, 223), 0.4)
        canvas.text(2, at(x0, g)[1] + 2, str(g), (120, 128, 135))
    tops = lambda p: max(c[1] for c in corners(p))
    highest = max(tops(p) for p in solid)
    lowest = min(tops(p) for p in solid)
    for p in sorted(solid, key=tops):
        cs = corners(p)
        outline = [at(cs[i][0], cs[i][2]) for i in (2, 3, 7, 6)]
        # A higher piece is lighter, so the heights read at a glance.
        k = 0.7 + 0.45 * (tops(p) - lowest) / max(highest - lowest, 1)
        canvas.polygon(outline, shade(LOOKS[p["look"]], k))
        for i in range(4):
            canvas.line(outline[i], outline[(i + 1) % 4], (40, 46, 52), 0.5)
        if p["size"][0] * scale > 34 and p["size"][2] * scale > 16:
            label = ("%.1f" % tops(p)).rstrip("0").rstrip(".")
            cx, cy = at(p["at"][0], p["at"][2])
            canvas.text(cx - len(label) * 4, cy - 5, label)
    if trail:
        for a, b in zip(trail, trail[1:]):
            if math.dist(a, b) < 3:
                canvas.line(at(a[0], a[2]), at(b[0], b[2]), (200, 30, 160), 1.0)
    for i, c in enumerate(level["checkpoints"]):
        x, y = at(c["at"][0], c["at"][2])
        canvas.ring(x, y, 9, (20, 20, 20), 1.6)
        canvas.text(x - 3, y - 5, str(i), (20, 20, 20))
    x, y = at(level["finish"][0], level["finish"][2])
    canvas.ring(x, y, 11, (200, 30, 160), 2.2)
    canvas.text(x - 3, y - 5, "F", (200, 30, 160))
    canvas.text(canvas.w - 30, 6, "N", (20, 20, 20), 3)
    canvas.save(path)

def view(level, trail, crop, yaw, path):
    pieces = [p for p in level["pieces"] if drawn(p)]
    if crop:
        x0, z0, x1, z1 = crop
        pieces = [p for p in pieces if x0 - 8 < p["at"][0] < x1 + 8 and z0 - 8 < p["at"][2] < z1 + 8]
    yaw, tilt = math.radians(yaw), math.radians(38)
    def project(point):
        x, y, z = point
        # Turn the world so the eye is in the south-east, then look down.
        rx = x * math.cos(yaw) - z * math.sin(yaw)
        rz = x * math.sin(yaw) + z * math.cos(yaw)
        return (rx, rz * math.sin(tilt) - y * math.cos(tilt), rz * math.cos(tilt) + y * math.sin(tilt))
    # Only what is above the fall line matters: cut the buildings off below it.
    floor = min(c["floor"] for c in level["checkpoints"]) - 4
    shown = []
    for p in pieces:
        cs = corners(p)
        cs = [(c[0], max(c[1], floor), c[2]) for c in cs]
        shown.append((p, cs))
    flat = [project(c) for _, cs in shown for c in cs]
    left, right = min(f[0] for f in flat), max(f[0] for f in flat)
    top, bottom = min(f[1] for f in flat), max(f[1] for f in flat)
    scale = min(1700 / (right - left), 1100 / (bottom - top), 40)
    canvas = Canvas(int((right - left) * scale) + 40, int((bottom - top) * scale) + 40)
    canvas.depth = [-1e9] * (canvas.w * canvas.h)
    screen = lambda f: (20 + (f[0] - left) * scale, 20 + (f[1] - top) * scale)
    light = (0.35, 0.8, 0.5)
    for p, cs in shown:
        flat = [project(c) for c in cs]
        for (idx, axis, sign) in FACES:
            quad = [cs[i] for i in idx]
            # The face's normal, from two of its edges.
            a = [quad[1][k] - quad[0][k] for k in range(3)]
            b = [quad[3][k] - quad[0][k] for k in range(3)]
            n = (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])
            length = math.sqrt(sum(v * v for v in n)) or 1.0
            n = tuple(v / length for v in n)
            centre = [sum(c[k] for c in quad) / 4 for k in range(3)]
            out = [centre[k] - p["at"][k] for k in range(3)]
            if sum(n[k] * out[k] for k in range(3)) < 0:
                n = tuple(-v for v in n)
            k = 0.55 + 0.5 * max(0.0, sum(n[i] * light[i] for i in range(3)))
            points = [screen(flat[i]) for i in idx]
            canvas.polygon(points, shade(LOOKS[p["look"]], k), [flat[i][2] for i in idx])
    for a, b in zip(trail or [], (trail or [])[1:]):
        if math.dist(a, b) < 3:
            pa, pb = project((a[0], a[1] + 0.2, a[2])), project((b[0], b[1] + 0.2, b[2]))
            canvas.line(screen(pa), screen(pb), (200, 30, 160), 1.0)
    for i, c in enumerate(level["checkpoints"]):
        x, y = screen(project((c["at"][0], c["at"][1] + 2.5, c["at"][2])))
        canvas.dot(x, y, 9, (255, 255, 255))
        canvas.ring(x, y, 9, (20, 20, 20), 1.4)
        canvas.text(x - 3, y - 5, str(i))
    x, y = screen(project((level["finish"][0], level["finish"][1] + 2.5, level["finish"][2])))
    canvas.dot(x, y, 11, (255, 255, 255))
    canvas.ring(x, y, 11, (200, 30, 160), 2.2)
    canvas.text(x - 3, y - 5, "F", (200, 30, 160))
    canvas.save(path)

def main():
    args = sys.argv[1:]
    if len(args) < 2:
        sys.exit(__doc__)
    level = json.load(open(args[0]))
    trail = None
    if len(args) > 2 and args[2] != "-":
        trail = [tuple(float(v) for v in line.split()) for line in open(args[2]) if line.strip()]
    yaw = float(args[3]) if len(args) > 3 else 35.0
    crop = [float(v) for v in args[4:8]] if len(args) >= 8 else None
    plan(level, trail, crop, args[1] + "-plan.png")
    view(level, trail, crop, yaw, args[1] + "-view.png")
    print(args[1] + "-plan.png", args[1] + "-view.png")

main()
