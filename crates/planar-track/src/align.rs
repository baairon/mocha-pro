//! Alignment of one frame level to the template level: the Lucas-Kanade family with
//! Levenberg-Marquardt damping, one level at a time.
//!
//! Residual (all rules): `r(x) = I(W(x)) - ((1 + gain) T(x) + offset)` over the selected template
//! pixels `x`, weighted by `w(x)`; the cost is `sum w r^2`. With `C` the matrix whose row for a
//! pixel is `c(x)` (pose columns, then, when the illumination is solved jointly, `-T(x)` and
//! `-1`), the damped Gauss-Newton step `s` solves `(A + lambda D) s = -b` with `A = C^T W C`,
//! `b = C^T W r` (Levenberg 1944; Marquardt 1963; Baker and Matthews 2004, section 4.1 for
//! Gauss-Newton and section 4.5 for Levenberg-Marquardt). The pose columns of the rule:
//!
//! * forward additive (Baker and Matthews 2004, section 2.2): `grad I(W) dW/dp` at the current
//!   parameters, the pose updated by `p <- p + s`;
//! * forward compositional (section 3.1): `grad I(W) dW/dx dW/dp(0)`, the pose updated by
//!   `H <- H * H(s)`;
//! * inverse compositional (section 3.2): `-(1 + gain) grad T dW/dp(0)`, computed once per
//!   template level, the pose updated by `H <- H * H(s)^-1`;
//! * efficient second order minimisation (Benhimane and Malis 2004): the mean of the
//!   forward compositional and the inverse compositional columns,
//!   `(J_I + (1 + gain) J_T) / 2`, the pose updated as forward compositional.
//!
//! Here `p` is the parameter vector of [`crate::model`], `W` the pose in the coordinates of this
//! level, and gradients are sampled bilinearly from gradient images.

use crate::config::{Acceptance, Config, Damping, Illumination, LevelParams, UpdateRule};
use crate::edt::distance_to_outside;
use crate::error::Error;
use crate::linalg::solve_spd;
use crate::model::{Dof, Pose};
use crate::pyramid::Level;
use crate::region::Region;

/// Why a level stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The largest corner motion of an accepted step fell below the step tolerance.
    StepTol,
    /// The relative cost fall fell below the cost tolerance, or the cost is zero.
    CostTol,
    /// Lambda exceeded its limit: no damped step improves the cost.
    LambdaMax,
    /// The iteration limit was reached.
    MaxIterations,
    /// Too many consecutive rejected steps.
    MaxRejections,
    /// Too few template pixels fall inside the frame.
    NoPixels,
}

impl StopReason {
    /// True for the reasons that count as converged.
    pub fn converged(self) -> bool {
        matches!(
            self,
            StopReason::StepTol | StopReason::CostTol | StopReason::LambdaMax
        )
    }
}

/// Selected template pixels of one pyramid level.
#[derive(Debug, Clone)]
pub struct LevelTemplate {
    /// Level to level 0 map of the template level, `x0 = sx x + ox`.
    pub level_to_base: [f64; 9],
    /// Inverse of the above.
    pub base_to_level: [f64; 9],
    /// The region corners in level coordinates.
    pub corners: Vec<(f64, f64)>,
    /// Pixel columns.
    pub x: Vec<f64>,
    /// Pixel rows.
    pub y: Vec<f64>,
    /// Template grey values.
    pub t: Vec<f64>,
    /// Weights.
    pub w: Vec<f64>,
    /// `grad T dW/dp(0)`, `n` numbers per pixel, for the model of the configuration.
    pub jt: Vec<f64>,
    /// Number of mask pixels before selection.
    pub candidates: usize,
}

/// A frame level with its gradient images.
#[derive(Debug, Clone)]
pub struct FrameLevel {
    /// Level image.
    pub img: crate::image::Image,
    /// Horizontal gradient image.
    pub gx: crate::image::Image,
    /// Vertical gradient image.
    pub gy: crate::image::Image,
    /// Level to level 0 map.
    pub level_to_base: [f64; 9],
    /// Level 0 to level map.
    pub base_to_level: [f64; 9],
}

