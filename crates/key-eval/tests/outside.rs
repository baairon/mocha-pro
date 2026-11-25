//! Behaviour outside the keyed range, and coincident keys, empty lists and time snapping.

use key_eval::{Coincident, Config, Empty, Key, KeyList, Outside, Side, Snap, VectorMode};

fn lin() -> Vec<Key<1>> {
    // slope 2 over time 0 to 4, values 0 to 8
    vec![Key::linear(0.0, [0.0]), Key::linear(4.0, [8.0])]
}

fn ev(keys: Vec<Key<1>>, cfg: &Config, t: f64) -> f64 {
    KeyList::new(keys, cfg).unwrap().evaluate(cfg, t)[0]
}

fn both(rule: Outside) -> Config {
    Config {
        before: rule,
        after: rule,
        ..Config::default()
    }
}

macro_rules! out {
    ($name:ident, $rule:expr, $t:expr, $want:expr) => {
        #[test]
        fn $name() {
            assert_eq!(ev(lin(), &both($rule), $t), $want);
        }
    };
}

// hold
out!(hold_before, Outside::Hold, -1.0, 0.0);
out!(hold_after, Outside::Hold, 6.0, 8.0);
// linear extrapolation by the end slope 2
out!(linear_before, Outside::Linear, -1.0, -2.0);
out!(linear_after, Outside::Linear, 6.0, 12.0);
// cycle: t = 5 maps to 1 (value 2), t = -1 maps to 3 (value 6), t = 9 maps to 1
out!(cycle_after, Outside::Cycle, 5.0, 2.0);
out!(cycle_before, Outside::Cycle, -1.0, 6.0);
out!(cycle_two_periods_after, Outside::Cycle, 9.0, 2.0);
out!(cycle_exact_period_after, Outside::Cycle, 8.0, 0.0);
out!(cycle_exact_period_before, Outside::Cycle, -4.0, 0.0);
// cycle with offset adds the number of cycles times (last - first) = 8
out!(offset_after, Outside::CycleOffset, 5.0, 10.0);
out!(offset_before, Outside::CycleOffset, -1.0, -2.0);
out!(offset_two_periods_after, Outside::CycleOffset, 9.0, 18.0);
out!(offset_exact_period_after, Outside::CycleOffset, 8.0, 16.0);
out!(offset_two_periods_before, Outside::CycleOffset, -5.0, -10.0);
// mirror: odd repeats run backwards
out!(mirror_after, Outside::Mirror, 5.0, 6.0);
out!(mirror_before, Outside::Mirror, -1.0, 2.0);
out!(mirror_two_periods_after, Outside::Mirror, 9.0, 2.0);
out!(mirror_exact_period_after, Outside::Mirror, 8.0, 0.0);
out!(mirror_exact_odd_period_before, Outside::Mirror, -4.0, 8.0);

#[test]
fn each_side_has_its_own_rule() {
    let c = Config {
        before: Outside::Linear,
        after: Outside::Mirror,
        ..Config::default()
    };
    assert_eq!(ev(lin(), &c, -1.0), -2.0);
    assert_eq!(ev(lin(), &c, 5.0), 6.0);
    let c = Config {
        before: Outside::Hold,
        after: Outside::CycleOffset,
        ..Config::default()
    };
    assert_eq!(ev(lin(), &c, -1.0), 0.0);
    assert_eq!(ev(lin(), &c, 5.0), 10.0);
}

#[test]
fn time_exactly_at_the_ends_is_inside() {
    for rule in [
        Outside::Hold,
        Outside::Linear,
        Outside::Cycle,
        Outside::CycleOffset,
        Outside::Mirror,
    ] {
        assert_eq!(ev(lin(), &both(rule), 0.0), 0.0);
        assert_eq!(ev(lin(), &both(rule), 4.0), 8.0);
    }
}

#[test]
fn linear_extrapolation_after_a_hold_segment_is_flat() {
    let keys = vec![Key::hold(0.0, [1.0]), Key::hold(4.0, [8.0])];
    assert_eq!(ev(keys.clone(), &both(Outside::Linear), 6.0), 8.0);
    assert_eq!(ev(keys, &both(Outside::Linear), -3.0), 1.0);
}

#[test]
fn linear_extrapolation_uses_hermite_end_slopes() {
    // Before: out slope of the first key; after: in slope of the last key.
    let keys = vec![
        Key::hermite(0.0, [0.0], [9.0], [3.0]),
        Key::hermite(4.0, [8.0], [-1.0], [9.0]),
    ];
    let c = both(Outside::Linear);
    assert_eq!(ev(keys.clone(), &c, -2.0), -6.0);
    assert_eq!(ev(keys, &c, 6.0), 6.0);
}

