//! Fixed algorithms for sine, cosine, arc tangent and arc sine, in turns.
//!
//! Sine and cosine of an angle `t` in turns: `f = t - round(t)` (exact, `|f| <= 0.5`, and it keeps
//! the sign and the relative precision of a small `t` of either sign), `q = 4 f`, `n = round(q)`,
//! `r = q - n` (exact, `|r| <= 0.5`), `a = r * pi/2` (`|a| <= pi/4`), then the Maclaurin series of
//! sine and cosine in nested (Horner) form with 10 terms each (sine to `a^21`, cosine to `a^20`; the
//! truncation error is below 1e-22), then an exact quadrant rotation by `n mod 4`. Quarter turns are
//! therefore exact. Source: Abramowitz and Stegun 1964, 4.3.65 and 4.3.66.
//!
//! Arc tangent of `a` in `[0, 1]`: when `a > tan(pi/8)` use `atan(a) = pi/4 + atan((a - 1)/(a + 1))`
//! (Abramowitz and Stegun 4.4.34), then the Maclaurin series of arc tangent for `|z| <= 0.4143` with 25
//! terms in nested form (4.4.42; truncation error below 1e-19). `atan2` reduces to that range by
//! octants with exact constants 0.125, 0.25 and 0.5 turn. Measured error against the platform arc
//! tangent in the tests stays below 1e-15 turns; the expected bound is a few units in the last place.

/// Two pi, the nearest `f64`.
pub const TAU: f64 = std::f64::consts::TAU;
/// Pi over two, the nearest `f64`.
pub const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
/// One over two pi, the nearest `f64`.
pub const INV_TAU: f64 = 0.15915494309189535;
/// tan(pi/8) = sqrt(2) - 1, the nearest `f64`.
pub const TAN_EIGHTH_PI: f64 = 0.41421356237309503;

const SIN_COS_TERMS: u32 = 10;
const ATAN_TERMS: u32 = 25;

fn neg(x: f64) -> f64 {
    0.0 - x
}

fn series(a: f64) -> (f64, f64) {
    let a2 = a * a;
    let mut s = 1.0;
    let mut c = 1.0;
    let mut k = SIN_COS_TERMS;
    while k >= 1 {
        let m = f64::from(k);
        s = 1.0 - a2 / ((2.0 * m) * (2.0 * m + 1.0)) * s;
        c = 1.0 - a2 / ((2.0 * m - 1.0) * (2.0 * m)) * c;
        k -= 1;
    }
    (a * s, c)
}

/// Sine and cosine of `t` turns. Not finite input gives (NaN, NaN).
pub fn sin_cos_turns(t: f64) -> (f64, f64) {
    if !t.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    let f = t - t.round();
    let q = f * 4.0;
    let n = q.round();
    let r = q - n;
    let (s, c) = series(r * HALF_PI);
    match (n as i64) & 3 {
        0 => (s, c),
        1 => (c, neg(s)),
        2 => (neg(s), neg(c)),
        _ => (neg(c), s),
    }
}

/// Sine and cosine of an angle in degrees, converted to turns by dividing by 360.
pub fn sin_cos_deg(deg: f64) -> (f64, f64) {
    sin_cos_turns(deg / 360.0)
}

/// Sine and cosine of an angle in radians, converted to turns by multiplying by 1/(2 pi).
pub fn sin_cos_rad(rad: f64) -> (f64, f64) {
    sin_cos_turns(rad * INV_TAU)
}

fn atan_unit_turns(a: f64) -> f64 {
    let (z, base) = if a > TAN_EIGHTH_PI {
        ((a - 1.0) / (a + 1.0), 0.125)
    } else {
        (a, 0.0)
    };
    let z2 = z * z;
    let mut acc = 0.0;
    let mut k = ATAN_TERMS;
    loop {
        acc = 1.0 / (2.0 * f64::from(k) + 1.0) - z2 * acc;
        if k == 0 {
            break;
        }
        k -= 1;
    }
    base + (z * acc) * INV_TAU
}

/// Angle of the point (x, y) in turns, in (-0.5, 0.5]. `atan2_turns(0.0, 0.0)` is 0.
pub fn atan2_turns(y: f64, x: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    let ax = x.abs();
    let ay = y.abs();
    if ax == 0.0 && ay == 0.0 {
        return 0.0;
    }
    let mut t = if ay > ax {
        0.25 - atan_unit_turns(ax / ay)
    } else {
        atan_unit_turns(ay / ax)
    };
    if x < 0.0 {
        t = 0.5 - t;
    }
    if y < 0.0 {
        t = neg(t);
    }
    t
}

/// Arc sine in turns, in [-0.25, 0.25]; the input is clamped to [-1, 1].
pub fn asin_turns(s: f64) -> f64 {
    if s.is_nan() {
        return f64::NAN;
    }
    let s = s.clamp(-1.0, 1.0);
    atan2_turns(s, ((1.0 - s) * (1.0 + s)).sqrt())
}
