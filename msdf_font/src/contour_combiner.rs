use crate::{edge_selector::EdgeSelector, shape::Shape};
use glam::DVec2;

pub(crate) trait ContourCombiner<E: EdgeSelector>: Send + Sync {
    fn new(shape: &Shape) -> Self;
    fn reset(&mut self, p: DVec2);
    fn edge_selector(&mut self, index: usize) -> &mut E;
    fn distance(&self) -> E::Distance;
}

#[derive(Default)]
pub(crate) struct SimpleContourCombiner<E: EdgeSelector> {
    shape_edge_selector: E,
}
impl<E: EdgeSelector> ContourCombiner<E> for SimpleContourCombiner<E> {
    #[inline]
    fn new(_: &Shape) -> Self {
        Self::default()
    }

    #[inline]
    fn reset(&mut self, p: DVec2) {
        self.shape_edge_selector.reset(p);
    }

    #[inline]
    fn edge_selector(&mut self, _: usize) -> &mut E {
        &mut self.shape_edge_selector
    }

    #[inline]
    fn distance(&self) -> E::Distance {
        self.shape_edge_selector.distance()
    }
}
