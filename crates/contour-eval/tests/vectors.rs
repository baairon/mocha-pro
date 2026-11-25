//! Hand computed vectors. Every expected value was computed in exact rational arithmetic
//! from the published definitions (Farin 2002; Blanc and Schlick 1995) and rounded once to
//! the nearest f64; it is written as its bit pattern. All inputs are dyadic, so every
//! operation of the crate is exact and the result must match bit for bit.

use contour_eval::{eval, Config, Contour, EvalForm, Kind, Point, TangentMode, XVariant};

fn pair(v: (f64, f64)) -> (u64, u64) {
    (v.0.to_bits(), v.1.to_bits())
}

#[test]
fn bezier_curve_a_bernstein() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                x: 1.0,
                y: 2.0,
                tan_in: (0.0, 0.0),
                tan_out: (2.0, 8.0),
                s: 0.0,
            },
            Point {
                x: 13.0,
                y: 4.0,
                tan_in: (-4.0, 8.0),
                tan_out: (0.0, 0.0),
                s: 0.0,
            },
        ],
    };
    let mut cfg = Config::canonical();
    cfg.form = EvalForm::Bernstein;
    assert_eq!(
        pair(eval(&c, 0, 0.0, &cfg).unwrap()),
        (0x3ff0000000000000, 0x4000000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 0, 0.125, &cfg).unwrap()),
        (0x3ffed00000000000, 0x4012d80000000000)
    ); // t = 1/8
    assert_eq!(
        pair(eval(&c, 0, 0.25, &cfg).unwrap()),
        (0x4009400000000000, 0x401b400000000000)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 0, 0.5, &cfg).unwrap()),
        (0x4019000000000000, 0x4022000000000000)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 0, 0.75, &cfg).unwrap()),
        (0x4023700000000000, 0x4020600000000000)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 0, 0.875, &cfg).unwrap()),
        (0x4026d60000000000, 0x401a280000000000)
    ); // t = 7/8
    assert_eq!(
        pair(eval(&c, 0, 1.0, &cfg).unwrap()),
        (0x402a000000000000, 0x4010000000000000)
    ); // t = 1
}

#[test]
fn bezier_curve_a_decasteljau() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                x: 1.0,
                y: 2.0,
                tan_in: (0.0, 0.0),
                tan_out: (2.0, 8.0),
                s: 0.0,
            },
            Point {
                x: 13.0,
                y: 4.0,
                tan_in: (-4.0, 8.0),
                tan_out: (0.0, 0.0),
                s: 0.0,
            },
        ],
    };
    let mut cfg = Config::canonical();
    cfg.form = EvalForm::DeCasteljau;
    assert_eq!(
        pair(eval(&c, 0, 0.0, &cfg).unwrap()),
        (0x3ff0000000000000, 0x4000000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 0, 0.125, &cfg).unwrap()),
        (0x3ffed00000000000, 0x4012d80000000000)
    ); // t = 1/8
    assert_eq!(
        pair(eval(&c, 0, 0.25, &cfg).unwrap()),
        (0x4009400000000000, 0x401b400000000000)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 0, 0.5, &cfg).unwrap()),
        (0x4019000000000000, 0x4022000000000000)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 0, 0.75, &cfg).unwrap()),
        (0x4023700000000000, 0x4020600000000000)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 0, 0.875, &cfg).unwrap()),
        (0x4026d60000000000, 0x401a280000000000)
    ); // t = 7/8
    assert_eq!(
        pair(eval(&c, 0, 1.0, &cfg).unwrap()),
        (0x402a000000000000, 0x4010000000000000)
    ); // t = 1
}

#[test]
fn bezier_curve_a_horner() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                x: 1.0,
                y: 2.0,
                tan_in: (0.0, 0.0),
                tan_out: (2.0, 8.0),
                s: 0.0,
            },
            Point {
                x: 13.0,
                y: 4.0,
                tan_in: (-4.0, 8.0),
                tan_out: (0.0, 0.0),
                s: 0.0,
            },
        ],
    };
    let mut cfg = Config::canonical();
    cfg.form = EvalForm::Horner;
    assert_eq!(
        pair(eval(&c, 0, 0.0, &cfg).unwrap()),
        (0x3ff0000000000000, 0x4000000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 0, 0.125, &cfg).unwrap()),
        (0x3ffed00000000000, 0x4012d80000000000)
    ); // t = 1/8
    assert_eq!(
        pair(eval(&c, 0, 0.25, &cfg).unwrap()),
        (0x4009400000000000, 0x401b400000000000)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 0, 0.5, &cfg).unwrap()),
        (0x4019000000000000, 0x4022000000000000)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 0, 0.75, &cfg).unwrap()),
        (0x4023700000000000, 0x4020600000000000)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 0, 0.875, &cfg).unwrap()),
        (0x4026d60000000000, 0x401a280000000000)
    ); // t = 7/8
    assert_eq!(
        pair(eval(&c, 0, 1.0, &cfg).unwrap()),
        (0x402a000000000000, 0x4010000000000000)
    ); // t = 1
}