impl FrameLevel {
    /// Build from a pyramid level.
    pub fn new(level: &Level) -> FrameLevel {
        let (gx, gy) = level.image.gradients();
        FrameLevel {
            img: level.image.clone(),
            gx,
            gy,
            level_to_base: level.to_base(),
            base_to_level: level.from_base(),
        }
    }
}

/// The gradient of the identity-point warp with respect to `q` for a point (two rows of eight).
pub fn dwdq_identity(x: f64, y: f64) -> [[f64; 8]; 2] {
    [
        [1.0, 0.0, x, y, 0.0, 0.0, -x * x, -x * y],
        [0.0, 1.0, 0.0, 0.0, x, y, -x * y, -y * y],
    ]
}

fn fold(dof_m: &[f64], n: usize, row8: &[f64; 8], out: &mut [f64]) {
    for (k, o) in out.iter_mut().enumerate().take(n) {
        let mut s = 0.0;
        for (j, r) in row8.iter().enumerate() {
            s += r * dof_m[j * n + k];
        }
        *o = s;
    }
}

impl LevelTemplate {
    /// Select and weight the template pixels of a level (see [`crate::config::SelectionConfig`]).
    pub fn build(level: &Level, region: &Region, cfg: &Config) -> Result<LevelTemplate, Error> {
        let img = &level.image;
        let (w, h) = (img.width(), img.height());
        let (rw, rh) = (region.width() as i64, region.height() as i64);
        let mut mask = vec![false; w * h];
        for y in 0..h {
            for x in 0..w {
                let bx = (level.sx * x as f64 + level.ox + 0.5).floor() as i64;
                let by = (level.sy * y as f64 + level.oy + 0.5).floor() as i64;
                let bx = bx.clamp(0, rw - 1) as usize;
                let by = by.clamp(0, rh - 1) as usize;
                mask[y * w + x] = region.mask()[by * region.width() + bx];
            }
        }
        let (gx, gy) = img.gradients();
        let mut cand: Vec<(f64, usize)> = Vec::new();
        for (i, &m) in mask.iter().enumerate() {
            if m {
                let a = gx.data()[i];
                let b = gy.data()[i];
                // The squared magnitude orders pixels like the magnitude does.
                cand.push((a * a + b * b, i));
            }
        }
        let candidates = cand.len();
        if candidates == 0 {
            return Err(Error::TooFewPixels);
        }
        let want =
            ((cfg.selection.min_fraction * candidates as f64).ceil() as usize).clamp(1, candidates);
        cand.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut chosen: Vec<usize> = cand.iter().take(want).map(|c| c.1).collect();
        chosen.sort_unstable();

        let dist = match cfg.selection.boundary {
            crate::config::BoundaryWeight::Off => None,
            _ => Some(distance_to_outside(
                &mask,
                w,
                h,
                cfg.selection.border_is_outside,
            )),
        };
        let dof = cfg.dof;
        let n = dof.count();
        let dof_m = dof.matrix();
        let mut lt = LevelTemplate {
            level_to_base: level.to_base(),
            base_to_level: level.from_base(),
            corners: region
                .corners()
                .iter()
                .map(|&(cx, cy)| ((cx - level.ox) / level.sx, (cy - level.oy) / level.sy))
                .collect(),
            x: Vec::new(),
            y: Vec::new(),
            t: Vec::new(),
            w: Vec::new(),
            jt: Vec::new(),
            candidates,
        };
        for &i in &chosen {
            let px = (i % w) as f64;
            let py = (i / w) as f64;
            let wt = match &dist {
                None => 1.0,
                Some(d) => {
                    let v = cfg.selection.boundary.weight(d[i]);
                    if v < cfg.selection.min_weight {
                        cfg.selection.min_weight
                    } else {
                        v
                    }
                }
            };
            let id = dwdq_identity(px, py);
            let (a, b) = (gx.data()[i], gy.data()[i]);
            let mut row8 = [0.0; 8];
            for (j, r) in row8.iter_mut().enumerate() {
                *r = a * id[0][j] + b * id[1][j];
            }
            let mut row = vec![0.0; n];
            fold(&dof_m, n, &row8, &mut row);
            lt.x.push(px);
            lt.y.push(py);
            lt.t.push(img.data()[i]);
            lt.w.push(wt);
            lt.jt.extend_from_slice(&row);
        }
        Ok(lt)
    }

