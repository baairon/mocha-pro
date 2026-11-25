//! Premultiply and unpremultiply: exact integer vectors and float cases.

use pixel_format::{
    premultiply_f32, premultiply_int, unpremultiply_f32, unpremultiply_int, Precision, Rounding,
    ZeroAlpha,
};

fn pre(c: u32, a: u32, max: u32) -> [u32; 5] {
    Rounding::ALL.map(|r| premultiply_int(c, a, max, r))
}

fn unpre(c: u32, a: u32, max: u32) -> [u32; 5] {
    Rounding::ALL.map(|r| unpremultiply_int(c, a, max, r, ZeroAlpha::ZeroColour))
}

// Order of Rounding::ALL: HalfUp, TiesEven, Truncate, Floor, Ceil.

#[test]
fn premultiply_alpha_zero_and_max() {
    assert_eq!(pre(200, 0, 255), [0; 5]);
    assert_eq!(pre(200, 255, 255), [200; 5]);
    assert_eq!(pre(255, 255, 255), [255; 5]);
    assert_eq!(pre(0, 77, 255), [0; 5]);
    assert_eq!(pre(32768, 32768, 32768), [32768; 5]);
    assert_eq!(pre(65535, 65535, 65535), [65535; 5]);
}

#[test]
fn premultiply_exact_division() {
    // 100 * 51 / 255 = 20, 255 * 128 / 255 = 128
    assert_eq!(pre(100, 51, 255), [20; 5]);
    assert_eq!(pre(255, 128, 255), [128; 5]);
}

#[test]
fn premultiply_below_and_above_a_half() {
    // 1 * 128 / 255 = 0.502, 1 * 127 / 255 = 0.498
    assert_eq!(pre(1, 128, 255), [1, 1, 0, 0, 1]);
    assert_eq!(pre(1, 127, 255), [0, 0, 0, 0, 1]);
}

#[test]
fn premultiply_exact_ties_with_a_made_up_even_maximum() {
    // max 4: 1*2/4 = 0.5, 3*2/4 = 1.5, 5 is out of range so use max 8: 5*4/8 = 2.5, 7*4/8 = 3.5
    assert_eq!(pre(1, 2, 4), [1, 0, 0, 0, 1]);
    assert_eq!(pre(3, 2, 4), [2, 2, 1, 1, 2]);
    assert_eq!(pre(5, 4, 8), [3, 2, 2, 2, 3]);
    assert_eq!(pre(7, 4, 8), [4, 4, 3, 3, 4]);
}

#[test]
fn premultiply_quarter_fractions() {
    // max 4: 1*1/4 = .25, 3*1/4 = .75, 3*3/4 = 2.25
    assert_eq!(pre(1, 1, 4), [0, 0, 0, 0, 1]);
    assert_eq!(pre(3, 1, 4), [1, 1, 0, 0, 1]);
    assert_eq!(pre(3, 3, 4), [2, 2, 2, 2, 3]);
}

#[test]
fn premultiply_zero_maximum_and_large_operands() {
    assert_eq!(pre(5, 5, 0), [0; 5]);
    // u32 operands near the top do not overflow: (2^32 - 1)^2 / (2^32 - 1)
    let m = u32::MAX;
    assert_eq!(pre(m, m, m), [m; 5]);
    assert_eq!(pre(m, m - 1, m), [m - 1; 5]);
}

#[test]
fn premultiply_sweep_is_bounded_by_both_inputs() {
    for a in (0..=255u32).step_by(5) {
        for c in 0..=255u32 {
            for r in Rounding::ALL {
                let v = premultiply_int(c, a, 255, r);
                assert!(v <= c.min(a) + 1, "c={c} a={a}");
                assert!(v <= 255);
            }
        }
    }
}

#[test]
fn premultiply_sweep_nearest_matches_a_second_derivation() {
    // round(c * a / 255) from f64 with an offset away from any tie (255 is odd, so no tie exists).
    for a in 0..=255u32 {
        for c in 0..=255u32 {
            let exact = f64::from(c) * f64::from(a) / 255.0;
            let expect = exact.round() as u32;
            assert_eq!(premultiply_int(c, a, 255, Rounding::HalfUp), expect);
            assert_eq!(premultiply_int(c, a, 255, Rounding::TiesEven), expect);
        }
    }
}

#[test]
fn unpremultiply_alpha_zero_rules() {
    for r in Rounding::ALL {
        assert_eq!(unpremultiply_int(9, 0, 255, r, ZeroAlpha::ZeroColour), 0);
        assert_eq!(unpremultiply_int(9, 0, 255, r, ZeroAlpha::KeepColour), 9);
        assert_eq!(unpremultiply_int(0, 0, 255, r, ZeroAlpha::KeepColour), 0);
    }
}

#[test]
fn unpremultiply_alpha_max_and_identity() {
    assert_eq!(unpre(77, 255, 255), [77; 5]);
    assert_eq!(unpre(0, 100, 255), [0; 5]);
    assert_eq!(unpre(32768, 32768, 32768), [32768; 5]);
}

#[test]
fn unpremultiply_exact_division() {
    // 20 * 255 / 51 = 100
    assert_eq!(unpre(20, 51, 255), [100; 5]);
}

#[test]
fn unpremultiply_ties() {
    // 64 * 255 / 128 = 127.5 ; max 100: 1 * 100 / 8 = 12.5 and 3 * 100 / 8 = 37.5
    assert_eq!(unpre(64, 128, 255), [128, 128, 127, 127, 128]);
    assert_eq!(unpre(1, 8, 100), [13, 12, 12, 12, 13]);
    assert_eq!(unpre(3, 8, 100), [38, 38, 37, 37, 38]);
}

