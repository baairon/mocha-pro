//! Hold, linear and Hermite keys. Expected values are worked by hand in rational arithmetic.

use key_eval::{Config, Key, KeyList, Side};

fn eval1(keys: Vec<Key<1>>, cfg: &Config, t: f64) -> f64 {
    KeyList::new(keys, cfg).unwrap().evaluate(cfg, t)[0]
}

fn hold_keys() -> Vec<Key<1>> {
    vec![Key::hold(0.0, [10.0]), Key::hold(4.0, [20.0])]
}

fn lin_keys() -> Vec<Key<1>> {
    vec![Key::linear(0.0, [10.0]), Key::linear(4.0, [20.0])]
}

// Smoothstep Hermite: zero slopes, values 0 to 8 over time 0 to 4.
fn herm_flat() -> Vec<Key<1>> {
    vec![
        Key::hermite(0.0, [0.0], [0.0], [0.0]),
        Key::hermite(4.0, [8.0], [0.0], [0.0]),
    ]
}

// Values 0 and 0, out slope 1 on the first key, in slope 1 on the second.
fn herm_slope() -> Vec<Key<1>> {
    vec![
        Key::hermite(0.0, [0.0], [0.0], [1.0]),
        Key::hermite(4.0, [0.0], [1.0], [0.0]),
    ]
}

macro_rules! exact {
    ($name:ident, $keys:expr, $t:expr, $want:expr) => {
        #[test]
        fn $name() {
            let got = eval1($keys, &Config::default(), $t);
            assert_eq!(got.to_bits(), ($want as f64).to_bits(), "got {got}");
        }
    };
}

// hold: the value of the first key over the open segment, the last key value at and after the end.
exact!(hold_quarter, hold_keys(), 1.0, 10.0);
exact!(hold_half, hold_keys(), 2.0, 10.0);
exact!(hold_three_quarter, hold_keys(), 3.0, 10.0);
exact!(hold_at_first_key, hold_keys(), 0.0, 10.0);
exact!(hold_at_last_key, hold_keys(), 4.0, 20.0);
exact!(hold_just_before_last, hold_keys(), 3.9375, 10.0);

// linear: 10 + 10 * s
exact!(linear_quarter, lin_keys(), 1.0, 12.5);
exact!(linear_half, lin_keys(), 2.0, 15.0);
exact!(linear_three_quarter, lin_keys(), 3.0, 17.5);
exact!(linear_at_first_key, lin_keys(), 0.0, 10.0);
exact!(linear_at_last_key, lin_keys(), 4.0, 20.0);
exact!(linear_eighth, lin_keys(), 0.5, 11.25);

// Hermite smoothstep: 8 * (3 s^2 - 2 s^3): s = 1/4 gives 8 * 5/32, 1/2 gives 4, 3/4 gives 8 * 27/32.
exact!(hermite_flat_quarter, herm_flat(), 1.0, 1.25);
exact!(hermite_flat_half, herm_flat(), 2.0, 4.0);
exact!(hermite_flat_three_quarter, herm_flat(), 3.0, 6.75);
exact!(hermite_flat_at_first, herm_flat(), 0.0, 0.0);
exact!(hermite_flat_at_last, herm_flat(), 4.0, 8.0);

// Hermite with slopes 1 and 1 on zero values over L = 4:
// s = 1/4: 4 * (9/64 - 3/64) = 3/8; s = 1/2: 0; s = 3/4: 4 * (3/64 - 9/64) = -3/8.
exact!(hermite_slope_quarter, herm_slope(), 1.0, 0.375);
exact!(hermite_slope_half, herm_slope(), 2.0, 0.0);
exact!(hermite_slope_three_quarter, herm_slope(), 3.0, -0.375);

#[test]
fn hermite_with_chord_slope_is_linear() {
    // values 0 to 8 over 4: chord slope 2 on both ends gives the straight line 2 t.
    let keys = vec![
        Key::hermite(0.0, [0.0], [2.0], [2.0]),
        Key::hermite(4.0, [8.0], [2.0], [2.0]),
    ];
    for (t, want) in [(1.0, 2.0), (2.0, 4.0), (3.0, 6.0)] {
        assert_eq!(eval1(keys.clone(), &Config::default(), t), want);
    }
}

#[test]
fn hermite_slope_uses_out_of_first_and_in_of_second() {
    // in slope of the first key and out slope of the last key never enter the segment.
    let a = vec![
        Key::hermite(0.0, [0.0], [9.0], [1.0]),
        Key::hermite(4.0, [0.0], [1.0], [9.0]),
    ];
    assert_eq!(eval1(a, &Config::default(), 1.0), 0.375);
}

#[test]
fn hermite_segment_uses_kind_of_first_key_only() {
    // The first key is linear: the second key's Hermite slopes are ignored on that segment.
    let keys = vec![
        Key::linear(0.0, [0.0]),
        Key::hermite(4.0, [8.0], [77.0], [77.0]),
    ];
    assert_eq!(eval1(keys, &Config::default(), 1.0), 2.0);
}

#[test]
fn mixed_kinds_across_three_keys() {
    let keys = vec![
        Key::hold(0.0, [1.0]),
        Key::linear(2.0, [3.0]),
        Key::linear(6.0, [7.0]),
    ];
    let c = Config::default();
    assert_eq!(eval1(keys.clone(), &c, 1.0), 1.0);
    assert_eq!(eval1(keys.clone(), &c, 2.0), 3.0);
    assert_eq!(eval1(keys.clone(), &c, 4.0), 5.0);
    assert_eq!(eval1(keys, &c, 6.0), 7.0);
}

