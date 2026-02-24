use crate::{contour_combiner::ContourCombiner, edge_selector::EdgeSelector, shape::Shape};
use glam::DVec2;
use std::marker::PhantomData;

pub(crate) struct ShapeDistanceFinder<E: EdgeSelector, C: ContourCombiner<E>> {
    shape: Shape,
    combiner: C,
    _p: PhantomData<E>,
}
impl<E: EdgeSelector, C: ContourCombiner<E>> ShapeDistanceFinder<E, C> {
    pub(crate) fn new(shape: Shape, combiner: C) -> Self {
        Self {
            shape,
            combiner,
            _p: PhantomData,
        }
    }

    pub(crate) fn distance(&mut self, p: DVec2) -> E::Distance {
        self.combiner.reset(p);

        for (i, contour) in self.shape.contours.iter().enumerate() {
            let len = contour.edges.len();
            if len == 0 {
                continue;
            }

            let selector = self.combiner.edge_selector(i);

            for i in 0..len {
                let prev = &contour.edges[(i + len - 1) % len];
                let curr = &contour.edges[i];
                let next = &contour.edges[(i + 1) % len];

                selector.add_edge(prev, curr, next);
            }
        }

        self.combiner.distance()
    }
}
