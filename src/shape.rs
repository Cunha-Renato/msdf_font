//! Some parts were taken from https://github.com/Chlumsky/msdfgen and others from https://gitlab.com/Kyarei/fdsm.
//!
use crate::{
    BitmapData, GlyphBitmapData,
    bounds::Bounds,
    contour::Contour,
    edge::Edge,
    edge_color::EdgeColor,
    edge_selector::{
        EdgeSelector, EdgeSelectorDistance, MultiDistanceSelector, TrueDistanceSelector,
    },
    error_correction::correct_error_msdf,
    shape_distance_finder::ShapeDistanceFinder,
};
use core::f64;
use glam::DVec2;
use kurbo::BezPath;
use linesweeper::topology::Topology;
use ttf_parser::OutlineBuilder;

#[derive(Debug)]
pub(crate) struct Shape {
    pub(crate) contours: Vec<Contour>,
}
impl Shape {
    #[inline]
    pub(crate) fn new(
        face: &ttf_parser::Face,
        glyph_id: ttf_parser::GlyphId,
        scale: f64,
    ) -> Option<Self> {
        let mut outliner = ShapeOutliner::new(scale);

        face.outline_glyph(glyph_id, &mut outliner);

        Some(Self {
            contours: outliner.into_contours()?,
        })
    }

    pub(crate) fn bounds(&self) -> Bounds {
        let mut bounds = Bounds::new();
        for contour in &self.contours {
            contour.bounds(&mut bounds);
        }

        bounds
    }

    fn generate_field<E, P, B>(
        &self,
        bitmap: &mut B,
        offset: DVec2,
        convert: impl Fn(E::Distance) -> P,
    ) where
        E: EdgeSelector,
        B: BitmapData<Pixel = P>,
    {
        let mut shape_distance_finder = ShapeDistanceFinder::<E>::new(self);
        let height = bitmap.height() as f64;

        for y in 0..bitmap.height() {
            let py = height - (y as f64 + 0.5) + offset.y;
            for x in 0..bitmap.width() {
                let p = DVec2::new(x as f64 + 0.5 + offset.x, py);
                bitmap.set_px(convert(shape_distance_finder.distance(p)), x, y);
            }
        }
    }

    #[inline]
    pub(crate) fn generate_sdf(
        &self,
        px_range: f64,
        offset: DVec2,
        bitmap: &mut impl BitmapData<Pixel = [u8; 1]>,
    ) {
        self.generate_field::<TrueDistanceSelector, _, _>(bitmap, offset, |d| {
            d.normalize_to_bytes(px_range)
        })
    }

