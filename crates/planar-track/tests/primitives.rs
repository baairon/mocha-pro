//! Image, pyramid, distance transform, model, region and configuration tests, with expected
//! values computed independently in the tests.

#![allow(clippy::needless_range_loop, clippy::manual_is_multiple_of)]

use planar_track::edt::{distance_to_outside, dt1d};
use planar_track::image::Image;
use planar_track::linalg::{inv3, mul3, solve_spd};
use planar_track::pyramid::{binomial_kernel, blur};
use planar_track::{
    Acceptance, BoundaryWeight, Config, ConfigSpace, Dof, Error, Pose, Pyramid, PyramidConfig,
    Region, UpdateRule,
};

fn img(w: usize, h: usize, v: &[f64]) -> Image {
    Image::new(w, h, v.to_vec()).unwrap()
}

// ---- image -------------------------------------------------------------------------------

#[test]
fn new_rejects_bad_sizes_and_values() {
    assert!(Image::new(0, 3, vec![]).is_err());
    assert!(Image::new(2, 2, vec![0.0; 3]).is_err());
    assert!(Image::new(1, 1, vec![f64::NAN]).is_err());
    assert!(Image::new(1, 1, vec![f64::INFINITY]).is_err());
    assert!(Image::new(2, 2, vec![0.0; 4]).is_ok());
}