#[test]
fn linear_extrapolation_uses_bezier_handle_slopes() {
    // Out handle offset (2, 1): slope 1/2 before. In handle (4 + 1, 8 - 3) gives slope 3 / 1 after.
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[2.0, 1.0]]),
        Key::bezier(4.0, [8.0], [[5.0, 11.0]], [[0.0, 0.0]]),
    ];
    // in handle (5, 11) is beyond the end: dt2 = 1, dv2 = 3: slope 3.
    let c = both(Outside::Linear);
    assert_eq!(ev(keys.clone(), &c, -4.0), -2.0);
    assert_eq!(ev(keys, &c, 6.0), 14.0);
}

#[test]
fn bezier_vertical_handle_extrapolates_by_chord_slope() {
    let keys = vec![
        Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[0.0, 5.0]]),
        Key::bezier(4.0, [8.0], [[4.0, 3.0]], [[0.0, 0.0]]),
    ];
    let c = both(Outside::Linear);
    assert_eq!(ev(keys.clone(), &c, -1.0), -2.0);
    assert_eq!(ev(keys, &c, 5.0), 10.0);
}

#[test]
fn cycle_over_a_non_linear_range() {
    // Three keys 0, 2, 6 with values 0, 4, 0 (linear): value at t = 8 is the value at 2.
    let keys = vec![
        Key::linear(0.0, [0.0]),
        Key::linear(2.0, [4.0]),
        Key::linear(6.0, [0.0]),
    ];
    assert_eq!(ev(keys.clone(), &both(Outside::Cycle), 8.0), 4.0);
    assert_eq!(ev(keys.clone(), &both(Outside::Cycle), 10.0), 2.0);
    assert_eq!(ev(keys, &both(Outside::Cycle), -1.0), 1.0);
}

#[test]
fn offset_cycle_with_zero_net_change_equals_cycle() {
    let keys = vec![
        Key::linear(0.0, [0.0]),
        Key::linear(2.0, [4.0]),
        Key::linear(6.0, [0.0]),
    ];
    for t in [-7.5, -1.0, 7.0, 13.25] {
        assert_eq!(
            ev(keys.clone(), &both(Outside::Cycle), t),
            ev(keys.clone(), &both(Outside::CycleOffset), t)
        );
    }
}

#[test]
fn cycle_vector_offset_is_per_component() {
    let keys = vec![Key::linear(0.0, [0.0, 1.0]), Key::linear(4.0, [8.0, -3.0])];
    let c = both(Outside::CycleOffset);
    let l = KeyList::new(keys, &c).unwrap();
    // component 1: value at t = 1 is 0, plus one cycle of (-3 - 1) = -4.
    assert_eq!(l.evaluate(&c, 5.0), [10.0, -4.0]);
}

#[test]
fn cycle_with_all_keys_at_one_time_holds() {
    let c = Config {
        coincident: Coincident::Jump,
        ..both(Outside::Cycle)
    };
    let l = KeyList::new(vec![Key::linear(1.0, [3.0]), Key::linear(1.0, [5.0])], &c).unwrap();
    assert_eq!(l.evaluate(&c, 7.0), [5.0]);
    assert_eq!(l.evaluate(&c, -7.0), [3.0]);
    assert_eq!(l.evaluate(&c, 1.0), [5.0]);
    let c2 = Config {
        jump_side: Side::Left,
        ..c
    };
    assert_eq!(l.evaluate(&c2, 1.0), [3.0]);
}

// coincident keys: (0,1) (2,5) (2,9) (4,3), linear
fn coin() -> Vec<Key<1>> {
    vec![
        Key::linear(0.0, [1.0]),
        Key::linear(2.0, [5.0]),
        Key::linear(2.0, [9.0]),
        Key::linear(4.0, [3.0]),
    ]
}

fn coin_cfg(rule: Coincident, side: Side) -> Config {
    Config {
        coincident: rule,
        jump_side: side,
        ..Config::default()
    }
}

#[test]
fn keep_first_drops_later_equal_times() {
    let c = coin_cfg(Coincident::KeepFirst, Side::Right);
    assert_eq!(ev(coin(), &c, 2.0), 5.0);
    assert_eq!(ev(coin(), &c, 1.0), 3.0);
    assert_eq!(ev(coin(), &c, 3.0), 4.0);
}

#[test]
fn keep_last_drops_earlier_equal_times() {
    let c = coin_cfg(Coincident::KeepLast, Side::Left);
    assert_eq!(ev(coin(), &c, 2.0), 9.0);
    assert_eq!(ev(coin(), &c, 1.0), 5.0);
    assert_eq!(ev(coin(), &c, 3.0), 6.0);
}

