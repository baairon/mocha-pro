use soft_edge::*;

fn hp(n: usize, edge: usize) -> Vec<f32> {
    (0..n).map(|x| if x < edge { 1.0 } else { 0.0 }).collect()
}

fn cfg(profile: Profile, direction: Direction) -> Config {
    Config {
        profile,
        direction,
        ..Config::default()
    }
}

#[test]
fn ramp_endpoints_all_profiles() {
    for p in [Profile::Linear, Profile::Smoothstep, Profile::Cosine] {
        assert_eq!(ramp(0.0, p), 0.0);
        assert_eq!(ramp(1.0, p), 1.0);
        assert_eq!(ramp(-5.0, p), 0.0);
        assert_eq!(ramp(7.0, p), 1.0);
        assert_eq!(ramp(f32::INFINITY, p), 1.0);
        assert_eq!(ramp(f32::NEG_INFINITY, p), 0.0);
    }
}

#[test]
fn ramp_linear_identity() {
    for t in [0.125f32, 0.3, 0.5, 0.99] {
        assert_eq!(ramp(t, Profile::Linear), t);
    }
}

#[test]
fn ramp_smoothstep_hand() {
    assert_eq!(ramp(0.5, Profile::Smoothstep), 0.5);
    assert_eq!(ramp(0.25, Profile::Smoothstep), 0.15625);
    assert_eq!(ramp(0.75, Profile::Smoothstep), 0.84375);
}

#[test]
fn ramp_cosine_vs_libm() {
    for i in 0..=64 {
        let t = i as f32 / 64.0;
        let want = 0.5 - 0.5 * (std::f64::consts::PI * t as f64).cos();
        assert!(
            (ramp(t, Profile::Cosine) as f64 - want).abs() < 3e-7,
            "t={t}"
        );
    }
}

#[test]
fn ramp_cosine_half_close() {
    assert!((ramp(0.5, Profile::Cosine) - 0.5).abs() < 1e-7);
}

#[test]
fn ramp_cosine_symmetry() {
    for i in 1..50 {
        let t = i as f32 / 64.0;
        let s = ramp(t, Profile::Cosine) + ramp(1.0 - t, Profile::Cosine);
        assert!((s - 1.0).abs() < 1e-6);
    }
}

#[test]
fn ramps_monotone() {
    for p in [Profile::Linear, Profile::Smoothstep, Profile::Cosine] {
        let mut prev = -1.0f32;
        for i in 0..=200 {
            let v = ramp(i as f32 / 200.0, p);
            assert!(v >= prev);
            prev = v;
        }
    }
}

#[test]
fn ramp_cosine_quarter_hand() {
    // 0.5 - 0.5 cos(pi/4) = 0.14644660940672624
    assert!((ramp(0.25, Profile::Cosine) - 0.146_446_61).abs() < 3e-7);
}

// half plane: inside x < 8 of 16, one row (clamp border). signed distance at x: 7.5 - x for
// inside side, i.e. 0.5 at x=7, -0.5 at x=8.

#[test]
fn inward_linear_hand() {
    let out = feather(
        &hp(16, 8),
        16,
        1,
        4.0,
        &cfg(Profile::Linear, Direction::Inward),
    )
    .unwrap();
    // t = s / 4 : x=7 -> .125, 6 -> .375, 5 -> .625, 4 -> .875, 3 -> 1
    assert_eq!(&out[3..=8], &[1.0, 0.875, 0.625, 0.375, 0.125, 0.0]);
    assert!(out[9..].iter().all(|&v| v == 0.0));
    assert_eq!(out[0], 1.0);
}

#[test]
fn outward_linear_hand() {
    let out = feather(
        &hp(16, 8),
        16,
        1,
        4.0,
        &cfg(Profile::Linear, Direction::Outward),
    )
    .unwrap();
    // t = (s + 4)/4 : x=7 -> 1.125 -> 1, x=8 -> .875, 9 -> .625, 10 -> .375, 11 -> .125, 12 -> 0
    assert_eq!(&out[6..=12], &[1.0, 1.0, 0.875, 0.625, 0.375, 0.125, 0.0]);
    assert_eq!(out[15], 0.0);
}

