//! Kernels and the separable blurs.
//!
//! Sources: R. Szeliski, "Computer Vision: Algorithms and Applications", 2nd edition, 2022,
//! section 3.2 (linear filtering, box and Gaussian kernels, separable filters). Gaussian weights
//! `exp(-x^2 / (2 sigma^2))` as in section 3.2.1 there. The series for `exp` is the Maclaurin
//! series (M. Abramowitz and I. A. Stegun, "Handbook of Mathematical Functions", 1964, section 4.2).
//!
//! Only add, subtract, multiply, divide and ceil/floor are used on floats, with no fused
//! multiply add, so results are bit identical on every platform.

use crate::border::{resolve_index, Border};
use crate::config::{Config, ExpMethod, FracRule};
use crate::Error;

const MAX_HALF: usize = 1 << 20;

/// The Gaussian parameter: a standard deviation or a radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GaussSpec {
    /// Standard deviation in samples.
    Sigma(f32),
    /// Radius; `sigma = radius / Config::radius_to_sigma`.
    Radius(f32),
}

/// `exp(-t)` for `t >= 0` by the Maclaurin series with `terms` terms after halving `t` to at most
/// one half and squaring the result back as many times. Infinite `t` gives 0.
pub fn exp_neg(t: f32, terms: u32) -> f32 {
    if t == 0.0 {
        return 1.0;
    }
    if !t.is_finite() {
        return 0.0;
    }
    let mut u = t;
    let mut halvings = 0u32;
    while u > 0.5 {
        u *= 0.5;
        halvings += 1;
    }
    let mut sum = 1.0f32;
    let mut term = 1.0f32;
    for i in 1..=terms {
        term = term * u / i as f32;
        if i % 2 == 1 {
            sum -= term;
        } else {
            sum += term;
        }
    }
    for _ in 0..halvings {
        sum *= sum;
    }
    sum
}

fn check_radius(r: f32) -> Result<(), Error> {
    if r.is_finite() && r >= 0.0 && r <= MAX_HALF as f32 {
        Ok(())
    } else {
        Err(Error::BadParameter(
            "radius must be finite, non-negative and at most 2^20",
        ))
    }
}

/// Box kernel (odd length, symmetric, taps in ascending offset order) for a radius.
pub fn box_kernel(radius: f32, rule: FracRule) -> Result<Vec<f32>, Error> {
    check_radius(radius)?;
    let n = radius.floor();
    let f = radius - n;
    let n = n as usize;
    let full = 2 * n + 1;
    if f == 0.0 {
        return Ok(vec![1.0 / full as f32; full]);
    }
    let len = full + 2;
    Ok(match rule {
        FracRule::EndTaps => {
            let total = full as f32 + f + f;
            (0..len)
                .map(|i| {
                    if i == 0 || i == len - 1 {
                        f / total
                    } else {
                        1.0 / total
                    }
                })
                .collect()
        }
        FracRule::Lerp => {
            let a = 1.0 / full as f32;
            let b = 1.0 / (full + 2) as f32;
            let inner = (1.0 - f) * a + f * b;
            (0..len)
                .map(|i| if i == 0 || i == len - 1 { f * b } else { inner })
                .collect()
        }
    })
}

