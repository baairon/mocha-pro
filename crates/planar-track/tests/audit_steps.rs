//! Independent slow reference for single Lucas-Kanade steps: finite difference Jacobians from
//! the definition of each rule, dyadic data so that every sum is exact in f64, and a dense
//! normal equation solve in exact rational arithmetic (i128 fractions).
#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]

use planar_track::align::{FrameLevel, LevelTemplate, Solver};
use planar_track::image::Image;
use planar_track::{Config, Dof, Illumination, Pose, Pyramid, Region, UpdateRule};

// ---- independent model table: q = (tx, ty, a11, a12, a21, a22, g1, g2) = M p ----
fn my_q(dof: Dof, p: &[f64]) -> [f64; 8] {
    let mut q = [0.0; 8];
    q[0] = p[0];
    q[1] = p[1];
    match dof {
        Dof::Translation => {}
        Dof::TransScale => {
            q[2] = p[2];
            q[5] = p[2];
        }
        Dof::Similarity => {
            q[2] = p[2];
            q[5] = p[2];
            q[4] = p[3];
            q[3] = -p[3];
        }
        Dof::Affine => q[2..6].copy_from_slice(&p[2..6]),
        Dof::Homography => q[2..8].copy_from_slice(&p[2..8]),
    }
    q
}
fn my_h(dof: Dof, p: &[f64]) -> [f64; 9] {
    let q = my_q(dof, p);
    [
        1.0 + q[2],
        q[3],
        q[0],
        q[4],
        1.0 + q[5],
        q[1],
        q[6],
        q[7],
        1.0,
    ]
}
fn mm(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut r = [0.0; 9];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                r[i * 3 + j] += a[i * 3 + k] * b[k * 3 + j];
            }
        }
    }
    r
}
fn wp(h: &[f64; 9], x: f64, y: f64) -> (f64, f64) {
    let z = h[6] * x + h[7] * y + h[8];
    (
        (h[0] * x + h[1] * y + h[2]) / z,
        (h[3] * x + h[4] * y + h[5]) / z,
    )
}
fn at(img: &[f64], w: usize, h: usize, x: i64, y: i64) -> f64 {
    img[(y.clamp(0, h as i64 - 1) as usize) * w + x.clamp(0, w as i64 - 1) as usize]
}
fn bil(img: &[f64], w: usize, h: usize, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (xi, yi) = (x0 as i64, y0 as i64);
    let a = at(img, w, h, xi, yi);
    let b = at(img, w, h, xi + 1, yi);
    let c = at(img, w, h, xi, yi + 1);
    let d = at(img, w, h, xi + 1, yi + 1);
    (1.0 - fy) * ((1.0 - fx) * a + fx * b) + fy * ((1.0 - fx) * c + fx * d)
}
fn grads(img: &[f64], w: usize, h: usize) -> (Vec<f64>, Vec<f64>) {
    let mut gx = vec![0.0; w * h];
    let mut gy = vec![0.0; w * h];
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            gx[y as usize * w + x as usize] =
                (at(img, w, h, x + 1, y) - at(img, w, h, x - 1, y)) / 2.0;
            gy[y as usize * w + x as usize] =
                (at(img, w, h, x, y + 1) - at(img, w, h, x, y - 1)) / 2.0;
        }
    }
    (gx, gy)
}

const N: usize = 12;
const RX0: usize = 3;
const RX1: usize = 7;
const RY0: usize = 3;
const RY1: usize = 6;

fn data(seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..N * N)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 33) % 4) as f64
        })
        .collect()
}

pub struct Ref {
    pub a: Vec<f64>,
    pub b: Vec<f64>,
    pub cost: f64,
}

