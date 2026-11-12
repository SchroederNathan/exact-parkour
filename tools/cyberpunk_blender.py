"""Headless cyberpunk art generator for Rooftop Rumble.

Run with Blender 3.6+ or 4.x:
    blender --background --python tools/cyberpunk_blender.py -- --output art/cyberpunk_city.glb

The generated GLB is intentionally a small, modular skyline diorama. The game can
use the baked model directly, or the script can be used as a reference when tuning
procedural Rust props. No external Python packages are required.
"""
from __future__ import annotations

import argparse
import math
import os
from pathlib import Path

import bpy  # type: ignore
from mathutils import Vector  # type: ignore

ROOT = Path(__file__).resolve().parents[1]


PALETTE = {
    "ink": (0.008, 0.012, 0.025, 1.0),
    "steel": (0.035, 0.065, 0.11, 1.0),
    "violet": (0.23, 0.035, 0.42, 1.0),
    "cyan": (0.0, 0.72, 1.0, 1.0),
    "magenta": (1.0, 0.035, 0.42, 1.0),
    "amber": (1.0, 0.28, 0.035, 1.0),
    "white": (0.72, 0.92, 1.0, 1.0),
}


def material(name: str, color, *, metallic=0.0, roughness=0.45, emission=None, strength=0.0):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    bsdf = m.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    if emission is not None:
        socket = bsdf.inputs.get("Emission Color") or bsdf.inputs.get("Emission")
        if socket:
            socket.default_value = emission
        power = bsdf.inputs.get("Emission Strength")
        if power:
            power.default_value = strength
    return m


def cube(name: str, location, scale, mat, bevel=0.0):
    bpy.ops.mesh.primitive_cube_add(location=location)
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel:
        mod = obj.modifiers.new("soft edges", "BEVEL")
        mod.width = bevel
        mod.segments = 2
    obj.data.materials.append(mat)
    return obj


def cylinder(name: str, location, radius, depth, mat, vertices=16, rotation=None):
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=depth, location=location)
    obj = bpy.context.object
    obj.name = name
    if rotation:
        obj.rotation_euler = rotation
    obj.data.materials.append(mat)
    return obj


def neon_strip(name, location, size, mat, rotation=None):
    obj = cube(name, location, size, mat, bevel=min(size) * 0.15)
    if rotation:
        obj.rotation_euler = rotation
    return obj


