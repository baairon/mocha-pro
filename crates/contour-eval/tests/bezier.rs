//! Cubic Bezier span tests (Farin 2002, chapters 3 and 4) on made up data.

use contour_eval::{
    bezier_split, eval, Config, Contour, ContourError, EvalForm, Kind, Point, Precision,
    TangentMode,
};

const FORMS: [EvalForm; 3] = [EvalForm::Bernstein, EvalForm::DeCasteljau, EvalForm::Horner];

fn cfg(form: EvalForm) -> Config {
    Config {
        form,
        ..Config::canonical()
    }
}

/// A one span open contour from four absolute control points.
fn span(p: [(f64, f64); 4]) -> Contour {
    Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                x: p[0].0,
                y: p[0].1,
                tan_in: (0.0, 0.0),
                tan_out: (p[1].0 - p[0].0, p[1].1 - p[0].1),
                s: 0.0,
            },
            Point {
                x: p[3].0,
                y: p[3].1,
                tan_in: (p[2].0 - p[3].0, p[2].1 - p[3].1),
                tan_out: (0.0, 0.0),
                s: 0.0,
            },
        ],
    }
}

const A: [(f64, f64); 4] = [(1.0, 2.0), (3.0, 10.0), (9.0, 12.0), (13.0, 4.0)];

fn lcg(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 11) as f64) / ((1u64 << 53) as f64)
}

#[test]
fn endpoints_exact_in_every_form() {
    let p = [(0.1, 0.7), (0.3, 5.9), (1.7, -2.3), (0.9, 0.33)];
    // Relative handles are rebuilt by subtraction, so use absolute mode to keep inputs exact.
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: p[1],
                ..Point::new(p[0].0, p[0].1)
            },
            Point {
                tan_in: p[2],
                ..Point::new(p[3].0, p[3].1)
            },
        ],
    };
    for form in FORMS {
        let mut g = cfg(form);
        g.tangents = TangentMode::Absolute;
        let s = eval(&c, 0, 0.0, &g).unwrap();
        let e = eval(&c, 0, 1.0, &g).unwrap();
        assert_eq!(
            (s.0.to_bits(), s.1.to_bits()),
            (p[0].0.to_bits(), p[0].1.to_bits())
        );
        assert_eq!(
            (e.0.to_bits(), e.1.to_bits()),
            (p[3].0.to_bits(), p[3].1.to_bits())
        );
    }
}

#[test]
fn negative_zero_anchor_kept_at_t0() {
    let c = span([(-0.0, 1.0), (1.0, 1.0), (2.0, 1.0), (3.0, 1.0)]);
    let r = eval(&c, 0, 0.0, &Config::canonical()).unwrap();
    assert_eq!(r.0.to_bits(), (-0.0f64).to_bits());
}

#[test]
fn forms_agree_bitwise_on_dyadic_data() {
    let c = span(A);
    for k in 0..=16 {
        let t = f64::from(k) / 16.0;
        let r: Vec<_> = FORMS
            .iter()
            .map(|&f| {
                let v = eval(&c, 0, t, &cfg(f)).unwrap();
                (v.0.to_bits(), v.1.to_bits())
            })
            .collect();
        assert_eq!(r[0], r[1], "t = {t}");
        assert_eq!(r[0], r[2], "t = {t}");
    }
}

#[test]
fn forms_agree_within_ulps_off_the_dyadic_grid() {
    let p = [(0.13, 0.71), (0.37, 5.93), (1.71, -2.31), (0.93, 0.337)];
    let c = span(p);
    for &t in &[0.1, 0.3, 0.37, 0.61, 0.9] {
        let b = eval(&c, 0, t, &cfg(EvalForm::Bernstein)).unwrap();
        for f in [EvalForm::DeCasteljau, EvalForm::Horner] {
            let v = eval(&c, 0, t, &cfg(f)).unwrap();
            assert!((v.0 - b.0).abs() < 1e-14, "{f:?} t={t}");
            assert!((v.1 - b.1).abs() < 1e-14, "{f:?} t={t}");
        }
    }
}

#[test]
fn split_halves_reproduce_curve_at_quarters() {
    let (l, r) = bezier_split(A, 0.5);
    let cl = span(l);
    let cr = span(r);
    let whole = span(A);
    let g = Config::canonical();
    let lq = eval(&cl, 0, 0.5, &g).unwrap();
    let rq = eval(&cr, 0, 0.5, &g).unwrap();
    let w25 = eval(&whole, 0, 0.25, &g).unwrap();
    let w75 = eval(&whole, 0, 0.75, &g).unwrap();
    assert!((lq.0 - w25.0).abs() < 1e-12 && (lq.1 - w25.1).abs() < 1e-12);
    assert!((rq.0 - w75.0).abs() < 1e-12 && (rq.1 - w75.1).abs() < 1e-12);
}

