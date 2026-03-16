//! Code taken from https://gitlab.com/Kyarei/fdsm/-/blob/main/fdsm/src/correct_error/artifact_classifier.rs and modified by me.

use crate::solvers::solve_quadratic;

pub const CANDIDATE: u8 = 1;
pub const ARTIFACT: u8 = 2;

/// An object that classifies artifacts in signed distance fields.
pub trait ArtifactClassifier {
    /// Evaluates whether the super::median value `xm` interpolated at `xt`
    /// in the range between `am` at `at` and `bm` at `bt` indicates
    /// an artifact.
    ///
    /// # Parameters
    ///
    /// * `at`: the parameter associated with the left end of the range
    /// * `bt`: the parameter associated with the right end of the range
    /// * `xt`: the parameter associated with the point of interest
    /// * `am`: the value associated with the left end of the range
    /// * `bm`: the value associated with the right end of the range
    /// * `xm`: the value associated with the point of interest
    ///
    /// # Returns
    ///
    /// A `u8` with the [`CANDIDATE`] bit set if this point is
    /// deemed a candidate for further evaluation, and with the
    /// [`ARTIFACT`] bit set if it is deemed as an artifact.
    fn range_test(&self, at: f64, bt: f64, xt: f64, am: f64, bm: f64, xm: f64) -> u8;
    /// Given the return value from [`ArtifactClassifier::evaluate`],
    /// returns whether the point of interest is an artifact.
    ///
    /// # Parameters
    ///
    /// * `t`: the parameter associated with the point of interest
    /// * `m`: the super::median value associated with the point of interest
    fn evaluate(&self, t: f64, m: f64, flags: u8) -> bool;
}

const ARTIFACT_T_EPSILON: f64 = 0.01;

// For some reason, rustc chooses not to inline this by default.
#[inline(always)]
fn is_in_range(t: f64) -> bool {
    t > ARTIFACT_T_EPSILON && t < (1.0 - ARTIFACT_T_EPSILON)
}

/// Checks if a linear interpolation artifact will occur at a point
/// where two specific color channels are equal – such points have
/// extreme super::median values.
///
/// # Parameters
///
/// * `classifier`: the [`ArtifactClassifier`] to use
/// * `am`: the super::median of `a`. This is passed in to avoid
///   recalculation, as the same value of `a` is passed in
///   many times.
/// * `bm`: the super::median of `b`.
/// * `a`: the color of the central texel
/// * `b`: the color of the peripheral texel
/// * `da`, `db`: the difference between a pair of channels for both
///   `a` and `b`.
fn has_linear_artifact_inner(
    classifier: &impl ArtifactClassifier,
    am: f64,
    bm: f64,
    a: [f64; 3],
    b: [f64; 3],
    da: f64,
    db: f64,
) -> bool {
    // Find interpolation ratio t (0 < t < 1) where two color channels are equal (`mix(da, db, t) == 0.0``).
    let t = da / (da - db);
    if is_in_range(t) {
        // Interpolate super::median at t and let the classifier decide if its value indicates an artifact.
        let xm = interpolated_median(a, b, t);
        classifier.evaluate(t, xm, classifier.range_test(0.0, 1.0, t, am, bm, xm))
    } else {
        false
    }
}

/// Checks if a linear interpolation artifact will occur in between
/// two horizontally or vertically adjacent texels `a` and `b`.
///
/// # Parameters
///
/// * `classifier`: the [`ArtifactClassifier`] to use
/// * `am`: the super::median of `a`. This is passed in to avoid
///   recalculation, as the same value of `a` is passed in
///   many times.
/// * `a`: the color of the central texel
/// * `b`: the color of the peripheral texel
pub fn has_linear_artifact(
    classifier: &impl ArtifactClassifier,
    am: f64,
    a: [f64; 3],
    b: [f64; 3],
) -> bool {
    let bm = super::median(b);
    (am - 0.5).abs() > (bm - 0.5).abs()
        && (has_linear_artifact_inner(classifier, am, bm, a, b, a[1] - a[0], b[1] - b[0])
            || has_linear_artifact_inner(classifier, am, bm, a, b, a[2] - a[1], b[2] - b[1])
            || has_linear_artifact_inner(classifier, am, bm, a, b, a[0] - a[2], b[0] - b[2]))
}

struct Alq {
    a: [f64; 3],
    l: [f64; 3],
    q: [f64; 3],
    am: f64,
    dm: f64,
}

