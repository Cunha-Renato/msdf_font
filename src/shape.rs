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

    fn generate_normalized_distance_field<
        E: EdgeSelector<Distance = impl EdgeSelectorDistance<Normalized = P>>,
        P,
        B: BitmapData<Pixel = P>,
    >(
        &self,
        bitmap: &mut B,
        px_range: f64,
        offset: DVec2,
    ) {
        let mut shape_distance_finder = ShapeDistanceFinder::<E>::new(self);
        for y in 0..bitmap.height() {
            for x in 0..bitmap.width() {
                let p =
                    DVec2::new(x as f64 + 0.5, bitmap.height() as f64 - (y as f64 + 0.5)) + offset;

                let bytes = shape_distance_finder.distance(p).normalize(px_range);

                bitmap.set_px(bytes, x, y);
            }
        }
    }

    fn generate_distance_field<
        E: EdgeSelector<Distance = impl EdgeSelectorDistance<Bytes = P>>,
        P,
        B: BitmapData<Pixel = P>,
    >(
        &self,
        bitmap: &mut B,
        px_range: f64,
        offset: DVec2,
    ) {
        let mut shape_distance_finder = ShapeDistanceFinder::<E>::new(self);
        for y in 0..bitmap.height() {
            for x in 0..bitmap.width() {
                let p =
                    DVec2::new(x as f64 + 0.5, bitmap.height() as f64 - (y as f64 + 0.5)) + offset;

                let bytes = shape_distance_finder
                    .distance(p)
                    .normalize_to_bytes(px_range);

                bitmap.set_px(bytes, x, y);
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
        self.generate_distance_field::<TrueDistanceSelector, _, _>(bitmap, px_range, offset)
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
            self.generate_distance_field::<MultiDistanceSelector, _, _>(bitmap, px_range, offset);
            return;
        }

        let mut normalized_bitmap = GlyphBitmapData::<f64, 3>::new(bitmap.width(), bitmap.height());

        self.generate_normalized_distance_field::<MultiDistanceSelector, _, _>(
            &mut normalized_bitmap,
            px_range,
            offset,
        );
        correct_error_msdf(&mut normalized_bitmap, self, px_range, &Default::default());

        for y in 0..bitmap.height() {
            for x in 0..bitmap.width() {
                let p = normalized_bitmap.get_px(x, y).map(|p| p.to_bytes()[0]);

                bitmap.set_px(p, x, y);
            }
        }
    }

    #[cfg(feature = "fix_geometry")]
    pub(crate) fn resolve_shape_geometry(&mut self) {
        use crate::edge::EdgeType;
        use kurbo::{BezPath, Point};
        use linesweeper::topology::Topology;

        if self.contours.is_empty() {
            return;
        }

        // Build a single BezPath from all contours
        let mut path = BezPath::new();
        for contour in &self.contours {
            if contour.edges.is_empty() {
                continue;
            }

            let mut edges = contour.edges.iter();
            let Some(first) = edges.next() else { continue };

            let start = match first.etype {
                EdgeType::Line { p0, .. } => p0,
                EdgeType::Quad { p0, .. } => p0,
            };

            path.move_to(Point::new(start.x, start.y));

            for edge in std::iter::once(first).chain(edges) {
                match edge.etype {
                    EdgeType::Line { p1, .. } => path.line_to(Point::new(p1.x, p1.y)),
                    EdgeType::Quad { p1, p2, .. } => {
                        path.quad_to(Point::new(p1.x, p1.y), Point::new(p2.x, p2.y))
                    }
                }
            }

            path.close_path();
        }

        // from_path returns Result<Topology<i32>, NonClosedPath>
        let topo = match Topology::from_path(&path, 1e-6) {
            Ok(t) => t,
            Err(_) => return,
        };

        // contours() takes a closure: winding number -> is inside?
        // NonZero rule: any non-zero winding is "inside"
        let contours = topo.contours(|w| *w != 0);

        self.contours.clear();

        for contour in contours.contours() {
            for path in contour.path.iter() {
                match path {
                    kurbo::PathEl::MoveTo(p) => self.move_to_scaled(DVec2::new(p.x, p.y)),
                    kurbo::PathEl::LineTo(p) => self.line_to_scaled(DVec2::new(p.x, p.y)),
                    kurbo::PathEl::QuadTo(p0, p1) => {
                        self.quad_to_scaled(DVec2::new(p0.x, p0.y), DVec2::new(p1.x, p1.y))
                    }
                    kurbo::PathEl::CurveTo(p0, p1, p2) => self.curve_to_scaled(
                        DVec2::new(p0.x, p0.y),
                        DVec2::new(p1.x, p1.y),
                        DVec2::new(p2.x, p2.y),
                    ),
                    kurbo::PathEl::ClosePath => self.close(),
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
impl Shape {
    fn move_to_scaled(&mut self, point: DVec2) {
        self.contours.push(Contour::default());
        self.position = point;
    }

    fn line_to_scaled(&mut self, point: DVec2) {
        if point != self.position {
            if let Some(last) = self.contours.last_mut() {
                last.edges.push(Edge::new_line(self.position, point));
            }

            self.position = point;
        }
    }

    fn quad_to_scaled(&mut self, control: DVec2, endpoint: DVec2) {
        if endpoint != self.position {
            if let Some(last) = self.contours.last_mut() {
                last.edges
                    .push(Edge::new_quad(self.position, control, endpoint));
            }

            self.position = endpoint;
        }
    }

    fn curve_to_scaled(&mut self, control: DVec2, control1: DVec2, endpoint: DVec2) {
        let p0 = self.position;

        if endpoint != self.position {
            let mut quads = Vec::new();
            cubic_to_quads(p0, control, control1, endpoint, 0.03, &mut quads); // tolerance in scaled units

            if let Some(contour) = self.contours.last_mut() {
                for q in quads {
                    contour.edges.push(Edge::new_quad(q[0], q[1], q[2]));
                }
            }

            self.position = endpoint;
        }
    }
}
impl OutlineBuilder for Shape {
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

    fn close(&mut self) {}
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

    if (cubic_mid - quad_mid).length() <= tolerance {
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
