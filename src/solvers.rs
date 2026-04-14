//! Code taken from https://github.com/Chlumsky/msdfgen.

use core::f64;

pub(crate) fn solve_quadratic(x: &mut [f64], a: f64, b: f64, c: f64) -> usize {
    // a == 0 -> linear equation
    if a == 0.0 || b.abs() > 1e12 * a.abs() {
        // a == 0, b == 0 -> no solution
        if b == 0.0 {
            return 0;
        }

        x[0] = -c / b;

        return 1;
    }

    let mut dscr = b * b - 4.0 * a * c;

    if dscr > 0.0 {
        dscr = dscr.sqrt();
        x[0] = (-b + dscr) / (2.0 * a);
        x[1] = (-b - dscr) / (2.0 * a);

        2
    } else if dscr == 0.0 {
        x[0] = -b / (2.0 * a);

        1
    } else {
        0
    }
}

fn solve_cubic_normed(x: &mut [f64; 3], mut a: f64, b: f64, c: f64) -> usize {
    let a2 = a * a;
    let q = (a2 - 3.0 * b) * (1.0 / 9.0);
    let r = (a * (2.0 * a2 - 9.0 * b) + 27.0 * c) * (1.0 / 54.0);
    let r2 = r * r;
    let q3 = q * q * q;
    a *= 1.0 / 3.0;

    if r2 < q3 {
        let t = (r / q3.sqrt()).clamp(-1.0, 1.0).acos();
        let q_sqrt = -2.0 * q.sqrt();
        let t_third = t * (1.0 / 3.0);

        x[0] = q_sqrt * t_third.cos() - a;
        x[1] = q_sqrt * (t_third + (2.0 / 3.0) * f64::consts::PI).cos() - a;
        x[2] = q_sqrt * (t_third - (2.0 / 3.0) * f64::consts::PI).cos() - a;

        3
    } else {
        let sign = if r < 0.0 { 1.0 } else { -1.0 };
        let u = sign * (r.abs() + (r2 - q3).sqrt()).cbrt();
        let v = if u == 0.0 { 0.0 } else { q / u };
        let uv_sum = u + v;
        let uv_diff = u - v;

        x[0] = uv_sum - a;

        if uv_diff.abs() < f64::EPSILON || uv_diff.abs() < 1e-12 * uv_sum.abs() {
            x[1] = -0.5 * uv_sum - a;
            return 2;
        }

        1
    }
}

pub(crate) fn solve_cubic(x: &mut [f64; 3], a: f64, b: f64, c: f64, d: f64) -> usize {
    if a != 0.0 {
        let bn = b / a;
        if bn.abs() < 1e6 {
            // Above this ratio, the numerical error gets larger than if we treated a as zero
            return solve_cubic_normed(x, bn, c / a, d / a);
        }
    }

    solve_quadratic(x, b, c, d)
}
