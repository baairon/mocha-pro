//! Exact tests of the conversions: hand computed vectors and exhaustive sweeps.

use pixel_format::{
    float_to_int, int_to_float, Clamp, FloatToInt, IntToFloat, Precision, Rescale, Rounding,
};

fn f2i(rounding: Rounding, clamp: Clamp, precision: Precision, scale: f64) -> FloatToInt {
    FloatToInt {
        scale,
        offset: 0.0,
        rounding,
        clamp,
        precision,
    }
}

fn all_modes(x: f32, max: u32) -> [u32; 5] {
    let mut out = [0u32; 5];
    for (i, r) in Rounding::ALL.into_iter().enumerate() {
        out[i] = float_to_int(
            x,
            max,
            &f2i(r, Clamp::Range, Precision::F64, f64::from(max)),
        );
    }
    out
}

// Order of Rounding::ALL: HalfUp, TiesEven, Truncate, Floor, Ceil.

#[test]
fn rounding_at_a_half_above_even() {
    // 0.125 * 4 = 0.5 (floor 0, even)
    assert_eq!(all_modes(0.125, 4), [1, 0, 0, 0, 1]);
}

#[test]
fn rounding_at_a_half_above_odd() {
    // 0.375 * 4 = 1.5
    assert_eq!(all_modes(0.375, 4), [2, 2, 1, 1, 2]);
}

#[test]
fn rounding_at_two_and_a_half() {
    // 0.625 * 4 = 2.5
    assert_eq!(all_modes(0.625, 4), [3, 2, 2, 2, 3]);
}

#[test]
fn rounding_at_three_and_a_half() {
    // 0.875 * 4 = 3.5
    assert_eq!(all_modes(0.875, 4), [4, 4, 3, 3, 4]);
}

#[test]
fn rounding_just_below_and_above_a_half() {
    // 0.49999997 and 0.50000006 as f32 times 1 (max 1)
    let below = f32::from_bits(0.5f32.to_bits() - 1);
    let above = f32::from_bits(0.5f32.to_bits() + 1);
    assert_eq!(all_modes(below, 1), [0, 0, 0, 0, 1]);
    assert_eq!(all_modes(above, 1), [1, 1, 0, 0, 1]);
}

#[test]
fn rounding_exact_integers_are_untouched() {
    for x in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
        let v = all_modes(x, 4);
        assert!(v.iter().all(|&n| n == v[0]), "{x} gave {v:?}");
    }
}

#[test]
fn endpoints_zero_and_one_for_every_mode() {
    assert_eq!(all_modes(0.0, 255), [0; 5]);
    assert_eq!(all_modes(1.0, 255), [255; 5]);
    assert_eq!(all_modes(1.0, 32768), [32768; 5]);
    assert_eq!(all_modes(1.0, 65535), [65535; 5]);
}

#[test]
fn round_f64_on_negative_values() {
    // -2.5, -1.5, -0.5
    let t = |x: f64| Rounding::ALL.map(|r| r.round_f64(x));
    assert_eq!(t(-2.5), [-2.0, -2.0, -2.0, -3.0, -2.0]);
    assert_eq!(t(-1.5), [-1.0, -2.0, -1.0, -2.0, -1.0]);
    assert_eq!(t(-0.5), [0.0, 0.0, 0.0, -1.0, 0.0]);
    // the sign of a zero result
    assert!(Rounding::TiesEven.round_f64(-0.5).is_sign_negative());
    assert!(Rounding::Truncate.round_f64(-0.25).is_sign_negative());
    assert!(Rounding::HalfUp.round_f64(-0.5).is_sign_positive());
}

#[test]
fn round_f32_matches_the_f64_rule_on_exact_halves() {
    for k in -20i32..=20 {
        let x = k as f32 * 0.5;
        for r in Rounding::ALL {
            assert_eq!(
                f64::from(r.round_f32(x)),
                r.round_f64(f64::from(x)),
                "{k} {r:?}"
            );
        }
    }
}

