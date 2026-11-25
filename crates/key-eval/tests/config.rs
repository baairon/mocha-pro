//! The configuration space, determinism, and pinned bit patterns.

use key_eval::{Config, Key, KeyList, Outside, Solver};
use std::collections::HashSet;

#[test]
fn default_config_values() {
    let c = Config::default();
    assert_eq!(c.frames_per_unit, 1.0);
    assert_eq!(c.before, Outside::Hold);
    assert_eq!(c.after, Outside::Hold);
    assert_eq!(c.solver, Solver::Cardano { polish: 2 });
}

#[test]
fn enumeration_has_the_product_size() {
    // 2 hold sides * 2 jump sides * 4 snaps * 2 vector modes * 2 handle modes * 3 clamps * 5 solvers
    // * 5 before * 5 after * 3 coincident * 2 empty
    assert_eq!(
        Config::enumerate().len(),
        2 * 2 * 4 * 2 * 2 * 3 * 5 * 5 * 5 * 3 * 2
    );
    assert_eq!(Config::enumerate().len(), 144_000);
}

#[test]
fn enumeration_has_no_duplicates() {
    let all = Config::enumerate();
    let set: HashSet<String> = all.iter().map(|c| format!("{c:?}")).collect();
    assert_eq!(set.len(), all.len());
}

#[test]
fn enumeration_contains_the_default_except_for_nothing() {
    let d = Config::default();
    assert!(Config::enumerate().contains(&d));
}

#[test]
fn axes_sizes() {
    let a = Config::axes();
    assert_eq!(a.hold_side.len(), 2);
    assert_eq!(a.jump_side.len(), 2);
    assert_eq!(a.snap.len(), 4);
    assert_eq!(a.vector_mode.len(), 2);
    assert_eq!(a.handle_mode.len(), 2);
    assert_eq!(a.clamp.len(), 3);
    assert_eq!(a.solver.len(), 5);
    assert_eq!(a.outside.len(), 5);
    assert_eq!(a.coincident.len(), 3);
    assert_eq!(a.empty.len(), 2);
}

fn probe_keys() -> Vec<Key<2>> {
    vec![
        Key::bezier(0.0, [0.0, 1.0], [[0.0, 0.0]; 2], [[1.0, 0.5], [1.0, 2.0]]),
        Key::bezier(3.0, [4.0, -1.0], [[2.0, 3.0], [2.0, -2.0]], [[4.0, 4.0]; 2]),
        Key::bezier(3.0, [5.0, 0.0], [[3.0, 5.0]; 2], [[3.5, 6.0]; 2]),
        Key::bezier(6.0, [2.0, 2.0], [[5.0, 1.0]; 2], [[0.0, 0.0]; 2]),
    ]
}

#[test]
fn every_configuration_gives_finite_deterministic_values() {
    let times = [-2.5, 0.0, 0.7, 3.0, 4.4, 6.0, 8.25];
    let mut acc = 0xcbf2_9ce4_8422_2325u64;
    for cfg in Config::enumerate() {
        let l = KeyList::new(probe_keys(), &cfg).unwrap();
        for &t in &times {
            let a = l.evaluate(&cfg, t);
            let b = l.evaluate(&cfg, t);
            for c in 0..2 {
                assert!(a[c].is_finite(), "{cfg:?} {t}");
                assert_eq!(a[c].to_bits(), b[c].to_bits());
                acc = (acc ^ a[c].to_bits()).wrapping_mul(0x100_0000_01b3);
            }
        }
    }
    // One digest over the whole space, pinned so a change of any result shows up.
    assert_eq!(acc, PINNED_SPACE_DIGEST, "digest {acc:#x}");
}

// Pinned from the first run of this test (a regression pin, not a hand computed value).
const PINNED_SPACE_DIGEST: u64 = 0x4708_04ce_81c4_b3a5;

#[test]
fn repeated_runs_give_identical_bits() {
    let cfg = Config::default();
    let l = KeyList::new(probe_keys(), &cfg).unwrap();
    let first: Vec<u64> = (0..200)
        .map(|i| l.evaluate(&cfg, i as f64 * 0.037)[0].to_bits())
        .collect();
    for _ in 0..3 {
        let again: Vec<u64> = (0..200)
            .map(|i| l.evaluate(&cfg, i as f64 * 0.037)[0].to_bits())
            .collect();
        assert_eq!(first, again);
    }
}

#[test]
fn key_values_are_returned_exactly_at_key_times_for_every_solver() {
    for solver in Config::axes().solver {
        let cfg = Config {
            solver,
            ..Config::default()
        };
        let l = KeyList::new(
            vec![
                Key::bezier(0.0, [1.5], [[0.0, 0.0]], [[1.0, 2.0]]),
                Key::bezier(2.0, [3.25], [[1.0, 3.0]], [[0.0, 0.0]]),
            ],
            &cfg,
        )
        .unwrap();
        assert_eq!(l.evaluate(&cfg, 0.0), [1.5]);
        assert_eq!(l.evaluate(&cfg, 2.0), [3.25]);
    }
}

#[test]
fn pinned_bit_patterns_of_a_non_dyadic_bezier() {
    // Pinned from this build (regression pins; the same on every IEEE 754 platform by construction).
    let cfg = Config::default();
    let l = KeyList::new(
        vec![
            Key::bezier(0.0, [0.0], [[0.0, 0.0]], [[0.3, 1.0]]),
            Key::bezier(1.0, [1.0], [[0.6, 0.2]], [[0.0, 0.0]]),
        ],
        &cfg,
    )
    .unwrap();
    let got: Vec<u64> = [0.1, 0.5, 0.9]
        .iter()
        .map(|&t| l.evaluate(&cfg, t)[0].to_bits())
        .collect();
    assert_eq!(got, PINNED_BEZIER.to_vec(), "{got:#x?}");
    // Independent slow check of the same values: 200 bisections on the Bernstein form.
    for (&t, &bits) in [0.1, 0.5, 0.9].iter().zip(PINNED_BEZIER.iter()) {
        let (mut lo, mut hi) = (0.0f64, 1.0f64);
        let x = |u: f64| {
            let w = 1.0 - u;
            3.0 * w * w * u * 0.3 + 3.0 * w * u * u * 0.6 + u * u * u
        };
        for _ in 0..200 {
            let m = 0.5 * (lo + hi);
            if x(m) < t {
                lo = m;
            } else {
                hi = m;
            }
        }
        let u = 0.5 * (lo + hi);
        let w = 1.0 - u;
        let y = 3.0 * w * w * u * 1.0 + 3.0 * w * u * u * 0.2 + u * u * u;
        assert!((f64::from_bits(bits) - y).abs() < 1e-12);
    }
}

const PINNED_BEZIER: [u64; 3] = [0x3fd158b06711c24e, 0x3fe29357af487542, 0x3fea8293011e8c50];
