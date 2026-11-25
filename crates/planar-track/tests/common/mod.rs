//! Synthetic scenes: a deterministic textured plane rendered through known homographies.
#![allow(dead_code)]

use planar_track::{Config, Dof, Image, Pose, Region, UpdateRule};

/// A deterministic hash of a lattice point to `[0, 1)`.
fn lattice(ix: i64, iy: i64, seed: u64) -> f64 {
    let mut h = (ix as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (iy as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ seed.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h >> 11) as f64 / 9_007_199_254_740_992.0
}

/// Value noise with smoothstep interpolation on cells of `cell` units.
fn value_noise(u: f64, v: f64, cell: f64, seed: u64) -> f64 {
    let x = u / cell;
    let y = v / cell;
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let (ix, iy) = (x0 as i64, y0 as i64);
    let a = lattice(ix, iy, seed);
    let b = lattice(ix + 1, iy, seed);
    let c = lattice(ix, iy + 1, seed);
    let d = lattice(ix + 1, iy + 1, seed);
    let top = a + (b - a) * sx;
    let bot = c + (d - c) * sx;
    top + (bot - top) * sy
}

/// The texture of the plane at a plane point, in `[0, 1]`.
pub fn texture(u: f64, v: f64) -> f64 {
    texture_seed(u, v, 7)
}

/// The texture for another seed (another plane pattern).
pub fn texture_seed(u: f64, v: f64, seed: u64) -> f64 {
    0.5 * value_noise(u, v, 9.0, seed)
        + 0.3 * value_noise(u, v, 5.0, seed + 1)
        + 0.2 * value_noise(u, v, 3.0, seed + 2)
}

pub const W: usize = 96;
pub const HT: usize = 96;

/// Render a frame: the pixel `x` shows the plane at `H^-1 x`, scaled by `gain` and shifted by
/// `offset`.
pub fn render(pose: &Pose, gain: f64, offset: f64, seed: u64) -> Image {
    let inv = pose.inverse().expect("invertible");
    Image::from_fn(W, HT, |x, y| {
        let (u, v) = inv.apply(x as f64, y as f64).expect("finite");
        gain * texture_seed(u, v, seed) + offset
    })
}

pub fn template() -> Image {
    render(&Pose::identity(), 1.0, 0.0, 7)
}

pub fn region() -> Region {
    Region::from_rect(W, HT, 24.0, 24.0, 71.0, 71.0).unwrap()
}

/// A pose built about the centre of the image: `T(c) * A(p) * T(-c)`.
pub fn about_centre(dof: Dof, p: &[f64]) -> Pose {
    let c = Pose::translation(48.0, 48.0);
    let ci = Pose::translation(-48.0, -48.0);
    c.compose(&Pose::from_params(dof, p)).compose(&ci)
}

/// The ground truth pose of frame `k` for a model: a small motion that grows with `k`.
pub fn truth(dof: Dof, k: usize) -> Pose {
    let k = k as f64;
    let p: Vec<f64> = match dof {
        Dof::Translation => vec![1.2 * k, -0.8 * k],
        Dof::TransScale => vec![0.9 * k, -0.6 * k, 0.008 * k],
        Dof::Similarity => vec![0.7 * k, 0.5 * k, 0.006 * k, 0.012 * k],
        Dof::Affine => vec![
            0.6 * k,
            -0.4 * k,
            0.006 * k,
            0.01 * k,
            -0.008 * k,
            0.004 * k,
        ],
        Dof::Homography => vec![
            0.6 * k,
            -0.4 * k,
            0.006 * k,
            0.01 * k,
            -0.008 * k,
            0.004 * k,
            4e-4 * k,
            -3e-4 * k,
        ],
    };
    about_centre(dof, &p)
}

/// The largest distance between the four region corners under two poses.
pub fn corner_err(a: &Pose, b: &Pose) -> f64 {
    let r = region();
    a.max_point_distance(b, &r.corners()).expect("mappable")
}

/// A test configuration: three levels, every pixel, the given model and rule.
pub fn config(dof: Dof, rule: UpdateRule) -> Config {
    let mut c = Config {
        dof,
        rule,
        ..Config::default()
    };
    c.pyramid.levels = 3;
    c.selection.min_fraction = 1.0;
    c
}

/// The tolerance on the largest corner error for noise free synthetic sequences, in pixels.
pub const TOL: f64 = 0.15;