#[test]
fn half_up_step_is_done_in_the_working_precision() {
    // 0.49999997 + 0.5 is 1.0 in f32 (the sum rounds up) but 0.99999997 in f64.
    let x = f32::from_bits(0.5f32.to_bits() - 1);
    assert_eq!(Rounding::HalfUp.round_f32(x), 1.0);
    assert_eq!(Rounding::HalfUp.round_f64(f64::from(x)), 0.0);
}

#[test]
fn precision_changes_the_result_at_a_made_up_tie() {
    // 0.1f32 * 245 is 24.50000036 in f64 and rounds to exactly 24.5 in f32.
    let x = 0.1f32;
    let a = f2i(Rounding::TiesEven, Clamp::Range, Precision::F64, 245.0);
    let b = f2i(Rounding::TiesEven, Clamp::Range, Precision::F32, 245.0);
    assert_eq!(float_to_int(x, 245, &a), 25);
    assert_eq!(float_to_int(x, 245, &b), 24);
}

#[test]
fn offset_is_added_after_the_multiply() {
    let c = FloatToInt {
        scale: 10.0,
        offset: 0.25,
        rounding: Rounding::Truncate,
        clamp: Clamp::Range,
        precision: Precision::F64,
    };
    // 0.5 * 10 + 0.25 = 5.25 -> 5 ; 0.75 * 10 + 0.25 = 7.75 -> 7
    assert_eq!(float_to_int(0.5, 10, &c), 5);
    assert_eq!(float_to_int(0.75, 10, &c), 7);
    let d = FloatToInt { offset: 0.5, ..c };
    // 0.25 * 10 + 0.5 = 3.0 -> 3, one ulp below stays 2 (made exact: 0.2499 * 10 + 0.5 = 2.999)
    assert_eq!(float_to_int(0.25, 10, &d), 3);
    assert_eq!(float_to_int(0.2499, 10, &d), 2);
}

#[test]
fn clamp_above_range() {
    let make = |c| f2i(Rounding::HalfUp, c, Precision::F64, 4.0);
    assert_eq!(float_to_int(2.0, 4, &make(Clamp::None)), 8);
    assert_eq!(float_to_int(2.0, 4, &make(Clamp::Range)), 4);
    assert_eq!(float_to_int(2.0, 4, &make(Clamp::RangeNanZero)), 4);
    assert_eq!(float_to_int(1.0e30, 4, &make(Clamp::None)), u32::MAX);
}

#[test]
fn clamp_below_range_and_negative_zero() {
    for c in Clamp::ALL {
        let k = f2i(Rounding::HalfUp, c, Precision::F64, 4.0);
        assert_eq!(float_to_int(-1.0, 4, &k), 0, "{c:?}");
        assert_eq!(float_to_int(-0.0, 4, &k), 0, "{c:?}");
        assert_eq!(float_to_int(-1.0e30, 4, &k), 0, "{c:?}");
    }
}

#[test]
fn nan_under_each_clamp_and_payload() {
    let payloads = [
        f32::NAN,
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0xFFC0_1234),
        f32::from_bits(0x7F80_0001),
    ];
    for x in payloads {
        assert!(x.is_nan());
        for p in Precision::ALL {
            let mk = |c| f2i(Rounding::HalfUp, c, p, 4.0);
            assert_eq!(float_to_int(x, 4, &mk(Clamp::None)), 0);
            assert_eq!(float_to_int(x, 4, &mk(Clamp::Range)), 4);
            assert_eq!(float_to_int(x, 4, &mk(Clamp::RangeNanZero)), 0);
        }
    }
}

#[test]
fn infinities_under_each_clamp() {
    let mk = |c| f2i(Rounding::TiesEven, c, Precision::F32, 255.0);
    assert_eq!(float_to_int(f32::INFINITY, 255, &mk(Clamp::None)), u32::MAX);
    assert_eq!(float_to_int(f32::INFINITY, 255, &mk(Clamp::Range)), 255);
    assert_eq!(
        float_to_int(f32::INFINITY, 255, &mk(Clamp::RangeNanZero)),
        255
    );
    for c in Clamp::ALL {
        assert_eq!(float_to_int(f32::NEG_INFINITY, 255, &mk(c)), 0);
    }
}

