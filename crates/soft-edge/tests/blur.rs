use soft_edge::*;

fn lit(w: usize, h: usize, x: usize, y: usize) -> Vec<f32> {
    let mut v = vec![0.0f32; w * h];
    v[y * w + x] = 1.0;
    v
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Slow f64 reference: direct 2D convolution with the outer product of the kernel.
fn slow(src: &[f32], w: usize, h: usize, k: &[f32], b: Border) -> Vec<f64> {
    let half = (k.len() / 2) as i64;
    let mut out = vec![0.0f64; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0f64;
            for (j, &ky) in k.iter().enumerate() {
                for (i, &kx) in k.iter().enumerate() {
                    let sy = resolve_index(y as i64 + j as i64 - half, h, b);
                    let sx = resolve_index(x as i64 + i as i64 - half, w, b);
                    if let (Some(sx), Some(sy)) = (sx, sy) {
                        acc += ky as f64 * kx as f64 * src[sy * w + sx] as f64;
                    }
                }
            }
            out[y * w + x] = acc;
        }
    }
    out
}

fn close(a: &[f32], b: &[f64], tol: f64) {
    for (x, y) in a.iter().zip(b) {
        assert!((*x as f64 - y).abs() < tol, "{x} vs {y}");
    }
}

#[test]
fn lit_pixel_box_r1_gives_kernel() {
    let t = 1.0f32 / 3.0;
    let out = box_blur(&lit(5, 5, 2, 2), 5, 5, 1.0, &Config::default()).unwrap();
    for y in 0..5 {
        for x in 0..5 {
            let want = if (1..=3).contains(&x) && (1..=3).contains(&y) {
                t * t
            } else {
                0.0
            };
            assert_eq!(out[y * 5 + x].to_bits(), want.to_bits());
        }
    }
}

#[test]
fn lit_pixel_box_half_exact() {
    let out = box_blur(&lit(3, 3, 1, 1), 3, 3, 0.5, &Config::default()).unwrap();
    let want = [
        0.0625f32, 0.125, 0.0625, 0.125, 0.25, 0.125, 0.0625, 0.125, 0.0625,
    ];
    assert_eq!(out, want);
}

#[test]
fn lit_pixel_gauss_gives_outer_product() {
    let cfg = Config::default();
    let k = gaussian_kernel(GaussSpec::Sigma(1.0), &cfg).unwrap();
    let n = k.len();
    let out = gaussian_blur(&lit(n, n, n / 2, n / 2), n, n, GaussSpec::Sigma(1.0), &cfg).unwrap();
    for y in 0..n {
        for x in 0..n {
            assert_eq!(out[y * n + x].to_bits(), (k[x] * k[y]).to_bits());
        }
    }
}

#[test]
fn lit_pixel_gauss_recurrence_gives_outer_product() {
    let cfg = Config {
        exp_method: ExpMethod::Recurrence,
        ..Config::default()
    };
    let spec = GaussSpec::Radius(3.0);
    let k = gaussian_kernel(spec, &cfg).unwrap();
    let n = k.len();
    let out = gaussian_blur(&lit(n, n, n / 2, n / 2), n, n, spec, &cfg).unwrap();
    for y in 0..n {
        for x in 0..n {
            assert_eq!(out[y * n + x].to_bits(), (k[x] * k[y]).to_bits());
        }
    }
}

#[test]
fn half_plane_box_half_radius_exact_ramp() {
    // one row, x < 4 is 0 and x >= 4 is 1, clamp border, radius 0.5 = [.25 .5 .25]
    let row: Vec<f32> = (0..8).map(|x| if x >= 4 { 1.0 } else { 0.0 }).collect();
    let out = box_blur(&row, 8, 1, 0.5, &Config::default()).unwrap();
    assert_eq!(out, vec![0.0, 0.0, 0.0, 0.25, 0.75, 1.0, 1.0, 1.0]);
}

#[test]
fn half_plane_box_r1_ramp() {
    let row: Vec<f32> = (0..8).map(|x| if x >= 4 { 1.0 } else { 0.0 }).collect();
    let out = box_blur(&row, 8, 1, 1.0, &Config::default()).unwrap();
    let t = 1.0f32 / 3.0;
    assert_eq!(out[2], 0.0);
    // one row with clamp: the column pass sums three equal rows of weight 1/3
    assert!((out[3] - t).abs() < 1e-6);
    assert!((out[4] - 2.0 * t).abs() < 1e-6);
    assert!((out[5] - 1.0).abs() < 1e-6);
    assert_eq!(out[0], 0.0);
}

#[test]
fn half_plane_box_r2_ramp_values() {
    let row: Vec<f32> = (0..12).map(|x| if x >= 6 { 1.0 } else { 0.0 }).collect();
    let out = box_blur(&row, 12, 1, 2.0, &Config::default()).unwrap();
    let want = [0.0, 0.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.0];
    // indices 2..: x=4 sees 6 in tap +2 -> 0.2
    let expect = [(4, 0.2), (5, 0.4), (6, 0.6), (7, 0.8), (8, 1.0), (3, 0.0)];
    for (i, v) in expect {
        assert!((out[i] - v).abs() < 1e-6, "{i}");
    }
    let _ = want;
}

