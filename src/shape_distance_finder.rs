//! Code taken from https://github.com/Chlumsky/msdfgen/blob/master/core/ShapeDistanceFinder.hpp.

use crate::{edge_selector::EdgeSelector, shape::Shape};
use glam::DVec2;

pub(crate) struct ShapeDistanceFinder<'a, E: EdgeSelector> {
    shape: &'a Shape,
    edge_selector: E,
}
impl<'a, E: EdgeSelector> ShapeDistanceFinder<'a, E> {
    pub(crate) fn new(shape: &'a Shape) -> Self {
        Self {
            shape,
            edge_selector: E::default(),
        }
    }

    pub(crate) fn distance(&mut self, p: DVec2) -> E::Distance {
        self.edge_selector.reset(p);

        for contour in &self.shape.contours {
            let len = contour.edges.len();

            if len == 0 {
                continue;
            }

            for i in 0..len {
                let prev = &contour.edges[(i + len - 1) % len];
                let curr = &contour.edges[i];
                let next = &contour.edges[(i + 1) % len];

                self.edge_selector.add_edge(prev, curr, next);
            }
        }

        self.edge_selector.distance()
    }
}
