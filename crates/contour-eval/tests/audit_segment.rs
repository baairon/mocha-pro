//! Audit round 1: the flatness tests measure the distance to the chord SEGMENT, not to the
//! infinite line through it. A collinear overshooting span must not count as flat.

use contour_eval::{flatten, Config, Contour, FlatTest, Flatten, Kind, Point};

/// Absolute handles on the x axis: P0 = (0,0), P1 = (5,0), P2 = (-4,0), P3 = (1,0). The curve
/// runs out to x about 1.7 and back to about -0.4, far outside the chord segment [0, 1].
fn overshoot() -> Contour {
    Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (5.0, 0.0),
                ..Point::new(0.0, 0.0)
            },
            Point {
                tan_in: (-4.0, 0.0),
                ..Point::new(1.0, 0.0)
            },
        ],
    }
}

fn abs_cfg(test: FlatTest) -> Config {
    let mut c = Config::canonical();
    c.tangents = contour_eval::TangentMode::Absolute;
    c.flat_test = test;
    c
}

#[test]
fn control_polygon_test_refuses_collinear_overshoot() {
    let rule = Flatten::Subdivide {
        tol: 0.01,
        max_depth: 12,
    };
    let poly = flatten(&overshoot(), &rule, &abs_cfg(FlatTest::ControlPolygon)).unwrap();
    assert!(
        poly.len() > 2,
        "collinear overshoot was accepted as flat: {poly:?}"
    );
}

#[test]
fn sampled_test_refuses_collinear_overshoot() {
    let rule = Flatten::Subdivide {
        tol: 0.01,
        max_depth: 12,
    };
    let poly = flatten(&overshoot(), &rule, &abs_cfg(FlatTest::Sampled)).unwrap();
    assert!(
        poly.len() > 2,
        "collinear overshoot was accepted as flat: {poly:?}"
    );
}

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
        let d = ((q.0 - a.0 - u * dx).powi(2) + (q.1 - a.1 - u * dy).powi(2)).sqrt();
        best = best.min(d);
    }
    best
}

/// The control polygon test is a bound: every curve point is within tol of the polyline.
#[test]
fn control_polygon_polyline_is_within_tol_of_the_curve() {
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let mut rnd = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut cfg = abs_cfg(FlatTest::ControlPolygon);
    cfg.closing_span = false;
    for case in 0..300 {
        let p = |r: &mut dyn FnMut() -> f64| (r() * 64.0 - 32.0, r() * 64.0 - 32.0);
        let (a, b, c, d) = (p(&mut rnd), p(&mut rnd), p(&mut rnd), p(&mut rnd));
        // every third case is collinear, the shape the line distance test gets wrong
        let (b, c) = if case % 3 == 0 {
            (
                (a.0 + (d.0 - a.0) * 3.0, a.1 + (d.1 - a.1) * 3.0),
                (a.0 - (d.0 - a.0) * 2.0, a.1 - (d.1 - a.1) * 2.0),
            )
        } else {
            (b, c)
        };
        let contour = Contour {
            kind: Kind::Bezier,
            closed: false,
            points: vec![
                Point {
                    tan_out: b,
                    ..Point::new(a.0, a.1)
                },
                Point {
                    tan_in: c,
                    ..Point::new(d.0, d.1)
                },
            ],
        };
        let tol = 0.05;
        let poly = flatten(&contour, &Flatten::Subdivide { tol, max_depth: 20 }, &cfg).unwrap();
        for i in 0..=200 {
            let t = f64::from(i) / 200.0;
            let q = contour_eval::eval(&contour, 0, t, &cfg).unwrap();
            let dist = dist_to_poly(&poly, q);
            assert!(dist <= tol * 1.0001, "case {case} t {t}: {dist} > {tol}");
        }
    }
}