#[test]
fn centred_linear_hand() {
    let out = feather(
        &hp(16, 8),
        16,
        1,
        4.0,
        &cfg(Profile::Linear, Direction::Centred),
    )
    .unwrap();
    // t = (s + 2)/4 : x=7 -> .625, 8 -> .375, 6 -> .875, 9 -> .125, 10 -> 0, 5 -> 1
    assert_eq!(&out[5..=10], &[1.0, 0.875, 0.625, 0.375, 0.125, 0.0]);
}

#[test]
fn inward_smoothstep_hand() {
    let out = feather(
        &hp(16, 8),
        16,
        1,
        4.0,
        &cfg(Profile::Smoothstep, Direction::Inward),
    )
    .unwrap();
    // t = .125 -> .125^2 * (3 - .25) = .04296875 ; t = .375 -> .140625 * 2.25
    assert_eq!(out[7], 0.042_968_75);
    assert_eq!(out[6], 0.140_625 * 2.25);
    assert_eq!(out[8], 0.0);
}

#[test]
fn outward_smoothstep_hand() {
    let out = feather(
        &hp(16, 8),
        16,
        1,
        4.0,
        &cfg(Profile::Smoothstep, Direction::Outward),
    )
    .unwrap();
    // x = 11: t = .125
    assert_eq!(out[11], 0.042_968_75);
    assert_eq!(out[7], 1.0);
}

#[test]
fn centred_smoothstep_symmetric() {
    let out = feather(
        &hp(16, 8),
        16,
        1,
        4.0,
        &cfg(Profile::Smoothstep, Direction::Centred),
    )
    .unwrap();
    assert_eq!(out[7] + out[8], 1.0);
    assert_eq!(out[6] + out[9], 1.0);
}

#[test]
fn cosine_directions_vs_libm() {
    for d in [Direction::Inward, Direction::Outward, Direction::Centred] {
        let out = feather(&hp(16, 8), 16, 1, 6.0, &cfg(Profile::Cosine, d)).unwrap();
        let lo = match d {
            Direction::Inward => 0.0,
            Direction::Outward => -6.0,
            Direction::Centred => -3.0,
        };
        for (x, &o) in out.iter().enumerate() {
            let s = 7.5 - x as f64;
            let t = ((s - lo) / 6.0).clamp(0.0, 1.0);
            let want = 0.5 - 0.5 * (std::f64::consts::PI * t).cos();
            assert!((o as f64 - want).abs() < 3e-7, "{d:?} x={x}");
        }
    }
}

#[test]
fn feather_2d_vertical_edge_rows_equal() {
    let (w, h) = (10, 6);
    let m: Vec<f32> = (0..w * h)
        .map(|i| if i % w < 5 { 1.0 } else { 0.0 })
        .collect();
    let out = feather(&m, w, h, 2.0, &cfg(Profile::Linear, Direction::Inward)).unwrap();
    for y in 1..h {
        assert_eq!(&out[..w], &out[y * w..(y + 1) * w]);
    }
    assert_eq!(out[4], 0.25);
    assert_eq!(out[3], 0.75);
}

#[test]
fn feather_2d_horizontal_edge_columns_equal() {
    let (w, h) = (4, 10);
    let m: Vec<f32> = (0..w * h)
        .map(|i| if i / w >= 5 { 1.0 } else { 0.0 })
        .collect();
    let out = feather(&m, w, h, 2.0, &cfg(Profile::Linear, Direction::Outward)).unwrap();
    // outside is rows < 5; row 4 is 0.5 from the crossing: t = (-0.5 + 2)/2 = .75
    assert_eq!(out[4 * w], 0.75);
    assert_eq!(out[3 * w + 1], 0.25);
    assert_eq!(out[5 * w], 1.0);
}

