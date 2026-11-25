//! X-spline tests against the properties stated by Blanc and Schlick (SIGGRAPH 1995).

use contour_eval::{
    eval, f_blend, g_blend, h_blend, xspline_weights, Config, Contour, ContourError, Kind, OpenEnd,
    Point, WeightMap, XVariant,
};

const P: [(f64, f64); 4] = [(1.0, 2.0), (5.0, 10.0), (13.0, 6.0), (17.0, 0.0)];

fn four(variant: XVariant, s1: f64, s2: f64) -> Contour {
    Contour {
        kind: Kind::XSpline { variant },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(P[0].0, P[0].1)
            },
            Point {
                s: s1,
                ..Point::new(P[1].0, P[1].1)
            },
            Point {
                s: s2,
                ..Point::new(P[2].0, P[2].1)
            },
            Point {
                s: 0.0,
                ..Point::new(P[3].0, P[3].1)
            },
        ],
    }
}

fn mid(s1: f64, s2: f64, t: f64) -> (f64, f64) {
    eval(&four(XVariant::General, s1, s2), 1, t, &Config::canonical()).unwrap()
}

fn close(a: (f64, f64), b: (f64, f64), tol: f64) -> bool {
    (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol
}

fn lcg(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 11) as f64) / ((1u64 << 53) as f64)
}

#[test]
fn f_blend_reaches_zero_and_one() {
    for &den in &[0.5, 1.0, 1.5, 2.0] {
        assert_eq!(f_blend(0.0, den), 0.0);
        assert_eq!(f_blend(den, den), 1.0);
    }
}

#[test]
fn f_blend_known_value() {
    // den = 2 gives p = 8; u = 0.25: u^3 (2 + u - 2 u^2) = (1/64) * 2.125
    assert_eq!(f_blend(0.5, 2.0).to_bits(), (2.125f64 / 64.0).to_bits());
}

#[test]
fn g_blend_reaches_zero_and_one() {
    for &q in &[0.0, 0.25, 0.5, 1.0] {
        assert_eq!(g_blend(0.0, q), 0.0);
        assert_eq!(g_blend(1.0, q), 1.0);
    }
}

#[test]
fn h_blend_vanishes_at_both_ends() {
    for &q in &[0.0, 0.25, 0.5, 1.0] {
        assert_eq!(h_blend(0.0, q), 0.0);
        assert_eq!(h_blend(1.0, q), 0.0);
    }
}

#[test]
fn g_and_h_known_values_at_half() {
    // q = 1, u = 0.5: G = 0.5 + 0.5 - 0.5 + 3 * 0.0625 - 0.0 ... evaluated by hand below.
    // G = q u + 2q u^2 + (8-12q) u^3 + (14q-11) u^4 + (4-5q) u^5
    //   = 0.5 + 0.5 - 4 * 0.125 + 3 * 0.0625 - 1 * 0.03125 = 0.65625
    assert_eq!(g_blend(0.5, 1.0), 0.65625);
    // H = q u + 2q u^2 - 2q u^4 - q u^5 = 0.5 + 0.5 - 0.125 - 0.03125 = 0.84375
    assert_eq!(h_blend(0.5, 1.0), 0.84375);
}

#[test]
fn weights_at_segment_start_for_shape_one() {
    let w = xspline_weights(1.0, 1.0, 0.0);
    assert_eq!(w, [0.25, 1.0, 0.25, 0.0]);
}

#[test]
fn weights_at_segment_end_for_shape_one() {
    let w = xspline_weights(1.0, 1.0, 1.0);
    assert_eq!(w, [0.0, 0.25, 1.0, 0.25]);
}

#[test]
fn weights_for_shape_zero_are_two_point_blend() {
    assert_eq!(xspline_weights(0.0, 0.0, 0.0), [0.0, 1.0, 0.0, 0.0]);
    assert_eq!(xspline_weights(0.0, 0.0, 1.0), [0.0, 0.0, 1.0, 0.0]);
    let w = xspline_weights(0.0, 0.0, 0.5);
    assert_eq!(w, [0.0, 0.4375, 0.4375, 0.0]);
}

