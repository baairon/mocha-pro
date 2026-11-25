use soft_edge::*;

fn brute(feature: &[bool], w: usize, h: usize) -> Vec<u64> {
    let mut out = vec![NO_FEATURE; w * h];
    for y in 0..h {
        for x in 0..w {
            for fy in 0..h {
                for fx in 0..w {
                    if feature[fy * w + fx] {
                        let dx = x as i64 - fx as i64;
                        let dy = y as i64 - fy as i64;
                        let d = (dx * dx + dy * dy) as u64;
                        if d < out[y * w + x] {
                            out[y * w + x] = d;
                        }
                    }
                }
            }
        }
    }
    out
}

fn lcg(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed >> 33
}

#[test]
fn single_point_hand_values() {
    let mut f = vec![false; 25];
    f[2 * 5 + 2] = true;
    let d = squared_distance_transform(&f, 5, 5);
    assert_eq!(d[12], 0);
    assert_eq!(d[13], 1);
    assert_eq!(d[14], 4);
    assert_eq!(d[0], 8);
    assert_eq!(d[1], 5);
    assert_eq!(d[6], 2);
}

#[test]
fn single_point_equals_brute() {
    for (px, py) in [(0, 0), (3, 1), (6, 4), (2, 2)] {
        let mut f = vec![false; 7 * 5];
        f[py * 7 + px] = true;
        assert_eq!(squared_distance_transform(&f, 7, 5), brute(&f, 7, 5));
    }
}

#[test]
fn square_block_equals_brute() {
    let (w, h) = (12, 10);
    let mut f = vec![false; w * h];
    for y in 3..7 {
        for x in 4..9 {
            f[y * w + x] = true;
        }
    }
    let d = squared_distance_transform(&f, w, h);
    assert_eq!(d, brute(&f, w, h));
    assert_eq!(d[5 * w + 6], 0);
    assert_eq!(d[0], 3 * 3 + 4 * 4);
}

#[test]
fn square_hole_inside_ones() {
    // feature = outside of a 5x5 block inside 9x9: centre is 3 away from the nearest outside pixel
    let (w, h) = (9, 9);
    let f: Vec<bool> = (0..w * h)
        .map(|i| !((2..7).contains(&(i % w)) && (2..7).contains(&(i / w))))
        .collect();
    let d = squared_distance_transform(&f, w, h);
    assert_eq!(d[4 * w + 4], 9);
    assert_eq!(d, brute(&f, w, h));
}

#[test]
fn no_feature_everywhere() {
    let d = squared_distance_transform(&[false; 12], 4, 3);
    assert!(d.iter().all(|&v| v == NO_FEATURE));
}

#[test]
fn all_feature_zero() {
    assert!(squared_distance_transform(&[true; 12], 4, 3)
        .iter()
        .all(|&v| v == 0));
}

#[test]
fn empty_ok() {
    assert!(squared_distance_transform(&[], 0, 5).is_empty());
}

#[test]
fn one_dimensional_rows_and_columns() {
    let f = [false, false, true, false, false, false, true];
    assert_eq!(
        squared_distance_transform(&f, 7, 1),
        vec![4, 1, 0, 1, 4, 1, 0]
    );
    assert_eq!(
        squared_distance_transform(&f, 1, 7),
        vec![4, 1, 0, 1, 4, 1, 0]
    );
}

#[test]
fn random_equals_brute() {
    let mut seed = 12345u64;
    for round in 0..40 {
        let w = 1 + (lcg(&mut seed) % 14) as usize;
        let h = 1 + (lcg(&mut seed) % 14) as usize;
        let density = 1 + (round % 5) as u64;
        let f: Vec<bool> = (0..w * h).map(|_| lcg(&mut seed) % 10 < density).collect();
        assert_eq!(
            squared_distance_transform(&f, w, h),
            brute(&f, w, h),
            "round {round}"
        );
    }
}

#[test]
fn sparse_far_points_equal_brute() {
    let (w, h) = (40, 3);
    let mut f = vec![false; w * h];
    f[0] = true;
    f[w * h - 1] = true;
    assert_eq!(squared_distance_transform(&f, w, h), brute(&f, w, h));
}

#[test]
fn diagonal_line_equals_brute() {
    let n = 16;
    let mut f = vec![false; n * n];
    for i in 0..n {
        f[i * n + i] = true;
    }
    assert_eq!(squared_distance_transform(&f, n, n), brute(&f, n, n));
}

#[test]
#[should_panic]
fn length_mismatch_panics() {
    squared_distance_transform(&[true; 3], 2, 2);
}

fn half_plane(w: usize, h: usize, edge: usize) -> Vec<f32> {
    (0..w * h)
        .map(|i| if i % w < edge { 1.0 } else { 0.0 })
        .collect()
}

#[test]
fn signed_distance_half_plane_hand() {
    // inside x < 4 of 8, one row, clamp: inside pixel x=3 is 1 from outside -> 0.5
    let sd = signed_distance(&half_plane(8, 1, 4), 8, 1, &Config::default()).unwrap();
    assert_eq!(sd, vec![3.5, 2.5, 1.5, 0.5, -0.5, -1.5, -2.5, -3.5]);
}

