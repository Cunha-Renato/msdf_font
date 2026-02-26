use crate::{Bounds, edge::Edge};
use glam::DVec2;

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
