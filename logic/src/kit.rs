//! The city's kit: textured boxes made here, and the models from `art/` with the
//! boxes a body can stand on.
//!
//! A surface is a set of textures from `art/textures` and `art/data` that repeats
//! every `tile` metres. A box takes one surface for its walls and one for its top,
//! so a wall of any size has bricks or windows of the same size. All the boxes of
//! a district are one model, which a test writes to `art/district.gltf`: the
//! engine loads the textures of a model from `art/` only for a model in `art/`.
//!
//! A `Piece` is one model that `tools/prepare-art.py` wrote: its origin is on the
//! ground under its centre (on its back face for a wall piece), its front faces
//! +Z and its length is along X. Its `solids` are boxes in that frame. They come
//! from the `DECK` lines that the tool prints.
use exact_game::asset::{MaterialData, MeshData, Model, Node};
use exact_game::*;
use exact_game_physics::{Collider, Shape};

use crate::park::SOLID;

/// A repeating surface.
pub struct Surface {
    pub color: &'static str,
    pub normal: Option<&'static str>,
    /// Roughness in green and metallic in blue.
    pub rough: Option<&'static str>,
    pub glow: Option<&'static str>,
    /// Metres for one repeat, across and along (or up).
    pub tile: [f32; 2],
    pub roughness: f32,
    pub glow_strength: f32,
}

const fn surface(color: &'static str, normal: &'static str, tile: [f32; 2], roughness: f32) -> Surface {
    Surface {
        color,
        normal: Some(normal),
        rough: None,
        glow: None,
        tile,
        roughness,
        glow_strength: 0.0,
    }
}

pub const ASPHALT: Surface = Surface {
    rough: Some("asphalt-mr.tex"),
    ..surface("asphalt.tex", "asphalt-n.tex", [7.0, 7.0], 1.0)
};
pub const ROOF: Surface = Surface {
    rough: Some("roof-mr.tex"),
    ..surface("roof.tex", "roof-n.tex", [4.0, 4.0], 1.0)
};
pub const CONCRETE: Surface = surface("concrete.tex", "concrete-n.tex", [5.0, 5.0], 0.55);
pub const BRICK: Surface = surface("brick.tex", "brick-n.tex", [2.2, 2.2], 0.6);
pub const TILE: Surface = surface("tile.tex", "tile-n.tex", [1.8, 1.8], 0.3);
pub const PLATE: Surface = surface("plate.tex", "plate-n.tex", [1.2, 1.2], 0.4);
pub const CORRUGATED: Surface = surface("corrugated.tex", "corrugated-n.tex", [2.4, 2.4], 0.5);
/// Four windows across and three floors up in one repeat.
pub const WINDOWS: Surface = Surface {
    glow: Some("windows-glow.tex"),
    glow_strength: 2.2,
    ..surface("windows.tex", "windows-n.tex", [12.0, 9.6], 0.6)
};
/// Eight windows by eight floors: the far towers.
pub const TOWER: Surface = Surface {
    glow: Some("tower-glow.tex"),
    glow_strength: 2.6,
    ..surface("tower.tex", "tower-n.tex", [26.0, 26.0], 0.35)
};

struct Textures(Vec<String>);
impl Textures {
    fn index(&mut self, name: &str) -> u32 {
        if let Some(i) = self.0.iter().position(|n| n == name) {
            return i as u32;
        }
        self.0.push(name.into());
        (self.0.len() - 1) as u32
    }
    fn material(&mut self, look: &Look) -> MaterialData {
        match look {
            Look::Surface(s) => MaterialData {
                name: s.color.trim_end_matches(".tex").into(),
                metallic: if s.rough.is_some() { 1.0 } else { 0.0 },
                roughness: s.roughness,
                base_color_texture: Some(self.index(s.color)),
                normal_texture: s.normal.map(|n| self.index(n)),
                metallic_roughness_texture: s.rough.map(|n| self.index(n)),
                emissive_texture: s.glow.map(|n| self.index(n)),
                emissive: [s.glow_strength; 3],
                ..MaterialData::default()
            },
            // A sign is a dark board that emits its image.
            Look::Sign(image) => MaterialData {
                name: image.trim_end_matches(".tex").into(),
                metallic: 0.0,
                roughness: 0.4,
                base_color: [0.02, 0.02, 0.025, 1.0],
                emissive_texture: Some(self.index(image)),
                emissive: [SIGN_GLOW; 3],
                ..MaterialData::default()
            },
        }
    }
}

