mod common;
use common::{bits, cfg, close, filters, img, ref_lanczos, run1};
use resample::{
    resample, resample_matrix, Border, Config, Filter, Footprint, Image, Layout, Matrix3,
    PixelCenter, TieRule,
};

fn sample_image() -> Vec<f32> {
    // 4 by 3, three channels, packed, dyadic values
    (0..36i32)
        .map(|i| (i * 7).rem_euclid(23) as f32 * 0.25 - 1.0)
        .collect()
}

#[test]
fn identity_reproduces_input_for_every_filter_packed() {
    let d = sample_image();
    for f in filters() {
        for center in [PixelCenter::Half, PixelCenter::Corner] {
            let c = Config {
                filter: f,
                center,
                ..Config::default()
            };
            let out = resample(&img(&d, 4, 3, 3), 4, 3, |x, y| (x, y), &c).unwrap();
            assert_eq!(bits(&out), bits(&d), "{f:?} {center:?}");
        }
    }
}

#[test]
fn identity_reproduces_input_for_every_filter_planar() {
    let d = sample_image();
    let im = Image {
        data: &d,
        width: 4,
        height: 3,
        channels: 3,
        layout: Layout::Planar,
    };
    for f in filters() {
        let out = resample(&im, 4, 3, |x, y| (x, y), &cfg(f)).unwrap();
        assert_eq!(bits(&out), bits(&d), "{f:?}");
    }
}

#[test]
fn identity_matrix_reproduces_input() {
    let d = sample_image();
    for f in filters() {
        let out = resample_matrix(&img(&d, 4, 3, 3), 4, 3, &Matrix3::IDENTITY, &cfg(f)).unwrap();
        assert_eq!(bits(&out), bits(&d), "{f:?}");
    }
}

#[test]
fn identity_for_every_border_rule() {
    let d = sample_image();
    for b in [
        Border::Zero,
        Border::Clamp,
        Border::Repeat,
        Border::Mirror,
        Border::MirrorNoEdge,
    ] {
        for f in filters() {
            let c = Config {
                filter: f,
                border: b,
                ..Config::default()
            };
            let out = resample(&img(&d, 4, 3, 3), 4, 3, |x, y| (x, y), &c).unwrap();
            assert_eq!(bits(&out), bits(&d), "{f:?} {b:?}");
        }
    }
}

#[test]
fn integer_translation_clamp() {
    let d = [1.0f32, 2.0, 3.0, 4.0];
    for f in filters() {
        let out = run1(&d, 4, 1, 4, 1, |x, y| (x + 1.0, y), &cfg(f));
        assert_eq!(out, vec![2.0, 3.0, 4.0, 4.0], "{f:?}");
    }
}

#[test]
fn integer_translation_zero_border() {
    let d = [1.0f32, 2.0, 3.0, 4.0];
    for f in filters() {
        let c = Config {
            filter: f,
            border: Border::Zero,
            ..Config::default()
        };
        let out = run1(&d, 4, 1, 4, 1, |x, y| (x + 1.0, y), &c);
        assert_eq!(out, vec![2.0, 3.0, 4.0, 0.0], "{f:?}");
    }
}

#[test]
fn integer_translation_negative_and_vertical() {
    let d = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2 wide, 3 high
    let c = Config {
        border: Border::Zero,
        ..cfg(Filter::Keys { a: -0.5 })
    };
    let out = run1(&d, 2, 3, 2, 3, |x, y| (x - 1.0, y + 1.0), &c);
    assert_eq!(out, vec![0.0, 3.0, 0.0, 5.0, 0.0, 0.0]);
}

#[test]
fn half_pixel_translation_bilinear_ramp() {
    let d = [0.0f32, 2.0, 4.0, 6.0];
    let out = run1(&d, 4, 1, 4, 1, |x, y| (x + 0.5, y), &cfg(Filter::Bilinear));
    assert_eq!(out, vec![1.0, 3.0, 5.0, 6.0]);
}

