mod common;
use common::{bits, cfg, img, ref_lanczos, run1};
use resample::{
    resample, AccumOrder, AlphaMode, Border, Config, ConfigSpace, Error, Filter, Footprint, Image,
    Layout, Normalize,
};
use std::collections::HashSet;

fn two_pixels() -> Vec<f32> {
    // channel 0 colour, channel 1 alpha: (1, 1) then (3, 0)
    vec![1.0, 1.0, 3.0, 0.0]
}

fn alpha_cfg(mode: AlphaMode) -> Config {
    Config {
        alpha: mode,
        alpha_channel: Some(1),
        ..cfg(Filter::Bilinear)
    }
}

fn mid(mode: AlphaMode) -> Vec<f32> {
    let d = two_pixels();
    resample(&img(&d, 2, 1, 2), 1, 1, |_, y| (1.0, y), &alpha_cfg(mode)).unwrap()
}

#[test]
fn straight_interpolation_of_colour_and_alpha() {
    // midpoint u = 0.5 : colour (1 + 3) / 2 = 2, alpha 0.5
    assert_eq!(mid(AlphaMode::Straight), vec![2.0, 0.5]);
}

#[test]
fn premultiplied_interpolation_of_colour_and_alpha() {
    // premultiplied colours 1 and 0 average to 0.5 ; alpha 0.5 ; colour 0.5 / 0.5 = 1
    assert_eq!(mid(AlphaMode::Premultiplied), vec![1.0, 0.5]);
}

#[test]
fn premultiplied_equals_straight_for_opaque_data() {
    let d: Vec<f32> = (0..16i32)
        .map(|i| {
            if i.rem_euclid(2) == 1 {
                1.0
            } else {
                i as f32 * 0.25
            }
        })
        .collect();
    let map = |x: f64, y: f64| (x * 0.6 + 0.2, y * 0.7);
    let a = resample(
        &img(&d, 4, 2, 2),
        3,
        2,
        map,
        &alpha_cfg(AlphaMode::Straight),
    )
    .unwrap();
    let b = resample(
        &img(&d, 4, 2, 2),
        3,
        2,
        map,
        &alpha_cfg(AlphaMode::Premultiplied),
    )
    .unwrap();
    for (x, y) in a.iter().zip(&b) {
        assert!((x - y).abs() < 1e-6);
    }
}

#[test]
fn premultiplied_zero_alpha_gives_zero_colour() {
    let d = vec![5.0f32, 0.0, 7.0, 0.0];
    let out = resample(
        &img(&d, 2, 1, 2),
        1,
        1,
        |_, y| (1.0, y),
        &alpha_cfg(AlphaMode::Premultiplied),
    )
    .unwrap();
    assert_eq!(out, vec![0.0, 0.0]);
}

#[test]
fn premultiplied_three_quarter_weights_by_hand() {
    // u = 0.25 : weights 0.75 and 0.25 ; premultiplied colour 0.75*1 + 0.25*0 = 0.75 ;
    // alpha 0.75 ; colour 1.0. Straight colour is 0.75*1 + 0.25*3 = 1.5.
    let d = two_pixels();
    let map = |_: f64, y: f64| (0.75, y);
    let p = resample(
        &img(&d, 2, 1, 2),
        1,
        1,
        map,
        &alpha_cfg(AlphaMode::Premultiplied),
    )
    .unwrap();
    let s = resample(
        &img(&d, 2, 1, 2),
        1,
        1,
        map,
        &alpha_cfg(AlphaMode::Straight),
    )
    .unwrap();
    assert_eq!(p, vec![1.0, 0.75]);
    assert_eq!(s, vec![1.5, 0.75]);
}

#[test]
fn alpha_channel_can_be_any_channel() {
    // alpha first : (alpha, colour)
    let d = vec![1.0f32, 1.0, 0.0, 3.0];
    let c = Config {
        alpha: AlphaMode::Premultiplied,
        alpha_channel: Some(0),
        ..cfg(Filter::Bilinear)
    };
    let out = resample(&img(&d, 2, 1, 2), 1, 1, |_, y| (1.0, y), &c).unwrap();
    assert_eq!(out, vec![0.5, 1.0]);
}

#[test]
fn alpha_channel_without_premultiplication_is_plain() {
    let d = two_pixels();
    let c = Config {
        alpha_channel: Some(1),
        ..cfg(Filter::Bilinear)
    };
    let a = resample(&img(&d, 2, 1, 2), 1, 1, |_, y| (1.0, y), &c).unwrap();
    assert_eq!(a, vec![2.0, 0.5]);
}