#[test]
fn subnormal_inputs_round_by_mode() {
    let tiny = f32::from_bits(1); // smallest subnormal
    assert!(tiny > 0.0 && !tiny.is_normal());
    assert_eq!(all_modes(tiny, 255), [0, 0, 0, 0, 1]);
    assert_eq!(all_modes(-tiny, 255), [0, 0, 0, 0, 0]);
    assert_eq!(all_modes(f32::MIN_POSITIVE, 255), [0, 0, 0, 0, 1]);
}

#[test]
fn clamp_is_applied_to_the_rounded_value() {
    // 4.4 / 4 = 1.1 scaled back: 4.4 rounds to 4 (in range), 4.6 rounds to 5 (clamped to 4).
    let k = f2i(Rounding::HalfUp, Clamp::Range, Precision::F64, 4.0);
    assert_eq!(float_to_int(1.1, 4, &k), 4);
    assert_eq!(float_to_int(1.15, 4, &k), 4);
    let n = f2i(Rounding::HalfUp, Clamp::None, Precision::F64, 4.0);
    assert_eq!(float_to_int(1.1, 4, &n), 4);
    assert_eq!(float_to_int(1.15, 4, &n), 5);
}

#[test]
fn int_to_float_vectors() {
    let c = IntToFloat {
        denom: 255.0,
        precision: Precision::F64,
    };
    assert_eq!(int_to_float(0, &c).to_bits(), 0x0000_0000);
    assert_eq!(int_to_float(255, &c).to_bits(), 0x3F80_0000);
    assert_eq!(int_to_float(51, &c).to_bits(), 0x3E4C_CCCD); // 0.2
    let d = IntToFloat {
        denom: 32768.0,
        precision: Precision::F32,
    };
    assert_eq!(int_to_float(16384, &d), 0.5);
    assert_eq!(int_to_float(32768, &d), 1.0);
    assert_eq!(int_to_float(1, &d).to_bits(), 0x3800_0000); // 2^-15
                                                            // values above full scale give floats above one, unclamped
    assert_eq!(int_to_float(65536, &d), 2.0);
}

#[test]
fn int_to_float_precision_differs_when_the_divisor_is_not_an_f32() {
    // 16777217 narrows to 16777216 in f32.
    let wide = IntToFloat {
        denom: 16_777_217.0,
        precision: Precision::F64,
    };
    let narrow = IntToFloat {
        precision: Precision::F32,
        ..wide
    };
    assert_eq!(int_to_float(8_388_608, &wide).to_bits(), 0x3EFF_FFFF);
    assert_eq!(int_to_float(8_388_608, &narrow), 0.5);
}

#[test]
fn rescale_full_range_to_host_range_vectors() {
    let r = Rescale {
        mul: 32768.0 / 65535.0,
        add: 0.5,
        rounding: Rounding::Truncate,
        precision: Precision::F64,
    };
    // 0, 1, 2, 3, the midpoint 32767 and 32768, the top two.
    assert_eq!(r.apply(0), 0);
    assert_eq!(r.apply(1), 1);
    assert_eq!(r.apply(2), 1);
    assert_eq!(r.apply(3), 2);
    assert_eq!(r.apply(32767), 16384);
    assert_eq!(r.apply(32768), 16384);
    assert_eq!(r.apply(32769), 16385);
    assert_eq!(r.apply(65534), 32767);
    assert_eq!(r.apply(65535), 32768);
}

#[test]
fn rescale_half_with_made_up_constants() {
    let r = Rescale {
        mul: 0.5,
        add: 0.5,
        rounding: Rounding::Truncate,
        precision: Precision::F64,
    };
    // v/2 + 0.5 truncated: 0->0, 1->1, 2->1, 3->2, 4->2, 65535->32768
    let got: Vec<u16> = [0u16, 1, 2, 3, 4, 65535]
        .iter()
        .map(|&v| r.apply(v))
        .collect();
    assert_eq!(got, [0, 1, 1, 2, 2, 32768]);
}