#[test]
fn jump_keeps_both_and_picks_side_at_the_time() {
    let r = coin_cfg(Coincident::Jump, Side::Right);
    let l = coin_cfg(Coincident::Jump, Side::Left);
    assert_eq!(ev(coin(), &r, 2.0), 9.0);
    assert_eq!(ev(coin(), &l, 2.0), 5.0);
    // off the jump each side interpolates from its own keys
    assert_eq!(ev(coin(), &r, 1.0), 3.0);
    assert_eq!(ev(coin(), &r, 3.0), 6.0);
    assert_eq!(ev(coin(), &l, 3.0), 6.0);
}

#[test]
fn keep_first_uses_input_order_not_value() {
    let keys = vec![
        Key::linear(2.0, [9.0]),
        Key::linear(2.0, [5.0]),
        Key::linear(3.0, [0.0]),
    ];
    let first = coin_cfg(Coincident::KeepFirst, Side::Right);
    let last = coin_cfg(Coincident::KeepLast, Side::Right);
    assert_eq!(ev(keys.clone(), &first, 2.0), 9.0);
    assert_eq!(ev(keys, &last, 2.0), 5.0);
}

#[test]
fn jump_at_the_first_key_time() {
    let keys = vec![
        Key::linear(0.0, [1.0]),
        Key::linear(0.0, [7.0]),
        Key::linear(2.0, [9.0]),
    ];
    let r = coin_cfg(Coincident::Jump, Side::Right);
    let l = coin_cfg(Coincident::Jump, Side::Left);
    assert_eq!(ev(keys.clone(), &r, 0.0), 7.0);
    assert_eq!(ev(keys.clone(), &l, 0.0), 1.0);
    assert_eq!(ev(keys, &r, 1.0), 8.0);
}

#[test]
fn jump_at_the_last_key_time_with_linear_extrapolation() {
    let keys = vec![
        Key::linear(0.0, [0.0]),
        Key::linear(2.0, [4.0]),
        Key::linear(2.0, [10.0]),
    ];
    let c = Config {
        after: Outside::Linear,
        ..coin_cfg(Coincident::Jump, Side::Left)
    };
    assert_eq!(ev(keys.clone(), &c, 2.0), 4.0);
    // The end slope comes from the last open segment (slope 2) and extrapolates from the last key.
    assert_eq!(ev(keys, &c, 3.0), 12.0);
}

#[test]
fn empty_list_zero_and_fill() {
    let z = Config::default();
    let l = KeyList::<3>::new(vec![], &z).unwrap();
    assert_eq!(l.evaluate(&z, 1.0), [0.0; 3]);
    let f = Config {
        empty: Empty::Fill(3.5),
        ..z
    };
    assert_eq!(l.evaluate(&f, -9.0), [3.5; 3]);
    assert!(l.evaluate(&f, f64::NAN)[0] == 3.5);
}

fn snap_cfg(s: Snap, fps: f64) -> Config {
    Config {
        snap: s,
        frames_per_unit: fps,
        ..Config::default()
    }
}

fn ramp() -> Vec<Key<1>> {
    vec![Key::linear(-4.0, [-4.0]), Key::linear(8.0, [8.0])]
}

macro_rules! snap {
    ($name:ident, $mode:expr, $fps:expr, $t:expr, $want:expr) => {
        #[test]
        fn $name() {
            assert_eq!(ev(ramp(), &snap_cfg($mode, $fps), $t), $want);
        }
    };
}