#[test]
fn bezier_curve_b_bernstein() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                x: -2.0,
                y: 0.5,
                tan_in: (0.0, 0.0),
                tan_out: (-4.0, -3.5),
                s: 0.0,
            },
            Point {
                x: 7.0,
                y: 0.75,
                tan_in: (-2.0, -9.75),
                tan_out: (0.0, 0.0),
                s: 0.0,
            },
        ],
    };
    let mut cfg = Config::canonical();
    cfg.form = EvalForm::Bernstein;
    assert_eq!(
        pair(eval(&c, 0, 0.0, &cfg).unwrap()),
        (0xc000000000000000, 0x3fe0000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 0, 0.125, &cfg).unwrap()),
        (0xc006c00000000000, 0xbfec9c0000000000)
    ); // t = 1/8
    assert_eq!(
        pair(eval(&c, 0, 0.25, &cfg).unwrap()),
        (0xc004800000000000, 0xc002780000000000)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 0, 0.5, &cfg).unwrap()),
        (0x3fd0000000000000, 0xc011600000000000)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 0, 0.75, &cfg).unwrap()),
        (0x4010c00000000000, 0xc00f280000000000)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 0, 0.875, &cfg).unwrap()),
        (0x4017800000000000, 0xc001a10000000000)
    ); // t = 7/8
    assert_eq!(
        pair(eval(&c, 0, 1.0, &cfg).unwrap()),
        (0x401c000000000000, 0x3fe8000000000000)
    ); // t = 1
}

#[test]
fn bezier_curve_b_decasteljau() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                x: -2.0,
                y: 0.5,
                tan_in: (0.0, 0.0),
                tan_out: (-4.0, -3.5),
                s: 0.0,
            },
            Point {
                x: 7.0,
                y: 0.75,
                tan_in: (-2.0, -9.75),
                tan_out: (0.0, 0.0),
                s: 0.0,
            },
        ],
    };
    let mut cfg = Config::canonical();
    cfg.form = EvalForm::DeCasteljau;
    assert_eq!(
        pair(eval(&c, 0, 0.0, &cfg).unwrap()),
        (0xc000000000000000, 0x3fe0000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 0, 0.125, &cfg).unwrap()),
        (0xc006c00000000000, 0xbfec9c0000000000)
    ); // t = 1/8
    assert_eq!(
        pair(eval(&c, 0, 0.25, &cfg).unwrap()),
        (0xc004800000000000, 0xc002780000000000)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 0, 0.5, &cfg).unwrap()),
        (0x3fd0000000000000, 0xc011600000000000)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 0, 0.75, &cfg).unwrap()),
        (0x4010c00000000000, 0xc00f280000000000)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 0, 0.875, &cfg).unwrap()),
        (0x4017800000000000, 0xc001a10000000000)
    ); // t = 7/8
    assert_eq!(
        pair(eval(&c, 0, 1.0, &cfg).unwrap()),
        (0x401c000000000000, 0x3fe8000000000000)
    ); // t = 1
}

