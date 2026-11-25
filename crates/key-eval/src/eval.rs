//! Evaluation of a key list at a time.

use crate::config::{Clamp, Config, Empty, HandleMode, Outside, Side, Snap, VectorMode};
use crate::key::{Key, KeyList, Kind};
use crate::solve::solve;

/// Largest f64 magnitude from which every value is already a whole number.
const WHOLE: f64 = 4503599627370496.0;

/// Floor from the exactly rounded cast to an integer; whole numbers and magnitudes of 2^52 and more are
/// returned unchanged.
pub fn floor(x: f64) -> f64 {
    if x.abs() >= WHOLE {
        return x;
    }
    let i = x as i64 as f64;
    if i > x {
        i - 1.0
    } else {
        i
    }
}

fn is_even_whole(f: f64) -> bool {
    let h = f / 2.0;
    floor(h) == h
}

/// Nearest whole number, an exact half going up. Written with the exact fraction `x - floor(x)` so that
/// `0.49999999999999994` rounds down.
pub fn round_half_up(x: f64) -> f64 {
    let f = floor(x);
    if x - f >= 0.5 {
        f + 1.0
    } else {
        f
    }
}

/// Nearest whole number, an exact half going to the even neighbour.
pub fn round_half_even(x: f64) -> f64 {
    let f = floor(x);
    let d = x - f;
    if d > 0.5 || (d == 0.5 && !is_even_whole(f)) {
        f + 1.0
    } else {
        f
    }
}

/// The time after snapping. A `frames_per_unit` that is not finite and positive disables snapping.
pub fn snap_time(cfg: &Config, t: f64) -> f64 {
    let fps = cfg.frames_per_unit;
    if cfg.snap == Snap::None || !(fps.is_finite() && fps > 0.0) {
        return t;
    }
    let x = t * fps;
    let r = match cfg.snap {
        Snap::None => x,
        Snap::Floor => floor(x),
        Snap::RoundHalfUp => round_half_up(x),
        Snap::RoundHalfEven => round_half_even(x),
    };
    r / fps
}

/// Handle offsets of a Bezier segment for component `c`, the time taken from component `tc`:
/// `(dt1, dv1, dt2, dv2)` with out handle `(ta + dt1, va + dv1)` and in handle `(tb + dt2, vb + dv2)`,
/// after the clamping rule.
fn offsets<const N: usize>(
    cfg: &Config,
    a: &Key<N>,
    b: &Key<N>,
    c: usize,
    tc: usize,
) -> (f64, f64, f64, f64) {
    let l = b.time - a.time;
    let (mut dt1, dv1, mut dt2, dv2) = match cfg.handle_mode {
        HandleMode::Absolute => (
            a.out_handle[tc][0] - a.time,
            a.out_handle[c][1] - a.value[c],
            b.in_handle[tc][0] - b.time,
            b.in_handle[c][1] - b.value[c],
        ),
        HandleMode::Offset => (
            a.out_handle[tc][0],
            a.out_handle[c][1],
            b.in_handle[tc][0],
            b.in_handle[c][1],
        ),
    };
    let (mut dv1, mut dv2) = (dv1, dv2);
    match cfg.clamp {
        Clamp::None => {}
        Clamp::ClampTime => {
            dt1 = dt1.clamp(0.0, l);
            dt2 = dt2.clamp(-l, 0.0);
        }
        Clamp::ScaleBoth => {
            let e1 = if dt1 > 0.0 { dt1 } else { 0.0 };
            let e2 = if dt2 < 0.0 { -dt2 } else { 0.0 };
            let mut f = 1.0;
            if e1 > l {
                f = l / e1;
            }
            if e2 > l && l / e2 < f {
                f = l / e2;
            }
            dt1 *= f;
            dt2 *= f;
            dv1 *= f;
            dv2 *= f;
        }
    }
    (dt1, dv1, dt2, dv2)
}

fn tcomp(cfg: &Config) -> impl Fn(usize) -> usize {
    let joint = cfg.vector_mode == VectorMode::Joint;
    move |c| if joint { 0 } else { c }
}

fn bezier_value<const N: usize>(cfg: &Config, a: &Key<N>, b: &Key<N>, c: usize, t: f64) -> f64 {
    let tc = tcomp(cfg)(c);
    let l = b.time - a.time;
    let (dt1, dv1, dt2, dv2) = offsets(cfg, a, b, c, tc);
    let p1 = dt1 / l;
    let p2 = (l + dt2) / l;
    let s = (t - a.time) / l;
    let u = solve(cfg.solver, p1, p2, s);
    let (va, vb) = (a.value[c], b.value[c]);
    let (y1, y2) = (va + dv1, vb + dv2);
    // de Casteljau (Farin 2002, the chapter on Bezier curves).
    let lerp = |p: f64, q: f64| p + (q - p) * u;
    let (q0, q1, q2) = (lerp(va, y1), lerp(y1, y2), lerp(y2, vb));
    let (r0, r1) = (lerp(q0, q1), lerp(q1, q2));
    lerp(r0, r1)
}

fn hermite_value<const N: usize>(a: &Key<N>, b: &Key<N>, c: usize, t: f64) -> f64 {
    let l = b.time - a.time;
    let s = (t - a.time) / l;
    let s2 = s * s;
    let s3 = s2 * s;
    let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
    let h10 = s3 - 2.0 * s2 + s;
    let h01 = -2.0 * s3 + 3.0 * s2;
    let h11 = s3 - s2;
    h00 * a.value[c] + h10 * l * a.out_slope[c] + h01 * b.value[c] + h11 * l * b.in_slope[c]
}

