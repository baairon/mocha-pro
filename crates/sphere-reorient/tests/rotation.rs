use sphere_reorient::{Angle, AngleSign, AxisOrder, Config, Mat3, RotationUse};

fn close(a: &Mat3, b: &Mat3, tol: f64) -> bool {
    (0..3).all(|i| (0..3).all(|j| (a.0[i][j] - b.0[i][j]).abs() <= tol))
}

#[test]
fn zero_angles_give_identity_for_every_order() {
    for order in AxisOrder::ALL {
        let cfg = Config {
            axis_order: order,
            ..Config::default()
        };
        let m = Mat3::from_yaw_pitch_roll(&cfg, Angle::ZERO, Angle::ZERO, Angle::ZERO);
        assert_eq!(m, Mat3::IDENTITY);
    }
}

#[test]
fn yaw_quarter_matrix_is_exact() {
    let m = Mat3::about_y(Angle::from_degrees(90.0));
    assert_eq!(m.0, [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]]);
}

#[test]
fn pitch_quarter_matrix_is_exact() {
    let m = Mat3::about_x(Angle::from_degrees(90.0));
    assert_eq!(m.0, [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]]);
}

#[test]
fn roll_quarter_matrix_is_exact() {
    let m = Mat3::about_z(Angle::from_degrees(90.0));
    assert_eq!(m.0, [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
}

#[test]
fn half_turn_pitch_matrix_is_exact() {
    let m = Mat3::about_x(Angle::from_degrees(180.0));
    assert_eq!(m.0, [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]]);
}

#[test]
fn angle_constructors() {
    assert_eq!(Angle::new(0.6, 0.8), Angle { cos: 0.6, sin: 0.8 });
    assert_eq!(Angle::from_turns(0.25), Angle { cos: 0.0, sin: 1.0 });
    assert_eq!(Angle::from_degrees(0.0), Angle::ZERO);
}

#[test]
fn caller_supplied_pythagorean_pair_is_used_verbatim() {
    let a = Angle::new(0.6, 0.8);
    let m = Mat3::about_z(a);
    assert_eq!(m.0[0], [0.6, -0.8, 0.0]);
    assert_eq!(m.0[1], [0.8, 0.6, 0.0]);
}

#[test]
fn identity_multiplication_and_transpose() {
    let m = Mat3::about_x(Angle::from_degrees(30.0));
    assert_eq!(m.mul(&Mat3::IDENTITY), m);
    assert_eq!(Mat3::IDENTITY.mul(&m), m);
    assert_eq!(m.transpose().transpose(), m);
    assert_eq!(Mat3::IDENTITY.transpose(), Mat3::IDENTITY);
}

#[test]
fn product_is_not_commutative_and_order_is_right_to_left() {
    let y = Mat3::about_y(Angle::from_degrees(90.0));
    let x = Mat3::about_x(Angle::from_degrees(90.0));
    let v = [0.0, 0.0, 1.0];
    // y.mul(x): x acts first. x sends z to -y... (0,0,1) -> (0,-1,0); then y keeps y.
    assert_eq!(y.mul(&x).apply(v), [0.0, -1.0, 0.0]);
    // x.mul(y): y acts first: z -> (1,0,0); then x keeps x.
    assert_eq!(x.mul(&y).apply(v), [1.0, 0.0, 0.0]);
}

#[test]
fn apply_exact_vectors() {
    let y = Mat3::about_y(Angle::from_degrees(90.0));
    assert_eq!(y.apply([0.0, 0.0, 1.0]), [1.0, 0.0, 0.0]);
    assert_eq!(y.apply([1.0, 0.0, 0.0]), [0.0, 0.0, -1.0]);
    assert_eq!(y.apply([0.0, 1.0, 0.0]), [0.0, 1.0, 0.0]);
}

#[test]
fn determinant_is_one() {
    assert_eq!(Mat3::IDENTITY.det(), 1.0);
    assert_eq!(Mat3::about_y(Angle::from_degrees(90.0)).det(), 1.0);
    let cfg = Config::default();
    let m = Mat3::from_yaw_pitch_roll(
        &cfg,
        Angle::from_degrees(33.0),
        Angle::from_degrees(-47.0),
        Angle::from_degrees(121.0),
    );
    assert!((m.det() - 1.0).abs() < 1e-15);
}

#[test]
fn matrix_times_transpose_is_identity() {
    for order in AxisOrder::ALL {
        let cfg = Config {
            axis_order: order,
            ..Config::default()
        };
        let m = Mat3::from_yaw_pitch_roll(
            &cfg,
            Angle::from_degrees(33.0),
            Angle::from_degrees(-47.0),
            Angle::from_degrees(121.0),
        );
        assert!(close(&m.mul(&m.transpose()), &Mat3::IDENTITY, 2e-15));
    }
}

#[test]
fn negated_sign_gives_the_transpose_for_one_axis() {
    let pos = Config::default();
    let neg = Config {
        angle_sign: AngleSign::Negated,
        ..pos
    };
    let a = Angle::from_degrees(90.0);
    let p = Mat3::from_yaw_pitch_roll(&pos, a, Angle::ZERO, Angle::ZERO);
    let n = Mat3::from_yaw_pitch_roll(&neg, a, Angle::ZERO, Angle::ZERO);
    assert_eq!(n, p.transpose());
    assert_ne!(n, p);
}

#[test]
fn the_six_orders_are_distinct_for_generic_angles() {
    let (y, p, r) = (
        Angle::from_degrees(40.0),
        Angle::from_degrees(25.0),
        Angle::from_degrees(70.0),
    );
    let all: Vec<Mat3> = AxisOrder::ALL
        .iter()
        .map(|&o| {
            Mat3::from_yaw_pitch_roll(
                &Config {
                    axis_order: o,
                    ..Config::default()
                },
                y,
                p,
                r,
            )
        })
        .collect();
    for i in 0..6 {
        for j in (i + 1)..6 {
            assert_ne!(all[i], all[j], "{i} {j}");
        }
    }
}

#[test]
fn orders_match_hand_products() {
    let (y, p, r) = (
        Angle::from_degrees(90.0),
        Angle::from_degrees(90.0),
        Angle::from_degrees(90.0),
    );
    let (my, mp, mr) = (Mat3::about_y(y), Mat3::about_x(p), Mat3::about_z(r));
    let f = |o| {
        Mat3::from_yaw_pitch_roll(
            &Config {
                axis_order: o,
                ..Config::default()
            },
            y,
            p,
            r,
        )
    };
    assert_eq!(f(AxisOrder::YawPitchRoll), my.mul(&mp).mul(&mr));
    assert_eq!(f(AxisOrder::YawRollPitch), my.mul(&mr).mul(&mp));
    assert_eq!(f(AxisOrder::PitchYawRoll), mp.mul(&my).mul(&mr));
    assert_eq!(f(AxisOrder::PitchRollYaw), mp.mul(&mr).mul(&my));
    assert_eq!(f(AxisOrder::RollYawPitch), mr.mul(&my).mul(&mp));
    assert_eq!(f(AxisOrder::RollPitchYaw), mr.mul(&mp).mul(&my));
}

#[test]
fn for_use_selects_matrix_or_transpose() {
    let m = Mat3::about_y(Angle::from_degrees(90.0));
    let d = Config::default();
    let t = Config {
        rotation_use: RotationUse::Transposed,
        ..d
    };
    assert_eq!(m.for_use(&d), m);
    assert_eq!(m.for_use(&t), m.transpose());
}

#[test]
fn rotation_matches_slow_reference_formula() {
    // Rodrigues-free reference: rotate a vector about y by an angle using the plain 2D formula.
    let a = Angle::from_degrees(37.0);
    let v = [0.3, -0.2, 0.9];
    let r = Mat3::about_y(a).apply(v);
    let (c, s) = (a.cos, a.sin);
    let want = [c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2]];
    for i in 0..3 {
        assert!((r[i] - want[i]).abs() < 1e-15);
    }
}