#[test]
fn bezier_curve_b_horner() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                x: -2.0,
                y: 0.5,
                tan_in: (0.0, 0.0),
                tan_out: (-4.0, -3.5),
                s: 0.0,
            },
            Point {
                x: 7.0,
                y: 0.75,
                tan_in: (-2.0, -9.75),
                tan_out: (0.0, 0.0),
                s: 0.0,
            },
        ],
    };
    let mut cfg = Config::canonical();
    cfg.form = EvalForm::Horner;
    assert_eq!(
        pair(eval(&c, 0, 0.0, &cfg).unwrap()),
        (0xc000000000000000, 0x3fe0000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 0, 0.125, &cfg).unwrap()),
        (0xc006c00000000000, 0xbfec9c0000000000)
    ); // t = 1/8
    assert_eq!(
        pair(eval(&c, 0, 0.25, &cfg).unwrap()),
        (0xc004800000000000, 0xc002780000000000)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 0, 0.5, &cfg).unwrap()),
        (0x3fd0000000000000, 0xc011600000000000)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 0, 0.75, &cfg).unwrap()),
        (0x4010c00000000000, 0xc00f280000000000)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 0, 0.875, &cfg).unwrap()),
        (0x4017800000000000, 0xc001a10000000000)
    ); // t = 7/8
    assert_eq!(
        pair(eval(&c, 0, 1.0, &cfg).unwrap()),
        (0x401c000000000000, 0x3fe8000000000000)
    ); // t = 1
}

#[test]
fn bezier_absolute_tangents_match_relative_curve() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                x: 1.0,
                y: 2.0,
                tan_in: (0.0, 0.0),
                tan_out: (3.0, 10.0),
                s: 0.0,
            },
            Point {
                x: 13.0,
                y: 4.0,
                tan_in: (9.0, 12.0),
                tan_out: (0.0, 0.0),
                s: 0.0,
            },
        ],
    };
    let mut cfg = Config::canonical();
    cfg.tangents = TangentMode::Absolute;
    assert_eq!(
        pair(eval(&c, 0, 0.0, &cfg).unwrap()),
        (0x3ff0000000000000, 0x4000000000000000)
    );
    assert_eq!(
        pair(eval(&c, 0, 0.125, &cfg).unwrap()),
        (0x3ffed00000000000, 0x4012d80000000000)
    );
    assert_eq!(
        pair(eval(&c, 0, 0.25, &cfg).unwrap()),
        (0x4009400000000000, 0x401b400000000000)
    );
    assert_eq!(
        pair(eval(&c, 0, 0.5, &cfg).unwrap()),
        (0x4019000000000000, 0x4022000000000000)
    );
    assert_eq!(
        pair(eval(&c, 0, 0.75, &cfg).unwrap()),
        (0x4023700000000000, 0x4020600000000000)
    );
    assert_eq!(
        pair(eval(&c, 0, 0.875, &cfg).unwrap()),
        (0x4026d60000000000, 0x401a280000000000)
    );
    assert_eq!(
        pair(eval(&c, 0, 1.0, &cfg).unwrap()),
        (0x402a000000000000, 0x4010000000000000)
    );
}

#[test]
fn xspline_vectors_s1_0() {
    // s1 = 0, s2 = 0
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(1.0, 2.0)
            },
            Point {
                s: 0.0,
                ..Point::new(5.0, 10.0)
            },
            Point {
                s: 0.0,
                ..Point::new(13.0, 6.0)
            },
            Point {
                s: 0.0,
                ..Point::new(17.0, 0.0)
            },
        ],
    };
    let cfg = Config::canonical();
    assert_eq!(
        pair(eval(&c, 1, 0.0, &cfg).unwrap()),
        (0x4014000000000000, 0x4024000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 1, 0.25, &cfg).unwrap()),
        (0x4016f53e62f53e63, 0x402342b06742b067)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 1, 0.5, &cfg).unwrap()),
        (0x4022000000000000, 0x4020000000000000)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 1, 0.75, &cfg).unwrap()),
        (0x40288560ce8560cf, 0x40197a9f317a9f31)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 1, 1.0, &cfg).unwrap()),
        (0x402a000000000000, 0x4018000000000000)
    ); // t = 1
}

#[test]
fn xspline_vectors_s1_1() {
    // s1 = 1, s2 = 1
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(1.0, 2.0)
            },
            Point {
                s: 1.0,
                ..Point::new(5.0, 10.0)
            },
            Point {
                s: 1.0,
                ..Point::new(13.0, 6.0)
            },
            Point {
                s: 0.0,
                ..Point::new(17.0, 0.0)
            },
        ],
    };
    let cfg = Config::canonical();
    assert_eq!(
        pair(eval(&c, 1, 0.0, &cfg).unwrap()),
        (0x4016aaaaaaaaaaab, 0x4020000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 1, 0.25, &cfg).unwrap()),
        (0x401cc2e6bdc80576, 0x402047a32846ff51)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 1, 0.5, &cfg).unwrap()),
        (0x4022000000000000, 0x401eb4de9bd37a6f)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 1, 0.75, &cfg).unwrap()),
        (0x40259e8ca11bfd45, 0x401b1020c49ba5e3)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 1, 1.0, &cfg).unwrap()),
        (0x4028aaaaaaaaaaab, 0x4016aaaaaaaaaaab)
    ); // t = 1
}

