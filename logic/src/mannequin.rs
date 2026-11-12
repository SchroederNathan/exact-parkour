//! Cyberpunk runner shells on the existing humanoid skeleton. Rigid limb
//! weights preserve the articulated armor as the ball joints move; the torso
//! alone blends across the spine so it can bend with the parkour animations.
use exact_game::asset::{MaterialData, MeshData, Model};
use exact_game::rig::Rig;
use exact_game::{math, Vec3};

const SIDES: usize = 32;
const ARMOR: [f32; 4] = [0.19, 0.24, 0.32, 1.0];
const PLATE: [f32; 4] = [0.28, 0.36, 0.48, 1.0];
const RUBBER: [f32; 4] = [0.028, 0.038, 0.070, 1.0];
const JOINT: [f32; 4] = [0.055, 0.075, 0.12, 1.0];
const EDGE: [f32; 4] = [0.33, 0.40, 0.52, 1.0];
const CYAN: [f32; 4] = [0.04, 0.72, 0.88, 1.0];
const MAGENTA: [f32; 4] = [0.84, 0.09, 0.40, 1.0];
const VISOR: [f32; 4] = [0.02, 0.12, 0.18, 1.0];
const GLOVE: [f32; 4] = [0.08, 0.11, 0.17, 1.0];
const BOOT: [f32; 4] = [0.035, 0.045, 0.075, 1.0];

// Distance along the part, half-width, half-depth. Rounded ends have one pole,
// rather than a ring of coincident vertices and degenerate triangles.
type Ring = [f32; 3];

struct Sculpt<'a> {
    mesh: MeshData,
    rig: &'a Rig,
    scale: f32,
}

