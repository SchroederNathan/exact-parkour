//! Finding the top edge of a wall in front of the body, with the physics queries.
//!
//! One probe serves every move that uses an edge. A vault and a climb look for a
//! low edge from the ground, and a grab looks for a high one from the air.
use crate::body::{HALF, HEIGHT, RADIUS};
use crate::park::{PROP, SOLID};
use exact_game::*;
use exact_game_physics::{self as physics, Shape};

/// The top edge of a wall, as the body meets it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
pub struct Ledge {
    /// The point of the edge straight ahead of the body.
    pub edge: Vec3,
    /// Horizontal unit normal of the wall's face, pointing at the body.
    pub out: Vec3,
    /// How far the top runs back from the edge before it ends, metres; `DEEP` when
    /// it runs further than a vault can cross.
    pub depth: f32,
    /// Whether the body can stand on the top, just behind the edge.
    pub standable: bool,
}

/// The depth reported for a top that does not end within a vault's reach.
pub const DEEP: f32 = 1.6;
/// Spacing of the downward rays that look for a top, metres.
const STEP: f32 = 0.1;
/// A surface is a top when its normal is at least this upright.
const FLAT: f32 = 0.7;

impl Ledge {
    /// Unit vector from the body into the wall.
    pub fn inward(&self) -> Vec3 {
        -self.out
    }
    /// The capsule centre of a body that stands on the top just behind the edge.
    pub fn stand(&self) -> Vec3 {
        self.edge + self.inward() * (RADIUS + 0.15) + Vec3::Y * (HALF + 0.02)
    }
    /// The capsule centre of a body that hangs from the edge by its hands.
    pub fn hang(&self) -> Vec3 {
        self.edge + self.out * (RADIUS + 0.04) - Vec3::Y * (HANG_REACH - HALF)
    }
}

/// How far the hands of a hanging body are above its feet, metres.
pub const HANG_REACH: f32 = 2.05;

/// Look from a capsule centred at `centre` along the horizontal unit vector `dir`
/// for a wall within `reach` metres of the capsule's surface whose top is between
/// `low` and `high` metres above the feet.
pub fn probe(w: &World, centre: Vec3, dir: Vec3, reach: f32, low: f32, high: f32) -> Option<Ledge> {
    let feet = centre.y - HALF;
    let q = physics::queries(w);
    let down = |ahead: f32| {
        let from = Vec3::new(centre.x, feet + high + 0.05, centre.z) + dir * ahead;
        q.raycast(from, -Vec3::Y, high - low + 0.05, SOLID)
            .filter(|hit| hit.normal.y > FLAT && hit.distance > 0.0)
    };
    // The nearest downward ray that lands on a top in the band.
    let steps = (reach / STEP) as u32;
    let (ahead, top) = (1..=steps)
        .map(|i| RADIUS + i as f32 * STEP)
        .find_map(|ahead| down(ahead).map(|hit| (ahead, hit.point.y)))?;
    // The face of that wall, just under its top.
    let level = Vec3::new(centre.x, top - 0.05, centre.z);
    let face = q.raycast(level, dir, ahead + 0.05, SOLID)?;
    let out = Vec3::new(face.normal.x, 0.0, face.normal.z).normalize_or_zero();
    if out == Vec3::ZERO || out.dot(dir) > -0.3 {
        return None;
    }
    let edge = Vec3::new(face.point.x, top, face.point.z);
    // How far the top runs back, measured along the wall's own normal.
    let into = -out;
    let on_top = |back: f32| {
        q.raycast(edge + into * back + Vec3::Y * 0.3, -Vec3::Y, 0.5, SOLID)
            .is_some_and(|hit| (hit.point.y - top).abs() < 0.12)
    };
    let depth = (1..=(DEEP / 0.2) as u32)
        .map(|i| i as f32 * 0.2)
        .find(|&back| !on_top(back))
        .map_or(DEEP, |back| back - 0.1);
    let mut ledge = Ledge {
        edge,
        out,
        depth,
        standable: false,
    };
    drop(q);
    ledge.standable = depth >= RADIUS && clear(w, ledge.stand());
    Some(ledge)
}

/// Whether an upright capsule centred at `centre` touches nothing solid.
pub fn clear(w: &World, centre: Vec3) -> bool {
    // A hair smaller than the body, so standing on a surface is not a touch.
    let shape = Shape::Capsule {
        radius: RADIUS - 0.02,
        height: HEIGHT - 0.04,
    };
    physics::overlap(w, &shape, Transform::at(centre.x, centre.y, centre.z), SOLID | PROP).is_empty()
}

/// The height of the ground under `point`, within `drop` metres below it.
pub fn ground(w: &World, point: Vec3, drop: f32) -> Option<f32> {
    physics::raycast(w, point, -Vec3::Y, drop, SOLID)
        .filter(|hit| hit.normal.y > FLAT)
        .map(|hit| hit.point.y)
}