/// Checks if a bilinear interpolation artifact will occur at a point
/// where two specific color channels are equal – such points have
/// extreme super::median values.
///
/// # Paramters
///
/// * `classifier`: the [`ArtifactClassifier`] to use
/// * `alq.am`: the super::median of `a`. This is passed in to avoid
///   recalculation, as the same value of `a` is passed in
///   many times.
/// * `alq.dm`: the super::median of the color of the diagonally adjacent
///   texel.
/// * `a`: the color of the central texel
/// * `l`: linear term
/// * `q`: quadratic term
/// * `da`, `dd`: the difference between a pair of channels for both
///   `a` and `d`.
/// * `dbc`: `db + dc - da`, where `db` and `dc` are defined similarly
///   for `b` and `c`
/// * `t_ex0`, `t_ex1`: The interpolation ratios for the local
///   extrema of the two color channels.
fn has_diagonal_artifact_inner(
    classifier: &impl ArtifactClassifier,
    alq: &Alq,
    da: f64,
    dbc_minus_da: f64,
    dd: f64,
    t_ex0: f64,
    t_ex1: f64,
) -> bool {
    let mut ts = [0.0; 2];
    solve_quadratic(&mut ts, dd - dbc_minus_da, dbc_minus_da - da, da);

    for t in ts {
        if is_in_range(t) {
            let xm = interpolated_median2(alq, t);
            let mut range_flags = classifier.range_test(0.0, 1.0, t, alq.am, alq.dm, xm);

            for t_ex in [t_ex0, t_ex1] {
                if t_ex > 0.0 && t_ex < 1.0 {
                    let mut t_end = [0.0, 1.0];
                    let mut em = [alq.am, alq.dm];
                    t_end[(t_ex > t) as usize] = t_ex;
                    em[(t_ex > t) as usize] = interpolated_median2(alq, t_ex);
                    range_flags |= classifier.range_test(t_end[0], t_end[1], t, em[0], em[1], xm)
                }
            }

            if classifier.evaluate(t, xm, range_flags) {
                return true;
            }
        }
    }
    false
}

/// Checks if a bilinear interpolation artifact will occur in between
/// two diagonally adjacent texels `a` and `d` (with `b` and `c`
/// forming the other diagonal).
///
/// # Parameters
///
/// * `classifier`: the [`ArtifactClassifier`] to use
/// * `am`: the super::median of `a`. This is passed in to avoid
///   recalculation, as the same value of `a` is passed in
///   many times.
/// * `a`: the color of the central texel
/// * `b`, `c`: the colors of the two texels neighboring both `a`
///   and `d`
/// * `d`: the color of the texel diagonally adjacent to `a`
pub fn has_diagonal_artifact(
    classifier: &impl ArtifactClassifier,
    am: f64,
    a: &[f64; 3],
    b: &[f64; 3],
    c: &[f64; 3],
    d: &[f64; 3],
) -> bool {
    let dm = super::median(*d);
    // Out of the pair, only report artifacts for the texel farthest from the edge to minimize side effects.
    if (am - 0.5).abs() >= (dm - 0.5).abs() {
        let abc = [a[0] - b[0] - c[0], a[1] - b[1] - c[1], a[2] - b[2] - c[2]];
        let l = [-a[0] - abc[0], -a[1] - abc[1], -a[2] - abc[2]];
        let q = [d[0] + abc[0], d[1] + abc[1], d[2] + abc[2]];
        let t_ex = [-0.5 * l[0] / q[0], -0.5 * l[1] / q[1], -0.5 * l[2] / q[2]];
        let alq = Alq {
            a: *a,
            l,
            q,
            am,
            dm,
        };
        has_diagonal_artifact_inner(
            classifier,
            &alq,
            a[1] - a[0],
            abc[0] - abc[1],
            d[1] - d[0],
            t_ex[0],
            t_ex[1],
        ) || has_diagonal_artifact_inner(
            classifier,
            &alq,
            a[2] - a[1],
            abc[1] - abc[2],
            d[2] - d[1],
            t_ex[1],
            t_ex[2],
        ) || has_diagonal_artifact_inner(
            classifier,
            &alq,
            a[0] - a[2],
            abc[2] - abc[0],
            d[0] - d[2],
            t_ex[2],
            t_ex[0],
        )
    } else {
        false
    }
}

#[inline]
fn interpolated_median(a: [f64; 3], b: [f64; 3], t: f64) -> f64 {
    super::median([
        super::mix(a[0], b[0], t),
        super::mix(a[1], b[1], t),
        super::mix(a[2], b[2], t),
    ])
}

fn interpolated_median2(alq: &Alq, t: f64) -> f64 {
    super::median3(
        t * (t * alq.q[0] + alq.l[0]) + alq.a[0],
        t * (t * alq.q[1] + alq.l[1]) + alq.a[1],
        t * (t * alq.q[2] + alq.l[2]) + alq.a[2],
    )
}

/// An artifact classifier that recognizes artifacts based on the
/// contents of the SDF alone.
#[derive(Debug, Clone)]
pub struct BaseArtifactClassifier {
    pub span: f64,
    pub protected: bool,
}

impl ArtifactClassifier for BaseArtifactClassifier {
    fn range_test(&self, at: f64, bt: f64, xt: f64, am: f64, bm: f64, xm: f64) -> u8 {
        if (am > 0.5 && bm > 0.5 && xm <= 0.5)
            || (am < 0.5 && bm < 0.5 && xm >= 0.5)
            || (!self.protected && super::median3(am, bm, xm) != xm)
        {
            let ax_span = (xt - at) * self.span;
            let bx_span = (bt - xt) * self.span;
            if !((xm - am).abs() <= ax_span && (xm - bm).abs() <= bx_span) {
                CANDIDATE | ARTIFACT
            } else {
                CANDIDATE
            }
        } else {
            0
        }
    }

    fn evaluate(&self, _t: f64, _m: f64, flags: u8) -> bool {
        (flags & ARTIFACT) != 0
    }
}