#[test]
fn sample_at_pixel_centres_is_exact() {
    let a = img(3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    for y in 0..2 {
        for x in 0..3 {
            assert_eq!(a.sample_clamped(x as f64, y as f64), a.at(x, y));
            assert_eq!(a.sample_inside(x as f64, y as f64), Some(a.at(x, y)));
        }
    }
}

#[test]
fn bilinear_value_at_a_fixed_point() {
    // [[1, 2], [3, 5]] at (0.25, 0.5): top 0.75 + 0.5 = 1.25, bottom 2.25 + 1.25 = 3.5,
    // result 0.5 * 1.25 + 0.5 * 3.5 = 2.375.
    let a = img(2, 2, &[1.0, 2.0, 3.0, 5.0]);
    assert_eq!(a.sample_inside(0.25, 0.5), Some(2.375));
    assert_eq!(a.sample_clamped(0.25, 0.5), 2.375);
}

#[test]
fn bilinear_value_at_the_middle_is_the_mean() {
    let a = img(2, 2, &[1.0, 2.0, 3.0, 5.0]);
    assert_eq!(a.sample_inside(0.5, 0.5), Some(2.75));
}

#[test]
fn bilinear_is_exact_for_a_plane() {
    let a = Image::from_fn(5, 4, |x, y| 3.0 * x as f64 - 2.0 * y as f64 + 1.0);
    for &(x, y) in &[(0.5, 0.5), (3.25, 1.75), (3.9, 2.1), (0.0, 3.0), (4.0, 0.0)] {
        let got = a.sample_inside(x, y).unwrap();
        assert!((got - (3.0 * x - 2.0 * y + 1.0)).abs() < 1e-12);
    }
}

#[test]
fn clamp_border_rule_replicates_edges() {
    let a = img(2, 2, &[1.0, 2.0, 3.0, 5.0]);
    assert_eq!(a.sample_clamped(-5.0, 0.0), 1.0);
    assert_eq!(a.sample_clamped(9.0, 1.0), 5.0);
    assert_eq!(a.sample_clamped(0.5, -3.0), 1.5);
    assert_eq!(a.sample_clamped(0.5, 7.0), 4.0);
}

#[test]
fn inside_border_rule_refuses_outside() {
    let a = img(2, 2, &[1.0, 2.0, 3.0, 5.0]);
    assert_eq!(a.sample_inside(-0.001, 0.0), None);
    assert_eq!(a.sample_inside(1.001, 0.0), None);
    assert_eq!(a.sample_inside(0.0, 1.001), None);
    assert_eq!(a.sample_inside(f64::NAN, 0.0), None);
    assert_eq!(a.sample_inside(1.0, 1.0), Some(5.0));
}

#[test]
fn clamped_integer_access() {
    let a = img(2, 2, &[1.0, 2.0, 3.0, 5.0]);
    assert_eq!(a.at_clamped(-1, -1), 1.0);
    assert_eq!(a.at_clamped(5, 5), 5.0);
    assert_eq!(a.at_clamped(1, 0), 2.0);
}

#[test]
fn gradient_of_a_fixed_image() {
    // 3 by 3:  1 2 4 / 8 16 32 / 3 5 7.
    let a = img(3, 3, &[1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 3.0, 5.0, 7.0]);
    let (gx, gy) = a.gradients();
    // gx: row 0: (2-1)/2, (4-1)/2, (4-2)/2; row 1: (16-8)/2, (32-8)/2, (32-16)/2; row 2: 1, 2, 1.
    assert_eq!(gx.data(), &[0.5, 1.5, 1.0, 4.0, 12.0, 8.0, 1.0, 2.0, 1.0]);
    // gy: column 0: (8-1)/2, (3-1)/2, (3-8)/2; column 1: (16-2)/2, (5-2)/2, (5-16)/2;
    // column 2: (32-4)/2, (7-4)/2, (7-32)/2.
    assert_eq!(
        gy.data(),
        &[3.5, 7.0, 14.0, 1.0, 1.5, 1.5, -2.5, -5.5, -12.5]
    );
}

#[test]
fn gradient_of_a_ramp_is_its_slope_inside() {
    let a = Image::from_fn(6, 5, |x, y| 2.0 * x as f64 + 3.0 * y as f64);
    let (gx, gy) = a.gradients();
    for y in 1..4 {
        for x in 1..5 {
            assert_eq!(gx.at(x, y), 2.0);
            assert_eq!(gy.at(x, y), 3.0);
        }
    }
    // Edge pixels see the replicated sample: half the slope.
    assert_eq!(gx.at(0, 2), 1.0);
    assert_eq!(gy.at(2, 0), 1.5);
}

#[test]
fn gradient_of_a_constant_is_zero() {
    let a = Image::from_fn(4, 4, |_, _| 0.7);
    let (gx, gy) = a.gradients();
    assert!(gx.data().iter().chain(gy.data()).all(|&v| v == 0.0));
}

// ---- pyramid ---------------------------------------------------------------------------------

#[test]
fn binomial_kernels() {
    assert_eq!(binomial_kernel(1), vec![1.0]);
    assert_eq!(binomial_kernel(3), vec![0.25, 0.5, 0.25]);
    assert_eq!(binomial_kernel(5), vec![0.0625, 0.25, 0.375, 0.25, 0.0625]);
    assert_eq!(
        binomial_kernel(7),
        vec![
            1.0 / 64.0,
            6.0 / 64.0,
            15.0 / 64.0,
            20.0 / 64.0,
            15.0 / 64.0,
            6.0 / 64.0,
            1.0 / 64.0
        ]
    );
}

#[test]
fn kernels_sum_to_one_and_are_symmetric() {
    for taps in [1, 3, 5, 7, 9, 11, 13, 15] {
        let k = binomial_kernel(taps);
        assert_eq!(k.len(), taps);
        assert_eq!(k.iter().sum::<f64>(), 1.0);
        for i in 0..taps {
            assert_eq!(k[i], k[taps - 1 - i]);
        }
    }
}

#[test]
fn blur_with_one_tap_is_identity() {
    let a = Image::from_fn(5, 5, |x, y| (x * 7 + y * 3) as f64);
    assert_eq!(blur(&a, &binomial_kernel(1)), a);
}

#[test]
fn blur_of_a_constant_is_the_constant() {
    let a = Image::from_fn(6, 6, |_, _| 0.375);
    assert_eq!(blur(&a, &binomial_kernel(5)), a);
}

#[test]
fn blur_of_an_impulse_is_the_outer_product() {
    let a = Image::from_fn(7, 7, |x, y| if (x, y) == (3, 3) { 16.0 } else { 0.0 });
    let b = blur(&a, &binomial_kernel(3));
    let k = [1.0, 2.0, 1.0];
    for dy in 0..3 {
        for dx in 0..3 {
            assert_eq!(b.at(2 + dx, 2 + dy), 16.0 * k[dx] * k[dy] / 16.0);
        }
    }
    assert_eq!(b.at(0, 0), 0.0);
}

/// Integer reference: a 3 tap blur with edge replication, scaled by 16 (kernel `[1 2 1] x [1 2 1]`).
fn blur3_times16(v: &[i64], w: usize, h: usize) -> Vec<i64> {
    let at = |x: i64, y: i64| {
        v[(y.clamp(0, h as i64 - 1) as usize) * w + x.clamp(0, w as i64 - 1) as usize]
    };
    let k = [1, 2, 1];
    let mut out = vec![0; w * h];
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            let mut s = 0;
            for dy in 0..3 {
                for dx in 0..3 {
                    s += k[dx] * k[dy] * at(x + dx as i64 - 1, y + dy as i64 - 1);
                }
            }
            out[y as usize * w + x as usize] = s;
        }
    }
    out
}