def make_neon_texture(path: Path):
    """Create a tiny dependency-free atlas with window and circuit motifs."""
    image = bpy.data.images.new("neon_grid", width=256, height=256, alpha=True)
    pixels = [0.0] * (256 * 256 * 4)
    for y in range(256):
        for x in range(256):
            band = (x // 16 + y // 32) % 4
            edge = (x % 16 == 0 or y % 32 == 0)
            if edge:
                rgb = (0.03, 0.5, 0.9) if band % 2 else (0.75, 0.02, 0.42)
                a = 1.0
            else:
                rgb = (0.005, 0.008, 0.02)
                a = 1.0
            i = (y * 256 + x) * 4
            pixels[i:i + 4] = [*rgb, a]
    image.pixels = pixels
    image.filepath_raw = str(path)
    image.file_format = "PNG"
    image.save()
    return image


def texture_material(name: str, image, emission_color):
    m = material(name, PALETTE["ink"], metallic=0.2, roughness=0.35, emission=emission_color, strength=2.4)
    nodes = m.node_tree.nodes
    links = m.node_tree.links
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = image
    tex.interpolation = "Closest"
    bsdf = nodes.get("Principled BSDF")
    links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    links.new(tex.outputs["Color"], bsdf.inputs.get("Emission Color") or bsdf.inputs["Emission"])
    return m


def make_building(index, x, y, z, width, depth, height, mats):
    body = cube(f"tower_{index:02d}", (x, y + height / 2, z), (width, height, depth), mats["steel"], bevel=0.14)
    # Layered rooftop machinery gives each tower a readable silhouette.
    cube(f"tower_{index:02d}_roof", (x, y + height + 0.14, z), (width * 0.86, 0.28, depth * 0.84), mats["ink"], bevel=0.06)
    for side in (-1, 1):
        neon_strip(f"tower_{index:02d}_edge_{side}", (x + side * width * 0.5, y + height * 0.52, z - depth * 0.505), (0.06, height * 0.86, 0.04), mats["cyan"] if index % 2 else mats["magenta"])
    rows = max(2, int(height / 1.8))
    cols = max(2, int(width / 0.9))
    for row in range(rows):
        wy = y + 0.85 + row * ((height - 1.2) / rows)
        for col in range(cols):
            wx = x - width * 0.38 + col * ((width * 0.76) / max(1, cols - 1))
            window_mat = mats["amber"] if (row + col + index) % 5 == 0 else mats["cyan"]
            cube(f"tower_{index:02d}_window_{row}_{col}", (wx, wy, z - depth * 0.512), (0.24, 0.46, 0.035), window_mat, bevel=0.025)
    # Antenna crown and a pair of rooftop coolant tanks.
    cylinder(f"tower_{index:02d}_antenna", (x, y + height + 1.1, z), 0.045, 1.9, mats["white"], vertices=8)
    neon_strip(f"tower_{index:02d}_antenna_light", (x, y + height + 1.8, z), (0.18, 0.08, 0.18), mats["magenta"], rotation=(0, 0, math.pi / 4))
    for side in (-1, 1):
        cylinder(f"tower_{index:02d}_tank_{side}", (x + side * width * 0.22, y + height + 0.5, z + depth * 0.22), 0.22, 0.55, mats["violet"], vertices=12)
    return body


def make_runner(mats):
    # Reference sculpt with the same proportions as the in-game procedural rig.
    root = cube("runner_root", (0, 1.05, 0), (0.54, 0.78, 0.34), mats["steel"], bevel=0.14)
    cylinder("runner_visor", (0, 1.32, -0.23), 0.19, 0.08, mats["cyan"], vertices=24, rotation=(math.pi / 2, 0, 0))
    cube("runner_mask", (0, 1.32, -0.14), (0.38, 0.24, 0.12), mats["ink"], bevel=0.05)
    neon_strip("runner_mask_glow", (0, 1.35, -0.205), (0.23, 0.035, 0.018), mats["magenta"])
    for side in (-1, 1):
        cylinder("runner_arm", (side * 0.43, 1.0, 0), 0.11, 0.78, mats["violet"], vertices=12, rotation=(0, 0, side * math.pi / 2))
        cylinder("runner_glove", (side * 0.79, 0.98, -0.04), 0.14, 0.22, mats["cyan"], vertices=12, rotation=(0, 0, side * math.pi / 2))
        cylinder("runner_leg", (side * 0.16, 0.42, 0), 0.13, 0.9, mats["steel"], vertices=12)
        cube("runner_boot", (side * 0.16, 0.08, -0.08), (0.28, 0.16, 0.52), mats["ink"], bevel=0.07)
    return root


def build(output: Path):
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for datablocks in (bpy.data.materials, bpy.data.meshes, bpy.data.curves, bpy.data.cameras, bpy.data.lights):
        for block in list(datablocks):
            if block.users == 0:
                datablocks.remove(block)

    tex_dir = ROOT / "art" / "textures"
    tex_dir.mkdir(parents=True, exist_ok=True)
    texture = make_neon_texture(tex_dir / "neon_grid.png")
    mats = {
        "ink": material("ink", PALETTE["ink"], metallic=0.3, roughness=0.34),
        "steel": material("steel", PALETTE["steel"], metallic=0.88, roughness=0.22),
        "violet": material("violet", PALETTE["violet"], metallic=0.35, roughness=0.3),
        "cyan": material("cyan", PALETTE["cyan"], emission=PALETTE["cyan"], strength=5.0),
        "magenta": material("magenta", PALETTE["magenta"], emission=PALETTE["magenta"], strength=5.0),
        "amber": material("amber", PALETTE["amber"], emission=PALETTE["amber"], strength=3.0),
        "white": material("white", PALETTE["white"], emission=PALETTE["white"], strength=2.0),
        "window_grid": texture_material("window_grid", texture, PALETTE["cyan"]),
    }

    cube("diorama_ground", (0, -0.2, 0), (42, 0.4, 36), mats["ink"], bevel=0.12)
    neon_strip("ground_grid_x", (0, 0.02, 0), (41, 0.035, 0.06), mats["cyan"])
    neon_strip("ground_grid_z", (0, 0.025, 0), (0.06, 0.035, 35), mats["magenta"])
    towers = [
        (-13, 0, -9, 5.5, 5.0, 10.5), (-5, 0, -10, 4.0, 4.0, 6.5),
        (4, 0, -11, 6.0, 5.0, 14.0), (13, 0, -8, 4.0, 4.0, 9.5),
        (-14, 0, 8, 4.5, 5.0, 8.0), (-5, 0, 10, 5.0, 4.0, 11.0),
        (7, 0, 9, 5.0, 5.0, 7.5), (15, 0, 8, 3.8, 4.0, 12.5),
    ]
    for i, spec in enumerate(towers):
        make_building(i, *spec, mats)
    # Elevated skybridge, ducts and a holographic billboard.
    cube("skybridge", (0, 4.2, -4.5), (14, 0.5, 1.3), mats["steel"], bevel=0.08)
    for x in (-6, 6):
        cylinder("skybridge_pylon", (x, 2.0, -4.5), 0.18, 4.0, mats["violet"], vertices=12)
    neon_strip("billboard_frame", (0, 6.0, -10.0), (5.8, 3.0, 0.16), mats["magenta"], rotation=(math.radians(0), 0, 0))
    cube("billboard_face", (0, 6.0, -9.88), (5.2, 2.4, 0.05), mats["window_grid"], bevel=0.02)
    for x in (-9, 9):
        cylinder("vent_stack", (x, 0.7, 0), 0.35, 1.4, mats["steel"], vertices=16)
    make_runner(mats)

    bpy.ops.object.select_all(action="SELECT")
    bpy.context.view_layer.objects.active = bpy.context.object
    output.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(filepath=str(output), export_format="GLB", use_selection=True, export_apply=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / "art" / "cyberpunk_city.glb")
    args = parser.parse_args()
    build(args.output.resolve())
    print(f"wrote {args.output.resolve()}")


if __name__ == "__main__":
    main()
