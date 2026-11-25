//! Flattening of a contour to a polyline.
//!
//! Rules: uniform (N segments per span), recursive midpoint subdivision until a flatness bound
//! (J. Lane and R. Riesenfeld, "A Theoretical Development for the Computer Generation and
//! Display of Piecewise Polynomial Surfaces", IEEE PAMI 2(1), 1980, the flatness test on the
//! control polygon), and a segment count from the length of each span.
//!
//! The polyline holds, for every span in order, the points at the start of each of its
//! segments; an open contour ends with the end point of its last span; a closed contour ends
//! with a copy of its first point only when [`crate::Config::repeat_first`] is set.

use crate::bezier;
use crate::real::Real;
use crate::{
    eval_prepared, Config, Contour, ContourError, EvalForm, FlatTest, Kind, Precision, Prepared,
};

/// The deepest subdivision a rule may ask for.
pub const MAX_DEPTH: u32 = 24;
/// The most segments a length rule gives one span.
pub const MAX_SEGMENTS: u32 = 4096;

/// A flattening rule with its constant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Flatten {
    /// Exactly `n` segments per span (n at least 1), at parameters i / n.
    Uniform(u32),
    /// Midpoint subdivision of the parameter interval until the flatness test passes with
    /// distance at most `tol`, or `max_depth` (at most [`MAX_DEPTH`]) is reached.
    Subdivide { tol: f64, max_depth: u32 },
    /// Per span, the length is estimated as the polyline through `samples` equal parameter
    /// steps; the segment count is the length over `step` rounded up, between 1 and
    /// [`MAX_SEGMENTS`], and the segments are uniform in the parameter.
    ByLength { step: f64, samples: u32 },
}

fn check_rule(rule: &Flatten) -> Result<(), ContourError> {
    match *rule {
        Flatten::Uniform(0) => Err(ContourError::BadFlatten("uniform count is 0")),
        Flatten::Uniform(n) if n > MAX_SEGMENTS => {
            Err(ContourError::BadFlatten("uniform count too large"))
        }
        Flatten::Uniform(_) => Ok(()),
        Flatten::Subdivide { tol, max_depth } => {
            if !tol.is_finite() || tol < 0.0 {
                Err(ContourError::BadFlatten(
                    "tolerance is negative or not finite",
                ))
            } else if max_depth > MAX_DEPTH {
                Err(ContourError::BadFlatten("depth above the maximum"))
            } else {
                Ok(())
            }
        }
        Flatten::ByLength { step, samples } => {
            if !step.is_finite() || step <= 0.0 {
                Err(ContourError::BadFlatten("step is not positive and finite"))
            } else if samples == 0 || samples > MAX_SEGMENTS {
                Err(ContourError::BadFlatten("sample count out of range"))
            } else {
                Ok(())
            }
        }
    }
}

/// Flattens a contour under `rule` and `cfg`.
pub fn flatten(
    contour: &Contour,
    rule: &Flatten,
    cfg: &Config,
) -> Result<Vec<(f64, f64)>, ContourError> {
    check_rule(rule)?;
    let spans = contour.span_count(cfg)?;
    if let Flatten::Subdivide { .. } = rule {
        if cfg.flat_test == FlatTest::ControlPolygon && contour.kind != Kind::Bezier {
            return Err(ContourError::UnsupportedFlatTest);
        }
    }
    match cfg.precision {
        Precision::F64 => run::<f64>(contour, rule, cfg, spans),
        Precision::F32 => run::<f32>(contour, rule, cfg, spans),
    }
}

fn run<R: Real>(
    contour: &Contour,
    rule: &Flatten,
    cfg: &Config,
    spans: usize,
) -> Result<Vec<(f64, f64)>, ContourError> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for span in 0..spans {
        let prep = contour.prepare::<R>(span, cfg);
        match *rule {
            Flatten::Uniform(n) => uniform(&prep, cfg.form, n, &mut out)?,
            Flatten::ByLength { step, samples } => {
                let len = length(&prep, cfg.form, samples)?;
                let n = segments_for(len, step);
                uniform(&prep, cfg.form, n, &mut out)?;
            }
            Flatten::Subdivide { tol, max_depth } => {
                subdivide(&prep, cfg, tol, max_depth, &mut out)?;
            }
        }
    }
    if contour.closed && cfg.closing_span {
        if cfg.repeat_first {
            out.push(out[0]);
        }
    } else {
        let last = contour.prepare::<R>(spans - 1, cfg);
        out.push(eval_prepared(&last, cfg.form, 1.0)?);
    }
    Ok(out)
}

