//! The fixed cube root and the curve parameter solvers, against independent slow references.

use key_eval::solve::{cbrt, closed_form, coeffs, solve};
use key_eval::{NewtonStart, Solver};

fn xpoly(p1: f64, p2: f64, u: f64) -> f64 {
    // Bernstein form (independent of the power form used by the crate).
    let w = 1.0 - u;
    3.0 * w * w * u * p1 + 3.0 * w * u * u * p2 + u * u * u
}

fn reference(p1: f64, p2: f64, s: f64) -> f64 {
    // 200 bisections, then the midpoint: a slow independent root of x(u) = s on [0, 1].
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..200 {
        let m = 0.5 * (lo + hi);
        if xpoly(p1, p2, m) < s {
            lo = m;
        } else {
            hi = m;
        }
    }
    0.5 * (lo + hi)
}

fn lcg(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 11) as f64 / (1u64 << 53) as f64
}

#[test]
fn cbrt_exact_cubes() {
    for (x, r) in [
        (8.0, 2.0),
        (27.0, 3.0),
        (0.125, 0.5),
        (1.0, 1.0),
        (-8.0, -2.0),
        (64.0, 4.0),
        (0.001953125, 0.125),
        (343.0, 7.0),
        (-0.015625, -0.25),
    ] {
        assert_eq!(cbrt(x).to_bits(), f64::to_bits(r), "cbrt({x})");
    }
}

#[test]
fn cbrt_keeps_zero_sign_and_special_values() {
    assert_eq!(cbrt(0.0).to_bits(), 0.0f64.to_bits());
    assert_eq!(cbrt(-0.0).to_bits(), (-0.0f64).to_bits());
    assert!(cbrt(f64::NAN).is_nan());
    assert_eq!(cbrt(f64::INFINITY), f64::INFINITY);
    assert_eq!(cbrt(f64::NEG_INFINITY), f64::NEG_INFINITY);
}

#[test]
fn cbrt_close_to_the_library_over_many_magnitudes() {
    let mut st = 7u64;
    for i in 0..4000 {
        let m = lcg(&mut st) + 0.5;
        let e = (i % 400) - 200;
        let x = m * 2f64.powi(e);
        let r = cbrt(x);
        // powf(1/3) errs by about ln(x) / 3 * 1e-16 relative, so this tolerance is loose; the exact cubes and
        // the cube check below pin the accuracy of cbrt itself.
        let want = x.powf(1.0 / 3.0);
        assert!(
            (r - want).abs() <= 1e-13 * want,
            "x {x} got {r} want {want}"
        );
        assert!((cbrt(-x) + r).abs() == 0.0);
    }
}

#[test]
fn cbrt_extremes() {
    for x in [f64::MIN_POSITIVE, 5e-324, 1e-310, f64::MAX, 1e300] {
        let r = cbrt(x);
        let want = x.powf(1.0 / 3.0);
        assert!(
            (r - want).abs() <= 1e-13 * want,
            "x {x} got {r} want {want}"
        );
    }
}

#[test]
fn cbrt_is_a_cube_root_to_within_one_ulp_of_the_cube() {
    let mut st = 11u64;
    for _ in 0..1024 {
        let x = lcg(&mut st) * 90.0 + 1e-3;
        let r = cbrt(x);
        let cube = r * r * r;
        assert!((cube - x).abs() <= 4.0 * f64::EPSILON * x, "{x}");
    }
}

#[test]
fn coefficients_of_the_time_polynomial() {
    assert_eq!(coeffs(0.0, 1.0), (-2.0, 3.0, 0.0));
    assert_eq!(coeffs(0.5, 1.0), (-0.5, 0.0, 1.5));
    assert_eq!(coeffs(0.25, 0.75), (-0.5, 0.75, 0.75));
}

#[test]
fn closed_form_chord_case_is_exact() {
    // p1 = 1/3, p2 = 2/3 up to rounding: u equals s within 4 ulps (a and b vanish to rounding).
    let u = solve(Solver::Cardano { polish: 2 }, 1.0 / 3.0, 2.0 / 3.0, 0.375);
    assert!((u - 0.375).abs() <= 4.0 * f64::EPSILON);
}