/// Value on the open segment between `a` and `b` (`a.time < t < b.time`), by the kind of `a`.
fn segment_value<const N: usize>(cfg: &Config, a: &Key<N>, b: &Key<N>, t: f64) -> [f64; N] {
    std::array::from_fn(|c| match a.kind {
        Kind::Hold => a.value[c],
        Kind::Linear => {
            let s = (t - a.time) / (b.time - a.time);
            a.value[c] + (b.value[c] - a.value[c]) * s
        }
        Kind::Hermite => hermite_value(a, b, c, t),
        Kind::Bezier => bezier_value(cfg, a, b, c, t),
    })
}

impl<const N: usize> KeyList<N> {
    /// Value at time `t` under `cfg`. An empty list gives `cfg.empty`; a non finite time gives NaN in every
    /// component. The coincident rule of `cfg` is applied by [`KeyList::new`], not here.
    pub fn evaluate(&self, cfg: &Config, t: f64) -> [f64; N] {
        let keys = &self.keys;
        if keys.is_empty() {
            let v = match cfg.empty {
                Empty::Zero => 0.0,
                Empty::Fill(v) => v,
            };
            return [v; N];
        }
        if !t.is_finite() {
            return [f64::NAN; N];
        }
        let t = snap_time(cfg, t);
        let n = keys.len();
        let (t0, tn) = (keys[0].time, keys[n - 1].time);
        if n == 1 {
            return keys[0].value;
        }
        if t < t0 {
            self.outside(cfg, cfg.before, t)
        } else if t > tn {
            self.outside(cfg, cfg.after, t)
        } else {
            self.inside(cfg, t)
        }
    }

    fn inside(&self, cfg: &Config, t: f64) -> [f64; N] {
        let keys = &self.keys;
        let n = keys.len();
        if keys[0].time == keys[n - 1].time {
            return match cfg.jump_side {
                Side::Left => keys[0].value,
                Side::Right => keys[n - 1].value,
            };
        }
        let first_ge = keys.partition_point(|k| k.time < t);
        let past_le = keys.partition_point(|k| k.time <= t);
        if first_ge < past_le {
            if past_le - first_ge > 1 {
                return match cfg.jump_side {
                    Side::Left => keys[first_ge].value,
                    Side::Right => keys[past_le - 1].value,
                };
            }
            if cfg.hold_side == Side::Left && first_ge > 0 && keys[first_ge - 1].kind == Kind::Hold
            {
                return keys[first_ge - 1].value;
            }
            return keys[first_ge].value;
        }
        segment_value(cfg, &keys[first_ge - 1], &keys[first_ge], t)
    }

    /// The slope per component at the first (`start`) or last key, from the end segment.
    fn edge_slope(&self, cfg: &Config, start: bool) -> [f64; N] {
        let keys = &self.keys;
        let n = keys.len();
        let j = if start {
            keys.partition_point(|k| k.time <= keys[0].time)
        } else {
            keys.partition_point(|k| k.time < keys[n - 1].time)
        };
        let (a, b) = (&keys[j - 1], &keys[j]);
        let l = b.time - a.time;
        std::array::from_fn(|c| {
            let chord = (b.value[c] - a.value[c]) / l;
            match a.kind {
                Kind::Hold => 0.0,
                Kind::Linear => chord,
                Kind::Hermite => {
                    if start {
                        a.out_slope[c]
                    } else {
                        b.in_slope[c]
                    }
                }
                Kind::Bezier => {
                    let tc = tcomp(cfg)(c);
                    let (dt1, dv1, dt2, dv2) = offsets(cfg, a, b, c, tc);
                    let (dt, dv) = if start { (dt1, dv1) } else { (dt2, dv2) };
                    if dt != 0.0 {
                        dv / dt
                    } else {
                        chord
                    }
                }
            }
        })
    }

    fn outside(&self, cfg: &Config, rule: Outside, t: f64) -> [f64; N] {
        let keys = &self.keys;
        let n = keys.len();
        let (first, last) = (&keys[0], &keys[n - 1]);
        let before = t < first.time;
        let (t0, tn) = (first.time, last.time);
        let edge = if before { first } else { last };
        let period = tn - t0;
        match rule {
            Outside::Linear if period > 0.0 => {
                let slope = self.edge_slope(cfg, before);
                let d = t - edge.time;
                std::array::from_fn(|c| edge.value[c] + slope[c] * d)
            }
            Outside::Hold | Outside::Linear => edge.value,
            Outside::Cycle | Outside::CycleOffset | Outside::Mirror => {
                if period <= 0.0 {
                    return edge.value;
                }
                let r0 = t - t0;
                let m = floor(r0 / period);
                let mut r = r0 - m * period;
                r = r.clamp(0.0, period);
                if rule == Outside::Mirror && !is_even_whole(m) {
                    r = period - r;
                }
                let tt = (t0 + r).clamp(t0, tn);
                let mut v = self.inside(cfg, tt);
                if rule == Outside::CycleOffset {
                    for (c, x) in v.iter_mut().enumerate() {
                        *x += m * (last.value[c] - first.value[c]);
                    }
                }
                v
            }
        }
    }
}
