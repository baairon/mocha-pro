//! Config: canonical text, parsing, enumeration and the conversions it drives.

use pixel_format::{
    AlphaOp, Clamp, Config, Depth, Precision, PremulOrder, Rounding, Space, ZeroAlpha,
};
use std::collections::HashSet;

fn bits(x: f32) -> u32 {
    x.to_bits()
}

#[test]
fn default_choice_text_is_pinned() {
    assert_eq!(
        Config::default_choice().canonical(),
        "rounding=half_up;precision=f64;clamp=range_nan_zero;zero_alpha=zero_colour;order=convert_first;u16_max=32768"
    );
}

#[test]
fn full_space_has_240_choices_with_unique_text() {
    let all: Vec<Config> = Config::enumerate(&Space::full()).collect();
    assert_eq!(all.len(), 5 * 2 * 3 * 2 * 2 * 2);
    let texts: HashSet<String> = all.iter().map(Config::canonical).collect();
    assert_eq!(texts.len(), all.len());
}

#[test]
fn every_enumerated_choice_parses_back() {
    for c in Config::enumerate(&Space::full()) {
        assert_eq!(Config::parse(&c.canonical()).unwrap(), c);
    }
}

#[test]
fn enumeration_order_is_fixed() {
    let all: Vec<Config> = Config::enumerate(&Space::full()).collect();
    assert_eq!(
        all[0].canonical(),
        "rounding=half_up;precision=f32;clamp=none;zero_alpha=zero_colour;order=convert_first;u16_max=32768"
    );
    assert_eq!(all[1].u16_max, 65535);
    assert_eq!(all[2].order, PremulOrder::AlphaFirst);
    assert_eq!(
        all[239].canonical(),
        "rounding=ceil;precision=f64;clamp=range_nan_zero;zero_alpha=keep_colour;order=alpha_first;u16_max=65535"
    );
}

#[test]
fn enumeration_skips_repeated_candidates() {
    let mut s = Space::full();
    s.roundings = vec![Rounding::Floor, Rounding::Floor, Rounding::Ceil];
    s.u16_maxes = vec![100, 100];
    let all: Vec<Config> = Config::enumerate(&s).collect();
    assert_eq!(all.len(), 2 * 2 * 3 * 2 * 2);
}

#[test]
fn enumeration_of_an_empty_candidate_list_is_empty() {
    let mut s = Space::full();
    s.clamps.clear();
    assert_eq!(Config::enumerate(&s).count(), 0);
}

#[test]
fn enumeration_of_one_point_space() {
    let d = Config::default_choice();
    let s = Space {
        roundings: vec![d.rounding],
        precisions: vec![d.precision],
        clamps: vec![d.clamp],
        zero_alphas: vec![d.zero_alpha],
        orders: vec![d.order],
        u16_maxes: vec![d.u16_max],
    };
    assert_eq!(Config::enumerate(&s).collect::<Vec<_>>(), vec![d]);
}

#[test]
fn parse_refuses_malformed_text() {
    let good = Config::default_choice().canonical();
    assert!(Config::parse(&good).is_ok());
    let bad = [
        String::new(),
        good.replace(";u16_max=32768", ""),
        good.replace("half_up", "round"),
        good.replace("precision", "prec"),
        good.replace("u16_max=32768", "u16_max=-1"),
        good.replace("u16_max=32768", "u16_max=65536"),
        good.replace("u16_max=32768", "u16_max="),
        good.replace("u16_max=32768", "u16_max=+5"),
        good.replace(
            "rounding=half_up;precision=f64",
            "precision=f64;rounding=half_up",
        ),
        good.replace("clamp=range_nan_zero", "clamp"),
        format!("{good};extra=1"),
        good.to_uppercase(),
    ];
    for b in &bad {
        assert!(Config::parse(b).is_err(), "accepted {b:?}");
    }
}

#[test]
fn parse_error_text_names_the_problem() {
    let e = Config::parse("rounding=nope;precision=f64;clamp=none;zero_alpha=zero_colour;order=convert_first;u16_max=1")
        .unwrap_err();
    assert!(e.to_string().contains("rounding"));
    assert!(e.to_string().contains("nope"));
}

