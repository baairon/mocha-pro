//! Distance based feather: a ramp over the signed distance to the zero crossing of a hard mask.
//!
//! The distance is the exact Euclidean distance transform of `edt` (Felzenszwalb and Huttenlocher
//! 2012). Squared distances are integers; the square root is the IEEE operation.
//!
//! Ramp profiles: linear; smoothstep `t^2 (3 - 2t)` (the cubic Hermite blend defined for
//! `smoothstep` in the OpenGL Shading Language specification, built-in functions); and the raised
//! cosine `0.5 - 0.5 cos(pi t) = sin^2(pi t / 2)` with `sin` by its Maclaurin series
//! (Abramowitz and Stegun 1964, section 4.3) cut at a fixed degree.

use crate::border::Border;
use crate::config::{Config, Direction, Profile};
use crate::edt::{squared_distance_transform, NO_FEATURE};
use crate::Error;

/// Ramp profile of `t`, with `t` clamped to `[0, 1]`; 0 maps to 0 and 1 maps to 1 exactly.
pub fn ramp(t: f32, profile: Profile) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match profile {
        Profile::Linear => t,
        Profile::Smoothstep => t * t * (3.0 - 2.0 * t),
        Profile::Cosine => {
            if t <= 0.5 {
                sin_sq_half_pi(t)
            } else {
                1.0 - sin_sq_half_pi(1.0 - t)
            }
        }
    }
}

/// `sin(pi t / 2)^2` for `t` in `[0, 0.5]`: the argument is at most `pi / 4`, and the series runs
/// to degree 13.
fn sin_sq_half_pi(t: f32) -> f32 {
    let y = std::f32::consts::FRAC_PI_2 * t;
    let y2 = y * y;
    let mut term = y;
    let mut sum = y;
    for i in 1..=6u32 {
        term = term * y2 / ((2 * i) * (2 * i + 1)) as f32;
        if i % 2 == 1 {
            sum -= term;
        } else {
            sum += term;
        }
    }
    sum * sum
}

/// Signed distance (positive inside) from each sample to the zero crossing of the hard mask
/// `src >= cfg.threshold`: for an inside sample, the distance to the nearest outside sample centre
/// minus `cfg.edge_offset`; for an outside sample, minus the distance to the nearest inside sample
/// centre plus `cfg.edge_offset`. `Border::Zero` treats everything beyond the buffer as outside;
/// the other rules put no edge at the buffer border (reflections never come nearer than the
/// buffer's own samples, so Clamp, Mirror and MirrorNoRepeat agree). A mask with no crossing gives
/// positive infinity (all inside) or negative infinity (all outside).
pub fn signed_distance(src: &[f32], w: usize, h: usize, cfg: &Config) -> Result<Vec<f32>, Error> {
    if w.checked_mul(h) != Some(src.len()) {
        return Err(Error::SizeMismatch);
    }
    if w == 0 || h == 0 {
        return Ok(Vec::new());
    }
    if w > crate::edt::MAX_SIDE - 2 || h > crate::edt::MAX_SIDE - 2 {
        return Err(Error::BadParameter("side too large"));
    }
    let pad = usize::from(cfg.border == Border::Zero);
    let (pw, ph) = (w + 2 * pad, h + 2 * pad);
    let mut inside = vec![false; pw * ph];
    for y in 0..h {
        for x in 0..w {
            inside[(y + pad) * pw + x + pad] = src[y * w + x] >= cfg.threshold;
        }
    }
    let outside: Vec<bool> = inside.iter().map(|&b| !b).collect();
    let to_out = squared_distance_transform(&outside, pw, ph);
    let to_in = squared_distance_transform(&inside, pw, ph);
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let i = (y + pad) * pw + x + pad;
            out.push(if inside[i] {
                match to_out[i] {
                    NO_FEATURE => f32::INFINITY,
                    d => (d as f64).sqrt() as f32 - cfg.edge_offset,
                }
            } else {
                match to_in[i] {
                    NO_FEATURE => f32::NEG_INFINITY,
                    d => -((d as f64).sqrt() as f32 - cfg.edge_offset),
                }
            });
        }
    }
    Ok(out)
}

/// Feather of a hard mask over `width` samples of signed distance: `ramp((s - lo) / width)` where
/// `s` is the signed distance and `lo` is 0 (Inward), `-width` (Outward) or `-width / 2` (Centred).
pub fn feather(
    src: &[f32],
    w: usize,
    h: usize,
    width: f32,
    cfg: &Config,
) -> Result<Vec<f32>, Error> {
    if !(width.is_finite() && width > 0.0) {
        return Err(Error::BadParameter("width must be finite and positive"));
    }
    let lo = match cfg.direction {
        Direction::Inward => 0.0,
        Direction::Outward => -width,
        Direction::Centred => -0.5 * width,
    };
    let sd = signed_distance(src, w, h, cfg)?;
    Ok(sd
        .iter()
        .map(|&s| ramp((s - lo) / width, cfg.profile))
        .collect())
}
