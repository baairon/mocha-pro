//! Independent shape data model.
//!
//! Every rule below is a labelled hypothesis, not a measured fact:
//! * the value at a key equals the stored value (INFERRED);
//! * between two keys each numeric field is interpolated linearly in time (INFERRED);
//! * the blend is computed in the form written in `lerp`, with the weight measured from the earlier
//!   key (INFERRED);
//! * boolean fields and absent optional fields hold the earlier key's value between keys (INFERRED);
//! * before the first key and after the last key the nearest key is held (INFERRED);
//! * the bounding rectangle spans the anchor positions of all points (INFERRED);
//! * of equal extreme coordinates the rectangle keeps the first point's value, which fixes the sign
//!   of a zero extreme (INFERRED).

use std::fmt;

/// One control point at one key. Optional edge fields may be absent.
#[derive(Clone, Debug, PartialEq)]
pub struct PointData {
    pub cusp: bool,
    pub enabled: bool,
    pub x: f64,
    pub y: f64,
    pub edge_w: Option<f64>,
    pub edge_r: Option<f64>,
    pub tan_in: (f64, f64),
    pub tan_out: (f64, f64),
}

/// All points of a contour at one key time.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe {
    pub time: f64,
    pub points: Vec<PointData>,
}

/// Left, top, width, height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ModelError {
    NoKeys,
    NonFiniteTime,
    KeysNotIncreasing,
    PointCountMismatch,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ModelError::NoKeys => "contour has no keys",
            ModelError::NonFiniteTime => "key time is not finite",
            ModelError::KeysNotIncreasing => "key times are not strictly increasing",
            ModelError::PointCountMismatch => "keys differ in point count",
        };
        f.write_str(s)
    }
}

impl std::error::Error for ModelError {}

/// A contour stored as keyframes.
#[derive(Clone, Debug, PartialEq)]
pub struct Contour {
    keys: Vec<Keyframe>,
}

/// Linear blend at weight `u`, in the form written below (INFERRED). It returns `a` exactly at
/// `u` = 0; at `u` = 1 it returns `b` only when `b - a` is exact, so a weight that rounds to 1
/// between two keys need not reproduce the later key's value.
pub fn lerp(a: f64, b: f64, u: f64) -> f64 {
    a + (b - a) * u
}

fn lerp_opt(a: Option<f64>, b: Option<f64>, u: f64) -> Option<f64> {
    match (a, b) {
        (Some(p), Some(q)) => Some(lerp(p, q, u)),
        _ => a,
    }
}

fn blend(a: &PointData, b: &PointData, u: f64) -> PointData {
    PointData {
        cusp: a.cusp,
        enabled: a.enabled,
        x: lerp(a.x, b.x, u),
        y: lerp(a.y, b.y, u),
        edge_w: lerp_opt(a.edge_w, b.edge_w, u),
        edge_r: lerp_opt(a.edge_r, b.edge_r, u),
        tan_in: (
            lerp(a.tan_in.0, b.tan_in.0, u),
            lerp(a.tan_in.1, b.tan_in.1, u),
        ),
        tan_out: (
            lerp(a.tan_out.0, b.tan_out.0, u),
            lerp(a.tan_out.1, b.tan_out.1, u),
        ),
    }
}

impl Contour {
    pub fn new(keys: Vec<Keyframe>) -> Result<Contour, ModelError> {
        let first = keys.first().ok_or(ModelError::NoKeys)?;
        let n = first.points.len();
        for k in &keys {
            if !k.time.is_finite() {
                return Err(ModelError::NonFiniteTime);
            }
            if k.points.len() != n {
                return Err(ModelError::PointCountMismatch);
            }
        }
        if keys.windows(2).any(|w| w[0].time >= w[1].time) {
            return Err(ModelError::KeysNotIncreasing);
        }
        Ok(Contour { keys })
    }

    pub fn keys(&self) -> &[Keyframe] {
        &self.keys
    }

    pub fn point_count(&self) -> usize {
        self.keys[0].points.len()
    }

    /// Points at time `t` under the hypotheses of the module docs. NaN `t` holds the first key.
    pub fn points_at(&self, t: f64) -> Vec<PointData> {
        let first = &self.keys[0];
        let last = &self.keys[self.keys.len() - 1];
        if t.is_nan() || t <= first.time {
            return first.points.clone();
        }
        if t >= last.time {
            return last.points.clone();
        }
        let i = self.keys.iter().rposition(|k| k.time <= t).unwrap_or(0);
        let (a, b) = (&self.keys[i], &self.keys[i + 1]);
        if t == a.time {
            return a.points.clone();
        }
        let u = (t - a.time) / (b.time - a.time);
        a.points
            .iter()
            .zip(&b.points)
            .map(|(p, q)| blend(p, q, u))
            .collect()
    }

    /// Bounding rectangle of the anchor positions at time `t`.
    pub fn bounds_at(&self, t: f64) -> Rect {
        let pts = self.points_at(t);
        let mut it = pts.iter();
        let Some(p0) = it.next() else {
            return Rect {
                left: 0.0,
                top: 0.0,
                width: 0.0,
                height: 0.0,
            };
        };
        let (mut x0, mut x1, mut y0, mut y1) = (p0.x, p0.x, p0.y, p0.y);
        for p in it {
            // strict comparisons keep the first of equal values, so -0.0 vs 0.0 is stable
            if p.x < x0 {
                x0 = p.x;
            }
            if p.x > x1 {
                x1 = p.x;
            }
            if p.y < y0 {
                y0 = p.y;
            }
            if p.y > y1 {
                y1 = p.y;
            }
        }
        Rect {
            left: x0,
            top: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }
}
