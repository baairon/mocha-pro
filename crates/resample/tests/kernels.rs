use resample::kernel::{
    bilinear, floor, keys, lanczos, mitchell, sinc, SINC_MAX_ERROR, SINC_TERMS,
};
use std::f64::consts::PI;
mod common;
use common::{ref_lanczos, ref_sinc};

fn grid(lim: f64, step: f64) -> Vec<f64> {
    let n = (lim / step) as i64;
    (-n..=n).map(|i| i as f64 * step).collect()
}

#[test]
fn sinc_at_zero_is_one() {
    assert_eq!(sinc(0.0), 1.0);
    assert_eq!(sinc(-0.0), 1.0);
}

#[test]
fn sinc_exact_zero_at_nonzero_integers() {
    for n in 1..=12 {
        assert_eq!(sinc(f64::from(n)), 0.0, "n {n}");
        assert_eq!(sinc(-f64::from(n)), 0.0, "n -{n}");
    }
}

#[test]
fn sinc_half_is_two_over_pi() {
    assert!((sinc(0.5) - 2.0 / PI).abs() < 1e-13);
}

#[test]
fn sinc_three_halves_is_minus_two_over_three_pi() {
    assert!((sinc(1.5) + 2.0 / (3.0 * PI)).abs() < 1e-13);
}

#[test]
fn sinc_quarter_closed_form() {
    let want = 2.0 * 2.0f64.sqrt() / PI;
    assert!((sinc(0.25) - want).abs() < 1e-13);
}

#[test]
fn sinc_is_even_bitwise() {
    for x in grid(9.0, 1.0 / 64.0) {
        assert_eq!(sinc(x).to_bits(), sinc(-x).to_bits(), "x {x}");
    }
}

#[test]
fn sinc_error_bound_against_libm_reference() {
    let mut worst = 0.0f64;
    for x in grid(8.0, 1.0 / 128.0)
        .into_iter()
        .chain([0.3, 1.7, 2.9, 5.55, 7.77])
    {
        worst = worst.max((sinc(x) - ref_sinc(x)).abs());
    }
    assert!(worst < SINC_MAX_ERROR, "worst {worst}");
}

#[test]
fn sinc_error_bound_is_tight_enough_to_matter() {
    // The series is accurate to well below the stated bound.
    let mut worst = 0.0f64;
    for x in grid(4.0, 1.0 / 97.0) {
        worst = worst.max((sinc(x) - ref_sinc(x)).abs());
    }
    assert!(worst < 1e-13, "worst {worst}");
}

#[test]
fn sinc_term_count_is_fixed() {
    assert_eq!(SINC_TERMS, 9);
}

#[test]
fn sinc_sign_alternates_between_zeros() {
    assert!(sinc(0.5) > 0.0);
    assert!(sinc(1.5) < 0.0);
    assert!(sinc(2.5) > 0.0);
    assert!(sinc(3.5) < 0.0);
    assert!(sinc(-1.5) < 0.0);
}

#[test]
fn floor_values() {
    assert_eq!(floor(-0.5), -1.0);
    assert_eq!(floor(-1.0), -1.0);
    assert_eq!(floor(2.9), 2.0);
    assert_eq!(floor(0.0), 0.0);
    assert_eq!(floor(-2.0000001), -3.0);
    assert_eq!(floor(7.0), 7.0);
    assert_eq!(floor(1e20), 1e20);
    assert!(floor(f64::NAN).is_nan());
}

#[test]
fn bilinear_values() {
    assert_eq!(bilinear(0.0), 1.0);
    assert_eq!(bilinear(0.25), 0.75);
    assert_eq!(bilinear(-0.25), 0.75);
    assert_eq!(bilinear(0.5), 0.5);
    assert_eq!(bilinear(1.0), 0.0);
    assert_eq!(bilinear(1.5), 0.0);
    assert_eq!(bilinear(-7.0), 0.0);
}

#[test]
fn keys_exact_at_integers() {
    for a in [-0.5, -0.75, -1.0, 0.3] {
        assert_eq!(keys(0.0, a), 1.0);
        assert_eq!(keys(1.0, a), 0.0);
        assert_eq!(keys(-1.0, a), 0.0);
        assert_eq!(keys(2.0, a), 0.0);
        assert_eq!(keys(3.5, a), 0.0);
    }
}

#[test]
fn keys_half_values_by_hand() {
    // a = -0.5: 1.5/8 - 2.5/4 + 1 = 0.5625 ; -0.5*3.375 + 2.5*2.25 - 6 + 2 = -0.0625
    assert_eq!(keys(0.5, -0.5), 0.5625);
    assert_eq!(keys(1.5, -0.5), -0.0625);
    // a = -1: 0.125 - 0.5 + 1 = 0.625 ; -3.375 + 11.25 - 12 + 4 = -0.125
    assert_eq!(keys(0.5, -1.0), 0.625);
    assert_eq!(keys(1.5, -1.0), -0.125);
    // a = 0: 0.25 - 0.75 + 1 = 0.5 ; outer lobe is zero
    assert_eq!(keys(0.5, 0.0), 0.5);
    assert_eq!(keys(1.5, 0.0), 0.0);
}

#[test]
fn keys_quarter_values_by_hand() {
    assert_eq!(keys(0.25, -0.5), 0.8671875);
    assert_eq!(keys(0.75, -0.5), 0.2265625);
    assert_eq!(keys(1.25, -0.5), -0.0703125);
    assert_eq!(keys(1.75, -0.5), -0.0234375);
}

