//! Turning polygons into edges in pixel space, and sorting the edges into pixel rows.
//!
//! Pixel space is the space in which pixel `(i, j)` of the target is the unit square
//! `[i, i + 1] x [j, j + 1]` and row `j` grows downward in the buffer. Every operation here is a
//! plain double operation, in the order written, so the result is the same on every platform.

use crate::config::{Config, Polygon, Target};
use crate::error::Error;
use crate::{MAX_COORD, MAX_PIXELS, MAX_SIDE};

/// A non-horizontal edge with `y0 < y1`; `dir` is +1 when the ring ran from `y0` to `y1`, else -1.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Edge {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub dx: f64,
    pub dy: f64,
    pub dir: i32,
}

impl Edge {
    fn new(ax: f64, ay: f64, bx: f64, by: f64) -> Option<Edge> {
        if ay == by {
            return None;
        }
        let (x0, y0, x1, y1, dir) = if ay < by {
            (ax, ay, bx, by, 1)
        } else {
            (bx, by, ax, ay, -1)
        };
        Some(Edge {
            x0,
            y0,
            x1,
            y1,
            dx: x1 - x0,
            dy: y1 - y0,
            dir,
        })
    }

    /// The x of the edge at height `y`: the end values outside the span, else
    /// `x0 + (dx * (y - y0)) / dy`.
    pub fn x_at(&self, y: f64) -> f64 {
        if y <= self.y0 {
            self.x0
        } else if y >= self.y1 {
            self.x1
        } else {
            self.x0 + (self.dx * (y - self.y0)) / self.dy
        }
    }
}

pub(crate) fn check_target(t: &Target) -> Result<(), Error> {
    if t.width > MAX_SIDE
        || t.height > MAX_SIDE
        || (t.width as u64) * (t.height as u64) > MAX_PIXELS
        || t.x.unsigned_abs() > MAX_COORD as u64
        || t.y.unsigned_abs() > MAX_COORD as u64
    {
        return Err(Error::BadTarget);
    }
    Ok(())
}

fn snap(v: f64, bits: Option<u32>) -> f64 {
    match bits {
        None => v,
        Some(b) => {
            let s = (1u64 << b) as f64;
            (v * s).round() / s
        }
    }
}

/// The edges of one polygon in pixel space. Coordinates are checked, snapped, moved by the target
/// origin and the centre, and flipped when `y_down` is false:
/// `x' = (x - tx) + (0.5 - centre)`; with `y_down`, `y' = (y - ty) + (0.5 - centre)`, else
/// `y' = (h - 0.5 + centre) - (y - ty)`.
pub(crate) fn polygon_edges(
    index: usize,
    poly: &Polygon,
    target: &Target,
    cfg: &Config,
) -> Result<Vec<Edge>, Error> {
    let shift = 0.5 - cfg.centre;
    let flip = target.height as f64 - 0.5 + cfg.centre;
    let (tx, ty) = (target.x as f64, target.y as f64);
    let mut edges = Vec::new();
    for (ri, ring) in poly.0.iter().enumerate() {
        let mut pts: Vec<(f64, f64)> = Vec::with_capacity(ring.len());
        for (pi, &(x, y)) in ring.iter().enumerate() {
            if !x.is_finite() || !y.is_finite() {
                return Err(Error::NonFinite {
                    polygon: index,
                    ring: ri,
                    point: pi,
                });
            }
            if x.abs() > MAX_COORD || y.abs() > MAX_COORD {
                return Err(Error::OutOfRange {
                    polygon: index,
                    ring: ri,
                    point: pi,
                });
            }
            let x = snap(x, cfg.snap_bits);
            let y = snap(y, cfg.snap_bits);
            let px = (x - tx) + shift;
            let py = if cfg.y_down {
                (y - ty) + shift
            } else {
                flip - (y - ty)
            };
            pts.push((px, py));
        }
        let n = pts.len();
        for k in 0..n {
            let a = pts[k];
            let b = pts[(k + 1) % n];
            if let Some(e) = Edge::new(a.0, a.1, b.0, b.1) {
                edges.push(e);
            }
        }
    }
    Ok(edges)
}

/// For each pixel row, the indexes of the edges whose closed y span meets the row's band. A sample
/// offset of 1.0 sits on the next row's top line, so an edge that starts exactly on an integer line
/// is also listed in the row above it.
pub(crate) fn bucket(edges: &[Edge], height: usize) -> Vec<Vec<u32>> {
    let mut rows: Vec<Vec<u32>> = vec![Vec::new(); height];
    for (k, e) in edges.iter().enumerate() {
        if e.y1 < 0.0 || e.y0 > height as f64 {
            continue;
        }
        let mut lo = e.y0.max(0.0).floor() as usize;
        if lo > 0 && lo as f64 == e.y0 {
            lo -= 1;
        }
        let hi = (e.y1.min(height as f64 - 1.0)).floor() as usize;
        for row in rows.iter_mut().take(hi + 1).skip(lo) {
            row.push(k as u32);
        }
    }
    rows
}