#[test]
fn shape_one_equals_uniform_cubic_bspline_at_segment_ends() {
    // Paper: basic X-splines are nearly identical to uniform cubic B-splines; at the segment
    // ends the point is exactly (P0 + 4 P1 + P2) / 6 and (P1 + 4 P2 + P3) / 6.
    let a = mid(1.0, 1.0, 0.0);
    let want_a = (
        (P[0].0 + 4.0 * P[1].0 + P[2].0) / 6.0,
        (P[0].1 + 4.0 * P[1].1 + P[2].1) / 6.0,
    );
    assert!(close(a, want_a, 1e-14));
    let b = mid(1.0, 1.0, 1.0);
    let want_b = (
        (P[1].0 + 4.0 * P[2].0 + P[3].0) / 6.0,
        (P[1].1 + 4.0 * P[2].1 + P[3].1) / 6.0,
    );
    assert!(close(b, want_b, 1e-14));
}

#[test]
fn shape_one_is_near_the_bspline_inside_the_segment() {
    let w = xspline_weights(1.0, 1.0, 0.5);
    let sum: f64 = w.iter().sum();
    let bs = [1.0 / 48.0, 23.0 / 48.0, 23.0 / 48.0, 1.0 / 48.0];
    for i in 0..4 {
        assert!((w[i] / sum - bs[i]).abs() < 0.005, "weight {i}");
    }
}

#[test]
fn shape_zero_passes_through_the_points_and_midpoint() {
    assert_eq!(mid(0.0, 0.0, 0.0), P[1]);
    assert_eq!(mid(0.0, 0.0, 1.0), P[2]);
    assert_eq!(mid(0.0, 0.0, 0.5), (9.0, 8.0));
}

#[test]
fn negative_shape_interpolates_exactly() {
    for &s in &[-1.0, -0.5, -0.25, 0.0] {
        assert_eq!(mid(s, 0.5, 0.0), P[1], "s1 = {s}");
        assert_eq!(mid(0.5, s, 1.0), P[2], "s2 = {s}");
    }
}

#[test]
fn negative_shape_interpolates_with_non_dyadic_values() {
    for &s in &[-0.9, -0.3, -0.07] {
        assert!(close(mid(s, 0.7, 0.0), P[1], 1e-14));
        assert!(close(mid(0.7, s, 1.0), P[2], 1e-14));
    }
}

#[test]
fn positive_shape_does_not_interpolate() {
    let r = mid(0.5, 0.5, 0.0);
    assert!(!close(r, P[1], 1e-3));
}

#[test]
fn shape_parameter_pulls_curve_toward_polygon() {
    // Larger s on both ends moves the segment middle further from the straight blend
    // (P1 + P2) / 2 toward the outer points in a fixed direction on this polygon.
    let a = mid(0.0, 0.0, 0.5);
    let b = mid(1.0, 1.0, 0.5);
    assert_ne!(a, b);
    assert!(close(a, (9.0, 8.0), 0.0));
}

#[test]
fn normalised_weights_sum_to_one() {
    let mut st = 7u64;
    for _ in 0..200 {
        let s1 = lcg(&mut st) * 2.0 - 1.0;
        let s2 = lcg(&mut st) * 2.0 - 1.0;
        let t = lcg(&mut st);
        let w = xspline_weights(s1, s2, t);
        let sum: f64 = w.iter().sum();
        let n: f64 = w.iter().map(|x| x / sum).sum();
        assert!((n - 1.0).abs() < 1e-14, "s=({s1},{s2}) t={t}");
    }
}

