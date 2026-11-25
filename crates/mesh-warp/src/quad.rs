//! Bilinear quad patch with its exact inverse.
//!
//! Source: G. Wolberg, "Digital Image Warping", 1990, chapter 3 (bilinear transformation). The
//! inverse solves the quadratic in `u` that results from eliminating `v` with planar cross
//! products (the usual closed form for the inverse bilinear map).

use crate::{cross, Vec2};

/// Which root of the quadratic is taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootChoice {
    /// `(-B - sqrt(D)) / (2A)`.
    Minus,
    /// `(-B + sqrt(D)) / (2A)`.
    Plus,
    /// The first of the minus and plus roots whose `(u, v)` lies in the unit square; when none
    /// does and outside points are accepted, the candidate nearest the unit square.
    FirstInUnit,
}

/// Choices of the inverse. The default is `FirstInUnit`, outside points rejected, exact test
/// `A == 0` for the linear case.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadConfig {
    pub root: RootChoice,
    /// Accept a `(u, v)` outside the unit square.
    pub accept_outside: bool,
    /// The quadratic is treated as linear when `|A| <= linear_eps`.
    pub linear_eps: f64,
}

impl Default for QuadConfig {
    fn default() -> Self {
        QuadConfig {
            root: RootChoice::FirstInUnit,
            accept_outside: false,
            linear_eps: 0.0,
        }
    }
}

/// Every combination of root choice and acceptance, with the default `linear_eps`.
pub fn quad_config_space() -> Vec<QuadConfig> {
    let mut v = Vec::new();
    for root in [RootChoice::Minus, RootChoice::Plus, RootChoice::FirstInUnit] {
        for accept_outside in [false, true] {
            v.push(QuadConfig {
                root,
                accept_outside,
                linear_eps: 0.0,
            });
        }
    }
    v
}

/// A bilinear patch: the unit square corners (0,0), (1,0), (1,1), (0,1) map to `corners[0..4]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadPatch {
    pub corners: [Vec2; 4],
}

impl QuadPatch {
    /// `P(u, v)` with weights `w00 = (1-u)(1-v)`, `w10 = u(1-v)`, `w11 = u v`, `w01 = (1-u) v`,
    /// summed left to right in the corner order 00, 10, 11, 01.
    pub fn forward(&self, u: f64, v: f64) -> Vec2 {
        let w00 = (1.0 - u) * (1.0 - v);
        let w10 = u * (1.0 - v);
        let w11 = u * v;
        let w01 = (1.0 - u) * v;
        let c = &self.corners;
        Vec2 {
            x: ((w00 * c[0].x + w10 * c[1].x) + w11 * c[2].x) + w01 * c[3].x,
            y: ((w00 * c[0].y + w10 * c[1].y) + w11 * c[2].y) + w01 * c[3].y,
        }
    }

    /// The inverse: `(u, v)` with `P(u, v) = p`, or `None`.
    ///
    /// With `a = P00`, `e = P10 - P00`, `f = P01 - P00`, `g = P00 - P10 + P11 - P01`, `h = p - a`
    /// and `x` the planar cross product: `A = e x g`, `B = e x f - h x g`, `C = -(h x f)`, and
    /// `A u^2 + B u + C = 0`. For `|A| <= linear_eps`, `u = -C / B`. Otherwise `D = B*B - 4*A*C`
    /// (no root when `D < 0`), and the roots are `(-B -+ sqrt(D)) / (2 A)`, evaluated as `q / A` and `C / q` with
    /// `q = -(B + sign(B) sqrt(D)) / 2` (no cancellation when `A` is tiny). Then, with
    /// `d = f + u g`, `v = (h.c - u e.c) / d.c` using the component `c` of larger `|d.c|`
    /// (x on a tie).
    pub fn inverse(&self, p: Vec2, cfg: &QuadConfig) -> Option<Vec2> {
        let c = &self.corners;
        let a = c[0];
        let e = Vec2::new(c[1].x - a.x, c[1].y - a.y);
        let f = Vec2::new(c[3].x - a.x, c[3].y - a.y);
        let g = Vec2::new(
            ((a.x - c[1].x) + c[2].x) - c[3].x,
            ((a.y - c[1].y) + c[2].y) - c[3].y,
        );
        let h = Vec2::new(p.x - a.x, p.y - a.y);
        let qa = cross(e, g);
        let qb = cross(e, f) - cross(h, g);
        let qc = -cross(h, f);
        let us: Vec<f64> = if qa.abs() <= cfg.linear_eps {
            if qb == 0.0 {
                return None;
            }
            let u = -qc / qb;
            vec![u, u]
        } else {
            let disc = qb * qb - 4.0 * qa * qc;
            if disc < 0.0 {
                return None;
            }
            let sq = disc.sqrt();
            // Cancellation free pairing: q = -(B + sign(B) sqrt(D)) / 2 gives the roots q / A and
            // C / q. The root of `-B - sqrt(D)` is q / A when B >= 0 and C / q otherwise.
            let q = -0.5 * (qb + sq.copysign(qb));
            if q == 0.0 {
                vec![0.0, 0.0]
            } else if qb >= 0.0 {
                vec![q / qa, qc / q]
            } else {
                vec![qc / q, q / qa]
            }
        };
        let mut cands: [Option<(f64, f64)>; 2] = [None, None];
        for (slot, &u) in cands.iter_mut().zip(&us) {
            let d = Vec2::new(f.x + u * g.x, f.y + u * g.y);
            let v = if d.x.abs() >= d.y.abs() {
                if d.x == 0.0 {
                    continue;
                }
                (h.x - u * e.x) / d.x
            } else {
                (h.y - u * e.y) / d.y
            };
            *slot = Some((u, v));
        }
        let in_unit = |&(u, v): &(f64, f64)| (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v);
        let pick = match cfg.root {
            RootChoice::Minus => cands[0],
            RootChoice::Plus => cands[1],
            RootChoice::FirstInUnit => {
                let hit = cands.iter().flatten().find(|q| in_unit(q)).copied();
                if hit.is_some() || !cfg.accept_outside {
                    hit
                } else {
                    let excess = |&(u, v): &(f64, f64)| {
                        let du = (0.0 - u).max(u - 1.0).max(0.0);
                        let dv = (0.0 - v).max(v - 1.0).max(0.0);
                        du * du + dv * dv
                    };
                    let mut best: Option<(f64, f64)> = None;
                    for q in cands.iter().flatten() {
                        if best.is_none_or(|b| excess(q) < excess(&b)) {
                            best = Some(*q);
                        }
                    }
                    best
                }
            }
        };
        let q = pick?;
        if !cfg.accept_outside && !in_unit(&q) {
            return None;
        }
        Some(Vec2::new(q.0, q.1))
    }

    /// The inverse as a closure from an output position to `(u, v)`.
    pub fn inverse_closure<'a>(
        &'a self,
        cfg: &'a QuadConfig,
    ) -> impl Fn(f64, f64) -> Option<(f64, f64)> + 'a {
        move |x, y| self.inverse(Vec2::new(x, y), cfg).map(|q| (q.x, q.y))
    }
}