#[test]
fn split_control_points_known_values() {
    let (l, r) = bezier_split(A, 0.5);
    let want_l: [(f64, f64); 4] = [(1.0, 2.0), (2.0, 6.0), (4.0, 8.5), (6.25, 9.0)];
    let want_r: [(f64, f64); 4] = [(6.25, 9.0), (8.5, 9.5), (11.0, 8.0), (13.0, 4.0)];
    for i in 0..4 {
        assert_eq!(
            (l[i].0.to_bits(), l[i].1.to_bits()),
            (want_l[i].0.to_bits(), want_l[i].1.to_bits())
        );
        assert_eq!(
            (r[i].0.to_bits(), r[i].1.to_bits()),
            (want_r[i].0.to_bits(), want_r[i].1.to_bits())
        );
    }
}

#[test]
fn split_at_ends_keeps_shared_point() {
    let (l, r) = bezier_split(A, 0.0);
    assert_eq!(l, [A[0]; 4]);
    assert_eq!(r, A);
    let (l, r) = bezier_split(A, 1.0);
    assert_eq!(l, A);
    assert_eq!(r, [A[3]; 4]);
}

#[test]
fn closed_contour_has_closing_span() {
    let pts = vec![
        Point {
            tan_out: (1.0, 0.0),
            tan_in: (-1.0, 0.0),
            ..Point::new(0.0, 0.0)
        },
        Point {
            tan_out: (0.0, 1.0),
            tan_in: (0.0, -1.0),
            ..Point::new(8.0, 0.0)
        },
        Point {
            tan_out: (-1.0, 0.0),
            tan_in: (1.0, 0.0),
            ..Point::new(4.0, 8.0)
        },
    ];
    let c = Contour {
        kind: Kind::Bezier,
        closed: true,
        points: pts,
    };
    let g = Config::canonical();
    assert_eq!(c.span_count(&g).unwrap(), 3);
    let a = eval(&c, 2, 0.0, &g).unwrap();
    let b = eval(&c, 2, 1.0, &g).unwrap();
    assert_eq!(a, (4.0, 8.0));
    assert_eq!(b, (0.0, 0.0));
}

#[test]
fn closed_two_point_contour_has_two_spans() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: true,
        points: vec![Point::new(0.0, 0.0), Point::new(4.0, 4.0)],
    };
    let g = Config::canonical();
    assert_eq!(c.span_count(&g).unwrap(), 2);
    assert_eq!(eval(&c, 1, 1.0, &g).unwrap(), (0.0, 0.0));
}

#[test]
fn open_two_point_contour_has_one_span() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![Point::new(0.0, 0.0), Point::new(4.0, 4.0)],
    };
    let g = Config::canonical();
    assert_eq!(c.span_count(&g).unwrap(), 1);
    // zero handles: a cubic with both inner control points on the ends; midpoint at t = 0.5
    // is (0.125 + 0.375 * 0 ...) = 0.5 * P0 + 0.5 * P3 weighted: 1/8 P0 + 3/8 P0 + 3/8 P3 + 1/8 P3.
    assert_eq!(eval(&c, 0, 0.5, &g).unwrap(), (2.0, 2.0));
}

#[test]
fn closing_span_off_makes_closed_behave_open() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: true,
        points: vec![
            Point::new(0.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(4.0, 4.0),
        ],
    };
    let mut g = Config::canonical();
    g.closing_span = false;
    assert_eq!(c.span_count(&g).unwrap(), 2);
    assert_eq!(
        eval(&c, 2, 0.0, &g),
        Err(ContourError::SpanOutOfRange { span: 2, spans: 2 })
    );
}

#[test]
fn coincident_points_give_that_point() {
    let c = span([(3.0, -5.0); 4]);
    for form in FORMS {
        for k in 0..=8 {
            let t = f64::from(k) / 8.0;
            assert_eq!(eval(&c, 0, t, &cfg(form)).unwrap(), (3.0, -5.0));
        }
    }
}

