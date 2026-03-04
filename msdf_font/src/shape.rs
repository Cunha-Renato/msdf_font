use crate::{
    BitmapData, Bounds, FieldType, GenerationConfig,
    contour::Contour,
    contour_combiner::{ContourCombiner, OverlappingContourCombiner, SimpleContourCombiner},
    edge::{Edge, EdgeColor},
    edge_coloring::{
        init_color, is_corner, switch_color, switch_color_banned, symmetrical_trichotomy,
    },
    edge_selector::{
        EdgeSelector, EdgeSelectorDistance, MultiDistanceSelector, TrueDistanceSelector,
    },
    shape_distance_finder::ShapeDistanceFinder,
    utils::flatten_edge,
};
use core::f64;
use glam::DVec2;
use i_overlay::{
    core::{fill_rule::FillRule, overlay::ContourDirection},
    float::{overlay::OverlayOptions, simplify::SimplifyShape},
};
#[cfg(feature = "rayon")]
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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

        let func = match config.field_type {
            FieldType::Msdf(max_angle) => {
                self.simple_coloring(f64::from(max_angle), 0);
                Self::generate_msdf
            }
            FieldType::Sdf => Self::generate_sdf,
        };

        func(self, config, bitmap);
    }

    fn simple_coloring(&mut self, angle_treshold: f64, mut seed: usize) {
        let cross_treshold = angle_treshold.sin();
        let mut color = init_color(&mut seed);
        let mut corners = vec![];

        for contour in &mut self.contours {
            if contour.edges.is_empty() {
                continue;
            }

            corners.clear();
            if let Some(last) = contour.edges.last() {
                let mut prev_dir = last.dir(1.0);

                for (i, edge) in contour.edges.iter().enumerate() {
                    if is_corner(
                        prev_dir.normalize(),
                        edge.dir(0.0).normalize(),
                        cross_treshold,
                    ) {
                        corners.push(i);
                    }

                    prev_dir = edge.dir(1.0);
                }
            }

            if corners.is_empty() {
                color = switch_color(color, &mut seed);

                for edge in &mut contour.edges {
                    edge.color = color;
                }
            } else if corners.len() == 1 {
                color = switch_color(color, &mut seed);
                let colors = [color, EdgeColor::White, switch_color(color, &mut seed)];
                color = colors[2];

                let corner = corners[0];

                if contour.edges.len() >= 3 {
                    let m = contour.edges.len();
                    for i in 0..m {
                        contour.edges[(corner + i) % m].color =
                            colors[1 + symmetrical_trichotomy(i, m)];
                    }
                } else if !contour.edges.is_empty() {
                    let first_thirds = contour.edges[0].split_in_thirds();
                    let mut parts: [Edge; 6];

                    if contour.edges.len() >= 2 {
                        let second_thirds = contour.edges[1].split_in_thirds();

                        parts = if corner == 0 {
                            [
                                first_thirds[0],  // color[0]
                                first_thirds[1],  // color[0]
                                first_thirds[2],  // color[1]
                                second_thirds[0], // color[1]
                                second_thirds[1], // color[2]
                                second_thirds[2], // color[2]
                            ]
                        } else {
                            [
                                second_thirds[0], // color[0]
                                second_thirds[1], // color[0]
                                second_thirds[2], // color[1]
                                first_thirds[0],  // color[1]
                                first_thirds[1],  // color[2]
                                first_thirds[2],  // color[2]
                            ]
                        };

                        parts[0].color = colors[0];
                        parts[1].color = colors[0];
                        parts[2].color = colors[1];
                        parts[3].color = colors[1];
                        parts[4].color = colors[2];
                        parts[5].color = colors[2];

                        contour.edges = parts.into();
                    } else {
                        let [mut a, mut b, mut c] = first_thirds;
                        a.color = colors[0];
                        b.color = colors[1];
                        c.color = colors[2];

                        contour.edges = vec![a, b, c];
                    }
                }
            } else {
                let corner_count = corners.len();
                let mut spline = 0;
                let start = corners[0];
                let m = contour.edges.len();
                color = switch_color(color, &mut seed);
                let initial_color = color;

                for i in 0..m {
                    let index = (start + i) % m;
                    if spline + 1 < corner_count && corners[spline + 1] == index {
                        spline += 1;
                        color = switch_color_banned(
                            color,
                            &mut seed,
                            EdgeColor::from_u8(if spline == corner_count - 1 {
                                initial_color as u8
                            } else {
                                0
                            }),
                        );
                    }
                    contour.edges[index].color = color;
                }
            }
        }
    }

    fn generate_distance_field<E: EdgeSelector, C: ContourCombiner<E>>(
        self,
        bitmap: &mut impl BitmapData,
        px_range: f64,
        offset: DVec2,
    ) {
        let contour_combiner = C::new(&self);
        let mut shape_distance_finder = ShapeDistanceFinder::new(&self, contour_combiner);
        for y in 0..bitmap.height() {
            for x in 0..bitmap.width() {
                let p =
                    DVec2::new(x as f64 + 0.5, bitmap.height() as f64 - (y as f64 + 0.5)) + offset;

                let min_dist = shape_distance_finder.distance(p);

                min_dist.to_bytes(px_range, |b| bitmap.set_px(b, x, y));
            }
        }
    }

    fn generate_sdf(self, config: GenerationConfig, bitmap: &mut impl BitmapData) {
        if config.overlapping {
            self.generate_distance_field::<TrueDistanceSelector, OverlappingContourCombiner<_>>(
                bitmap,
                config.px_range,
                config.offset,
            )
        } else {
            self.generate_distance_field::<TrueDistanceSelector, SimpleContourCombiner<_>>(
                bitmap,
                config.px_range,
                config.offset,
            )
        }
    }

    fn generate_msdf(self, config: GenerationConfig, bitmap: &mut impl BitmapData) {
        if config.overlapping {
            self.generate_distance_field::<MultiDistanceSelector, OverlappingContourCombiner<_>>(
                bitmap,
                config.px_range,
                config.offset,
            )
        } else {
            self.generate_distance_field::<MultiDistanceSelector, SimpleContourCombiner<_>>(
                bitmap,
                config.px_range,
                config.offset,
            )
        }
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

                #[cfg(feature = "rayon")]
                let pts: Vec<[f64; 2]> = contour.edges.par_iter().flat_map(flatten_edge).collect();
                #[cfg(not(feature = "rayon"))]
                let pts: Vec<[f64; 2]> = contour.edges.iter().flat_map(flatten_edge).collect();

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

                let mut contour = crate::contour::Contour::default();
                let n = poly.len();

                for i in 0..n {
                    let p0 = DVec2::new(poly[i][0], poly[i][1]);
                    let p1 = DVec2::new(poly[(i + 1) % n][0], poly[(i + 1) % n][1]);

                    if p0 != p1 {
                        contour.edges.push(crate::edge::Edge::new_line(p0, p1));
                    }
                }

                if !contour.edges.is_empty() {
                    self.contours.push(contour);
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