#[test]
fn rescale_saturates_at_both_ends() {
    let up = Rescale {
        mul: 2.0,
        add: 0.0,
        rounding: Rounding::HalfUp,
        precision: Precision::F64,
    };
    assert_eq!(up.apply(40000), 65535);
    assert_eq!(up.apply(32767), 65534);
    let down = Rescale {
        mul: 1.0,
        add: -10.0,
        rounding: Rounding::HalfUp,
        precision: Precision::F64,
    };
    assert_eq!(down.apply(5), 0);
    assert_eq!(down.apply(10), 0);
    assert_eq!(down.apply(11), 1);
}

#[test]
fn rescale_rounding_modes_at_a_made_up_tie() {
    // v * 0.5 + 0.0 at v = 5 is 2.5
    let at = |rounding| {
        Rescale {
            mul: 0.5,
            add: 0.0,
            rounding,
            precision: Precision::F64,
        }
        .apply(5)
    };
    assert_eq!(Rounding::ALL.map(at), [3, 2, 2, 2, 3]);
}

#[test]
fn rescale_sweep_equals_the_integer_formula() {
    // floor(v * 32768 / 65535 + 1/2) = (65536 v + 65535) / 131070 in integers; the real value is
    // never an integer (odd numerator over an even denominator), so doubles agree exactly.
    let r = Rescale {
        mul: 32768.0 / 65535.0,
        add: 0.5,
        rounding: Rounding::Truncate,
        precision: Precision::F64,
    };
    for v in 0..=65535u32 {
        let expect = (65536 * u64::from(v) + 65535) / 131_070;
        assert_eq!(u64::from(r.apply(v as u16)), expect, "v={v}");
    }
}

#[test]
fn rescale_sweep_is_monotone_and_in_range_for_every_mode_and_precision() {
    for rounding in Rounding::ALL {
        for precision in Precision::ALL {
            let r = Rescale {
                mul: 32768.0 / 65535.0,
                add: 0.5,
                rounding,
                precision,
            };
            let mut last = 0u16;
            for v in 0..=65535u32 {
                let o = r.apply(v as u16);
                assert!(o >= last, "{rounding:?} {precision:?} v={v}");
                assert!(o <= 32769);
                last = o;
            }
        }
    }
}