#[test]
fn keys_is_even() {
    for t in grid(2.5, 1.0 / 16.0) {
        assert_eq!(keys(t, -0.6).to_bits(), keys(-t, -0.6).to_bits());
    }
}

#[test]
fn keys_partition_of_unity() {
    for a in [-0.5, -0.75, -1.0, -0.2] {
        for i in 0..64 {
            let f = f64::from(i) / 64.0;
            let s = keys(f + 1.0, a) + keys(f, a) + keys(f - 1.0, a) + keys(f - 2.0, a);
            assert!((s - 1.0).abs() < 1e-14, "a {a} f {f} sum {s}");
        }
    }
}

#[test]
fn keys_is_continuous_at_one_and_two() {
    for a in [-0.5, -1.0] {
        assert!(keys(1.0 - 1e-9, a).abs() < 1e-8);
        assert!(keys(1.0 + 1e-9, a).abs() < 1e-8);
        assert!(keys(2.0 - 1e-9, a).abs() < 1e-8);
    }
}

#[test]
fn mitchell_catmull_rom_equals_keys_half() {
    for t in grid(2.5, 1.0 / 32.0) {
        assert!(
            (mitchell(t, 0.0, 0.5) - keys(t, -0.5)).abs() < 1e-14,
            "t {t}"
        );
    }
}

#[test]
fn mitchell_recommended_values_by_hand() {
    let b = 1.0 / 3.0;
    assert!((mitchell(0.0, b, b) - 8.0 / 9.0).abs() < 1e-15);
    assert!((mitchell(1.0, b, b) - 1.0 / 18.0).abs() < 1e-15);
    assert_eq!(mitchell(2.0, b, b), 0.0);
}

#[test]
fn mitchell_bspline_values_by_hand() {
    assert!((mitchell(0.0, 1.0, 0.0) - 4.0 / 6.0).abs() < 1e-15);
    assert!((mitchell(1.0, 1.0, 0.0) - 1.0 / 6.0).abs() < 1e-15);
    assert!((mitchell(0.5, 1.0, 0.0) - 23.0 / 48.0).abs() < 1e-15);
}

#[test]
fn mitchell_b_half_c_zero_taps() {
    // (7.5 t^3 - 12 t^2 + 5) / 6 inside, (-0.5 t^3 + 3 t^2 - 6 t + 4) / 6 outside
    assert!((mitchell(0.25, 0.5, 0.0) - 4.3671875 / 6.0).abs() < 1e-15);
    assert!((mitchell(0.75, 0.5, 0.0) - 1.4140625 / 6.0).abs() < 1e-15);
    assert!((mitchell(1.25, 0.5, 0.0) - 0.2109375 / 6.0).abs() < 1e-15);
    assert!((mitchell(1.75, 0.5, 0.0) - 0.0078125 / 6.0).abs() < 1e-15);
}

#[test]
fn mitchell_partition_of_unity_and_evenness() {
    for (b, c) in [(0.0, 0.5), (1.0 / 3.0, 1.0 / 3.0), (1.0, 0.0), (0.2, 0.7)] {
        for i in 0..32 {
            let f = f64::from(i) / 32.0;
            let s = mitchell(f + 1.0, b, c)
                + mitchell(f, b, c)
                + mitchell(f - 1.0, b, c)
                + mitchell(f - 2.0, b, c);
            assert!((s - 1.0).abs() < 1e-14, "b {b} c {c} f {f}");
            assert_eq!(mitchell(f, b, c).to_bits(), mitchell(-f, b, c).to_bits());
        }
    }
}

#[test]
fn mitchell_zero_outside_support() {
    assert_eq!(mitchell(2.0, 0.3, 0.3), 0.0);
    assert_eq!(mitchell(-2.5, 0.3, 0.3), 0.0);
}

#[test]
fn lanczos_exact_integers_and_support() {
    for lobes in 1..=4 {
        assert_eq!(lanczos(0.0, lobes), 1.0);
        for n in 1..=6 {
            assert_eq!(lanczos(f64::from(n), lobes), 0.0);
            assert_eq!(lanczos(-f64::from(n), lobes), 0.0);
        }
        assert_eq!(lanczos(f64::from(lobes) + 0.25, lobes), 0.0);
    }
}

#[test]
fn lanczos_matches_libm_reference() {
    for lobes in 1..=4u32 {
        for x in grid(5.0, 1.0 / 32.0) {
            let d = (lanczos(x, lobes) - ref_lanczos(x, f64::from(lobes))).abs();
            assert!(d < 2e-12, "lobes {lobes} x {x} d {d}");
        }
    }
}

#[test]
fn lanczos_two_half_closed_form() {
    // sinc(0.5) sinc(0.25) = (2/pi) (2 sqrt 2 / pi)
    let want = 4.0 * 2.0f64.sqrt() / (PI * PI);
    assert!((lanczos(0.5, 2) - want).abs() < 1e-13);
}

#[test]
fn lanczos_is_even() {
    for x in grid(3.2, 1.0 / 16.0) {
        assert_eq!(lanczos(x, 3).to_bits(), lanczos(-x, 3).to_bits());
    }
}

#[test]
fn lanczos_one_lobe_is_positive_inside() {
    for x in grid(0.99, 1.0 / 20.0) {
        assert!(lanczos(x, 1) > 0.0, "x {x}");
    }
}
