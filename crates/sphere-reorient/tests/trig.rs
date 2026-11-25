use sphere_reorient::trig::*;

fn bits(p: (f64, f64)) -> (u64, u64) {
    (p.0.to_bits(), p.1.to_bits())
}

#[test]
fn zero_turn_is_exact() {
    assert_eq!(
        bits(sin_cos_turns(0.0)),
        (0.0f64.to_bits(), 1.0f64.to_bits())
    );
}

#[test]
fn quarter_turn_is_exact() {
    assert_eq!(
        bits(sin_cos_turns(0.25)),
        (1.0f64.to_bits(), 0.0f64.to_bits())
    );
}

#[test]
fn half_turn_is_exact() {
    assert_eq!(
        bits(sin_cos_turns(0.5)),
        (0.0f64.to_bits(), (-1.0f64).to_bits())
    );
}

#[test]
fn three_quarter_turn_is_exact() {
    assert_eq!(
        bits(sin_cos_turns(0.75)),
        ((-1.0f64).to_bits(), 0.0f64.to_bits())
    );
}

#[test]
fn negative_quarter_turn_is_exact() {
    assert_eq!(sin_cos_turns(-0.25), (-1.0, 0.0));
}

#[test]
fn whole_turns_reduce_exactly() {
    assert_eq!(sin_cos_turns(1.0), (0.0, 1.0));
    assert_eq!(sin_cos_turns(-3.0), (0.0, 1.0));
    assert_eq!(sin_cos_turns(7.25), (1.0, 0.0));
    assert_eq!(sin_cos_turns(1000000.5), (0.0, -1.0));
}

#[test]
fn degrees_at_right_angles_are_exact() {
    assert_eq!(sin_cos_deg(90.0), (1.0, 0.0));
    assert_eq!(sin_cos_deg(180.0), (0.0, -1.0));
    assert_eq!(sin_cos_deg(270.0), (-1.0, 0.0));
    assert_eq!(sin_cos_deg(360.0), (0.0, 1.0));
    assert_eq!(sin_cos_deg(-90.0), (-1.0, 0.0));
}

#[test]
fn thirty_degrees_is_one_half() {
    let (s, c) = sin_cos_deg(30.0);
    assert!((s - 0.5).abs() < 1e-16);
    assert!((c - 0.866_025_403_784_438_6).abs() < 2e-16);
}

#[test]
fn forty_five_degrees_is_sqrt_half() {
    let (s, c) = sin_cos_deg(45.0);
    let h = 0.5f64.sqrt();
    assert!((s - h).abs() < 2e-16 && (c - h).abs() < 2e-16);
}

#[test]
fn matches_platform_sin_cos_over_a_sweep() {
    // Independent slow reference: the platform functions, used only inside the test.
    let mut worst: f64 = 0.0;
    for i in -2000..=2000 {
        let t = f64::from(i) / 777.0;
        let (s, c) = sin_cos_turns(t);
        let a = t * TAU;
        worst = worst.max((s - a.sin()).abs()).max((c - a.cos()).abs());
    }
    assert!(worst < 5e-13, "worst {worst}");
}

#[test]
fn matches_platform_on_reduced_range_tightly() {
    let mut worst: f64 = 0.0;
    for i in -1000..=1000 {
        let t = f64::from(i) / 4000.0;
        let (s, c) = sin_cos_turns(t);
        let a = t * TAU;
        worst = worst.max((s - a.sin()).abs()).max((c - a.cos()).abs());
    }
    assert!(worst < 5e-16, "worst {worst}");
}

#[test]
fn pythagoras_holds() {
    for i in 0..500 {
        let (s, c) = sin_cos_turns(f64::from(i) * 0.00123);
        assert!((s * s + c * c - 1.0).abs() < 4e-16);
    }
}

#[test]
fn non_finite_input_gives_nan() {
    assert!(sin_cos_turns(f64::NAN).0.is_nan());
    assert!(sin_cos_turns(f64::INFINITY).1.is_nan());
    assert!(sin_cos_deg(f64::NEG_INFINITY).0.is_nan());
}

#[test]
fn radians_wrapper_agrees() {
    let (s, c) = sin_cos_rad(HALF_PI);
    assert!((s - 1.0).abs() < 1e-15 && c.abs() < 1e-15);
    let (s, c) = sin_cos_rad(1.0);
    assert!((s - 1.0f64.sin()).abs() < 1e-15 && (c - 1.0f64.cos()).abs() < 1e-15);
}

#[test]
fn constants_are_consistent() {
    assert_eq!(TAU, HALF_PI * 4.0);
    assert!((TAU * INV_TAU - 1.0).abs() < 2e-16);
    assert!((TAN_EIGHTH_PI - (2.0f64.sqrt() - 1.0)).abs() < 2e-16);
}

#[test]
fn atan2_axes_and_diagonals_are_exact() {
    assert_eq!(atan2_turns(0.0, 1.0), 0.0);
    assert_eq!(atan2_turns(1.0, 0.0), 0.25);
    assert_eq!(atan2_turns(0.0, -1.0), 0.5);
    assert_eq!(atan2_turns(-1.0, 0.0), -0.25);
    assert_eq!(atan2_turns(1.0, 1.0), 0.125);
    assert_eq!(atan2_turns(1.0, -1.0), 0.375);
    assert_eq!(atan2_turns(-1.0, 1.0), -0.125);
    assert_eq!(atan2_turns(-1.0, -1.0), -0.375);
}