#[test]
fn basic_variant_weights_are_non_negative() {
    for &s1 in &[0.0, 0.25, 0.5, 1.0] {
        for &s2 in &[0.0, 0.25, 0.5, 1.0] {
            for k in 0..=20 {
                let w = xspline_weights(s1, s2, f64::from(k) / 20.0);
                for x in w {
                    assert!(x >= 0.0, "s=({s1},{s2}) k={k} w={x}");
                }
            }
        }
    }
}

#[test]
fn general_variant_has_negative_weights() {
    // The paper notes that negative shape parameters make the functions partly negative.
    let mut found = false;
    for k in 0..=100 {
        let w = xspline_weights(-1.0, -1.0, f64::from(k) / 100.0);
        if w.iter().any(|&x| x < 0.0) {
            found = true;
        }
    }
    assert!(found);
}

fn symmetric(s: f64) -> Contour {
    Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(-6.0, 0.0)
            },
            Point {
                s,
                ..Point::new(-2.0, 4.0)
            },
            Point {
                s,
                ..Point::new(2.0, 4.0)
            },
            Point {
                s: 0.0,
                ..Point::new(6.0, 0.0)
            },
        ],
    }
}

#[test]
fn symmetric_polygon_gives_symmetric_curve() {
    for &s in &[-1.0, -0.4, 0.0, 0.3, 1.0] {
        let c = symmetric(s);
        let g = Config::canonical();
        for k in 0..=10 {
            let t = f64::from(k) / 10.0;
            let a = eval(&c, 1, t, &g).unwrap();
            let b = eval(&c, 1, 1.0 - t, &g).unwrap();
            assert!((a.0 + b.0).abs() < 1e-13, "s={s} t={t}");
            assert!((a.1 - b.1).abs() < 1e-13, "s={s} t={t}");
        }
    }
}

#[test]
fn scaling_by_two_is_exact() {
    let a = four(XVariant::General, 0.3, -0.6);
    let mut b = a.clone();
    for p in &mut b.points {
        p.x *= 2.0;
        p.y *= 2.0;
    }
    let g = Config::canonical();
    for k in 0..=8 {
        let t = f64::from(k) / 8.0;
        let u = eval(&a, 1, t, &g).unwrap();
        let v = eval(&b, 1, t, &g).unwrap();
        assert_eq!(v.0.to_bits(), (u.0 * 2.0).to_bits());
        assert_eq!(v.1.to_bits(), (u.1 * 2.0).to_bits());
    }
}

#[test]
fn dyadic_translation_is_affine_invariant() {
    let a = four(XVariant::General, 0.5, 1.0);
    let mut b = a.clone();
    for p in &mut b.points {
        p.x += 32.0;
        p.y -= 16.0;
    }
    let g = Config::canonical();
    for k in 0..=8 {
        let t = f64::from(k) / 8.0;
        let u = eval(&a, 1, t, &g).unwrap();
        let v = eval(&b, 1, t, &g).unwrap();
        assert!(close(v, (u.0 + 32.0, u.1 - 16.0), 1e-13));
    }
}

#[test]
fn adjacent_spans_meet() {
    let pts: Vec<Point> = [
        (0.0, 0.0, 0.0),
        (3.0, 5.0, 0.6),
        (7.0, 4.0, -0.5),
        (9.0, 9.0, 1.0),
        (14.0, 6.0, 0.0),
        (15.0, 1.0, -1.0),
    ]
    .iter()
    .map(|&(x, y, s)| Point {
        s,
        ..Point::new(x, y)
    })
    .collect();
    let g = Config::canonical();
    for closed in [false, true] {
        let c = Contour {
            kind: Kind::XSpline {
                variant: XVariant::General,
            },
            closed,
            points: pts.clone(),
        };
        let n = c.span_count(&g).unwrap();
        for i in 0..n {
            let end = eval(&c, i, 1.0, &g).unwrap();
            let start = eval(&c, (i + 1) % n, 0.0, &g).unwrap();
            if closed || i + 1 < n {
                assert!(
                    close(end, start, 1e-12),
                    "closed={closed} span {i}: {end:?} {start:?}"
                );
            }
        }
    }
}

