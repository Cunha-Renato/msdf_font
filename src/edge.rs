//! Code taken from https://github.com/Chlumsky/msdfgen/blob/master/core/edge-segments.cpp.

use crate::{
    bounds::Bounds, distance::SignedDistance, edge_color::EdgeColor, solvers::solve_cubic,
    vec2::Vec2Ext,
};
use glam::DVec2;

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
        Self::new_line_color(p0, p1, EdgeColor::WHITE)
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
        Self::new_quad_color(p0, p1, p2, EdgeColor::WHITE)
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

    #[inline]
    pub(crate) fn point_0(&self) -> DVec2 {
        match self.etype {
            EdgeType::Line { p0, .. } | EdgeType::Quad { p0, .. } => p0,
        }
    }

    #[inline]
    pub(crate) fn point_1(&self) -> DVec2 {
        match self.etype {
            EdgeType::Line { p1, .. } => p1,
            EdgeType::Quad { p2, .. } => p2,
        }
    }

    pub(crate) fn dir_0(&self) -> DVec2 {
        match self.etype {
            EdgeType::Line { p0, p1 } => p1 - p0,
            EdgeType::Quad { p0, p1, p2 } => {
                let tan = p1 - p0;
                if tan == DVec2::ZERO { p2 - p0 } else { tan }
            }
        }
    }

    pub(crate) fn dir_1(&self) -> DVec2 {
        match self.etype {
            EdgeType::Line { p0, p1 } => p1 - p0,
            EdgeType::Quad { p0, p1, p2 } => {
                let tan = p2 - p1;
                if tan == DVec2::ZERO { p2 - p0 } else { tan }
            }
        }
    }

    pub(crate) fn sd(&self, p: DVec2, param: &mut f64) -> SignedDistance {
        match self.etype {
            EdgeType::Line { p0, p1 } => {
                let aq = p - p0;
                let ab = p1 - p0;
                let inv_ab_len_sq = 1.0 / ab.dot(ab);
                *param = aq.dot(ab) * inv_ab_len_sq;
                let eq = if *param > 0.5 { p1 } else { p0 } - p;
                let endpoint_distance = eq.length();

                if *param > 0.0 && *param < 1.0 {
                    let ortho_distance = ab.orthonormal(false, false).dot(aq);
                    if ortho_distance.abs() < endpoint_distance {
                        return SignedDistance::new(ortho_distance, 0.0);
                    }
                }

                let ab_len = ab.length();
                let cross = aq.perp_dot(ab);
                SignedDistance::new(
                    cross.signum() * endpoint_distance,
                    (ab.dot(eq) / (ab_len * endpoint_distance)).abs(),
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

                let ep_dir0 = self.dir_0();
                let inv_ep0_sq = 1.0 / ep_dir0.dot(ep_dir0);
                let qa_len = qa.length();
                let mut min_distance = ep_dir0.perp_dot(qa).signum() * qa_len;
                *param = -qa.dot(ep_dir0) * inv_ep0_sq;

                let pb = p2 - p;
                let pb_len = pb.length();
                if pb_len < min_distance.abs() {
                    let ep_dir1 = self.dir_1();
                    min_distance = ep_dir1.perp_dot(pb).signum() * pb_len;
                    *param = (p - p1).dot(ep_dir1) / ep_dir1.dot(ep_dir1);
                }

                for t in t.into_iter().take(solutions) {
                    if t > 0.0 && t < 1.0 {
                        let qe = qa + t * (2.0 * ab + t * br);
                        let distance = qe.length();
                        if distance <= min_distance.abs() {
                            min_distance = (ab + t * br).perp_dot(qe).signum() * distance;
                            *param = t;
                        }
                    }
                }

                if *param >= 0.0 && *param <= 1.0 {
                    return SignedDistance::new(min_distance, 0.0);
                }

                if *param < 0.5 {
                    let ep_dir0 = self.dir_0();
                    SignedDistance::new(
                        min_distance,
                        (ep_dir0.dot(qa) / (ep_dir0.length() * qa_len)).abs(),
                    )
                } else {
                    let ep_dir1 = self.dir_1();
                    SignedDistance::new(
                        min_distance,
                        (ep_dir1.dot(pb) / (ep_dir1.length() * pb_len)).abs(),
                    )
                }
            }
        }
    }

    pub(crate) fn bounds(&self, bounds: &mut Bounds) {
        match &self.etype {
            EdgeType::Line { p0, p1 } => {
                p0.bound_point(bounds);
                p1.bound_point(bounds);
            }
            EdgeType::Quad { p0, p1, p2 } => {
                p0.bound_point(bounds);
                p2.bound_point(bounds);

                let bot = (p1 - p0) - (p2 - p1);

                if bot.x != 0.0 {
                    let param = (p1.x - p0.x) / bot.x;

                    if param > 0.0 && param < 1.0 {
                        self.point(param).bound_point(bounds);
                    }
                }

                if bot.y != 0.0 {
                    let param = (p1.y - p0.y) / bot.y;

                    if param > 0.0 && param < 1.0 {
                        self.point(param).bound_point(bounds);
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

    #[inline]
    pub(crate) fn is_corner(&self, other: &Self, alpha: f64) -> bool {
        is_corner(self.dir_1(), other.dir_0(), alpha)
    }
}

#[inline]
fn is_corner(a: DVec2, b: DVec2, alpha: f64) -> bool {
    a.dot(b) <= 0.0 || a.perp_dot(b).abs() > alpha
}
