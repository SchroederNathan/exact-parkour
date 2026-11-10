//! What a race level is made of: boxes, checkpoints and a finish.
//!
//! A level is a city block seen from its roofs. `building` raises a box from the
//! street, and every other piece stands on a height the designer names. Lengths are
//! metres, Y is up, north is -Z, and a yaw is degrees about Y.
use exact_game::*;

/// The colour of a piece. A level uses one colour for one meaning, so a player
/// reads the route from the colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// Building bodies: light concrete, dark slate, warm brick, cool glass.
    Concrete,
    Slate,
    Brick,
    Glass,
    /// Roof furniture that is not on the route: parapets, vents, tanks.
    Trim,
    /// A wall to vault over or mantle onto.
    Vault,
    /// A wall or an edge to grab and climb.
    Climb,
    /// A plank, a beam, a pipe or a catwalk to run along.
    Beam,
    /// A ramp or a stair.
    Ramp,
    /// A bar to slide under.
    Danger,
    /// The level's own colour: checkpoint pads and the finish.
    Accent,
    /// The far city, which nothing touches.
    Haze,
}

impl Look {
    pub fn material(self, theme: &str) -> Material {
        match self {
            Look::Concrete => Material::grid(Color::hex("#8f989c"), 3.0).rough(0.9),
            Look::Slate => Material::grid(Color::hex("#59646d"), 3.0).rough(0.9),
            Look::Brick => Material::grid(Color::hex("#96705c"), 3.0).rough(0.95),
            Look::Glass => Material::grid(Color::hex("#5f8296"), 3.0).rough(0.35),
            Look::Trim => Material::hex("#454d55").rough(0.8),
            Look::Vault => Material::hex("#d9822b").rough(0.7),
            Look::Climb => Material::hex("#2f9c95").rough(0.7),
            Look::Beam => Material::hex("#e3b341").rough(0.7),
            Look::Ramp => Material::hex("#b3a17a").rough(0.85),
            Look::Danger => Material::hex("#d0473f").rough(0.6),
            Look::Accent => Material::hex(theme).rough(0.6),
            Look::Haze => Material::hex("#7f929e").rough(1.0),
        }
    }
}

/// How a piece meets the body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Stands, blocks, and has edges to vault, grab and climb.
    Solid,
    /// Blocks a body on its feet and lets a sliding body through. It has no edge
    /// to grab.
    Bar,
    /// Drawn only.
    Scenery,
    /// Stands and blocks, and is not drawn.
    Unseen,
}

/// One box. `at` is its centre and `size` its full extents. `yaw` and `pitch` are
/// degrees about Y, then about the box's own X.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub name: String,
    pub at: [f32; 3],
    pub size: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub look: Look,
    pub kind: Kind,
}

impl Piece {
    pub fn new(name: &str, at: [f32; 3], size: [f32; 3], look: Look) -> Self {
        Self {
            name: name.into(),
            at,
            size,
            yaw: 0.0,
            pitch: 0.0,
            look,
            kind: Kind::Solid,
        }
    }
    pub fn look(&mut self, look: Look) -> &mut Self {
        self.look = look;
        self
    }
    /// Turn the box about Y, degrees. It turns about its own centre.
    pub fn turned(&mut self, yaw: f32) -> &mut Self {
        self.yaw = yaw;
        self
    }
    pub fn scenery(&mut self) -> &mut Self {
        self.kind = Kind::Scenery;
        self
    }
    pub fn pose(&self) -> Transform {
        Transform {
            position: Vec3::from_array(self.at),
            rotation: Quat::from_rotation_y(self.yaw.to_radians())
                * Quat::from_rotation_x(self.pitch.to_radians()),
            ..Transform::default()
        }
    }
    /// Height of the top face of an unpitched piece.
    pub fn top(&self) -> f32 {
        self.at[1] + self.size[1] / 2.0
    }
    /// Height of the bottom face of an unpitched piece.
    pub fn bottom(&self) -> f32 {
        self.at[1] - self.size[1] / 2.0
    }
}

/// Where a runner who fell starts again. `at` is where the feet stand, and the
/// runner faces `toward`, a point on the map given as `[x, z]`. A runner whose feet
/// go under `floor` returns here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Checkpoint {
    pub at: [f32; 3],
    pub toward: [f32; 2],
    pub floor: f32,
}