#[test]
fn local_control_sharp_end_ignores_outer_neighbour() {
    // s1 = 0 means P0 has no weight; moving P0 leaves the segment unchanged.
    let a = four(XVariant::General, 0.0, 0.5);
    let mut b = a.clone();
    b.points[0].x = -100.0;
    let g = Config::canonical();
    for k in 0..=8 {
        let t = f64::from(k) / 8.0;
        assert_eq!(eval(&a, 1, t, &g).unwrap(), eval(&b, 1, t, &g).unwrap());
    }
}

#[test]
fn local_control_moving_far_point_changes_positive_shape_curve() {
    let a = four(XVariant::General, 1.0, 1.0);
    let mut b = a.clone();
    b.points[0].x = -100.0;
    let g = Config::canonical();
    assert_ne!(eval(&a, 1, 0.5, &g).unwrap(), eval(&b, 1, 0.5, &g).unwrap());
}

#[test]
fn weight_map_affine_gives_sharp_point() {
    let mut g = Config::canonical();
    g.weight_map = WeightMap { a: 2.0, b: -1.0 };
    let c = four(XVariant::General, 0.5, 0.5); // maps to s = 0
    assert_eq!(eval(&c, 1, 0.0, &g).unwrap(), P[1]);
    assert_eq!(eval(&c, 1, 0.5, &g).unwrap(), (9.0, 8.0));
}

#[test]
fn weight_map_enters_range_check() {
    let mut g = Config::canonical();
    g.weight_map = WeightMap { a: 0.01, b: 0.0 };
    let c = four(XVariant::General, 50.0, 0.0); // maps to 0.5
    assert!(eval(&c, 1, 0.25, &g).is_ok());
    let c = four(XVariant::General, 150.0, 0.0); // maps to 1.5
    assert!(matches!(
        eval(&c, 1, 0.25, &g),
        Err(ContourError::ShapeOutOfRange { index: 1, .. })
    ));
}

#[test]
fn basic_variant_refuses_negative_shape() {
    let c = four(XVariant::Basic, -0.5, 0.5);
    assert!(matches!(
        eval(&c, 1, 0.5, &Config::canonical()),
        Err(ContourError::ShapeOutOfRange { index: 1, .. })
    ));
}

#[test]
fn basic_variant_accepts_zero_to_one() {
    for &s in &[0.0, 0.5, 1.0] {
        assert!(eval(&four(XVariant::Basic, s, s), 1, 0.5, &Config::canonical()).is_ok());
    }
}

#[test]
fn general_variant_range_is_minus_one_to_one() {
    let g = Config::canonical();
    assert!(eval(&four(XVariant::General, -1.0, 1.0), 1, 0.5, &g).is_ok());
    for bad in [-1.0001, 1.0001, f64::NAN] {
        assert!(eval(&four(XVariant::General, bad, 0.0), 1, 0.5, &g).is_err());
    }
}

#[test]
fn basic_and_general_agree_on_shared_range() {
    let g = Config::canonical();
    let a = eval(&four(XVariant::Basic, 0.5, 0.25), 1, 0.375, &g).unwrap();
    let b = eval(&four(XVariant::General, 0.5, 0.25), 1, 0.375, &g).unwrap();
    assert_eq!(a, b);
}

#[test]
fn open_two_point_contour_sharp_ends() {
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![Point::new(0.0, 0.0), Point::new(8.0, 4.0)],
    };
    let g = Config::canonical();
    assert_eq!(c.span_count(&g).unwrap(), 1);
    assert_eq!(eval(&c, 0, 0.0, &g).unwrap(), (0.0, 0.0));
    assert_eq!(eval(&c, 0, 1.0, &g).unwrap(), (8.0, 4.0));
    assert_eq!(eval(&c, 0, 0.5, &g).unwrap(), (4.0, 2.0));
}