#[test]
fn pyramid_of_a_fixed_image_matches_integer_arithmetic() {
    let vals: Vec<i64> = vec![3, 9, 1, 7, 2, 8, 4, 6, 5, 0, 9, 3, 1, 7, 2, 8];
    let image = Image::new(4, 4, vals.iter().map(|&v| v as f64).collect()).unwrap();
    let cfg = PyramidConfig {
        levels: 2,
        scale: 2.0,
        kernel_taps: 3,
        min_size: 1,
    };
    let p = Pyramid::build(&image, &cfg).unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p.level(0).image, image);
    let b16 = blur3_times16(&vals, 4, 4);
    let l1 = &p.level(1).image;
    assert_eq!((l1.width(), l1.height()), (2, 2));
    // Level 1 pixel (i, j) is the mean of the blurred 2 by 2 block at (2i, 2j) (position 2i + 1/2).
    for j in 0..2 {
        for i in 0..2 {
            let s = b16[(2 * j) * 4 + 2 * i]
                + b16[(2 * j) * 4 + 2 * i + 1]
                + b16[(2 * j + 1) * 4 + 2 * i]
                + b16[(2 * j + 1) * 4 + 2 * i + 1];
            assert_eq!(l1.at(i, j), s as f64 / 64.0, "pixel {i},{j}");
        }
    }
}

#[test]
fn pyramid_level_maps_for_scale_two() {
    let image = Image::from_fn(16, 16, |x, y| (x + y) as f64);
    let cfg = PyramidConfig {
        levels: 3,
        scale: 2.0,
        kernel_taps: 3,
        min_size: 1,
    };
    let p = Pyramid::build(&image, &cfg).unwrap();
    assert_eq!(p.len(), 3);
    let l1 = p.level(1);
    assert_eq!((l1.sx, l1.ox), (2.0, 0.5));
    let l2 = p.level(2);
    assert_eq!((l2.image.width(), l2.sx, l2.ox), (4, 4.0, 1.5));
}

#[test]
fn level_matrices_are_inverse_of_each_other() {
    let image = Image::from_fn(20, 14, |x, y| (x * y) as f64);
    let cfg = PyramidConfig {
        levels: 3,
        scale: 1.5,
        kernel_taps: 5,
        min_size: 4,
    };
    let p = Pyramid::build(&image, &cfg).unwrap();
    for i in 0..p.len() {
        let l = p.level(i);
        let m = mul3(&l.to_base(), &l.from_base());
        for (a, b) in m.iter().zip([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]) {
            assert!((a - b).abs() < 1e-12);
        }
    }
}

#[test]
fn pyramid_sizes_follow_the_rounding_rule() {
    let image = Image::from_fn(25, 17, |x, y| (x + y) as f64);
    let cfg = PyramidConfig {
        levels: 3,
        scale: 2.0,
        kernel_taps: 3,
        min_size: 1,
    };
    let p = Pyramid::build(&image, &cfg).unwrap();
    // floor(25 / 2 + 1/2) = 13, floor(17 / 2 + 1/2) = 9; then 7 and 5.
    assert_eq!(
        (p.level(1).image.width(), p.level(1).image.height()),
        (13, 9)
    );
    assert_eq!(
        (p.level(2).image.width(), p.level(2).image.height()),
        (7, 5)
    );
}