#[test]
fn convert_sample_eight_to_float() {
    let c = Config::default_choice();
    assert_eq!(c.convert_sample(Depth::U8, Depth::F32, 0), bits(0.0));
    assert_eq!(c.convert_sample(Depth::U8, Depth::F32, 255), bits(1.0));
    assert_eq!(c.convert_sample(Depth::U8, Depth::F32, 51), 0x3E4C_CCCD);
}

#[test]
fn convert_sample_float_to_each_integer_depth() {
    let c = Config::default_choice();
    let h = Depth::U16 { max: 32768 };
    let f = Depth::U16 { max: 65535 };
    assert_eq!(c.convert_sample(Depth::F32, Depth::U8, bits(0.5)), 128);
    assert_eq!(c.convert_sample(Depth::F32, h, bits(0.5)), 16384);
    assert_eq!(c.convert_sample(Depth::F32, f, bits(0.5)), 32768);
    assert_eq!(c.convert_sample(Depth::F32, Depth::U8, bits(1.5)), 255);
    assert_eq!(c.convert_sample(Depth::F32, Depth::U8, bits(-1.5)), 0);
    assert_eq!(c.convert_sample(Depth::F32, Depth::U8, bits(f32::NAN)), 0);
}

#[test]
fn convert_sample_between_integer_depths() {
    let c = Config::default_choice();
    let h = Depth::U16 { max: 32768 };
    assert_eq!(c.convert_sample(Depth::U8, h, 255), 32768);
    assert_eq!(c.convert_sample(Depth::U8, h, 0), 0);
    assert_eq!(c.convert_sample(h, Depth::U8, 32768), 255);
    assert_eq!(c.convert_sample(h, Depth::U8, 16384), 128);
    // 8 bit to 8 bit is the identity
    for v in 0..=255 {
        assert_eq!(c.convert_sample(Depth::U8, Depth::U8, v), v);
    }
}

#[test]
fn convert_sample_float_to_float_is_the_identity_on_bits() {
    let c = Config::default_choice();
    for b in [0u32, 0x8000_0000, 0x7FC0_0123, 0x7F80_0000, 1, 0x3F80_0000] {
        assert_eq!(c.convert_sample(Depth::F32, Depth::F32, b), b);
    }
}

#[test]
fn convert_sample_follows_the_rounding_choice() {
    let h = Depth::U16 { max: 32768 };
    let mut c = Config::default_choice();
    // 8 bit 1 is 1/255 of full scale: 32768 / 255 = 128.50196
    assert_eq!(c.convert_sample(Depth::U8, h, 1), 129);
    c.rounding = Rounding::Truncate;
    assert_eq!(c.convert_sample(Depth::U8, h, 1), 128);
    c.rounding = Rounding::Ceil;
    assert_eq!(c.convert_sample(Depth::U8, h, 1), 129);
}

#[test]
fn convert_sample_follows_the_clamp_choice() {
    let mut c = Config::default_choice();
    let x = bits(3.0);
    assert_eq!(c.convert_sample(Depth::F32, Depth::U8, x), 255);
    c.clamp = Clamp::None;
    assert_eq!(c.convert_sample(Depth::F32, Depth::U8, x), 765);
    c.clamp = Clamp::Range;
    assert_eq!(c.convert_sample(Depth::F32, Depth::U8, bits(f32::NAN)), 255);
}

#[test]
fn convert_pixel_without_alpha_operation() {
    let c = Config::default_choice();
    let px = [bits(0.0), bits(0.5), bits(1.0), bits(0.25)];
    assert_eq!(
        c.convert_pixel(Depth::F32, Depth::U8, px, AlphaOp::None),
        [0, 128, 255, 64]
    );
}