#[test]
fn atan2_zero_vector_is_zero() {
    assert_eq!(atan2_turns(0.0, 0.0), 0.0);
    assert_eq!(atan2_turns(-0.0, -0.0), 0.0);
}

#[test]
fn atan2_negative_axis_is_plus_half() {
    assert_eq!(atan2_turns(0.0, -5.0), 0.5);
    assert_eq!(atan2_turns(-0.0, -5.0), 0.5);
}

#[test]
fn atan2_is_scale_invariant_for_powers_of_two() {
    assert_eq!(
        atan2_turns(3.0, 4.0),
        atan2_turns(3.0 * 1024.0, 4.0 * 1024.0)
    );
    assert_eq!(atan2_turns(3.0, 4.0), atan2_turns(3.0 / 256.0, 4.0 / 256.0));
}

#[test]
fn atan2_three_four_five() {
    let t = atan2_turns(3.0, 4.0);
    assert!((t - 3.0f64.atan2(4.0) * INV_TAU).abs() < 1e-16);
}

#[test]
fn atan2_matches_platform_over_a_sweep() {
    let mut worst: f64 = 0.0;
    for i in -60..=60 {
        for j in -60..=60 {
            if i == 0 && j == 0 {
                continue;
            }
            let (y, x) = (f64::from(i) * 0.37, f64::from(j) * 0.91);
            let r = y.atan2(x) / TAU;
            worst = worst.max((atan2_turns(y, x) - r).abs());
        }
    }
    assert!(worst < 1e-15, "worst {worst}");
}

#[test]
fn atan2_inverts_own_sine_and_cosine() {
    // Independent reference built from the crate's own sin_cos_turns: find t with the direction of t.
    for k in 0..40 {
        let truth = -0.5 + f64::from(k) * 0.0253 + 0.001;
        let (s, c) = sin_cos_turns(truth);
        assert!((atan2_turns(s, c) - truth).abs() < 1e-15);
    }
}

#[test]
fn atan2_range_is_half_open() {
    for i in -50..=50 {
        for j in -50..=50 {
            let t = atan2_turns(f64::from(i), f64::from(j));
            assert!(t > -0.5 && t <= 0.5);
        }
    }
}

#[test]
fn atan2_nan_gives_nan() {
    assert!(atan2_turns(f64::NAN, 1.0).is_nan());
    assert!(atan2_turns(1.0, f64::NAN).is_nan());
}

#[test]
fn asin_special_values() {
    assert_eq!(asin_turns(0.0), 0.0);
    assert_eq!(asin_turns(1.0), 0.25);
    assert_eq!(asin_turns(-1.0), -0.25);
    assert_eq!(asin_turns(2.0), 0.25);
    assert_eq!(asin_turns(-7.0), -0.25);
    assert!(asin_turns(f64::NAN).is_nan());
}

#[test]
fn asin_one_half_is_a_twelfth() {
    assert!((asin_turns(0.5) - 1.0 / 12.0).abs() < 1e-16);
    assert!((asin_turns(-0.5) + 1.0 / 12.0).abs() < 1e-16);
}

#[test]
fn asin_matches_platform_over_a_sweep() {
    let mut worst: f64 = 0.0;
    for i in -1000..=1000 {
        let s = f64::from(i) / 1000.0;
        worst = worst.max((asin_turns(s) - s.asin() / TAU).abs());
    }
    assert!(worst < 2e-15, "worst {worst}");
}

#[test]
fn sin_then_asin_round_trips() {
    for i in -20..=20 {
        let t = f64::from(i) * 0.0125;
        let (s, _) = sin_cos_turns(t);
        assert!((asin_turns(s) - t).abs() < 2e-15);
    }
}

#[test]
fn results_are_deterministic() {
    let run = || {
        let mut h: u64 = 0xcbf29ce484222325;
        for i in 0..3000 {
            let t = f64::from(i) * 0.000731 - 1.0;
            let (s, c) = sin_cos_turns(t);
            let a = atan2_turns(s, c + 0.1);
            for v in [s, c, a, asin_turns(s)] {
                h = (h ^ v.to_bits()).wrapping_mul(0x100000001b3);
            }
        }
        h
    };
    assert_eq!(run(), run());
}

#[test]
fn pinned_bit_patterns() {
    assert_eq!(sin_cos_turns(0.1).0.to_bits(), PIN_SIN_TENTH);
    assert_eq!(sin_cos_turns(0.1).1.to_bits(), PIN_COS_TENTH);
    assert_eq!(atan2_turns(1.0, 3.0).to_bits(), PIN_ATAN_THIRD);
}

const PIN_SIN_TENTH: u64 = 4603469518431672926;
const PIN_COS_TENTH: u64 = 4605462196814083240;
const PIN_ATAN_THIRD: u64 = 4587540699137972719;