/// How bright a neon sign is.
const SIGN_GLOW: f32 = 3.2;

#[derive(Clone, Copy)]
enum Look {
    Surface(&'static Surface),
    Sign(&'static str),
}
impl Look {
    fn key(&self) -> &'static str {
        match self {
            Look::Surface(s) => s.color,
            Look::Sign(image) => image,
        }
    }
}

#[derive(Default)]
struct Faces(MeshData);
impl Faces {
    /// One rectangle: `p` is its four corners, counter-clockwise seen from outside,
    /// starting at the lower left; `uv` is the texture position of that corner and
    /// of the upper right one.
    fn quad(&mut self, p: [Vec3; 4], uv: [[f32; 2]; 2]) {
        let n = (p[1] - p[0]).cross(p[3] - p[0]).normalize_or_zero();
        let i = (self.0.positions.len() / 3) as u32;
        let ([u0, v0], [u1, v1]) = (uv[0], uv[1]);
        for (corner, t) in p.iter().zip([[u0, v0], [u1, v0], [u1, v1], [u0, v1]]) {
            self.0.positions.extend(corner.to_array());
            self.0.normals.extend(n.to_array());
            self.0.uvs.extend(t);
        }
        self.0.indices.extend([i, i + 1, i + 2, i, i + 2, i + 3]);
    }
    fn finish(mut self, material: u32) -> MeshData {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for p in self.0.positions.chunks_exact(3) {
            let p = Vec3::new(p[0], p[1], p[2]);
            lo = lo.min(p);
            hi = hi.max(p);
        }
        self.0.bounds = [lo.x, lo.y, lo.z, hi.x, hi.y, hi.z];
        self.0.material = material;
        self.0
    }
}

/// A box of the district that the body can stand on: its name, centre and size.
pub struct Box {
    pub name: String,
    pub at: Vec3,
    pub size: Vec3,
}

/// Every textured box and sign of a district, in the world's own frame. It is
/// one model, `art/district.gltf`, with one mesh for each surface, so the whole
/// district is a dozen draws. `logic/tests/city.rs` writes the file and fails
/// when the file is not what this code builds.
#[derive(Default)]
pub struct District {
    groups: Vec<(Look, Faces)>,
    pub boxes: Vec<Box>,
}

impl District {
    fn faces(&mut self, look: Look) -> &mut Faces {
        let at = match self.groups.iter().position(|(l, _)| l.key() == look.key()) {
            Some(at) => at,
            None => {
                self.groups.push((look, Faces::default()));
                self.groups.len() - 1
            }
        };
        &mut self.groups[at].1
    }

    /// A box centred at `at` with `walls` on its four sides and `top` on its
    /// upper face; it has no lower face. Each box starts its textures at its own
    /// place, so two boxes side by side do not show the same stains.
    pub fn scenery(&mut self, at: Vec3, size: Vec3, walls: &'static Surface, top: &'static Surface) {
        let h = size * 0.5;
        let shift = (at.x * 0.37 + at.z * 0.61).rem_euclid(16.0);
        // Around the box: south (+Z), east, north, west. `along` runs to the
        // right when the face is seen from outside.
        let corners = [
            (Vec3::new(-h.x, 0.0, h.z), Vec3::X, size.x),
            (Vec3::new(h.x, 0.0, h.z), Vec3::NEG_Z, size.z),
            (Vec3::new(h.x, 0.0, -h.z), Vec3::NEG_X, size.x),
            (Vec3::new(-h.x, 0.0, -h.z), Vec3::Z, size.z),
        ];
        let mut around = shift;
        let sides = self.faces(Look::Surface(walls));
        for (from, along, length) in corners {
            let (lo, up) = (at + from - Vec3::Y * h.y, Vec3::Y * size.y);
            let u0 = around / walls.tile[0];
            let u1 = (around + length) / walls.tile[0];
            // The image's top row is the top of the wall.
            let v = size.y / walls.tile[1];
            sides.quad(
                [lo, lo + along * length, lo + along * length + up, lo + up],
                [[u0, v], [u1, 0.0]],
            );
            around += length;
        }
        let y = Vec3::Y * h.y;
        self.faces(Look::Surface(top)).quad(
            [
                at + Vec3::new(-h.x, 0.0, h.z) + y,
                at + Vec3::new(h.x, 0.0, h.z) + y,
                at + Vec3::new(h.x, 0.0, -h.z) + y,
                at + Vec3::new(-h.x, 0.0, -h.z) + y,
            ],
            [
                [shift / top.tile[0], (shift + size.z) / top.tile[1]],
                [(shift + size.x) / top.tile[0], shift / top.tile[1]],
            ],
        );
    }

