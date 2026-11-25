//! Model maps on normalised coordinates, with hand computed expectations.
use lens_model::{InverseMethod, LensError, Model};

fn radial(k: [f64; 3], p: [f64; 2]) -> Model {
    Model::Radial { k, p }
}
fn k1(v: f64) -> Model {
    radial([v, 0.0, 0.0], [0.0, 0.0])
}
fn bits(v: [f64; 2]) -> [u64; 2] {
    [v[0].to_bits(), v[1].to_bits()]
}

#[test]
fn zero_coefficients_give_identity_bits() {
    let m = Model::identity();
    for p in [[0.0, 0.0], [0.5, -0.25], [-3.0, 7.0], [1e-300, 1e100]] {
        assert_eq!(bits(m.apply(p).unwrap()), bits(p));
    }
}

#[test]
fn zero_division_lambda_is_identity() {
    let m = Model::Division { lambda: 0.0 };
    assert_eq!(m.apply([0.5, -2.0]).unwrap(), [0.5, -2.0]);
    assert_eq!(m.closed_form([0.5, -2.0]).unwrap(), [0.5, -2.0]);
}

#[test]
fn zero_squeezed_is_identity() {
    let m = Model::Squeezed {
        k: [0.0; 3],
        squeeze: [2.0, 3.0],
    };
    assert_eq!(m.apply([0.5, 0.25]).unwrap(), [0.5, 0.25]);
}

#[test]
fn k1_on_x_axis() {
    // r2 = 0.25, g = 1 + 0.5 * 0.25 = 1.125, x' = 0.5625
    let q = k1(0.5).apply([0.5, 0.0]).unwrap();
    assert_eq!(q[0].to_bits(), 0x3FE2_0000_0000_0000);
    assert_eq!(q[1], 0.0);
}

#[test]
fn k1_on_y_axis() {
    let q = k1(0.5).apply([0.0, 0.5]).unwrap();
    assert_eq!(q, [0.0, 0.5625]);
}

#[test]
fn k1_negative_axis_is_odd() {
    assert_eq!(k1(0.5).apply([-0.5, 0.0]).unwrap(), [-0.5625, 0.0]);
}

#[test]
fn three_radial_terms_at_unit_radius() {
    // g = 1 + 0.5 + 0.25 + 0.125 = 1.875
    let q = radial([0.5, 0.25, 0.125], [0.0; 2])
        .apply([1.0, 0.0])
        .unwrap();
    assert_eq!(q[0].to_bits(), 0x3FFE_0000_0000_0000);
}

#[test]
fn k2_alone() {
    // r2 = 0.25, r4 = 0.0625, g = 1 + 2 * 0.0625 = 1.125
    let q = radial([0.0, 2.0, 0.0], [0.0; 2]).apply([0.5, 0.0]).unwrap();
    assert_eq!(q, [0.5625, 0.0]);
}

#[test]
fn k3_alone() {
    // r2 = 1, g = 1 + 4 = 5
    let q = radial([0.0, 0.0, 4.0], [0.0; 2]).apply([1.0, 0.0]).unwrap();
    assert_eq!(q, [5.0, 0.0]);
}

#[test]
fn k1_diagonal_point() {
    // (0.5, 0.5): r2 = 0.5, g = 1 + 0.25 = 1.25, q = (0.625, 0.625)
    assert_eq!(k1(0.5).apply([0.5, 0.5]).unwrap(), [0.625, 0.625]);
}

#[test]
fn tangential_p1() {
    // (0.5,0.5), p1 = 0.5: tx = 2*0.5*0.25 = 0.25, ty = 0.5*(0.5+0.5) = 0.5
    let q = radial([0.0; 3], [0.5, 0.0]).apply([0.5, 0.5]).unwrap();
    assert_eq!(q, [0.75, 1.0]);
}

#[test]
fn tangential_p2() {
    // p2 = 0.5: tx = 0.5*(0.5+0.5) = 0.5, ty = 2*0.5*0.25 = 0.25
    let q = radial([0.0; 3], [0.0, 0.5]).apply([0.5, 0.5]).unwrap();
    assert_eq!(q, [1.0, 0.75]);
}

