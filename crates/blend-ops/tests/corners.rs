//! Every mode on the corner values 0 and 1 for source and backdrop (opaque grey pixels), and one
//! hand worked mid pair per mode. Expectations are worked by hand from the cited formulas.
use blend_ops::{composite_pixel, BlendMode as M, Config, PorterDuff, Rgba};

fn run(mode: M, cs: f64, cb: f64) -> f64 {
    let s = Rgba::new(cs, cs, cs, 1.0);
    let d = Rgba::new(cb, cb, cb, 1.0);
    let o = composite_pixel(s, d, mode, PorterDuff::SrcOver, 1.0, &Config::default());
    assert_eq!(o.r.to_bits(), o.g.to_bits());
    assert_eq!(o.r.to_bits(), o.b.to_bits());
    assert_eq!(o.a, 1.0);
    o.r
}

macro_rules! corner {
    ($name:ident, $mode:expr, $e00:expr, $e01:expr, $e10:expr, $e11:expr) => {
        mod $name {
            use super::*;
            // arguments of run: (cs, cb); the expected values are listed as (cs,cb) = 00, 01, 10, 11
            #[test]
            fn src0_dst0() {
                assert_eq!(run($mode, 0.0, 0.0).to_bits(), ($e00 as f64).to_bits());
            }
            #[test]
            fn src0_dst1() {
                assert_eq!(run($mode, 0.0, 1.0).to_bits(), ($e01 as f64).to_bits());
            }
            #[test]
            fn src1_dst0() {
                assert_eq!(run($mode, 1.0, 0.0).to_bits(), ($e10 as f64).to_bits());
            }
            #[test]
            fn src1_dst1() {
                assert_eq!(run($mode, 1.0, 1.0).to_bits(), ($e11 as f64).to_bits());
            }
        }
    };
}

corner!(normal, M::Normal, 0.0, 0.0, 1.0, 1.0);
corner!(multiply, M::Multiply, 0.0, 0.0, 0.0, 1.0);
corner!(screen, M::Screen, 0.0, 1.0, 1.0, 1.0);
corner!(overlay, M::Overlay, 0.0, 1.0, 0.0, 1.0);
corner!(darken, M::Darken, 0.0, 0.0, 0.0, 1.0);
corner!(lighten, M::Lighten, 0.0, 1.0, 1.0, 1.0);
corner!(dodge, M::ColorDodge, 0.0, 1.0, 0.0, 1.0);
corner!(burn, M::ColorBurn, 0.0, 1.0, 0.0, 1.0);
corner!(hard_light, M::HardLight, 0.0, 0.0, 1.0, 1.0);
corner!(soft_light, M::SoftLight, 0.0, 1.0, 0.0, 1.0);
corner!(difference, M::Difference, 0.0, 1.0, 1.0, 0.0);
corner!(exclusion, M::Exclusion, 0.0, 1.0, 1.0, 0.0);
corner!(add, M::Add, 0.0, 1.0, 1.0, 2.0);
corner!(subtract, M::Subtract, 0.0, 1.0, -1.0, 0.0);

// Non separable modes on grey corners: Hue, Saturation and Color return the backdrop luminosity,
// Luminosity the source luminosity (Lum of a grey is the grey value up to rounding).
macro_rules! nonsep {
    ($name:ident, $mode:expr, $pick_src:expr) => {
        #[test]
        fn $name() {
            for cs in [0.0, 1.0] {
                for cb in [0.0, 1.0] {
                    let e = if $pick_src { cs } else { cb };
                    let g = run($mode, cs, cb);
                    assert!((g - e).abs() <= 4e-16, "{} {} {}", cs, cb, g);
                }
            }
        }
    };
}
nonsep!(hue_corners, M::Hue, false);
nonsep!(saturation_corners, M::Saturation, false);
nonsep!(color_corners, M::Color, false);
nonsep!(luminosity_corners, M::Luminosity, true);