#[test]
fn half_plane_ramp_is_linear_second_difference_zero() {
    let row: Vec<f32> = (0..16).map(|x| if x >= 8 { 1.0 } else { 0.0 }).collect();
    let out = box_blur(&row, 16, 1, 3.0, &Config::default()).unwrap();
    for x in 6..=9 {
        let d1 = out[x + 1] - out[x];
        let d2 = out[x] - out[x - 1];
        assert!((d1 - d2).abs() < 1e-6);
    }
}

#[test]
fn constant_stays_constant_clamp_and_mirror() {
    for b in [Border::Clamp, Border::Mirror, Border::MirrorNoRepeat] {
        let cfg = Config {
            border: b,
            ..Config::default()
        };
        let src = vec![0.5f32; 6 * 4];
        let out = box_blur(&src, 6, 4, 2.0, &cfg).unwrap();
        assert!(out.iter().all(|&v| (v - 0.5).abs() < 1e-6));
    }
}

#[test]
fn zero_border_darkens_edges() {
    let cfg = Config {
        border: Border::Zero,
        ..Config::default()
    };
    let out = box_blur(&[1.0; 4], 4, 1, 0.5, &cfg).unwrap();
    // rows: [.75 1 1 .75], then the column pass sees only the middle tap of weight .5
    assert_eq!(out, vec![0.375, 0.5, 0.5, 0.375]);
}

#[test]
fn clamp_vs_mirror_differ_at_radius_two() {
    let src = [1.0f32, 0.0, 0.0, 0.0];
    let c = box_blur(&src, 4, 1, 2.0, &Config::default()).unwrap();
    let m = box_blur(
        &src,
        4,
        1,
        2.0,
        &Config {
            border: Border::Mirror,
            ..Config::default()
        },
    )
    .unwrap();
    // pixel 0, taps -2..2 -> clamp: 1,1,1,0,0 = .6 ; mirror: idx 1,0,0,1,2 -> 0,1,1,0,0 = .4
    assert!((c[0] - 0.6).abs() < 1e-6);
    assert!((m[0] - 0.4).abs() < 1e-6);
}

#[test]
fn box_matches_slow_reference_all_borders() {
    let src: Vec<f32> = (0..7 * 5)
        .map(|i| ((i * 37 + 11) % 17) as f32 / 16.0)
        .collect();
    for b in Border::ALL {
        for r in [0.0f32, 0.5, 1.0, 1.75, 3.0] {
            let cfg = Config {
                border: b,
                ..Config::default()
            };
            let k = box_kernel(r, cfg.frac_rule).unwrap();
            let got = box_blur(&src, 7, 5, r, &cfg).unwrap();
            close(&got, &slow(&src, 7, 5, &k, b), 1e-5);
        }
    }
}

#[test]
fn gauss_matches_slow_reference_all_borders() {
    let src: Vec<f32> = (0..9 * 6)
        .map(|i| ((i * 29 + 5) % 13) as f32 / 12.0)
        .collect();
    for b in Border::ALL {
        let cfg = Config {
            border: b,
            ..Config::default()
        };
        let spec = GaussSpec::Sigma(1.3);
        let k = gaussian_kernel(spec, &cfg).unwrap();
        close(
            &gaussian_blur(&src, 9, 6, spec, &cfg).unwrap(),
            &slow(&src, 9, 6, &k, b),
            1e-5,
        );
    }
}

#[test]
fn repeated_box_two_passes_is_kernel_square() {
    // two passes of [.25 .5 .25] on a lit pixel (1D row, clamp is irrelevant away from edges)
    let out = box_blur_repeated(&lit(9, 1, 4, 0), 9, 1, 0.5, 2, &Config::default()).unwrap();
    let want = [0.0, 0.0, 0.0625, 0.25, 0.375, 0.25, 0.0625, 0.0, 0.0];
    assert_eq!(out, want.to_vec());
}

#[test]
fn repeated_zero_passes_is_copy() {
    let src = lit(4, 4, 1, 2);
    assert_eq!(
        box_blur_repeated(&src, 4, 4, 1.0, 0, &Config::default()).unwrap(),
        src
    );
}

#[test]
fn repeated_one_pass_equals_single() {
    let src: Vec<f32> = (0..30).map(|i| (i % 7) as f32 / 7.0).collect();
    let cfg = Config::default();
    assert_eq!(
        bits(&box_blur_repeated(&src, 6, 5, 1.5, 1, &cfg).unwrap()),
        bits(&box_blur(&src, 6, 5, 1.5, &cfg).unwrap())
    );
}

#[test]
fn repeated_three_passes_equals_chained() {
    let src: Vec<f32> = (0..30).map(|i| (i % 5) as f32 / 5.0).collect();
    let cfg = Config::default();
    let mut c = src.clone();
    for _ in 0..3 {
        c = box_blur(&c, 6, 5, 1.0, &cfg).unwrap();
    }
    assert_eq!(
        bits(&box_blur_repeated(&src, 6, 5, 1.0, 3, &cfg).unwrap()),
        bits(&c)
    );
}

