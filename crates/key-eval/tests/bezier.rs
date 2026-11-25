//! Bezier segments, solvers, handle modes and clamping. Dyadic cases are worked by hand.
//!
//! Segment from time 0 to 16, values 0 to 8, absolute handles out (8, 2) and in (16, 6): the normalized
//! handle times are p1 = 1/2 and p2 = 1, so x(u) = -u^3/2 + 3 u / 2 and y(u) has control values 0, 2, 6, 8.
//! u = 1/2: x = 11/16, y = (3*2 + 3*6 + 8) / 8 = 4.
//! u = 1/4: x = 47/128, y = (27*2 + 9*6 + 8) / 64 = 116/64 = 29/16.
//! u = 3/4: x = 117/128, y = (9*2 + 27*6 + 27*8) / 64 = 396/64 = 6.1875.

use key_eval::{Clamp, Config, HandleMode, Key, KeyList, NewtonStart, Solver, VectorMode};

fn solvers() -> Vec<Solver> {
    vec![
        Solver::Cardano { polish: 0 },
        Solver::Cardano { polish: 2 },
        Solver::Bisection { steps: 60 },
        Solver::Newton {
            steps: 8,
            start: NewtonStart::Half,
        },
        Solver::Newton {
            steps: 8,
            start: NewtonStart::Chord,
        },
        Solver::Newton {
            steps: 8,
            start: NewtonStart::Fixed(0.9),
        },
    ]
}

fn seg_keys() -> Vec<Key<1>> {
    vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[8.0, 2.0]]),
        Key::bezier(16.0, [8.0], [[16.0, 6.0]], [[16.0, 8.0]]),
    ]
}

fn with_solver(s: Solver) -> Config {
    Config {
        solver: s,
        ..Config::default()
    }
}

fn eval(keys: Vec<Key<1>>, cfg: &Config, t: f64) -> f64 {
    KeyList::new(keys, cfg).unwrap().evaluate(cfg, t)[0]
}

fn close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "got {a}, want {b}");
}

macro_rules! dyadic {
    ($name:ident, $t:expr, $want:expr) => {
        #[test]
        fn $name() {
            for s in solvers() {
                let got = eval(seg_keys(), &with_solver(s), $t);
                close(got, $want, 1e-12);
            }
        }
    };
}

dyadic!(bezier_u_half, 11.0, 4.0);
dyadic!(bezier_u_quarter, 47.0 * 16.0 / 128.0, 1.8125);
// u = 3/4: x = 117/128, y = (1*0 + 9*2 + 27*6 + 27*8) / 64 = 396/64 = 6.1875.
dyadic!(bezier_u_three_quarter, 117.0 * 16.0 / 128.0, 6.1875);

#[test]
fn bezier_endpoints_are_the_key_values() {
    for s in solvers() {
        let c = with_solver(s);
        assert_eq!(eval(seg_keys(), &c, 0.0), 0.0);
        assert_eq!(eval(seg_keys(), &c, 16.0), 8.0);
    }
}

#[test]
fn bezier_offset_mode_equals_absolute_mode() {
    // out offset (8, 2), in offset (0, -2): the same points as the absolute case.
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[8.0, 2.0]]),
        Key::bezier(16.0, [8.0], [[0.0, -2.0]], [[0.0, 0.0]]),
    ];
    let c = Config {
        handle_mode: HandleMode::Offset,
        ..Config::default()
    };
    close(eval(keys, &c, 11.0), 4.0, 1e-12);
}

#[test]
fn bezier_offset_mode_with_shifted_segment() {
    // The same segment moved to times 96 to 112 and values 50 to 58 with offsets unchanged.
    let keys = vec![
        Key::bezier(96.0, [50.0], [[0.0, 0.0]], [[8.0, 2.0]]),
        Key::bezier(112.0, [58.0], [[0.0, -2.0]], [[0.0, 0.0]]),
    ];
    let c = Config {
        handle_mode: HandleMode::Offset,
        ..Config::default()
    };
    close(eval(keys, &c, 107.0), 54.0, 1e-12);
}

