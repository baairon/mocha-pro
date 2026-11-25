//! The resampling loop.

use crate::border::wrap;
use crate::config::{AccumOrder, AlphaMode, Config, Error, Filter, Footprint, Normalize, TieRule};
use crate::kernel::{bilinear, floor, keys, lanczos, mitchell};

/// Sample layout of a buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    /// Interleaved: index `(y * width + x) * channels + c`.
    Packed,
    /// One plane per channel: index `c * width * height + y * width + x`.
    Planar,
}

/// Borrowed source image.
#[derive(Clone, Copy, Debug)]
pub struct Image<'a> {
    pub data: &'a [f32],
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub layout: Layout,
}

/// Number of samples of a buffer, or an error for zero or overflowing dimensions.
fn sample_count(w: usize, h: usize, ch: usize) -> Result<usize, Error> {
    if ch == 0 {
        return Err(Error::BadDimensions);
    }
    w.checked_mul(h)
        .and_then(|n| n.checked_mul(ch))
        .ok_or(Error::BadDimensions)
}

fn index(layout: Layout, w: usize, h: usize, ch: usize, x: usize, y: usize, c: usize) -> usize {
    match layout {
        Layout::Packed => (y * w + x) * ch + c,
        Layout::Planar => c * w * h + y * w + x,
    }
}

/// Coordinates beyond this magnitude, and non finite ones, give a zero pixel.
const COORD_LIMIT: f64 = 1073741824.0;

fn valid(v: f64) -> bool {
    v.abs() <= COORD_LIMIT
}

/// Nearest index of `u` under a tie rule.
fn nearest_index(u: f64, tie: TieRule) -> i64 {
    let f = floor(u);
    let d = u - f;
    let fi = f as i64;
    if d < 0.5 {
        fi
    } else if d > 0.5 {
        fi + 1
    } else {
        match tie {
            TieRule::HalfUp => fi + 1,
            TieRule::HalfDown => fi,
            TieRule::HalfEven => {
                if fi.rem_euclid(2) == 0 {
                    fi
                } else {
                    fi + 1
                }
            }
        }
    }
}

/// Taps `(index, weight)` along one axis for source coordinate `u`; `extent` is the box width.
fn axis_taps(u: f64, cfg: &Config, extent: f64) -> Vec<(i64, f64)> {
    let f = floor(u) as i64;
    let radius = |r: i64| -> Vec<i64> { (f - r + 1..=f + r).collect() };
    match cfg.filter {
        Filter::Nearest => vec![(nearest_index(u, cfg.tie), 1.0)],
        Filter::Bilinear => radius(1)
            .into_iter()
            .map(|i| (i, bilinear(u - i as f64)))
            .collect(),
        Filter::Keys { a } => radius(2)
            .into_iter()
            .map(|i| (i, keys(u - i as f64, a)))
            .collect(),
        Filter::Mitchell { b, c } => radius(2)
            .into_iter()
            .map(|i| (i, mitchell(u - i as f64, b, c)))
            .collect(),
        Filter::Lanczos { lobes } => radius(i64::from(lobes))
            .into_iter()
            .map(|i| (i, lanczos(u - i as f64, lobes)))
            .collect(),
        Filter::Box => {
            // Pixel i covers [i - 0.5, i + 0.5] in index space.
            let lo = u - extent * 0.5;
            let hi = u + extent * 0.5;
            let first = floor(lo + 0.5) as i64;
            let last = floor(hi + 0.5) as i64;
            (first..=last)
                .filter_map(|i| {
                    let a = if lo > i as f64 - 0.5 {
                        lo
                    } else {
                        i as f64 - 0.5
                    };
                    let b = if hi < i as f64 + 0.5 {
                        hi
                    } else {
                        i as f64 + 0.5
                    };
                    let o = b - a;
                    if o > 0.0 {
                        Some((i, o / extent))
                    } else {
                        None
                    }
                })
                .collect()
        }
    }
}

