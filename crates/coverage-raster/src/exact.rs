//! Exact area coverage by slabs.
//!
//! For one pixel row the edges that meet the row band are clipped to it. The band is cut at every
//! height where an edge starts or ends inside it and where two edges cross, so that inside one slab
//! no two edges cross and their left to right order is fixed. In a slab the edges are sorted by
//! their x at the middle height and swept with the winding number; each stretch the fill rule calls
//! inside is a trapezoid between two straight lines. The area of a trapezoid inside the unit column
//! `[i, i + 1]` is the integral over the slab of `clamp(x_right - i, 0, 1) - clamp(x_left - i, 0, 1)`,
//! which is closed form because each x is linear in the height. The row's coverage is the sum over
//! slabs and trapezoids, clamped to 0 to 1 to absorb rounding.
//!
//! Operation order, for platform independence: only `+`, `-`, `*`, `/`, comparisons, `floor`, `min`,
//! `max` on doubles, each written as a separate statement without fused multiply add. The sum order
//! is slab by slab from the top, trapezoid by trapezoid from the left, column by column.
//!
//! Error: for a polygon whose vertices have absolute value at most `M` in pixel space, every x
//! and height used has an absolute rounding error of a few units of `M * 2^-53`, and each
//! coverage sums a bounded number of such terms, so the error of a pixel is of the order of
//! `M * 2^-53` times the number of slabs of its row. This is a statement of order, not a proof; the
//! tests measure it against closed forms (the bound is an estimate of the order, and the tests check it on examples). When all vertices
//! are multiples of `2^-k` with small `k` and no two edges cross inside a pixel row, every value is
//! exact.

use crate::config::FillRule;
use crate::edges::Edge;

/// The mean over the slab of `clamp(x - i, 0, 1)` when `x` runs linearly from `a` to `b`.
pub(crate) fn clamp_mean(a: f64, b: f64, i: f64) -> f64 {
    let ua = a - i;
    let ub = b - i;
    if ua <= 0.0 && ub <= 0.0 {
        return 0.0;
    }
    if ua >= 1.0 && ub >= 1.0 {
        return 1.0;
    }
    if (0.0..=1.0).contains(&ua) && (0.0..=1.0).contains(&ub) {
        return (ua + ub) * 0.5;
    }
    let d = ub - ua;
    let mut s = [0.0f64; 4];
    let mut n = 1;
    let mut cuts = [(0.0 - ua) / d, (1.0 - ua) / d];
    if cuts[0] > cuts[1] {
        cuts.swap(0, 1);
    }
    for c in cuts {
        if c > 0.0 && c < 1.0 {
            s[n] = c;
            n += 1;
        }
    }
    s[n] = 1.0;
    n += 1;
    let mut total = 0.0;
    for k in 0..n - 1 {
        let (sa, sb) = (s[k], s[k + 1]);
        let len = sb - sa;
        let ma = ua + d * sa;
        let mb = ua + d * sb;
        let mid = ua + d * ((sa + sb) * 0.5);
        if mid <= 0.0 {
            continue;
        }
        if mid >= 1.0 {
            total += len;
        } else {
            total += len * ((ma.clamp(0.0, 1.0) + mb.clamp(0.0, 1.0)) * 0.5);
        }
    }
    total
}

fn add_span(out: &mut [f64], dt: f64, l: (f64, f64), r: (f64, f64)) {
    let w = out.len() as f64;
    let minx = l.0.min(l.1);
    let maxx = r.0.max(r.1);
    if maxx <= 0.0 || minx >= w {
        return;
    }
    let i0 = minx.max(0.0).floor() as usize;
    let i1 = (maxx.min(w).ceil() as usize).min(out.len());
    for (i, cell) in out.iter_mut().enumerate().take(i1).skip(i0) {
        let c = i as f64;
        let hr = clamp_mean(r.0, r.1, c);
        let hl = clamp_mean(l.0, l.1, c);
        *cell += dt * (hr - hl);
    }
}

fn inside(fill: FillRule, wind: i32) -> bool {
    match fill {
        FillRule::NonZero => wind != 0,
        FillRule::EvenOdd => wind & 1 != 0,
    }
}