#[test]
fn bezier_absolute_mode_with_shifted_segment() {
    let keys = vec![
        Key::bezier(96.0, [50.0], [[0.0, 0.0]], [[104.0, 52.0]]),
        Key::bezier(112.0, [58.0], [[112.0, 56.0]], [[0.0, 0.0]]),
    ];
    close(eval(keys, &Config::default(), 107.0), 54.0, 1e-12);
}

#[test]
fn bezier_chord_handles_equal_linear() {
    // Handles on the chord at one third and two thirds: x(u) = u and y(u) = 8 u.
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[16.0 / 3.0, 8.0 / 3.0]]),
        Key::bezier(16.0, [8.0], [[32.0 / 3.0, 16.0 / 3.0]], [[0.0, 0.0]]),
    ];
    for s in solvers() {
        let c = with_solver(s);
        for t in [4.0, 8.0, 12.0, 1.0, 15.0] {
            close(eval(keys.clone(), &c, t), t / 2.0, 1e-12);
        }
    }
}

#[test]
fn bezier_chord_handles_exactly_linear_by_the_closed_form() {
    // Normalized p1 = 1/3 and p2 = 2/3 give a = 0 and b = 0 up to rounding; the result is within 1 ulp.
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[1.0, 1.0]]),
        Key::bezier(3.0, [3.0], [[2.0, 2.0]], [[0.0, 0.0]]),
    ];
    let c = Config::default();
    let got = eval(keys, &c, 1.5);
    close(got, 1.5, 4.0 * f64::EPSILON);
}

#[test]
fn bezier_handles_at_ends_give_smoothstep_line() {
    // Handles at the end keys' own times (flat start and end) with values on the chord give y = 8 x.
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[0.0, 0.0]]),
        Key::bezier(4.0, [8.0], [[4.0, 8.0]], [[4.0, 8.0]]),
    ];
    for t in [1.0, 2.0, 3.0] {
        close(eval(keys.clone(), &Config::default(), t), 2.0 * t, 1e-12);
    }
}

#[test]
fn bezier_flat_value_handles_make_smoothstep_in_value() {
    // Time handles on the chord, value handles flat: y(u) = 8 (3 u^2 - 2 u^3) with x(u) = 4 u.
    // t = 2: u = 1/2 gives 4. t = 1: u = 1/4 gives 8 * 5/32 = 1.25. t = 3 gives 6.75.
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[4.0 / 3.0, 0.0]]),
        Key::bezier(4.0, [8.0], [[8.0 / 3.0, 8.0]], [[0.0, 0.0]]),
    ];
    for s in solvers() {
        let c = with_solver(s);
        close(eval(keys.clone(), &c, 2.0), 4.0, 1e-12);
        close(eval(keys.clone(), &c, 1.0), 1.25, 1e-12);
        close(eval(keys.clone(), &c, 3.0), 6.75, 1e-12);
    }
}

#[test]
fn bezier_equals_hermite_with_matching_slopes() {
    // Hermite slopes m0 and m1 over L correspond to Bezier handles at L/3 with values va + m0 L/3 and
    // vb - m1 L/3 (Farin 2002, Hermite to Bezier conversion). Slopes 1 and -2 on values 1 and 3 over L = 6.
    let herm = vec![
        Key::hermite(0.0, [1.0], [0.0], [1.0]),
        Key::hermite(6.0, [3.0], [-2.0], [0.0]),
    ];
    let bez = vec![
        Key::bezier(0.0, [1.0], [[0.0, 0.0]], [[2.0, 3.0]]),
        Key::bezier(6.0, [3.0], [[4.0, 7.0]], [[0.0, 0.0]]),
    ];
    let c = Config::default();
    for i in 1..24 {
        let t = i as f64 / 4.0;
        close(eval(bez.clone(), &c, t), eval(herm.clone(), &c, t), 1e-12);
    }
}