macro_rules! mid {
    ($name:ident, $mode:expr, $cs:expr, $cb:expr, $e:expr) => {
        #[test]
        fn $name() {
            assert_eq!(run($mode, $cs, $cb).to_bits(), ($e as f64).to_bits());
        }
    };
}
// cs = 0.75, cb = 0.5 unless noted (all dyadic, so every result is exact).
mid!(mid_normal, M::Normal, 0.75, 0.5, 0.75);
mid!(mid_multiply, M::Multiply, 0.75, 0.5, 0.375);
mid!(mid_screen, M::Screen, 0.75, 0.5, 0.875);
// Overlay = HardLight(cs'=cb=0.5 <= 0.5, cb'=0.75): 0.75 * (2 * 0.5) = 0.75
mid!(mid_overlay, M::Overlay, 0.75, 0.5, 0.75);
mid!(mid_darken, M::Darken, 0.75, 0.5, 0.5);
mid!(mid_lighten, M::Lighten, 0.75, 0.5, 0.75);
// dodge cs=0.5, cb=0.25: min(1, 0.25 / 0.5) = 0.5
mid!(mid_dodge, M::ColorDodge, 0.5, 0.25, 0.5);
// burn cs=0.5, cb=0.75: 1 - min(1, 0.25 / 0.5) = 0.5
mid!(mid_burn, M::ColorBurn, 0.5, 0.75, 0.5);
// hard light cs=0.75 > 0.5: screen(0.5, 0.5) = 0.5 + 0.5 - 0.25 = 0.75
mid!(mid_hard_light, M::HardLight, 0.75, 0.5, 0.75);
// soft light cs=0.75, cb=0.25: D(0.25) = ((4 - 12) * 0.25 + 4) * 0.25 = 0.5; 0.25 + 0.5 * 0.25
mid!(mid_soft_light, M::SoftLight, 0.75, 0.25, 0.375);
// soft light cs=0.75, cb=0.5625: sqrt = 0.75; 0.5625 + 0.5 * 0.1875 = 0.65625
mid!(mid_soft_light_sqrt, M::SoftLight, 0.75, 0.5625, 0.65625);
// soft light cs=0.75, cb=0.0625 (cubic branch): D = 0.20703125; 0.0625 + 0.5 * 0.14453125
mid!(
    mid_soft_light_cubic,
    M::SoftLight,
    0.75,
    0.0625,
    0.134765625
);
// soft light cs=0.25 <= 0.5, cb=0.5: 0.5 - 0.5 * 0.5 * 0.5 = 0.375
mid!(mid_soft_light_dark, M::SoftLight, 0.25, 0.5, 0.375);
mid!(mid_difference, M::Difference, 0.75, 0.5, 0.25);
mid!(mid_exclusion, M::Exclusion, 0.75, 0.5, 0.5);
mid!(mid_add, M::Add, 0.75, 0.5, 1.25);
mid!(mid_subtract, M::Subtract, 0.75, 0.5, -0.25);

#[test]
fn raw_bit_patterns() {
    assert_eq!(run(M::Multiply, 0.75, 0.5).to_bits(), 0x3FD8_0000_0000_0000);
    assert_eq!(run(M::Screen, 0.75, 0.5).to_bits(), 0x3FEC_0000_0000_0000);
    assert_eq!(run(M::Darken, 0.75, 0.5).to_bits(), 0x3FE0_0000_0000_0000);
    assert_eq!(
        run(M::Difference, 0.75, 0.5).to_bits(),
        0x3FD0_0000_0000_0000
    );
    assert_eq!(run(M::Add, 0.75, 0.5).to_bits(), 0x3FF4_0000_0000_0000);
}

#[test]
fn legacy_soft_light_differs_in_cubic_branch() {
    let cfg = Config {
        soft_light: blend_ops::SoftLight::Legacy,
        ..Config::default()
    };
    // cb = 0.0625, cs = 0.75: sqrt(cb) = 0.25, 0.0625 + 0.5 * 0.1875 = 0.15625
    let s: Rgba<f64> = Rgba::new(0.75, 0.75, 0.75, 1.0);
    let d = Rgba::new(0.0625, 0.0625, 0.0625, 1.0);
    let o = composite_pixel(s, d, M::SoftLight, PorterDuff::SrcOver, 1.0, &cfg);
    assert_eq!(o.r.to_bits(), 0.15625f64.to_bits());
}

#[test]
fn legacy_soft_light_equals_w3c_above_quarter() {
    let cfg = Config {
        soft_light: blend_ops::SoftLight::Legacy,
        ..Config::default()
    };
    let s: Rgba<f64> = Rgba::new(0.75, 0.75, 0.75, 1.0);
    let d = Rgba::new(0.5625, 0.5625, 0.5625, 1.0);
    let o = composite_pixel(s, d, M::SoftLight, PorterDuff::SrcOver, 1.0, &cfg);
    assert_eq!(o.r.to_bits(), 0.65625f64.to_bits());
}
