use crate::{Bounds, edge::Edge};

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
}
