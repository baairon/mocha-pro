//! Tests added to close mutants: asymmetric alphas for all twelve operators, signed zeros,
//! zero alpha premultiplied inputs, negative backdrop in dodge, and exact clip identities.
use blend_ops::*;

#[test]
fn twelve_operators_asymmetric_alphas() {
    // straight source grey 0.5 alpha 0.125, straight backdrop grey 1 alpha 0.25, premultiplied out
    let cfg = Config {
        output_form: Form::Premultiplied,
        ..Config::default()
    };
    let s: Rgba<f64> = Rgba::new(0.5, 0.5, 0.5, 0.125);
    let d: Rgba<f64> = Rgba::new(1.0, 1.0, 1.0, 0.25);
    let want: [(PorterDuff, f64, f64); 12] = [
        (PorterDuff::Clear, 0.0, 0.0),
        (PorterDuff::Src, 0.0625, 0.125),
        (PorterDuff::Dst, 0.25, 0.25),
        (PorterDuff::SrcOver, 0.28125, 0.34375),
        (PorterDuff::DstOver, 0.296875, 0.34375),
        (PorterDuff::SrcIn, 0.015625, 0.03125),
        (PorterDuff::DstIn, 0.03125, 0.03125),
        (PorterDuff::SrcOut, 0.046875, 0.09375),
        (PorterDuff::DstOut, 0.21875, 0.21875),
        (PorterDuff::SrcAtop, 0.234375, 0.25),
        (PorterDuff::DstAtop, 0.078125, 0.125),
        (PorterDuff::Xor, 0.265625, 0.3125),
    ];
    for (pd, c, a) in want {
        let o = composite_pixel(s, d, BlendMode::Normal, pd, 1.0, &cfg);
        assert_eq!(o.r.to_bits(), c.to_bits(), "{:?} colour", pd);
        assert_eq!(o.a.to_bits(), a.to_bits(), "{:?} alpha", pd);
    }
}

#[test]
fn factors_asymmetric() {
    let f = |p: PorterDuff| p.factors(0.125f64, 0.25f64);
    assert_eq!(f(PorterDuff::SrcOver), (1.0, 0.875));
    assert_eq!(f(PorterDuff::DstOver), (0.75, 1.0));
    assert_eq!(f(PorterDuff::SrcIn), (0.25, 0.0));
    assert_eq!(f(PorterDuff::DstIn), (0.0, 0.125));
    assert_eq!(f(PorterDuff::SrcOut), (0.75, 0.0));
    assert_eq!(f(PorterDuff::DstOut), (0.0, 0.875));
    assert_eq!(f(PorterDuff::SrcAtop), (0.25, 0.875));
    assert_eq!(f(PorterDuff::DstAtop), (0.75, 0.125));
    assert_eq!(f(PorterDuff::Xor), (0.75, 0.875));
}

#[test]
fn premultiplied_zero_alpha_source_over_opaque_backdrop() {
    let cfg = Config {
        input_form: Form::Premultiplied,
        ..Config::default()
    };
    let z: Rgba<f64> = Rgba::new(0.0, 0.0, 0.0, 0.0);
    let d: Rgba<f64> = Rgba::new(0.5, 0.25, 0.75, 1.0);
    let o = composite_pixel(z, d, BlendMode::Multiply, PorterDuff::SrcOver, 1.0, &cfg);
    assert_eq!(o, d);
    let o = composite_pixel(d, z, BlendMode::Multiply, PorterDuff::SrcOver, 1.0, &cfg);
    assert_eq!(o, d);
}

#[test]
fn signed_zero_of_min_and_max_is_the_first_argument() {
    assert_eq!(Real::min(0.0f64, -0.0).to_bits(), 0.0f64.to_bits());
    assert_eq!(Real::min(-0.0f64, 0.0).to_bits(), (-0.0f64).to_bits());
    assert_eq!(Real::max(0.0f64, -0.0).to_bits(), 0.0f64.to_bits());
    assert_eq!(Real::max(-0.0f64, 0.0).to_bits(), (-0.0f64).to_bits());
}

#[test]
fn dodge_with_negative_backdrop_is_not_guarded() {
    let c = Config::default();
    assert_eq!(blend_channel(BlendMode::ColorDodge, -0.25, 0.5, &c), -0.5);
}

#[test]
fn burn_with_backdrop_above_one_is_not_guarded() {
    let c = Config::default();
    // cb = 1.5 is not the guard value 1: 1 - min(1, (1 - 1.5) / 0.5) = 1 - (-1) = 2
    assert_eq!(blend_channel(BlendMode::ColorBurn, 1.5, 0.5, &c), 2.0);
}

#[test]
fn clip_color_leaves_boundary_values_untouched() {
    // maximum exactly 1 and minimum exactly 0 are inside the range, so nothing is rescaled
    let a = clip_color([1.0f64, 0.5, 0.25]);
    assert_eq!(a.map(f64::to_bits), [1.0f64, 0.5, 0.25].map(f64::to_bits));
    let b = clip_color([0.0f64, 0.98, 0.09]);
    assert_eq!(b.map(f64::to_bits), [0.0f64, 0.98, 0.09].map(f64::to_bits));
}

#[test]
fn set_sat_scales_middle_by_mid_minus_min_over_range() {
    let o = set_sat([0.0f64, 0.25, 1.0], 0.5);
    assert_eq!(o, [0.0, 0.125, 0.5]);
    let o = set_sat([0.5f64, 0.0, 1.0], 1.0);
    assert_eq!(o, [0.5, 0.0, 1.0]);
}

#[test]
fn matte_buffer_premultiplied_colour_scaled_per_channel() {
    let cfg = Config {
        input_form: Form::Premultiplied,
        output_form: Form::Premultiplied,
        ..Config::default()
    };
    let mut o = [0.0f64; 4];
    composite_buffers(
        &[0.5, 0.25, 0.125, 0.5],
        &[0.0, 0.0, 0.0, 0.0],
        &mut o,
        Some(&[0.5]),
        1.0,
        BlendMode::Normal,
        PorterDuff::Src,
        &cfg,
    )
    .unwrap();
    assert_eq!(o, [0.25, 0.125, 0.0625, 0.25]);
}