impl Checkpoint {
    /// The yaw of a body that stands here, radians; zero faces south (+Z).
    pub fn yaw(&self) -> f32 {
        math::atan2(self.toward[0] - self.at[0], self.toward[1] - self.at[2])
    }
}

/// The underside of a slide bar above the surface it stands on, and its height.
pub const BAR_CLEAR: f32 = 1.05;
pub const BAR_TALL: f32 = 1.5;
/// The highest step a stair is built from. The capsule walks up this much.
pub const RISER: f32 = 0.15;

/// One race: its name, its boxes and its route.
#[derive(Clone, Debug)]
pub struct Level {
    pub title: &'static str,
    pub subtitle: &'static str,
    /// The level's colour, as CSS hex.
    pub theme: &'static str,
    pub pieces: Vec<Piece>,
    /// The first checkpoint is the start.
    pub checkpoints: Vec<Checkpoint>,
    /// Where the feet stand at the finish gate.
    pub finish: [f32; 3],
    /// The height under which a runner has fallen, for the checkpoints that follow.
    pub floor: f32,
}

impl Level {
    pub fn new(title: &'static str, subtitle: &'static str, theme: &'static str) -> Self {
        Self {
            title,
            subtitle,
            theme,
            pieces: Vec::new(),
            checkpoints: Vec::new(),
            finish: [0.0; 3],
            floor: 3.0,
        }
    }

    pub fn add(&mut self, piece: Piece) -> &mut Piece {
        self.pieces.push(piece);
        self.pieces.last_mut().unwrap()
    }

    /// A building: a box from the street to a flat roof at height `roof`. `at` is
    /// its middle on the map, `[x, z]`, and `size` its `[width (X), depth (Z)]`.
    pub fn building(&mut self, name: &str, at: [f32; 2], size: [f32; 2], roof: f32) -> &mut Piece {
        self.add(Piece::new(
            name,
            [at[0], roof / 2.0, at[1]],
            [size[0], roof, size[1]],
            Look::Concrete,
        ))
    }

    /// A box that stands on the height `at[1]`: `at` is the middle of its
    /// underside, and `size` is `[width (X), height, depth (Z)]`. A wall, a vent,
    /// a parapet, a step, an upper storey.
    pub fn on(&mut self, name: &str, at: [f32; 3], size: [f32; 3], look: Look) -> &mut Piece {
        self.add(Piece::new(
            name,
            [at[0], at[1] + size[1] / 2.0, at[2]],
            size,
            look,
        ))
    }

    /// A box whose top face is at the height `at[1]`: `at` is the middle of its
    /// top, and `size` is `[width (X), thickness, depth (Z)]`. A plank, a bridge, a
    /// catwalk, a balcony.
    pub fn deck(&mut self, name: &str, at: [f32; 3], size: [f32; 3], look: Look) -> &mut Piece {
        self.add(Piece::new(
            name,
            [at[0], at[1] - size[1] / 2.0, at[2]],
            size,
            look,
        ))
    }

