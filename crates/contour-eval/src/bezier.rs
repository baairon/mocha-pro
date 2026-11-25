//! Cubic Bezier spans.
//!
//! Definition (PUBLISHED): G. Farin, "Curves and Surfaces for CAGD: A Practical Guide", 5th ed.,
//! Morgan Kaufmann, 2002, chapter 4 (Bernstein form, equation 4.2 for n = 3) and chapter 3
//! (de Casteljau algorithm, section 3.1). With t in [0, 1]:
//!
//! B(t) = (1-t)^3 P0 + 3 (1-t)^2 t P1 + 3 (1-t) t^2 P2 + t^3 P3.
//!
//! Three evaluation forms are offered ([`crate::EvalForm`]); they are equal in exact arithmetic
//! and may differ in the last bits. At t = 0 and t = 1 every form returns P0 and P3 unchanged.

use crate::real::Real;
use crate::EvalForm;

pub(crate) type P2<R> = (R, R);

fn lerp<R: Real>(a: R, b: R, t: R) -> R {
    (R::from_f64(1.0) - t) * a + t * b
}

fn comp<R: Real>(form: EvalForm, a: R, b: R, c: R, d: R, t: R) -> R {
    let one = R::from_f64(1.0);
    let three = R::from_f64(3.0);
    match form {
        EvalForm::Bernstein => {
            let mt = one - t;
            let b0 = mt * mt * mt;
            let b1 = three * mt * mt * t;
            let b2 = three * mt * t * t;
            let b3 = t * t * t;
            b0 * a + b1 * b + b2 * c + b3 * d
        }
        EvalForm::DeCasteljau => {
            let ab = lerp(a, b, t);
            let bc = lerp(b, c, t);
            let cd = lerp(c, d, t);
            let abc = lerp(ab, bc, t);
            let bcd = lerp(bc, cd, t);
            lerp(abc, bcd, t)
        }
        EvalForm::Horner => {
            let c1 = three * (b - a);
            let c2 = three * ((a - R::from_f64(2.0) * b) + c);
            let c3 = (d - a) + three * (b - c);
            a + t * (c1 + t * (c2 + t * c3))
        }
    }
}

pub(crate) fn eval<R: Real>(form: EvalForm, p: &[P2<R>; 4], t: R) -> P2<R> {
    if t == R::from_f64(0.0) {
        return p[0];
    }
    if t == R::from_f64(1.0) {
        return p[3];
    }
    (
        comp(form, p[0].0, p[1].0, p[2].0, p[3].0, t),
        comp(form, p[0].1, p[1].1, p[2].1, p[3].1, t),
    )
}

pub(crate) fn split<R: Real>(p: &[P2<R>; 4], t: R) -> ([P2<R>; 4], [P2<R>; 4]) {
    let l = |a: P2<R>, b: P2<R>| (lerp(a.0, b.0, t), lerp(a.1, b.1, t));
    let ab = l(p[0], p[1]);
    let bc = l(p[1], p[2]);
    let cd = l(p[2], p[3]);
    let abc = l(ab, bc);
    let bcd = l(bc, cd);
    let m = l(abc, bcd);
    ([p[0], ab, abc, m], [m, bcd, cd, p[3]])
}

/// True when both inner control points lie within `tol` of the chord segment P0 P3 (Lane and Riesenfeld
/// 1980 flatness test on the control polygon). A zero length chord compares point distances.
pub(crate) fn flat_enough(p: &[P2<f64>; 4], tol: f64) -> bool {
    within(p[0], p[3], p[1], tol) && within(p[0], p[3], p[2], tol)
}

/// True when `q` is at most `tol` from the segment `a b` (from the point `a` when the two
/// coincide). The distance is to the segment, not to the infinite line: a point beyond an end of
/// the chord is measured from that end, so a span that runs out past its chord is not flat.
/// Compared in squared form so no square root is needed.
pub(crate) fn within(a: P2<f64>, b: P2<f64>, q: P2<f64>, tol: f64) -> bool {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len2 = dx * dx + dy * dy;
    let qx = q.0 - a.0;
    let qy = q.1 - a.1;
    if len2 == 0.0 {
        return qx * qx + qy * qy <= tol * tol;
    }
    let dot = qx * dx + qy * dy;
    if dot <= 0.0 {
        return qx * qx + qy * qy <= tol * tol;
    }
    if dot >= len2 {
        let ex = q.0 - b.0;
        let ey = q.1 - b.1;
        return ex * ex + ey * ey <= tol * tol;
    }
    let cross = dx * qy - dy * qx;
    cross * cross <= tol * tol * len2
}
