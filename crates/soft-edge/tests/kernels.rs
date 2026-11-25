use soft_edge::*;

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn box_r0_is_identity() {
    assert_eq!(box_kernel(0.0, FracRule::EndTaps).unwrap(), vec![1.0]);
    assert_eq!(box_kernel(0.0, FracRule::Lerp).unwrap(), vec![1.0]);
}

#[test]
fn box_r1() {
    let t = 1.0f32 / 3.0;
    assert_eq!(box_kernel(1.0, FracRule::EndTaps).unwrap(), vec![t, t, t]);
}

#[test]
fn box_r2() {
    assert_eq!(box_kernel(2.0, FracRule::Lerp).unwrap(), vec![0.2f32; 5]);
}

#[test]
fn box_half_end_taps_exact() {
    assert_eq!(
        box_kernel(0.5, FracRule::EndTaps).unwrap(),
        vec![0.25, 0.5, 0.25]
    );
}

#[test]
fn box_quarter_end_taps_exact() {
    // total = 1 + 0.5 = 1.5; weights 0.25/1.5 and 1/1.5
    let k = box_kernel(0.25, FracRule::EndTaps).unwrap();
    assert_eq!(k, vec![0.25f32 / 1.5, 1.0 / 1.5, 0.25f32 / 1.5]);
}

#[test]
fn box_one_and_half_end_taps() {
    let k = box_kernel(1.5, FracRule::EndTaps).unwrap();
    let total = 3.0f32 + 1.0;
    assert_eq!(k.len(), 5);
    assert_eq!(k[0], 0.5 / total);
    assert_eq!(k[2], 1.0 / total);
    assert_eq!(k[4], k[0]);
}

#[test]
fn box_half_lerp_exact() {
    // (1 - .5) * 1 + .5 * (1/3) = 0.5 + 1/6 ; ends .5 / 3
    let k = box_kernel(0.5, FracRule::Lerp).unwrap();
    let b = 1.0f32 / 3.0;
    assert_eq!(k, vec![0.5 * b, 0.5 * 1.0 + 0.5 * b, 0.5 * b]);
}

#[test]
fn box_lerp_sums_near_one() {
    for r in [0.1f32, 0.7, 1.3, 4.9] {
        let s: f32 = box_kernel(r, FracRule::Lerp).unwrap().iter().sum();
        assert!((s - 1.0).abs() < 1e-6);
    }
}

#[test]
fn box_end_taps_sums_near_one() {
    for r in [0.1f32, 0.7, 1.3, 4.9] {
        let s: f32 = box_kernel(r, FracRule::EndTaps).unwrap().iter().sum();
        assert!((s - 1.0).abs() < 1e-6);
    }
}

#[test]
fn box_kernels_symmetric() {
    for rule in [FracRule::EndTaps, FracRule::Lerp] {
        for r in [0.3f32, 1.0, 2.6] {
            let k = box_kernel(r, rule).unwrap();
            let mut rev = k.clone();
            rev.reverse();
            assert_eq!(bits(&k), bits(&rev));
            assert_eq!(k.len() % 2, 1);
        }
    }
}

#[test]
fn box_length_rule() {
    assert_eq!(box_kernel(2.0, FracRule::EndTaps).unwrap().len(), 5);
    assert_eq!(box_kernel(2.01, FracRule::EndTaps).unwrap().len(), 7);
    assert_eq!(box_kernel(2.01, FracRule::Lerp).unwrap().len(), 7);
}

#[test]
fn box_bad_radius() {
    for r in [-1.0f32, f32::NAN, f32::INFINITY, 3.0e6] {
        assert!(box_kernel(r, FracRule::EndTaps).is_err());
    }
}

#[test]
fn exp_zero_is_one() {
    assert_eq!(exp_neg(0.0, 12), 1.0);
}

#[test]
fn exp_inf_is_zero() {
    assert_eq!(exp_neg(f32::INFINITY, 12), 0.0);
}

#[test]
fn exp_matches_libm_small() {
    for i in 0..40 {
        let t = i as f32 * 0.25;
        let want = (-(t as f64)).exp();
        let got = exp_neg(t, 12) as f64;
        assert!(
            (got - want).abs() < 2e-6 * want.max(1e-3),
            "t={t} got={got} want={want}"
        );
    }
}

#[test]
fn exp_one_term_is_one_minus_u() {
    // t = 0.5 needs no halving: terms=1 gives 1 - 0.5
    assert_eq!(exp_neg(0.5, 1), 0.5);
}

#[test]
fn exp_two_terms_hand() {
    // u = 0.5: 1 - 0.5 + 0.125
    assert_eq!(exp_neg(0.5, 2), 0.625);
}

#[test]
fn exp_halving_square() {
    // t = 1 -> u = 0.5, one halving: (1 - 0.5)^2 with one term
    assert_eq!(exp_neg(1.0, 1), 0.25);
}

#[test]
fn exp_large_underflows_to_zero() {
    assert_eq!(exp_neg(1.0e30, 12), 0.0);
}

fn gk(spec: GaussSpec, cfg: &Config) -> Vec<f32> {
    gaussian_kernel(spec, cfg).unwrap()
}

#[test]
fn gauss_sigma0_identity() {
    assert_eq!(gk(GaussSpec::Sigma(0.0), &Config::default()), vec![1.0]);
}