fn uniform<R: Real>(
    prep: &Prepared<R>,
    form: EvalForm,
    n: u32,
    out: &mut Vec<(f64, f64)>,
) -> Result<(), ContourError> {
    for i in 0..n {
        out.push(eval_prepared(prep, form, f64::from(i) / f64::from(n))?);
    }
    Ok(())
}

fn length<R: Real>(prep: &Prepared<R>, form: EvalForm, samples: u32) -> Result<f64, ContourError> {
    let mut prev = eval_prepared(prep, form, 0.0)?;
    let mut total = 0.0;
    for i in 1..=samples {
        let q = eval_prepared(prep, form, f64::from(i) / f64::from(samples))?;
        let dx = q.0 - prev.0;
        let dy = q.1 - prev.1;
        total += (dx * dx + dy * dy).sqrt();
        prev = q;
    }
    Ok(total)
}

/// Segments for a span of length `len`: len / step rounded up, between 1 and MAX_SEGMENTS.
pub(crate) fn segments_for(len: f64, step: f64) -> u32 {
    let ratio = len / step;
    if ratio.is_nan() || ratio <= 1.0 {
        return 1;
    }
    if ratio >= f64::from(MAX_SEGMENTS) {
        return MAX_SEGMENTS;
    }
    let mut n = ratio as u32;
    if f64::from(n) < ratio {
        n += 1;
    }
    n
}

fn subdivide<R: Real>(
    prep: &Prepared<R>,
    cfg: &Config,
    tol: f64,
    max_depth: u32,
    out: &mut Vec<(f64, f64)>,
) -> Result<(), ContourError> {
    if cfg.flat_test == FlatTest::ControlPolygon {
        if let Prepared::Bezier(q) = prep {
            return poly_rec(q, tol, max_depth, out);
        }
    }
    let a = eval_prepared(prep, cfg.form, 0.0)?;
    let b = eval_prepared(prep, cfg.form, 1.0)?;
    out.push(a);
    mid_rec(prep, cfg.form, (0.0, a), (1.0, b), tol, max_depth, out)
}

fn poly_rec<R: Real>(
    q: &[(R, R); 4],
    tol: f64,
    depth: u32,
    out: &mut Vec<(f64, f64)>,
) -> Result<(), ContourError> {
    let f = |p: (R, R)| (p.0.to_f64(), p.1.to_f64());
    let pts = [f(q[0]), f(q[1]), f(q[2]), f(q[3])];
    if depth == 0 || bezier::flat_enough(&pts, tol) {
        if !pts[0].0.is_finite() || !pts[0].1.is_finite() {
            return Err(ContourError::NonFiniteResult);
        }
        out.push(pts[0]);
        return Ok(());
    }
    let (l, r) = bezier::split(q, R::from_f64(0.5));
    poly_rec(&l, tol, depth - 1, out)?;
    poly_rec(&r, tol, depth - 1, out)
}

fn mid_rec<R: Real>(
    prep: &Prepared<R>,
    form: EvalForm,
    lo: (f64, (f64, f64)),
    hi: (f64, (f64, f64)),
    tol: f64,
    depth: u32,
    out: &mut Vec<(f64, f64)>,
) -> Result<(), ContourError> {
    if depth == 0 {
        return Ok(());
    }
    let span = hi.0 - lo.0;
    let tm = lo.0 + span / 2.0;
    let m = eval_prepared(prep, form, tm)?;
    let mut flat = bezier::within(lo.1, hi.1, m, tol);
    if flat {
        for k in [0.25, 0.75] {
            let q = eval_prepared(prep, form, lo.0 + span * k)?;
            if !bezier::within(lo.1, hi.1, q, tol) {
                flat = false;
                break;
            }
        }
    }
    if flat {
        return Ok(());
    }
    mid_rec(prep, form, lo, (tm, m), tol, depth - 1, out)?;
    out.push(m);
    mid_rec(prep, form, (tm, m), hi, tol, depth - 1, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_round_up() {
        assert_eq!(segments_for(12.0, 3.0), 4);
        assert_eq!(segments_for(12.0, 5.0), 3);
        assert_eq!(segments_for(12.0, 12.0), 1);
        assert_eq!(segments_for(12.5, 12.0), 2);
    }

    #[test]
    fn segments_have_floor_and_cap() {
        assert_eq!(segments_for(0.0, 1.0), 1);
        assert_eq!(segments_for(0.5, 1.0), 1);
        assert_eq!(segments_for(f64::NAN, 1.0), 1);
        assert_eq!(segments_for(1e12, 1.0), MAX_SEGMENTS);
    }
}