#[test]
fn open_end_choice_changes_shape_one_ends() {
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 1.0,
                ..Point::new(0.0, 0.0)
            },
            Point {
                s: 1.0,
                ..Point::new(6.0, 0.0)
            },
        ],
    };
    let mut g = Config::canonical();
    // Repeated end with its own s = 1: (5 P0 + P1) / 6 at the start.
    let a = eval(&c, 0, 0.0, &g).unwrap();
    assert!(close(a, (1.0, 0.0), 1e-14));
    g.open_end = OpenEnd::RepeatEndSharp;
    let b = eval(&c, 0, 0.0, &g).unwrap();
    assert_eq!(b, (0.0, 0.0));
    let e = eval(&c, 0, 1.0, &g).unwrap();
    assert_eq!(e, (6.0, 0.0));
}

#[test]
fn closed_three_point_contour_wraps() {
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: true,
        points: vec![
            Point {
                s: -1.0,
                ..Point::new(0.0, 0.0)
            },
            Point {
                s: -1.0,
                ..Point::new(8.0, 0.0)
            },
            Point {
                s: -1.0,
                ..Point::new(4.0, 8.0)
            },
        ],
    };
    let g = Config::canonical();
    assert_eq!(c.span_count(&g).unwrap(), 3);
    assert_eq!(eval(&c, 2, 0.0, &g).unwrap(), (4.0, 8.0));
    assert_eq!(eval(&c, 2, 1.0, &g).unwrap(), (0.0, 0.0));
}

#[test]
fn closed_two_point_contour_has_two_spans() {
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: true,
        points: vec![Point::new(0.0, 0.0), Point::new(8.0, 4.0)],
    };
    assert_eq!(c.span_count(&Config::canonical()).unwrap(), 2);
}

#[test]
fn closed_without_closing_span_is_open() {
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: true,
        points: vec![
            Point::new(0.0, 0.0),
            Point::new(8.0, 0.0),
            Point::new(8.0, 8.0),
        ],
    };
    let mut g = Config::canonical();
    g.closing_span = false;
    assert_eq!(c.span_count(&g).unwrap(), 2);
}

#[test]
fn coincident_points_with_shape_zero_stay_put() {
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![Point::new(2.0, 3.0); 4],
    };
    for k in 0..=4 {
        let t = f64::from(k) / 4.0;
        assert_eq!(eval(&c, 1, t, &Config::canonical()).unwrap(), (2.0, 3.0));
    }
}

#[test]
fn coincident_points_with_positive_shape_stay_put_within_rounding() {
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.7,
                ..Point::new(2.0, 3.0)
            };
            4
        ],
    };
    for k in 0..=10 {
        let t = f64::from(k) / 10.0;
        let r = eval(&c, 1, t, &Config::canonical()).unwrap();
        assert!(close(r, (2.0, 3.0), 1e-14));
    }
}

#[test]
fn x_spline_ignores_bezier_handles() {
    let a = four(XVariant::General, 0.5, 0.5);
    let mut b = a.clone();
    b.points[1].tan_out = (f64::NAN, 3.0);
    let g = Config::canonical();
    assert_eq!(eval(&a, 1, 0.3, &g).unwrap(), eval(&b, 1, 0.3, &g).unwrap());
}

#[test]
fn single_precision_x_spline_close_to_double() {
    let c = four(XVariant::General, 0.3, -0.6);
    let mut g = Config::canonical();
    g.precision = contour_eval::Precision::F32;
    let a = eval(&c, 1, 0.3, &g).unwrap();
    let b = eval(&c, 1, 0.3, &Config::canonical()).unwrap();
    assert!(close(a, b, 1e-5));
    assert_ne!(a.0.to_bits(), b.0.to_bits());
}
