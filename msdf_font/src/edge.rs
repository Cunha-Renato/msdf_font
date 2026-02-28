use crate::{Bounds, SignedDistance, Vec2Ext, bound_point, solvers::solve_cubic};
use glam::DVec2;

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
        Self::new_line_color(p0, p1, EdgeColor::White)
    }

    #[inline]
    pub(crate) const fn new_line_color(p0: DVec2, p1: DVec2, color: EdgeColor) -> Self {
        Self {
            etype: EdgeType::Line { p0, p1 },
            color,
        }
    }

    #[inline]
    pub(crate) const fn new_quad(p0: DVec2, p1: DVec2, p2: DVec2) -> Self {
        Self::new_quad_color(p0, p1, p2, EdgeColor::White)
    }

    #[inline]
    pub(crate) const fn new_quad_color(p0: DVec2, p1: DVec2, p2: DVec2, color: EdgeColor) -> Self {
        Self {
            etype: EdgeType::Quad { p0, p1, p2 },
            color,
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

    pub(crate) fn bounds(&self, bounds: &mut Bounds) {
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

    pub(crate) fn split_in_thirds(&self) -> [Self; 3] {
        const THIRD: f64 = 1.0 / 3.0;
        const TWO_THIRDS: f64 = 2.0 / 3.0;
        let pt = self.point(THIRD);
        let ptt = self.point(TWO_THIRDS);

        match self.etype {
            EdgeType::Line { p0, p1 } => [
                Self::new_line_color(p0, pt, self.color),
                Self::new_line_color(pt, ptt, self.color),
                Self::new_line_color(ptt, p1, self.color),
            ],
            EdgeType::Quad { p0, p1, p2 } => [
                Self::new_quad_color(p0, p0.lerp(p1, THIRD), pt, self.color),
                Self::new_quad_color(
                    pt,
                    p0.lerp(p1, 5.0 / 9.0).lerp(p1.lerp(p2, 4.0 / 9.0), 0.5),
                    ptt,
                    self.color,
                ),
                Self::new_quad_color(ptt, p1.lerp(p2, TWO_THIRDS), p2, self.color),
            ],
        }
    }
}