#[test]
fn clamp_none_uses_handles_as_given() {
    // Out handle time 6 beyond the segment end 4, not clamped: p1 = 1.5, result differs from the clamped.
    let keys = clamp_keys(3.0);
    let none = eval(keys.clone(), &Config::default(), 3.5);
    let cl = eval(
        keys,
        &Config {
            clamp: Clamp::ClampTime,
            ..Config::default()
        },
        3.5,
    );
    assert!((none - cl).abs() > 1e-3, "{none} {cl}");
}

// Segment 0..4 values 0..8, out handle (6, v1), in handle (4, 8).
fn clamp_keys(v1: f64) -> Vec<Key<1>> {
    vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[6.0, v1]]),
        Key::bezier(4.0, [8.0], [[4.0, 8.0]], [[0.0, 0.0]]),
    ]
}

#[test]
fn clamp_time_pins_handle_time_to_segment_end() {
    // p1 = p2 = 1: x(u) = 1 - (1-u)^3; u = 1/2 gives x = 7/8, t = 3.5; y control 0, v1 = 3, 8, 8:
    // y = (0 + 3*3 + 3*8 + 8) / 8 = 5.125.
    let c = Config {
        clamp: Clamp::ClampTime,
        ..Config::default()
    };
    close(eval(clamp_keys(3.0), &c, 3.5), 5.125, 1e-12);
}

#[test]
fn clamp_scale_both_scales_value_offset_too() {
    // Factor 4/6 = 2/3: out handle becomes (4, 2); y = (0 + 3*2 + 24 + 8) / 8 = 4.75.
    let c = Config {
        clamp: Clamp::ScaleBoth,
        ..Config::default()
    };
    close(eval(clamp_keys(3.0), &c, 3.5), 4.75, 1e-12);
}

#[test]
fn clamp_modes_agree_when_handles_inside_the_segment() {
    let keys = seg_keys();
    let base = eval(keys.clone(), &Config::default(), 11.0);
    for cl in [Clamp::ClampTime, Clamp::ScaleBoth] {
        let c = Config {
            clamp: cl,
            ..Config::default()
        };
        assert_eq!(eval(keys.clone(), &c, 11.0).to_bits(), base.to_bits());
    }
}

#[test]
fn clamp_time_pins_negative_out_handle_to_key_time() {
    // out handle time -2 (before the segment) becomes 0: p1 = 0; with p2 = 1 this is the smoothstep
    // time curve and y control 0, 0, 8, 8 gives y = 8 x, so t = 8 on a 0..16 segment gives 4.
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[-2.0, 0.0]]),
        Key::bezier(16.0, [8.0], [[16.0, 8.0]], [[0.0, 0.0]]),
    ];
    let c = Config {
        clamp: Clamp::ClampTime,
        ..Config::default()
    };
    close(eval(keys, &c, 8.0), 4.0, 1e-12);
}

#[test]
fn clamp_scale_both_pins_oversized_in_handle() {
    // in handle time -8 on a segment 0..4 (extent 12 > 4): factor 1/3 on the in handle and the out handle.
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[0.0, 0.0]]),
        Key::bezier(4.0, [8.0], [[-8.0, 8.0]], [[0.0, 0.0]]),
    ];
    // After scaling by 4/12: in offset dt2 = -4, dv2 = 0; p2 = 0; p1 = 0: x(u) = u^3.
    // y control 0, 0, 8, 8: y = 8 (3 u^2 - 2 u^3). t = 0.5 (s = 1/8): u = 1/2, y = 4.
    let c = Config {
        clamp: Clamp::ScaleBoth,
        ..Config::default()
    };
    close(eval(keys, &c, 0.5), 4.0, 1e-12);
}

fn vec2_keys(time_handles_differ: bool) -> Vec<Key<2>> {
    let (o1, i1) = if time_handles_differ {
        (4.0, 8.0)
    } else {
        (8.0, 16.0)
    };
    vec![
        Key::bezier(
            0.0,
            [0.0, 0.0],
            [[0.0, 0.0], [0.0, 0.0]],
            [[8.0, 2.0], [o1, 2.0]],
        ),
        Key::bezier(
            16.0,
            [8.0, 8.0],
            [[16.0, 6.0], [i1, 6.0]],
            [[0.0, 0.0], [0.0, 0.0]],
        ),
    ]
}