#[test]
fn convert_pixel_orders_differ_with_a_directed_rounding() {
    let px = [bits(0.5), bits(0.5), bits(0.5), bits(0.25)];
    let mut c = Config::default_choice();
    c.rounding = Rounding::Ceil;
    // convert first: colour 128, alpha 64, 128 * 64 / 255 = 32.13 -> 33
    c.order = PremulOrder::ConvertFirst;
    assert_eq!(
        c.convert_pixel(Depth::F32, Depth::U8, px, AlphaOp::Premultiply),
        [33, 33, 33, 64]
    );
    // alpha first: 0.5 * 0.25 = 0.125 -> 31.875 -> 32
    c.order = PremulOrder::AlphaFirst;
    assert_eq!(
        c.convert_pixel(Depth::F32, Depth::U8, px, AlphaOp::Premultiply),
        [32, 32, 32, 64]
    );
}

#[test]
fn convert_pixel_unpremultiply_float_to_eight() {
    let px = [bits(0.25), bits(0.125), bits(0.5), bits(0.5)];
    let mut c = Config::default_choice();
    c.order = PremulOrder::AlphaFirst;
    // 0.5, 0.25, 1.0 then to 8 bit: 127.5 -> 128, 63.75 -> 64, 255
    assert_eq!(
        c.convert_pixel(Depth::F32, Depth::U8, px, AlphaOp::Unpremultiply),
        [128, 64, 255, 128]
    );
}

#[test]
fn convert_pixel_zero_alpha_rule_applies() {
    let px = [7, 8, 9, 0];
    let mut c = Config::default_choice();
    assert_eq!(
        c.convert_pixel(Depth::U8, Depth::U8, px, AlphaOp::Unpremultiply),
        [0, 0, 0, 0]
    );
    c.zero_alpha = ZeroAlpha::KeepColour;
    assert_eq!(
        c.convert_pixel(Depth::U8, Depth::U8, px, AlphaOp::Unpremultiply),
        [7, 8, 9, 0]
    );
    c.order = PremulOrder::AlphaFirst;
    assert_eq!(
        c.convert_pixel(Depth::U8, Depth::U8, px, AlphaOp::Unpremultiply),
        [7, 8, 9, 0]
    );
}

#[test]
fn convert_pixel_alpha_sample_is_never_premultiplied() {
    let c = Config::default_choice();
    let px = [200, 100, 50, 128];
    let out = c.convert_pixel(Depth::U8, Depth::U8, px, AlphaOp::Premultiply);
    assert_eq!(out, [100, 50, 25, 128]);
}

#[test]
fn convert_pixel_float_premultiply_keeps_float_bits() {
    let c = Config::default_choice();
    let px = [bits(1.0), bits(0.5), bits(2.0), bits(0.5)];
    let out = c.convert_pixel(Depth::F32, Depth::F32, px, AlphaOp::Premultiply);
    assert_eq!(out, [bits(0.5), bits(0.25), bits(1.0), bits(0.5)]);
    let back = c.convert_pixel(Depth::F32, Depth::F32, out, AlphaOp::Unpremultiply);
    assert_eq!(back, px);
}

#[test]
fn rules_carry_the_configuration() {
    let mut c = Config::default_choice();
    c.precision = Precision::F32;
    c.rounding = Rounding::Floor;
    let r = c.float_to_int_rule(255);
    assert_eq!(r.scale, 255.0);
    assert_eq!(r.offset, 0.0);
    assert_eq!(r.rounding, Rounding::Floor);
    assert_eq!(r.precision, Precision::F32);
    assert_eq!(r.clamp, c.clamp);
    let u = c.int_to_float_rule(65535);
    assert_eq!(u.denom, 65535.0);
    assert_eq!(u.precision, Precision::F32);
    assert_eq!(c.u16_depth(), Depth::U16 { max: 32768 });
}

#[test]
fn order_names_parse_back() {
    for o in PremulOrder::ALL {
        assert_eq!(PremulOrder::parse(o.name()), Some(o));
    }
    assert_eq!(PremulOrder::parse("first"), None);
}

#[test]
fn parse_refuses_a_non_canonical_number() {
    let text = |n: &str| {
        format!("rounding=half_up;precision=f64;clamp=none;zero_alpha=zero_colour;order=convert_first;u16_max={n}")
    };
    assert!(Config::parse(&text("32768")).is_ok());
    assert!(Config::parse(&text("0")).is_ok());
    for bad in ["032768", "00", "007", "+7"] {
        assert!(Config::parse(&text(bad)).is_err(), "accepted {bad:?}");
    }
}