impl Sculpt<'_> {
    fn joint(&self, name: &str) -> usize {
        self.rig
            .bones()
            .iter()
            .position(|b| b.name == name)
            .unwrap()
    }

    fn vertex(&mut self, at: Vec3, bone: usize, color: [f32; 4], torso: bool) -> u32 {
        let index = (self.mesh.positions.len() / 3) as u32;
        self.mesh.positions.extend((at * self.scale).to_array());
        self.mesh.normals.extend([0.0; 3]);
        self.mesh.colors.extend(color);
        self.mesh.uvs.extend([0.0; 2]);
        let chest = if torso { self.joint("chest") } else { bone };
        let t = if torso {
            ((at.y - 1.24) / 0.14).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let t = t * t * (3.0 - 2.0 * t);
        self.mesh.joints.extend([bone as u16, chest as u16, 0, 0]);
        self.mesh.weights.extend([1.0 - t, t, 0.0, 0.0]);
        index
    }

    fn shell(&mut self, name: &str, from: Vec3, to: Vec3, rings: &[Ring], color: [f32; 4]) {
        let bone = self.joint(name);
        let axis = (to - from).normalize();
        let right = (Vec3::X - axis * axis.x).normalize();
        let front = right.cross(axis);
        let mut previous = Vec::new();
        for &[t, width, depth] in rings {
            let centre = from.lerp(to, t);
            let mut current = Vec::new();
            let count = if width == 0.0 { 1 } else { SIDES };
            for side in 0..count {
                let (sin, cos) = math::sin_cos(side as f32 * std::f32::consts::TAU / SIDES as f32);
                let at = centre + right * (cos * width) + front * (sin * depth);
                current.push(self.vertex(at, bone, color, name == "spine"));
            }
            if !previous.is_empty() {
                for side in 0..SIDES {
                    let next = (side + 1) % SIDES;
                    if previous.len() == 1 {
                        self.mesh
                            .indices
                            .extend([previous[0], current[side], current[next]]);
                    } else if current.len() == 1 {
                        self.mesh
                            .indices
                            .extend([previous[side], current[0], previous[next]]);
                    } else {
                        self.mesh.indices.extend([
                            previous[side],
                            current[side],
                            previous[next],
                            previous[next],
                            current[side],
                            current[next],
                        ]);
                    }
                }
            }
            previous = current;
        }
    }

    fn ball(&mut self, bone: &str, centre: Vec3, radii: Vec3) {
        let mut rings = Vec::new();
        for step in 0..=12 {
            let angle = step as f32 * std::f32::consts::PI / 12.0;
            let (sin, cos) = math::sin_cos(angle);
            let radius = if step == 0 || step == 12 { 0.0 } else { sin };
            rings.push([(1.0 - cos) * 0.5, radius * radii.x, radius * radii.z]);
        }
        self.shell(
            bone,
            centre - Vec3::Y * radii.y,
            centre + Vec3::Y * radii.y,
            &rings,
            JOINT,
        );
    }

    fn limb(&mut self, name: &str, width: f32, depth: f32, taper: f32, joint: f32) {
        let bone = &self.rig.bones()[self.joint(name)];
        let head = bone.head / self.scale;
        let tail = bone.tail / self.scale;
        self.ball(name, head, Vec3::splat(joint));
        self.shell(
            name,
            head,
            tail,
            &[
                [0.035, 0.0, 0.0],
                [0.065, width * 0.55, depth * 0.55],
                [0.11, width * 0.85, depth * 0.85],
                [0.19, width, depth],
                [0.34, width, depth],
                [0.52, width * 0.93, depth * 0.93],
                [0.72, width * 0.80, depth * 0.80],
                [0.86, width * taper, depth * taper],
                [0.92, width * taper * 0.8, depth * taper * 0.8],
                [0.96, width * taper * 0.45, depth * taper * 0.45],
                [0.975, 0.0, 0.0],
            ],
            ARMOR,
        );
    }

    /// A short armored sleeve around a bone. The sleeve is a separate closed
    /// shell so it can keep its crisp accent color while the underlying limb
    /// continues to use the existing animation weights.
    fn band(&mut self, name: &str, start: f32, end: f32, width: f32, depth: f32, color: [f32; 4]) {
        let bone = &self.rig.bones()[self.joint(name)];
        let from = bone.head / self.scale;
        let to = bone.tail / self.scale;
        let a = from.lerp(to, start);
        let b = from.lerp(to, end);
        self.shell(
            name,
            a,
            b,
            &[
                [0.0, 0.0, 0.0],
                [0.10, width, depth],
                [0.52, width, depth],
                [0.90, width * 0.90, depth * 0.90],
                [1.0, 0.0, 0.0],
            ],
            color,
        );
    }

    fn panel(
        &mut self,
        name: &str,
        from: Vec3,
        to: Vec3,
        front: f32,
        width: f32,
        depth: f32,
        color: [f32; 4],
    ) {
        let offset = Vec3::Z * front;
        self.shell(
            name,
            from + offset,
            to + offset,
            &[
                [0.0, 0.0, 0.0],
                [0.08, width * 0.72, depth * 0.66],
                [0.22, width, depth],
                [0.72, width * 0.94, depth],
                [0.92, width * 0.70, depth * 0.62],
                [1.0, 0.0, 0.0],
            ],
            color,
        );
    }

    fn finish(mut self) -> MeshData {
        let vertex = |i: u32| Vec3::from_slice(&self.mesh.positions[i as usize * 3..]);
        let mut normals = vec![Vec3::ZERO; self.mesh.positions.len() / 3];
        for tri in self.mesh.indices.chunks_exact(3) {
            let normal = (vertex(tri[1]) - vertex(tri[0])).cross(vertex(tri[2]) - vertex(tri[0]));
            for &i in tri {
                normals[i as usize] += normal;
            }
        }
        self.mesh.normals = normals
            .into_iter()
            .flat_map(|n| n.normalize().to_array())
            .collect();
        let mut low = Vec3::splat(f32::INFINITY);
        let mut high = Vec3::splat(f32::NEG_INFINITY);
        for point in self.mesh.positions.chunks_exact(3).map(Vec3::from_slice) {
            low = low.min(point);
            high = high.max(point);
        }
        self.mesh.bounds = [low.x, low.y, low.z, high.x, high.y, high.z];
        self.mesh
    }
}

