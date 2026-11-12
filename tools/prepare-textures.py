# Turn the generated surface images into the tiling textures under art/.
#
#   uv run --with pillow --with numpy python tools/prepare-textures.py <image dir> art
#
# The image dir holds roof, concrete, brick, tile, asphalt, plate, corrugated,
# windows, tower, sky and signs (.png), as Meshy's text-to-image made them.
#
# art/textures/<name>.png      base colour (sRGB, repeats)
# art/textures/<name>-glow.png what the surface emits (windows, tower)
# art/data/<name>-n.png        normal map, made from the base colour's brightness
# art/data/<name>-mr.png       roughness in green, metallic in blue: the puddles
import sys, os
import numpy as np
from PIL import Image, ImageFilter

src, art = sys.argv[1], sys.argv[2]
tex, data = os.path.join(art, "textures"), os.path.join(art, "data")
os.makedirs(tex, exist_ok=True)
os.makedirs(data, exist_ok=True)


def load(name, size=1024):
    im = Image.open(os.path.join(src, name + ".png")).convert("RGB")
    return np.asarray(im.resize((size, size), Image.LANCZOS), dtype=np.float32) / 255.0


def save(a, path, size=None):
    im = Image.fromarray((np.clip(a, 0, 1) * 255 + 0.5).astype(np.uint8))
    if size:
        im = im.resize((size, size), Image.LANCZOS)
    im.save(path, optimize=True)


# Normal, roughness and glow maps are half the size of the base colour: the bake
# writes each image three times, and the web build has a limit on the total.
SMALL = 512


