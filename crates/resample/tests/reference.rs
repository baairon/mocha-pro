//! Independent slow reference: direct triple loop over taps with clamp indexing, kernels
//! written separately from the crate (libm sin, different algebraic arrangement).
mod common;
use common::{cfg, img, ref_lanczos};
use resample::{resample, Filter};

fn clampi(i: i64, n: usize) -> usize {
    i.clamp(0, n as i64 - 1) as usize
}

fn keys_ref(t: f64, a: f64) -> f64 {
    let x = t.abs();
    if x < 1.0 {
        // Horner form
        ((a + 2.0) * x - (a + 3.0)) * x * x + 1.0
    } else if x < 2.0 {
        (((a * x - 5.0 * a) * x) + 8.0 * a) * x - 4.0 * a
    } else {
        0.0
    }
}

fn mitchell_ref(t: f64, b: f64, c: f64) -> f64 {
    let x = t.abs();
    if x < 1.0 {
        (((12.0 - 9.0 * b - 6.0 * c) * x + (-18.0 + 12.0 * b + 6.0 * c)) * x * x + 6.0 - 2.0 * b)
            / 6.0
    } else if x < 2.0 {
        ((((-b - 6.0 * c) * x + (6.0 * b + 30.0 * c)) * x + (-12.0 * b - 48.0 * c)) * x
            + 8.0 * b
            + 24.0 * c)
            / 6.0
    } else {
        0.0
    }
}

fn slow(
    src: (&[f32], usize, usize),
    (dw, dh): (usize, usize),
    s: (f64, f64, f64, f64),
    radius: i64,
    k: &dyn Fn(f64) -> f64,
) -> Vec<f64> {
    let (d, w, h) = src;
    let mut out = vec![0.0; dw * dh];
    for y in 0..dh {
        for x in 0..dw {
            let u = s.0 * (x as f64 + 0.5) + s.1 - 0.5;
            let v = s.2 * (y as f64 + 0.5) + s.3 - 0.5;
            let fu = u.floor() as i64;
            let fv = v.floor() as i64;
            let mut acc = 0.0;
            for j in fv - radius + 1..=fv + radius {
                for i in fu - radius + 1..=fu + radius {
                    let wgt = k(u - i as f64) * k(v - j as f64);
                    acc += wgt * f64::from(d[clampi(j, h) * w + clampi(i, w)]);
                }
            }
            out[y * dw + x] = acc;
        }
    }
    out
}

fn data() -> Vec<f32> {
    (0..42i32)
        .map(|i| ((i * 13).rem_euclid(17) as f32) * 0.5 - 2.0)
        .collect()
}

fn check(filter: Filter, radius: i64, k: &dyn Fn(f64) -> f64, s: (f64, f64, f64, f64), tol: f64) {
    let d = data();
    let map = move |x: f64, y: f64| (s.0 * x + s.1, s.2 * y + s.3);
    let out = resample(&img(&d, 7, 6, 1), 9, 8, map, &cfg(filter)).unwrap();
    let want = slow((&d, 7, 6), (9, 8), s, radius, k);
    for (i, (a, b)) in out.iter().zip(&want).enumerate() {
        assert!(
            (f64::from(*a) - b).abs() <= tol,
            "{filter:?} {i}: {a} vs {b}"
        );
    }
}

const SCALES: [(f64, f64, f64, f64); 4] = [
    (0.8, 0.0, 0.8, 0.0),
    (1.3, 0.2, 0.6, -0.4),
    (0.5, 1.1, 1.7, 0.3),
    (1.0, 0.37, 1.0, -0.61),
];

#[test]
fn bilinear_matches_slow_reference() {
    for s in SCALES {
        check(Filter::Bilinear, 1, &|t| (1.0 - t.abs()).max(0.0), s, 1e-5);
    }
}

#[test]
fn keys_matches_slow_reference() {
    for a in [-0.5, -0.75, -1.0] {
        for s in SCALES {
            check(Filter::Keys { a }, 2, &|t| keys_ref(t, a), s, 1e-5);
        }
    }
}

#[test]
fn mitchell_matches_slow_reference() {
    for (b, c) in [(0.0, 0.5), (1.0 / 3.0, 1.0 / 3.0), (1.0, 0.0), (0.25, 0.4)] {
        for s in SCALES {
            check(
                Filter::Mitchell { b, c },
                2,
                &|t| mitchell_ref(t, b, c),
                s,
                1e-5,
            );
        }
    }
}

#[test]
fn lanczos_matches_slow_reference_with_libm_sinc() {
    for lobes in 1..=4u32 {
        for s in SCALES {
            check(
                Filter::Lanczos { lobes },
                i64::from(lobes),
                &|t| ref_lanczos(t, f64::from(lobes)),
                s,
                1e-5,
            );
        }
    }
}

#[test]
fn nearest_matches_slow_reference() {
    let d = data();
    for s in SCALES {
        let map = move |x: f64, y: f64| (s.0 * x + s.1, s.2 * y + s.3);
        let out = resample(&img(&d, 7, 6, 1), 9, 8, map, &cfg(Filter::Nearest)).unwrap();
        for y in 0..8 {
            for x in 0..9 {
                let u = s.0 * (x as f64 + 0.5) + s.1 - 0.5;
                let v = s.2 * (y as f64 + 0.5) + s.3 - 0.5;
                let (iu, iv) = ((u + 0.5).floor() as i64, (v + 0.5).floor() as i64);
                // exact ties are measure zero for these scales except at integers plus half
                let want = d[clampi(iv, 6) * 7 + clampi(iu, 7)];
                if ((u + 0.5) - (u + 0.5).floor() - 0.0).abs() > 1e-9
                    && ((v + 0.5) - (v + 0.5).floor()).abs() > 1e-9
                {
                    assert_eq!(out[y * 9 + x], want, "{x} {y}");
                }
            }
        }
    }
}