/// Resample `src` into a `dst_w` by `dst_h` buffer of the same layout and channel count.
///
/// For output pixel `(X, Y)` the mapping is called with `(X + o, Y + o)` where `o` is the
/// centre offset of [`Config::center`] and returns the source coordinate in the same
/// convention. The result is rounded to `f32` once, at the end. Output pixels whose mapped
/// coordinate is not finite or exceeds 2^30 in magnitude are zero.
/// Filter outputs are not clamped: cubic and Lanczos kernels may overshoot.
pub fn resample<M>(
    src: &Image<'_>,
    dst_w: usize,
    dst_h: usize,
    map: M,
    cfg: &Config,
) -> Result<Vec<f32>, Error>
where
    M: Fn(f64, f64) -> (f64, f64),
{
    let ch = src.channels;
    if src.width == 0 || src.height == 0 {
        return Err(Error::BadDimensions);
    }
    if sample_count(src.width, src.height, ch)? != src.data.len() {
        return Err(Error::BadLength);
    }
    cfg.validate(ch)?;
    let mut out = vec![0f32; sample_count(dst_w, dst_h, ch)?];
    let off = cfg.center.offset();
    let premul = cfg.alpha == AlphaMode::Premultiplied;
    let mut acc = vec![0f64; ch];
    for y in 0..dst_h {
        for x in 0..dst_w {
            let px = x as f64 + off;
            let py = y as f64 + off;
            let (sx, sy) = map(px, py);
            if !valid(sx) || !valid(sy) {
                continue;
            }
            let (fw, fh) = match (cfg.filter, cfg.footprint) {
                (Filter::Box, Footprint::Unit) => (1.0, 1.0),
                (Filter::Box, Footprint::Fixed { w, h }) => (w, h),
                (Filter::Box, Footprint::Jacobian) => {
                    let (rx, ry) = map(px + 1.0, py);
                    let (dx, dy) = map(px, py + 1.0);
                    let w = (rx - sx).abs() + (dx - sx).abs();
                    let h = (ry - sy).abs() + (dy - sy).abs();
                    if !valid(w) || !valid(h) {
                        continue;
                    }
                    (
                        if w > cfg.min_extent {
                            w
                        } else {
                            cfg.min_extent
                        },
                        if h > cfg.min_extent {
                            h
                        } else {
                            cfg.min_extent
                        },
                    )
                }
                _ => (1.0, 1.0),
            };
            let xt = axis_taps(sx - off, cfg, fw);
            let yt = axis_taps(sy - off, cfg, fh);
            acc.iter_mut().for_each(|a| *a = 0.0);
            let mut wsum = 0.0f64;
            let mut wib = 0.0f64;
            let mut add = |ix: i64, wx: f64, iy: i64, wy: f64| {
                let w = wx * wy;
                wsum += w;
                let inside =
                    (0..src.width as i64).contains(&ix) && (0..src.height as i64).contains(&iy);
                if inside {
                    wib += w;
                }
                if w == 0.0 {
                    // A tap of weight exactly zero must not read the sample: 0 * inf or
                    // 0 * NaN would spread a non finite sample to pixels that do not use it.
                    return;
                }
                let (Some(cx), Some(cy)) = (
                    wrap(ix, src.width, cfg.border),
                    wrap(iy, src.height, cfg.border),
                ) else {
                    return;
                };
                let a = cfg.alpha_channel.map(|c| {
                    f64::from(src.data[index(src.layout, src.width, src.height, ch, cx, cy, c)])
                });
                for (c, slot) in acc.iter_mut().enumerate() {
                    let mut v = f64::from(
                        src.data[index(src.layout, src.width, src.height, ch, cx, cy, c)],
                    );
                    if let (true, Some(al)) = (premul, a) {
                        if Some(c) != cfg.alpha_channel {
                            v *= al;
                        }
                    }
                    *slot += w * v;
                }
            };
            match cfg.order {
                AccumOrder::RowMajor => {
                    for &(iy, wy) in &yt {
                        for &(ix, wx) in &xt {
                            add(ix, wx, iy, wy);
                        }
                    }
                }
                AccumOrder::ColumnMajor => {
                    for &(ix, wx) in &xt {
                        for &(iy, wy) in &yt {
                            add(ix, wx, iy, wy);
                        }
                    }
                }
            }
            let denom = match cfg.normalize {
                Normalize::None => None,
                Normalize::Sum => Some(wsum),
                Normalize::SumInBounds => Some(wib),
            };
            if let Some(d) = denom {
                for a in acc.iter_mut() {
                    *a = if d == 0.0 { 0.0 } else { *a / d };
                }
            }
            if let (true, Some(ac)) = (premul, cfg.alpha_channel) {
                let al = acc[ac];
                for (c, a) in acc.iter_mut().enumerate() {
                    if c != ac {
                        *a = if al == 0.0 { 0.0 } else { *a / al };
                    }
                }
            }
            for (c, a) in acc.iter().enumerate() {
                out[index(src.layout, dst_w, dst_h, ch, x, y, c)] = *a as f32;
            }
        }
    }
    Ok(out)
}

/// A 3 by 3 matrix acting on homogeneous coordinates, row major: the inverse mapping from
/// output to source coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix3(pub [[f64; 3]; 3]);

impl Matrix3 {
    pub const IDENTITY: Matrix3 = Matrix3([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

    /// Source = output + `(tx, ty)`.
    pub fn translation(tx: f64, ty: f64) -> Self {
        Matrix3([[1.0, 0.0, tx], [0.0, 1.0, ty], [0.0, 0.0, 1.0]])
    }

    /// Source = output scaled by `(sx, sy)`.
    pub fn scale(sx: f64, sy: f64) -> Self {
        Matrix3([[sx, 0.0, 0.0], [0.0, sy, 0.0], [0.0, 0.0, 1.0]])
    }

    /// Apply to a point: `x' = (m00 x + m01 y + m02) / w`, `y'` likewise,
    /// `w = m20 x + m21 y + m22`. The sums are evaluated left to right. A zero `w` gives
    /// NaN coordinates (a zero output pixel).
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let m = &self.0;
        let w = m[2][0] * x + m[2][1] * y + m[2][2];
        if w == 0.0 {
            return (f64::NAN, f64::NAN);
        }
        let nx = m[0][0] * x + m[0][1] * y + m[0][2];
        let ny = m[1][0] * x + m[1][1] * y + m[1][2];
        if w == 1.0 {
            (nx, ny)
        } else {
            (nx / w, ny / w)
        }
    }
}

/// [`resample`] with the mapping given as a [`Matrix3`].
pub fn resample_matrix(
    src: &Image<'_>,
    dst_w: usize,
    dst_h: usize,
    m: &Matrix3,
    cfg: &Config,
) -> Result<Vec<f32>, Error> {
    resample(src, dst_w, dst_h, |x, y| m.apply(x, y), cfg)
}