def seamless(a):
    """Blend the image with itself moved by half its size, so both edges meet."""
    h, w = a.shape[:2]
    moved = np.roll(a, (h // 2, w // 2), axis=(0, 1))
    y = np.abs(np.linspace(-1, 1, h))[:, None]
    x = np.abs(np.linspace(-1, 1, w))[None, :]
    # One in the middle, zero on the edges, with a narrow change.
    k = np.clip((1 - np.maximum(x, y)) * 6, 0, 1)[..., None]
    k = k * k * (3 - 2 * k)
    return a * k + moved * (1 - k)


def noise(size, cells, seed):
    """Smooth periodic noise in 0..1."""
    rng = np.random.default_rng(seed)
    total = np.zeros((size, size), dtype=np.float32)
    amp, norm = 1.0, 0.0
    while cells <= size // 4:
        grid = rng.random((cells, cells)).astype(np.float32)
        img = Image.fromarray((grid * 255).astype(np.uint8)).resize((size * 3, size * 3), Image.BICUBIC)
        # Tile before resizing so the result repeats; take the middle copy.
        big = np.tile(grid, (3, 3))
        img = Image.fromarray((big * 255).astype(np.uint8)).resize((size * 3, size * 3), Image.BICUBIC)
        total += amp * np.asarray(img, dtype=np.float32)[size:2 * size, size:2 * size] / 255.0
        norm += amp
        amp *= 0.5
        cells *= 2
    return total / norm


def normal_map(a, strength):
    height = a.mean(axis=2)
    height = np.asarray(Image.fromarray((height * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.2)), dtype=np.float32) / 255.0
    dx = (np.roll(height, -1, 1) - np.roll(height, 1, 1)) * strength
    dy = (np.roll(height, -1, 0) - np.roll(height, 1, 0)) * strength
    n = np.stack([-dx, dy, np.ones_like(dx)], axis=2)
    n /= np.linalg.norm(n, axis=2, keepdims=True)
    return n * 0.5 + 0.5


def puddles(a, name, cover, seed, dry_rough, darken=0.55):
    """Dark, smooth pools where low noise falls under `cover`."""
    size = a.shape[0]
    n = noise(size, 4, seed)
    wet = np.clip((cover - n) / 0.05, 0, 1)
    wet = wet * wet * (3 - 2 * wet)
    out = a * (1 - wet[..., None] * (1 - darken))
    rough = dry_rough * (1 - wet) + 0.04 * wet
    mr = np.stack([np.ones_like(rough), rough, np.zeros_like(rough)], axis=2)
    save(mr, os.path.join(data, name + "-mr.png"), SMALL)
    return out, wet


# Surfaces. `blend` is for images with no pattern that must line up.
for name, blend, bump in [
    ("roof", False, 5.0), ("concrete", True, 4.0), ("brick", False, 6.0), ("tile", False, 5.0),
    ("asphalt", True, 7.0), ("plate", False, 8.0), ("corrugated", False, 6.0),
]:
    a = load(name)
    if blend:
        a = seamless(a)
    n = normal_map(a, bump)
    if name == "asphalt":
        a, wet = puddles(a * 0.8, name, 0.47, 7, 0.7)
        n = n * (1 - wet[..., None]) + np.array([0.5, 0.5, 1.0]) * wet[..., None]
    if name == "roof":
        a, wet = puddles(a * 0.85, name, 0.42, 11, 0.62)
        n = n * (1 - wet[..., None]) + np.array([0.5, 0.5, 1.0]) * wet[..., None]
    save(a, os.path.join(tex, name + ".png"))
    save(n, os.path.join(data, name + "-n.png"), SMALL)

# Lit windows: the base colour stays, and the bright panes also emit.
for name, rule in [("windows", "warm"), ("tower", "any")]:
    a = load(name)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    if rule == "warm":
        mask = (r > 0.62) & (g > 0.45) & (b < 0.85 * r)
    else:
        mask = a.max(axis=2) > 0.55
    soft = np.asarray(Image.fromarray((mask * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.5)), dtype=np.float32) / 255.0
    save(a * (0.55 if name == "windows" else 0.8), os.path.join(tex, name + ".png"))
    save(a * soft[..., None], os.path.join(tex, name + "-glow.png"), SMALL)
    save(normal_map(a, 3.0), os.path.join(data, name + "-n.png"), SMALL)

# The sky: an equirectangular map, twice as wide as tall, with its left and
# right edges blended into each other.
sky = Image.open(os.path.join(src, "sky.png")).convert("RGB").resize((2048, 1024), Image.LANCZOS)
s = np.asarray(sky, dtype=np.float32) / 255.0
m = 160
ramp = np.linspace(0, 1, m)[None, :, None]
left, right = s[:, :m].copy(), s[:, -m:].copy()
mirror = right[:, ::-1]
s[:, :m] = mirror * (1 - ramp) * 0.5 + left * (0.5 + 0.5 * ramp)
s[:, -m:] = right * (1 - 0.5 * ramp) + left[:, ::-1] * 0.5 * ramp
save(s, os.path.join(tex, "sky.png"))

# Four neon signs from one sheet, on black.
sheet = Image.open(os.path.join(src, "signs.png")).convert("RGB")
w, h = sheet.size
for name, box in [
    ("sign-sake", (0, 0, w // 2, h // 2)), ("sign-hotel", (w // 2, 0, w, h // 2)),
    ("sign-ramen", (0, h // 2, w // 2, h)), ("sign-pharmacy", (w // 2, h // 2, w, h)),
]:
    part = np.asarray(sheet.crop(box).resize((512, 512), Image.LANCZOS), dtype=np.float32) / 255.0
    # Push the dim haze around the tubes to black so only the tubes glow.
    part = np.clip((part - 0.06) / 0.94, 0, 1)
    save(part, os.path.join(tex, name + ".png"))

# A cloud puff for the particle clouds: white, with a soft ragged alpha.
size = 256
yy, xx = np.mgrid[0:size, 0:size].astype(np.float32)
d = np.sqrt(((xx - size / 2) / (size / 2)) ** 2 + ((yy - size / 2) / (size / 2)) ** 2)
alpha = np.clip(1.15 - d, 0, 1) ** 1.5 * (0.35 + 0.65 * noise(size, 4, 3))
alpha = np.clip(alpha * 1.6, 0, 1) * np.clip((1 - d) * 4, 0, 1)
puff = np.dstack([np.ones((size, size, 3), dtype=np.float32), alpha])
Image.fromarray((puff * 255 + 0.5).astype(np.uint8), "RGBA").save(os.path.join(tex, "cloud.png"))
print("textures:", sorted(os.listdir(tex)))
print("data:", sorted(os.listdir(data)))
