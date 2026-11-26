//! Brightness correction models.

use crate::config::*;
use crate::geom::*;
use crate::types::*;

/// Fraction of residuals that are inliers.
pub const INLIER_FRACTION: f32 = 0.8;
/// Variance term of the interpolated model.
pub const GAIN_VARIANCE: f32 = 0.001;

/// Gain ratios of one sample points (interpolated model).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CtxGain {
    pub x: i32,
    pub y: i32,
    pub w: [f32; 3],
}

/// Correction model chosen per synthesis; per source data is indexed by the source position in the
/// source list given to it.
#[derive(Clone, Debug)]
pub enum IllumSet {
    None,
    Linear(Vec<[[f32; 4]; 4]>),
    Interpolated(Vec<Vec<CtxGain>>),
}

/// Apply the illumination transform of source `k` to a colour at pixel (px, py); alpha is kept.
pub fn apply_illum(
    set: &IllumSet,
    k: usize,
    c: [f32; 4],
    px: i32,
    py: i32,
    r: &FillConfig,
) -> Res<[f32; 4]> {
    match set {
        IllumSet::None => Ok(c),
        IllumSet::Linear(ms) => {
            let m = ms
                .get(k)
                .ok_or(RemoveError::Invalid("no matrix for this source"))?;
            let mut o = [0.0f32; 4];
            for (i, slot) in o.iter_mut().enumerate() {
                *slot = match r.matrix_layout {
                    MatrixLayout::ColumnVector => {
                        m[i][0] * c[0] + m[i][1] * c[1] + m[i][2] * c[2] + m[i][3] * c[3]
                    }
                    MatrixLayout::RowVector => {
                        m[0][i] * c[0] + m[1][i] * c[1] + m[2][i] * c[2] + m[3][i] * c[3]
                    }
                };
            }
            o[3] = c[3];
            Ok(o)
        }
        IllumSet::Interpolated(per) => {
            let ctx = per
                .get(k)
                .ok_or(RemoveError::Invalid("no context gains for this source"))?;
            let g = interpolated_gain(ctx, px, py);
            Ok([c[0] * g[0], c[1] * g[1], c[2] * g[2], c[3]])
        }
    }
}

/// Render time gain of the interpolated model.
pub fn interpolated_gain(ctx: &[CtxGain], px: i32, py: i32) -> [f32; 3] {
    let mut sum = [0.0f32; 3];
    let mut sw = 0.0f32;
    let mut any = false;
    for g in ctx {
        if g.w == [0.0, 0.0, 0.0] {
            continue;
        }
        let dx = (px - g.x) as f32;
        let dy = (py - g.y) as f32;
        let d2 = dx * dx + dy * dy;
        if d2 == 0.0 {
            return g.w;
        }
        let wt = 1.0f32 / (d2 * d2);
        for i in 0..3 {
            sum[i] += g.w[i] * wt;
        }
        sw += wt;
        any = true;
    }
    if !any {
        return [1.0, 1.0, 1.0];
    }
    [sum[0] / sw, sum[1] / sw, sum[2] / sw]
}

/// Per component ratio for one sample point, sqrt((X^2 + 0.001) / (Y^2 + 0.001)), 0 where Y is
/// all zero in the first three components.
pub fn gain_ratio(x: [f32; 4], y: [f32; 4]) -> [f32; 3] {
    if y[0] == 0.0 && y[1] == 0.0 && y[2] == 0.0 {
        return [0.0; 3];
    }
    let mut w = [0.0f32; 3];
    for i in 0..3 {
        w[i] = ((x[i] * x[i] + GAIN_VARIANCE) / (y[i] * y[i] + GAIN_VARIANCE)).sqrt();
    }
    w
}

/// One sample points pair of the linear fit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContextSample {
    pub x: [f32; 4],
    pub y: [f32; 4],
    pub geometric_reject: bool,
}

