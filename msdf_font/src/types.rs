use crate::{Vec2Ext, bound_point, solvers::solve_cubic};
use core::f64;
use glam::DVec2;
use std::cmp::Ordering;

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub enum FieldType {
    Msdf(f32),
    #[default]
    Sdf,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GenerationConfig {
    pub(crate) px_range: f64,
    pub(crate) offset: DVec2,
    pub(crate) bitmap_size: (usize, usize),
    pub(crate) overlapping: bool,
    pub(crate) field_type: FieldType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SignedDistance {
    pub(crate) distance: f64,
    pub(crate) dot: f64,
}
impl Default for SignedDistance {
    fn default() -> Self {
        Self {
            distance: f64::INFINITY,
            dot: f64::NEG_INFINITY,
        }
    }
}
impl SignedDistance {
    #[inline]
    const fn new(distance: f64, dot: f64) -> Self {
        Self { distance, dot }
    }
}
impl PartialOrd for SignedDistance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = self.distance.abs();
        let b = other.distance.abs();

        match a.partial_cmp(&b)? {
            Ordering::Equal => self.dot.partial_cmp(&other.dot),
            ord => Some(ord),
        }
    }
}

pub(crate) struct MultiDistance {
    pub(crate) r: f64,
    pub(crate) g: f64,
    pub(crate) b: f64,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EdgeColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
}
impl EdgeColor {
    #[inline]
    pub(crate) const fn from_u8(value: u8) -> Self {
        match value & 0b111 {
            0 => Self::Black,
            1 => Self::Red,
            2 => Self::Green,
            3 => Self::Yellow,
            4 => Self::Blue,
            5 => Self::Magenta,
            6 => Self::Cyan,
            _ => Self::White,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum EdgeType {
    Line { p0: DVec2, p1: DVec2 },
    Quad { p0: DVec2, p1: DVec2, p2: DVec2 },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Edge {
    pub(crate) etype: EdgeType,
    pub(crate) color: EdgeColor,
}
impl Edge {
    #[inline]
    pub(crate) const fn new_line(p0: DVec2, p1: DVec2) -> Self {
        Self {
            etype: EdgeType::Line { p0, p1 },
            color: EdgeColor::White,
        }
    }

    #[inline]
    pub(crate) const fn new_quad(p0: DVec2, p1: DVec2, p2: DVec2) -> Self {
        Self {
            etype: EdgeType::Quad { p0, p1, p2 },
            color: EdgeColor::White,
        }
    }

    pub(crate) fn point(&self, param: f64) -> DVec2 {
        match self.etype {
            EdgeType::Line { p0, p1 } => p0.lerp(p1, param),
            EdgeType::Quad { p0, p1, p2 } => p0.lerp(p1, param).lerp(p1.lerp(p2, param), param),
        }
    }

    pub(crate) fn dir(&self, param: f64) -> DVec2 {
        match self.etype {
            EdgeType::Line { p0, p1 } => p1 - p0,
            EdgeType::Quad { p0, p1, p2 } => {
                let tan = (p1 - p0).lerp(p2 - p1, param);
                if tan == DVec2::ZERO { p2 - p1 } else { tan }
            }
        }
    }

    pub(crate) fn sd(&self, p: DVec2, param: &mut f64) -> SignedDistance {
        match self.etype {
            EdgeType::Line { p0, p1 } => {
                let aq = p - p0;
                let ab = p1 - p0;
                *param = aq.dot(ab) / ab.dot(ab);
                let eq = if *param > 0.5 { p1 } else { p0 } - p;

                let enpoint_distance = eq.length();

                if *param > 0.0 && *param < 1.0 {
                    let ortho_distance = ab.orthonormal(false, false).dot(aq);

                    if ortho_distance.abs() < enpoint_distance {
                        return SignedDistance::new(ortho_distance, 0.0);
                    }
                }

                SignedDistance::new(
                    aq.cross(ab).signum() * enpoint_distance,
                    ab.normalize().dot(eq.normalize()).abs(),
                )
            }
            EdgeType::Quad { p0, p1, p2 } => {
                let qa = p0 - p;
                let ab = p1 - p0;
                let br = p2 - p1 - ab;
                let a = br.dot(br);
                let b = 3.0 * ab.dot(br);
                let c = 2.0f64.mul_add(ab.dot(ab), qa.dot(br));
                let d = qa.dot(ab);
                let mut t = [0.0; 3];
                let solutions = solve_cubic(&mut t, a, b, c, d);

                let mut ep_dir = self.dir(0.0);
                let mut min_distance = (ep_dir.cross(qa)).signum() * qa.length(); // distance from A
                *param = -qa.dot(ep_dir) / ep_dir.dot(ep_dir);
                let distance = (p2 - p).length(); // distance from B
                if distance < min_distance.abs() {
                    ep_dir = self.dir(1.0);
                    min_distance = (ep_dir.cross(p2 - p)).signum() * distance;
                    *param = (p - p1).dot(ep_dir) / ep_dir.dot(ep_dir);
                }
                for t in t.into_iter().take(solutions) {
                    if t > 0.0 && t < 1.0 {
                        let qe = qa + 2.0 * t * ab + t * t * br;
                        let distance = qe.length();
                        if distance <= min_distance.abs() {
                            min_distance = (ab + t * br).cross(qe).signum() * distance;
                            *param = t;
                        }
                    }
                }

                if *param >= 0.0 && *param <= 1.0 {
                    return SignedDistance::new(min_distance, 0.0);
                }
                if *param < 0.5 {
                    SignedDistance::new(
                        min_distance,
                        self.dir(0.0).normalize().dot(qa.normalize()).abs(),
                    )
                } else {
                    SignedDistance::new(
                        min_distance,
                        self.dir(1.0).normalize().dot((p2 - p).normalize()).abs(),
                    )
                }
            }
        }
    }

    fn bounds(&self, bounds: &mut Bounds) {
        match self.etype {
            EdgeType::Line { p0, p1 } => {
                bound_point(p0, bounds);
                bound_point(p1, bounds);
            }
            EdgeType::Quad { p0, p1, p2 } => {
                bound_point(p0, bounds);
                bound_point(p2, bounds);

                let bot = (p1 - p0) - (p2 - p1);

                if bot.x != 0.0 {
                    let param = (p1.x - p0.x) / bot.x;

                    if param > 0.0 && param < 1.0 {
                        bound_point(self.point(param), bounds);
                    }
                }

                if bot.y != 0.0 {
                    let param = (p1.y - p0.y) / bot.y;

                    if param > 0.0 && param < 1.0 {
                        bound_point(self.point(param), bounds);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Bounds {
    pub(crate) min: DVec2,
    pub(crate) max: DVec2,
}
impl Bounds {
    #[inline]
    pub(crate) const fn new() -> Self {
        Self {
            min: DVec2::new(f64::INFINITY, f64::INFINITY),
            max: DVec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    #[inline]
    pub(crate) const fn size(self) -> DVec2 {
        DVec2::new(self.max.x - self.min.x, self.max.y - self.min.y)
    }
}

#[derive(Debug, Default)]
pub(crate) struct Contour {
    pub(crate) edges: Vec<Edge>,
}
impl Contour {
    #[inline]
    pub(crate) fn bounds(&self, bounds: &mut Bounds) {
        for edge in &self.edges {
            edge.bounds(bounds);
        }
    }

    pub(crate) fn winding(&self) -> i32 {
        let mut total = 0.0;

        match self.edges.len() {
            0 => return 0,
            1 => {
                let a = self.edges[0].point(0.0);
                let b = self.edges[0].point(1.0 / 3.0);
                let c = self.edges[0].point(2.0 / 3.0);

                total += Self::shoelace(a, b) + Self::shoelace(b, c) + Self::shoelace(c, a);
            }
            2 => {
                let a = self.edges[0].point(0.0);
                let b = self.edges[0].point(0.5);
                let c = self.edges[1].point(0.0);
                let d = self.edges[1].point(0.5);

                total += Self::shoelace(a, b)
                    + Self::shoelace(b, c)
                    + Self::shoelace(c, d)
                    + Self::shoelace(d, a);
            }
            _ => {
                if let Some(prev_e) = self.edges.last() {
                    let mut prev = prev_e.point(0.0);

                    for edge in &self.edges {
                        let cur = edge.point(0.0);
                        total += Self::shoelace(prev, cur);
                        prev = cur;
                    }
                }
            }
        }

        total.signum() as i32
    }

    #[inline]
    const fn shoelace(a: DVec2, b: DVec2) -> f64 {
        (b.x - a.x) * (a.y + b.y)
    }
}