    /// A box as `scenery`, which the body can also stand on and climb.
    pub fn block(&mut self, name: &str, at: Vec3, size: Vec3, walls: &'static Surface, top: &'static Surface) {
        self.scenery(at, size, walls, top);
        self.boxes.push(Box {
            name: name.into(),
            at,
            size,
        });
    }

    /// A neon sign `width` by `height` centred at `at`. `yaw` degrees turn its
    /// face from south (+Z).
    pub fn sign(&mut self, image: &'static str, at: Vec3, yaw: f32, width: f32, height: f32) {
        let turn = turn(yaw);
        let (w, h) = (turn * Vec3::X * width * 0.5, Vec3::Y * height * 0.5);
        self.faces(Look::Sign(image))
            .quad([at - w - h, at + w - h, at + w + h, at - w + h], [[0.0, 1.0], [1.0, 0.0]]);
    }

    pub fn model(self) -> Model {
        let mut textures = Textures(Vec::new());
        let mut materials = Vec::new();
        let mut meshes = Vec::new();
        for (i, (look, faces)) in self.groups.into_iter().enumerate() {
            materials.push(textures.material(&look));
            meshes.push(faces.finish(i as u32));
        }
        let mut bounds = [f32::INFINITY, f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
        for m in &meshes {
            for k in 0..3 {
                bounds[k] = bounds[k].min(m.bounds[k]);
                bounds[k + 3] = bounds[k + 3].max(m.bounds[k + 3]);
            }
        }
        Model {
            bounds,
            nodes: (0..meshes.len() as u32)
                .map(|mesh| Node {
                    mesh: Some(mesh),
                    ..Node::default()
                })
                .collect(),
            meshes,
            materials,
            textures: textures.0,
            ..Model::default()
        }
    }
}

/// `model` as a glTF file and its buffer, which the file names `<stem>.bin`. The
/// images are the ones under `art/textures` and `art/data`, by path, so the bake
/// shares each with every other model.
pub fn gltf(model: &Model, stem: &str) -> (String, Vec<u8>) {
    let mut bin = Vec::<u8>::new();
    let (mut views, mut accessors, mut meshes) = (Vec::new(), Vec::new(), Vec::new());
    let mut view = |bytes: &[u8], bin: &mut Vec<u8>, target: u32| {
        views.push(format!(
            r#"{{"buffer":0,"byteOffset":{},"byteLength":{},"target":{target}}}"#,
            bin.len(),
            bytes.len()
        ));
        bin.extend_from_slice(bytes);
        views.len() - 1
    };
    let floats = |v: &[f32]| v.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
    for mesh in &model.meshes {
        let count = mesh.positions.len() / 3;
        let b = mesh.bounds;
        let p = view(&floats(&mesh.positions), &mut bin, 34962);
        let n = view(&floats(&mesh.normals), &mut bin, 34962);
        let t = view(&floats(&mesh.uvs), &mut bin, 34962);
        let indices: Vec<u8> = mesh.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        let i = view(&indices, &mut bin, 34963);
        let first = accessors.len();
        accessors.push(format!(
            r#"{{"bufferView":{p},"componentType":5126,"count":{count},"type":"VEC3","min":[{},{},{}],"max":[{},{},{}]}}"#,
            b[0], b[1], b[2], b[3], b[4], b[5]
        ));
        accessors.push(format!(r#"{{"bufferView":{n},"componentType":5126,"count":{count},"type":"VEC3"}}"#));
        accessors.push(format!(r#"{{"bufferView":{t},"componentType":5126,"count":{count},"type":"VEC2"}}"#));
        accessors.push(format!(
            r#"{{"bufferView":{i},"componentType":5125,"count":{},"type":"SCALAR"}}"#,
            mesh.indices.len()
        ));
        meshes.push(format!(
            r#"{{"primitives":[{{"attributes":{{"POSITION":{first},"NORMAL":{},"TEXCOORD_0":{}}},"indices":{},"material":{}}}]}}"#,
            first + 1,
            first + 2,
            first + 3,
            mesh.material
        ));
    }
    let materials: Vec<String> = model
        .materials
        .iter()
        .map(|m| {
            let texture = |key: &str, index: Option<u32>| index.map_or(String::new(), |i| format!(r#","{key}":{{"index":{i}}}"#));
            let glow = m.emissive[0];
            let emission = if glow > 0.0 {
                format!(
                    r#","emissiveFactor":[1,1,1],"extensions":{{"KHR_materials_emissive_strength":{{"emissiveStrength":{glow}}}}}"#
                )
            } else {
                String::new()
            };
            format!(
                r#"{{"name":"{}","pbrMetallicRoughness":{{"baseColorFactor":[{},{},{},1],"metallicFactor":{},"roughnessFactor":{}{}{}}}{}{}{}}}"#,
                m.name,
                m.base_color[0],
                m.base_color[1],
                m.base_color[2],
                m.metallic,
                m.roughness,
                texture("baseColorTexture", m.base_color_texture),
                texture("metallicRoughnessTexture", m.metallic_roughness_texture),
                texture("normalTexture", m.normal_texture),
                texture("emissiveTexture", m.emissive_texture),
                emission,
            )
        })
        .collect();
    let images: Vec<String> = model
        .textures
        .iter()
        .map(|name| {
            let file = name.trim_end_matches(".tex");
            let folder = if file.ends_with("-n") || file.ends_with("-mr") { "data" } else { "textures" };
            format!(r#"{{"uri":"{folder}/{file}.png"}}"#)
        })
        .collect();
    let textures: Vec<String> = (0..images.len()).map(|i| format!(r#"{{"source":{i},"sampler":0}}"#)).collect();
    let nodes: Vec<String> = (0..meshes.len()).map(|i| format!(r#"{{"mesh":{i}}}"#)).collect();
    let roots: Vec<String> = (0..meshes.len()).map(|i| i.to_string()).collect();
    let json = format!(
        concat!(
            r#"{{"asset":{{"version":"2.0","generator":"exact-beat-em-up kit"}},"#,
            r#""extensionsUsed":["KHR_materials_emissive_strength"],"#,
            r#""scene":0,"scenes":[{{"nodes":[{roots}]}}],"nodes":[{nodes}],"meshes":[{meshes}],"#,
            r#""materials":[{materials}],"textures":[{textures}],"images":[{images}],"#,
            r#""samplers":[{{"magFilter":9729,"minFilter":9987,"wrapS":10497,"wrapT":10497}}],"#,
            r#""accessors":[{accessors}],"bufferViews":[{views}],"#,
            r#""buffers":[{{"uri":"{stem}.bin","byteLength":{length}}}]}}"#,
            "\n"
        ),
        roots = roots.join(","),
        nodes = nodes.join(","),
        meshes = meshes.join(","),
        materials = materials.join(","),
        textures = textures.join(","),
        images = images.join(","),
        accessors = accessors.join(","),
        views = views.join(","),
        length = bin.len(),
        stem = stem,
    );
    (json, bin)
}

/// One box of a piece, in the piece's frame. `at` is its centre and `size` its full
/// extents, metres. `pitch` is degrees about X: a positive pitch has its high end
/// at the back (-Z).
pub struct Solid {
    pub at: [f32; 3],
    pub size: [f32; 3],
    pub pitch: f32,
}

const fn solid(at: [f32; 3], size: [f32; 3]) -> Solid {
    Solid {
        at,
        size,
        pitch: 0.0,
    }
}

/// One model from `art/`.
pub struct Piece {
    /// The stem of its file: `art/<name>.glb`.
    pub name: &'static str,
    pub solids: &'static [Solid],
    /// How far under its place the model goes, so a base slab is level with the
    /// floor it stands on.
    pub sink: f32,
}

/// Metres from the camera at which a piece draws its `-far` model.
pub const FAR: f32 = 42.0;

/// The low landing of the fire escape, the height of one step and the number of
/// steps to the middle landing.
pub const ESCAPE_LOW: f32 = 1.15;
pub const ESCAPE_STEP: f32 = 0.15;
pub const ESCAPE_STEPS: usize = 9;
pub const ESCAPE_MID: f32 = ESCAPE_LOW + ESCAPE_STEP * ESCAPE_STEPS as f32;
/// Its upper landing, one climb above the middle one.
pub const ESCAPE_TOP: f32 = 4.0;

pub const AWNING: Piece = Piece {
    name: "awning",
    solids: &[Solid {
        at: [0.0, 0.8, 1.05],
        size: [3.2, 0.08, 2.2],
        pitch: 18.0,
    }],
    sink: 0.0,
};
pub const BALCONY: Piece = Piece {
    name: "balcony",
    solids: &[
        solid([0.0, 0.575, 0.9], [3.0, 0.25, 1.0]),
        solid([0.0, 1.8, 0.15], [3.6, 3.6, 0.3]),
    ],
    sink: 0.0,
};
pub const BILLBOARD: Piece = Piece {
    name: "billboard",
    solids: &[
        solid([0.0, 3.7, 0.0], [7.8, 2.0, 0.3]),
        solid([-0.2, 2.6, 0.6], [7.6, 0.2, 1.4]),
        solid([-3.3, 1.25, 0.0], [0.8, 2.5, 1.2]),
        solid([3.3, 1.25, 0.0], [0.8, 2.5, 1.2]),
    ],
    sink: 0.0,
};
pub const CRANE: Piece = Piece {
    name: "crane",
    solids: &[
        solid([0.0, 0.4, 0.0], [8.2, 0.8, 10.0]),
        solid([-0.9, 3.3, -1.0], [3.4, 5.0, 3.6]),
    ],
    sink: 0.0,
};
pub const ELEVATOR: Piece = Piece {
    name: "elevator",
    solids: &[solid([0.0, 7.5, -0.3], [4.0, 15.0, 3.3])],
    sink: 0.0,
};
pub const FACADE: Piece = Piece {
    name: "facade",
    solids: &[solid([0.0, 5.35, -0.1], [6.6, 10.7, 6.6])],
    sink: 0.2,
};
pub const FIRE_ESCAPE: Piece = Piece {
    name: "fire-escape",
    solids: &[
        solid([-1.9, ESCAPE_LOW - 0.1, 0.8], [1.6, 0.2, 2.15]),
        solid([1.84, ESCAPE_MID - 0.1, 1.15], [1.68, 0.2, 1.35]),
        solid([1.38, ESCAPE_TOP - 0.1, -0.7], [2.57, 0.2, 2.3]),
    ],
    sink: 0.0,
};
pub const GARDEN: Piece = Piece {
    name: "garden",
    solids: &[solid([0.0, 0.65, -0.7], [8.8, 1.3, 3.0])],
    sink: 0.2,
};
pub const HVAC: Piece = Piece {
    name: "hvac",
    solids: &[
        solid([-2.3, 0.7, 0.9], [3.0, 1.2, 3.2]),
        solid([1.9, 1.0, -0.4], [3.8, 1.8, 3.8]),
    ],
    sink: 0.1,
};
pub const LADDER: Piece = Piece {
    name: "ladder",
    solids: &[],
    sink: 0.0,
};
pub const NEON: Piece = Piece {
    name: "neon",
    solids: &[
        solid([0.0, 3.9, 0.0], [6.6, 3.0, 0.5]),
        solid([-2.5, 1.2, 0.0], [0.9, 2.4, 1.2]),
        solid([2.5, 1.2, 0.0], [0.9, 2.4, 1.2]),
    ],
    sink: 0.0,
};
pub const PIPES: Piece = Piece {
    name: "pipes",
    solids: &[
        solid([0.0, 2.5, 0.25], [4.6, 5.0, 0.5]),
        solid([0.0, 1.9, 0.9], [4.2, 3.2, 0.9]),
        solid([0.0, 4.0, 1.0], [4.2, 0.15, 1.3]),
    ],
    sink: 0.0,
};
pub const POLE: Piece = Piece {
    name: "pole",
    solids: &[
        solid([0.4, 4.0, 0.0], [0.45, 8.0, 0.45]),
        solid([0.4, 0.5, 0.0], [1.1, 1.0, 1.1]),
    ],
    sink: 0.0,
};
pub const SCAFFOLD: Piece = Piece {
    name: "scaffold",
    solids: &[solid([0.0, 4.4, 0.0], [3.8, 8.8, 2.6])],
    sink: 0.0,
};
/// The top of the skybridge above its base.
pub const SKYBRIDGE_TOP: f32 = 3.95;
pub const SKYBRIDGE: Piece = Piece {
    name: "skybridge",
    solids: &[
        solid([0.0, 1.85, 0.0], [14.0, 2.9, 2.9]),
        solid([0.0, 3.62, 0.0], [13.9, 0.66, 2.14]),
    ],
    sink: 0.0,
};
/// The roof of the stall above the street.
pub const STALL_ROOF: f32 = 2.65;
pub const STALL: Piece = Piece {
    name: "stall",
    solids: &[
        solid([0.0, STALL_ROOF / 2.0, -0.4], [4.6, STALL_ROOF, 2.5]),
        Solid {
            at: [0.0, 2.2, 1.25],
            size: [4.6, 0.1, 0.9],
            pitch: 26.0,
        },
    ],
    sink: 0.0,
};
pub const VIADUCT: Piece = Piece {
    name: "viaduct",
    solids: &[
        solid([0.0, 6.4, 0.0], [24.0, 1.0, 6.7]),
        solid([1.5, 2.95, 0.0], [5.0, 5.9, 3.0]),
    ],
    sink: 0.0,
};
/// The deck of the covered walkway above its base.
pub const WALKWAY_DECK: f32 = 1.0;
pub const WALKWAY: Piece = Piece {
    name: "walkway",
    solids: &[
        solid([0.0, WALKWAY_DECK - 0.125, 0.0], [13.9, 0.25, 1.9]),
        solid([0.0, 1.5, -0.95], [13.9, 1.0, 0.08]),
        solid([0.0, 1.5, 0.95], [13.9, 1.0, 0.08]),
        solid([0.0, 3.3, 0.0], [13.6, 0.2, 1.9]),
    ],
    sink: 0.0,
};
pub const ZIP_ANCHOR: Piece = Piece {
    name: "zip-anchor",
    solids: &[
        solid([0.0, 0.1, 0.0], [2.6, 0.2, 2.4]),
        solid([0.0, 1.3, 0.0], [1.6, 2.4, 1.6]),
    ],
    sink: 0.0,
};

pub const PIECES: [&Piece; 19] = [
    &AWNING,
    &BALCONY,
    &BILLBOARD,
    &CRANE,
    &ELEVATOR,
    &FACADE,
    &FIRE_ESCAPE,
    &GARDEN,
    &HVAC,
    &LADDER,
    &NEON,
    &PIPES,
    &POLE,
    &SCAFFOLD,
    &SKYBRIDGE,
    &STALL,
    &VIADUCT,
    &WALKWAY,
    &ZIP_ANCHOR,
];

fn turn(yaw: f32) -> Quat {
    Quat::from_rotation_y(yaw.to_radians())
}

/// A static box that the body can stand on and climb.
pub fn collider(w: &mut World, name: &str, at: Vec3, rotation: Quat, size: Vec3) -> Entity {
    w.spawn_named(
        name,
        (
            Transform {
                position: at,
                rotation,
                ..Transform::default()
            },
            Collider {
                shape: Shape::Box { half: size * 0.5 },
                layer: SOLID,
                ..Collider::default()
            },
        ),
    )
}

/// Put `piece` in the world as `name`, its origin at `at`, turned `yaw` degrees
/// about Y: at zero its front faces south (+Z), at 90 east (+X).
pub fn place(w: &mut World, name: &str, piece: &Piece, at: Vec3, yaw: f32) -> Entity {
    let rotation = turn(yaw);
    let model = w.spawn_named(
        name,
        (
            Transform {
                position: at - Vec3::Y * piece.sink,
                rotation,
                ..Transform::default()
            },
            Mesh::asset(format!("{}.model", piece.name)),
            ModelLod {
                levels: vec![LodLevel {
                    distance: FAR,
                    model: format!("{}-far.model", piece.name),
                }],
                hide: None,
            },
        ),
    );
    let floor = at - Vec3::Y * piece.sink;
    for (i, s) in piece.solids.iter().enumerate() {
        collider(
            w,
            &format!("{name}-solid-{i}"),
            floor + rotation * Vec3::from_array(s.at),
            rotation * Quat::from_rotation_x(s.pitch.to_radians()),
            Vec3::from_array(s.size),
        );
    }
    if piece.name == FIRE_ESCAPE.name {
        // The flight from the low landing east to the middle one: steps that the
        // capsule walks up.
        let run = 2.1 / ESCAPE_STEPS as f32;
        for i in 0..ESCAPE_STEPS {
            let top = ESCAPE_LOW + ESCAPE_STEP * (i + 1) as f32;
            let x = -1.1 + run * (i as f32 + 0.5);
            collider(
                w,
                &format!("{name}-step-{i}"),
                floor + rotation * Vec3::new(x, top - 0.15, 1.15),
                rotation,
                Vec3::new(run, 0.3, 1.35),
            );
        }
    }
    model
}