#[test]
fn half_pixel_translation_bilinear_ramp_corner_convention() {
    let d = [0.0f32, 2.0, 4.0, 6.0];
    let c = Config {
        center: PixelCenter::Corner,
        ..cfg(Filter::Bilinear)
    };
    let out = run1(&d, 4, 1, 4, 1, |x, y| (x + 0.5, y), &c);
    assert_eq!(out, vec![1.0, 3.0, 5.0, 6.0]);
}

#[test]
fn centre_conventions_differ_for_scaling() {
    // Map x -> x / 2 : with centre offset 0.5 the source index is X/2 - 0.25, with Corner X/2.
    let d = [0.0f32, 4.0];
    let half = run1(&d, 2, 1, 4, 1, |x, y| (x / 2.0, y), &cfg(Filter::Bilinear));
    assert_eq!(half, vec![0.0, 1.0, 3.0, 4.0]);
    let c = Config {
        center: PixelCenter::Corner,
        ..cfg(Filter::Bilinear)
    };
    let corner = run1(&d, 2, 1, 4, 1, |x, y| (x / 2.0, y), &c);
    assert_eq!(corner, vec![0.0, 2.0, 4.0, 4.0]);
}

#[test]
fn vertical_half_pixel_translation_uses_rows() {
    let d = [0.0f32, 10.0, 20.0]; // 1 wide, 3 high
    let out = run1(&d, 1, 3, 1, 3, |x, y| (x, y + 0.5), &cfg(Filter::Bilinear));
    assert_eq!(out, vec![5.0, 15.0, 20.0]);
}

#[test]
fn nearest_tie_rules_on_half_translation() {
    let d = [10.0f32, 20.0, 30.0, 40.0, 50.0];
    let map = |x: f64, y: f64| (x + 0.5, y);
    let mk = |tie| Config {
        tie,
        ..cfg(Filter::Nearest)
    };
    // source coordinate is X + 0.5, a tie between X and X + 1
    assert_eq!(
        run1(&d, 5, 1, 4, 1, map, &mk(TieRule::HalfUp)),
        vec![20.0, 30.0, 40.0, 50.0]
    );
    assert_eq!(
        run1(&d, 5, 1, 4, 1, map, &mk(TieRule::HalfDown)),
        vec![10.0, 20.0, 30.0, 40.0]
    );
    assert_eq!(
        run1(&d, 5, 1, 4, 1, map, &mk(TieRule::HalfEven)),
        vec![10.0, 30.0, 30.0, 50.0]
    );
}

#[test]
fn nearest_tie_rules_with_negative_coordinates() {
    let d = [10.0f32, 20.0, 30.0];
    let map = |x: f64, y: f64| (x - 1.5, y);
    let mk = |tie| Config {
        tie,
        ..cfg(Filter::Nearest)
    };
    // u = -1.5, -0.5, 0.5, 1.5 ; clamp border
    assert_eq!(
        run1(&d, 3, 1, 4, 1, map, &mk(TieRule::HalfUp)),
        vec![10.0, 10.0, 20.0, 30.0]
    );
    assert_eq!(
        run1(&d, 3, 1, 4, 1, map, &mk(TieRule::HalfDown)),
        vec![10.0, 10.0, 10.0, 20.0]
    );
    assert_eq!(
        run1(&d, 3, 1, 4, 1, map, &mk(TieRule::HalfEven)),
        vec![10.0, 10.0, 10.0, 30.0]
    );
}

#[test]
fn nearest_away_from_ties_ignores_rule() {
    let d = [10.0f32, 20.0, 30.0];
    let map = |x: f64, y: f64| (x + 0.3, y);
    for tie in [TieRule::HalfUp, TieRule::HalfDown, TieRule::HalfEven] {
        let c = Config {
            tie,
            ..cfg(Filter::Nearest)
        };
        assert_eq!(run1(&d, 3, 1, 3, 1, map, &c), vec![10.0, 20.0, 30.0]);
        let map2 = |x: f64, y: f64| (x + 0.7, y);
        assert_eq!(run1(&d, 3, 1, 3, 1, map2, &c), vec![20.0, 30.0, 30.0]);
    }
}

