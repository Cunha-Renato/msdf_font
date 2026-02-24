use glam::DVec2;

use crate::{
    Contour,
    edge_selector::{EdgeSelector, EdgeSelectorDistance},
    shape::Shape,
};

pub(crate) trait ContourCombiner<E: EdgeSelector> {
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

pub(crate) struct OverlappingContourCombiner<E: EdgeSelector> {
    edge_selectors: Vec<E>,
    windings: Vec<i32>,
    p: DVec2,
}
impl<E: EdgeSelector> ContourCombiner<E> for OverlappingContourCombiner<E> {
    fn new(shape: &Shape) -> Self {
        let windings = shape
            .contours
            .iter()
            .map(Contour::winding)
            .collect::<Vec<_>>();

        Self {
            edge_selectors: vec![E::default(); shape.contours.len()],
            windings,
            p: DVec2::default(),
        }
    }

    fn reset(&mut self, p: DVec2) {
        self.p = p;

        self.edge_selectors.iter_mut().for_each(|es| es.reset(p));
    }

    #[inline]
    fn edge_selector(&mut self, index: usize) -> &mut E {
        &mut self.edge_selectors[index]
    }

    fn distance(&self) -> E::Distance {
        let contour_count = self.edge_selectors.len();

        let mut shape_edge_selector = E::default();
        let mut inner_edge_selector = E::default();
        let mut outer_edge_selector = E::default();

        shape_edge_selector.reset(self.p);
        inner_edge_selector.reset(self.p);
        outer_edge_selector.reset(self.p);

        for i in 0..contour_count {
            let edge_distance = self.edge_selectors[i].distance();

            shape_edge_selector.merge(&self.edge_selectors[i]);

            if self.windings[i] > 0 && edge_distance.resolve() >= 0.0 {
                inner_edge_selector.merge(&self.edge_selectors[i]);
            }

            if self.windings[i] < 0 && edge_distance.resolve() <= 0.0 {
                outer_edge_selector.merge(&self.edge_selectors[i]);
            }
        }

        let shape_distance = shape_edge_selector.distance();
        let inner_distance = inner_edge_selector.distance();
        let outer_distance = outer_edge_selector.distance();

        let inner_scalar_distance = inner_distance.resolve();
        let outer_scalar_distance = outer_distance.resolve();
        let mut distance;
        let winding;

        if inner_scalar_distance >= 0.0 && inner_scalar_distance.abs() < outer_scalar_distance.abs()
        {
            distance = inner_distance;
            winding = 1;

            for i in 0..contour_count {
                if self.windings[i] > 0 {
                    let contour_distance = self.edge_selectors[i].distance();

                    if contour_distance.resolve().abs() < outer_scalar_distance.abs()
                        && contour_distance.resolve() > distance.resolve()
                    {
                        distance = contour_distance;
                    }
                }
            }
        } else if outer_scalar_distance <= 0.0
            && outer_scalar_distance.abs() < inner_scalar_distance.abs()
        {
            distance = outer_distance;
            winding = -1;

            for i in 0..contour_count {
                if self.windings[i] < 0 {
                    let contour_distance = self.edge_selectors[i].distance();

                    if contour_distance.resolve().abs() < inner_scalar_distance.abs()
                        && contour_distance.resolve() < distance.resolve()
                    {
                        distance = contour_distance;
                    }
                }
            }
        } else {
            return shape_distance;
        }

        for i in 0..contour_count {
            if self.windings[i] != winding {
                let contour_distance = self.edge_selectors[i].distance();

                if contour_distance.resolve() * distance.resolve() >= 0.0
                    && contour_distance.resolve().abs() < distance.resolve().abs()
                {
                    distance = contour_distance;
                }
            }
        }

        if (distance.resolve() - shape_distance.resolve()).abs() < f64::EPSILON {
            distance = shape_distance;
        }

        distance
    }
}