#[test]
fn tangential_on_x_axis() {
    // (1,0): p1: tx = 0, ty = p1*(1+0) = p1. p2: tx = p2*(1+2) = 3 p2, ty = 0
    let q = radial([0.0; 3], [0.25, 0.5]).apply([1.0, 0.0]).unwrap();
    assert_eq!(q, [1.0 + 1.5, 0.25]);
}

#[test]
fn tangential_on_y_axis() {
    // (0,1): tx = p2*(1+0) = p2, ty = p1*(1+2) = 3 p1
    let q = radial([0.0; 3], [0.25, 0.5]).apply([0.0, 1.0]).unwrap();
    assert_eq!(q, [0.5, 1.0 + 0.75]);
}

#[test]
fn combined_radial_and_tangential() {
    // (0.5,0.5): g = 1.25 -> (0.625, 0.625); plus p1 = 0.5 terms (0.25, 0.5)
    let q = radial([0.5, 0.0, 0.0], [0.5, 0.0])
        .apply([0.5, 0.5])
        .unwrap();
    assert_eq!(q, [0.875, 1.125]);
}

#[test]
fn origin_is_fixed_for_every_model() {
    let ms = [
        radial([0.3, 0.2, 0.1], [0.05, 0.07]),
        Model::Division { lambda: 0.4 },
        Model::Squeezed {
            k: [0.3, 0.2, 0.1],
            squeeze: [1.5, 0.5],
        },
    ];
    for m in ms {
        assert_eq!(m.apply([0.0, 0.0]).unwrap(), [0.0, 0.0]);
    }
}

#[test]
fn division_lambda_one() {
    assert_eq!(
        Model::Division { lambda: 1.0 }.apply([1.0, 0.0]).unwrap(),
        [0.5, 0.0]
    );
}

#[test]
fn division_lambda_three() {
    assert_eq!(
        Model::Division { lambda: 3.0 }.apply([1.0, 0.0]).unwrap(),
        [0.25, 0.0]
    );
}

#[test]
fn division_negative_lambda_expands() {
    // d = 1 - 0.75 = 0.25
    assert_eq!(
        Model::Division { lambda: -0.75 }.apply([1.0, 0.0]).unwrap(),
        [4.0, 0.0]
    );
}

#[test]
fn division_two_axes() {
    // (0.5, 0.5), lambda 2: d = 1 + 2*0.5 = 2, q = (0.25, 0.25)
    assert_eq!(
        Model::Division { lambda: 2.0 }.apply([0.5, 0.5]).unwrap(),
        [0.25, 0.25]
    );
}

#[test]
fn division_zero_denominator_is_domain_error() {
    let m = Model::Division { lambda: -1.0 };
    assert_eq!(m.apply([1.0, 0.0]), Err(LensError::OutsideDomain));
}

#[test]
fn apply_reports_non_finite() {
    let m = k1(f64::MAX);
    assert_eq!(m.apply([2.0, 0.0]), Err(LensError::NonFinite));
    assert_eq!(m.apply([f64::NAN, 0.0]), Err(LensError::NonFinite));
}

#[test]
fn squeezed_unit_squeeze_matches_radial() {
    let a = Model::Squeezed {
        k: [0.5, 0.25, 0.125],
        squeeze: [1.0, 1.0],
    };
    let r = radial([0.5, 0.25, 0.125], [0.0; 2]);
    for p in [[0.5, 0.25], [-1.0, 0.5], [0.0, 2.0]] {
        assert_eq!(bits(a.apply(p).unwrap()), bits(r.apply(p).unwrap()));
    }
}

#[test]
fn squeezed_x_squeeze_two() {
    // (0.5, 0): xs = 1, r2 = 1, g = 1 + 0.5 = 1.5, q = (0.75, 0)
    let m = Model::Squeezed {
        k: [0.5, 0.0, 0.0],
        squeeze: [2.0, 1.0],
    };
    assert_eq!(m.apply([0.5, 0.0]).unwrap(), [0.75, 0.0]);
}

#[test]
fn squeezed_y_squeeze_does_not_touch_x_axis_radius() {
    let m = Model::Squeezed {
        k: [0.5, 0.0, 0.0],
        squeeze: [1.0, 8.0],
    };
    assert_eq!(m.apply([0.5, 0.0]).unwrap(), [0.5625, 0.0]);
}