#[test]
fn pyramid_stops_at_the_minimum_size() {
    let image = Image::from_fn(40, 40, |x, y| (x + y) as f64);
    let cfg = PyramidConfig {
        levels: 8,
        scale: 2.0,
        kernel_taps: 3,
        min_size: 12,
    };
    let p = Pyramid::build(&image, &cfg).unwrap();
    assert_eq!(p.len(), 2); // 40, 20; the next would be 10 < 12
    assert!(!p.is_empty());
}

#[test]
fn pyramid_of_a_constant_stays_constant() {
    let image = Image::from_fn(32, 32, |_, _| 0.5);
    let p = Pyramid::build(&image, &PyramidConfig::default()).unwrap();
    for i in 0..p.len() {
        assert!(p.level(i).image.data().iter().all(|&v| v == 0.5));
    }
}

#[test]
fn pyramid_config_validation() {
    let ok = PyramidConfig::default();
    assert!(ok.validate().is_ok());
    assert!(PyramidConfig {
        levels: 0,
        ..ok.clone()
    }
    .validate()
    .is_err());
    assert!(PyramidConfig {
        scale: 1.0,
        ..ok.clone()
    }
    .validate()
    .is_err());
    assert!(PyramidConfig {
        scale: 9.0,
        ..ok.clone()
    }
    .validate()
    .is_err());
    assert!(PyramidConfig {
        kernel_taps: 4,
        ..ok.clone()
    }
    .validate()
    .is_err());
    assert!(PyramidConfig {
        kernel_taps: 17,
        ..ok.clone()
    }
    .validate()
    .is_err());
    assert!(PyramidConfig { min_size: 0, ..ok }.validate().is_err());
}

// ---- distance transform ----------------------------------------------------------------------

#[test]
fn dt1d_of_a_single_zero() {
    let inf = 1e20;
    let d = dt1d(&[inf, inf, 0.0, inf, inf]);
    assert_eq!(d, vec![4.0, 1.0, 0.0, 1.0, 4.0]);
}

#[test]
fn dt1d_of_two_zeros() {
    let inf = 1e20;
    let d = dt1d(&[0.0, inf, inf, inf, inf, 0.0]);
    assert_eq!(d, vec![0.0, 1.0, 4.0, 4.0, 1.0, 0.0]);
}

#[test]
fn dt1d_with_finite_heights() {
    // min over p of ((q - p)^2 + f[p]) by hand for f = [4, 9, 1, 9].
    let d = dt1d(&[4.0, 9.0, 1.0, 9.0]);
    assert_eq!(d, vec![4.0, 2.0, 1.0, 2.0]);
}

#[test]
fn dt1d_empty_and_single() {
    assert!(dt1d(&[]).is_empty());
    assert_eq!(dt1d(&[3.0]), vec![3.0]);
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 33
}

fn brute(mask: &[bool], w: usize, h: usize, border: bool) -> Vec<f64> {
    let mut out = Vec::new();
    let (lo, hi_x, hi_y) = if border {
        (-1i64, w as i64, h as i64)
    } else {
        (0, w as i64 - 1, h as i64 - 1)
    };
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            if !mask[y as usize * w + x as usize] {
                out.push(0.0);
                continue;
            }
            let mut best = f64::INFINITY;
            for oy in lo..=hi_y {
                for ox in lo..=hi_x {
                    let outside = ox < 0
                        || oy < 0
                        || ox >= w as i64
                        || oy >= h as i64
                        || !mask[oy as usize * w + ox as usize];
                    if outside {
                        let d = ((x - ox) * (x - ox) + (y - oy) * (y - oy)) as f64;
                        if d < best {
                            best = d;
                        }
                    }
                }
            }
            out.push(best.sqrt());
        }
    }
    out
}

#[test]
fn distance_transform_matches_brute_force_on_random_masks() {
    let mut s = 12345u64;
    for round in 0..6 {
        let (w, h) = (9 + round, 7 + round);
        let mask: Vec<bool> = (0..w * h).map(|_| lcg(&mut s) % 4 != 0).collect();
        for border in [true, false] {
            if !border && mask.iter().all(|&m| m) {
                continue;
            }
            let got = distance_to_outside(&mask, w, h, border);
            let want = brute(&mask, w, h, border);
            if !border && !mask.iter().any(|&m| !m) {
                continue;
            }
            assert_eq!(got, want, "round {round}, border {border}");
        }
    }
}