fn each_border_row(border: Border) -> Vec<f32> {
    let d = [1.0f32, 2.0, 3.0];
    let c = Config {
        border,
        ..cfg(Filter::Nearest)
    };
    run1(&d, 3, 1, 9, 1, |x, y| (x - 3.0, y), &c)
}

#[test]
fn border_zero_three_pixel_row() {
    assert_eq!(
        each_border_row(Border::Zero),
        vec![0., 0., 0., 1., 2., 3., 0., 0., 0.]
    );
}

#[test]
fn border_clamp_three_pixel_row() {
    assert_eq!(
        each_border_row(Border::Clamp),
        vec![1., 1., 1., 1., 2., 3., 3., 3., 3.]
    );
}

#[test]
fn border_repeat_three_pixel_row() {
    assert_eq!(
        each_border_row(Border::Repeat),
        vec![1., 2., 3., 1., 2., 3., 1., 2., 3.]
    );
}

#[test]
fn border_mirror_three_pixel_row() {
    assert_eq!(
        each_border_row(Border::Mirror),
        vec![3., 2., 1., 1., 2., 3., 3., 2., 1.]
    );
}

#[test]
fn border_mirror_no_edge_three_pixel_row() {
    assert_eq!(
        each_border_row(Border::MirrorNoEdge),
        vec![2., 3., 2., 1., 2., 3., 2., 1., 2.]
    );
}

#[test]
fn border_applies_to_vertical_axis() {
    let d = [1.0f32, 2.0, 3.0]; // 1 wide, 3 high
    let c = Config {
        border: Border::Mirror,
        ..cfg(Filter::Nearest)
    };
    let out = run1(&d, 1, 3, 1, 6, |x, y| (x, y - 3.0), &c);
    assert_eq!(out, vec![3., 2., 1., 1., 2., 3.]);
}

#[test]
fn bilinear_zero_border_edge_blends_with_zero() {
    let d = [1.0f32, 1.0];
    let c = Config {
        border: Border::Zero,
        ..cfg(Filter::Bilinear)
    };
    let out = run1(&d, 2, 1, 2, 1, |x, y| (x - 0.5, y), &c);
    // u = -0.5 : taps -1 and 0 with weight 0.5 each; u = 0.5 : both inside
    assert_eq!(out, vec![0.5, 1.0]);
}

#[test]
fn scale_by_two_nearest() {
    let d = [0.0f32, 4.0, 8.0, 12.0];
    let out = run1(
        &d,
        2,
        2,
        4,
        4,
        |x, y| (x / 2.0, y / 2.0),
        &cfg(Filter::Nearest),
    );
    let row0 = [0.0, 0.0, 4.0, 4.0];
    let row1 = [8.0, 8.0, 12.0, 12.0];
    let want: Vec<f32> = [row0, row0, row1, row1].concat();
    assert_eq!(out, want);
}

#[test]
fn scale_by_two_bilinear() {
    let d = [0.0f32, 4.0, 8.0, 12.0];
    let out = run1(
        &d,
        2,
        2,
        4,
        4,
        |x, y| (x / 2.0, y / 2.0),
        &cfg(Filter::Bilinear),
    );
    let want: Vec<f32> = [
        [0., 1., 3., 4.],
        [2., 3., 5., 6.],
        [6., 7., 9., 10.],
        [8., 9., 11., 12.],
    ]
    .concat();
    assert_eq!(out, want);
}

#[test]
fn scale_by_two_box_unit_matches_bilinear_here() {
    let d = [0.0f32, 4.0, 8.0, 12.0];
    let out = run1(&d, 2, 2, 4, 4, |x, y| (x / 2.0, y / 2.0), &cfg(Filter::Box));
    let want: Vec<f32> = [
        [0., 1., 3., 4.],
        [2., 3., 5., 6.],
        [6., 7., 9., 10.],
        [8., 9., 11., 12.],
    ]
    .concat();
    assert_eq!(out, want);
}