#[test]
fn hold_side_left_gives_previous_hold_value_at_key() {
    let keys = vec![
        Key::hold(0.0, [1.0]),
        Key::hold(2.0, [5.0]),
        Key::hold(4.0, [9.0]),
    ];
    let c = Config {
        hold_side: Side::Left,
        ..Config::default()
    };
    assert_eq!(eval1(keys.clone(), &c, 2.0), 1.0);
    assert_eq!(eval1(keys.clone(), &c, 4.0), 5.0);
    assert_eq!(eval1(keys.clone(), &c, 0.0), 1.0);
    assert_eq!(eval1(keys, &c, 3.0), 5.0);
}

#[test]
fn hold_side_right_gives_key_value_at_key() {
    let keys = vec![
        Key::hold(0.0, [1.0]),
        Key::hold(2.0, [5.0]),
        Key::hold(4.0, [9.0]),
    ];
    let c = Config {
        hold_side: Side::Right,
        ..Config::default()
    };
    assert_eq!(eval1(keys.clone(), &c, 2.0), 5.0);
    assert_eq!(eval1(keys, &c, 4.0), 9.0);
}

#[test]
fn hold_side_left_ignores_non_hold_previous_segment() {
    let keys = vec![Key::linear(0.0, [1.0]), Key::hold(2.0, [5.0])];
    let c = Config {
        hold_side: Side::Left,
        ..Config::default()
    };
    assert_eq!(eval1(keys, &c, 2.0), 5.0);
}

#[test]
fn hold_side_left_then_linear_key_value_when_previous_is_hold() {
    // key 1 is linear, but the segment before it is hold: Left still gives the held value 1.
    let keys = vec![
        Key::hold(0.0, [1.0]),
        Key::linear(2.0, [5.0]),
        Key::linear(4.0, [9.0]),
    ];
    let c = Config {
        hold_side: Side::Left,
        ..Config::default()
    };
    assert_eq!(eval1(keys.clone(), &c, 2.0), 1.0);
    assert_eq!(eval1(keys, &c, 3.0), 7.0);
}

#[test]
fn unsorted_input_is_sorted_stably() {
    let keys = vec![Key::linear(4.0, [20.0]), Key::linear(0.0, [10.0])];
    assert_eq!(eval1(keys, &Config::default(), 1.0), 12.5);
}

#[test]
fn vector_components_are_independent_for_linear() {
    let keys = vec![
        Key::linear(0.0, [0.0, 10.0, -4.0]),
        Key::linear(4.0, [4.0, 20.0, 4.0]),
    ];
    let c = Config::default();
    let v = KeyList::new(keys, &c).unwrap().evaluate(&c, 1.0);
    assert_eq!(v, [1.0, 12.5, -2.0]);
}

#[test]
fn vector_components_are_independent_for_hermite() {
    let keys = vec![
        Key::hermite(0.0, [0.0, 0.0], [0.0, 0.0], [0.0, 1.0]),
        Key::hermite(4.0, [8.0, 0.0], [0.0, 1.0], [0.0, 0.0]),
    ];
    let c = Config::default();
    let v = KeyList::new(keys, &c).unwrap().evaluate(&c, 1.0);
    assert_eq!(v, [1.25, 0.375]);
}

#[test]
fn zero_dimension_lists_evaluate_to_empty_arrays() {
    let c = Config::default();
    let l = KeyList::<0>::new(vec![Key::linear(0.0, []), Key::linear(1.0, [])], &c).unwrap();
    assert_eq!(l.evaluate(&c, 0.5), []);
}

#[test]
fn non_finite_field_is_refused() {
    let c = Config::default();
    let bad = vec![Key::linear(0.0, [1.0]), Key::linear(1.0, [f64::NAN])];
    assert_eq!(
        KeyList::new(bad, &c).unwrap_err(),
        key_eval::KeyError::NonFinite(1)
    );
    let inf = vec![Key::linear(f64::INFINITY, [1.0])];
    assert!(KeyList::new(inf, &c).is_err());
    let h = vec![Key::hermite(0.0, [1.0], [f64::NAN], [0.0])];
    assert!(KeyList::new(h, &c).is_err());
    let b = vec![Key::bezier(
        0.0,
        [1.0],
        [[0.0, f64::INFINITY]],
        [[0.0, 0.0]],
    )];
    assert!(KeyList::new(b, &c).is_err());
}

#[test]
fn non_finite_time_gives_nan() {
    let c = Config::default();
    let l = KeyList::new(vec![Key::linear(0.0, [1.0]), Key::linear(1.0, [2.0])], &c).unwrap();
    assert!(l.evaluate(&c, f64::NAN)[0].is_nan());
    assert!(l.evaluate(&c, f64::INFINITY)[0].is_nan());
    assert!(l.evaluate(&c, f64::NEG_INFINITY)[0].is_nan());
}

#[test]
fn single_key_is_constant_under_every_outside_rule() {
    use key_eval::Outside::*;
    for rule in [Hold, Linear, Cycle, CycleOffset, Mirror] {
        let c = Config {
            before: rule,
            after: rule,
            ..Config::default()
        };
        let l = KeyList::new(vec![Key::linear(2.0, [7.0])], &c).unwrap();
        for t in [-5.0, 2.0, 9.0] {
            assert_eq!(l.evaluate(&c, t), [7.0]);
        }
    }
}

#[test]
fn accessors_report_the_normalized_list() {
    let c = Config::default();
    let l = KeyList::new(vec![Key::linear(1.0, [1.0]), Key::linear(0.0, [0.0])], &c).unwrap();
    assert_eq!(l.len(), 2);
    assert!(!l.is_empty());
    assert_eq!(l.keys()[0].time, 0.0);
    let e = KeyList::<1>::new(vec![], &c).unwrap();
    assert!(e.is_empty());
    assert_eq!(e.len(), 0);
}
