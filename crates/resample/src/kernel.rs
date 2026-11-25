//! One dimensional reconstruction kernels.
//!
//! Every function uses only addition, subtraction, multiplication, division and comparison on
//! `f64`, so results are bit identical on every IEEE 754 platform. No fused multiply add is
//! used and no libm function is called.
//!
//! Published definitions:
//! - Keys, "Cubic Convolution Interpolation for Digital Image Processing", IEEE Trans.
//!   Acoustics, Speech and Signal Processing 29(6), 1981, section on the cubic convolution
//!   kernel with free parameter `a` (equation 4 of the paper's kernel definition).
//! - Mitchell and Netravali, "Reconstruction Filters in Computer Graphics", SIGGRAPH 1988,
//!   section 4, the two parameter cubic family with parameters `B` and `C`.
//! - Duchon, "Lanczos Filtering in One and Two Dimensions", J. Applied Meteorology 18, 1979,
//!   and Turkowski, "Filters for Common Resampling Tasks", Graphics Gems I, 1990: the windowed
//!   sinc `sinc(x) * sinc(x / a)` for `|x| < a`.

use std::f64::consts::PI;

/// Number of terms of the sine series used by [`sinc`].
pub const SINC_TERMS: u32 = 9;

/// Absolute error bound of [`sinc`] against the exact function (the series remainder of
/// `P(z)` is about `(pi/2)^18 / 19!`, 2.8e-14, measured maximum 2.78e-14 against a 60 digit
/// oracle; the tests check a bound of 1e-12 against a libm reference).
pub const SINC_MAX_ERROR: f64 = 1e-12;

/// Normalised sinc, `sin(pi x) / (pi x)`, with `sinc(0) = 1`.
///
/// The function is even, so the argument is replaced by its absolute value first.
///
/// Algorithm: split `x = n + r` with `n = floor(x + 0.5)` and `r` in `[-0.5, 0.5]` (exact in
/// binary floating point for `|x| < 2^50`). Then `sin(pi x) = (-1)^n sin(pi r)` and
/// `sin(pi r) = pi r * P(z)` with `z = (pi r)^2` and
/// `P(z) = sum_{k=0}^{8} (-z)^k / (2k+1)!` evaluated in the nested form
/// `1 - z/(2*3) * (1 - z/(4*5) * (...))` with exactly [`SINC_TERMS`] terms.
/// Hence `sinc(x) = (-1)^n r P(z) / x`. At non zero integers `r = 0` and the result is an
/// exact zero.
pub fn sinc(x: f64) -> f64 {
    let x = x.abs();
    if x == 0.0 {
        return 1.0;
    }
    let n = floor(x + 0.5);
    let r = x - n;
    let pr = PI * r;
    let z = pr * pr;
    let mut p = 1.0;
    let mut k = SINC_TERMS - 1;
    while k >= 1 {
        let d = f64::from(2 * k) * f64::from(2 * k + 1);
        p = 1.0 - z * p / d;
        k -= 1;
    }
    let neg = n - 2.0 * floor(n * 0.5) != 0.0;
    let v = r * p / x;
    if neg {
        -v
    } else {
        v
    }
}

/// Floor of an `f64` using only comparisons and exact integer conversion.
pub fn floor(x: f64) -> f64 {
    if x.is_nan() || x.abs() >= 4.0e15 {
        return x;
    }
    let t = x as i64 as f64;
    if t > x {
        t - 1.0
    } else {
        t
    }
}

/// Triangle (bilinear) kernel, support 1.
pub fn bilinear(t: f64) -> f64 {
    let a = t.abs();
    if a < 1.0 {
        1.0 - a
    } else {
        0.0
    }
}

/// Keys cubic convolution kernel with parameter `a`, support 2 (Keys 1981).
///
/// `|t| <= 1`: `(a+2)|t|^3 - (a+3)|t|^2 + 1`; `1 < |t| < 2`: `a|t|^3 - 5a|t|^2 + 8a|t| - 4a`.
/// The kernel is interpolating, so the values at `t = 0, 1, 2` are returned as the exact
/// integers 1, 0, 0 (mathematically equal to the polynomials, exact in floating point).
pub fn keys(t: f64, a: f64) -> f64 {
    let x = t.abs();
    if x == 0.0 {
        1.0
    } else if x == 1.0 || x >= 2.0 {
        0.0
    } else if x < 1.0 {
        let x2 = x * x;
        (a + 2.0) * x2 * x - (a + 3.0) * x2 + 1.0
    } else {
        let x2 = x * x;
        a * x2 * x - 5.0 * a * x2 + 8.0 * a * x - 4.0 * a
    }
}

/// Mitchell and Netravali two parameter cubic, support 2 (Mitchell and Netravali 1988).
///
/// `|t| < 1`: `((12-9B-6C)|t|^3 + (-18+12B+6C)|t|^2 + (6-2B)) / 6`;
/// `1 <= |t| < 2`: `((-B-6C)|t|^3 + (6B+30C)|t|^2 + (-12B-48C)|t| + (8B+24C)) / 6`.
pub fn mitchell(t: f64, b: f64, c: f64) -> f64 {
    let x = t.abs();
    let x2 = x * x;
    let x3 = x2 * x;
    if x < 1.0 {
        ((12.0 - 9.0 * b - 6.0 * c) * x3 + (-18.0 + 12.0 * b + 6.0 * c) * x2 + (6.0 - 2.0 * b))
            / 6.0
    } else if x < 2.0 {
        ((-b - 6.0 * c) * x3
            + (6.0 * b + 30.0 * c) * x2
            + (-12.0 * b - 48.0 * c) * x
            + (8.0 * b + 24.0 * c))
            / 6.0
    } else {
        0.0
    }
}

/// Lanczos kernel with `lobes` lobes per side, `sinc(t) * sinc(t / lobes)` for `|t| < lobes`
/// (Duchon 1979, Turkowski 1990). Values at integers are the exact integers 1 and 0.
pub fn lanczos(t: f64, lobes: u32) -> f64 {
    let x = t.abs();
    let a = f64::from(lobes);
    if x == 0.0 {
        1.0
    } else if x >= a || x == floor(x) {
        0.0
    } else {
        sinc(x) * sinc(x / a)
    }
}