/// 4x4 multiply with optional transposes.
pub fn mat4_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4], ta: bool, tb: bool) -> [[f32; 4]; 4] {
    let mut o = [[0.0f32; 4]; 4];
    for (i, row) in o.iter_mut().enumerate() {
        for (j, slot) in row.iter_mut().enumerate() {
            let mut s = 0.0f32;
            for k in 0..4 {
                let av = if ta { a[k][i] } else { a[i][k] };
                let bv = if tb { b[j][k] } else { b[k][j] };
                s += av * bv;
            }
            *slot = s;
        }
    }
    o
}

/// out[i][j] = sum over included n (ascending) of a[n][i] * b[n][j]: Y^T Y or Y^T X.
pub fn gram(a: &[[f32; 4]], b: &[[f32; 4]], include: &[bool]) -> [[f32; 4]; 4] {
    let mut o = [[0.0f32; 4]; 4];
    for n in 0..a.len() {
        if !include[n] {
            continue;
        }
        for i in 0..4 {
            for j in 0..4 {
                o[i][j] += a[n][i] * b[n][j];
            }
        }
    }
    o
}

/// Diagonal solve: entry of Y^T X over entry of Y^T Y, 1 when the divisor is 0; other terms 0.
pub fn solve_normal_eq(yty: &[[f32; 4]; 4], ytx: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut m = [[0.0f32; 4]; 4];
    for i in 0..4 {
        m[i][i] = if yty[i][i] == 0.0 {
            1.0
        } else {
            ytx[i][i] / yty[i][i]
        };
    }
    m
}

/// Squared length of (A - B) over the first three components, A = the matrix applied to Y, B = X;
/// the largest float for geometric rejects.
pub fn residuals(samples: &[ContextSample], m: &[[f32; 4]; 4], r: &FillConfig) -> Vec<f32> {
    samples
        .iter()
        .map(|s| {
            if s.geometric_reject {
                return f32::MAX;
            }
            let a = apply_illum(&IllumSet::Linear(vec![*m]), 0, s.y, 0, 0, r).unwrap_or(s.y);
            let mut t = 0.0f32;
            for i in 0..3 {
                let d = a[i] - s.x[i];
                t += d * d;
            }
            t
        })
        .collect()
}

/// Trim limit: sorted residual at index floor(valid count * 0.8).
pub fn trim_limit(res: &[f32]) -> Res<f32> {
    let valid = res.iter().filter(|&&v| v != f32::MAX).count();
    if valid == 0 {
        return Err(RemoveError::Refused("no valid sample points for the fit"));
    }
    let mut sorted = res.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let idx = (valid as f32 * INLIER_FRACTION).floor() as usize;
    Ok(sorted[idx.min(sorted.len() - 1)])
}

/// Robust per source linear fit: solve, filter photometric rejects, solve again (`fit_passes`).
pub fn fit_linear(samples: &[ContextSample], r: &FillConfig) -> Res<[[f32; 4]; 4]> {
    let xs: Vec<[f32; 4]> = samples.iter().map(|s| s.x).collect();
    let ys: Vec<[f32; 4]> = samples.iter().map(|s| s.y).collect();
    let mut include: Vec<bool> = samples.iter().map(|s| !s.geometric_reject).collect();
    let mut m = [[0.0f32; 4]; 4];
    for pass in 0..r.fit_passes.max(1) {
        if !include.iter().any(|&b| b) {
            return Err(RemoveError::Refused("no valid sample points for the fit"));
        }
        let yty = gram(&ys, &ys, &include);
        let ytx = gram(&ys, &xs, &include);
        m = solve_normal_eq(&yty, &ytx);
        if pass + 1 < r.fit_passes {
            let res = residuals(samples, &m, r);
            let thr = trim_limit(&res)?;
            for (i, inc) in include.iter_mut().enumerate() {
                *inc = !samples[i].geometric_reject && res[i] <= thr;
            }
        }
    }
    Ok(m)
}

