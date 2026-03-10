use crate::{MultiDistance, SignedDistance, Vec2Ext, edge::Edge, edge_color::EdgeColor};
use core::f64;
use glam::DVec2;

pub(crate) trait EdgeSelectorDistance {
    // fn resolve(&self) -> f64;
    fn to_bytes<F: FnOnce(&[u8])>(self, px_range: f64, f: F);
}
impl EdgeSelectorDistance for f64 {
    // #[inline]
    // fn resolve(&self) -> f64 {
    //     *self
    // }

    #[inline]
    fn to_bytes<F: FnOnce(&[u8])>(self, px_range: f64, f: F) {
        let normalized = (self / px_range + 0.5).clamp(0.0, 1.0);

        f(&[(normalized * 255.0).round() as u8]);
    }
}
impl EdgeSelectorDistance for MultiDistance {
    // #[inline]
    // fn resolve(&self) -> f64 {
    //     median(self.r, self.g, self.b)
    // }

    fn to_bytes<F: FnOnce(&[u8])>(self, px_range: f64, f: F) {
        let mut bytes = [0u8; 3];
        self.r.to_bytes(px_range, |b| bytes[0] = b[0]);
        self.g.to_bytes(px_range, |b| bytes[1] = b[0]);
        self.b.to_bytes(px_range, |b| bytes[2] = b[0]);

        f(&bytes);
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
            let perpendicular_distance = ep.cross(edge_dir);

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
            let dir = edge.dir(0.0).normalize();
            let aq = origin - edge.point(0.0);
            let ts = aq.dot(dir);

            if ts < 0.0 {
                let perpendicular_distance = aq.cross(dir);
                if perpendicular_distance.abs() <= distance.distance.abs() {
                    distance.distance = perpendicular_distance;
                    distance.dot = 0.0;
                }
            }
        } else if param > 1.0 {
            let dir = edge.dir(1.0).normalize();
            let bq = origin - edge.point(1.0);
            let ts = bq.dot(dir);

            if ts > 0.0 {
                let perpendicular_distance = bq.cross(dir);
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
    p: DVec2,
    r: PerpendicularDistanceSelectorBase,
    g: PerpendicularDistanceSelectorBase,
    b: PerpendicularDistanceSelectorBase,
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

        if edge.color & EdgeColor::RED != EdgeColor::BLACK {
            self.r.add_true_edge_distance(*edge, distance, param);
        }

        if edge.color & EdgeColor::GREEN != EdgeColor::BLACK {
            self.g.add_true_edge_distance(*edge, distance, param);
        }

        if edge.color & EdgeColor::BLUE != EdgeColor::BLACK {
            self.b.add_true_edge_distance(*edge, distance, param);
        }

        let ap = self.p - edge.point(0.0);
        let bp = self.p - edge.point(1.0);
        let a_dir = edge.dir(0.0).normalize_or_zero();
        let b_dir = edge.dir(1.0).normalize_or_zero();
        let prev_dir = prev.dir(1.0).normalize_or_zero();
        let next_dir = next.dir(0.0).normalize_or_zero();

        let add = ap.dot((prev_dir + a_dir).normalize_or_zero());
        let bdd = -bp.dot((b_dir + next_dir).normalize_or_zero());

        if add > 0.0 {
            let mut pd = distance.distance;
            if PerpendicularDistanceSelectorBase::perpendicular_distance(&mut pd, ap, -a_dir) {
                pd = -pd;

                if edge.color & EdgeColor::RED != EdgeColor::BLACK {
                    self.r.add_perpendicular_distance(pd);
                }

                if edge.color & EdgeColor::GREEN != EdgeColor::BLACK {
                    self.g.add_perpendicular_distance(pd);
                }

                if edge.color & EdgeColor::BLUE != EdgeColor::BLACK {
                    self.b.add_perpendicular_distance(pd);
                }
            }
        }

        if bdd > 0.0 {
            let mut pd = distance.distance;
            if PerpendicularDistanceSelectorBase::perpendicular_distance(&mut pd, bp, b_dir) {
                if edge.color & EdgeColor::RED != EdgeColor::BLACK {
                    self.r.add_perpendicular_distance(pd);
                }

                if edge.color & EdgeColor::GREEN != EdgeColor::BLACK {
                    self.g.add_perpendicular_distance(pd);
                }

                if edge.color & EdgeColor::BLUE != EdgeColor::BLACK {
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
