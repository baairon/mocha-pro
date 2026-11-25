//! Flattening tests: counts, determinism, deviation bounds, constants.

use contour_eval::{
    eval, flatten, Config, Contour, ContourError, FlatTest, Flatten, Kind, Point, Precision,
    XVariant,
};

fn bez_open() -> Contour {
    // three spans over four anchors, relative handles with dyadic values
    Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (4.0, 8.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_in: (-4.0, 0.0),
                tan_out: (4.0, 0.0),
                ..Point::new(16.0, 8.0)
            },
            Point {
                tan_in: (-4.0, 0.0),
                tan_out: (4.0, -8.0),
                ..Point::new(32.0, 8.0)
            },
            Point {
                tan_in: (-4.0, 8.0),
                ..Point::new(48.0, 0.0)
            },
        ],
    }
}

fn bez_closed() -> Contour {
    Contour {
        closed: true,
        ..bez_open()
    }
}

fn x_open(s: f64) -> Contour {
    Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: (0..4)
            .map(|i| Point {
                s,
                ..Point::new(f64::from(i) * 4.0, if i % 2 == 0 { 0.0 } else { 4.0 })
            })
            .collect(),
    }
}

fn bits(v: &[(f64, f64)]) -> Vec<(u64, u64)> {
    v.iter().map(|p| (p.0.to_bits(), p.1.to_bits())).collect()
}

/// Distance from `q` to the polyline.
fn dist_to_poly(poly: &[(f64, f64)], q: (f64, f64)) -> f64 {
    let mut best = f64::INFINITY;
    for w in poly.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        let u = if len2 == 0.0 {
            0.0
        } else {
            (((q.0 - a.0) * dx + (q.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
        };
        let (px, py) = (a.0 + u * dx, a.1 + u * dy);
        let d = ((q.0 - px).powi(2) + (q.1 - py).powi(2)).sqrt();
        if d < best {
            best = d;
        }
    }
    best
}

#[test]
fn uniform_open_bezier_count() {
    let p = flatten(&bez_open(), &Flatten::Uniform(4), &Config::canonical()).unwrap();
    assert_eq!(p.len(), 3 * 4 + 1);
}

#[test]
fn uniform_closed_count_without_repeat() {
    let p = flatten(&bez_closed(), &Flatten::Uniform(4), &Config::canonical()).unwrap();
    assert_eq!(p.len(), 4 * 4);
}

#[test]
fn uniform_closed_count_with_repeat() {
    let mut g = Config::canonical();
    g.repeat_first = true;
    let p = flatten(&bez_closed(), &Flatten::Uniform(4), &g).unwrap();
    assert_eq!(p.len(), 4 * 4 + 1);
    assert_eq!(bits(&p[..1]), bits(&p[16..]));
}

#[test]
fn uniform_one_segment_gives_the_anchors() {
    let p = flatten(&bez_open(), &Flatten::Uniform(1), &Config::canonical()).unwrap();
    assert_eq!(p, vec![(0.0, 0.0), (16.0, 8.0), (32.0, 8.0), (48.0, 0.0)]);
}

#[test]
fn uniform_points_are_the_eval_at_i_over_n() {
    let c = bez_open();
    let g = Config::canonical();
    let p = flatten(&c, &Flatten::Uniform(8), &g).unwrap();
    for span in 0..3 {
        for i in 0..8 {
            let q = eval(&c, span, f64::from(i) / 8.0, &g).unwrap();
            assert_eq!(bits(&[p[span * 8 + i as usize]]), bits(&[q]));
        }
    }
}

#[test]
fn uniform_x_spline_count() {
    let p = flatten(&x_open(0.0), &Flatten::Uniform(3), &Config::canonical()).unwrap();
    assert_eq!(p.len(), 3 * 3 + 1);
    assert_eq!(p[0], (0.0, 0.0));
    assert_eq!(*p.last().unwrap(), (12.0, 4.0));
}

#[test]
fn uniform_closed_x_spline_count() {
    let mut c = x_open(0.5);
    c.closed = true;
    let p = flatten(&c, &Flatten::Uniform(5), &Config::canonical()).unwrap();
    assert_eq!(p.len(), 4 * 5);
}

#[test]
fn two_point_open_contour_uniform() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![Point::new(0.0, 0.0), Point::new(4.0, 4.0)],
    };
    let p = flatten(&c, &Flatten::Uniform(2), &Config::canonical()).unwrap();
    assert_eq!(p, vec![(0.0, 0.0), (2.0, 2.0), (4.0, 4.0)]);
}