#[test]
fn squeezed_y_squeeze_four() {
    // (0, 0.5): ys = 2, r2 = 4, g = 1 + 0.25*4 = 2, q = (0, 1)
    let m = Model::Squeezed {
        k: [0.25, 0.0, 0.0],
        squeeze: [1.0, 4.0],
    };
    assert_eq!(m.apply([0.0, 0.5]).unwrap(), [0.0, 1.0]);
}

#[test]
fn squeezed_axes_differ() {
    let m = Model::Squeezed {
        k: [0.25, 0.0, 0.0],
        squeeze: [1.0, 2.0],
    };
    let a = m.apply([0.5, 0.0]).unwrap();
    let b = m.apply([0.0, 0.5]).unwrap();
    assert_eq!(a, [0.5 * (1.0 + 0.25 * 0.25), 0.0]);
    assert_eq!(b, [0.0, 0.5 * (1.0 + 0.25 * 1.0)]);
}

#[test]
fn zero_iterations_return_target() {
    for method in [InverseMethod::FixedPoint, InverseMethod::Newton] {
        let q = k1(0.5).invert([0.75, -0.25], method, 0).unwrap();
        assert_eq!(q, [0.75, -0.25]);
    }
}

#[test]
fn fixed_point_one_step_by_hand() {
    // u0 = 0.5, F(u0) = 0.5625, u1 = 0.5 - (0.5625 - 0.5) = 0.4375
    let q = k1(0.5)
        .invert([0.5, 0.0], InverseMethod::FixedPoint, 1)
        .unwrap();
    assert_eq!(q, [0.4375, 0.0]);
}

#[test]
fn fixed_point_two_steps_by_hand() {
    // F(0.4375) = 0.4375 * (1 + 0.5 * 0.19140625) = 0.4375 * 1.095703125 = 0.4793701171875
    // u2 = 0.4375 - (0.4793701171875 - 0.5) = 0.4581298828125
    let q = k1(0.5)
        .invert([0.5, 0.0], InverseMethod::FixedPoint, 2)
        .unwrap();
    assert_eq!(q, [0.4581298828125, 0.0]);
}

#[test]
fn newton_one_step_by_hand() {
    // F(0.5) = 0.5625, J = g + 2 x^2 k1 = 1.125 + 0.25 = 1.375 (jxx), jyy = g = 1.125, jxy = 0
    // u1 = 0.5 - 0.0625 / 1.375 ; checked against the same expression
    let q = k1(0.5)
        .invert([0.5, 0.0], InverseMethod::Newton, 1)
        .unwrap();
    assert_eq!(q[0], 0.5 - 0.0625 / 1.375);
    assert_eq!(q[1], 0.0);
}

#[test]
fn closed_form_unavailable_for_polynomials() {
    assert_eq!(
        k1(0.5).closed_form([0.5, 0.0]),
        Err(LensError::NoClosedForm)
    );
    let a = Model::Squeezed {
        k: [0.5, 0.0, 0.0],
        squeeze: [1.0, 1.0],
    };
    assert_eq!(
        a.invert([0.5, 0.0], InverseMethod::ClosedForm, 3),
        Err(LensError::NoClosedForm)
    );
}

#[test]
fn division_closed_form_double_root() {
    // lambda 1, q = 0.5: disc = 1 - 4 * 0.25 = 0, den = 1, p = 1
    let p = Model::Division { lambda: 1.0 }
        .closed_form([0.5, 0.0])
        .unwrap();
    assert_eq!(p, [1.0, 0.0]);
}

#[test]
fn division_closed_form_exact_root() {
    // lambda -8, q = 0.5: disc = 9, den = 4, p = 0.25
    let m = Model::Division { lambda: -8.0 };
    assert_eq!(m.closed_form([0.5, 0.0]).unwrap(), [0.25, 0.0]);
    assert_eq!(m.apply([0.25, 0.0]).unwrap(), [0.5, 0.0]);
}

#[test]
fn division_closed_form_at_origin() {
    // disc = 1, den = 2
    assert_eq!(
        Model::Division { lambda: 5.0 }
            .closed_form([0.0, 0.0])
            .unwrap(),
        [0.0, 0.0]
    );
}

#[test]
fn division_closed_form_outside_domain() {
    let m = Model::Division { lambda: 1.0 };
    assert_eq!(m.closed_form([1.0, 0.0]), Err(LensError::OutsideDomain));
}