#[test]
fn distance_transform_of_a_filled_square() {
    let (w, h) = (7, 7);
    let mask: Vec<bool> = (0..w * h)
        .map(|i| (1..6).contains(&(i % w)) && (1..6).contains(&(i / w)))
        .collect();
    let d = distance_to_outside(&mask, w, h, true);
    assert_eq!(d[3 * w + 3], 3.0);
    assert_eq!(d[w + 1], 1.0);
    assert_eq!(d[0], 0.0);
    assert_eq!(d[2 * w + 2], 2.0);
}

#[test]
fn border_flag_changes_edge_distances() {
    let mask = vec![true; 5 * 5];
    let with = distance_to_outside(&mask, 5, 5, true);
    assert_eq!(with[0], 1.0);
    assert_eq!(with[2 * 5 + 2], 3.0);
    let without = distance_to_outside(&mask, 5, 5, false);
    assert!(without[0] > 1e5, "no outside pixel at all");
}

#[test]
fn diagonal_distances_are_euclidean() {
    let mut mask = vec![true; 6 * 6];
    mask[0] = false;
    let d = distance_to_outside(&mask, 6, 6, false);
    assert_eq!(d[0], 0.0);
    assert_eq!(d[6 + 1], 2.0f64.sqrt());
    assert_eq!(d[3 * 6 + 4], 5.0);
}

// ---- linear algebra --------------------------------------------------------------------------

#[test]
fn inverse_of_a_known_matrix() {
    let m = [2.0, 0.0, 1.0, 0.0, 4.0, 0.0, 0.0, 0.0, 1.0];
    assert_eq!(
        inv3(&m).unwrap(),
        [0.5, 0.0, -0.5, 0.0, 0.25, 0.0, 0.0, 0.0, 1.0]
    );
    assert!(inv3(&[0.0; 9]).is_none());
    assert!(inv3(&[1.0, 2.0, 3.0, 2.0, 4.0, 6.0, 0.0, 0.0, 1.0]).is_none());
}

#[test]
fn cholesky_solves_a_small_system() {
    // A = [[4, 2], [2, 3]], x = [1, 2] gives b = [8, 8].
    let x = solve_spd(&[4.0, 2.0, 2.0, 3.0], &[8.0, 8.0], 2).unwrap();
    assert!((x[0] - 1.0).abs() < 1e-12 && (x[1] - 2.0).abs() < 1e-12);
    assert!(solve_spd(&[1.0, 2.0, 2.0, 1.0], &[1.0, 1.0], 2).is_none());
    assert!(solve_spd(&[0.0], &[1.0], 1).is_none());
}

// ---- model -------------------------------------------------------------------------------------

#[test]
fn parameter_counts() {
    let counts: Vec<usize> = Dof::ALL.iter().map(|d| d.count()).collect();
    assert_eq!(counts, vec![2, 3, 4, 6, 8]);
}

#[test]
fn zero_parameters_are_the_identity() {
    for d in Dof::ALL {
        assert_eq!(
            Pose::from_params(d, &vec![0.0; d.count()]),
            Pose::identity()
        );
    }
}

#[test]
fn translation_pose_matrix() {
    let p = Pose::from_params(Dof::Translation, &[3.0, -2.0]);
    assert_eq!(p.h, [1.0, 0.0, 3.0, 0.0, 1.0, -2.0, 0.0, 0.0, 1.0]);
}

#[test]
fn scale_pose_matrix() {
    let p = Pose::from_params(Dof::TransScale, &[1.0, 2.0, 0.5]);
    assert_eq!(p.h, [1.5, 0.0, 1.0, 0.0, 1.5, 2.0, 0.0, 0.0, 1.0]);
}

#[test]
fn similarity_pose_matrix() {
    let p = Pose::from_params(Dof::Similarity, &[1.0, 2.0, 0.25, 0.5]);
    assert_eq!(p.h, [1.25, -0.5, 1.0, 0.5, 1.25, 2.0, 0.0, 0.0, 1.0]);
}