snap!(snap_none_keeps_time, Snap::None, 1.0, 2.5, 2.5);
snap!(snap_floor_2_5, Snap::Floor, 1.0, 2.5, 2.0);
snap!(snap_up_2_5, Snap::RoundHalfUp, 1.0, 2.5, 3.0);
snap!(snap_even_2_5, Snap::RoundHalfEven, 1.0, 2.5, 2.0);
snap!(snap_floor_3_5, Snap::Floor, 1.0, 3.5, 3.0);
snap!(snap_up_3_5, Snap::RoundHalfUp, 1.0, 3.5, 4.0);
snap!(snap_even_3_5, Snap::RoundHalfEven, 1.0, 3.5, 4.0);
snap!(snap_floor_negative_half, Snap::Floor, 1.0, -0.5, -1.0);
snap!(snap_up_negative_half, Snap::RoundHalfUp, 1.0, -0.5, 0.0);
snap!(snap_even_negative_half, Snap::RoundHalfEven, 1.0, -0.5, 0.0);
snap!(
    snap_even_negative_one_half,
    Snap::RoundHalfEven,
    1.0,
    -1.5,
    -2.0
);
snap!(snap_up_below_half, Snap::RoundHalfUp, 1.0, 2.25, 2.0);
snap!(snap_even_above_half, Snap::RoundHalfEven, 1.0, 2.75, 3.0);
snap!(
    snap_up_tiny_below_half,
    Snap::RoundHalfUp,
    1.0,
    0.49999999999999994,
    0.0
);
snap!(
    snap_even_tiny_below_half,
    Snap::RoundHalfEven,
    1.0,
    0.49999999999999994,
    0.0
);
snap!(snap_floor_whole_frame, Snap::Floor, 1.0, 3.0, 3.0);
snap!(snap_fps4_floor, Snap::Floor, 4.0, 0.4, 0.25);
snap!(snap_fps4_round, Snap::RoundHalfUp, 4.0, 0.4, 0.5);
snap!(snap_fps4_round_low, Snap::RoundHalfUp, 4.0, 0.3, 0.25);
snap!(snap_fps4_half_up, Snap::RoundHalfUp, 4.0, 0.375, 0.5);
snap!(snap_fps4_half_even, Snap::RoundHalfEven, 4.0, 0.375, 0.5);
snap!(
    snap_fps4_half_even_down,
    Snap::RoundHalfEven,
    4.0,
    0.625,
    0.5
);
snap!(snap_fps_zero_is_ignored, Snap::Floor, 0.0, 2.5, 2.5);
snap!(snap_fps_negative_is_ignored, Snap::Floor, -2.0, 2.5, 2.5);
snap!(snap_fps_nan_is_ignored, Snap::Floor, f64::NAN, 2.5, 2.5);
snap!(
    snap_huge_time_unchanged,
    Snap::Floor,
    1.0,
    4503599627370497.0,
    8.0
);

#[test]
fn snap_applies_before_the_outside_rule() {
    // t = 8.6 floors to 8, which is the last key time (inside), so no extrapolation.
    let c = Config {
        after: Outside::Linear,
        ..snap_cfg(Snap::Floor, 1.0)
    };
    assert_eq!(ev(ramp(), &c, 8.6), 8.0);
}

#[test]
fn snap_helpers_match_their_definitions() {
    use key_eval::eval::{floor, round_half_even, round_half_up};
    assert_eq!(floor(-0.25), -1.0);
    assert_eq!(floor(-1.0), -1.0);
    assert_eq!(floor(2.999), 2.0);
    assert_eq!(floor(1e300), 1e300);
    assert_eq!(round_half_up(-2.5), -2.0);
    assert_eq!(round_half_even(-2.5), -2.0);
    assert_eq!(round_half_even(-3.5), -4.0);
    assert_eq!(round_half_even(0.5), 0.0);
    assert_eq!(round_half_even(1.5), 2.0);
}

#[test]
fn linear_after_uses_the_last_segment_when_it_is_shorter_than_one_unit() {
    let keys = vec![
        Key::linear(0.0, [0.0]),
        Key::linear(3.0, [3.0]),
        Key::linear(3.5, [5.0]),
    ];
    // the last segment has slope 4, the one before it slope 1
    assert_eq!(ev(keys, &both(Outside::Linear), 4.5), 9.0);
}

#[test]
fn linear_before_a_joint_bezier_uses_the_time_of_component_zero() {
    let a = Key::<2>::bezier(
        0.0,
        [0.0, 0.0],
        [[-1.0, 0.0], [-1.0, 0.0]],
        [[1.0, 2.0], [3.0, 4.0]],
    );
    let b = Key::<2>::bezier(
        4.0,
        [8.0, 8.0],
        [[3.0, 6.0], [3.0, 6.0]],
        [[5.0, 8.0], [5.0, 8.0]],
    );
    let mut cfg = both(Outside::Linear);
    let per = {
        let l = KeyList::new(vec![a, b], &cfg).unwrap();
        l.evaluate(&cfg, -1.0)
    };
    // per component: slopes 2 / 1 and 4 / 3
    assert_eq!(per[0], -2.0);
    assert_eq!(per[1], -4.0 / 3.0);
    cfg.vector_mode = VectorMode::Joint;
    let joint = KeyList::new(vec![a, b], &cfg).unwrap().evaluate(&cfg, -1.0);
    // one time handle (component 0) for both: slopes 2 / 1 and 4 / 1
    assert_eq!(joint[0], -2.0);
    assert_eq!(joint[1], -4.0);
}

#[test]
fn mirror_just_after_the_last_key_stays_inside_the_range() {
    // -1.163 + (0.9 - -1.163) rounds above 0.9, so the folded time must be clamped to the key range.
    let keys = vec![Key::linear(-1.163, [0.0]), Key::linear(0.9, [2.063])];
    let t = f64::from_bits(0.9f64.to_bits() + 1);
    let v = ev(keys, &both(Outside::Mirror), t);
    assert!((v - 2.063).abs() < 1e-12, "{v}");
}
