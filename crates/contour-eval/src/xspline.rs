//! X-spline spans.
//!
//! Source (PUBLISHED): C. Blanc and C. Schlick, "X-Splines: A Spline Model Designed for the
//! End-User", Proceedings of SIGGRAPH 1995, pp. 377 to 386. A curve segment between control
//! points P1 and P2 is a weighted mean of the four points P0..P3 around it, with weights from
//! blending functions that depend on the shape parameters s1 of P1 and s2 of P2 (paper
//! sections 3 and 4). The weights are normalised by their sum (the sum is not 1 for the
//! general functions). A positive shape parameter gives an approximating point (blending
//! function F, a quintic whose width depends on s), a negative one an interpolating point
//! (functions G and H), and s = 0 a sharp point through which the curve passes.
//!
//! The polynomials, with u in [0, 1] the normalised position inside the function's support:
//!
//! * F(u; p) = u^3 (10 - p + (2p - 15) u + (6 - p) u^2), with p = 2 D^2 for a support
//!   of width D (the paper's Hermite-like quintic; F(0) = 0, F(1) = 1);
//! * G(u; q) = q u + 2q u^2 + (8 - 12q) u^3 + (14q - 11) u^4 + (4 - 5q) u^5 (p fixed to 2);
//! * H(u; q) = q u + 2q u^2 - 2q u^4 - q u^5;
//! * q = -s for a negative shape parameter s.
//!
//! The knot offsets for a positive parameter are (for a segment with local parameter t in
//! [0, 1]): the function of P0 has support ending at t = s1 and is F((s1 - t)/(1 + s1)); that
//! of P2 is F((t + s1)/(1 + s1)); that of P1 is F((1 + s2 - t)/(1 + s2)); that of P3 is
//! F((t - 1 + s2)/(1 + s2)) for t above 1 - s2. For a negative parameter the functions are
//! H(-t), G(t) for P0 and P2 and G(1 - t), H(t - 1) for P1 and P3, with q = -s.
//!
//! OPEN in the paper and exposed as configuration by this crate: how the ends of an open
//! contour are handled (the paper treats control polygons without fixing it for a contour
//! that ends), how a stored weight maps to s, and which of the two variants is used.
//! The general variant takes s in [-1, 1]; the basic variant takes s in [0, 1] only (paper
//! section 3, "basic X-spline", the same F with no negative influence). The printed
//! page of the paper was not machine readable in the copy that could be fetched, so the
//! formulas above are written from the widely used reference implementation of the paper
//! (the xfig drawing program) and are not verified against the paper itself. They are
//! checked against the paper's stated properties by the tests: a sharp point at s = 0,
//! equality with the uniform cubic B-spline point (P0 + 4 P1 + P2) / 6 at the segment start
//! for s1 = s2 = 1, interpolation for s <= 0, and continuity of adjacent segments.

use crate::real::Real;
use crate::ContourError;

/// F blending function; `num` over `den` is the normalised position, p = 2 den^2.
pub(crate) fn f_blend<R: Real>(num: R, den: R) -> R {
    let p = R::from_f64(2.0) * den * den;
    let u = num / den;
    let u2 = u * u;
    u * u2
        * ((R::from_f64(10.0) - p)
            + (R::from_f64(2.0) * p - R::from_f64(15.0)) * u
            + (R::from_f64(6.0) - p) * u2)
}

pub(crate) fn g_blend<R: Real>(u: R, q: R) -> R {
    let c = R::from_f64;
    u * (q + u
        * (c(2.0) * q
            + u * (c(8.0) - c(12.0) * q + u * (c(14.0) * q - c(11.0) + u * (c(4.0) - c(5.0) * q)))))
}

pub(crate) fn h_blend<R: Real>(u: R, q: R) -> R {
    let c = R::from_f64;
    let u2 = u * u;
    u * (q + u * (c(2.0) * q + u2 * (-(c(2.0) * q) - u * q)))
}

/// Unnormalised weights [w0, w1, w2, w3] of P0..P3 for local parameter `t` and shape parameters
/// `s1` (of P1) and `s2` (of P2).
pub(crate) fn weights<R: Real>(s1: R, s2: R, t: R) -> [R; 4] {
    let zero = R::from_f64(0.0);
    let one = R::from_f64(1.0);
    let (a0, a2) = if s1 < zero {
        let q = -s1;
        (h_blend(-t, q), g_blend(t, q))
    } else {
        // Tk = 1 + s1 for P0 (support ends at t = s1), Tk = 1 - s1 for P2.
        let a0 = if t < s1 {
            f_blend(t - s1, -(one + s1))
        } else {
            zero
        };
        (a0, f_blend(t + s1, one + s1))
    };
    let (a1, a3) = if s2 < zero {
        let q = -s2;
        (g_blend(one - t, q), h_blend(t - one, q))
    } else {
        let a1 = f_blend(t - one - s2, -(one + s2));
        let a3 = if t > one - s2 {
            f_blend(t - one + s2, one + s2)
        } else {
            zero
        };
        (a1, a3)
    };
    [a0, a1, a2, a3]
}

/// The normalised point of the segment: sum of weight times point over the sum of weights.
pub(crate) fn point<R: Real>(
    pts: &[(R, R); 4],
    s1: R,
    s2: R,
    t: R,
) -> Result<(R, R), ContourError> {
    let w = weights(s1, s2, t);
    let sum = w[0] + w[1] + w[2] + w[3];
    if sum == R::from_f64(0.0) || !sum.to_f64().is_finite() {
        return Err(ContourError::ZeroWeightSum);
    }
    let x = w[0] * pts[0].0 + w[1] * pts[1].0 + w[2] * pts[2].0 + w[3] * pts[3].0;
    let y = w[0] * pts[0].1 + w[1] * pts[1].1 + w[2] * pts[2].1 + w[3] * pts[3].1;
    Ok((x / sum, y / sum))
}