#[test]
fn unpremultiply_is_capped_at_the_maximum() {
    assert_eq!(unpre(200, 100, 255), [255; 5]);
    assert_eq!(unpre(5, 1, 10), [10; 5]);
    // 50 * 10 / 4 = 125 would exceed 10
    assert_eq!(unpre(50, 4, 10), [10; 5]);
}

#[test]
fn unpremultiply_sweep_undoes_premultiply_within_one_step() {
    for a in 1..=255u32 {
        for c in (0..=255u32).step_by(3) {
            let p = premultiply_int(c, a, 255, Rounding::HalfUp);
            let u = unpremultiply_int(p, a, 255, Rounding::HalfUp, ZeroAlpha::ZeroColour);
            // one step of colour costs at most 255 / (2 a) + 1 after the way back
            let bound = 255 / (2 * a) + 1;
            assert!(u.abs_diff(c) <= bound, "c={c} a={a} u={u}");
        }
    }
}

#[test]
fn float_premultiply_vectors() {
    for p in Precision::ALL {
        assert_eq!(premultiply_f32(0.5, 0.5, p), 0.25);
        assert_eq!(premultiply_f32(1.0, 0.0, p), 0.0);
        assert_eq!(premultiply_f32(0.75, 1.0, p), 0.75);
        assert_eq!(premultiply_f32(2.0, 0.5, p), 1.0); // above one is not clamped
        assert_eq!(premultiply_f32(-0.5, 0.5, p), -0.25);
        assert!(premultiply_f32(f32::NAN, 1.0, p).is_nan());
        assert_eq!(premultiply_f32(f32::INFINITY, 0.5, p), f32::INFINITY);
        assert!(premultiply_f32(f32::INFINITY, 0.0, p).is_nan());
    }
}

#[test]
fn float_unpremultiply_vectors() {
    for p in Precision::ALL {
        let z = ZeroAlpha::ZeroColour;
        assert_eq!(unpremultiply_f32(0.25, 0.5, p, z), 0.5);
        assert_eq!(unpremultiply_f32(0.5, 1.0, p, z), 0.5);
        assert_eq!(unpremultiply_f32(0.5, 0.25, p, z), 2.0); // not clamped
        assert!(unpremultiply_f32(f32::NAN, 0.5, p, z).is_nan());
    }
}

#[test]
fn float_unpremultiply_zero_alpha_rules_with_both_signs() {
    for p in Precision::ALL {
        for alpha in [0.0f32, -0.0] {
            assert_eq!(unpremultiply_f32(0.3, alpha, p, ZeroAlpha::ZeroColour), 0.0);
            assert_eq!(unpremultiply_f32(0.3, alpha, p, ZeroAlpha::KeepColour), 0.3);
        }
        // a subnormal alpha is not zero
        let tiny = f32::from_bits(1);
        let v = unpremultiply_f32(0.0, tiny, p, ZeroAlpha::KeepColour);
        assert_eq!(v, 0.0);
        assert_eq!(unpremultiply_f32(tiny, tiny, p, ZeroAlpha::ZeroColour), 1.0);
    }
}

#[test]
fn zero_alpha_names_parse_back() {
    for z in ZeroAlpha::ALL {
        assert_eq!(ZeroAlpha::parse(z.name()), Some(z));
    }
    assert_eq!(ZeroAlpha::parse("zero"), None);
}

#[test]
fn hand_computed_integer_alpha_cases() {
    // max 100: 1 * 50 / 100 = 0.5, 3 * 50 / 100 = 1.5, 5 * 50 / 100 = 2.5
    let rows = [
        (1, [1, 0, 0, 0, 1]),
        (3, [2, 2, 1, 1, 2]),
        (5, [3, 2, 2, 2, 3]),
    ];
    for (c, want) in rows {
        let got = Rounding::ALL.map(|r| premultiply_int(c, 50, 100, r));
        assert_eq!(got, want, "colour {c}");
    }
    // 100 * 255 / 128 = 199.21875; capped at max; alpha 0 rules
    let got = Rounding::ALL.map(|r| unpremultiply_int(100, 128, 255, r, ZeroAlpha::ZeroColour));
    assert_eq!(got, [199, 199, 199, 199, 200]);
    assert_eq!(
        unpremultiply_int(200, 100, 255, Rounding::HalfUp, ZeroAlpha::ZeroColour),
        255
    );
    assert_eq!(
        unpremultiply_int(7, 0, 255, Rounding::HalfUp, ZeroAlpha::ZeroColour),
        0
    );
    assert_eq!(
        unpremultiply_int(7, 0, 255, Rounding::HalfUp, ZeroAlpha::KeepColour),
        7
    );
}

#[test]
fn float_forms_keep_signs() {
    for p in Precision::ALL {
        // a negative alpha is not zero alpha: the quotient keeps its sign
        assert_eq!(unpremultiply_f32(1.0, -0.5, p, ZeroAlpha::ZeroColour), -2.0);
        assert_eq!(unpremultiply_f32(-1.0, 0.5, p, ZeroAlpha::ZeroColour), -2.0);
        // the sign of a zero product survives
        assert_eq!(premultiply_f32(-0.0, 1.0, p).to_bits(), 0x8000_0000);
    }
}