#[test]
fn premultiplied_works_on_planar_data() {
    let d = vec![1.0f32, 3.0, 1.0, 0.0]; // colour plane, alpha plane
    let im = Image {
        data: &d,
        width: 2,
        height: 1,
        channels: 2,
        layout: Layout::Planar,
    };
    let out = resample(
        &im,
        1,
        1,
        |_, y| (1.0, y),
        &alpha_cfg(AlphaMode::Premultiplied),
    )
    .unwrap();
    assert_eq!(out, vec![1.0, 0.5]);
}

fn lanczos_const(normalize: Normalize) -> f32 {
    // constant 1 image, u = 0.5 : the four Lanczos 2 weights sum to a little over 1
    let d = vec![1.0f32; 16];
    let c = Config {
        normalize,
        ..cfg(Filter::Lanczos { lobes: 2 })
    };
    resample(&img(&d, 4, 4, 1), 1, 1, |_, _| (2.0, 2.0), &c).unwrap()[0]
}

#[test]
fn normalisation_none_keeps_kernel_gain() {
    let w = |t: f64| ref_lanczos(t, 2.0);
    let s1 = 2.0 * (w(0.5) + w(1.5));
    let want = s1 * s1;
    assert!(want > 1.02);
    assert!((f64::from(lanczos_const(Normalize::None)) - want).abs() < 1e-6);
}

#[test]
fn normalisation_sum_gives_unit_gain() {
    assert!((lanczos_const(Normalize::Sum) - 1.0).abs() < 1e-6);
}

#[test]
fn normalisation_sum_in_bounds_equals_sum_when_fully_inside() {
    assert_eq!(
        lanczos_const(Normalize::SumInBounds).to_bits(),
        lanczos_const(Normalize::Sum).to_bits()
    );
}

#[test]
fn sum_in_bounds_compensates_a_zero_border() {
    let d = [1.0f32, 1.0];
    let run = |n| {
        let c = Config {
            border: Border::Zero,
            normalize: n,
            ..cfg(Filter::Bilinear)
        };
        run1(&d, 2, 1, 1, 1, |_, y| (0.0, y), &c)
    };
    // u = -0.5 : taps -1 and 0 with weight 0.5 each
    assert_eq!(run(Normalize::None), vec![0.5]);
    assert_eq!(run(Normalize::Sum), vec![0.5]);
    assert_eq!(run(Normalize::SumInBounds), vec![1.0]);
}

#[test]
fn sum_in_bounds_with_nothing_inside_is_zero() {
    let d = [1.0f32, 1.0];
    let c = Config {
        border: Border::Zero,
        normalize: Normalize::SumInBounds,
        ..cfg(Filter::Bilinear)
    };
    assert_eq!(run1(&d, 2, 1, 1, 1, |_, y| (-9.5, y), &c), vec![0.0]);
}

#[test]
fn sum_normalisation_with_only_zero_samples_gives_zero() {
    // No tap lies inside the image under a zero border, so every sample is zero and the
    // normalised result is zero.
    let d = [4.0f32, 4.0];
    let c = Config {
        border: Border::Zero,
        normalize: Normalize::Sum,
        ..cfg(Filter::Bilinear)
    };
    assert_eq!(run1(&d, 2, 1, 1, 1, |_, y| (-9.5, y), &c), vec![0.0]);
}

#[test]
fn accumulation_order_is_exact_on_dyadic_data() {
    let d: Vec<f32> = (0..20i32).map(|i| i.rem_euclid(5) as f32 * 0.5).collect();
    let map = |x: f64, y: f64| (x * 0.5 + 0.25, y * 0.5 + 0.25);
    let mk = |order| Config {
        order,
        ..cfg(Filter::Keys { a: -0.5 })
    };
    let a = resample(&img(&d, 5, 4, 1), 6, 5, map, &mk(AccumOrder::RowMajor)).unwrap();
    let b = resample(&img(&d, 5, 4, 1), 6, 5, map, &mk(AccumOrder::ColumnMajor)).unwrap();
    assert_eq!(bits(&a), bits(&b));
}

#[test]
fn accumulation_order_changes_rounding_only() {
    let d: Vec<f32> = (0..30i32)
        .map(|i| ((i * 37).rem_euclid(29) as f32) * 0.173 - 1.1)
        .collect();
    let map = |x: f64, y: f64| (x * 0.61 + 0.13, y * 0.77 - 0.29);
    let mk = |order| Config {
        order,
        ..cfg(Filter::Lanczos { lobes: 3 })
    };
    let a = resample(&img(&d, 6, 5, 1), 7, 6, map, &mk(AccumOrder::RowMajor)).unwrap();
    let b = resample(&img(&d, 6, 5, 1), 7, 6, map, &mk(AccumOrder::ColumnMajor)).unwrap();
    for (x, y) in a.iter().zip(&b) {
        assert!((x - y).abs() < 1e-5);
    }
}