#[test]
fn feather_square_corner_distance() {
    // 3x3 inside block in a 9x9 field; the pixel diagonal to the corner is sqrt(2) outside
    let (w, h) = (9, 9);
    let m: Vec<f32> = (0..w * h)
        .map(|i| {
            if (3..6).contains(&(i % w)) && (3..6).contains(&(i / w)) {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    let out = feather(&m, w, h, 4.0, &cfg(Profile::Linear, Direction::Outward)).unwrap();
    let s = -(2.0f64.sqrt() as f32 - 0.5);
    assert_eq!(out[2 * w + 2], (s + 4.0) / 4.0);
    assert_eq!(out[2 * w + 3], 0.875);
}

#[test]
fn feather_uniform_masks() {
    for d in [Direction::Inward, Direction::Outward, Direction::Centred] {
        let c = cfg(Profile::Smoothstep, d);
        assert!(feather(&[1.0; 4], 2, 2, 3.0, &c)
            .unwrap()
            .iter()
            .all(|&v| v == 1.0));
        assert!(feather(&[0.0; 4], 2, 2, 3.0, &c)
            .unwrap()
            .iter()
            .all(|&v| v == 0.0));
    }
}

#[test]
fn zero_border_feathers_image_edge() {
    let c = Config {
        border: Border::Zero,
        ..cfg(Profile::Linear, Direction::Inward)
    };
    let out = feather(&[1.0; 8], 8, 1, 4.0, &c).unwrap();
    // ring above and below: distance 1 -> s = .5 -> .125
    assert_eq!(out, vec![0.125; 8]);
}

#[test]
fn bad_width() {
    for w in [0.0f32, -1.0, f32::NAN, f32::INFINITY] {
        assert!(feather(&[1.0, 0.0], 2, 1, w, &Config::default()).is_err());
    }
}

#[test]
fn feather_output_in_unit_range_whole_space() {
    let m: Vec<f32> = (0..7 * 7)
        .map(|i| ((i * 5) % 3 == 0) as u8 as f32)
        .collect();
    for c in Config::enumerate() {
        let out = feather(&m, 7, 7, 3.0, &c).unwrap();
        assert!(out.iter().all(|v| (0.0..=1.0).contains(v)), "{c:?}");
    }
}

#[test]
fn whole_space_blurs_run_and_are_deterministic() {
    let src: Vec<f32> = (0..36).map(|i| (i % 4) as f32 / 3.0).collect();
    for c in Config::enumerate() {
        let a = box_blur(&src, 6, 6, 1.5, &c).unwrap();
        assert_eq!(a, box_blur(&src, 6, 6, 1.5, &c).unwrap());
        let g = gaussian_blur(&src, 6, 6, GaussSpec::Sigma(1.0), &c).unwrap();
        assert_eq!(
            g,
            gaussian_blur(&src, 6, 6, GaussSpec::Sigma(1.0), &c).unwrap()
        );
    }
}

#[test]
fn feather_deterministic() {
    let m = hp(16, 8);
    let c = cfg(Profile::Cosine, Direction::Centred);
    let a = feather(&m, 16, 1, 5.0, &c).unwrap();
    let b: Vec<u32> = a.iter().map(|v| v.to_bits()).collect();
    for _ in 0..3 {
        let r: Vec<u32> = feather(&m, 16, 1, 5.0, &c)
            .unwrap()
            .iter()
            .map(|v| v.to_bits())
            .collect();
        assert_eq!(b, r);
    }
}

#[test]
fn feather_edge_offset_zero_shifts_ramp() {
    let c = Config {
        edge_offset: 0.0,
        ..cfg(Profile::Linear, Direction::Inward)
    };
    let out = feather(&hp(16, 8), 16, 1, 4.0, &c).unwrap();
    assert_eq!(&out[4..=8], &[1.0, 0.75, 0.5, 0.25, 0.0]);
}

#[test]
fn feather_width_scales_ramp() {
    let c = cfg(Profile::Linear, Direction::Inward);
    let w2 = feather(&hp(16, 8), 16, 1, 2.0, &c).unwrap();
    assert_eq!(w2[7], 0.25);
    assert_eq!(w2[6], 0.75);
    assert_eq!(w2[5], 1.0);
}

#[test]
fn centred_is_half_at_crossing_pair() {
    let c = cfg(Profile::Linear, Direction::Centred);
    let out = feather(&hp(16, 8), 16, 1, 2.0, &c).unwrap();
    assert_eq!(out[7] + out[8], 1.0);
    assert_eq!(out[7], 0.75);
}
