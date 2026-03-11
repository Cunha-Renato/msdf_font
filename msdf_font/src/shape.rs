use crate::{
    BitmapData, Bounds, FieldType, GenerationConfig,
    contour::Contour,
    contour_combiner::{ContourCombiner, SimpleContourCombiner},
    edge::Edge,
    edge_color::EdgeColor,
    edge_selector::{
        EdgeSelector, EdgeSelectorDistance, MultiDistanceSelector, TrueDistanceSelector,
    },
    shape_distance_finder::ShapeDistanceFinder,
};
use core::f64;
use glam::DVec2;
use i_overlay::{
    core::{fill_rule::FillRule, overlay::ContourDirection},
    float::{overlay::OverlayOptions, simplify::SimplifyShape},
};
use ttf_parser::OutlineBuilder;

#[derive(Debug)]
pub(crate) struct Shape {
    pub(crate) contours: Vec<Contour>,
    position: DVec2,
    scale: f64,
}
impl Shape {
    #[inline]
    pub(crate) const fn new(scale: f64) -> Self {
        Self {
            contours: vec![],
            position: DVec2::ZERO,
            scale,
        }
    }

    pub(crate) fn bounds(&self) -> Bounds {
        let mut bounds = Bounds::new();
        for contour in &self.contours {
            contour.bounds(&mut bounds);
        }

        bounds
    }

    pub(crate) fn generate_bitmap(
        mut self,
        config: GenerationConfig,
        bitmap: &mut impl BitmapData,
    ) {
        if config.fix_geometry {
            self.resolve_shape_geometry();
        }

        match config.field_type {
            FieldType::Msdf { max_angle } => {
                self.coloring_simple(max_angle, 0);
                self.generate_msdf(config, bitmap);
            }
            FieldType::Sdf => self.generate_sdf(config, bitmap),
        }
    }

    fn generate_distance_field<E: EdgeSelector, C: ContourCombiner<E>>(
        &self,
        bitmap: &mut impl BitmapData,
        px_range: f64,
        offset: DVec2,
    ) {
        let contour_combiner = C::new(self);
        let mut shape_distance_finder = ShapeDistanceFinder::new(self, contour_combiner);
        for y in 0..bitmap.height() {
            for x in 0..bitmap.width() {
                let p =
                    DVec2::new(x as f64 + 0.5, bitmap.height() as f64 - (y as f64 + 0.5)) + offset;

                let min_dist = shape_distance_finder.distance(p);

                min_dist.to_bytes(px_range, |b| bitmap.set_px(b, x, y));
            }
        }
    }

    fn generate_sdf(&self, config: GenerationConfig, bitmap: &mut impl BitmapData) {
        self.generate_distance_field::<TrueDistanceSelector, SimpleContourCombiner<_>>(
            bitmap,
            config.px_range,
            config.offset,
        )
    }

    fn generate_msdf(&self, config: GenerationConfig, bitmap: &mut impl BitmapData) {
        self.generate_distance_field::<MultiDistanceSelector, SimpleContourCombiner<_>>(
            bitmap,
            config.px_range,
            config.offset,
        )
    }

    fn resolve_shape_geometry(&mut self) {
        if self.contours.is_empty() {
            return;
        }

        // Convert contours to i_overlay's shape format:
        // Vec<Vec<Vec<[f64; 2]>>> = Vec<Shape> where Shape = Vec<Contour>
        // Each contour is a flat list of edge start points (auto-closed)
        let shape: Vec<Vec<[f64; 2]>> = self
            .contours
            .iter()
            .filter_map(|contour| {
                if contour.edges.is_empty() {
                    return None;
                }

                let pts: Vec<[f64; 2]> = contour
                    .edges
                    .iter()
                    .flat_map(Edge::as_lines)
                    .map(|p| [p.x, p.y])
                    .collect();

                if pts.len() < 3 {
                    return None;
                }

                Some(pts)
            })
            .collect();

        if shape.is_empty() {
            return;
        }

        // simplify() resolves self-intersections under the NonZero fill rule
        // and returns clean non-overlapping contours
        let result: Vec<Vec<Vec<[f64; 2]>>> = shape.simplify_shape_custom(
            FillRule::NonZero,
            OverlayOptions {
                output_direction: ContourDirection::Clockwise,
                ..Default::default()
            },
            Default::default(),
        );

        if result.is_empty() {
            return;
        }

        self.contours.clear();

        for shape in result {
            for poly in shape {
                if poly.len() < 2 {
                    continue;
                }

                let mut contour = Contour::default();
                let n = poly.len();
                
                for i in 0..n {
                    let p0 = DVec2::new(poly[i][0], poly[i][1]);
                    let p1 = DVec2::new(poly[(i + 1) % n][0], poly[(i + 1) % n][1]);

                    if p0 != p1 {
                        contour.edges.push(Edge::new_line(p0, p1));
                    }
                }

                if !contour.edges.is_empty() {
                    self.contours.push(contour);
                }
            }
        }
    }