#[test]
fn quadratic_branch_matches_reference() {
    // p1 = 2/3 and p2 = 1 give a = 2 - 3 + 1 = 0 in floating point, so the quadratic branch runs.
    let (p1, p2) = (2.0 / 3.0, 1.0);
    let co = coeffs(p1, p2);
    assert_eq!(co.0, 0.0);
    for s in [0.1, 0.25, 0.5, 0.75, 0.9] {
        let u = solve(Solver::Cardano { polish: 2 }, p1, p2, s);
        let want = reference(p1, p2, s);
        assert!((u - want).abs() < 1e-9, "s {s}: {u} vs {want}");
    }
}

#[test]
fn identity_curve_gives_the_query_back_for_every_solver() {
    let solvers = [
        Solver::Cardano { polish: 0 },
        Solver::Cardano { polish: 3 },
        Solver::Bisection { steps: 60 },
        Solver::Newton {
            steps: 6,
            start: NewtonStart::Chord,
        },
    ];
    for sv in solvers {
        for s in [0.125, 0.25, 0.5, 0.75, 0.875] {
            let u = solve(sv, 1.0 / 3.0, 2.0 / 3.0, s);
            assert!((u - s).abs() < 1e-12, "{sv:?} {s} {u}");
        }
    }
}

fn sweep(solver: Solver, lo: f64, hi: f64, tol: f64, seed: u64) {
    let mut st = seed;
    for _ in 0..1500 {
        let p1 = lo + (hi - lo) * lcg(&mut st);
        let p2 = lo + (hi - lo) * lcg(&mut st);
        let s = 0.02 + 0.96 * lcg(&mut st);
        let u = solve(solver, p1, p2, s);
        assert!((0.0..=1.0).contains(&u));
        let res = (xpoly(p1, p2, u) - s).abs();
        assert!(
            res <= tol,
            "{solver:?} p1 {p1} p2 {p2} s {s} u {u} residual {res}"
        );
    }
}

#[test]
fn cardano_residual_over_monotone_handles() {
    sweep(Solver::Cardano { polish: 2 }, 0.0, 1.0, 1e-10, 1);
}

#[test]
fn cardano_without_polish_residual_over_monotone_handles() {
    sweep(Solver::Cardano { polish: 0 }, 0.0, 1.0, 1e-6, 2);
}

#[test]
fn bisection_residual_over_monotone_handles() {
    sweep(Solver::Bisection { steps: 60 }, 0.0, 1.0, 1e-14, 3);
}

#[test]
fn bisection_residual_over_crossing_handles() {
    sweep(Solver::Bisection { steps: 60 }, -1.0, 2.0, 1e-14, 4);
}

#[test]
fn cardano_residual_over_crossing_handles() {
    sweep(Solver::Cardano { polish: 2 }, -1.0, 2.0, 1e-7, 5);
}

#[test]
fn newton_residual_over_monotone_handles() {
    sweep(
        Solver::Newton {
            steps: 40,
            start: NewtonStart::Chord,
        },
        0.2,
        0.8,
        1e-12,
        6,
    );
}

#[test]
fn closed_form_matches_bisection_root_on_monotone_handles() {
    let mut st = 99u64;
    for _ in 0..800 {
        let p1 = lcg(&mut st);
        let p2 = lcg(&mut st);
        let s = 0.05 + 0.9 * lcg(&mut st);
        let a = solve(Solver::Cardano { polish: 2 }, p1, p2, s);
        let b = reference(p1, p2, s);
        // x is monotone here; where x'(u) is tiny the root is ill conditioned, so compare through x.
        assert!((xpoly(p1, p2, a) - xpoly(p1, p2, b)).abs() < 1e-9);
    }
}

#[test]
fn bisection_zero_steps_returns_the_middle() {
    assert_eq!(solve(Solver::Bisection { steps: 0 }, 0.3, 0.7, 0.2), 0.5);
}

#[test]
fn bisection_one_step_halves_once() {
    // x(0.5) = 0.5 for p1 = 0.3, p2 = 0.7 (symmetric); s = 0.2 < x(0.5) so hi = 0.5, midpoint 0.25.
    assert_eq!(solve(Solver::Bisection { steps: 1 }, 0.3, 0.7, 0.2), 0.25);
    // s above x(0.5): lo = 0.5, midpoint 0.75.
    assert_eq!(solve(Solver::Bisection { steps: 1 }, 0.3, 0.7, 0.9), 0.75);
}