#[test]
fn evenly_spaced_collinear_points_are_linear_in_t() {
    let c = span([(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]);
    for form in FORMS {
        for k in 0..=16 {
            let t = f64::from(k) / 16.0;
            let r = eval(&c, 0, t, &cfg(form)).unwrap();
            assert_eq!(r.0.to_bits(), (3.0 * t).to_bits());
            assert_eq!(r.1, 0.0);
        }
    }
}

#[test]
fn single_precision_matches_double_on_exact_data() {
    let c = span(A);
    let mut g = Config::canonical();
    g.precision = Precision::F32;
    for k in 0..=8 {
        let t = f64::from(k) / 8.0;
        assert_eq!(
            eval(&c, 0, t, &g).unwrap(),
            eval(&c, 0, t, &Config::canonical()).unwrap()
        );
    }
}

#[test]
fn single_precision_result_is_an_f32_value() {
    let c = span([(0.1, 0.2), (0.3, 5.9), (1.7, -2.3), (0.9, 0.33)]);
    let mut g = Config::canonical();
    g.precision = Precision::F32;
    let r = eval(&c, 0, 0.1, &g).unwrap();
    assert_eq!(f64::from(r.0 as f32), r.0);
    assert_eq!(f64::from(r.1 as f32), r.1);
    let d = eval(&c, 0, 0.1, &Config::canonical()).unwrap();
    assert!((r.0 - d.0).abs() < 1e-6 && (r.1 - d.1).abs() < 1e-6);
    assert_ne!(r.0.to_bits(), d.0.to_bits());
}

#[test]
fn bezier_span_reports_relative_and_absolute_points() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (2.0, 8.0),
                ..Point::new(1.0, 2.0)
            },
            Point {
                tan_in: (-4.0, 8.0),
                ..Point::new(13.0, 4.0)
            },
        ],
    };
    let rel = c.bezier_span(0, &Config::canonical()).unwrap();
    assert_eq!(rel, A);
    let c2 = Contour {
        points: vec![
            Point {
                tan_out: (3.0, 10.0),
                ..Point::new(1.0, 2.0)
            },
            Point {
                tan_in: (9.0, 12.0),
                ..Point::new(13.0, 4.0)
            },
        ],
        ..c
    };
    let mut g = Config::canonical();
    g.tangents = TangentMode::Absolute;
    assert_eq!(c2.bezier_span(0, &g).unwrap(), A);
}

#[test]
fn curve_stays_in_convex_hull_of_control_points() {
    let c = span(A);
    let mut st = 1u64;
    for _ in 0..200 {
        let t = lcg(&mut st);
        for form in FORMS {
            let r = eval(&c, 0, t, &cfg(form)).unwrap();
            assert!(r.0 >= 1.0 - 1e-12 && r.0 <= 13.0 + 1e-12);
            assert!(r.1 >= 2.0 - 1e-12 && r.1 <= 12.0 + 1e-12);
        }
    }
}

#[test]
fn reversed_control_points_trace_the_same_curve() {
    let rev = [A[3], A[2], A[1], A[0]];
    let f = span(A);
    let b = span(rev);
    let g = Config::canonical();
    for k in 0..=10 {
        let t = f64::from(k) / 10.0;
        let u = eval(&f, 0, t, &g).unwrap();
        let v = eval(&b, 0, 1.0 - t, &g).unwrap();
        assert!((u.0 - v.0).abs() < 1e-13 && (u.1 - v.1).abs() < 1e-13);
    }
}

#[test]
fn scaling_by_two_scales_the_result_exactly() {
    let big = [(2.0, 4.0), (6.0, 20.0), (18.0, 24.0), (26.0, 8.0)];
    let a = span(A);
    let b = span(big);
    for form in FORMS {
        let u = eval(&a, 0, 0.3, &cfg(form)).unwrap();
        let v = eval(&b, 0, 0.3, &cfg(form)).unwrap();
        assert_eq!(v.0.to_bits(), (u.0 * 2.0).to_bits());
        assert_eq!(v.1.to_bits(), (u.1 * 2.0).to_bits());
    }
}

#[test]
fn translation_moves_the_result() {
    let moved: [(f64, f64); 4] = [
        (A[0].0 + 64.0, A[0].1 - 32.0),
        (A[1].0 + 64.0, A[1].1 - 32.0),
        (A[2].0 + 64.0, A[2].1 - 32.0),
        (A[3].0 + 64.0, A[3].1 - 32.0),
    ];
    let u = eval(&span(A), 0, 0.375, &Config::canonical()).unwrap();
    let v = eval(&span(moved), 0, 0.375, &Config::canonical()).unwrap();
    assert_eq!(v, (u.0 + 64.0, u.1 - 32.0));
}

#[test]
fn bezier_ignores_the_x_spline_weight() {
    let mut c = span(A);
    let g = Config::canonical();
    let before = eval(&c, 0, 0.25, &g).unwrap();
    c.points[0].s = f64::NAN;
    assert_eq!(eval(&c, 0, 0.25, &g).unwrap(), before);
}