    /// Number of selected pixels.
    pub fn len(&self) -> usize {
        self.x.len()
    }

    /// True when no pixel is selected (never the case for a built template).
    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }
}

/// Normal equations of one linearisation.
#[derive(Debug, Clone)]
pub struct Lin {
    /// `A`, `m * m`.
    pub a: Vec<f64>,
    /// `b`, `m`.
    pub b: Vec<f64>,
    /// `sum w r^2`.
    pub cost: f64,
    /// `sum w`.
    pub wsum: f64,
    /// Pixels inside the frame.
    pub valid: usize,
}

impl Lin {
    /// The weighted mean squared residual.
    pub fn mean(&self) -> f64 {
        if self.wsum > 0.0 {
            self.cost / self.wsum
        } else {
            f64::INFINITY
        }
    }
}

/// Result of one level.
#[derive(Debug, Clone)]
pub struct LevelOutcome {
    /// Pose in level coordinates.
    pub pose: Pose,
    /// Gain.
    pub gain: f64,
    /// Offset.
    pub offset: f64,
    /// Iterations used (accepted and rejected).
    pub iterations: usize,
    /// Why the level stopped.
    pub stop: StopReason,
    /// Final weighted mean squared residual (infinite when no pixel was usable).
    pub mean_cost: f64,
}

/// The solver for one level.
pub struct Solver<'a> {
    cfg: &'a Config,
    tpl: &'a LevelTemplate,
    fr: &'a FrameLevel,
    dof_m: Vec<f64>,
    n: usize,
    gain_cols: bool,
}

