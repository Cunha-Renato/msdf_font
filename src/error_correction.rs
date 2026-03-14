//! Error correction for MSDF generation.
//!
//! Corrects artifacts where two false edges come too close together,
//! by flattening affected texels to their median value.

use crate::{BitmapData, GlyphBitmapData, edge_color::EdgeColor, shape::Shape};

const ERROR: u8 = 1;
const PROTECTED: u8 = 2;
const PROTECTION_RADIUS_TOLERANCE: f64 = 1.001;

// ── helpers ──────────────────────────────────────────────────────────────────

#[inline]
fn median(p: [f64; 3]) -> f64 {
    p[0].max(p[1]).min(p[1].max(p[2])).max(p[0].min(p[1]))
}

#[inline]
fn mix(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}

fn edge_between_texels_channel(a: [f64; 3], b: [f64; 3], ch: usize) -> bool {
    let t = (a[ch] - 0.5) / (a[ch] - b[ch]);
    if t > 0.0 && t < 1.0 {
        let c = [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)];
        median(c) == c[ch]
    } else {
        false
    }
}

fn edge_between_texels(a: [f64; 3], b: [f64; 3]) -> EdgeColor {
    let mut c = EdgeColor::BLACK;
    if edge_between_texels_channel(a, b, 0) {
        c |= EdgeColor::RED;
    }
    if edge_between_texels_channel(a, b, 1) {
        c |= EdgeColor::GREEN;
    }
    if edge_between_texels_channel(a, b, 2) {
        c |= EdgeColor::BLUE;
    }
    c
}

// ── core struct ───────────────────────────────────────────────────────────────

struct ErrorCorrection<'a> {
    sdf: &'a mut GlyphBitmapData<f64, 3>,
    stencil: &'a mut GlyphBitmapData<u8, 1>,
    inv_range: f64,
}

impl<'a> ErrorCorrection<'a> {
    fn new(
        sdf: &'a mut GlyphBitmapData<f64, 3>,
        stencil: &'a mut GlyphBitmapData<u8, 1>,
        inv_range: f64,
    ) -> Self {
        Self {
            sdf,
            stencil,
            inv_range,
        }
    }

    #[inline]
    fn protect(&mut self, x: usize, y: usize) {
        self.stencil
            .set_px([self.stencil.get_px(x, y)[0] | PROTECTED], x, y);
    }

    #[inline]
    fn mark_error(&mut self, x: usize, y: usize) {
        self.stencil
            .set_px([self.stencil.get_px(x, y)[0] | ERROR], x, y);
    }

    // ── protection passes ────────────────────────────────────────────────────