#[test]
fn scale_by_two_keys_half_by_hand() {
    // Row [0, 4], one row high, clamp. Weights at fraction 0.25: -0.0703125, 0.8671875,
    // 0.2265625, -0.0234375 ; at fraction 0.75 mirrored. Clamping folds the outer taps.
    let d = [0.0f32, 4.0];
    let out = run1(
        &d,
        2,
        1,
        4,
        1,
        |x, y| (x / 2.0, y),
        &cfg(Filter::Keys { a: -0.5 }),
    );
    assert_eq!(out, vec![-0.28125, 0.8125, 3.1875, 4.28125]);
}

#[test]
fn scale_by_two_mitchell_b_half_by_hand() {
    // weights at fraction 0.25 for (B, C) = (0.5, 0): 0.03515625, 0.727864583, 0.235677083,
    // 0.001302083 ; the right pixel gets the last two.
    let d = [0.0f32, 4.0];
    let out = run1(
        &d,
        2,
        1,
        4,
        1,
        |x, y| (x / 2.0, y),
        &cfg(Filter::Mitchell { b: 0.5, c: 0.0 }),
    );
    let p = 5.6875 / 6.0; // 4 * (1.4140625 + 0.0078125) / 6
    close(&out[1..3], &[p, 4.0 - p], 1e-6);
}

#[test]
fn scale_by_two_lanczos_two_against_reference() {
    let d = [0.0f32, 4.0];
    let out = run1(
        &d,
        2,
        1,
        4,
        1,
        |x, y| (x / 2.0, y),
        &cfg(Filter::Lanczos { lobes: 2 }),
    );
    // u = 0.25, taps -1..2 at distances 1.25, 0.25, -0.75, -1.75 ; clamp folds -1,0 and 1,2
    let w = |t: f64| ref_lanczos(t, 2.0);
    let right = 4.0 * (w(-0.75) + w(-1.75));
    let left = 4.0 * (w(1.25) + w(0.25)) * 0.0 + 0.0;
    assert_eq!(left, 0.0);
    assert!(
        (f64::from(out[1]) - right).abs() < 1e-5,
        "{} {right}",
        out[1]
    );
}

#[test]
fn box_minification_footprint_rules() {
    let d = [0.0f32, 0.0, 8.0, 0.0, 0.0, 0.0, 16.0, 0.0];
    let map = |x: f64, y: f64| (4.0 * x, y);
    let mk = |footprint| Config {
        footprint,
        ..cfg(Filter::Box)
    };
    // output pixel 0 maps to source centre 2 (index 1.5), pixel 1 to 6 (index 5.5)
    assert_eq!(
        run1(&d, 8, 1, 2, 1, map, &mk(Footprint::Unit)),
        vec![4.0, 8.0]
    );
    assert_eq!(
        run1(&d, 8, 1, 2, 1, map, &mk(Footprint::Jacobian)),
        vec![2.0, 4.0]
    );
    assert_eq!(
        run1(
            &d,
            8,
            1,
            2,
            1,
            map,
            &mk(Footprint::Fixed { w: 4.0, h: 1.0 })
        ),
        vec![2.0, 4.0]
    );
    assert_eq!(
        run1(
            &d,
            8,
            1,
            2,
            1,
            map,
            &mk(Footprint::Fixed { w: 2.0, h: 1.0 })
        ),
        vec![4.0, 8.0]
    );
}

#[test]
fn box_jacobian_floor_applies_in_magnification() {
    // magnifying by 2: extent 0.5, floor 1 gives a unit box, floor 0.25 keeps 0.5
    let d = [0.0f32, 8.0];
    let map = |x: f64, y: f64| (x / 2.0, y);
    let unit = Config {
        footprint: Footprint::Jacobian,
        ..cfg(Filter::Box)
    };
    let narrow = Config {
        min_extent: 0.25,
        ..unit
    };
    let a = run1(&d, 2, 1, 4, 1, map, &unit);
    assert_eq!(a, vec![0.0, 2.0, 6.0, 8.0]);
    // narrow box of width 0.5 at u = 0.25 covers [0, 0.5]: pixel 0 only
    let b = run1(&d, 2, 1, 4, 1, map, &narrow);
    assert_eq!(b, vec![0.0, 0.0, 8.0, 8.0]);
}

