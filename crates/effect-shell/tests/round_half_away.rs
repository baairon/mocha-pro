//! The no_std rounding helper used by the freestanding build must equal `f64::round` everywhere.

use effect_shell::setup::round_half_away;

#[test]
fn equals_std_round_on_ties_edges_and_a_sweep() {
    let mut cases = vec![
        0.0,
        -0.0,
        0.49999999999999994,
        -0.49999999999999994,
        0.5,
        -0.5,
        1.5,
        -1.5,
        2.5,
        -2.5,
        4503599627370495.5,
        4503599627370496.0,
        -4503599627370497.0,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut x = -300.0;
    while x < 300.0 {
        cases.push(x);
        x += 0.125;
    }
    for c in cases {
        let (a, b) = (round_half_away(c), c.round());
        // Value equality: only the sign of a zero result may differ, and the caller casts to an integer.
        assert!(a == b || (a.is_nan() && b.is_nan()), "round of {c:e}");
    }
    assert!(round_half_away(f64::NAN).is_nan());
}