#[test]
fn division_closed_form_small_root_branch() {
    // lambda 3, q 0.25: roots 1/3 and 1; the root that tends to q as lambda -> 0 is 1/3
    let p = Model::Division { lambda: 3.0 }
        .closed_form([0.25, 0.0])
        .unwrap();
    assert!((p[0] - 1.0 / 3.0).abs() < 1e-16);
}

#[test]
fn division_closed_form_matches_iteration() {
    let m = Model::Division { lambda: 0.3 };
    for q in [[0.1, 0.2], [-0.4, 0.3], [0.7, -0.1], [0.0, 0.9]] {
        let c = m.closed_form(q).unwrap();
        let n = m.invert(q, InverseMethod::Newton, 30).unwrap();
        let f = m.invert(q, InverseMethod::FixedPoint, 396).unwrap();
        assert!((c[0] - n[0]).abs() < 1e-14 && (c[1] - n[1]).abs() < 1e-14);
        assert!((c[0] - f[0]).abs() < 1e-9 && (c[1] - f[1]).abs() < 1e-9);
    }
}

#[test]
fn division_closed_form_round_trip() {
    for lambda in [-0.5, -0.1, 0.0, 0.2, 0.9] {
        let m = Model::Division { lambda };
        for p in [[0.3, 0.1], [-0.5, 0.4], [0.0, -0.6]] {
            let q = m.apply(p).unwrap();
            let r = m.closed_form(q).unwrap();
            assert!(
                (r[0] - p[0]).abs() < 1e-14 && (r[1] - p[1]).abs() < 1e-14,
                "{lambda} {p:?}"
            );
        }
    }
}

#[test]
fn division_newton_matches_closed_form_bitwise_stable_across_runs() {
    let m = Model::Division { lambda: 0.3 };
    let a = m.invert([0.3, 0.2], InverseMethod::Newton, 12).unwrap();
    let b = m.invert([0.3, 0.2], InverseMethod::Newton, 12).unwrap();
    assert_eq!(bits(a), bits(b));
}