    /// A straight slab from `from` to `to`, `width` wide and `thick` deep, whose
    /// top face holds both points on its centre line. With the two heights equal
    /// it is a plank at any angle on the map; otherwise it is a ramp. A body walks
    /// a slope up to 50 degrees, and sprints at full speed up 12 degrees or less.
    pub fn ramp(
        &mut self,
        name: &str,
        from: [f32; 3],
        to: [f32; 3],
        width: f32,
        thick: f32,
        look: Look,
    ) -> &mut Piece {
        let (from, to) = (Vec3::from_array(from), Vec3::from_array(to));
        let along = to - from;
        let length = along.length();
        let yaw = math::atan2(along.x, along.z);
        let pitch = -math::asin(along.y / length);
        let up = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch) * Vec3::Y;
        let centre = (from + to) / 2.0 - up * (thick / 2.0);
        let mut piece = Piece::new(name, centre.to_array(), [width, thick, length], look);
        piece.yaw = yaw.to_degrees();
        piece.pitch = pitch.to_degrees();
        self.add(piece)
    }

    /// A stair from `from` to `to`, `width` wide. `from` and `to` are the middles
    /// of its two ends, at the heights of the surfaces it joins. The steps are
    /// drawn only: the body runs on an unseen ramp over their edges, because a
    /// real step stops the capsule for a moment. A slope slows a sprint: give a
    /// stair that must keep the speed a run of 4.5 times its rise.
    pub fn stairs(&mut self, name: &str, from: [f32; 3], to: [f32; 3], width: f32, look: Look) {
        let ramp = self.ramp(&format!("{name}-ramp"), from, to, width, 0.3, look);
        ramp.kind = Kind::Unseen;
        let (low, high) = if from[1] <= to[1] { (from, to) } else { (to, from) };
        let (low, high) = (Vec3::from_array(low), Vec3::from_array(high));
        let rise = high.y - low.y;
        let steps = math::ceil(rise / RISER).max(1.0) as usize;
        let run = Vec3::new(high.x - low.x, 0.0, high.z - low.z);
        let tread = run.length() / steps as f32;
        let forward = run.normalize();
        let yaw = math::atan2(forward.x, forward.z).to_degrees();
        // Step `n` is as high as the ramp is at its near edge, so the ramp rests
        // on the edges of the steps. The surface at the top is the last step.
        for step in 1..steps {
            let height = rise * step as f32 / steps as f32;
            let middle = low + forward * (tread * (step as f32 + 0.5));
            let mut piece = Piece::new(
                &format!("{name}-{step}"),
                [middle.x, low.y + height / 2.0, middle.z],
                [width, height, tread],
                look,
            );
            piece.yaw = yaw;
            piece.kind = Kind::Scenery;
            self.add(piece);
        }
    }

    /// A slide gate across a path: a bar that a sliding body goes under and a
    /// running body does not pass, between two posts. `at` is the middle of the
    /// opening on the surface it stands on, `span` is the width of the opening,
    /// and `yaw` is the direction of travel through it, degrees (0 or 180 for a
    /// path along Z, 90 or 270 for a path along X). Close the path beside the
    /// posts with a wall or a roof edge.
    pub fn gate(&mut self, name: &str, at: [f32; 3], span: f32, yaw: f32) {
        let turn = Quat::from_rotation_y(yaw.to_radians());
        let base = Vec3::from_array(at);
        let post = 0.4;
        let tall = BAR_CLEAR + BAR_TALL;
        for (side, sign) in [("west", -1.0), ("east", 1.0)] {
            let foot = base + turn * Vec3::new(sign * (span + post) / 2.0, 0.0, 0.0);
            let mut piece = Piece::new(
                &format!("{name}-post-{side}"),
                [foot.x, foot.y + tall / 2.0, foot.z],
                [post, tall, post],
                Look::Trim,
            );
            piece.yaw = yaw;
            self.add(piece);
        }
        let mut bar = Piece::new(
            &format!("{name}-bar"),
            [base.x, base.y + BAR_CLEAR + BAR_TALL / 2.0, base.z],
            [span, BAR_TALL, 0.3],
            Look::Danger,
        );
        bar.yaw = yaw;
        bar.kind = Kind::Bar;
        self.add(bar);
    }

    /// A building of the far city, which is drawn and never touched.
    pub fn backdrop(&mut self, name: &str, at: [f32; 2], size: [f32; 2], roof: f32) -> &mut Piece {
        let piece = self.building(name, at, size, roof);
        piece.look = Look::Haze;
        piece.kind = Kind::Scenery;
        piece
    }

    /// A checkpoint whose feet stand at `at`, facing the map point `toward`. The
    /// first one is the start.
    pub fn checkpoint(&mut self, at: [f32; 3], toward: [f32; 2]) {
        self.checkpoints.push(Checkpoint {
            at,
            toward,
            floor: self.floor,
        });
    }

    /// The height under which a runner has fallen and starts again, for every
    /// checkpoint added after this call.
    pub fn fall_below(&mut self, floor: f32) {
        self.floor = floor;
    }

    pub fn finish_at(&mut self, at: [f32; 3]) {
        self.finish = at;
    }

    /// The box on the map that holds every piece the body can touch:
    /// `[min x, min z, max x, max z]`.
    pub fn bounds(&self) -> [f32; 4] {
        let mut bounds = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
        for piece in self.pieces.iter().filter(|p| p.kind != Kind::Scenery) {
            // The far city is not part of the level.
            let reach = math::hypot(piece.size[0], piece.size[2]) / 2.0;
            bounds[0] = bounds[0].min(piece.at[0] - reach);
            bounds[1] = bounds[1].min(piece.at[2] - reach);
            bounds[2] = bounds[2].max(piece.at[0] + reach);
            bounds[3] = bounds[3].max(piece.at[2] + reach);
        }
        bounds
    }
}
