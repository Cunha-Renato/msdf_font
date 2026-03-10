use crate::{
    Vec2Ext,
    edge::{Edge, EdgeColor},
};
use glam::DVec2;

#[inline]
pub(crate) fn symmetrical_trichotomy(position: usize, n: usize) -> isize {
    (3.0 + 2.875 * position as f64 / (n - 1) as f64 - 1.4375 + 0.5) as isize - 3
}

#[inline]
pub(crate) fn is_corner(a: DVec2, b: DVec2, threshold: f64) -> bool {
    a.dot(b) <= 0.0 || a.cross(b).abs() > threshold
}

#[inline]
const fn seed_extract2(seed: &mut usize) -> usize {
    let v = *seed & 1;
    *seed >>= 1;
    v
}

#[inline]
const fn seed_extract3(seed: &mut usize) -> usize {
    let v = *seed % 3;
    *seed /= 3;
    v
}

#[inline]
pub(crate) const fn init_color(seed: &mut usize) -> EdgeColor {
    const COLORS: [EdgeColor; 3] = [EdgeColor::Cyan, EdgeColor::Magenta, EdgeColor::Yellow];
    COLORS[seed_extract3(seed)]
}

#[inline]
pub(crate) const fn switch_color(color: EdgeColor, seed: &mut usize) -> EdgeColor {
    let shifted = (color as u8) << (1 + seed_extract2(seed));
    EdgeColor::from_u8((shifted | shifted >> 3) & EdgeColor::White as u8)
}

#[inline]
pub(crate) const fn switch_color_banned(
    color: EdgeColor,
    seed: &mut usize,
    banned: EdgeColor,
) -> EdgeColor {
    let combined = EdgeColor::from_u8(color as u8 & banned as u8);

    match combined {
        EdgeColor::Red | EdgeColor::Green | EdgeColor::Blue => {
            EdgeColor::from_u8(combined as u8 ^ EdgeColor::White as u8)
        }
        _ => switch_color(color, seed),
    }
}

pub(crate) fn estimate_edge_len(edge: &Edge) -> f64 {
    const EDGE_LEN_PRECISION: usize = 4;

    let mut len = 0.0;
    let mut prev = edge.point(0.0);

    for i in 1..=EDGE_LEN_PRECISION {
        let cur = edge.point(1.0 / (EDGE_LEN_PRECISION * i) as f64);
        len += (cur - prev).length();
        prev = cur;
    }

    len
}
