#![allow(dead_code)]
//! Shared helpers: shapes, configurations, and an independent slow reference.

use coverage_raster::*;

pub type Pt = (f64, f64);

pub fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
    Polygon::from_ring(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

pub fn rect_cw(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
    Polygon::from_ring(vec![(x0, y0), (x0, y1), (x1, y1), (x1, y0)])
}

pub fn tri(a: Pt, b: Pt, c: Pt) -> Polygon {
    Polygon::from_ring(vec![a, b, c])
}

pub fn cfg(method: Method) -> Config {
    let mut c = Config::canonical();
    c.method = method;
    c.quantise = None;
    c
}

pub fn exact() -> Config {
    cfg(Method::ExactArea)
}

pub fn run(polys: &[Polygon], w: u32, h: u32, c: &Config) -> Coverage {
    rasterise(polys, &Target::new(w, h), c).expect("rasterise")
}

pub fn one(p: Polygon, w: u32, h: u32, c: &Config) -> Coverage {
    run(&[p], w, h, c)
}

/// The values as rows, for readable comparisons.
pub fn rows(c: &Coverage) -> Vec<Vec<f64>> {
    c.data
        .chunks(c.width as usize)
        .map(|r| r.to_vec())
        .collect()
}

pub fn assert_rows(c: &Coverage, want: &[&[f64]]) {
    let got = rows(c);
    assert_eq!(got.len(), want.len(), "row count");
    for (j, (g, w)) in got.iter().zip(want.iter()).enumerate() {
        assert_eq!(g.as_slice(), *w, "row {j}");
    }
}

pub fn assert_close(c: &Coverage, want: &[&[f64]], tol: f64) {
    let got = rows(c);
    assert_eq!(got.len(), want.len(), "row count");
    for (j, (g, w)) in got.iter().zip(want.iter()).enumerate() {
        assert_eq!(g.len(), w.len(), "row width");
        for (i, (a, b)) in g.iter().zip(w.iter()).enumerate() {
            assert!((a - b).abs() <= tol, "pixel ({i},{j}): got {a}, want {b}");
        }
    }
}

/// Twice the signed area of all rings (shoelace), halved.
pub fn shoelace(p: &Polygon) -> f64 {
    let mut s = 0.0;
    for ring in &p.0 {
        let n = ring.len();
        for k in 0..n {
            let a = ring[k];
            let b = ring[(k + 1) % n];
            s += a.0 * b.1 - b.0 * a.1;
        }
    }
    s * 0.5
}

/// Independent reference: supersample each pixel on an `n` by `n` grid, deciding inside with the
/// classic winding number (`is_left` test) over all rings, with the same convention as the crate's
/// default (centre 0.5, rows grow with y). Returns coverage per pixel.
pub fn brute(p: &Polygon, w: u32, h: u32, n: u32, even_odd: bool) -> Vec<f64> {
    let mut out = vec![0.0; (w * h) as usize];
    for j in 0..h {
        for i in 0..w {
            let mut hit = 0u32;
            for b in 0..n {
                for a in 0..n {
                    let sx = i as f64 + (a as f64 + 0.5) / n as f64;
                    let sy = j as f64 + (b as f64 + 0.5) / n as f64;
                    let mut wn = 0i32;
                    for ring in &p.0 {
                        let m = ring.len();
                        for k in 0..m {
                            let (x0, y0) = ring[k];
                            let (x1, y1) = ring[(k + 1) % m];
                            let left = (x1 - x0) * (sy - y0) - (sx - x0) * (y1 - y0);
                            if y0 <= sy {
                                if y1 > sy && left > 0.0 {
                                    wn += 1;
                                }
                            } else if y1 <= sy && left < 0.0 {
                                wn -= 1;
                            }
                        }
                    }
                    let inside = if even_odd { wn & 1 != 0 } else { wn != 0 };
                    if inside {
                        hit += 1;
                    }
                }
            }
            out[(j * w + i) as usize] = hit as f64 / (n * n) as f64;
        }
    }
    out
}

/// Length of the part of segment a-b inside the square [x, x+1] x [y, y+1] (Liang-Barsky).
fn clipped_len(a: Pt, b: Pt, x: f64, y: f64) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let mut t0 = 0.0f64;
    let mut t1 = 1.0f64;
    for (p, q) in [
        (-dx, a.0 - x),
        (dx, x + 1.0 - a.0),
        (-dy, a.1 - y),
        (dy, y + 1.0 - a.1),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return 0.0;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    if t1 <= t0 {
        0.0
    } else {
        (t1 - t0) * (dx * dx + dy * dy).sqrt()
    }
}

/// Bound on the error of `brute` with `n` samples per side against the true area of a pixel: an
/// error needs a sample cell touched by an edge; a segment of length L touches at most
/// `2 * L * n + 2` cells (each of area `1/n^2`).
pub fn brute_bound(p: &Polygon, i: u32, j: u32, n: u32) -> f64 {
    let mut l = 0.0;
    for ring in &p.0 {
        let m = ring.len();
        for k in 0..m {
            l += clipped_len(ring[k], ring[(k + 1) % m], i as f64, j as f64);
        }
    }
    let nf = n as f64;
    (2.0 * l * nf + 2.0 * ring_count(p) as f64) / (nf * nf)
}

fn ring_count(p: &Polygon) -> usize {
    p.0.len().max(1)
}

/// A small deterministic generator (a 64 bit linear congruence), for repeatable shapes.
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    /// A multiple of 1/8 in 0 to 8.
    pub fn eighth(&mut self) -> f64 {
        (self.next() % 65) as f64 / 8.0
    }
}