#[test]
fn results_are_deterministic_across_runs() {
    let d: Vec<f32> = (0..30i32)
        .map(|i| ((i * 37).rem_euclid(29) as f32) * 0.173 - 1.1)
        .collect();
    let map = |x: f64, y: f64| (x * 0.61 + 0.13, y * 0.77 - 0.29);
    for f in common::filters() {
        let a = resample(&img(&d, 6, 5, 1), 7, 6, map, &cfg(f)).unwrap();
        let b = resample(&img(&d, 6, 5, 1), 7, 6, map, &cfg(f)).unwrap();
        assert_eq!(bits(&a), bits(&b), "{f:?}");
    }
}

#[test]
fn lanczos_three_output_checksum_is_pinned() {
    // FNV-1a over the output bit patterns: pins the arithmetic of the whole path, sinc included.
    let d: Vec<f32> = (0..30i32)
        .map(|i| ((i * 37).rem_euclid(29) as f32) * 0.173 - 1.1)
        .collect();
    let map = |x: f64, y: f64| (x * 0.61 + 0.13, y * 0.77 - 0.29);
    let out = resample(
        &img(&d, 6, 5, 1),
        7,
        6,
        map,
        &cfg(Filter::Lanczos { lobes: 3 }),
    )
    .unwrap();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bits(&out) {
        for k in 0..4 {
            h ^= u64::from((b >> (8 * k)) & 0xff);
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    assert_eq!(h, PINNED_LANCZOS3);
}

const PINNED_LANCZOS3: u64 = 490741934646975935;

#[test]
fn error_buffer_length_mismatch() {
    let d = [1.0f32; 5];
    let r = resample(&img(&d, 2, 2, 1), 1, 1, |x, y| (x, y), &Config::default());
    assert_eq!(r, Err(Error::BadLength));
}

#[test]
fn error_zero_dimensions() {
    let d: [f32; 0] = [];
    assert_eq!(
        resample(&img(&d, 0, 2, 1), 1, 1, |x, y| (x, y), &Config::default()),
        Err(Error::BadDimensions)
    );
    assert_eq!(
        resample(&img(&d, 2, 0, 1), 1, 1, |x, y| (x, y), &Config::default()),
        Err(Error::BadDimensions)
    );
    assert_eq!(
        resample(&img(&d, 2, 2, 0), 1, 1, |x, y| (x, y), &Config::default()),
        Err(Error::BadDimensions)
    );
}

#[test]
fn error_overflowing_dimensions() {
    let d = [1.0f32];
    let r = resample(
        &img(&d, usize::MAX, 2, 1),
        1,
        1,
        |x, y| (x, y),
        &Config::default(),
    );
    assert_eq!(r, Err(Error::BadDimensions));
    let r = resample(
        &img(&d, 1, 1, 1),
        usize::MAX,
        2,
        |x, y| (x, y),
        &Config::default(),
    );
    assert_eq!(r, Err(Error::BadDimensions));
}

#[test]
fn error_lanczos_zero_lobes() {
    let d = [1.0f32];
    let r = resample(
        &img(&d, 1, 1, 1),
        1,
        1,
        |x, y| (x, y),
        &cfg(Filter::Lanczos { lobes: 0 }),
    );
    assert_eq!(r, Err(Error::BadLobes));
}

#[test]
fn error_alpha_configuration() {
    let d = [1.0f32, 1.0];
    let c = Config {
        alpha_channel: Some(2),
        ..Config::default()
    };
    assert_eq!(
        resample(&img(&d, 1, 1, 2), 1, 1, |x, y| (x, y), &c),
        Err(Error::BadAlpha)
    );
    let c = Config {
        alpha: AlphaMode::Premultiplied,
        ..Config::default()
    };
    assert_eq!(
        resample(&img(&d, 1, 1, 2), 1, 1, |x, y| (x, y), &c),
        Err(Error::BadAlpha)
    );
    let c = Config {
        alpha_channel: Some(1),
        ..Config::default()
    };
    assert!(resample(&img(&d, 1, 1, 2), 1, 1, |x, y| (x, y), &c).is_ok());
}

#[test]
fn error_footprint_values() {
    let d = [1.0f32];
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let c = Config {
            min_extent: bad,
            ..cfg(Filter::Box)
        };
        assert_eq!(c.validate(1), Err(Error::BadFootprint));
        let c = Config {
            footprint: Footprint::Fixed { w: bad, h: 1.0 },
            ..cfg(Filter::Box)
        };
        assert_eq!(
            resample(&img(&d, 1, 1, 1), 1, 1, |x, y| (x, y), &c),
            Err(Error::BadFootprint)
        );
        let c = Config {
            footprint: Footprint::Fixed { w: 1.0, h: bad },
            ..cfg(Filter::Box)
        };
        assert_eq!(c.validate(1), Err(Error::BadFootprint));
    }
}

#[test]
fn error_display_texts_are_distinct() {
    let all = [
        Error::BadDimensions,
        Error::BadLength,
        Error::BadLobes,
        Error::BadAlpha,
        Error::BadFootprint,
    ];
    let texts: HashSet<String> = all.iter().map(|e| e.to_string()).collect();
    assert_eq!(texts.len(), 5);
    assert!(texts.iter().all(|t| !t.is_empty()));
}

#[test]
fn default_config_is_valid_and_documented_values() {
    let c = Config::default();
    assert!(c.validate(1).is_ok());
    assert_eq!(c.filter, Filter::Bilinear);
    assert_eq!(c.border, Border::Clamp);
    assert_eq!(c.alpha, AlphaMode::Straight);
    assert_eq!(c.alpha_channel, None);
    assert_eq!(c.normalize, Normalize::None);
    assert_eq!(c.order, AccumOrder::RowMajor);
    assert_eq!(c.footprint, Footprint::Unit);
    assert_eq!(c.min_extent, 1.0);
}

#[test]
fn centre_offsets() {
    assert_eq!(resample::PixelCenter::Half.offset(), 0.5);
    assert_eq!(resample::PixelCenter::Corner.offset(), 0.0);
}

#[test]
fn config_space_size_and_uniqueness() {
    let s = ConfigSpace::standard();
    assert_eq!(s.len(), 11 * 3 * 2 * 5 * 3 * 2 * 2);
    assert!(!s.is_empty());
    let all = s.all();
    assert_eq!(all.len(), s.len());
    let uniq: HashSet<String> = all.iter().map(|c| format!("{c:?}")).collect();
    assert_eq!(uniq.len(), all.len());
}

#[test]
fn config_space_order_is_fixed_with_last_axis_fastest() {
    let all = ConfigSpace::standard().all();
    assert_eq!(all[0].footprint, Footprint::Unit);
    assert_eq!(all[1].footprint, Footprint::Jacobian);
    assert_eq!(all[0].filter, Filter::Nearest);
    assert_eq!(all.last().unwrap().filter, Filter::Box);
    assert_eq!(all[2].order, AccumOrder::ColumnMajor);
}

#[test]
fn config_space_empty_axis() {
    let mut s = ConfigSpace::standard();
    s.borders.clear();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
    assert!(s.all().is_empty());
}

#[test]
fn config_space_keeps_base_fields() {
    let mut s = ConfigSpace::standard();
    s.base.min_extent = 2.0;
    s.base.alpha_channel = Some(0);
    assert!(s
        .all()
        .iter()
        .all(|c| c.min_extent == 2.0 && c.alpha_channel == Some(0)));
}

#[test]
fn every_standard_config_runs_deterministically_and_stays_finite() {
    let d: Vec<f32> = (0..12i32)
        .map(|i| (i * 5).rem_euclid(11) as f32 * 0.5 - 1.0)
        .collect();
    let map = |x: f64, y: f64| (x * 0.75 - 0.3, y * 1.25 + 0.1);
    for c in ConfigSpace::standard().all() {
        let a = resample(&img(&d, 4, 3, 1), 5, 4, map, &c).unwrap();
        let b = resample(&img(&d, 4, 3, 1), 5, 4, map, &c).unwrap();
        assert_eq!(bits(&a), bits(&b), "{c:?}");
        assert!(a.iter().all(|v| v.is_finite()), "{c:?}");
        assert_eq!(a.len(), 20);
    }
}

#[test]
fn sum_normalised_clamped_constant_image_stays_constant() {
    let d = vec![0.5f32; 12];
    for mut c in ConfigSpace::standard().all() {
        if c.normalize != Normalize::Sum || c.border == Border::Zero {
            continue;
        }
        c.footprint = Footprint::Unit;
        let out = resample(
            &img(&d, 4, 3, 1),
            5,
            4,
            |x, y| (x * 0.75 - 0.3, y * 1.25 + 0.1),
            &c,
        )
        .unwrap();
        assert!(out.iter().all(|v| (v - 0.5).abs() < 1e-6), "{c:?}");
    }
}