#[test]
fn signed_distance_edge_offset_zero() {
    let cfg = Config {
        edge_offset: 0.0,
        ..Config::default()
    };
    let sd = signed_distance(&half_plane(6, 1, 3), 6, 1, &cfg).unwrap();
    assert_eq!(sd, vec![3.0, 2.0, 1.0, -1.0, -2.0, -3.0]);
}

#[test]
fn signed_distance_diagonal_uses_sqrt() {
    // single inside pixel in the middle of 5x5, others outside
    let mut m = vec![0.0f32; 25];
    m[12] = 1.0;
    let sd = signed_distance(&m, 5, 5, &Config::default()).unwrap();
    assert_eq!(sd[12], 0.5);
    assert_eq!(sd[0], -((8.0f64.sqrt() as f32) - 0.5));
    assert_eq!(sd[13], -0.5);
}

#[test]
fn signed_distance_zero_border_adds_ring() {
    let cfg = Config {
        border: Border::Zero,
        ..Config::default()
    };
    let sd = signed_distance(&[1.0; 5], 5, 1, &cfg).unwrap();
    // distance to the ring above/below is 1 for every pixel of a 1 row image
    assert_eq!(sd, vec![0.5; 5]);
    let sd3 = signed_distance(&[1.0; 25], 5, 5, &cfg).unwrap();
    assert_eq!(sd3[12], 2.5);
    assert_eq!(sd3[0], 0.5);
}

#[test]
fn signed_distance_uniform_is_infinite() {
    let cfg = Config::default();
    assert!(signed_distance(&[1.0; 6], 3, 2, &cfg)
        .unwrap()
        .iter()
        .all(|&v| v == f32::INFINITY));
    assert!(signed_distance(&[0.0; 6], 3, 2, &cfg)
        .unwrap()
        .iter()
        .all(|&v| v == f32::NEG_INFINITY));
}

#[test]
fn signed_distance_threshold_field() {
    let m = [0.2f32, 0.4, 0.6, 0.8];
    let lo = Config {
        threshold: 0.3,
        ..Config::default()
    };
    let hi = Config {
        threshold: 0.7,
        ..Config::default()
    };
    assert_eq!(
        signed_distance(&m, 4, 1, &lo).unwrap(),
        vec![-0.5, 0.5, 1.5, 2.5]
    );
    assert_eq!(
        signed_distance(&m, 4, 1, &hi).unwrap(),
        vec![-2.5, -1.5, -0.5, 0.5]
    );
}

#[test]
fn signed_distance_threshold_is_inclusive() {
    let sd = signed_distance(&[0.5, 0.0], 2, 1, &Config::default()).unwrap();
    assert_eq!(sd, vec![0.5, -0.5]);
}

#[test]
fn signed_distance_nan_is_outside() {
    let sd = signed_distance(&[f32::NAN, 1.0], 2, 1, &Config::default()).unwrap();
    assert_eq!(sd, vec![-0.5, 0.5]);
}

#[test]
fn clamp_mirror_agree_for_distance() {
    let mut seed = 9u64;
    let m: Vec<f32> = (0..9 * 7).map(|_| (lcg(&mut seed) % 2) as f32).collect();
    let r = |b| {
        signed_distance(
            &m,
            9,
            7,
            &Config {
                border: b,
                ..Config::default()
            },
        )
        .unwrap()
    };
    assert_eq!(r(Border::Clamp), r(Border::Mirror));
    assert_eq!(r(Border::Clamp), r(Border::MirrorNoRepeat));
}

#[test]
fn signed_distance_matches_brute_random() {
    let mut seed = 77u64;
    let (w, h) = (11, 8);
    let m: Vec<f32> = (0..w * h)
        .map(|_| u8::from(lcg(&mut seed).is_multiple_of(3)) as f32)
        .collect();
    let cfg = Config::default();
    let sd = signed_distance(&m, w, h, &cfg).unwrap();
    let ins: Vec<bool> = m.iter().map(|&v| v >= 0.5).collect();
    let outs: Vec<bool> = ins.iter().map(|&b| !b).collect();
    let (bi, bo) = (brute(&ins, w, h), brute(&outs, w, h));
    for i in 0..w * h {
        let want = if ins[i] {
            if bo[i] == NO_FEATURE {
                f32::INFINITY
            } else {
                (bo[i] as f64).sqrt() as f32 - 0.5
            }
        } else if bi[i] == NO_FEATURE {
            f32::NEG_INFINITY
        } else {
            -((bi[i] as f64).sqrt() as f32 - 0.5)
        };
        assert_eq!(sd[i].to_bits(), want.to_bits());
    }
}

#[test]
fn distance_deterministic() {
    let m = half_plane(10, 4, 5);
    let a = signed_distance(&m, 10, 4, &Config::default()).unwrap();
    assert_eq!(a, signed_distance(&m, 10, 4, &Config::default()).unwrap());
}