#[test]
fn newton_zero_steps_returns_the_start() {
    let h = |start| Solver::Newton { steps: 0, start };
    assert_eq!(solve(h(NewtonStart::Half), 0.3, 0.7, 0.2), 0.5);
    assert_eq!(solve(h(NewtonStart::Chord), 0.3, 0.7, 0.2), 0.2);
    assert_eq!(solve(h(NewtonStart::Fixed(0.8)), 0.3, 0.7, 0.2), 0.8);
    assert_eq!(solve(h(NewtonStart::Fixed(7.0)), 0.3, 0.7, 0.2), 1.0);
    assert_eq!(solve(h(NewtonStart::Fixed(-7.0)), 0.3, 0.7, 0.2), 0.0);
}

#[test]
fn newton_one_step_from_half_on_the_symmetric_curve_stays() {
    // x(0.5) = 0.5 exactly, so s = 0.5 is a fixed point of the first step.
    let u = solve(
        Solver::Newton {
            steps: 1,
            start: NewtonStart::Half,
        },
        0.25,
        0.75,
        0.5,
    );
    assert_eq!(u, 0.5);
}

#[test]
fn newton_stationary_start_stays_put() {
    // p1 = 0, p2 = 0: x = u^3 has x'(0) = 0, so the step is skipped and the start 0 stays.
    let u = solve(
        Solver::Newton {
            steps: 5,
            start: NewtonStart::Fixed(0.0),
        },
        0.0,
        0.0,
        0.5,
    );
    assert_eq!(u, 0.0);
}

#[test]
fn solvers_are_deterministic() {
    for sv in [
        Solver::Cardano { polish: 2 },
        Solver::Bisection { steps: 30 },
        Solver::Newton {
            steps: 5,
            start: NewtonStart::Half,
        },
    ] {
        let a = solve(sv, 0.37, 0.91, 0.43);
        let b = solve(sv, 0.37, 0.91, 0.43);
        assert_eq!(a.to_bits(), b.to_bits());
    }
}

#[test]
fn closed_form_quadratic_branch_without_polish_is_accurate() {
    // a = 0 exactly (p1 = 2/3, p2 = 1): the quadratic formula alone must give a small residual.
    for s in [0.125, 0.25, 0.5, 0.75, 0.875] {
        let u = solve(Solver::Cardano { polish: 0 }, 2.0 / 3.0, 1.0, s);
        assert!((xpoly(2.0 / 3.0, 1.0, u) - s).abs() < 1e-12, "s {s} u {u}");
    }
}

#[test]
fn closed_form_takes_the_smallest_root_when_several_lie_in_range() {
    // p1 = 2, p2 = -1: x(u) = 10 u^3 - 15 u^2 + 6 u is not monotone, and x(u) = 1/2 has three roots in
    // [0, 1] (one is exactly 1/2). The smallest, about 0.12, is taken.
    let u = solve(Solver::Cardano { polish: 2 }, 2.0, -1.0, 0.5);
    assert!(u < 0.2, "{u}");
    assert!((xpoly(2.0, -1.0, u) - 0.5).abs() < 1e-12);
}

#[test]
fn closed_form_triple_root_case() {
    // p1 = 1, p2 = 0: x(u) = 4 u^3 - 6 u^2 + 3 u = 1/2 + 4 (u - 1/2)^3, so s = 1/2 has the triple root 1/2.
    let u = solve(Solver::Cardano { polish: 0 }, 1.0, 0.0, 0.5);
    assert!((u - 0.5).abs() < 1e-5, "{u}");
}

#[test]
fn closed_form_near_a_quadratic_still_finds_the_root() {
    // The cubic coefficient is a rounding error away from zero (one third is not exact in f64), so the
    // closed form cancels away its digits; the solver must still return a point on the curve.
    let cases = [
        (0.0, 0.33333333333333326, 0.65625),
        (0.0, 0.3333333333333335, 0.5),
        (0.0, 0.33333333333333326, 0.3333333333333333),
        (0.6666666666666665, 1.0, 0.135),
        (0.6666666666666666, 0.9999999999999999, 0.65625),
        (0.3333333333333335, 0.666666666666667, 0.3333333333333333),
    ];
    for polish in [0u32, 2] {
        for (p1, p2, s) in cases {
            let u = solve(Solver::Cardano { polish }, p1, p2, s);
            assert!(
                (xpoly(p1, p2, u) - s).abs() < 1e-9,
                "polish {polish} p1 {p1} p2 {p2} s {s} gave u {u}"
            );
            assert!(
                (u - reference(p1, p2, s)).abs() < 1e-6,
                "p1 {p1} p2 {p2} s {s}"
            );
        }
    }
}

