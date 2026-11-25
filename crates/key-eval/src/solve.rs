//! Solvers of the Bezier curve parameter. Only add, subtract, multiply, divide and square root are used,
//! and every loop runs a fixed number of times, so results are the same on every IEEE 754 platform.
//!
//! The problem is normalized: for a segment from `ta` to `tb` with handle times `x1` and `x2`, let
//! `p1 = (x1 - ta) / L`, `p2 = (x2 - ta) / L`, `s = (t - ta) / L`. The time polynomial of a cubic Bezier
//! curve with control abscissae `0, p1, p2, 1` in the power basis is
//! `x(u) = a u^3 + b u^2 + c u` with `a = 3 p1 - 3 p2 + 1`, `b = 3 p2 - 6 p1`, `c = 3 p1`
//! (Farin, Curves and Surfaces for CAGD, 5th ed., 2002, the chapter on Bezier curves, conversion to
//! the power basis), and the solvers find `u` in `[0, 1]` with `x(u) = s`.

use crate::config::{NewtonStart, Solver};

/// Number of Newton steps in [`cbrt`].
pub const CBRT_STEPS: u32 = 48;
/// Number of bisection steps used by the cosine free three root case of the closed form.
pub const TRIG_FREE_STEPS: u32 = 64;
/// Largest time residual `|x(u) - s|` of the closed form root that is accepted; a larger residual
/// sends [`solve`] to bisection with [`TRIG_FREE_STEPS`] halvings.
pub const CLOSED_FORM_RESIDUAL: f64 = 1e-12;

/// Real cube root by a fixed algorithm: the start is the integer bit pattern estimate
/// `bits / 3 + 0x2AAAAAAAAAAAAAAA` (exact thirds of the exponent bias), then 48 Newton steps
/// `y = (2 y + x / (y y)) / 3` (Newton 1669 applied to `y^3 - x`; Press et al., Numerical Recipes, 3rd
/// ed., 2007, section 9.4), then the neighbour in units in the last place with the smallest `|y^3 - x|`
/// is chosen (the first on a tie). Zero, NaN and infinities are returned as they are.
pub fn cbrt(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let neg = x < 0.0;
    let ax = x.abs();
    let mut y = f64::from_bits(ax.to_bits() / 3 + 0x2AAA_AAAA_AAAA_AAAA);
    for _ in 0..CBRT_STEPS {
        y = (2.0 * y + ax / (y * y)) / 3.0;
    }
    let up = f64::from_bits(y.to_bits() + 1);
    let down = f64::from_bits(y.to_bits() - 1);
    let err = |v: f64| (v * v * v - ax).abs();
    let mut best = y;
    for c in [up, down] {
        if err(c) < err(best) {
            best = c;
        }
    }
    if neg {
        -best
    } else {
        best
    }
}

/// The polynomial coefficients `(a, b, c)` of the normalized time curve.
pub fn coeffs(p1: f64, p2: f64) -> (f64, f64, f64) {
    (3.0 * p1 - 3.0 * p2 + 1.0, 3.0 * p2 - 6.0 * p1, 3.0 * p1)
}

fn xpoly(co: (f64, f64, f64), u: f64) -> f64 {
    ((co.0 * u + co.1) * u + co.2) * u
}

fn xderiv(co: (f64, f64, f64), u: f64) -> f64 {
    (3.0 * co.0 * u + 2.0 * co.1) * u + co.2
}

fn clamp01(u: f64) -> f64 {
    u.clamp(0.0, 1.0)
}

fn newton_steps(co: (f64, f64, f64), s: f64, start: f64, steps: u32) -> f64 {
    let mut u = clamp01(start);
    for _ in 0..steps {
        let d = xderiv(co, u);
        if d != 0.0 {
            u = clamp01(u - (xpoly(co, u) - s) / d);
        }
    }
    u
}

