//! Code taken from https://gitlab.com/Kyarei/fdsm/-/blob/main/fdsm/src/correct_error.rs and modified by me.

mod artifact_classifier;

use crate::{BitmapData, GlyphBitmapData, edge_color::EdgeColor, shape::Shape};
use artifact_classifier::*;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
/// The mode of operation for the MSDF error correction pass.
enum ErrorCorrectionMode {
    /// Corrects artifacts at edges and other discontinuous distances only if doing so does not affect edges or corners.
    #[default]
    EdgePriority,
    /// Only corrects artifacts at edges.
    EdgeOnly,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
/// Configuration of whether to use an algorithm that computes the exact shape distance at the positions of suspected artifacts. This algorithm can be much slower.
enum DistanceCheckMode {
    /// Never computes exact shape distance.
    Never,
    /// Only computes exact shape distance at edges. Provides a good balance between speed and precision.
    #[default]
    AtEdge,
}

#[derive(Debug, Clone)]
/// The configuration of the MSDF error correction pass.
pub(crate) struct ErrorCorrectionConfig {
    /// The mode of error correction.
    mode: ErrorCorrectionMode,
    /// Specifies when to compute exact distances.
    distance_check: DistanceCheckMode,
    /// The minimum ratio between the actual and maximum expected distance delta to be considered an error.
    min_deviation_ratio: f64,
}
impl Default for ErrorCorrectionConfig {
    fn default() -> Self {
        Self {
            mode: Default::default(),
            distance_check: Default::default(),
            min_deviation_ratio: 10.0 / 9.0,
        }
    }
}

const ERROR: u8 = 1;
const PROTECTED: u8 = 2;
const RADIUS_TOLERANCE: f64 = 1.001;

struct ErrorCorrection<'a> {
    stencil: &'a mut GlyphBitmapData<u8, 1>,
    config: &'a ErrorCorrectionConfig,
    inv_range: f64,
}
impl ErrorCorrection<'_> {
    fn protect(&mut self, x: usize, y: usize) {
        self.stencil
            .set_px([self.stencil.get_px(x, y)[0] | PROTECTED], x, y);
    }

    fn mark_error(&mut self, x: usize, y: usize) {
        self.stencil
            .set_px([self.stencil.get_px(x, y)[0] | ERROR], x, y);
    }

    fn protect_extreme_channels(
        &mut self,
        x: usize,
        y: usize,
        msd: [f64; 3],
        m: f64,
        mask: EdgeColor,
    ) {
        if (mask.has_red() && msd[0] != m)
            || (mask.has_green() && msd[1] != m)
            || (mask.has_blue() && msd[2] != m)
        {
            self.stencil
                .set_px([self.stencil.get_px(x, y)[0] | PROTECTED], x, y);
        }
    }

    fn protect_all(&mut self) {
        for px in self.stencil.bytes_mut() {
            *px |= PROTECTED;
        }
    }

    fn protect_corners(&mut self, shape: &Shape) {
        for contour in &shape.contours {
            let edges = &contour.edges;

            for i in 0..edges.len() {
                let curr = &edges[i];
                let prev = &edges[i.checked_sub(1).unwrap_or(edges.len() - 1)];

                let common_color = prev.color & curr.color;
                if !common_color.is_bright() {
                    let p = curr.point(0.0);
                    let l = (p.x - 0.5).floor() as isize;
                    let b = (p.y - 0.5).floor() as isize;
                    let r = l + 1;
                    let t = b + 1;
                    if l < self.stencil.width() as isize
                        && b < self.stencil.height() as isize
                        && r >= 0
                        && t >= 0
                    {
                        let r = r as usize;
                        let t = t as usize;

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

    fn protect_edges(&mut self, sdf: &impl BitmapData<Pixel = [f64; 3]>) {
        let radius = RADIUS_TOLERANCE * self.inv_range;

        // Horizontal texel pairs
        for y in 0..sdf.height() {
            for x in 0..(sdf.width() - 1) {
                // Safety: These points are in-bounds.
                let left = sdf.get_px(x, y);
                let right = sdf.get_px(x + 1, y);
                let lm = median(left);
                let rm = median(right);
                if (lm - 0.5).abs() + (rm - 0.5).abs() < radius {
                    let mask = edge_between_texels(left, right);
                    self.protect_extreme_channels(x, y, left, lm, mask);
                    self.protect_extreme_channels(x + 1, y, right, rm, mask);
                }
            }
        }
        // Vertical texel pairs
        for y in 0..(sdf.height() - 1) {
            for x in 0..sdf.width() {
                // Safety: These points are in-bounds.
                let bottom = sdf.get_px(x, y);
                let top = sdf.get_px(x, y + 1);
                let bm = median(bottom);
                let tm = median(top);
                if (bm - 0.5).abs() + (tm - 0.5).abs() < radius {
                    let mask = edge_between_texels(bottom, top);
                    self.protect_extreme_channels(x, y, bottom, bm, mask);
                    self.protect_extreme_channels(x, y + 1, top, tm, mask);
                }
            }
        }
        // Diagonal texel pairs
        for y in 0..(sdf.height() - 1) {
            for x in 0..(sdf.width() - 1) {
                // Safety: These points are in-bounds.
                let lb = sdf.get_px(x, y);
                let lt = sdf.get_px(x, y + 1);
                let rb = sdf.get_px(x + 1, y);
                let rt = sdf.get_px(x + 1, y + 1);
                let mlb = median(lb);
                let mlt = median(lt);
                let mrb = median(rb);
                let mrt = median(rt);
                if (mlb - 0.5).abs() + (mrt - 0.5).abs() < radius {
                    let mask = edge_between_texels(lb, rt);
                    self.protect_extreme_channels(x, y, lb, mlb, mask);
                    self.protect_extreme_channels(x + 1, y + 1, rt, mrt, mask);
                }
                if (mrb - 0.5).abs() + (mlt - 0.5).abs() < radius {
                    let mask = edge_between_texels(rb, lt);
                    self.protect_extreme_channels(x + 1, y, rb, mrb, mask);
                    self.protect_extreme_channels(x, y + 1, lt, mlt, mask);
                }
            }
        }
    }

    fn find_errors(&mut self, sdf: &impl BitmapData<Pixel = [f64; 3]>) {
        let hspan = self.config.min_deviation_ratio * self.inv_range;
        let vspan = hspan;
        let dspan = hspan * 2.0f64.sqrt();
        for y in 0..sdf.height() {
            for x in 0..sdf.width() {
                // Safety: c is in bounds, and we check that
                // the other points are in bounds.
                let c = sdf.get_px(x, y);
                let cm = median(c);
                let protected = (self.stencil.get_px(x, y)[0] & PROTECTED) != 0;
                let l = (x > 0).then(|| sdf.get_px(x - 1, y));
                let r = (x < sdf.width() - 1).then(|| sdf.get_px(x + 1, y));
                let b = (y > 0).then(|| sdf.get_px(x, y - 1));
                let t = (y < sdf.height() - 1).then(|| sdf.get_px(x, y + 1));

                let is_error = 'err: {
                    let hspan_class = BaseArtifactClassifier {
                        span: hspan,
                        protected,
                    };
                    let vspan_class = BaseArtifactClassifier {
                        span: vspan,
                        protected,
                    };
                    let dspan_class = BaseArtifactClassifier {
                        span: dspan,
                        protected,
                    };

                    if l.is_some_and(|l| has_linear_artifact(&hspan_class, cm, c, l))
                        || b.is_some_and(|b| has_linear_artifact(&vspan_class, cm, c, b))
                        || r.is_some_and(|r| has_linear_artifact(&hspan_class, cm, c, r))
                        || t.is_some_and(|t| has_linear_artifact(&vspan_class, cm, c, t))
                    {
                        break 'err true;
                    }
                    if let Some(l) = l {
                        if let Some(b) = b {
                            // Safety: If (x - 1, y) and (x, y - 1) are in bounds, then so is (x - 1, y - 1).
                            if has_diagonal_artifact(
                                &dspan_class,
                                cm,
                                &c,
                                &l,
                                &b,
                                &sdf.get_px(x - 1, y - 1),
                            ) {
                                break 'err true;
                            }
                        }
                        if let Some(t) = t {
                            // Safety: If (x - 1, y) and (x, y + 1) are in bounds, then so is (x - 1, y + 1).
                            if has_diagonal_artifact(
                                &dspan_class,
                                cm,
                                &c,
                                &l,
                                &t,
                                &sdf.get_px(x - 1, y + 1),
                            ) {
                                break 'err true;
                            }
                        }
                    }
                    if let Some(r) = r {
                        if let Some(b) = b {
                            // Safety: If (x + 1, y) and (x, y - 1) are in bounds, then so is (x + 1, y - 1).
                            if has_diagonal_artifact(
                                &dspan_class,
                                cm,
                                &c,
                                &r,
                                &b,
                                &sdf.get_px(x + 1, y - 1),
                            ) {
                                break 'err true;
                            }
                        }
                        if let Some(t) = t {
                            // Safety: If (x + 1, y) and (x, y + 1) are in bounds, then so is (x + 1, y + 1).
                            if has_diagonal_artifact(
                                &dspan_class,
                                cm,
                                &c,
                                &r,
                                &t,
                                &sdf.get_px(x + 1, y + 1),
                            ) {
                                break 'err true;
                            }
                        }
                    }
                    false
                };
                if is_error {
                    self.mark_error(x, y);
                }
            }
        }
    }

    fn apply(
        &self,
        sdf: &impl BitmapData<Pixel = [f64; 3]>,
        bitmap: &mut impl BitmapData<Pixel = [u8; 3]>,
    ) {
        use crate::edge_selector::EdgeSelectorDistance;

        for y in 0..sdf.height() {
            for x in 0..sdf.width() {
                let mut px = sdf.get_px(x, y);

                if self.stencil.get_px(x, y)[0] & ERROR != 0 {
                    let p = sdf.get_px(x, y);
                    let m = median(p);
                    px = [m; 3];
                }

                bitmap.set_px(px.map(|p| p.to_bytes()[0]), x, y);
            }
        }
    }
}

fn get_error_correction_plan<'a>(
    stencil: &'a mut GlyphBitmapData<u8, 1>,
    sdf: &impl BitmapData<Pixel = [f64; 3]>,
    shape: &Shape,
    range: f64,
    config: &'a ErrorCorrectionConfig,
) -> ErrorCorrection<'a> {
    let mut correction = ErrorCorrection {
        stencil,
        config,
        inv_range: 1.0 / range,
    };

    match config.mode {
        ErrorCorrectionMode::EdgePriority => {
            correction.protect_corners(shape);
            correction.protect_edges(sdf);
        }
        ErrorCorrectionMode::EdgeOnly => correction.protect_all(),
    }
    if config.distance_check == DistanceCheckMode::Never
        || (config.distance_check == DistanceCheckMode::AtEdge
            && config.mode != ErrorCorrectionMode::EdgeOnly)
    {
        correction.find_errors(sdf);
        if config.distance_check == DistanceCheckMode::AtEdge {
            correction.protect_all();
        }
    }

    correction
}

pub(crate) fn correct_error_msdf(
    shape: &Shape,
    range: f64,
    config: &ErrorCorrectionConfig,
    msdf: &mut impl BitmapData<Pixel = [f64; 3]>,
    bitmap: &mut impl BitmapData<Pixel = [u8; 3]>,
) {
    let mut stencil = GlyphBitmapData::<u8, 1>::new(msdf.width(), msdf.height());
    let correction = get_error_correction_plan(&mut stencil, msdf, shape, range, config);

    correction.apply(msdf, bitmap);
}

// UTILS

#[inline]
const fn median(v: [f64; 3]) -> f64 {
    median3(v[0], v[1], v[2])
}

#[inline]
const fn median3(a: f64, b: f64, c: f64) -> f64 {
    f64::max(f64::min(a, b), f64::min(f64::max(a, b), c))
}

#[inline]
const fn mix(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}

fn edge_between_texels_channel(a: [f64; 3], b: [f64; 3], channel: usize) -> bool {
    let t = (a[channel] - 0.5) / (a[channel] - b[channel]);
    if t > 0.0 && t < 1.0 {
        let c = [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)];
        median(c) == c[channel]
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