    #[inline]
    pub(crate) fn generate_msdf(
        &mut self,
        px_range: f64,
        offset: DVec2,
        max_angle: f64,
        error_correction: bool,
        bitmap: &mut impl BitmapData<Pixel = [u8; 3]>,
    ) {
        self.coloring_simple(max_angle, 0);

        if !error_correction {
            self.generate_field::<MultiDistanceSelector, _, _>(bitmap, offset, |d| {
                d.normalize_to_bytes(px_range)
            });
            return;
        }

        let mut normalized_bitmap = GlyphBitmapData::<f64, 3>::new(bitmap.width(), bitmap.height());
        self.generate_field::<MultiDistanceSelector, _, _>(&mut normalized_bitmap, offset, |d| {
            d.normalize(px_range)
        });

        correct_error_msdf(&mut normalized_bitmap, self, px_range, &Default::default());

        // Copy normalized distances to the output bitmap, converting to bytes.
        for y in 0..bitmap.height() {
            for x in 0..bitmap.width() {
                let p = normalized_bitmap.get_px(x, y).map(|p| p.to_bytes()[0]);

                bitmap.set_px(p, x, y);
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
}

struct ShapeOutliner {
    path: BezPath,
    scale: f64,
}
impl ShapeOutliner {
    #[inline]
    fn new(scale: f64) -> Self {
        Self {
            path: BezPath::new(),
            scale,
        }
    }

    #[inline]
    fn scale_point(&self, x: f32, y: f32) -> kurbo::Point {
        kurbo::Point {
            x: f64::from(x) * self.scale,
            y: f64::from(y) * self.scale,
        }
    }

    #[inline]
    fn move_to_scaled(&mut self, point: kurbo::Point) {
        self.path.move_to(point);
    }

    #[inline]
    fn line_to_scaled(&mut self, point: kurbo::Point) {
        self.path.line_to(point);
    }

    #[inline]
    fn quad_to_scaled(&mut self, p1: kurbo::Point, p2: kurbo::Point) {
        self.path.quad_to(p1, p2);
    }

    #[inline]
    fn curve_to_scaled(&mut self, p1: kurbo::Point, p2: kurbo::Point, p3: kurbo::Point) {
        self.path.curve_to(p1, p2, p3);
    }

    fn into_contours(self) -> Option<Vec<Contour>> {
        use kurbo::PathEl;

        let topo = Topology::from_path(&self.path, 1e-6).ok()?;
        let _contours = topo.contours(|w| *w != 0);

        let mut _quad_buffer = Vec::new();
        let contours = _contours
            .contours()
            .map(|c| {
                let mut edges = Vec::new();
                let mut position = DVec2::ZERO;

                for path in c.path.iter() {
                    match path {
                        PathEl::MoveTo(p) => position = DVec2::new(p.x, p.y),
                        PathEl::LineTo(p) => {
                            let p = DVec2::new(p.x, p.y);
                            edges.push(Edge::new_line(position, p));
                            position = p;
                        }
                        PathEl::QuadTo(p, p1) => {
                            let p = DVec2::new(p.x, p.y);
                            let p1 = DVec2::new(p1.x, p1.y);

                            edges.push(Edge::new_quad(position, p, p1));
                            position = p1;
                        }
                        PathEl::CurveTo(p, p1, p2) => {
                            let p = DVec2::new(p.x, p.y);
                            let p1 = DVec2::new(p1.x, p1.y);
                            let p2 = DVec2::new(p2.x, p2.y);

                            _quad_buffer.clear();
                            cubic_to_quads(position, p, p1, p2, 0.03, &mut _quad_buffer);

                            edges.extend(
                                _quad_buffer
                                    .iter()
                                    .map(|q| Edge::new_quad(q[0], q[1], q[2])),
                            );

                            position = p2;
                        }
                        _ => {}
                    }
                }

                Contour { edges }
            })
            .collect::<Vec<_>>();

        Some(contours)
    }
}
impl OutlineBuilder for ShapeOutliner {
    #[inline]
    fn move_to(&mut self, x: f32, y: f32) {
        self.move_to_scaled(self.scale_point(x, y));
    }

    #[inline]
    fn line_to(&mut self, x: f32, y: f32) {
        let endpoint = self.scale_point(x, y);
        self.line_to_scaled(endpoint);
    }

    #[inline]
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let endpoint = self.scale_point(x, y);
        let control = self.scale_point(x1, y1);

        self.quad_to_scaled(control, endpoint);
    }

    #[inline]
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p1 = self.scale_point(x1, y1);
        let p2 = self.scale_point(x2, y2);
        let p3 = self.scale_point(x, y);

        self.curve_to_scaled(p1, p2, p3);
    }

    fn close(&mut self) {
        self.path.close_path();
    }
}

fn cubic_to_quads(
    p0: DVec2,
    p1: DVec2,
    p2: DVec2,
    p3: DVec2,
    tolerance: f64,
    out: &mut Vec<[DVec2; 3]>,
) {
    // Measure how far the best-fit quad deviates from the cubic.
    // The maximum error of degree reduction is bounded by:
    //   error ≈ (3/4) * |P1 + P2 - P0 - P3| / 4  (rough bound)
    // A tighter bound: check the midpoint.
    let q1 = (p1 * 1.5 - p0 * 0.5) * 0.5 + (p2 * 1.5 - p3 * 0.5) * 0.5;

    // Cubic midpoint at t=0.5
    let cubic_mid = p0 * 0.125 + p1 * 0.375 + p2 * 0.375 + p3 * 0.125;
    // Quad midpoint at t=0.5
    let quad_mid = p0 * 0.25 + q1 * 0.5 + p3 * 0.25;

    if (cubic_mid - quad_mid).length_squared() <= tolerance * tolerance {
        out.push([p0, q1, p3]);
    } else {
        let (left, right) = split_cubic(p0, p1, p2, p3, 0.5);
        cubic_to_quads(left[0], left[1], left[2], left[3], tolerance, out);
        cubic_to_quads(right[0], right[1], right[2], right[3], tolerance, out);
    }
}

fn split_cubic(p0: DVec2, p1: DVec2, p2: DVec2, p3: DVec2, t: f64) -> ([DVec2; 4], [DVec2; 4]) {
    let p01 = p0.lerp(p1, t);
    let p12 = p1.lerp(p2, t);
    let p23 = p2.lerp(p3, t);
    let p012 = p01.lerp(p12, t);
    let p123 = p12.lerp(p23, t);
    let p0123 = p012.lerp(p123, t);

    ([p0, p01, p012, p0123], [p0123, p123, p23, p3])
}