#[test]
fn size_mismatch_errors() {
    let cfg = Config::default();
    assert_eq!(
        box_blur(&[0.0; 5], 2, 2, 1.0, &cfg),
        Err(Error::SizeMismatch)
    );
    assert_eq!(
        box_blur_repeated(&[0.0; 5], 2, 2, 1.0, 2, &cfg),
        Err(Error::SizeMismatch)
    );
    assert_eq!(
        gaussian_blur(&[0.0; 5], 2, 2, GaussSpec::Sigma(1.0), &cfg),
        Err(Error::SizeMismatch)
    );
    assert_eq!(
        feather(&[0.0; 5], 2, 2, 1.0, &cfg),
        Err(Error::SizeMismatch)
    );
    assert_eq!(
        signed_distance(&[0.0; 5], 2, 2, &cfg),
        Err(Error::SizeMismatch)
    );
}

#[test]
fn empty_buffers_ok() {
    let cfg = Config::default();
    assert_eq!(box_blur(&[], 0, 3, 1.0, &cfg).unwrap(), Vec::<f32>::new());
    assert_eq!(
        gaussian_blur(&[], 3, 0, GaussSpec::Sigma(1.0), &cfg).unwrap(),
        Vec::<f32>::new()
    );
    assert_eq!(feather(&[], 0, 0, 1.0, &cfg).unwrap(), Vec::<f32>::new());
}

#[test]
fn even_kernel_rejected() {
    assert!(convolve_separable(&[0.0; 4], 2, 2, &[0.5, 0.5], Border::Clamp).is_err());
}

#[test]
fn one_by_one_all_borders() {
    for b in Border::ALL {
        let cfg = Config {
            border: b,
            ..Config::default()
        };
        let out = box_blur(&[1.0], 1, 1, 1.0, &cfg).unwrap()[0];
        let t = 1.0f32 / 3.0;
        let want = if b == Border::Zero { t * t } else { 1.0 };
        assert!((out - want).abs() < 1e-6, "{b:?}");
    }
}

#[test]
fn non_square_separable_order_irrelevant_for_symmetric_input() {
    let cfg = Config::default();
    let src = lit(7, 3, 3, 1);
    let a = box_blur(&src, 7, 3, 1.0, &cfg).unwrap();
    // transpose, blur, transpose back
    let mut t = vec![0.0f32; 21];
    for y in 0..3 {
        for x in 0..7 {
            t[x * 3 + y] = src[y * 7 + x];
        }
    }
    let tb = box_blur(&t, 3, 7, 1.0, &cfg).unwrap();
    for y in 0..3 {
        for x in 0..7 {
            assert_eq!(a[y * 7 + x].to_bits(), tb[x * 3 + y].to_bits());
        }
    }
}

#[test]
fn blur_is_deterministic() {
    let src: Vec<f32> = (0..64).map(|i| ((i * 13) % 10) as f32 / 9.0).collect();
    let cfg = Config::default();
    let a = gaussian_blur(&src, 8, 8, GaussSpec::Sigma(1.4), &cfg).unwrap();
    for _ in 0..3 {
        assert_eq!(
            bits(&a),
            bits(&gaussian_blur(&src, 8, 8, GaussSpec::Sigma(1.4), &cfg).unwrap())
        );
    }
}

#[test]
fn gauss_sigma0_is_copy() {
    let src: Vec<f32> = (0..12).map(|i| i as f32).collect();
    let out = gaussian_blur(&src, 4, 3, GaussSpec::Sigma(0.0), &Config::default()).unwrap();
    assert_eq!(out, src);
}

#[test]
fn blur_conserves_mass_in_interior() {
    let out = box_blur(&lit(15, 15, 7, 7), 15, 15, 2.5, &Config::default()).unwrap();
    let s: f32 = out.iter().sum();
    assert!((s - 1.0).abs() < 1e-5);
}

#[test]
fn fractional_end_taps_lit_pixel_values() {
    // radius 0.25: kernel [a b a], a = 0.25/1.5, b = 1/1.5 on a single row
    let out = box_blur(&lit(3, 1, 1, 0), 3, 1, 0.25, &Config::default()).unwrap();
    let a = 0.25f32 / 1.5;
    let b = 1.0f32 / 1.5;
    assert_eq!(out, vec![a, b, a]);
}

#[test]
fn golden_bits_box_third() {
    assert_eq!((1.0f32 / 3.0).to_bits(), 0x3eaa_aaab);
    let k = box_kernel(1.0, FracRule::EndTaps).unwrap();
    assert_eq!(k[1].to_bits(), 0x3eaa_aaab);
}

#[test]
fn golden_bits_dyadic_ramp() {
    let row: Vec<f32> = (0..4).map(|x| if x >= 2 { 1.0 } else { 0.0 }).collect();
    let out = box_blur(&row, 4, 1, 0.5, &Config::default()).unwrap();
    assert_eq!(bits(&out), vec![0, 0x3e80_0000, 0x3f40_0000, 0x3f80_0000]);
}