pub fn skin(model: &mut Model, rig: &Rig) {
    let mut sculpt = Sculpt {
        mesh: MeshData::default(),
        rig,
        scale: rig.bones()[1].head.y / 0.96,
    };
    let v = Vec3::new;
    sculpt.shell(
        "hips",
        v(0.0, 0.92, 0.0),
        v(0.0, 1.105, 0.0),
        &[
            [0.0, 0.0, 0.0],
            [0.04, 0.10, 0.065],
            [0.12, 0.139, 0.087],
            [0.28, 0.147, 0.097],
            [0.53, 0.144, 0.10],
            [0.76, 0.133, 0.093],
            [0.90, 0.115, 0.08],
            [0.97, 0.087, 0.062],
            [1.0, 0.0, 0.0],
        ],
        ARMOR,
    );
    sculpt.ball("spine", v(0.0, 1.115, 0.0), v(0.063, 0.052, 0.060));
    sculpt.shell(
        "spine",
        v(0.0, 1.14, 0.0),
        v(0.0, 1.50, 0.0),
        &[
            [0.0, 0.0, 0.0],
            [0.025, 0.078, 0.055],
            [0.07, 0.098, 0.070],
            [0.17, 0.105, 0.077],
            [0.34, 0.122, 0.089],
            [0.54, 0.149, 0.103],
            [0.72, 0.165, 0.106],
            [0.84, 0.164, 0.098],
            [0.93, 0.144, 0.082],
            [0.985, 0.095, 0.057],
            [1.0, 0.0, 0.0],
        ],
        ARMOR,
    );
    sculpt.ball("neck", v(0.0, 1.532, 0.0), v(0.043, 0.060, 0.043));
    sculpt.shell(
        "head",
        v(0.0, 1.58, 0.0),
        v(0.0, 1.80, 0.0),
        &[
            [0.0, 0.0, 0.0],
            [0.025, 0.044, 0.045],
            [0.09, 0.063, 0.061],
            [0.25, 0.072, 0.071],
            [0.48, 0.084, 0.080],
            [0.70, 0.087, 0.082],
            [0.84, 0.078, 0.074],
            [0.94, 0.054, 0.052],
            [0.985, 0.025, 0.024],
            [1.0, 0.0, 0.0],
        ],
        // The runner's helmet is a sealed mask rather than an exposed head.
        ARMOR,
    );

    sculpt.panel(
        "head",
        v(0.0, 1.69, 0.0),
        v(0.0, 1.76, 0.0),
        -0.078,
        0.078,
        0.018,
        VISOR,
    );
    for (side, x) in [("left", 1.0), ("right", -1.0)] {
        let _ = side;
        sculpt.shell(
            "head",
            v(0.043 * x, 1.585, -0.035),
            v(0.043 * x, 1.705, -0.035),
            &[
                [0.0, 0.0, 0.0],
                [0.12, 0.035, 0.042],
                [0.42, 0.043, 0.050],
                [0.78, 0.038, 0.044],
                [1.0, 0.0, 0.0],
            ],
            PLATE,
        );
    }
    for x in [-0.028, 0.028] {
        sculpt.shell(
            "neck",
            v(x, 1.505, -0.055),
            v(x, 1.575, -0.055),
            &[
                [0.0, 0.0, 0.0],
                [0.16, 0.020, 0.022],
                [0.45, 0.026, 0.026],
                [0.84, 0.021, 0.022],
                [1.0, 0.0, 0.0],
            ],
            RUBBER,
        );
    }
    sculpt.panel(
        "neck",
        v(0.0, 1.50, 0.0),
        v(0.0, 1.58, 0.0),
        -0.088,
        0.060,
        0.016,
        CYAN,
    );

    sculpt.shell(
        "head",
        v(0.0, 1.66, -0.078),
        v(0.0, 1.66, -0.125),
        &[
            [0.0, 0.0, 0.0],
            [0.12, 0.052, 0.030],
            [0.34, 0.066, 0.034],
            [0.66, 0.070, 0.036],
            [0.88, 0.056, 0.030],
            [1.0, 0.0, 0.0],
        ],
        VISOR,
    );
    sculpt.band("neck", 0.12, 0.88, 0.056, 0.050, MAGENTA);
    sculpt.band("spine", 0.15, 0.38, 0.172, 0.112, CYAN);
    sculpt.band("spine", 0.62, 0.84, 0.166, 0.108, MAGENTA);

    sculpt.panel(
        "spine",
        v(0.0, 1.17, 0.0),
        v(0.0, 1.45, 0.0),
        -0.092,
        0.155,
        0.030,
        PLATE,
    );
    sculpt.panel(
        "spine",
        v(0.0, 1.23, 0.0),
        v(0.0, 1.40, 0.0),
        -0.126,
        0.052,
        0.016,
        VISOR,
    );
    sculpt.panel(
        "spine",
        v(0.0, 1.41, 0.0),
        v(0.0, 1.49, 0.0),
        -0.100,
        0.165,
        0.022,
        EDGE,
    );

    sculpt.band("hips", 0.05, 0.28, 0.158, 0.112, RUBBER);
    for (x, z, width, depth, color) in [
        (0.145, -0.055, 0.048, 0.040, MAGENTA),
        (-0.145, -0.042, 0.058, 0.046, PLATE),
    ] {
        sculpt.shell(
            "hips",
            v(x, 0.985, z),
            v(x, 1.105, z),
            &[
                [0.0, 0.0, 0.0],
                [0.12, width * 0.74, depth * 0.72],
                [0.35, width, depth],
                [0.78, width * 0.92, depth * 0.92],
                [1.0, 0.0, 0.0],
            ],
            color,
        );
    }

    for (side, sign) in [("l", 1.0), ("r", -1.0)] {
        let n = |part: &str| format!("{part}_{side}");
        sculpt.limb(&n("thigh"), 0.072, 0.077, 0.62, 0.049);
        sculpt.limb(&n("shin"), 0.055, 0.060, 0.54, 0.039);
        sculpt.limb(&n("upper_arm"), 0.053, 0.056, 0.62, 0.051);
        sculpt.limb(&n("forearm"), 0.044, 0.047, 0.55, 0.031);
        sculpt.limb(&n("hand"), 0.034, 0.023, 0.66, 0.023);
        sculpt.band(&n("thigh"), 0.20, 0.43, 0.085, 0.090, EDGE);
        sculpt.band(&n("thigh"), 0.70, 0.86, 0.078, 0.082, MAGENTA);
        sculpt.band(&n("shin"), 0.28, 0.60, 0.066, 0.068, CYAN);
        sculpt.band(&n("upper_arm"), 0.03, 0.22, 0.066, 0.070, MAGENTA);
        sculpt.band(&n("forearm"), 0.42, 0.78, 0.052, 0.052, CYAN);
        sculpt.band(&n("hand"), 0.0, 0.82, 0.040, 0.027, GLOVE);
        sculpt.band(&n("upper_arm"), 0.0, 0.28, 0.076, 0.080, PLATE);
        let forearm = &rig.bones()[sculpt.joint(&n("forearm"))];
        let forearm_head = forearm.head / sculpt.scale;
        let forearm_tail = forearm.tail / sculpt.scale;
        sculpt.panel(
            &n("forearm"),
            forearm_head,
            forearm_tail,
            -0.042,
            0.046,
            0.018,
            EDGE,
        );
        let shin = &rig.bones()[sculpt.joint(&n("shin"))];
        let shin_head = shin.head / sculpt.scale;
        let shin_tail = shin.tail / sculpt.scale;
        sculpt.panel(
            &n("shin"),
            shin_head.lerp(shin_tail, 0.18),
            shin_head.lerp(shin_tail, 0.52),
            -0.056,
            0.064,
            0.022,
            PLATE,
        );
        let foot = n("foot");
        sculpt.ball(&foot, v(0.09 * sign, 0.098, -0.015), v(0.033, 0.037, 0.033));
        sculpt.shell(
            &foot,
            v(0.09 * sign, 0.051, -0.079),
            v(0.09 * sign, 0.041, 0.183),
            &[
                [0.0, 0.0, 0.0],
                [0.04, 0.034, 0.028],
                [0.14, 0.045, 0.044],
                [0.28, 0.049, 0.051],
                [0.43, 0.051, 0.046],
                [0.60, 0.056, 0.037],
                [0.78, 0.057, 0.031],
                [0.90, 0.047, 0.027],
                [0.97, 0.027, 0.018],
                [1.0, 0.0, 0.0],
            ],
            BOOT,
        );
        sculpt.shell(
            &foot,
            v(0.09 * sign, 0.052, -0.10),
            v(0.09 * sign, 0.052, 0.19),
            &[
                [0.0, 0.0, 0.0],
                [0.08, 0.046, 0.014],
                [0.35, 0.060, 0.020],
                [0.80, 0.058, 0.018],
                [0.96, 0.042, 0.014],
                [1.0, 0.0, 0.0],
            ],
            RUBBER,
        );
        sculpt.shell(
            &foot,
            v(0.09 * sign, 0.082, -0.105),
            v(0.09 * sign, 0.082, -0.015),
            &[
                [0.0, 0.0, 0.0],
                [0.18, 0.053, 0.030],
                [0.72, 0.058, 0.033],
                [1.0, 0.0, 0.0],
            ],
            EDGE,
        );
    }
    let mesh = sculpt.finish();
    model.bounds = mesh.bounds;
    model.meshes = vec![mesh];
    model.materials = vec![MaterialData {
        name: "Neon runner composite".into(),
        base_color: [0.72, 0.82, 0.92, 1.0],
        metallic: 0.78,
        rim: 0.34,
        roughness: 0.28,
        emissive: [0.015, 0.055, 0.09],
        ..MaterialData::default()
    }];
}
