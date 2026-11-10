//! The far city: buildings around a level that are drawn and never touched. They
//! come from the level's own bounds, so every run of a level has the same skyline.
use super::kit::{Kind, Level, Look, Piece};
use exact_game::math;

/// The side of one city lot, how far the city reaches past the level, and how far
/// from the level's pieces the first buildings stand, metres.
const LOT: f32 = 26.0;
const REACH: f32 = 120.0;
const CLEAR: f32 = 14.0;

// A number from 0 to 1 that depends only on the lot and `salt`.
fn chance(column: i32, row: i32, salt: u32) -> f32 {
    let mut h = (column as u32).wrapping_mul(0x9E37_79B1)
        ^ (row as u32).wrapping_mul(0x85EB_CA77)
        ^ salt.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65535.0
}

/// The buildings of the far city around `level`.
pub fn around(level: &Level) -> Vec<Piece> {
    let [west, north, east, south] = level.bounds();
    if west > east {
        return Vec::new();
    }
    // The city is about as tall as the level's own roofs.
    let roofs: Vec<f32> = level
        .pieces
        .iter()
        .filter(|p| p.kind == Kind::Solid && p.bottom() < 0.5)
        .map(|p| p.top())
        .collect();
    let typical = if roofs.is_empty() {
        20.0
    } else {
        roofs.iter().sum::<f32>() / roofs.len() as f32
    };
    let first_column = math::floor((west - REACH) / LOT) as i32;
    let last_column = math::ceil((east + REACH) / LOT) as i32;
    let first_row = math::floor((north - REACH) / LOT) as i32;
    let last_row = math::ceil((south + REACH) / LOT) as i32;
    let mut city = Vec::new();
    for row in first_row..=last_row {
        for column in first_column..=last_column {
            if chance(column, row, 1) < 0.3 {
                continue;
            }
            let width = 11.0 + 9.0 * chance(column, row, 2);
            let depth = 11.0 + 9.0 * chance(column, row, 3);
            let x = (column as f32 + 0.5) * LOT + (chance(column, row, 4) - 0.5) * (LOT - width);
            let z = (row as f32 + 0.5) * LOT + (chance(column, row, 5) - 0.5) * (LOT - depth);
            let inside = x + width / 2.0 > west - CLEAR
                && x - width / 2.0 < east + CLEAR
                && z + depth / 2.0 > north - CLEAR
                && z - depth / 2.0 < south + CLEAR;
            if inside {
                continue;
            }
            let roof = typical * (0.45 + 1.1 * chance(column, row, 6) * chance(column, row, 7));
            let mut piece = Piece::new(
                &format!("city-{column}-{row}"),
                [x, roof / 2.0, z],
                [width, roof, depth],
                Look::Haze,
            );
            piece.kind = Kind::Scenery;
            city.push(piece);
        }
    }
    city
}