#[test]
fn closed_form_is_accurate_within_a_few_ulps_of_the_quadratic_case() {
    // Sweep handle times within a few ulps of the exact quadratic cases (thirds, zero).
    let steps = [-3i64, -2, -1, 0, 1, 2, 3];
    for &(b1, b2) in &[
        (0.0f64, 1.0 / 3.0),
        (1.0 / 3.0, 2.0 / 3.0),
        (2.0 / 3.0, 1.0),
    ] {
        for &d1 in &steps {
            for &d2 in &steps {
                let nudge = |x: f64, d: i64| {
                    let bits = x.to_bits() as i64 + d;
                    if x == 0.0 {
                        0.0
                    } else {
                        f64::from_bits(bits as u64)
                    }
                };
                let (p1, p2) = (nudge(b1, d1), nudge(b2, d2));
                for k in 1..32 {
                    let s = k as f64 / 32.0;
                    for polish in [0u32, 2] {
                        let u = solve(Solver::Cardano { polish }, p1, p2, s);
                        assert!(
                            (xpoly(p1, p2, u) - s).abs() < 1e-9,
                            "polish {polish} p1 {p1:e} p2 {p2:e} s {s} gave u {u}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn closed_form_alone_gives_the_exact_quadratic_roots() {
    // x(u) = u^2 exactly (a = 0, c = 0): the roots of u^2 = s are exact for dyadic squares. The closed
    // form is read directly, so the bisection fallback of `solve` cannot hide a fault in it.
    let co = coeffs(0.0, 1.0 / 3.0);
    assert_eq!(co, (0.0, 1.0, 0.0));
    for (s, root) in [(0.25, 0.5), (0.0625, 0.25), (0.5625, 0.75)] {
        let r = closed_form(co, s);
        assert_eq!(r[1], root, "s {s}");
        assert_eq!(r[0], -root, "s {s}");
    }
    // x(u) = 2 u^2 + u from p1 = 1/3, p2 = 1/2... a = 0 needs 3 p1 - 3 p2 + 1 = 0: p1 = 1/6, p2 = 1/2.
    let co = coeffs(1.0 / 6.0, 0.5);
    assert_eq!(co.0, 0.0);
    for s in [0.125, 0.3, 0.9] {
        let r = closed_form(co, s);
        for root in [r[0], r[1]] {
            let res = co.1 * root * root + co.2 * root - s;
            assert!(res.abs() < 1e-14, "s {s} root {root}");
        }
    }
}

#[test]
fn closed_form_alone_gives_three_real_roots() {
    // x(u) = 10 u^3 - 15 u^2 + 6 u has a local maximum 0.7236 and minimum 0.2764: between them every
    // level has three real roots, and each of the three slots must be one of them.
    let co = coeffs(2.0, -1.0);
    assert_eq!(co, (10.0, -15.0, 6.0));
    for s in [0.3, 0.4, 0.5, 0.6, 0.7] {
        let r = closed_form(co, s);
        for root in r {
            let res = ((10.0 * root - 15.0) * root + 6.0) * root - s;
            assert!(res.abs() < 1e-12, "s {s} root {root} residual {res}");
        }
        let mut sorted = r;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(
            sorted[1] - sorted[0] > 1e-3 && sorted[2] - sorted[1] > 1e-3,
            "distinct roots at s {s}"
        );
    }
}

#[test]
fn closed_form_alone_gives_the_single_real_root() {
    // Monotone cubic with a = 4: x(u) = 4 u^3 - 6 u^2 + 3 u (p1 = 1, p2 = 0), root of the level 0.5 is 0.5.
    let co = coeffs(1.0, 0.0);
    for s in [0.1, 0.5, 0.9] {
        let r = closed_form(co, s);
        let res = ((4.0 * r[0] - 6.0) * r[0] + 3.0) * r[0] - s;
        assert!(res.abs() < 1e-12, "s {s}");
        assert_eq!(r[0], r[2]);
    }
}
