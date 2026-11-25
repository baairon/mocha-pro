//! Point and supersample coverage by scanline winding.
//!
//! A sample at `(px, py)` is inside when the winding number of the edges to its right is inside the
//! fill rule. An edge counts for a sample row `py` when `y0 <= py < y1` (a sample on the smaller y
//! end of an edge counts, on the larger end does not), and counts for a sample when the edge's x at
//! `py` is strictly greater than `px`. Together these are the top left rule in buffer space: a
//! sample exactly on a left or top edge is inside, exactly on a right or bottom edge is outside,
//! where top is the smaller row index.

use crate::config::{FillRule, Pattern};
use crate::edges::Edge;

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// The offsets of a pattern, in the order listed in the documentation of [`Pattern`].
pub fn pattern_offsets(p: &Pattern) -> Vec<(f64, f64)> {
    match p {
        Pattern::Grid(n) => {
            let nf = *n as f64;
            let mut v = Vec::with_capacity((*n as usize) * (*n as usize));
            for b in 0..*n {
                for a in 0..*n {
                    v.push(((a as f64 + 0.5) / nf, (b as f64 + 0.5) / nf));
                }
            }
            v
        }
        Pattern::Rotated(n) => {
            let n = *n;
            let nf = n as f64;
            let stride = if n == 1 {
                0
            } else {
                let mut s = (0.382 * nf).ceil() as u32;
                while gcd(s, n) != 1 {
                    s += 1;
                }
                s
            };
            (0..n)
                .map(|k| {
                    let r = ((k as u64 * stride as u64) % n as u64) as f64;
                    ((k as f64 + 0.5) / nf, (r + 0.5) / nf)
                })
                .collect()
        }
        Pattern::Offsets(o) => o.clone(),
    }
}

/// Adds, for pixel row `j`, the count of samples inside to `counts` (one per column).
pub(crate) fn sample_row(
    edges: &[Edge],
    ids: &[u32],
    j: usize,
    offsets: &[(f64, f64)],
    fill: FillRule,
    counts: &mut [u32],
) {
    if ids.is_empty() {
        return;
    }
    let mut cross: Vec<(f64, i32)> = Vec::new();
    let mut prefix: Vec<i32> = Vec::new();
    for &(ox, oy) in offsets {
        let py = j as f64 + oy;
        cross.clear();
        for &id in ids {
            let e = &edges[id as usize];
            if e.y0 <= py && py < e.y1 {
                cross.push((e.x_at(py), e.dir));
            }
        }
        if cross.is_empty() {
            continue;
        }
        cross.sort_by(|a, b| a.0.total_cmp(&b.0));
        prefix.clear();
        let mut run = 0;
        prefix.push(0);
        for c in &cross {
            run += c.1;
            prefix.push(run);
        }
        let total = run;
        for (i, cell) in counts.iter_mut().enumerate() {
            let px = i as f64 + ox;
            let k = cross.partition_point(|c| c.0 <= px);
            let wind = total - prefix[k];
            let inside = match fill {
                FillRule::NonZero => wind != 0,
                FillRule::EvenOdd => wind & 1 != 0,
            };
            if inside {
                *cell += 1;
            }
        }
    }
}
