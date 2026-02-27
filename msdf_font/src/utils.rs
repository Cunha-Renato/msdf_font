use crate::{
    Bounds,
    edge::{Edge, EdgeType},
};
use glam::DVec2;

pub(crate) trait Vec2Ext {
    fn cross(self, rhs: Self) -> f64;

    fn orthonormal(self, polarity: bool, allow_zero: bool) -> Self;
}
impl Vec2Ext for DVec2 {
    #[inline]
    fn cross(self, b: Self) -> f64 {
        self.x.mul_add(b.y, -(self.y * b.x))
    }

    fn orthonormal(self, polarity: bool, allow_zero: bool) -> Self {
        let len = self.length();

        if len.abs() > f64::EPSILON {
            let inv = 1.0 / len;

            if polarity {
                Self::new(-self.y * inv, self.x * inv)
            } else {
                Self::new(self.y * inv, -self.x * inv)
            }
        } else if polarity {
            Self::new(0.0, if allow_zero { 0.0 } else { 1.0 })
        } else {
            Self::new(0.0, if allow_zero { 0.0 } else { -1.0 })
        }
    }
}

#[inline]
pub(crate) const fn bound_point(p: DVec2, bounds: &mut Bounds) {
    bounds.min.x = bounds.min.x.min(p.x);
    bounds.min.y = bounds.min.y.min(p.y);
    bounds.max.x = bounds.max.x.max(p.x);
    bounds.max.y = bounds.max.y.max(p.y);
}

#[inline]
pub(crate) const fn median(a: f64, b: f64, c: f64) -> f64 {
    a.min(b).max(a.max(b).min(c))
}

pub(crate) fn flatten_edge(edge: &Edge) -> Vec<[f64; 2]> {
    match edge.etype {
        EdgeType::Line { p0, .. } => vec![[p0.x, p0.y]],
        EdgeType::Quad { p0, p1, p2 } => {
            let steps = quad_flatten_steps(p0, p1, p2);

            (0..steps)
                .map(|i| {
                    let t = i as f64 / steps as f64;
                    let p = edge.point(t);

                    [p.x, p.y]
                })
                .collect()
        }
    }
}

fn quad_flatten_steps(p0: DVec2, p1: DVec2, p2: DVec2) -> usize {
    const TOLERANCE: f64 = 0.1;
    const MIN_STEPS: usize = 4;
    const MAX_STEPS: usize = 32;

    // Max deviation of a quadratic bezier from a straight line is:
    // deviation = 0.25 * |p1 - 0.5*(p0+p2)|
    let mid = (p0 + p2) * 0.5;
    let deviation = (p1 - mid).length() * 0.25;

    if deviation <= TOLERANCE {
        return MIN_STEPS;
    }

    // Number of steps needed so that each chord is within tolerance:
    // steps = sqrt(deviation / tolerance)  (from standard subdivision formula)
    let steps = (deviation / TOLERANCE).sqrt().ceil() as usize;
    steps.clamp(MIN_STEPS, MAX_STEPS)
}