/// Gaussian smoothed (sigma at least 0.5) or single bilinear tap sample.
pub fn smoothed_sample(img: &Image, sx: f32, sy: f32, r: &FillConfig) -> [f32; 4] {
    let sigma = r.smoothing_sigma;
    if sigma < 0.5 {
        return (r.sampler)(img, sx, sy);
    }
    let rad = (2.0f32 * sigma).ceil() as i32;
    let mut acc = [0.0f32; 4];
    let mut ws = 0.0f32;
    for dy in -rad..=rad {
        for dx in -rad..=rad {
            let d2 = (dx * dx + dy * dy) as f32;
            let w = (r.exp)(-d2 / (2.0f32 * sigma * sigma));
            let c = (r.sampler)(img, sx + dx as f32, sy + dy as f32);
            for k in 0..4 {
                acc[k] += c[k] * w;
            }
            ws += w;
        }
    }
    [acc[0] / ws, acc[1] / ws, acc[2] / ws, acc[3] / ws]
}

/// Inputs of the context constructors.
pub struct ContextInputs<'a> {
    pub target: &'a Image,
    pub tc: Camera,
    pub source: &'a Source,
    pub context: &'a Matte,
}

/// X and Y of one context pixel, or a geometric reject.
pub fn context_pair(ci: &ContextInputs, x: i32, y: i32, r: &FillConfig) -> Res<ContextSample> {
    let (tx, ty) = (
        (x - ci.target.win.l) as f32 + 0.5,
        (y - ci.target.win.t) as f32 + 0.5,
    );
    let xv = smoothed_sample(ci.target, tx, ty, r);
    let (px, py) = transfer_px(&ci.tc, &ci.source.cam, &ci.source.h, x, y)?;
    let on_from = ci.source.cover.win.contains(px, py) && ci.source.cover.get(px, py) != 0;
    let (qx, qy) = sample_position(&ci.tc, ci.source, x, y)?;
    let m = r.inside_margin;
    let sw = ci.source.pixels.win.w as f32;
    let sh = ci.source.pixels.win.h as f32;
    let inside = qx >= m && qy >= m && qx <= sw - m && qy <= sh - m;
    if !on_from || !inside {
        return Ok(ContextSample {
            x: xv,
            y: [0.0; 4],
            geometric_reject: true,
        });
    }
    Ok(ContextSample {
        x: xv,
        y: smoothed_sample(&ci.source.pixels, qx, qy, r),
        geometric_reject: false,
    })
}

/// Samples of the linear fit: pixels of the context matte on a step grid over its window.
pub fn build_linear_samples(ci: &ContextInputs, r: &FillConfig) -> Res<Vec<ContextSample>> {
    let w = ci.context.win;
    let step = r.context_step.max(1);
    let mut out = vec![];
    let mut y = w.t;
    while y < w.t + w.h {
        let mut x = w.l;
        while x < w.l + w.w {
            if ci.context.get(x, y) != 0 {
                out.push(context_pair(ci, x, y, r)?);
            }
            x += step;
        }
        y += step;
    }
    Ok(out)
}

/// Context values of the interpolated model: sample points with a 4 neighbour inside the target
/// matte (the internal boundary). Geometric rejects are left out (INFERRED).
pub fn build_interpolated_gains(
    ci: &ContextInputs,
    fill: &Matte,
    r: &FillConfig,
) -> Res<Vec<CtxGain>> {
    let w = ci.context.win;
    let mut out = vec![];
    for y in w.t..w.t + w.h {
        for x in w.l..w.l + w.w {
            if ci.context.get(x, y) == 0 {
                continue;
            }
            let near = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|&(dx, dy)| fill.get(x + dx, y + dy) != 0);
            if !near {
                continue;
            }
            let s = context_pair(ci, x, y, r)?;
            if s.geometric_reject {
                continue;
            }
            out.push(CtxGain {
                x,
                y,
                w: gain_ratio(s.x, s.y),
            });
        }
    }
    Ok(out)
}

/// The context matte construction is not given by the specification.
pub fn context_matte_from_layers() -> Res<Matte> {
    Err(RemoveError::Unspecified(
        "context matte construction and its host sequencing",
    ))
}