/// Reference normal equations at pose parameters `p` (model `dof`), gain, offset.
fn reference(
    dof: Dof,
    rule: UpdateRule,
    p: &[f64],
    gain: f64,
    offset: f64,
    joint: bool,
    t: &[f64],
    f: &[f64],
    eps: f64,
) -> Ref {
    let n = dof.count();
    let m = n + if joint { 2 } else { 0 };
    let (tgx, tgy) = grads(t, N, N);
    let (fgx, fgy) = grads(f, N, N);
    let hp = my_h(dof, p);
    let mut a = vec![0.0; m * m];
    let mut b = vec![0.0; m];
    let mut cost = 0.0;
    for y in RY0..=RY1 {
        for x in RX0..=RX1 {
            let (xf, yf) = (x as f64, y as f64);
            let (u, v) = wp(&hp, xf, yf);
            let val = bil(f, N, N, u, v);
            let tv = t[y * N + x];
            let r = val - ((1.0 + gain) * tv + offset);
            cost += r * r;
            let gi = (bil(&fgx, N, N, u, v), bil(&fgy, N, N, u, v));
            let gt = (tgx[y * N + x], tgy[y * N + x]);
            let mut c = vec![0.0; m];
            for k in 0..n {
                let mut pp = p.to_vec();
                let mut pm = p.to_vec();
                pp[k] += eps;
                pm[k] -= eps;
                let mut ep = vec![0.0; n];
                let mut em = vec![0.0; n];
                ep[k] = eps;
                em[k] = -eps;
                // FA: W(p + e) central difference
                let fa = {
                    let (u1, v1) = wp(&my_h(dof, &pp), xf, yf);
                    let (u2, v2) = wp(&my_h(dof, &pm), xf, yf);
                    ((u1 - u2) / (2.0 * eps), (v1 - v2) / (2.0 * eps))
                };
                // FC: W(H * H(e)) central difference
                let fc = {
                    let (u1, v1) = wp(&mm(&hp, &my_h(dof, &ep)), xf, yf);
                    let (u2, v2) = wp(&mm(&hp, &my_h(dof, &em)), xf, yf);
                    ((u1 - u2) / (2.0 * eps), (v1 - v2) / (2.0 * eps))
                };
                // template side: W(e) at the pixel
                let tw = {
                    let (u1, v1) = wp(&my_h(dof, &ep), xf, yf);
                    let (u2, v2) = wp(&my_h(dof, &em), xf, yf);
                    ((u1 - u2) / (2.0 * eps), (v1 - v2) / (2.0 * eps))
                };
                let jfa = gi.0 * fa.0 + gi.1 * fa.1;
                let jfc = gi.0 * fc.0 + gi.1 * fc.1;
                let jt = gt.0 * tw.0 + gt.1 * tw.1;
                c[k] = match rule {
                    UpdateRule::ForwardAdditive => jfa,
                    UpdateRule::ForwardCompositional => jfc,
                    UpdateRule::InverseCompositional => -(1.0 + gain) * jt,
                    UpdateRule::Esm => 0.5 * (jfc + (1.0 + gain) * jt),
                };
            }
            if joint {
                c[n] = -tv;
                c[n + 1] = -1.0;
            }
            for i in 0..m {
                b[i] += c[i] * r;
                for j in 0..m {
                    a[i * m + j] += c[i] * c[j];
                }
            }
        }
    }
    Ref { a, b, cost }
}