#[test]
fn gauss_length_from_truncation() {
    let c = Config::default();
    assert_eq!(gk(GaussSpec::Sigma(1.0), &c).len(), 7);
    assert_eq!(gk(GaussSpec::Sigma(1.1), &c).len(), 9); // ceil(3.3) = 4
    let c2 = Config { trunc_k: 2.0, ..c };
    assert_eq!(gk(GaussSpec::Sigma(1.0), &c2).len(), 5);
}

#[test]
fn gauss_radius_maps_to_sigma() {
    let c = Config::default();
    assert_eq!(
        bits(&gk(GaussSpec::Radius(2.0), &c)),
        bits(&gk(GaussSpec::Sigma(1.0), &c))
    );
    let c4 = Config {
        radius_to_sigma: 4.0,
        ..c
    };
    assert_eq!(
        bits(&gk(GaussSpec::Radius(2.0), &c4)),
        bits(&gk(GaussSpec::Sigma(0.5), &c))
    );
}

#[test]
fn gauss_normalised_symmetric() {
    for &m in &[ExpMethod::Series, ExpMethod::Recurrence] {
        let c = Config {
            exp_method: m,
            ..Config::default()
        };
        let k = gk(GaussSpec::Sigma(1.7), &c);
        let s: f32 = k.iter().sum();
        assert!((s - 1.0).abs() < 1e-6);
        let mut r = k.clone();
        r.reverse();
        assert_eq!(bits(&k), bits(&r));
    }
}

#[test]
fn gauss_matches_f64_reference() {
    for &m in &[ExpMethod::Series, ExpMethod::Recurrence] {
        for &sigma in &[0.5f32, 1.0, 2.3, 4.0] {
            let c = Config {
                exp_method: m,
                ..Config::default()
            };
            let k = gk(GaussSpec::Sigma(sigma), &c);
            let hw = (k.len() / 2) as i64;
            let raw: Vec<f64> = (-hw..=hw)
                .map(|x| (-((x * x) as f64) / (2.0 * (sigma as f64).powi(2))).exp())
                .collect();
            let tot: f64 = raw.iter().sum();
            for (a, b) in k.iter().zip(raw.iter()) {
                assert!((*a as f64 - b / tot).abs() < 3e-6, "{m:?} sigma={sigma}");
            }
        }
    }
}

#[test]
fn gauss_methods_agree_closely() {
    let a = Config::default();
    let b = Config {
        exp_method: ExpMethod::Recurrence,
        ..a
    };
    let (ka, kb) = (gk(GaussSpec::Sigma(2.0), &a), gk(GaussSpec::Sigma(2.0), &b));
    for (x, y) in ka.iter().zip(kb.iter()) {
        assert!((x - y).abs() < 1e-5);
    }
}

#[test]
fn gauss_peak_is_centre_and_decreasing() {
    let k = gk(GaussSpec::Sigma(1.5), &Config::default());
    let c = k.len() / 2;
    for i in c..k.len() - 1 {
        assert!(k[i] > k[i + 1]);
    }
}

#[test]
fn gauss_recurrence_hand_sigma_half() {
    // sigma = 0.5: denom = 0.5, q = exp(-2); hw = ceil(1.5) = 2; weights q^(x^2)
    let c = Config {
        exp_method: ExpMethod::Recurrence,
        ..Config::default()
    };
    let k = gk(GaussSpec::Sigma(0.5), &c);
    assert_eq!(k.len(), 5);
    let q = (-2.0f64).exp();
    let raw = [q.powi(4), q, 1.0, q, q.powi(4)];
    let tot: f64 = raw.iter().sum();
    for (a, b) in k.iter().zip(raw.iter()) {
        assert!((*a as f64 - b / tot).abs() < 1e-6);
    }
}

#[test]
fn gauss_bad_parameters() {
    let c = Config::default();
    for s in [
        GaussSpec::Sigma(-1.0),
        GaussSpec::Sigma(f32::NAN),
        GaussSpec::Sigma(f32::INFINITY),
        GaussSpec::Radius(f32::NAN),
    ] {
        assert!(gaussian_kernel(s, &c).is_err());
    }
    assert!(gaussian_kernel(GaussSpec::Sigma(1.0), &Config { trunc_k: 0.0, ..c }).is_err());
    assert!(gaussian_kernel(
        GaussSpec::Sigma(1.0),
        &Config {
            trunc_k: f32::NAN,
            ..c
        }
    )
    .is_err());
    assert!(gaussian_kernel(GaussSpec::Sigma(1.0), &Config { exp_terms: 0, ..c }).is_err());
    assert!(gaussian_kernel(
        GaussSpec::Radius(1.0),
        &Config {
            radius_to_sigma: 0.0,
            ..c
        }
    )
    .is_err());
    assert!(gaussian_kernel(GaussSpec::Sigma(1.0e9), &c).is_err());
}

#[test]
fn gauss_more_terms_no_worse() {
    let lo = Config {
        exp_terms: 3,
        ..Config::default()
    };
    let hi = Config {
        exp_terms: 14,
        ..Config::default()
    };
    let sigma = 1.0f32;
    let err = |c: &Config| {
        let k = gk(GaussSpec::Sigma(sigma), c);
        let hw = (k.len() / 2) as i64;
        let raw: Vec<f64> = (-hw..=hw)
            .map(|x| (-((x * x) as f64) / 2.0).exp())
            .collect();
        let tot: f64 = raw.iter().sum();
        k.iter()
            .zip(raw.iter())
            .map(|(a, b)| (*a as f64 - b / tot).abs())
            .fold(0.0, f64::max)
    };
    assert!(err(&hi) < err(&lo));
}