#[test]
fn sweep_round_trip_is_exact_for_nearest_modes() {
    for &max in &[255u32, 32768, 65535] {
        for precision in Precision::ALL {
            let up = IntToFloat {
                denom: f64::from(max),
                precision,
            };
            for rounding in [Rounding::HalfUp, Rounding::TiesEven] {
                let down = f2i(rounding, Clamp::Range, precision, f64::from(max));
                for v in 0..=max {
                    let x = int_to_float(v, &up);
                    assert_eq!(
                        float_to_int(x, max, &down),
                        v,
                        "max={max} v={v} {rounding:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn sweep_round_trip_stays_within_one_for_directed_modes() {
    for &max in &[255u32, 65535] {
        let up = IntToFloat {
            denom: f64::from(max),
            precision: Precision::F32,
        };
        for rounding in [Rounding::Truncate, Rounding::Floor, Rounding::Ceil] {
            let down = f2i(rounding, Clamp::Range, Precision::F32, f64::from(max));
            for v in 0..=max {
                let back = float_to_int(int_to_float(v, &up), max, &down);
                assert!(back.abs_diff(v) <= 1, "max={max} v={v} {rounding:?} {back}");
            }
        }
    }
}

#[test]
fn sweep_round_trip_is_exact_for_every_mode_at_a_power_of_two() {
    let up = IntToFloat {
        denom: 32768.0,
        precision: Precision::F32,
    };
    for rounding in Rounding::ALL {
        let down = f2i(rounding, Clamp::Range, Precision::F32, 32768.0);
        for v in 0..=32768u32 {
            assert_eq!(float_to_int(int_to_float(v, &up), 32768, &down), v);
        }
    }
}

#[test]
fn sweep_float_to_int_is_monotone_and_in_range() {
    for &max in &[255u32, 32768, 65535] {
        for rounding in Rounding::ALL {
            for precision in Precision::ALL {
                let k = f2i(rounding, Clamp::Range, precision, f64::from(max));
                let mut last = 0u32;
                for i in 0..=40_000u32 {
                    let x = -0.25 + 1.5 * (i as f32) / 40_000.0;
                    let o = float_to_int(x, max, &k);
                    assert!(o >= last && o <= max, "max={max} x={x} {rounding:?}");
                    last = o;
                }
                assert_eq!(last, max);
            }
        }
    }
}

#[test]
fn sweep_int_to_float_is_strictly_increasing() {
    for &max in &[255u32, 65535] {
        for precision in Precision::ALL {
            let up = IntToFloat {
                denom: f64::from(max),
                precision,
            };
            let mut last = -1.0f32;
            for v in 0..=max {
                let x = int_to_float(v, &up);
                assert!(x > last);
                last = x;
            }
            assert_eq!(last, 1.0);
        }
    }
}

#[test]
fn sweep_nearest_rounding_error_is_at_most_a_half() {
    let k = f2i(Rounding::HalfUp, Clamp::Range, Precision::F64, 255.0);
    let e = f2i(Rounding::TiesEven, Clamp::Range, Precision::F64, 255.0);
    for i in 0..=10_000u32 {
        let x = i as f32 / 10_000.0;
        let exact = f64::from(x) * 255.0;
        for c in [&k, &e] {
            let o = f64::from(float_to_int(x, 255, c));
            assert!((o - exact).abs() <= 0.5 + 1e-9, "x={x}");
        }
    }
}

#[test]
fn directed_rounding_brackets_the_exact_value() {
    let t = f2i(Rounding::Truncate, Clamp::Range, Precision::F64, 255.0);
    let f = f2i(Rounding::Floor, Clamp::Range, Precision::F64, 255.0);
    let c = f2i(Rounding::Ceil, Clamp::Range, Precision::F64, 255.0);
    for i in 0..=10_000u32 {
        let x = i as f32 / 10_000.0;
        let exact = f64::from(x) * 255.0;
        let (lo, hi) = (float_to_int(x, 255, &f), float_to_int(x, 255, &c));
        assert!(f64::from(lo) <= exact && exact <= f64::from(hi));
        assert_eq!(float_to_int(x, 255, &t), lo);
        assert!(hi - lo <= 1);
    }
}

#[test]
fn names_parse_back() {
    for r in Rounding::ALL {
        assert_eq!(Rounding::parse(r.name()), Some(r));
    }
    for p in Precision::ALL {
        assert_eq!(Precision::parse(p.name()), Some(p));
    }
    for c in Clamp::ALL {
        assert_eq!(Clamp::parse(c.name()), Some(c));
    }
    assert_eq!(Rounding::parse("Half_Up"), None);
    assert_eq!(Precision::parse("f16"), None);
    assert_eq!(Clamp::parse(""), None);
}

#[test]
fn f32_path_applies_the_offset_and_rounds_in_f32() {
    let c = FloatToInt {
        scale: 10.0,
        offset: 0.25,
        rounding: Rounding::Truncate,
        clamp: Clamp::Range,
        precision: Precision::F32,
    };
    assert_eq!(float_to_int(0.5, 10, &c), 5);
    assert_eq!(float_to_int(0.75, 10, &c), 7);
    let d = FloatToInt { offset: -0.5, ..c };
    assert_eq!(float_to_int(0.5, 10, &d), 4);
    // the half up step runs in f32: 0.49999997 + 0.5 is 1.0 there, but 0 in f64
    let below = f32::from_bits(0.5f32.to_bits() - 1);
    let h32 = f2i(Rounding::HalfUp, Clamp::Range, Precision::F32, 1.0);
    let h64 = f2i(Rounding::HalfUp, Clamp::Range, Precision::F64, 1.0);
    assert_eq!(float_to_int(below, 1, &h32), 1);
    assert_eq!(float_to_int(below, 1, &h64), 0);
}

#[test]
fn f64_multiply_and_add_are_not_fused() {
    // x * 0.1 is not exact in f64: the separately rounded product plus the offset is exactly 3.0,
    // while a fused multiply add would give 2.9999999999999996 and floor to 2.
    let x = f32::from_bits(0x4220_0419);
    let c = FloatToInt {
        scale: 0.1,
        offset: -1.0004001617431646,
        rounding: Rounding::Floor,
        clamp: Clamp::None,
        precision: Precision::F64,
    };
    assert_eq!(float_to_int(x, 255, &c), 3);
}