#[test]
fn joint_mode_uses_component_zero_time_for_all_components() {
    let c = Config {
        vector_mode: VectorMode::Joint,
        ..Config::default()
    };
    let l = KeyList::new(vec2_keys(true), &c).unwrap();
    let v = l.evaluate(&c, 11.0);
    close(v[0], 4.0, 1e-12);
    close(v[1], 4.0, 1e-12);
}

#[test]
fn per_component_mode_uses_own_time_handles() {
    let c = Config::default();
    let l = KeyList::new(vec2_keys(true), &c).unwrap();
    let v = l.evaluate(&c, 11.0);
    close(v[0], 4.0, 1e-12);
    assert!((v[1] - 4.0).abs() > 1e-3, "{}", v[1]);
}

#[test]
fn both_vector_modes_agree_when_time_handles_match() {
    let a = Config::default();
    let b = Config {
        vector_mode: VectorMode::Joint,
        ..Config::default()
    };
    let la = KeyList::new(vec2_keys(false), &a).unwrap();
    let lb = KeyList::new(vec2_keys(false), &b).unwrap();
    for t in [1.0, 5.0, 11.0, 15.0] {
        assert_eq!(la.evaluate(&a, t), lb.evaluate(&b, t));
    }
}

#[test]
fn solvers_agree_with_each_other_on_a_generic_segment() {
    let keys = vec![
        Key::bezier(1.0, [-3.0], [[0.0, 0.0]], [[2.5, 4.0]]),
        Key::bezier(7.0, [5.0], [[5.0, -2.0]], [[0.0, 0.0]]),
    ];
    let reference = eval(
        keys.clone(),
        &with_solver(Solver::Bisection { steps: 60 }),
        3.3,
    );
    for s in solvers() {
        close(eval(keys.clone(), &with_solver(s), 3.3), reference, 1e-9);
    }
}

#[test]
fn bezier_three_real_roots_case_still_lands_in_the_segment() {
    // Handles that cross each other in time make x(u) non monotone (three real roots exist).
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[3.0, 0.0]]),
        Key::bezier(1.0, [1.0], [[-2.0, 1.0]], [[0.0, 0.0]]),
    ];
    for s in solvers() {
        let v = eval(keys.clone(), &with_solver(s), 0.5);
        assert!(v.is_finite() && (-0.5..=1.5).contains(&v), "{v}");
    }
}

#[test]
fn zero_length_handles_do_not_divide_by_zero() {
    // Newton from a fixed start of 0.9 is left out: it steps onto the stationary end (x'(1) = 0) here, which
    // is the stated behaviour of a fixed step Newton solver.
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[0.0, 0.0]]),
        Key::bezier(1.0, [1.0], [[1.0, 1.0]], [[0.0, 0.0]]),
    ];
    for s in solvers().into_iter().take(5) {
        let v = eval(keys.clone(), &with_solver(s), 0.5);
        close(v, 0.5, 1e-12);
    }
}

#[test]
fn clamp_scale_both_uses_the_more_restrictive_handle() {
    // Segment 0..4, values 0..8. Out handle (8, 4): extent 8; in handle (-2, 4): extent 6. The factor is
    // 4/8 = 1/2 (not 4/6), so out becomes (4, 2) and in becomes (1, 6): p1 = 1, p2 = 1/4.
    // x(1/2) = 3.25/8 - 5.25/4 + 3/2 = 19/32, so t = 4 * 19/32 = 2.375 and y(1/2) = (0 + 6 + 18 + 8) / 8 = 4.
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[8.0, 4.0]]),
        Key::bezier(4.0, [8.0], [[-2.0, 4.0]], [[0.0, 0.0]]),
    ];
    let c = Config {
        clamp: Clamp::ScaleBoth,
        ..Config::default()
    };
    close(eval(keys, &c, 2.375), 4.0, 1e-12);
}