/// Coverage of pixel row `j` (width `out.len()`) from the edges `ids` of `edges`. `out` is added to.
pub(crate) fn exact_row(edges: &[Edge], ids: &[u32], j: usize, fill: FillRule, out: &mut [f64]) {
    let lo = j as f64;
    let hi = lo + 1.0;
    let mut clips: Vec<(usize, f64, f64)> = Vec::with_capacity(ids.len());
    for &id in ids {
        let e = &edges[id as usize];
        let a = e.y0.max(lo);
        let b = e.y1.min(hi);
        if a < b {
            clips.push((id as usize, a, b));
        }
    }
    if clips.is_empty() {
        return;
    }
    let mut ys = vec![lo, hi];
    for &(_, a, b) in &clips {
        ys.push(a);
        ys.push(b);
    }
    for p in 0..clips.len() {
        for q in p + 1..clips.len() {
            let (ep, ap, bp) = clips[p];
            let (eq, aq, bq) = clips[q];
            let s = ap.max(aq);
            let t = bp.min(bq);
            if s < t {
                let d0 = edges[ep].x_at(s) - edges[eq].x_at(s);
                let d1 = edges[ep].x_at(t) - edges[eq].x_at(t);
                if (d0 < 0.0 && d1 > 0.0) || (d0 > 0.0 && d1 < 0.0) {
                    let y = s + (t - s) * (d0 / (d0 - d1));
                    if y > s && y < t {
                        ys.push(y);
                    }
                }
            }
        }
    }
    ys.sort_by(|a, b| a.total_cmp(b));
    ys.dedup();
    let mut active: Vec<(f64, f64, f64, i32)> = Vec::new();
    for pair in ys.windows(2) {
        let (t0, t1) = (pair[0], pair[1]);
        if t1 <= t0 {
            continue;
        }
        let mid = t0 + (t1 - t0) * 0.5;
        active.clear();
        for &(e, a, b) in &clips {
            if a <= t0 && b >= t1 {
                let ed = &edges[e];
                active.push((ed.x_at(mid), ed.x_at(t0), ed.x_at(t1), ed.dir));
            }
        }
        active.sort_by(|p, q| {
            p.0.total_cmp(&q.0)
                .then((p.1 + p.2).total_cmp(&(q.1 + q.2)))
        });
        let dt = t1 - t0;
        let mut wind = 0;
        let mut start: Option<(f64, f64)> = None;
        for &(_, x0, x1, dir) in &active {
            let before = inside(fill, wind);
            wind += dir;
            let after = inside(fill, wind);
            if !before && after {
                start = Some((x0, x1));
            } else if before && !after {
                if let Some(l) = start.take() {
                    add_span(out, dt, l, (x0, x1));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_mean_constant_cases() {
        assert_eq!(clamp_mean(-3.0, -1.0, 0.0), 0.0);
        assert_eq!(clamp_mean(3.0, 5.0, 0.0), 1.0);
        assert_eq!(clamp_mean(0.25, 0.75, 0.0), 0.5);
        assert_eq!(clamp_mean(1.25, 1.75, 1.0), 0.5);
    }

    #[test]
    fn clamp_mean_ramp_through_the_column() {
        // x from -1 to 2: below 0 for a third, a ramp for a third, above 1 for a third:
        // 0 + 0.5/3 + 1/3 = 0.5.
        assert_eq!(clamp_mean(-1.0, 2.0, 0.0), 0.5);
        assert_eq!(clamp_mean(2.0, -1.0, 0.0), 0.5);
    }

    #[test]
    fn clamp_mean_enters_the_column_from_one_side() {
        // x from 0.5 to 2.5: ramp 0.5 to 1 over a quarter (mean 0.75 * 0.25), then 1 for the rest
        // (0.75): 0.1875 + 0.75 = 0.9375.
        assert_eq!(clamp_mean(0.5, 2.5, 0.0), 0.9375);
    }

    #[test]
    fn clamp_mean_touching_cases() {
        assert_eq!(clamp_mean(0.0, 0.0, 0.0), 0.0);
        assert_eq!(clamp_mean(1.0, 1.0, 0.0), 1.0);
        assert_eq!(clamp_mean(-1.0, 0.0, 0.0), 0.0);
    }
}
