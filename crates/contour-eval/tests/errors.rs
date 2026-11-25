//! Typed refusals: no input makes the crate panic.

use contour_eval::{eval, Config, Contour, ContourError, Kind, Point, Precision, XVariant};

fn bez(points: Vec<Point>) -> Contour {
    Contour {
        kind: Kind::Bezier,
        closed: false,
        points,
    }
}

fn xs(points: Vec<Point>) -> Contour {
    Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points,
    }
}

#[test]
fn empty_contour_refused() {
    assert_eq!(
        eval(&bez(vec![]), 0, 0.5, &Config::canonical()),
        Err(ContourError::TooFewPoints { have: 0, need: 2 })
    );
}

#[test]
fn one_point_refused_for_both_kinds() {
    let p = vec![Point::new(1.0, 1.0)];
    let g = Config::canonical();
    assert_eq!(
        eval(&bez(p.clone()), 0, 0.5, &g),
        Err(ContourError::TooFewPoints { have: 1, need: 2 })
    );
    assert_eq!(
        eval(&xs(p), 0, 0.5, &g),
        Err(ContourError::TooFewPoints { have: 1, need: 2 })
    );
}

#[test]
fn nan_and_infinite_coordinates_refused() {
    let g = Config::canonical();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let c = bez(vec![Point::new(bad, 0.0), Point::new(1.0, 1.0)]);
        assert_eq!(
            eval(&c, 0, 0.5, &g),
            Err(ContourError::NonFinite {
                index: 0,
                field: "x"
            })
        );
        let c = bez(vec![Point::new(0.0, 0.0), Point::new(1.0, bad)]);
        assert_eq!(
            eval(&c, 0, 0.5, &g),
            Err(ContourError::NonFinite {
                index: 1,
                field: "y"
            })
        );
    }
}

#[test]
fn nan_handle_refused_for_bezier_only() {
    let g = Config::canonical();
    let mut p = vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
    p[0].tan_out = (f64::NAN, 0.0);
    assert_eq!(
        eval(&bez(p.clone()), 0, 0.5, &g),
        Err(ContourError::NonFinite {
            index: 0,
            field: "tan_out.x"
        })
    );
    assert!(eval(&xs(p), 0, 0.5, &g).is_ok());
}

#[test]
fn nan_weight_refused_for_x_spline_only() {
    let g = Config::canonical();
    let mut p = vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
    p[1].s = f64::NAN;
    assert_eq!(
        eval(&xs(p.clone()), 0, 0.5, &g),
        Err(ContourError::NonFinite {
            index: 1,
            field: "s"
        })
    );
    assert!(eval(&bez(p), 0, 0.5, &g).is_ok());
}

#[test]
fn span_index_out_of_range() {
    let c = bez(vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
    assert_eq!(
        eval(&c, 1, 0.5, &Config::canonical()),
        Err(ContourError::SpanOutOfRange { span: 1, spans: 1 })
    );
    assert_eq!(
        eval(&c, usize::MAX, 0.5, &Config::canonical()),
        Err(ContourError::SpanOutOfRange {
            span: usize::MAX,
            spans: 1
        })
    );
}

#[test]
fn parameter_out_of_range() {
    let c = bez(vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
    let g = Config::canonical();
    for t in [-0.0001, 1.0001, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            eval(&c, 0, t, &g),
            Err(ContourError::ParamOutOfRange),
            "t = {t}"
        );
    }
    assert!(eval(&c, 0, -0.0, &g).is_ok());
}

#[test]
fn single_precision_overflow_is_reported() {
    let c = bez(vec![Point::new(3.0e38, 0.0), Point::new(3.0e38, 0.0)]);
    let mut g = Config::canonical();
    g.precision = Precision::F32;
    // the anchors are within range but the weighted sum overflows
    let mut c2 = c.clone();
    c2.points[0].tan_out = (3.0e38, 0.0);
    c2.points[1].tan_in = (3.0e38, 0.0);
    assert_eq!(eval(&c2, 0, 0.5, &g), Err(ContourError::NonFiniteResult));
    // the same data in f64 is fine
    assert!(eval(&c2, 0, 0.5, &Config::canonical()).is_ok());
}

#[test]
fn single_precision_input_beyond_f32_is_reported() {
    let c = bez(vec![Point::new(1.0e300, 0.0), Point::new(1.0, 0.0)]);
    let mut g = Config::canonical();
    g.precision = Precision::F32;
    assert_eq!(eval(&c, 0, 0.0, &g), Err(ContourError::NonFiniteResult));
}

#[test]
fn bezier_span_refuses_x_spline() {
    let c = xs(vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
    assert_eq!(
        c.bezier_span(0, &Config::canonical()),
        Err(ContourError::UnsupportedKind)
    );
}

#[test]
fn bezier_span_range_checked() {
    let c = bez(vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
    assert_eq!(
        c.bezier_span(3, &Config::canonical()),
        Err(ContourError::SpanOutOfRange { span: 3, spans: 1 })
    );
}

#[test]
fn coincident_points_are_accepted() {
    let c = bez(vec![Point::new(2.0, 2.0), Point::new(2.0, 2.0)]);
    assert_eq!(eval(&c, 0, 0.5, &Config::canonical()), Ok((2.0, 2.0)));
}

#[test]
fn messages_name_the_problem() {
    let cases: Vec<(ContourError, &str)> = vec![
        (ContourError::TooFewPoints { have: 1, need: 2 }, "1 points"),
        (
            ContourError::NonFinite {
                index: 3,
                field: "x",
            },
            "point 3",
        ),
        (ContourError::SpanOutOfRange { span: 5, spans: 2 }, "span 5"),
        (ContourError::ParamOutOfRange, "[0, 1]"),
        (ContourError::ShapeOutOfRange { index: 1, s: 2.5 }, "2.5"),
        (ContourError::ZeroWeightSum, "zero"),
        (ContourError::NonFiniteResult, "not finite"),
        (ContourError::BadFlatten("why"), "why"),
        (ContourError::UnsupportedFlatTest, "Bezier"),
        (ContourError::UnsupportedKind, "another kind"),
    ];
    for (e, needle) in cases {
        assert!(e.to_string().contains(needle), "{e:?}");
        let _: &dyn std::error::Error = &e;
    }
}