#[test]
fn flatten_is_deterministic() {
    for rule in [
        Flatten::Uniform(7),
        Flatten::Subdivide {
            tol: 0.05,
            max_depth: 12,
        },
        Flatten::ByLength {
            step: 1.5,
            samples: 16,
        },
    ] {
        for c in [bez_open(), bez_closed(), x_open(0.4)] {
            let g = Config::canonical();
            let a = flatten(&c, &rule, &g).unwrap();
            let b = flatten(&c.clone(), &rule, &g.clone()).unwrap();
            assert_eq!(bits(&a), bits(&b));
        }
    }
}

#[test]
fn flatten_is_deterministic_across_all_configs() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: true,
        points: vec![
            Point {
                tan_out: (2.0, 0.0),
                tan_in: (-2.0, 0.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_out: (0.0, 2.0),
                tan_in: (0.0, -2.0),
                ..Point::new(8.0, 0.0)
            },
            Point {
                tan_out: (-2.0, 0.0),
                tan_in: (2.0, 0.0),
                ..Point::new(4.0, 8.0)
            },
        ],
    };
    for g in Config::enumerate() {
        let a = flatten(
            &c,
            &Flatten::Subdivide {
                tol: 0.1,
                max_depth: 10,
            },
            &g,
        );
        let b = flatten(
            &c,
            &Flatten::Subdivide {
                tol: 0.1,
                max_depth: 10,
            },
            &g,
        );
        assert_eq!(a, b);
        assert!(a.is_ok());
    }
}

#[test]
fn subdivide_control_polygon_respects_tolerance() {
    let c = bez_open();
    let mut g = Config::canonical();
    g.flat_test = FlatTest::ControlPolygon;
    let tol = 0.05;
    let poly = flatten(&c, &Flatten::Subdivide { tol, max_depth: 16 }, &g).unwrap();
    assert!(poly.len() > 8);
    for span in 0..3 {
        for k in 0..=64 {
            let q = eval(&c, span, f64::from(k) / 64.0, &g).unwrap();
            assert!(dist_to_poly(&poly, q) <= tol + 1e-9, "span {span} k {k}");
        }
    }
}

#[test]
fn subdivide_midpoint_stays_near_tolerance() {
    let c = bez_open();
    let g = Config::canonical();
    let tol = 0.05;
    let poly = flatten(&c, &Flatten::Subdivide { tol, max_depth: 16 }, &g).unwrap();
    for span in 0..3 {
        for k in 0..=64 {
            let q = eval(&c, span, f64::from(k) / 64.0, &g).unwrap();
            assert!(dist_to_poly(&poly, q) <= 2.0 * tol, "span {span} k {k}");
        }
    }
}

#[test]
fn subdivide_x_spline_midpoint_test_runs() {
    let g = Config::canonical();
    let c = x_open(0.5);
    let poly = flatten(
        &c,
        &Flatten::Subdivide {
            tol: 0.01,
            max_depth: 14,
        },
        &g,
    )
    .unwrap();
    assert!(poly.len() > 4);
    for span in 0..3 {
        for k in 0..=32 {
            let q = eval(&c, span, f64::from(k) / 32.0, &g).unwrap();
            assert!(dist_to_poly(&poly, q) <= 0.05);
        }
    }
}

#[test]
fn subdivide_huge_tolerance_keeps_only_span_ends() {
    for test in [FlatTest::Sampled, FlatTest::ControlPolygon] {
        let mut g = Config::canonical();
        g.flat_test = test;
        let p = flatten(
            &bez_open(),
            &Flatten::Subdivide {
                tol: 1e9,
                max_depth: 10,
            },
            &g,
        )
        .unwrap();
        assert_eq!(p, vec![(0.0, 0.0), (16.0, 8.0), (32.0, 8.0), (48.0, 0.0)]);
    }
}

#[test]
fn subdivide_depth_zero_is_one_segment_per_span() {
    for test in [FlatTest::Sampled, FlatTest::ControlPolygon] {
        let mut g = Config::canonical();
        g.flat_test = test;
        let p = flatten(
            &bez_open(),
            &Flatten::Subdivide {
                tol: 0.0,
                max_depth: 0,
            },
            &g,
        )
        .unwrap();
        assert_eq!(p.len(), 4);
    }
}

#[test]
fn subdivide_straight_line_stops_at_zero_tolerance() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (4.0, 0.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_in: (-4.0, 0.0),
                ..Point::new(12.0, 0.0)
            },
        ],
    };
    let mut g = Config::canonical();
    g.flat_test = FlatTest::ControlPolygon;
    let p = flatten(
        &c,
        &Flatten::Subdivide {
            tol: 0.0,
            max_depth: 10,
        },
        &g,
    )
    .unwrap();
    assert_eq!(p, vec![(0.0, 0.0), (12.0, 0.0)]);
}

