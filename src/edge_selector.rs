//! Code taken from https://github.com/Chlumsky/msdfgen/blob/master/core/edge-selectors.cpp.

use crate::{
    distance::{MultiDistance, SignedDistance},
    edge::Edge,
    edge_color::EdgeColor,
};
use glam::DVec2;

pub(crate) trait EdgeSelectorDistance {
    type Normalized;
    type Bytes;

    fn normalize(self, px_range: f64) -> Self::Normalized;
    fn normalize_to_bytes(self, px_range: f64) -> Self::Bytes;
    fn to_bytes(self) -> Self::Bytes;
}
impl EdgeSelectorDistance for f64 {
    type Normalized = Self;
    type Bytes = [u8; 1];

    #[inline]
    fn normalize(self, px_range: f64) -> Self::Normalized {
        (self / px_range + 0.5).clamp(0.0, 1.0)
    }

    #[inline]
    fn normalize_to_bytes(self, px_range: f64) -> Self::Bytes {
        self.normalize(px_range).to_bytes()
    }

    #[inline]
    fn to_bytes(self) -> Self::Bytes {
        [(self * 255.0).round() as u8]
    }
}
impl EdgeSelectorDistance for MultiDistance {
    type Normalized = [f64; 3];
    type Bytes = [u8; 3];

    #[inline]
    fn normalize(self, px_range: f64) -> Self::Normalized {
        [
            self.r.normalize(px_range),
            self.g.normalize(px_range),
            self.b.normalize(px_range),
        ]
    }

    #[inline]
    fn normalize_to_bytes(self, px_range: f64) -> Self::Bytes {
        [
            self.r.normalize_to_bytes(px_range)[0],
            self.g.normalize_to_bytes(px_range)[0],
            self.b.normalize_to_bytes(px_range)[0],
        ]
    }

    #[inline]
    fn to_bytes(self) -> Self::Bytes {
        [
            self.r.to_bytes()[0],
            self.g.to_bytes()[0],
            self.b.to_bytes()[0],
        ]
    }
}

pub(crate) trait EdgeSelector: Default + Clone + Send + Sync {
    type Distance: EdgeSelectorDistance;

    fn reset(&mut self, p: DVec2);

    fn add_edge(&mut self, prev: &Edge, curr: &Edge, next: &Edge);

    fn distance(&self) -> Self::Distance;
}

#[derive(Default, Clone)]
pub(crate) struct TrueDistanceSelector {
    min_distance: SignedDistance,
    p: DVec2,
}
impl EdgeSelector for TrueDistanceSelector {
    type Distance = f64;

    fn reset(&mut self, p: DVec2) {
        self.min_distance = SignedDistance::default();
        self.p = p;
    }

    fn add_edge(&mut self, _: &Edge, edge: &Edge, _: &Edge) {
        let distance = edge.sd(self.p, &mut 0.0);

        if distance < self.min_distance {
            self.min_distance = distance;
        }
    }

    #[inline]
    fn distance(&self) -> Self::Distance {
        self.min_distance.distance
    }
}

#[derive(Default, Clone)]
struct PerpendicularDistanceSelectorBase {
    near_edge: Option<Edge>,
    min_true_distance: SignedDistance,
    min_negative_perpendicular_distance: f64,
    min_positive_perpendicular_distance: f64,
    near_edge_param: f64,
}
impl PerpendicularDistanceSelectorBase {
    fn reset(&mut self) {
        self.min_true_distance = SignedDistance::default();
        self.min_negative_perpendicular_distance = f64::NEG_INFINITY;
        self.min_positive_perpendicular_distance = f64::INFINITY;
        self.near_edge = None;
        self.near_edge_param = 0.0;
    }

    fn add_true_edge_distance(&mut self, edge: Edge, distance: SignedDistance, param: f64) {
        if distance < self.min_true_distance {
            self.min_true_distance = distance;
            self.near_edge = Some(edge);
            self.near_edge_param = param;
        }
    }

    fn add_perpendicular_distance(&mut self, distance: f64) {
        if distance <= 0.0 && distance > self.min_negative_perpendicular_distance {
            self.min_negative_perpendicular_distance = distance;
        }

        if distance >= 0.0 && distance < self.min_positive_perpendicular_distance {
            self.min_positive_perpendicular_distance = distance;
        }
    }

    fn perpendicular_distance(distance: &mut f64, ep: DVec2, edge_dir: DVec2) -> bool {
        let ts = ep.dot(edge_dir);

        if ts > 0.0 {
            let perpendicular_distance = ep.perp_dot(edge_dir);

            if perpendicular_distance.abs() < distance.abs() {
                *distance = perpendicular_distance;
                return true;
            }
        }

        false
    }