#[test]
fn xspline_vectors_s1_2() {
    // s1 = -1, s2 = -1
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(1.0, 2.0)
            },
            Point {
                s: -1.0,
                ..Point::new(5.0, 10.0)
            },
            Point {
                s: -1.0,
                ..Point::new(13.0, 6.0)
            },
            Point {
                s: 0.0,
                ..Point::new(17.0, 0.0)
            },
        ],
    };
    let cfg = Config::canonical();
    assert_eq!(
        pair(eval(&c, 1, 0.0, &cfg).unwrap()),
        (0x4014000000000000, 0x4024000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 1, 0.25, &cfg).unwrap()),
        (0x401eb71fc4345238, 0x4023f01de5d6e3f9)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 1, 0.5, &cfg).unwrap()),
        (0x4022000000000000, 0x4022555555555555)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 1, 0.75, &cfg).unwrap()),
        (0x4024a4701de5d6e4, 0x40200c255f4c9cf7)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 1, 1.0, &cfg).unwrap()),
        (0x402a000000000000, 0x4018000000000000)
    ); // t = 1
}

#[test]
fn xspline_vectors_s1_3() {
    // s1 = -1/2, s2 = 1
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(1.0, 2.0)
            },
            Point {
                s: -0.5,
                ..Point::new(5.0, 10.0)
            },
            Point {
                s: 1.0,
                ..Point::new(13.0, 6.0)
            },
            Point {
                s: 0.0,
                ..Point::new(17.0, 0.0)
            },
        ],
    };
    let cfg = Config::canonical();
    assert_eq!(
        pair(eval(&c, 1, 0.0, &cfg).unwrap()),
        (0x4014000000000000, 0x4024000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 1, 0.25, &cfg).unwrap()),
        (0x401b7a01eae807ac, 0x40235d104cb44133)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 1, 0.5, &cfg).unwrap()),
        (0x4022241a41a41a42, 0x40207b13b13b13b1)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 1, 0.75, &cfg).unwrap()),
        (0x4025baa32a32a32a, 0x401b512492492492)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 1, 1.0, &cfg).unwrap()),
        (0x4028aaaaaaaaaaab, 0x4016aaaaaaaaaaab)
    ); // t = 1
}

#[test]
fn xspline_vectors_s1_4() {
    // s1 = 1, s2 = -1/2
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(1.0, 2.0)
            },
            Point {
                s: 1.0,
                ..Point::new(5.0, 10.0)
            },
            Point {
                s: -0.5,
                ..Point::new(13.0, 6.0)
            },
            Point {
                s: 0.0,
                ..Point::new(17.0, 0.0)
            },
        ],
    };
    let cfg = Config::canonical();
    assert_eq!(
        pair(eval(&c, 1, 0.0, &cfg).unwrap()),
        (0x4016aaaaaaaaaaab, 0x4020000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 1, 0.25, &cfg).unwrap()),
        (0x401c8ab9ab9ab9ac, 0x40205e8168168168)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 1, 0.5, &cfg).unwrap()),
        (0x4021dbe5be5be5be, 0x401faaaaaaaaaaab)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 1, 0.75, &cfg).unwrap()),
        (0x402642ff0a8bfc2a, 0x401c97baf75eebdd)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 1, 1.0, &cfg).unwrap()),
        (0x402a000000000000, 0x4018000000000000)
    ); // t = 1
}

#[test]
fn xspline_vectors_s1_5() {
    // s1 = 0, s2 = 1
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(1.0, 2.0)
            },
            Point {
                s: 0.0,
                ..Point::new(5.0, 10.0)
            },
            Point {
                s: 1.0,
                ..Point::new(13.0, 6.0)
            },
            Point {
                s: 0.0,
                ..Point::new(17.0, 0.0)
            },
        ],
    };
    let cfg = Config::canonical();
    assert_eq!(
        pair(eval(&c, 1, 0.0, &cfg).unwrap()),
        (0x4014000000000000, 0x4024000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 1, 0.25, &cfg).unwrap()),
        (0x4016f9b0388d6174, 0x4023391ecd008143)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 1, 0.5, &cfg).unwrap()),
        (0x4020be45306eb3e4, 0x4020660dd67c8a61)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 1, 0.75, &cfg).unwrap()),
        (0x40258016c9863b1b, 0x401b3ddd1b6a757f)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 1, 1.0, &cfg).unwrap()),
        (0x4028aaaaaaaaaaab, 0x4016aaaaaaaaaaab)
    ); // t = 1
}