fn bisect(co: (f64, f64, f64), s: f64, steps: u32) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..steps {
        let mid = (lo + hi) / 2.0;
        if xpoly(co, mid) < s {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// Candidate roots of `a u^3 + b u^2 + c u - s = 0` by closed form (three slots, repeated when fewer). The
/// result is unchecked: [`solve`] tests its residual and falls back to bisection.
pub fn closed_form(co: (f64, f64, f64), s: f64) -> [f64; 3] {
    let (a, b, c) = co;
    if a == 0.0 {
        if b == 0.0 {
            let r = if c == 0.0 { 0.0 } else { s / c };
            return [r, r, r];
        }
        // Quadratic b u^2 + c u - s = 0 by the cancellation free form (Numerical Recipes 3rd ed. 5.6).
        let mut disc = c * c + 4.0 * b * s;
        if disc < 0.0 {
            disc = 0.0;
        }
        let sq = disc.sqrt();
        let q = if c >= 0.0 {
            -(c + sq) / 2.0
        } else {
            -(c - sq) / 2.0
        };
        let r1 = q / b;
        let r2 = if q == 0.0 { r1 } else { -s / q };
        return [r1, r2, r1];
    }
    let bb = b / a;
    let cc = c / a;
    let dd = -s / a;
    let p = cc - bb * bb / 3.0;
    let q = 2.0 * bb * bb * bb / 27.0 - bb * cc / 3.0 + dd;
    let shift = bb / 3.0;
    let disc = q * q / 4.0 + p * p * p / 27.0;
    if disc >= 0.0 {
        // One real root (Cardano 1545; Numerical Recipes 3rd ed. 5.6).
        let sd = disc.sqrt();
        let w = cbrt(-q / 2.0 + sd) + cbrt(-q / 2.0 - sd);
        let r = w - shift;
        [r, r, r]
    } else {
        // Three real roots: w_k = 2 m cos(alpha - 2 pi k / 3) with m = sqrt(-p / 3) and
        // cos(3 alpha) = (-q / 2) / m^3. cos(alpha) = y solves 4 y^3 - 3 y = cos(3 alpha) on [0.5, 1]
        // (Chebyshev triple angle identity), found by 64 bisections; the other two roots follow from the
        // angle sum formulas with sin(alpha) = sqrt(1 - y^2) and sqrt(3). No cosine is evaluated.
        let m = (-p / 3.0).sqrt();
        let k = ((-q / 2.0) / (m * m * m)).clamp(-1.0, 1.0);
        let (mut lo, mut hi) = (0.5, 1.0);
        for _ in 0..TRIG_FREE_STEPS {
            let mid = (lo + hi) / 2.0;
            if 4.0 * mid * mid * mid - 3.0 * mid < k {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let y = (lo + hi) / 2.0;
        let sn = (1.0 - y * y).sqrt();
        let h = 3.0f64.sqrt() / 2.0;
        let w0 = 2.0 * m * y;
        let w1 = 2.0 * m * (-y / 2.0 - sn * h);
        let w2 = 2.0 * m * (-y / 2.0 + sn * h);
        [w0 - shift, w1 - shift, w2 - shift]
    }
}

fn dist01(u: f64) -> f64 {
    if u < 0.0 {
        -u
    } else if u > 1.0 {
        u - 1.0
    } else {
        0.0
    }
}

/// Solve `x(u) = s` for `u` in `[0, 1]` with the chosen solver. `p1`, `p2` are the normalized handle times
/// and `s` the normalized query time, `0 < s < 1`.
pub fn solve(solver: Solver, p1: f64, p2: f64, s: f64) -> f64 {
    let co = coeffs(p1, p2);
    match solver {
        Solver::Cardano { polish } => {
            let cands = closed_form(co, s);
            let mut best = cands[0];
            for &c in &cands[1..] {
                let (dc, db) = (dist01(c), dist01(best));
                if dc < db || (dc == db && c < best) {
                    best = c;
                }
            }
            let mut u = clamp01(best);
            // Near a quadratic (a tiny but not zero) the closed form cancels away all its digits and can
            // return a point far from the curve; a root of the normalized curve has a tiny residual.
            let residual = (xpoly(co, u) - s).abs();
            if residual > CLOSED_FORM_RESIDUAL || residual.is_nan() {
                u = bisect(co, s, TRIG_FREE_STEPS);
            }
            newton_steps(co, s, u, polish)
        }
        Solver::Bisection { steps } => bisect(co, s, steps),
        Solver::Newton { steps, start } => {
            let st = match start {
                NewtonStart::Half => 0.5,
                NewtonStart::Chord => s,
                NewtonStart::Fixed(v) => v,
            };
            newton_steps(co, s, st, steps)
        }
    }
}
