#!/usr/bin/env python3
"""Draw a film strip of the body from the lines the `film` test writes.

    python3 tools/film.py film.jsonl out.png [first] [last] [every] [columns]

Each frame is a side view (the wall's face is the vertical line, its top the
horizontal one) over a view from behind. The left limbs are blue, the right red.
"""
import json, struct, sys, zlib

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
    def __init__(self, width, height):
        self.w, self.h = width, height
        self.px = bytearray([255]) * (width * height * 3)
    def dot(self, x, y, r, color):
        for yy in range(int(y - r), int(y + r) + 1):
            for xx in range(int(x - r), int(x + r) + 1):
                if 0 <= xx < self.w and 0 <= yy < self.h and (xx - x) ** 2 + (yy - y) ** 2 <= r * r:
                    i = (yy * self.w + xx) * 3
                    self.px[i:i + 3] = bytes(color)
    def line(self, a, b, r, color):
        steps = int(max(abs(b[0] - a[0]), abs(b[1] - a[1]))) + 1
        for s in range(steps + 1):
            k = s / steps
            self.dot(a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, r, color)

FONT = {
    "0": "111101101101111", "1": "010110010010111", "2": "111001111100111",
    "3": "111001111001111", "4": "101101111001001", "5": "111100111001111",
    "6": "111100111101111", "7": "111001001001001", "8": "111101111101111",
    "9": "111101111001111", ".": "000000000000010", "-": "000000111000000",
    " ": "000000000000000",
}
LETTERS = {
    "a": "010101111101101", "c": "111100100100111", "g": "111100101101111",
    "h": "101101111101101", "v": "101101101101010", "r": "110101110101101",
    "s": "111100111001111", "l": "100100100100111", "i": "111010010010111",
    "d": "110101101101110", "n": "111101101101101", "o": "111101101101111",
    "u": "101101101101111", "t": "111010010010010", "m": "101111111101101",
    "b": "110101110101110", "e": "111100111100111", "k": "101101110101101",
}
FONT.update(LETTERS)

def text(canvas, x, y, words, color=(60, 60, 60), size=2):
    for ch in words:
        bits = FONT.get(ch, FONT[" "])
        for i, bit in enumerate(bits):
            if bit == "1":
                cx, cy = x + (i % 3) * size, y + (i // 3) * size
                for dy in range(size):
                    for dx in range(size):
                        canvas.dot(cx + dx, cy + dy, 0.5, color)
        x += 4 * size

BONES = [
    ("hips", "spine", 0), ("spine", "chest", 0), ("chest", "neck", 0), ("neck", "head", 0),
    ("thigh_r", "shin_r", 2), ("shin_r", "foot_r", 2), ("foot_r", "toe_r", 2),
    ("upper_arm_r", "forearm_r", 2), ("forearm_r", "hand_r", 2),
    ("hips", "thigh_r", 2), ("chest", "upper_arm_r", 2),
    ("thigh_l", "shin_l", 1), ("shin_l", "foot_l", 1), ("foot_l", "toe_l", 1),
    ("upper_arm_l", "forearm_l", 1), ("forearm_l", "hand_l", 1),
    ("hips", "thigh_l", 1), ("chest", "upper_arm_l", 1),
]
COLORS = [(40, 40, 40), (30, 90, 200), (210, 50, 40)]

def main():
    src, out = sys.argv[1], sys.argv[2]
    first = int(sys.argv[3]) if len(sys.argv) > 3 else 0
    last = int(sys.argv[4]) if len(sys.argv) > 4 else 10 ** 9
    every = int(sys.argv[5]) if len(sys.argv) > 5 else 6
    columns = int(sys.argv[6]) if len(sys.argv) > 6 else 10
    lines = [json.loads(l) for l in open(src)]
    head, frames = lines[0], [f for f in lines[1:] if first <= f["t"] <= last and (f["t"] - first) % every == 0]
    top = head["top"]
    scale = 95.0
    cell_w, side_h, back_h = 250, 330, 250
    cell_h = side_h + back_h
    rows = (len(frames) + columns - 1) // columns
    canvas = Canvas(cell_w * columns, cell_h * rows)
    for n, f in enumerate(frames):
        ox, oy = (n % columns) * cell_w, (n // columns) * cell_h
        hips = f["j"]["hips"]
        # The side view follows the hips along the ground and keeps the height.
        base = min(hips[1] - 1.0, top - 2.2) if head["scene"] != "jumps" else -0.1
        base = max(base, hips[1] - 2.6)
        def side(p):
            return (ox + cell_w / 2 + (p[0] - hips[0]) * scale, oy + side_h - 20 - (p[1] - base) * scale)
        def back(p):
            return (ox + cell_w / 2 - (p[2] - hips[2]) * scale, oy + cell_h - 14 - (p[1] - base) * scale)
        grey = (190, 190, 190)
        canvas.line((ox, oy), (ox + cell_w - 1, oy), 0.5, (225, 225, 225))
        canvas.line((ox, oy), (ox, oy + cell_h - 1), 0.5, (225, 225, 225))
        # Ground, and the wall: its face at zero ahead, its top.
        g = side((hips[0], 0.0))
        if oy < g[1] < oy + side_h:
            canvas.line((ox + 2, g[1]), (ox + cell_w - 2, g[1]), 0.6, grey)
        if head["scene"] != "jumps":
            face = side((0.0, top))
            if ox < face[0] < ox + cell_w:
                canvas.line(face, (face[0], min(oy + side_h, side((0, 0))[1])), 1.0, (120, 150, 120))
                canvas.line(face, (ox + cell_w - 2, face[1]), 1.0, (120, 150, 120))
            edge = back((0, top, hips[2]))
            if oy + side_h < edge[1] < oy + cell_h:
                canvas.line((ox + 2, edge[1]), (ox + cell_w - 2, edge[1]), 0.6, (120, 150, 120))
        gb = back((0, 0.0, hips[2]))
        if oy + side_h < gb[1] < oy + cell_h:
            canvas.line((ox + 2, gb[1]), (ox + cell_w - 2, gb[1]), 0.6, grey)
        for view in (side, back):
            for order in (2, 0, 1) if view is side else (0, 1, 2):
                for a, b, which in BONES:
                    if which == order:
                        canvas.line(view(f["j"][a]), view(f["j"][b]), 2.0 if which == 0 else 1.6, COLORS[which])
            head_at = view(f["j"]["head"])
            canvas.dot(head_at[0], head_at[1], 7, (40, 40, 40))
            for hand, which in (("hand_l", 1), ("hand_r", 2)):
                p = view(f["j"][hand])
                canvas.dot(p[0], p[1], 3, COLORS[which])
        text(canvas, ox + 5, oy + 5, "%d %s %.2f" % (f["t"], f["act"], f["u"]))
    png(out, canvas.w, canvas.h, canvas.px)
    print(out, canvas.w, canvas.h, len(frames), "frames")

main()