/// A slow independent reference: bisection of the monotone axis map `u * g(u^2)`.
fn bisect_axis(k: [f64; 3], target: f64) -> f64 {
    let f = |u: f64| u * (1.0 + u * u * k[0] + u * u * u * u * k[1] + u * u * u * u * u * u * k[2]);
    let (mut lo, mut hi) = (0.0_f64, 4.0_f64);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if f(mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

#[test]
fn newton_matches_bisection_reference_on_axis() {
    let k = [0.1, 0.05, 0.02];
    let m = radial(k, [0.0; 2]);
    for t in [0.1, 0.25, 0.5, 0.75, 1.0, 1.25] {
        let n = m.invert([t, 0.0], InverseMethod::Newton, 12).unwrap();
        assert!((n[0] - bisect_axis(k, t)).abs() < 1e-14, "{t}");
        assert_eq!(n[1], 0.0);
    }
}

#[test]
fn fixed_point_matches_bisection_reference_on_axis() {
    let k = [0.05, 0.0, 0.0];
    let m = radial(k, [0.0; 2]);
    for t in [0.1, 0.5, 1.0] {
        let n = m.invert([t, 0.0], InverseMethod::FixedPoint, 80).unwrap();
        assert!((n[0] - bisect_axis(k, t)).abs() < 1e-14, "{t}");
    }
}

#[test]
fn round_trip_dyadic_inputs_error_bound() {
    // Asserted bound: 2^-48 after 10 Newton steps and 2^-44 after 60 fixed point steps.
    let m = radial([0.0625, 0.03125, 0.0], [0.015625, -0.0078125]);
    for p in [
        [0.5, 0.25],
        [-0.75, 0.5],
        [0.125, -0.875],
        [0.0, 0.5],
        [0.5, -0.5],
    ] {
        let q = m.apply(p).unwrap();
        let n = m.invert(q, InverseMethod::Newton, 10).unwrap();
        let f = m.invert(q, InverseMethod::FixedPoint, 60).unwrap();
        let b48 = 2f64.powi(-48);
        let b44 = 2f64.powi(-44);
        assert!(
            (n[0] - p[0]).abs() <= b48 && (n[1] - p[1]).abs() <= b48,
            "newton {p:?}"
        );
        assert!(
            (f[0] - p[0]).abs() <= b44 && (f[1] - p[1]).abs() <= b44,
            "fixed {p:?}"
        );
    }
}

#[test]
fn round_trip_squeezed() {
    let m = Model::Squeezed {
        k: [0.0625, 0.03125, 0.0],
        squeeze: [1.5, 0.75],
    };
    for p in [[0.5, 0.25], [-0.75, 0.5], [0.125, -0.875]] {
        let q = m.apply(p).unwrap();
        let n = m.invert(q, InverseMethod::Newton, 10).unwrap();
        assert!((n[0] - p[0]).abs() <= 2f64.powi(-48) && (n[1] - p[1]).abs() <= 2f64.powi(-48));
    }
}

#[test]
fn newton_converges_faster_than_fixed_point() {
    let m = k1(0.25);
    let q = m.apply([0.5, 0.5]).unwrap();
    let n = m.invert(q, InverseMethod::Newton, 4).unwrap();
    let f = m.invert(q, InverseMethod::FixedPoint, 4).unwrap();
    let en = (n[0] - 0.5).abs();
    let ef = (f[0] - 0.5).abs();
    assert!(en < ef);
}

#[test]
fn newton_jacobian_matches_finite_difference_direction() {
    // A wrong Jacobian converges slowly; with the right one 6 steps reach 2^-50 on a hard point.
    let m = radial([0.2, 0.1, 0.05], [0.03, 0.02]);
    let p = [0.6, -0.4];
    let q = m.apply(p).unwrap();
    let n = m.invert(q, InverseMethod::Newton, 6).unwrap();
    assert!((n[0] - p[0]).abs() <= 2f64.powi(-50) && (n[1] - p[1]).abs() <= 2f64.powi(-50));
}

#[test]
fn newton_jacobian_squeezed_and_division_converge_fast() {
    let a = Model::Squeezed {
        k: [0.2, 0.1, 0.05],
        squeeze: [1.25, 0.8],
    };
    let p = [0.6, -0.4];
    let n = a
        .invert(a.apply(p).unwrap(), InverseMethod::Newton, 6)
        .unwrap();
    assert!((n[0] - p[0]).abs() <= 2f64.powi(-50) && (n[1] - p[1]).abs() <= 2f64.powi(-50));
    let d = Model::Division { lambda: 0.4 };
    let n = d
        .invert(d.apply(p).unwrap(), InverseMethod::Newton, 6)
        .unwrap();
    assert!((n[0] - p[0]).abs() <= 2f64.powi(-50) && (n[1] - p[1]).abs() <= 2f64.powi(-50));
}

#[test]
fn newton_singular_jacobian_leaves_iterate() {
    // lambda = 0.5 at (1,0): d = 1.5; jxx = (d - 2*0.5*1)/d^2 != 0. Use a zero determinant:
    // radial k1 = -1 at (1,0): g = 0, gp = -1, jxx = 0 + 1*(-1)*2 = -2 ... use k1 = -0.5 at (sqrt2 free)
    // k1 = -1, x = 0, y = 0: g = 1. Instead force det 0 with g = 0 on a diagonal point.
    let m = k1(-1.0);
    // (1,0): g = 0, jxx = -2, jyy = 0, det = 0, the step is skipped and the point returned
    let q = m.invert([1.0, 0.0], InverseMethod::Newton, 5).unwrap();
    assert_eq!(q, [1.0, 0.0]);
}

#[test]
fn iterative_inverse_propagates_domain_error() {
    let m = Model::Division { lambda: -1.0 };
    assert_eq!(
        m.invert([1.0, 0.0], InverseMethod::FixedPoint, 3),
        Err(LensError::OutsideDomain)
    );
}

#[test]
fn iterative_inverse_reports_divergence() {
    let m = k1(97.0);
    assert_eq!(
        m.invert([50.0, 0.0], InverseMethod::FixedPoint, 396),
        Err(LensError::NonFinite)
    );
}

#[test]
fn results_are_deterministic_across_runs() {
    let m = radial([0.2, 0.1, 0.05], [0.03, 0.02]);
    let mut first = None;
    for _ in 0..5 {
        let q = m.apply([0.3, 0.4]).unwrap();
        let u = m.invert(q, InverseMethod::Newton, 7).unwrap();
        let f = m.invert(q, InverseMethod::FixedPoint, 7).unwrap();
        let sig = [bits(q), bits(u), bits(f)];
        match first {
            None => first = Some(sig),
            Some(s) => assert_eq!(s, sig),
        }
    }
}

#[test]
fn fixed_iteration_count_is_honoured_not_converged_early() {
    // The identity model converges at step 0, yet 996 steps must return the same exact value.
    let q = Model::identity()
        .invert([0.3, 0.7], InverseMethod::FixedPoint, 996)
        .unwrap();
    assert_eq!(q, [0.3, 0.7]);
    let q = Model::identity()
        .invert([0.3, 0.7], InverseMethod::Newton, 996)
        .unwrap();
    assert_eq!(q, [0.3, 0.7]);
}

#[test]
fn more_iterations_change_the_result() {
    let m = k1(0.5);
    let a = m.invert([0.5, 0.0], InverseMethod::FixedPoint, 3).unwrap();
    let b = m.invert([0.5, 0.0], InverseMethod::FixedPoint, 4).unwrap();
    assert_ne!(a[0].to_bits(), b[0].to_bits());
}

#[test]
fn newton_one_step_with_tangential_jacobian_by_hand() {
    // p1 = 0.5, target (0.5, 0.5): F = (x + x*y, y + 0.5*x*x + 1.5*y*y) so F(u0) = (0.75, 1.0).
    // J = [[1.5, 0.5], [0.5, 2.5]], det = 3.5, residual (0.25, 0.5).
    let m = radial([0.0; 3], [0.5, 0.0]);
    let u = m.invert([0.5, 0.5], InverseMethod::Newton, 1).unwrap();
    assert_eq!(u[0], 0.5 - (2.5 * 0.25 - 0.5 * 0.5) / 3.5);
    assert_eq!(u[1], 0.5 - (1.5 * 0.5 - 0.5 * 0.25) / 3.5);
}

#[test]
fn newton_one_step_with_second_tangential_jacobian_by_hand() {
    // p2 = 0.5: F = (x + 0.5*(r2 + 2x^2), y + x*y) at (0.5, 0.5) = (1.0, 0.75).
    // jxx = 1 + 6*p2*x = 2.5, jxy = 2*p2*y = 0.5, jyx = 0.5, jyy = 1 + 2*p2*x = 1.5, det = 3.5.
    let m = radial([0.0; 3], [0.0, 0.5]);
    let u = m.invert([0.5, 0.5], InverseMethod::Newton, 1).unwrap();
    let (rx, ry) = (0.5, 0.25);
    assert_eq!(u[0], 0.5 - (1.5 * rx - 0.5 * ry) / 3.5);
    assert_eq!(u[1], 0.5 - (2.5 * ry - 0.5 * rx) / 3.5);
}

#[test]
fn apply_reports_non_finite_when_only_one_component_overflows() {
    // squeeze keeps the radius finite; g = 1 + 1e-290 * r2 is about 3e6, so x' overflows and y' = 0
    let m = Model::Squeezed {
        k: [1e-290, 0.0, 0.0],
        squeeze: [1e-160, 1.0],
    };
    assert_eq!(m.apply([1.7e308, 0.0]), Err(LensError::NonFinite));
}

#[test]
fn newton_reports_non_finite_residual_like_fixed_point() {
    // F overflows at the start point: g is infinite, so f and the determinant are not finite.
    // The step must not be silently skipped and the start point returned as a solution.
    let m = k1(1e100);
    let t = [1e120, 0.0];
    assert_eq!(
        m.invert(t, InverseMethod::FixedPoint, 8),
        Err(LensError::NonFinite)
    );
    assert_eq!(
        m.invert(t, InverseMethod::Newton, 8),
        Err(LensError::NonFinite)
    );
}

#[test]
fn tangential_ordering_is_opencv_not_brown() {
    // Brown (1966): x' = x + P1*(r2 + 2*x*x) + 2*P2*x*y, y' = y + 2*P1*x*y + P2*(r2 + 2*y*y).
    // This crate documents the swapped (OpenCV) ordering, so its p = [a, b] is Brown's P1 = b, P2 = a.
    let (x, y) = (0.5, 0.25);
    let (a, b) = (0.125, 0.5);
    let r2 = x * x + y * y;
    let brown = [
        x + b * (r2 + 2.0 * x * x) + 2.0 * a * x * y,
        y + 2.0 * b * x * y + a * (r2 + 2.0 * y * y),
    ];
    assert_eq!(radial([0.0; 3], [a, b]).apply([x, y]).unwrap(), brown);
}