impl<'a> Solver<'a> {
    /// A solver for a template level and a frame level.
    pub fn new(cfg: &'a Config, tpl: &'a LevelTemplate, fr: &'a FrameLevel) -> Solver<'a> {
        Solver {
            cfg,
            tpl,
            fr,
            dof_m: cfg.dof.matrix(),
            n: cfg.dof.count(),
            gain_cols: cfg.illumination == Illumination::Joint,
        }
    }

    /// Size of the linear system.
    pub fn system_size(&self) -> usize {
        self.n + if self.gain_cols { 2 } else { 0 }
    }

    /// Residual and normal equations at a pose, gain and offset. With `jac` false only the cost
    /// is accumulated.
    pub fn linearize(&self, pose: &Pose, gain: f64, offset: f64, jac: bool) -> Lin {
        let n = self.n;
        let m = self.system_size();
        let h = &pose.h;
        let rule = self.cfg.rule;
        let mut lin = Lin {
            a: vec![0.0; m * m],
            b: vec![0.0; m],
            cost: 0.0,
            wsum: 0.0,
            valid: 0,
        };
        let scale = 1.0 + gain;
        let mut c = vec![0.0; m];
        let mut ji = vec![0.0; n];
        for i in 0..self.tpl.len() {
            let (x, y) = (self.tpl.x[i], self.tpl.y[i]);
            let zz = h[6] * x + h[7] * y + h[8];
            if zz.is_nan() || zz <= 1e-9 {
                continue;
            }
            let xx = h[0] * x + h[1] * y + h[2];
            let yy = h[3] * x + h[4] * y + h[5];
            let (u, v) = (xx / zz, yy / zz);
            let val = match self.fr.img.sample_inside(u, v) {
                Some(val) => val,
                None => continue,
            };
            let t = self.tpl.t[i];
            let r = val - (scale * t + offset);
            let wt = self.tpl.w[i];
            lin.valid += 1;
            lin.cost += wt * r * r;
            lin.wsum += wt;
            if !jac {
                continue;
            }
            let jt = &self.tpl.jt[i * n..(i + 1) * n];
            if rule != UpdateRule::InverseCompositional {
                let a = self.fr.gx.sample_clamped(u, v);
                let b = self.fr.gy.sample_clamped(u, v);
                let mut row8 = [0.0; 8];
                if rule == UpdateRule::ForwardAdditive {
                    let z2 = zz * zz;
                    let d0 = [
                        1.0 / zz,
                        0.0,
                        x / zz,
                        y / zz,
                        0.0,
                        0.0,
                        -xx * x / z2,
                        -xx * y / z2,
                    ];
                    let d1 = [
                        0.0,
                        1.0 / zz,
                        0.0,
                        0.0,
                        x / zz,
                        y / zz,
                        -yy * x / z2,
                        -yy * y / z2,
                    ];
                    for (j, r8) in row8.iter_mut().enumerate() {
                        *r8 = a * d0[j] + b * d1[j];
                    }
                } else {
                    let z2 = zz * zz;
                    let j00 = (h[0] * zz - xx * h[6]) / z2;
                    let j01 = (h[1] * zz - xx * h[7]) / z2;
                    let j10 = (h[3] * zz - yy * h[6]) / z2;
                    let j11 = (h[4] * zz - yy * h[7]) / z2;
                    let ga = a * j00 + b * j10;
                    let gb = a * j01 + b * j11;
                    let id = dwdq_identity(x, y);
                    for (j, r8) in row8.iter_mut().enumerate() {
                        *r8 = ga * id[0][j] + gb * id[1][j];
                    }
                }
                fold(&self.dof_m, n, &row8, &mut ji);
            }
            for k in 0..n {
                c[k] = match rule {
                    UpdateRule::ForwardAdditive | UpdateRule::ForwardCompositional => ji[k],
                    UpdateRule::InverseCompositional => -(scale * jt[k]),
                    UpdateRule::Esm => 0.5 * (ji[k] + scale * jt[k]),
                };
            }
            if self.gain_cols {
                c[n] = -t;
                c[n + 1] = -1.0;
            }
            for p in 0..m {
                let wc = wt * c[p];
                lin.b[p] += wc * r;
                for (q, cq) in c.iter().enumerate() {
                    lin.a[p * m + q] += wc * cq;
                }
            }
        }
        lin
    }

    /// Closed form weighted least squares gain and offset for a pose: the minimiser over
    /// `(gain, offset)` of `sum w (I(W) - ((1 + gain) T + offset))^2`. The given values are
    /// returned when the system is degenerate.
    pub fn regress(&self, pose: &Pose, gain: f64, offset: f64) -> (f64, f64) {
        let h = &pose.h;
        let (mut stt, mut st, mut sw, mut sit, mut si) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for i in 0..self.tpl.len() {
            let (x, y) = (self.tpl.x[i], self.tpl.y[i]);
            let zz = h[6] * x + h[7] * y + h[8];
            if zz.is_nan() || zz <= 1e-9 {
                continue;
            }
            let u = (h[0] * x + h[1] * y + h[2]) / zz;
            let v = (h[3] * x + h[4] * y + h[5]) / zz;
            if let Some(val) = self.fr.img.sample_inside(u, v) {
                let t = self.tpl.t[i];
                let wt = self.tpl.w[i];
                stt += wt * t * t;
                st += wt * t;
                sw += wt;
                sit += wt * val * t;
                si += wt * val;
            }
        }
        let det = stt * sw - st * st;
        if det.is_nan() || det <= 1e-12 * (stt * sw).abs() || det <= 0.0 {
            return (gain, offset);
        }
        let a = (sit * sw - si * st) / det;
        let b = (stt * si - st * sit) / det;
        (a - 1.0, b)
    }

    fn damped_step(&self, lin: &Lin, lambda: f64) -> Option<Vec<f64>> {
        let m = self.system_size();
        let mut a = lin.a.clone();
        for i in 0..m {
            let d = lin.a[i * m + i];
            a[i * m + i] += match self.cfg.lm.damping {
                Damping::Levenberg => lambda,
                Damping::Marquardt => {
                    if d > 0.0 {
                        lambda * d
                    } else {
                        lambda
                    }
                }
            };
        }
        let rhs: Vec<f64> = lin.b.iter().map(|v| -v).collect();
        solve_spd(&a, &rhs, m)
    }

    /// Apply a step: the new pose, gain and offset. `None` when the new pose is singular.
    pub fn apply_step(
        &self,
        pose: &Pose,
        gain: f64,
        offset: f64,
        s: &[f64],
    ) -> Option<(Pose, f64, f64)> {
        let n = self.n;
        let dof: Dof = self.cfg.dof;
        let new = match self.cfg.rule {
            UpdateRule::ForwardAdditive => {
                // Additive in the eight numbers q, so that a pose which is not exactly in the
                // model (a model conjugated by an anisotropic level map) keeps its off-model part.
                let mut q = pose.to_q().ok()?;
                for (j, qj) in q.iter_mut().enumerate() {
                    for (k, sk) in s.iter().enumerate().take(n) {
                        *qj += self.dof_m[j * n + k] * sk;
                    }
                }
                Pose::from_q(&q)
            }
            UpdateRule::ForwardCompositional | UpdateRule::Esm => {
                pose.compose(&Pose::from_params(dof, &s[..n]))
            }
            UpdateRule::InverseCompositional => {
                pose.compose(&Pose::from_params(dof, &s[..n]).inverse().ok()?)
            }
        };
        let new = new.normalised().ok()?;
        if new.h.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let (g, o) = if self.gain_cols {
            (gain + s[n], offset + s[n + 1])
        } else {
            (gain, offset)
        };
        Some((new, g, o))
    }

    fn corner_motion(&self, a: &Pose, b: &Pose) -> f64 {
        a.max_point_distance(b, &self.tpl.corners)
            .unwrap_or(f64::INFINITY)
    }

    /// Run the Levenberg-Marquardt iteration of one level from a starting pose in level
    /// coordinates.
    pub fn solve(&self, start: &Pose, gain0: f64, offset0: f64, lp: &LevelParams) -> LevelOutcome {
        let lm = &self.cfg.lm;
        let alternate = self.cfg.illumination == Illumination::Alternate;
        let mut pose = *start;
        let (mut gain, mut offset) = (gain0, offset0);
        let mut lambda = lm.lambda_init;
        let mut iterations = 0;
        let mut rejections = 0;
        let mut dirty = true;
        let mut cur = self.linearize(&pose, gain, offset, true);
        let finish =
            |pose: Pose, gain: f64, offset: f64, it: usize, stop: StopReason, mean: f64| {
                LevelOutcome {
                    pose,
                    gain,
                    offset,
                    iterations: it,
                    stop,
                    mean_cost: mean,
                }
            };
        loop {
            if dirty {
                if alternate {
                    let (g, o) = self.regress(&pose, gain, offset);
                    gain = g;
                    offset = o;
                }
                cur = self.linearize(&pose, gain, offset, true);
                dirty = false;
            }
            if cur.valid < self.cfg.min_valid_pixels {
                return finish(
                    pose,
                    gain,
                    offset,
                    iterations,
                    StopReason::NoPixels,
                    cur.mean(),
                );
            }
            let mean_old = cur.mean();
            if mean_old < 1e-24 {
                return finish(
                    pose,
                    gain,
                    offset,
                    iterations,
                    StopReason::CostTol,
                    mean_old,
                );
            }
            if iterations >= lp.max_iterations {
                return finish(
                    pose,
                    gain,
                    offset,
                    iterations,
                    StopReason::MaxIterations,
                    mean_old,
                );
            }
            iterations += 1;
            let trial = self
                .damped_step(&cur, lambda)
                .and_then(|s| self.apply_step(&pose, gain, offset, &s).map(|t| (s, t)));
            let mut accepted = None;
            let mut trial_motion = f64::INFINITY;
            if let Some((s, (np, ng, no))) = trial {
                trial_motion = self.corner_motion(&pose, &np);
                let lin = self.linearize(&np, ng, no, false);
                if lin.valid >= self.cfg.min_valid_pixels {
                    let mean_new = lin.mean();
                    let ok = match lm.acceptance {
                        Acceptance::Decrease => mean_new < mean_old,
                        Acceptance::DecreaseOrEqual => mean_new <= mean_old,
                        Acceptance::GainRatio(min) => {
                            let m = self.system_size();
                            let mut quad = 0.0;
                            let mut lin_term = 0.0;
                            for p in 0..m {
                                lin_term += cur.b[p] * s[p];
                                for q in 0..m {
                                    quad += s[p] * cur.a[p * m + q] * s[q];
                                }
                            }
                            let pred = -(lin_term + 0.5 * quad);
                            let actual = 0.5 * (mean_old - mean_new) * cur.wsum;
                            pred > 0.0 && actual >= min * pred
                        }
                    };
                    if ok {
                        accepted = Some((np, ng, no, mean_new));
                    }
                }
            }
            match accepted {
                Some((np, ng, no, mean_new)) => {
                    let motion = self.corner_motion(&pose, &np);
                    pose = np;
                    gain = ng;
                    offset = no;
                    lambda = (lambda * lm.lambda_down).max(lm.lambda_min);
                    rejections = 0;
                    dirty = true;
                    if motion < lp.step_tol {
                        let mean = self.linearize(&pose, gain, offset, false).mean();
                        return finish(pose, gain, offset, iterations, StopReason::StepTol, mean);
                    }
                    let fall = (mean_old - mean_new) / mean_old;
                    if fall < lp.cost_tol {
                        return finish(
                            pose,
                            gain,
                            offset,
                            iterations,
                            StopReason::CostTol,
                            mean_new,
                        );
                    }
                }
                None => {
                    // A rejected step that would move the corners by less than the step
                    // tolerance means no further gain is possible at this precision.
                    if trial_motion < lp.step_tol {
                        return finish(
                            pose,
                            gain,
                            offset,
                            iterations,
                            StopReason::StepTol,
                            mean_old,
                        );
                    }
                    lambda = if lambda > 0.0 {
                        lambda * lm.lambda_up
                    } else {
                        lm.lambda_min.max(1e-12) * lm.lambda_up
                    };
                    rejections += 1;
                    if lambda > lp.lambda_max {
                        return finish(
                            pose,
                            gain,
                            offset,
                            iterations,
                            StopReason::LambdaMax,
                            mean_old,
                        );
                    }
                    if rejections >= lp.max_rejections {
                        return finish(
                            pose,
                            gain,
                            offset,
                            iterations,
                            StopReason::MaxRejections,
                            mean_old,
                        );
                    }
                }
            }
        }
    }
}

/// Residual statistics of a pose on one level (used at level 0 for the frame report).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    /// Root mean square residual over the selected pixels inside the frame (unweighted).
    pub rms: f64,
    /// Fraction of the selected pixels that are inside the frame and have `|r| <= threshold`.
    pub inlier_fraction: f64,
    /// Fraction of the selected pixels inside the frame.
    pub valid_fraction: f64,
}