// ---- exact rationals ----
#[derive(Clone, Copy, Debug)]
struct Fr(i128, i128);
fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}
impl Fr {
    fn new(n: i128, d: i128) -> Fr {
        let g = gcd(n, d).max(1);
        let s = if d < 0 { -1 } else { 1 };
        Fr(s * n / g, s * d / g)
    }
    fn from_f64(v: f64) -> Fr {
        let mut d: i128 = 1;
        let mut x = v;
        while x != x.floor() {
            x *= 2.0;
            d *= 2;
            assert!(d < (1i128 << 60), "not dyadic enough");
        }
        Fr::new(x as i128, d)
    }
    fn to_f64(self) -> f64 {
        self.0 as f64 / self.1 as f64
    }
    fn sub(self, o: Fr) -> Fr {
        let g = gcd(self.1, o.1).max(1);
        let (d1, d2) = (self.1 / g, o.1 / g);
        Fr::new(self.0 * d2 - o.0 * d1, self.1 * d2)
    }
    fn mul(self, o: Fr) -> Fr {
        let g1 = gcd(self.0, o.1).max(1);
        let g2 = gcd(o.0, self.1).max(1);
        Fr::new((self.0 / g1) * (o.0 / g2), (self.1 / g2) * (o.1 / g1))
    }
    fn div(self, o: Fr) -> Fr {
        self.mul(Fr::new(o.1, o.0))
    }
}
fn solve_exact(a: &[f64], b: &[f64], m: usize) -> Vec<f64> {
    let mut g: Vec<Vec<Fr>> = (0..m)
        .map(|i| {
            let mut row: Vec<Fr> = (0..m).map(|j| Fr::from_f64(a[i * m + j])).collect();
            row.push(Fr::from_f64(b[i]));
            row
        })
        .collect();
    for c in 0..m {
        let piv = (c..m).find(|&r| g[r][c].0 != 0).expect("singular");
        g.swap(c, piv);
        for r in 0..m {
            if r != c {
                let f = g[r][c].div(g[c][c]);
                for k in c..=m {
                    let t = g[c][k].mul(f);
                    g[r][k] = g[r][k].sub(t);
                }
            }
        }
    }
    (0..m).map(|i| g[i][m].div(g[i][i]).to_f64()).collect()
}

fn setup(dof: Dof, rule: UpdateRule, illum: Illumination) -> (Config, Image, Image, Region) {
    let mut cfg = Config {
        dof,
        rule,
        illumination: illum,
        ..Config::default()
    };
    cfg.pyramid.levels = 1;
    cfg.selection.min_fraction = 1.0;
    let t = Image::new(N, N, data(11)).unwrap();
    let f = Image::new(N, N, data(29)).unwrap();
    let region = Region::from_rect(N, N, RX0 as f64, RY0 as f64, RX1 as f64, RY1 as f64).unwrap();
    (cfg, t, f, region)
}

fn params_for(dof: Dof) -> Vec<f64> {
    let all = [0.25, -0.125, 0.0625, -0.03125, 0.046875, 0.015625, 0.0, 0.0];
    all[..dof.count()].to_vec()
}

#[test]
fn normal_equations_match_the_reference_all_rules_all_affine_models() {
    for &dof in &[
        Dof::Translation,
        Dof::TransScale,
        Dof::Similarity,
        Dof::Affine,
    ] {
        for &rule in &UpdateRule::ALL {
            for &(joint, gain, offset) in
                &[(false, 0.0, 0.0), (true, 0.25, -0.5), (false, 0.125, 1.0)]
            {
                let illum = if joint {
                    Illumination::Joint
                } else {
                    Illumination::Off
                };
                let (cfg, t, f, region) = setup(dof, rule, illum);
                let mut p = params_for(dof);
                p[0] += 2.0; // shift the window inside the frame
                let pose = Pose { h: my_h(dof, &p) };
                let tp = Pyramid::build(&t, &cfg.pyramid).unwrap();
                let fp = Pyramid::build(&f, &cfg.pyramid).unwrap();
                let tpl = LevelTemplate::build(tp.level(0), &region, &cfg).unwrap();
                let fr = FrameLevel::new(fp.level(0));
                let solver = Solver::new(&cfg, &tpl, &fr);
                let lin = solver.linearize(&pose, gain, offset, true);
                let rf = reference(dof, rule, &p, gain, offset, joint, t.data(), f.data(), 1.0);
                assert_eq!(lin.valid, 20);
                let tag = format!("{dof:?} {rule:?} joint={joint} g={gain}");
                assert!(
                    (lin.cost - rf.cost).abs() <= 1e-9 * (1.0 + rf.cost),
                    "cost {tag}"
                );
                for i in 0..rf.b.len() {
                    assert!(
                        (lin.b[i] - rf.b[i]).abs() <= 1e-9 * (1.0 + rf.b[i].abs()),
                        "b[{i}] {tag}: {} vs {}",
                        lin.b[i],
                        rf.b[i]
                    );
                }
                for i in 0..rf.a.len() {
                    assert!(
                        (lin.a[i] - rf.a[i]).abs() <= 1e-9 * (1.0 + rf.a[i].abs()),
                        "a[{i}] {tag}: {} vs {}",
                        lin.a[i],
                        rf.a[i]
                    );
                }
            }
        }
    }
}

