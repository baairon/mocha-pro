#![allow(dead_code)]
use resample::{resample, Config, Filter, Image, Layout};

pub fn img(data: &[f32], w: usize, h: usize, ch: usize) -> Image<'_> {
    Image {
        data,
        width: w,
        height: h,
        channels: ch,
        layout: Layout::Packed,
    }
}

pub fn cfg(filter: Filter) -> Config {
    Config {
        filter,
        ..Config::default()
    }
}

/// Run with a closure mapping, one channel packed.
pub fn run1<M: Fn(f64, f64) -> (f64, f64)>(
    data: &[f32],
    w: usize,
    h: usize,
    dw: usize,
    dh: usize,
    map: M,
    c: &Config,
) -> Vec<f32> {
    resample(&img(data, w, h, 1), dw, dh, map, c).unwrap()
}

pub fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

pub fn close(a: &[f32], b: &[f64], tol: f64) {
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        assert!((f64::from(*x) - y).abs() <= tol, "index {i}: {x} vs {y}");
    }
}

/// Every filter variant used by the identity tests.
pub fn filters() -> Vec<Filter> {
    vec![
        Filter::Nearest,
        Filter::Bilinear,
        Filter::Keys { a: -0.5 },
        Filter::Keys { a: -0.75 },
        Filter::Keys { a: -1.0 },
        Filter::Mitchell { b: 0.0, c: 0.5 },
        Filter::Lanczos { lobes: 1 },
        Filter::Lanczos { lobes: 2 },
        Filter::Lanczos { lobes: 3 },
        Filter::Box,
    ]
}

/// Independent slow reference sinc using libm (tests only).
pub fn ref_sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let p = std::f64::consts::PI * x;
        p.sin() / p
    }
}

pub fn ref_lanczos(t: f64, a: f64) -> f64 {
    if t.abs() >= a {
        0.0
    } else {
        ref_sinc(t) * ref_sinc(t / a)
    }
}