#[test]
fn smaller_tolerance_gives_at_least_as_many_points() {
    let g = Config::canonical();
    let mut last = 0;
    for &tol in &[4.0, 1.0, 0.25, 0.0625, 0.015625] {
        let n = flatten(&bez_open(), &Flatten::Subdivide { tol, max_depth: 16 }, &g)
            .unwrap()
            .len();
        assert!(n >= last, "tol {tol}");
        last = n;
    }
}

#[test]
fn control_polygon_output_has_dyadic_parameters() {
    // depth 1 on a curved span splits once: the polyline holds anchor, split point, end.
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (0.0, 8.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_in: (0.0, 8.0),
                ..Point::new(8.0, 0.0)
            },
        ],
    };
    let mut g = Config::canonical();
    g.flat_test = FlatTest::ControlPolygon;
    let p = flatten(
        &c,
        &Flatten::Subdivide {
            tol: 0.0,
            max_depth: 1,
        },
        &g,
    )
    .unwrap();
    // B(0.5) = (4, 6)
    assert_eq!(p, vec![(0.0, 0.0), (4.0, 6.0), (8.0, 0.0)]);
}

fn line12() -> Contour {
    Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (4.0, 0.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_in: (-4.0, 0.0),
                ..Point::new(12.0, 0.0)
            },
        ],
    }
}

#[test]
fn by_length_counts_segments_from_length() {
    let g = Config::canonical();
    let p = flatten(
        &line12(),
        &Flatten::ByLength {
            step: 3.0,
            samples: 16,
        },
        &g,
    )
    .unwrap();
    assert_eq!(p.len(), 5);
    assert_eq!(p[1], (3.0, 0.0));
    let p = flatten(
        &line12(),
        &Flatten::ByLength {
            step: 5.0,
            samples: 16,
        },
        &g,
    )
    .unwrap();
    assert_eq!(p.len(), 4); // ceil(12 / 5) = 3 segments
    let p = flatten(
        &line12(),
        &Flatten::ByLength {
            step: 100.0,
            samples: 16,
        },
        &g,
    )
    .unwrap();
    assert_eq!(p.len(), 2);
}

#[test]
fn by_length_zero_length_span_gets_one_segment() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![Point::new(1.0, 1.0), Point::new(1.0, 1.0)],
    };
    let p = flatten(
        &c,
        &Flatten::ByLength {
            step: 0.5,
            samples: 8,
        },
        &Config::canonical(),
    )
    .unwrap();
    assert_eq!(p, vec![(1.0, 1.0), (1.0, 1.0)]);
}

#[test]
fn by_length_is_capped() {
    let p = flatten(
        &line12(),
        &Flatten::ByLength {
            step: 1e-9,
            samples: 4,
        },
        &Config::canonical(),
    )
    .unwrap();
    assert_eq!(p.len(), 4096 + 1);
}

#[test]
fn single_precision_flatten_close_to_double() {
    let mut g = Config::canonical();
    let a = flatten(&bez_open(), &Flatten::Uniform(9), &g).unwrap();
    g.precision = Precision::F32;
    let b = flatten(&bez_open(), &Flatten::Uniform(9), &g).unwrap();
    assert_eq!(a.len(), b.len());
    for (u, v) in a.iter().zip(&b) {
        assert!((u.0 - v.0).abs() < 1e-4 && (u.1 - v.1).abs() < 1e-4);
    }
}

#[test]
fn bad_rules_are_refused() {
    let g = Config::canonical();
    let c = bez_open();
    let bad = [
        Flatten::Uniform(0),
        Flatten::Uniform(4097),
        Flatten::Subdivide {
            tol: -1.0,
            max_depth: 4,
        },
        Flatten::Subdivide {
            tol: f64::NAN,
            max_depth: 4,
        },
        Flatten::Subdivide {
            tol: f64::INFINITY,
            max_depth: 4,
        },
        Flatten::Subdivide {
            tol: 0.1,
            max_depth: 25,
        },
        Flatten::ByLength {
            step: 0.0,
            samples: 4,
        },
        Flatten::ByLength {
            step: f64::NAN,
            samples: 4,
        },
        Flatten::ByLength {
            step: f64::INFINITY,
            samples: 4,
        },
        Flatten::ByLength {
            step: 1.0,
            samples: 0,
        },
    ];
    for r in bad {
        assert!(
            matches!(flatten(&c, &r, &g), Err(ContourError::BadFlatten(_))),
            "{r:?}"
        );
    }
}