/// Normalised Gaussian kernel for `spec` under the truncation, exponential and radius rules of
/// `cfg`. Sigma 0 gives the identity kernel `[1.0]`.
pub fn gaussian_kernel(spec: GaussSpec, cfg: &Config) -> Result<Vec<f32>, Error> {
    if !(cfg.trunc_k.is_finite() && cfg.trunc_k > 0.0) {
        return Err(Error::BadParameter("trunc_k must be finite and positive"));
    }
    if cfg.exp_terms == 0 {
        return Err(Error::BadParameter("exp_terms must be at least 1"));
    }
    let sigma = match spec {
        GaussSpec::Sigma(s) => s,
        GaussSpec::Radius(r) => {
            if !(cfg.radius_to_sigma.is_finite() && cfg.radius_to_sigma > 0.0) {
                return Err(Error::BadParameter(
                    "radius_to_sigma must be finite and positive",
                ));
            }
            r / cfg.radius_to_sigma
        }
    };
    if !(sigma.is_finite() && sigma >= 0.0) {
        return Err(Error::BadParameter("sigma must be finite and non-negative"));
    }
    if sigma == 0.0 {
        return Ok(vec![1.0]);
    }
    let reach = (cfg.trunc_k * sigma).ceil();
    if reach > MAX_HALF as f32 || reach.is_nan() {
        return Err(Error::BadParameter("kernel half width above 2^20"));
    }
    let hw = reach as usize;
    let denom = 2.0 * sigma * sigma;
    let mut half = Vec::with_capacity(hw + 1);
    match cfg.exp_method {
        ExpMethod::Series => {
            // The centre weight is exactly 1: with `denom` underflowed to 0, `0 / 0` would be NaN.
            half.push(1.0);
            for x in 1..=hw {
                let xf = x as f32;
                half.push(exp_neg(xf * xf / denom, cfg.exp_terms));
            }
        }
        ExpMethod::Recurrence => {
            let q = exp_neg(1.0 / denom, cfg.exp_terms);
            let q2 = q * q;
            let mut g = 1.0f32;
            let mut r = q;
            half.push(g);
            for _ in 0..hw {
                g *= r;
                r *= q2;
                half.push(g);
            }
        }
    }
    let mut k: Vec<f32> = half[1..].iter().rev().copied().collect();
    k.extend_from_slice(&half);
    let mut sum = 0.0f32;
    for &w in &k {
        sum += w;
    }
    if sum <= 0.0 || sum.is_nan() {
        return Err(Error::BadParameter("kernel weights sum to zero"));
    }
    for w in &mut k {
        *w /= sum;
    }
    Ok(k)
}

fn check_dims(len: usize, w: usize, h: usize) -> Result<(), Error> {
    if w.checked_mul(h) == Some(len) {
        Ok(())
    } else {
        Err(Error::SizeMismatch)
    }
}

/// Separable convolution: rows with `kernel`, then columns with `kernel`. The kernel length must
/// be odd. Taps accumulate in ascending offset order in `f32`; with `Border::Zero` an outside
/// tap adds nothing.
pub fn convolve_separable(
    src: &[f32],
    w: usize,
    h: usize,
    kernel: &[f32],
    border: Border,
) -> Result<Vec<f32>, Error> {
    check_dims(src.len(), w, h)?;
    if kernel.len().is_multiple_of(2) {
        return Err(Error::BadParameter("kernel length must be odd"));
    }
    if w == 0 || h == 0 {
        return Ok(Vec::new());
    }
    let half = (kernel.len() / 2) as i64;
    let mut tmp = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0f32;
            for (ki, &wt) in kernel.iter().enumerate() {
                if let Some(sx) = resolve_index(x as i64 + ki as i64 - half, w, border) {
                    acc += wt * src[y * w + sx];
                }
            }
            tmp[y * w + x] = acc;
        }
    }
    let mut out = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0f32;
            for (ki, &wt) in kernel.iter().enumerate() {
                if let Some(sy) = resolve_index(y as i64 + ki as i64 - half, h, border) {
                    acc += wt * tmp[sy * w + x];
                }
            }
            out[y * w + x] = acc;
        }
    }
    Ok(out)
}

/// Separable box blur of a `w * h` row major buffer.
pub fn box_blur(
    src: &[f32],
    w: usize,
    h: usize,
    radius: f32,
    cfg: &Config,
) -> Result<Vec<f32>, Error> {
    let k = box_kernel(radius, cfg.frac_rule)?;
    convolve_separable(src, w, h, &k, cfg.border)
}

/// `passes` successive box blurs (0 passes returns a copy).
pub fn box_blur_repeated(
    src: &[f32],
    w: usize,
    h: usize,
    radius: f32,
    passes: u32,
    cfg: &Config,
) -> Result<Vec<f32>, Error> {
    check_dims(src.len(), w, h)?;
    let k = box_kernel(radius, cfg.frac_rule)?;
    let mut cur = src.to_vec();
    for _ in 0..passes {
        cur = convolve_separable(&cur, w, h, &k, cfg.border)?;
    }
    Ok(cur)
}

/// Separable Gaussian blur of a `w * h` row major buffer.
pub fn gaussian_blur(
    src: &[f32],
    w: usize,
    h: usize,
    spec: GaussSpec,
    cfg: &Config,
) -> Result<Vec<f32>, Error> {
    let k = gaussian_kernel(spec, cfg)?;
    convolve_separable(src, w, h, &k, cfg.border)
}