#[test]
fn affine_pose_matrix() {
    let p = Pose::from_params(Dof::Affine, &[1.0, 2.0, 0.1, 0.2, 0.3, 0.4]);
    assert_eq!(p.h, [1.1, 0.2, 1.0, 0.3, 1.4, 2.0, 0.0, 0.0, 1.0]);
}

#[test]
fn homography_pose_matrix() {
    let p = Pose::from_params(Dof::Homography, &[1.0, 2.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
    assert_eq!(p.h, [1.1, 0.2, 1.0, 0.3, 1.4, 2.0, 0.5, 0.6, 1.0]);
}

#[test]
fn params_round_trip_for_every_model() {
    let full = [1.5, -2.5, 0.125, 0.25, -0.375, 0.5, 0.0625, -0.03125];
    for d in Dof::ALL {
        let p: Vec<f64> = match d {
            Dof::TransScale => vec![1.5, -2.5, 0.125],
            Dof::Similarity => vec![1.5, -2.5, 0.125, 0.25],
            _ => full[..d.count()].to_vec(),
        };
        let pose = Pose::from_params(d, &p);
        assert_eq!(pose.params(d).unwrap(), p, "{}", d.name());
    }
}

#[test]
fn projection_averages_the_diagonal_for_scale() {
    let pose = Pose::from_params(Dof::Affine, &[0.0, 0.0, 0.25, 0.0, 0.0, 0.5]);
    let p = pose.params(Dof::TransScale).unwrap();
    assert_eq!(p[2], 0.375);
}

#[test]
fn projection_of_rotation_onto_similarity() {
    let pose = Pose::from_params(Dof::Affine, &[0.0, 0.0, 0.0, -0.5, 0.25, 0.0]);
    let p = pose.params(Dof::Similarity).unwrap();
    // b from (a21 - a12) / 2 = (0.25 + 0.5) / 2.
    assert_eq!(p[3], 0.375);
    assert_eq!(p[2], 0.0);
}

#[test]
fn compose_applies_the_right_operand_first() {
    let t = Pose::translation(1.0, 0.0);
    let s = Pose::from_params(Dof::TransScale, &[0.0, 0.0, 1.0]); // scale by 2
    assert_eq!(t.compose(&s).apply(3.0, 0.0), Some((7.0, 0.0)));
    assert_eq!(s.compose(&t).apply(3.0, 0.0), Some((8.0, 0.0)));
}

#[test]
fn inverse_round_trip() {
    let p = Pose::from_params(
        Dof::Homography,
        &[3.0, 1.0, 0.1, 0.05, -0.02, 0.2, 0.001, 0.002],
    );
    let q = p.inverse().unwrap();
    let id = p.compose(&q).normalised().unwrap();
    for (a, b) in id.h.iter().zip(Pose::identity().h) {
        assert!((a - b).abs() < 1e-12);
    }
    let (x, y) = p.apply(5.0, 7.0).unwrap();
    let (u, v) = q.apply(x, y).unwrap();
    assert!((u - 5.0).abs() < 1e-9 && (v - 7.0).abs() < 1e-9);
}

#[test]
fn closed_models_stay_closed_under_composition() {
    let a = Pose::from_params(Dof::Similarity, &[1.0, 2.0, 0.1, 0.2]);
    let b = Pose::from_params(Dof::Similarity, &[-1.0, 0.5, -0.05, 0.1]);
    let c = a.compose(&b);
    let p = c.params(Dof::Similarity).unwrap();
    let back = Pose::from_params(Dof::Similarity, &p);
    for (x, y) in c.normalised().unwrap().h.iter().zip(back.h) {
        assert!((x - y).abs() < 1e-12);
    }
}

#[test]
fn normalised_and_singular_cases() {
    let p = Pose {
        h: [2.0, 0.0, 4.0, 0.0, 2.0, 6.0, 0.0, 0.0, 2.0],
    };
    assert_eq!(
        p.normalised().unwrap().h,
        [1.0, 0.0, 2.0, 0.0, 1.0, 3.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(
        Pose {
            h: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0]
        }
        .normalised(),
        Err(Error::Singular)
    );
    assert_eq!(Pose { h: [0.0; 9] }.inverse(), Err(Error::Singular));
}

#[test]
fn apply_refuses_the_plane_at_infinity() {
    let p = Pose {
        h: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
    };
    assert!(p.apply(0.0, 1.0).is_none());
    assert!(p.apply(1.0, 1.0).is_some());
}

#[test]
fn max_point_distance_of_two_translations() {
    let a = Pose::translation(1.0, 0.0);
    let b = Pose::translation(4.0, 4.0);
    assert_eq!(
        a.max_point_distance(&b, &[(0.0, 0.0), (9.0, 9.0)]),
        Some(5.0)
    );
}

// ---- region ------------------------------------------------------------------------------------

#[test]
fn rect_region_counts_and_corners() {
    let r = Region::from_rect(10, 8, 2.0, 1.0, 5.0, 3.0).unwrap();
    assert_eq!(r.count(), 4 * 3);
    assert_eq!(
        r.corners(),
        [(2.0, 1.0), (5.0, 1.0), (5.0, 3.0), (2.0, 3.0)]
    );
    assert!(r.mask()[10 + 2]);
    assert!(!r.mask()[10 + 1]);
    assert_eq!((r.width(), r.height()), (10, 8));
}

#[test]
fn quad_region_is_orientation_independent() {
    let a = Region::from_quad(12, 12, [(2.0, 2.0), (9.0, 2.0), (9.0, 9.0), (2.0, 9.0)]).unwrap();
    let b = Region::from_quad(12, 12, [(2.0, 2.0), (2.0, 9.0), (9.0, 9.0), (9.0, 2.0)]).unwrap();
    assert_eq!(a.mask(), b.mask());
    assert_eq!(a.count(), 64);
}

#[test]
fn diamond_region_is_smaller_than_its_box() {
    let d = Region::from_quad(12, 12, [(6.0, 1.0), (11.0, 6.0), (6.0, 11.0), (1.0, 6.0)]).unwrap();
    assert!(d.count() < 11 * 11 && d.count() > 40);
    assert!(d.mask()[6 * 12 + 6]);
    assert!(!d.mask()[12 + 1]);
}

#[test]
fn mask_region_has_bounding_box_corners() {
    let mut m = vec![false; 6 * 6];
    m[2 * 6 + 1] = true;
    m[4 * 6 + 3] = true;
    let r = Region::from_mask(6, 6, m).unwrap();
    assert_eq!(
        r.corners(),
        [(1.0, 2.0), (3.0, 2.0), (3.0, 4.0), (1.0, 4.0)]
    );
}

#[test]
fn empty_and_mismatched_regions_are_refused() {
    assert!(Region::from_mask(3, 3, vec![false; 9]).is_err());
    assert!(Region::from_mask(3, 3, vec![true; 8]).is_err());
    assert!(Region::from_rect(5, 5, 10.0, 10.0, 12.0, 12.0).is_err());
    assert!(Region::from_quad(0, 5, [(0.0, 0.0); 4]).is_err());
}

// ---- configuration ----------------------------------------------------------------------------

#[test]
fn default_configuration_is_valid() {
    assert!(Config::default().validate().is_ok());
}

macro_rules! invalid {
    ($name:ident, $edit:expr) => {
        #[test]
        fn $name() {
            let mut c = Config::default();
            let edit: fn(&mut Config) = $edit;
            edit(&mut c);
            assert!(matches!(c.validate(), Err(Error::Config(_))));
        }
    };
}

invalid!(rejects_negative_lambda, |c| c.lm.lambda_init = -1.0);
invalid!(rejects_lambda_up_one, |c| c.lm.lambda_up = 1.0);
invalid!(rejects_lambda_down_zero, |c| c.lm.lambda_down = 0.0);
invalid!(rejects_lambda_down_above_one, |c| c.lm.lambda_down = 1.5);
invalid!(rejects_gain_ratio_one, |c| c.lm.acceptance =
    Acceptance::GainRatio(1.0));
invalid!(rejects_empty_level_params, |c| c.level_params.clear());
invalid!(rejects_zero_iterations, |c| c.level_params[0]
    .max_iterations = 0);
invalid!(rejects_zero_fraction, |c| c.selection.min_fraction = 0.0);
invalid!(rejects_fraction_above_one, |c| c.selection.min_fraction =
    1.5);
invalid!(rejects_zero_min_weight, |c| c.selection.min_weight = 0.0);
invalid!(rejects_zero_ramp, |c| c.selection.boundary =
    BoundaryWeight::Linear { ramp: 0.0 });
invalid!(rejects_zero_ncc_step, |c| c.ncc.step = 0);
invalid!(rejects_bad_ncc_fraction, |c| c.ncc.min_valid_fraction = 0.0);
invalid!(rejects_bad_ncc_score, |c| c.ncc.min_score = 2.0);
invalid!(rejects_negative_inlier_threshold, |c| c.inlier_threshold =
    -0.1);
invalid!(rejects_zero_min_pixels, |c| c.min_valid_pixels = 0);
invalid!(rejects_bad_pyramid, |c| c.pyramid.levels = 0);

#[test]
fn boundary_weight_formulas() {
    let lin = BoundaryWeight::Linear { ramp: 4.0 };
    assert_eq!(lin.weight(1.0), 0.25);
    assert_eq!(lin.weight(9.0), 1.0);
    let sm = BoundaryWeight::Smoothstep { ramp: 4.0 };
    assert_eq!(sm.weight(2.0), 0.5);
    assert_eq!(sm.weight(1.0), 0.0625 * (3.0 - 0.5));
    let q = BoundaryWeight::Quadratic { ramp: 4.0 };
    assert_eq!(q.weight(2.0), 0.25);
    assert_eq!(BoundaryWeight::Off.weight(0.0), 1.0);
}

#[test]
fn level_params_index_from_the_coarsest() {
    let mut c = Config::default();
    let mut lp = c.level_params[0].clone();
    lp.max_iterations = 7;
    c.level_params.push(lp);
    assert_eq!(c.params_for_level(3, 4).max_iterations, 30);
    assert_eq!(c.params_for_level(2, 4).max_iterations, 7);
    assert_eq!(c.params_for_level(0, 4).max_iterations, 7);
}

#[test]
fn config_space_size_is_the_product_of_the_dimensions() {
    let space = ConfigSpace;
    let dims = space.dimensions();
    assert_eq!(dims.len(), 8);
    assert_eq!(space.size(), 5 * 4 * 3 * 2 * 3 * 4 * 2 * 2);
    assert_eq!(
        space.size(),
        dims.iter().map(|d| d.choices.len()).product::<usize>()
    );
}

#[test]
fn config_space_entries_are_valid_and_distinct() {
    let space = ConfigSpace;
    let all: Vec<Config> = space.iter().collect();
    assert_eq!(all.len(), space.size());
    for c in &all {
        assert!(c.validate().is_ok());
    }
    let mut seen = std::collections::HashSet::new();
    for c in &all {
        assert!(seen.insert(format!("{c:?}")));
    }
}

#[test]
fn config_space_first_and_last() {
    let space = ConfigSpace;
    let first = space.get(0).unwrap();
    assert_eq!(
        (first.dof, first.rule),
        (Dof::Translation, UpdateRule::ForwardAdditive)
    );
    let last = space.get(space.size() - 1).unwrap();
    assert_eq!((last.dof, last.rule), (Dof::Homography, UpdateRule::Esm));
    assert!(last.ncc.enabled);
    assert!(space.get(space.size()).is_none());
    assert!(space.digits(space.size()).is_none());
}

#[test]
fn config_space_digits_decode_in_mixed_radix() {
    let space = ConfigSpace;
    assert_eq!(space.digits(0).unwrap(), vec![0; 8]);
    assert_eq!(space.digits(1).unwrap(), vec![0, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(space.digits(2).unwrap(), vec![0, 0, 0, 0, 0, 0, 1, 0]);
}

#[test]
fn config_space_covers_every_model_and_rule_pair() {
    let mut pairs = std::collections::HashSet::new();
    for c in ConfigSpace.iter() {
        pairs.insert((c.dof, c.rule));
    }
    assert_eq!(pairs.len(), 20);
}