    fn protect_corners(&mut self, shape: &Shape) {
        for contour in &shape.contours {
            let edges = &contour.edges;

            for i in 0..edges.len() {
                let curr = &edges[i];
                let prev = &edges[i.checked_sub(1).unwrap_or(edges.len() - 1)];
                let common_color = prev.color & curr.color;
                if !common_color.is_bright() {
                    // This is a corner
                    let p = curr.point(0.0);
                    let l = (p.x - 0.5).floor() as i64;
                    let b = (p.y - 0.5).floor() as i64;
                    let r = l + 1;
                    let t = b + 1;
                    if l < self.stencil.width() as i64
                        && b < self.stencil.height() as i64
                        && r >= 0
                        && t >= 0
                    {
                        let r = r as usize;
                        let t = t as usize;
                        unsafe {
                            if let Ok(b) = usize::try_from(b) {
                                if let Ok(l) = usize::try_from(l) {
                                    self.protect(l, b);
                                }
                                if r < self.stencil.width() {
                                    self.protect(r, b);
                                }
                            }
                            if t < self.stencil.height() {
                                if let Ok(l) = usize::try_from(l) {
                                    self.protect(l, t);
                                }
                                if r < self.stencil.width() {
                                    self.protect(r, t);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn protect_edges(&mut self) {
        let radius = (PROTECTION_RADIUS_TOLERANCE * self.inv_range) as f64;
        let w = self.sdf.width;
        let h = self.sdf.height;

        // Horizontal pairs
        for y in 0..h {
            for x in 0..(w - 1) {
                let l = self.sdf.get_px(x, y);
                let r = self.sdf.get_px(x + 1, y);
                let lm = median(l);
                let rm = median(r);
                if (lm - 0.5).abs() + (rm - 0.5).abs() < radius {
                    let mask = edge_between_texels(l, r);
                    self.protect_extreme(x, y, l, lm, mask);
                    self.protect_extreme(x + 1, y, r, rm, mask);
                }
            }
        }
        // Vertical pairs
        for y in 0..(h - 1) {
            for x in 0..w {
                let b = self.sdf.get_px(x, y);
                let t = self.sdf.get_px(x, y + 1);
                let bm = median(b);
                let tm = median(t);
                if (bm - 0.5).abs() + (tm - 0.5).abs() < radius {
                    let mask = edge_between_texels(b, t);
                    self.protect_extreme(x, y, b, bm, mask);
                    self.protect_extreme(x, y + 1, t, tm, mask);
                }
            }
        }
        // Diagonal pairs
        for y in 0..(h - 1) {
            for x in 0..(w - 1) {
                let lb = self.sdf.get_px(x, y);
                let lt = self.sdf.get_px(x, y + 1);
                let rb = self.sdf.get_px(x + 1, y);
                let rt = self.sdf.get_px(x + 1, y + 1);
                let mlb = median(lb);
                let mlt = median(lt);
                let mrb = median(rb);
                let mrt = median(rt);

                if (mlb - 0.5).abs() + (mrt - 0.5).abs() < radius {
                    let mask = edge_between_texels(lb, rt);
                    self.protect_extreme(x, y, lb, mlb, mask);
                    self.protect_extreme(x + 1, y + 1, rt, mrt, mask);
                }
                if (mrb - 0.5).abs() + (mlt - 0.5).abs() < radius {
                    let mask = edge_between_texels(rb, lt);
                    self.protect_extreme(x + 1, y, rb, mrb, mask);
                    self.protect_extreme(x, y + 1, lt, mlt, mask);
                }
            }
        }
    }

    fn protect_extreme(&mut self, x: usize, y: usize, px: [f64; 3], m: f64, mask: EdgeColor) {
        if (mask.has_red() && px[0] != m)
            || (mask.has_green() && px[1] != m)
            || (mask.has_blue() && px[2] != m)
        {
            self.protect(x, y);
        }
    }

    // ── error finding ────────────────────────────────────────────────────────

    fn find_errors_with_distance_check(&mut self, prepared_shape: &PreparedColoredShape) {
        let hspan = (10.0 / 9.0) * self.inv_range;
        let vspan = hspan;
        let dspan = hspan * 2.0f64.sqrt();
        let w = self.sdf.width;
        let h = self.sdf.height;

        // We need to read from sdf and stencil while potentially writing to stencil,
        // so collect errors first then apply.
        let mut errors: Vec<(usize, usize)> = Vec::new();

        let mut checker =
            ShapeDistanceChecker::new(self.sdf, prepared_shape, self.inv_range, 10.0 / 9.0);

        for y in 0..h {
            for x in 0..w {
                let stencil = self.stencil[y * w + x];
                if (stencil & ERROR) != 0 {
                    continue;
                }

                let c = self.sdf.get_px(x, y);
                let cm = median(c);

                checker.shape_coord = Point::new(x as f64 + 0.5, y as f64 + 0.5);
                checker.sdf_coord = Point::new(x as f64 + 0.5, y as f64 + 0.5);
                checker.msd = c;
                checker.protected = (stencil & PROTECTED) != 0;

                let l = (x > 0).then(|| self.sdf.get_px(x - 1, y));
                let r = (x < w - 1).then(|| self.sdf.get_px(x + 1, y));
                let b = (y > 0).then(|| self.sdf.get_px(x, y - 1));
                let t = (y < h - 1).then(|| self.sdf.get_px(x, y + 1));

                let is_error = 'err: {
                    if l.is_some_and(|l| {
                        has_linear_artifact(
                            &checker.classifier(Vector2::new(-1.0, 0.0), hspan),
                            cm,
                            c,
                            l,
                        )
                    }) || r.is_some_and(|r| {
                        has_linear_artifact(
                            &checker.classifier(Vector2::new(1.0, 0.0), hspan),
                            cm,
                            c,
                            r,
                        )
                    }) || b.is_some_and(|b| {
                        has_linear_artifact(
                            &checker.classifier(Vector2::new(0.0, -1.0), vspan),
                            cm,
                            c,
                            b,
                        )
                    }) || t.is_some_and(|t| {
                        has_linear_artifact(
                            &checker.classifier(Vector2::new(0.0, 1.0), vspan),
                            cm,
                            c,
                            t,
                        )
                    }) {
                        break 'err true;
                    }

                    if let Some(l) = l {
                        if let Some(b) = b {
                            if has_diagonal_artifact(
                                &checker.classifier(Vector2::new(-1.0, -1.0), dspan),
                                cm,
                                &c,
                                &l,
                                &b,
                                &self.sdf.get_px(x - 1, y - 1),
                            ) {
                                break 'err true;
                            }
                        }
                        if let Some(t) = t {
                            if has_diagonal_artifact(
                                &checker.classifier(Vector2::new(-1.0, 1.0), dspan),
                                cm,
                                &c,
                                &l,
                                &t,
                                &self.sdf.get_px(x - 1, y + 1),
                            ) {
                                break 'err true;
                            }
                        }
                    }
                    if let Some(r) = r {
                        if let Some(b) = b {
                            if has_diagonal_artifact(
                                &checker.classifier(Vector2::new(1.0, -1.0), dspan),
                                cm,
                                &c,
                                &r,
                                &b,
                                &self.sdf.get_px(x + 1, y - 1),
                            ) {
                                break 'err true;
                            }
                        }
                        if let Some(t) = t {
                            if has_diagonal_artifact(
                                &checker.classifier(Vector2::new(1.0, 1.0), dspan),
                                cm,
                                &c,
                                &r,
                                &t,
                                &self.sdf.get_px(x + 1, y + 1),
                            ) {
                                break 'err true;
                            }
                        }
                    }
                    false
                };

                if is_error {
                    errors.push((x, y));
                }
            }
        }

        for (x, y) in errors {
            self.mark_error(x, y);
        }
    }

    // ── apply ────────────────────────────────────────────────────────────────

    fn apply(&mut self) {
        let w = self.sdf.width;
        let h = self.sdf.height;
        for y in 0..h {
            for x in 0..w {
                if (self.stencil[y * w + x] & ERROR) != 0 {
                    let m = median(self.sdf.get_px(x, y));
                    self.sdf.set_px([m, m, m], x, y);
                }
            }
        }
    }
}

// ── public API ────────────────────────────────────────────────────────────────

/// Corrects MSDF artifacts in place.
///
/// `sdf` must contain normalized [0, 1] f64 values.
/// `range` must be the same value used during MSDF generation.
/// Run this before converting to u8.
pub fn correct_error_msdf(
    sdf: &mut GlyphBitmapData<f64, 3>,
    shape: &Shape<ColoredContour>,
    prepared_shape: &PreparedColoredShape,
    range: f64,
) {
    let mut ec = ErrorCorrection::new(sdf, 1.0 / range);
    ec.protect_corners(shape);
    ec.protect_edges();
    ec.find_errors_with_distance_check(prepared_shape);
    ec.apply();
}