#[test]
fn homography_normal_equations_match_a_finite_difference_reference() {
    let dof = Dof::Homography;
    for &rule in &UpdateRule::ALL {
        let (cfg, t, f, region) = setup(dof, rule, Illumination::Off);
        let p = vec![
            2.25, -0.125, 0.0625, -0.03125, 0.046875, 0.015625, 0.004, -0.003,
        ];
        let pose = Pose { h: my_h(dof, &p) };
        let tp = Pyramid::build(&t, &cfg.pyramid).unwrap();
        let fp = Pyramid::build(&f, &cfg.pyramid).unwrap();
        let tpl = LevelTemplate::build(tp.level(0), &region, &cfg).unwrap();
        let fr = FrameLevel::new(fp.level(0));
        let solver = Solver::new(&cfg, &tpl, &fr);
        let lin = solver.linearize(&pose, 0.0, 0.0, true);
        let rf = reference(dof, rule, &p, 0.0, 0.0, false, t.data(), f.data(), 1e-6);
        for i in 0..8 {
            assert!(
                (lin.b[i] - rf.b[i]).abs() <= 1e-5 * (1.0 + rf.b[i].abs()),
                "b[{i}] {rule:?}: {} vs {}",
                lin.b[i],
                rf.b[i]
            );
        }
        for i in 0..64 {
            assert!(
                (lin.a[i] - rf.a[i]).abs() <= 1e-5 * (1.0 + rf.a[i].abs()),
                "a[{i}] {rule:?}: {} vs {}",
                lin.a[i],
                rf.a[i]
            );
        }
    }
}

#[test]
fn damped_solve_matches_exact_rational_elimination() {
    for &dof in &[Dof::Translation] {
        for &rule in &UpdateRule::ALL {
            let (cfg, t, f, region) = setup(dof, rule, Illumination::Off);
            let p: Vec<f64> = [2.25, -0.25, 0.125, -0.125][..dof.count()].to_vec();
            let pose = Pose { h: my_h(dof, &p) };
            let tp = Pyramid::build(&t, &cfg.pyramid).unwrap();
            let fp = Pyramid::build(&f, &cfg.pyramid).unwrap();
            let tpl = LevelTemplate::build(tp.level(0), &region, &cfg).unwrap();
            let fr = FrameLevel::new(fp.level(0));
            let solver = Solver::new(&cfg, &tpl, &fr);
            let lin = solver.linearize(&pose, 0.0, 0.0, true);
            let m = dof.count();
            let mut a = lin.a.clone();
            for i in 0..m {
                a[i * m + i] += 1.0;
            }
            let rhs: Vec<f64> = lin.b.iter().map(|v| -v).collect();
            let exact = solve_exact(&a, &rhs, m);
            let got = planar_track::linalg::solve_spd(&a, &rhs, m).unwrap();
            for i in 0..m {
                assert!(
                    (exact[i] - got[i]).abs() <= 1e-12 * (1.0 + exact[i].abs()),
                    "{dof:?} {rule:?}"
                );
            }
        }
    }
}

#[test]
fn model_table_matches_the_independent_definition() {
    for &dof in &Dof::ALL {
        let p: Vec<f64> = (0..dof.count()).map(|k| 0.01 * (k as f64 + 1.0)).collect();
        assert_eq!(Pose::from_params(dof, &p).h, my_h(dof, &p), "{dof:?}");
        let back = Pose { h: my_h(dof, &p) }.params(dof).unwrap();
        for k in 0..p.len() {
            assert!((back[k] - p[k]).abs() < 1e-15, "{dof:?}");
        }
    }
}