/// Statistics of `pose` (level coordinates) on a template and frame level.
pub fn stats(
    tpl: &LevelTemplate,
    fr: &FrameLevel,
    pose: &Pose,
    gain: f64,
    offset: f64,
    threshold: f64,
) -> Stats {
    let h = &pose.h;
    let (mut sum, mut valid, mut inl) = (0.0, 0usize, 0usize);
    for i in 0..tpl.len() {
        let (x, y) = (tpl.x[i], tpl.y[i]);
        let zz = h[6] * x + h[7] * y + h[8];
        if zz.is_nan() || zz <= 1e-9 {
            continue;
        }
        let u = (h[0] * x + h[1] * y + h[2]) / zz;
        let v = (h[3] * x + h[4] * y + h[5]) / zz;
        if let Some(val) = fr.img.sample_inside(u, v) {
            let r = val - ((1.0 + gain) * tpl.t[i] + offset);
            sum += r * r;
            valid += 1;
            if r.abs() <= threshold {
                inl += 1;
            }
        }
    }
    let total = tpl.len() as f64;
    Stats {
        rms: if valid > 0 {
            (sum / valid as f64).sqrt()
        } else {
            f64::INFINITY
        },
        inlier_fraction: inl as f64 / total,
        valid_fraction: valid as f64 / total,
    }
}