#[test]
fn xspline_vectors_s1_6() {
    // s1 = 1, s2 = 0
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(1.0, 2.0)
            },
            Point {
                s: 1.0,
                ..Point::new(5.0, 10.0)
            },
            Point {
                s: 0.0,
                ..Point::new(13.0, 6.0)
            },
            Point {
                s: 0.0,
                ..Point::new(17.0, 0.0)
            },
        ],
    };
    let cfg = Config::canonical();
    assert_eq!(
        pair(eval(&c, 1, 0.0, &cfg).unwrap()),
        (0x4016aaaaaaaaaaab, 0x4020000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 1, 0.25, &cfg).unwrap()),
        (0x401cffd26cf389cb, 0x40202d7c42effa4e)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 1, 0.5, &cfg).unwrap()),
        (0x402341bacf914c1c, 0x401d983759f22983)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 1, 0.75, &cfg).unwrap()),
        (0x40288327e3b94f46, 0x4019528e63f9f0da)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 1, 1.0, &cfg).unwrap()),
        (0x402a000000000000, 0x4018000000000000)
    ); // t = 1
}

#[test]
fn xspline_vectors_s1_7() {
    // s1 = -1, s2 = 0
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(1.0, 2.0)
            },
            Point {
                s: -1.0,
                ..Point::new(5.0, 10.0)
            },
            Point {
                s: 0.0,
                ..Point::new(13.0, 6.0)
            },
            Point {
                s: 0.0,
                ..Point::new(17.0, 0.0)
            },
        ],
    };
    let cfg = Config::canonical();
    assert_eq!(
        pair(eval(&c, 1, 0.0, &cfg).unwrap()),
        (0x4014000000000000, 0x4024000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 1, 0.25, &cfg).unwrap()),
        (0x402003dd38ff08b2, 0x40238a24b59d76d3)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 1, 0.5, &cfg).unwrap()),
        (0x4025400000000000, 0x4020400000000000)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 1, 0.75, &cfg).unwrap()),
        (0x402915c3e2fad15c, 0x4019c3e2fad15c3e)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 1, 1.0, &cfg).unwrap()),
        (0x402a000000000000, 0x4018000000000000)
    ); // t = 1
}

#[test]
fn xspline_vectors_s1_8() {
    // s1 = -1/2, s2 = -1/4
    let c = Contour {
        kind: Kind::XSpline {
            variant: XVariant::General,
        },
        closed: false,
        points: vec![
            Point {
                s: 0.0,
                ..Point::new(1.0, 2.0)
            },
            Point {
                s: -0.5,
                ..Point::new(5.0, 10.0)
            },
            Point {
                s: -0.25,
                ..Point::new(13.0, 6.0)
            },
            Point {
                s: 0.0,
                ..Point::new(17.0, 0.0)
            },
        ],
    };
    let cfg = Config::canonical();
    assert_eq!(
        pair(eval(&c, 1, 0.0, &cfg).unwrap()),
        (0x4014000000000000, 0x4024000000000000)
    ); // t = 0
    assert_eq!(
        pair(eval(&c, 1, 0.25, &cfg).unwrap()),
        (0x401b6bb25136bb25, 0x402384295d116728)
    ); // t = 1/4
    assert_eq!(
        pair(eval(&c, 1, 0.5, &cfg).unwrap()),
        (0x4022d6b5ad6b5ad7, 0x4020bdef7bdef7be)
    ); // t = 1/2
    assert_eq!(
        pair(eval(&c, 1, 0.75, &cfg).unwrap()),
        (0x40279843c9908fcb, 0x401b5e3c159196b9)
    ); // t = 3/4
    assert_eq!(
        pair(eval(&c, 1, 1.0, &cfg).unwrap()),
        (0x402a000000000000, 0x4018000000000000)
    ); // t = 1
}