#[test]
fn box_mean_of_constant_is_constant() {
    let d = vec![0.5f32; 36];
    let c = Config {
        footprint: Footprint::Jacobian,
        ..cfg(Filter::Box)
    };
    let out = resample(&img(&d, 6, 6, 1), 2, 2, |x, y| (3.0 * x, 3.0 * y), &c).unwrap();
    assert_eq!(out, vec![0.5; 4]);
}

#[test]
fn box_two_dimensional_mean() {
    // 4 by 4 with a single bright pixel, 2 by 2 output: each output pixel is a 2 by 2 mean
    let mut d = vec![0.0f32; 16];
    d[5] = 16.0; // x 1, y 1
    let c = Config {
        footprint: Footprint::Jacobian,
        ..cfg(Filter::Box)
    };
    let out = resample(&img(&d, 4, 4, 1), 2, 2, |x, y| (2.0 * x, 2.0 * y), &c).unwrap();
    assert_eq!(out, vec![4.0, 0.0, 0.0, 0.0]);
}

#[test]
fn box_rotation_footprint_is_the_bounding_box() {
    // a 90 degree swap has extent 1 on each axis
    let d: Vec<f32> = (0..9i32).map(|i| i as f32).collect();
    let c = Config {
        center: PixelCenter::Corner,
        footprint: Footprint::Jacobian,
        ..cfg(Filter::Box)
    };
    let m = Matrix3([[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
    let out = resample_matrix(&img(&d, 3, 3, 1), 3, 3, &m, &c).unwrap();
    let want: Vec<f32> = vec![0., 3., 6., 1., 4., 7., 2., 5., 8.];
    assert_eq!(out, want);
}

#[test]
fn transpose_with_nearest_non_square() {
    let d = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0]; // 3 wide, 2 high
    let c = Config {
        center: PixelCenter::Corner,
        ..cfg(Filter::Nearest)
    };
    let m = Matrix3([[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
    let out = resample_matrix(&img(&d, 3, 2, 1), 2, 3, &m, &c).unwrap();
    assert_eq!(out, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
}

#[test]
fn matrix_apply_values() {
    assert_eq!(Matrix3::IDENTITY.apply(3.0, -2.0), (3.0, -2.0));
    assert_eq!(Matrix3::translation(1.5, -2.0).apply(1.0, 1.0), (2.5, -1.0));
    assert_eq!(Matrix3::scale(2.0, 0.5).apply(3.0, 4.0), (6.0, 2.0));
    let p = Matrix3([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 2.0]]);
    assert_eq!(p.apply(4.0, 6.0), (2.0, 3.0));
    let q = Matrix3([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 1.0]]);
    assert_eq!(q.apply(3.0, 8.0), (0.75, 2.0));
    let s = Matrix3([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [0.0, 0.0, 1.0]]);
    assert_eq!(s.apply(1.0, 1.0), (6.0, 15.0));
}

#[test]
fn matrix_zero_w_gives_zero_pixels() {
    let m = Matrix3([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]]);
    let (x, y) = m.apply(1.0, 1.0);
    assert!(x.is_nan() && y.is_nan());
    let d = [5.0f32, 6.0];
    let out = resample_matrix(&img(&d, 2, 1, 1), 2, 1, &m, &cfg(Filter::Bilinear)).unwrap();
    assert_eq!(out, vec![0.0, 0.0]);
}

#[test]
fn matrix_and_closure_agree() {
    let d = sample_image();
    let m = Matrix3([[0.9, 0.1, 0.3], [-0.1, 1.1, -0.2], [0.0, 0.0, 1.0]]);
    for f in filters() {
        let a = resample_matrix(&img(&d, 4, 3, 3), 5, 4, &m, &cfg(f)).unwrap();
        let b = resample(&img(&d, 4, 3, 3), 5, 4, |x, y| m.apply(x, y), &cfg(f)).unwrap();
        assert_eq!(bits(&a), bits(&b));
    }
}

#[test]
fn non_finite_and_huge_coordinates_give_zero_pixels() {
    let d = [5.0f32, 6.0];
    let c = cfg(Filter::Nearest);
    assert_eq!(run1(&d, 2, 1, 1, 1, |_, y| (f64::NAN, y), &c), vec![0.0]);
    assert_eq!(
        run1(&d, 2, 1, 1, 1, |_, y| (f64::INFINITY, y), &c),
        vec![0.0]
    );
    assert_eq!(run1(&d, 2, 1, 1, 1, |x, _| (x, 1.0e12), &c), vec![0.0]);
    assert_eq!(run1(&d, 2, 1, 1, 1, |x, _| (-1.0e12, x), &c), vec![0.0]);
    assert_eq!(
        run1(&d, 2, 1, 1, 1, |x, y| (x - 0.5, y - 0.5), &c),
        vec![5.0]
    );
}

#[test]
fn non_finite_jacobian_gives_zero_pixel() {
    let d = [5.0f32, 6.0];
    let c = Config {
        footprint: Footprint::Jacobian,
        ..cfg(Filter::Box)
    };
    let map = |x: f64, y: f64| if x > 1.0 { (1.0e15, y) } else { (x, y) };
    let out = run1(&d, 2, 1, 1, 1, map, &c);
    assert_eq!(out, vec![0.0]);
}

#[test]
fn zero_sized_output_is_empty() {
    let d = [1.0f32];
    assert!(run1(&d, 1, 1, 0, 3, |x, y| (x, y), &cfg(Filter::Bilinear)).is_empty());
    assert!(run1(&d, 1, 1, 3, 0, |x, y| (x, y), &cfg(Filter::Bilinear)).is_empty());
}

#[test]
fn output_size_can_differ_from_input() {
    let d = sample_image();
    let out = resample(
        &img(&d, 4, 3, 3),
        7,
        2,
        |x, y| (x, y),
        &cfg(Filter::Bilinear),
    )
    .unwrap();
    assert_eq!(out.len(), 7 * 2 * 3);
}

#[test]
fn planar_and_packed_agree_for_every_filter() {
    let packed = sample_image();
    let mut planar = vec![0.0f32; 36];
    for p in 0..12 {
        for c in 0..3 {
            planar[c * 12 + p] = packed[p * 3 + c];
        }
    }
    let map = |x: f64, y: f64| (0.8 * x + 0.3, 1.2 * y - 0.4);
    for f in filters() {
        let a = resample(&img(&packed, 4, 3, 3), 5, 4, map, &cfg(f)).unwrap();
        let pi = Image {
            data: &planar,
            width: 4,
            height: 3,
            channels: 3,
            layout: Layout::Planar,
        };
        let b = resample(&pi, 5, 4, map, &cfg(f)).unwrap();
        for p in 0..20 {
            for c in 0..3 {
                assert_eq!(
                    a[p * 3 + c].to_bits(),
                    b[c * 20 + p].to_bits(),
                    "{f:?} {p} {c}"
                );
            }
        }
    }
}

#[test]
fn channels_are_independent() {
    let d = sample_image();
    let both = resample(
        &img(&d, 4, 3, 3),
        5,
        4,
        |x, y| (x * 0.7, y * 0.9),
        &cfg(Filter::Keys { a: -0.5 }),
    )
    .unwrap();
    let first: Vec<f32> = d.iter().step_by(3).copied().collect();
    let one = run1(
        &first,
        4,
        3,
        5,
        4,
        |x, y| (x * 0.7, y * 0.9),
        &cfg(Filter::Keys { a: -0.5 }),
    );
    for (i, v) in one.iter().enumerate() {
        assert_eq!(both[i * 3].to_bits(), v.to_bits());
    }
}

#[test]
fn scale_by_two_keys_minus_one_by_hand() {
    // a = -1, fraction 0.25 weights: -0.140625 (t 1.25), 0.890625, 0.296875, -0.046875 (t 1.75)
    let d = [0.0f32, 4.0];
    let out = run1(
        &d,
        2,
        1,
        4,
        1,
        |x, y| (x / 2.0, y),
        &cfg(Filter::Keys { a: -1.0 }),
    );
    assert_eq!(out, vec![-0.5625, 1.0, 3.0, 4.5625]);
}

#[test]
fn keys_overshoot_is_not_clamped() {
    // u = 1.25 on [0, 1, 1, 1]: weights -0.0703125, 0.8671875, 0.2265625, -0.0234375
    let up = run1(
        &[0.0f32, 1.0, 1.0, 1.0],
        4,
        1,
        1,
        1,
        |_, y| (1.75, y),
        &cfg(Filter::Keys { a: -0.5 }),
    );
    assert_eq!(up, vec![1.0703125]);
    let down = run1(
        &[1.0f32, 0.0, 0.0, 0.0],
        4,
        1,
        1,
        1,
        |_, y| (1.75, y),
        &cfg(Filter::Keys { a: -0.5 }),
    );
    assert_eq!(down, vec![-0.0703125]);
}

#[test]
fn box_jacobian_min_extent_widens_the_footprint() {
    let d = [0.0f32, 0.0, 8.0, 0.0, 0.0, 0.0, 16.0, 0.0];
    let map = |x: f64, y: f64| (4.0 * x, y);
    let c = |m| Config {
        footprint: Footprint::Jacobian,
        min_extent: m,
        ..cfg(Filter::Box)
    };
    // extent is 4: a floor of 1 changes nothing, a floor of 8 gives box [-2.5, 5.5] around 1.5
    // with weights 1/8 per unit: pixel 2 holds 8 over one unit -> 1.0
    assert_eq!(run1(&d, 8, 1, 2, 1, map, &c(1.0)), vec![2.0, 4.0]);
    assert_eq!(run1(&d, 8, 1, 2, 1, map, &c(8.0))[0], 1.0);
}

#[test]
fn box_jacobian_footprint_includes_cross_terms() {
    // Source = (2Y, 2X) with Corner centre: the mapped unit square has extent 2 on both axes
    // although neither axis step is along x. One bright pixel at (2, 2) is spread over a
    // 2 by 2 footprint: weights 0.25, 0.5, 0.25 per axis give 36 * 0.5 * 0.5 = 9.
    let mut d = vec![0.0f32; 36];
    d[2 * 6 + 2] = 36.0;
    let c = Config {
        center: PixelCenter::Corner,
        border: Border::Zero,
        footprint: Footprint::Jacobian,
        ..cfg(Filter::Box)
    };
    let m = Matrix3([[0.0, 2.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
    let out = resample_matrix(&img(&d, 6, 6, 1), 3, 3, &m, &c).unwrap();
    let mut want = vec![0.0f32; 9];
    want[4] = 9.0;
    assert_eq!(out, want);
}

#[test]
fn box_jacobian_extent_follows_each_axis_separately() {
    // Scale 4 along x and 1 along y: a wide, short footprint. Bright pixel at (1, 0) in an
    // 8 by 1 row: output pixel 0 maps to index 1.5 with footprint [-0.5, 3.5] -> 1/4 each.
    let mut d = vec![0.0f32; 8];
    d[1] = 8.0;
    let c = Config {
        footprint: Footprint::Jacobian,
        ..cfg(Filter::Box)
    };
    let out = run1(&d, 8, 1, 2, 1, |x, y| (4.0 * x, y), &c);
    assert_eq!(out, vec![2.0, 0.0]);
    // transposed: 1 by 8 column, same expectation along y
    let out = run1(&d, 1, 8, 1, 2, |x, y| (x, 4.0 * y), &c);
    assert_eq!(out, vec![2.0, 0.0]);
}
