use soft_edge::*;

#[test]
fn tiny_sigma_series_matches_recurrence_identity() {
    for sigma in [1e-30f32, 1e-24, 1e-23, 1e-3] {
        for exp_method in [ExpMethod::Series, ExpMethod::Recurrence] {
            let cfg = Config {
                exp_method,
                ..Config::default()
            };
            let k = gaussian_kernel(GaussSpec::Sigma(sigma), &cfg).unwrap();
            assert_eq!(k.len() % 2, 1, "{sigma:e} {exp_method:?}");
            let mid = k.len() / 2;
            assert_eq!(k[mid], 1.0, "{sigma:e} {exp_method:?}");
            assert!(k.iter().enumerate().all(|(i, &v)| i == mid || v < 1e-6));
        }
    }
}

#[test]
fn tiny_radius_gaussian_is_not_an_error() {
    let k = gaussian_kernel(GaussSpec::Radius(1e-30), &Config::default()).unwrap();
    assert_eq!(k[k.len() / 2], 1.0);
}

#[test]
fn huge_dimensions_are_a_size_mismatch_not_a_panic() {
    let cfg = Config::default();
    assert_eq!(
        box_blur(&[], usize::MAX, 2, 1.0, &cfg),
        Err(Error::SizeMismatch)
    );
    assert_eq!(
        gaussian_blur(&[], 2, usize::MAX, GaussSpec::Sigma(1.0), &cfg),
        Err(Error::SizeMismatch)
    );
    assert_eq!(
        signed_distance(&[], usize::MAX, 2, &cfg),
        Err(Error::SizeMismatch)
    );
    assert_eq!(
        feather(&[], usize::MAX, 2, 1.0, &cfg),
        Err(Error::SizeMismatch)
    );
    assert_eq!(
        box_blur_repeated(&[], usize::MAX, 2, 1.0, 1, &cfg),
        Err(Error::SizeMismatch)
    );
}

#[test]
fn gaussian_kernels_are_normalised_to_a_few_ulps() {
    for exp_method in [ExpMethod::Series, ExpMethod::Recurrence] {
        let cfg = Config {
            exp_method,
            ..Config::default()
        };
        for sigma in [0.3f32, 0.8, 1.7, 4.0, 9.5, 20.0] {
            let k = gaussian_kernel(GaussSpec::Sigma(sigma), &cfg).unwrap();
            let s: f64 = k.iter().map(|&v| v as f64).sum();
            assert!((s - 1.0).abs() < 5e-7, "{sigma} {exp_method:?} {s}");
        }
    }
}