#[test]
fn control_polygon_test_needs_a_bezier_contour() {
    let mut g = Config::canonical();
    g.flat_test = FlatTest::ControlPolygon;
    let r = flatten(
        &x_open(0.0),
        &Flatten::Subdivide {
            tol: 0.1,
            max_depth: 4,
        },
        &g,
    );
    assert_eq!(r, Err(ContourError::UnsupportedFlatTest));
    // other rules do not use the test
    assert!(flatten(&x_open(0.0), &Flatten::Uniform(2), &g).is_ok());
}

#[test]
fn flatten_refuses_bad_contours() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![Point::new(0.0, 0.0)],
    };
    assert!(matches!(
        flatten(&c, &Flatten::Uniform(2), &Config::canonical()),
        Err(ContourError::TooFewPoints { have: 1, need: 2 })
    ));
}

#[test]
fn rule_limits_are_inclusive() {
    let g = Config::canonical();
    let c = line12();
    assert!(flatten(&c, &Flatten::Uniform(4096), &g).is_ok());
    assert!(flatten(
        &c,
        &Flatten::Subdivide {
            tol: 1e9,
            max_depth: 24
        },
        &g
    )
    .is_ok());
    assert!(flatten(
        &c,
        &Flatten::ByLength {
            step: 1.0,
            samples: 4096
        },
        &g
    )
    .is_ok());
    assert!(flatten(
        &c,
        &Flatten::ByLength {
            step: 1.0,
            samples: 4097
        },
        &g
    )
    .is_err());
    assert!(flatten(
        &c,
        &Flatten::Subdivide {
            tol: 0.0,
            max_depth: 0
        },
        &g
    )
    .is_ok());
}

#[test]
fn control_polygon_test_needs_both_inner_points_near_the_chord() {
    // P1 lies on the chord, P2 does not: the span is not flat and splits once at depth 1.
    let mut g = Config::canonical();
    g.flat_test = FlatTest::ControlPolygon;
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (4.0, 0.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_in: (-4.0, 8.0),
                ..Point::new(12.0, 0.0)
            },
        ],
    };
    let p = flatten(
        &c,
        &Flatten::Subdivide {
            tol: 1.0,
            max_depth: 1,
        },
        &g,
    )
    .unwrap();
    assert_eq!(p.len(), 3);
    // the mirrored case: P2 on the chord, P1 off it
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (4.0, 8.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_in: (-4.0, 0.0),
                ..Point::new(12.0, 0.0)
            },
        ],
    };
    let p = flatten(
        &c,
        &Flatten::Subdivide {
            tol: 1.0,
            max_depth: 1,
        },
        &g,
    )
    .unwrap();
    assert_eq!(p.len(), 3);
}

#[test]
fn sampled_test_splits_at_the_parameter_midpoint() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (0.0, 8.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_in: (0.0, 8.0),
                ..Point::new(8.0, 0.0)
            },
        ],
    };
    let p = flatten(
        &c,
        &Flatten::Subdivide {
            tol: 0.0,
            max_depth: 1,
        },
        &Config::canonical(),
    )
    .unwrap();
    assert_eq!(p, vec![(0.0, 0.0), (4.0, 6.0), (8.0, 0.0)]);
}

#[test]
fn sampled_test_looks_at_the_three_quarter_point() {
    // y(t) = 24 t^2 (1 - t): 3.0 at t = 1/2, 1.125 at 1/4, 3.375 at 3/4; the tolerance
    // accepts the first two and refuses the last, so the span splits once at depth 1.
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (4.0, 0.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_in: (-4.0, 8.0),
                ..Point::new(12.0, 0.0)
            },
        ],
    };
    let p = flatten(
        &c,
        &Flatten::Subdivide {
            tol: 3.2,
            max_depth: 1,
        },
        &Config::canonical(),
    )
    .unwrap();
    assert_eq!(p.len(), 3);
}

#[test]
fn sampled_test_looks_at_the_quarter_point() {
    // The mirror image: y(t) = 24 t (1 - t)^2 is large at 1/4 only.
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (4.0, 8.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_in: (-4.0, 0.0),
                ..Point::new(12.0, 0.0)
            },
        ],
    };
    let p = flatten(
        &c,
        &Flatten::Subdivide {
            tol: 3.2,
            max_depth: 1,
        },
        &Config::canonical(),
    )
    .unwrap();
    assert_eq!(p.len(), 3);
}