    fn distance_to_perpendicular_distance(
        edge: &Edge,
        distance: &mut SignedDistance,
        origin: DVec2,
        param: f64,
    ) {
        if param < 0.0 {
            let dir = edge.dir_0().normalize();
            let aq = origin - edge.point_0();
            let ts = aq.dot(dir);

            if ts < 0.0 {
                let perpendicular_distance = aq.perp_dot(dir);
                if perpendicular_distance.abs() <= distance.distance.abs() {
                    distance.distance = perpendicular_distance;
                    distance.dot = 0.0;
                }
            }
        } else if param > 1.0 {
            let dir = edge.dir_1().normalize();
            let bq = origin - edge.point_1();
            let ts = bq.dot(dir);

            if ts > 0.0 {
                let perpendicular_distance = bq.perp_dot(dir);
                if perpendicular_distance.abs() <= distance.distance.abs() {
                    distance.distance = perpendicular_distance;
                    distance.dot = 0.0;
                }
            }
        }
    }

    fn compute_distance(&self, p: DVec2) -> f64 {
        let mut min_distance = if self.min_true_distance.distance < 0.0 {
            self.min_negative_perpendicular_distance
        } else {
            self.min_positive_perpendicular_distance
        };

        if let Some(near_edge) = self.near_edge {
            let mut distance = self.min_true_distance;
            PerpendicularDistanceSelectorBase::distance_to_perpendicular_distance(
                &near_edge,
                &mut distance,
                p,
                self.near_edge_param,
            );

            if distance.distance.abs() < min_distance.abs() {
                min_distance = distance.distance;
            }
        }

        min_distance
    }
}

#[derive(Default, Clone)]
pub(crate) struct MultiDistanceSelector {
    r: PerpendicularDistanceSelectorBase,
    g: PerpendicularDistanceSelectorBase,
    b: PerpendicularDistanceSelectorBase,
    p: DVec2,
}
impl EdgeSelector for MultiDistanceSelector {
    type Distance = MultiDistance;

    fn reset(&mut self, p: DVec2) {
        self.r.reset();
        self.g.reset();
        self.b.reset();
        self.p = p;
    }

    fn add_edge(&mut self, prev: &Edge, edge: &Edge, next: &Edge) {
        let mut param = 0.0;
        let distance = edge.sd(self.p, &mut param);

        let color = edge.color;
        let use_r = color & EdgeColor::RED != EdgeColor::BLACK;
        let use_g = color & EdgeColor::GREEN != EdgeColor::BLACK;
        let use_b = color & EdgeColor::BLUE != EdgeColor::BLACK;

        if use_r {
            self.r.add_true_edge_distance(*edge, distance, param);
        }
        if use_g {
            self.g.add_true_edge_distance(*edge, distance, param);
        }
        if use_b {
            self.b.add_true_edge_distance(*edge, distance, param);
        }

        let ap = self.p - edge.point_0();
        let bp = self.p - edge.point_1();

        let a_dir = edge.dir_0().normalize();
        let b_dir = edge.dir_1().normalize();
        let prev_dir = prev.dir_1().normalize();
        let next_dir = next.dir_0().normalize();

        let add = ap.dot(prev_dir + a_dir);
        let bdd = -bp.dot(b_dir + next_dir);

        if add > 0.0 {
            let mut pd = distance.distance;
            if PerpendicularDistanceSelectorBase::perpendicular_distance(&mut pd, ap, -a_dir) {
                pd = -pd;
                if use_r {
                    self.r.add_perpendicular_distance(pd);
                }
                if use_g {
                    self.g.add_perpendicular_distance(pd);
                }
                if use_b {
                    self.b.add_perpendicular_distance(pd);
                }
            }
        }

        if bdd > 0.0 {
            let mut pd = distance.distance;
            if PerpendicularDistanceSelectorBase::perpendicular_distance(&mut pd, bp, b_dir) {
                if use_r {
                    self.r.add_perpendicular_distance(pd);
                }
                if use_g {
                    self.g.add_perpendicular_distance(pd);
                }
                if use_b {
                    self.b.add_perpendicular_distance(pd);
                }
            }
        }
    }

    fn distance(&self) -> Self::Distance {
        MultiDistance {
            r: self.r.compute_distance(self.p),
            g: self.g.compute_distance(self.p),
            b: self.b.compute_distance(self.p),
        }
    }
}