    fn coloring_simple(&mut self, alpha: f64, mut seed: usize) {
        let sin_alpha = alpha.sin();

        for contour in &mut self.contours {
            let mut corners = Vec::new();

            if let Some(last_edge) = contour.edges.last() {
                if last_edge.is_corner(&contour.edges[0], sin_alpha) {
                    corners.push(0);
                }

                for i in 0..(contour.edges.len() - 1) {
                    if contour.edges[i].is_corner(&contour.edges[i + 1], sin_alpha) {
                        corners.push(i + 1);
                    }
                }
            }

            let s = corners.first().copied().unwrap_or(0);
            if s != 0 {
                contour.edges.rotate_left(s);
                for c in &mut corners {
                    *c -= s;
                }
            }

            if corners.len() == 1 {
                let color = EdgeColor::WHITE.switch(&mut seed, EdgeColor::BLACK);
                let color2 = color.switch(&mut seed, EdgeColor::BLACK);

                let colors = [color, EdgeColor::WHITE, color2];

                match contour.edges.len() {
                    0 => (),
                    1 => {
                        let split = contour.edges[0].split_in_thirds();
                        contour.edges = split
                            .into_iter()
                            .zip(colors)
                            .map(|(mut edge, color)| {
                                edge.color = color;
                                edge
                            })
                            .collect();
                    }
                    2 => {
                        let split_0 = contour.edges[0].split_in_thirds();
                        let split_1 = contour.edges[1].split_in_thirds();
                        contour.edges = split_0
                            .into_iter()
                            .chain(split_1)
                            .enumerate()
                            .map(|(i, mut edge)| {
                                edge.color = colors[i / 2];
                                edge
                            })
                            .collect();
                    }
                    _ => {
                        let num_edge = contour.edges.len();

                        for (i, edge) in contour.edges.iter_mut().enumerate() {
                            // WTF is this?
                            let index = (num_edge - 1 + 46 * i) / (16 * (num_edge - 1));
                            edge.color = colors[index];
                        }
                    }
                }
            } else if !contour.edges.is_empty() {
                let mut spline = 0;
                let mut color = EdgeColor::WHITE.switch(&mut seed, EdgeColor::BLACK);
                let initial_color = color;

                for (i, edge) in contour.edges.iter_mut().enumerate() {
                    if corners.get(spline + 1) == Some(&i) {
                        spline += 1;
                        color = color.switch(
                            &mut seed,
                            if spline == corners.len() - 1 {
                                initial_color
                            } else {
                                EdgeColor::BLACK
                            },
                        )
                    }
                    edge.color = color;
                }
            }
        }
    }

    #[inline]
    fn scale_point(&self, x: f32, y: f32) -> DVec2 {
        DVec2::new(f64::from(x), f64::from(y)) * self.scale
    }
}
impl OutlineBuilder for Shape {
    fn move_to(&mut self, x: f32, y: f32) {
        self.contours.push(Contour::default());
        self.position = self.scale_point(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let endpoint = self.scale_point(x, y);

        if endpoint != self.position {
            if let Some(last) = self.contours.last_mut() {
                last.edges.push(Edge::new_line(self.position, endpoint));
            }

            self.position = endpoint;
        }
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let endpoint = self.scale_point(x, y);
        let control = self.scale_point(x1, y1);

        if endpoint != self.position {
            if let Some(last) = self.contours.last_mut() {
                last.edges
                    .push(Edge::new_quad(self.position, control, endpoint));
            }

            self.position = endpoint;
        }
    }

    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
        todo!();
    }

    fn close(&mut self) {}
}
