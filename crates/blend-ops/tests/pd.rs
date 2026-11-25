//! The twelve Porter and Duff operators on one hand worked pair (Normal blend):
//! straight source grey 0.5 alpha 0.5, straight backdrop grey 1 alpha 0.25, premultiplied output.
use blend_ops::{composite_pixel, BlendMode, Config, Form, PorterDuff as P, Rgba};

fn run(pd: P) -> (f64, f64) {
    let cfg = Config {
        output_form: Form::Premultiplied,
        ..Config::default()
    };
    let s: Rgba<f64> = Rgba::new(0.5, 0.5, 0.5, 0.5);
    let d: Rgba<f64> = Rgba::new(1.0, 1.0, 1.0, 0.25);
    let o = composite_pixel(s, d, BlendMode::Normal, pd, 1.0, &cfg);
    assert_eq!(o.r.to_bits(), o.b.to_bits());
    (o.r, o.a)
}

macro_rules! pd {
    ($name:ident, $op:expr, $c:expr, $a:expr) => {
        #[test]
        fn $name() {
            let (c, a) = run($op);
            assert_eq!(c.to_bits(), ($c as f64).to_bits());
            assert_eq!(a.to_bits(), ($a as f64).to_bits());
        }
    };
}
pd!(clear, P::Clear, 0.0, 0.0);
pd!(src, P::Src, 0.25, 0.5);
pd!(dst, P::Dst, 0.25, 0.25);
pd!(src_over, P::SrcOver, 0.375, 0.625);
pd!(dst_over, P::DstOver, 0.4375, 0.625);
pd!(src_in, P::SrcIn, 0.0625, 0.125);
pd!(dst_in, P::DstIn, 0.125, 0.125);
pd!(src_out, P::SrcOut, 0.1875, 0.375);
pd!(dst_out, P::DstOut, 0.125, 0.125);
pd!(src_atop, P::SrcAtop, 0.1875, 0.25);
pd!(dst_atop, P::DstAtop, 0.3125, 0.5);
pd!(xor, P::Xor, 0.3125, 0.5);

#[test]
fn twelve_operators() {
    assert_eq!(P::ALL.len(), 12);
    for (i, a) in P::ALL.iter().enumerate() {
        for b in &P::ALL[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn factors_table() {
    let f = |p: P| p.factors(0.5f64, 0.25f64);
    assert_eq!(f(P::Clear), (0.0, 0.0));
    assert_eq!(f(P::Src), (1.0, 0.0));
    assert_eq!(f(P::Dst), (0.0, 1.0));
    assert_eq!(f(P::SrcOver), (1.0, 0.5));
    assert_eq!(f(P::DstOver), (0.75, 1.0));
    assert_eq!(f(P::SrcIn), (0.25, 0.0));
    assert_eq!(f(P::DstIn), (0.0, 0.5));
    assert_eq!(f(P::SrcOut), (0.75, 0.0));
    assert_eq!(f(P::DstOut), (0.0, 0.5));
    assert_eq!(f(P::SrcAtop), (0.25, 0.5));
    assert_eq!(f(P::DstAtop), (0.75, 0.5));
    assert_eq!(f(P::Xor), (0.75, 0.5));
}

#[test]
fn straight_output_divides_by_alpha() {
    let cfg = Config::default();
    let s: Rgba<f64> = Rgba::new(0.5, 0.5, 0.5, 0.5);
    let d: Rgba<f64> = Rgba::new(1.0, 1.0, 1.0, 0.25);
    let o = composite_pixel(s, d, BlendMode::Normal, P::SrcOver, 1.0, &cfg);
    assert_eq!(o.a, 0.625);
    assert_eq!(o.r, 0.6); // 0.375 / 0.625 correctly rounded
}

#[test]
fn zero_alpha_result_has_zero_colour() {
    let o = composite_pixel(
        Rgba::new(0.5, 0.5, 0.5, 0.5),
        Rgba::new(1.0, 1.0, 1.0, 1.0),
        BlendMode::Normal,
        P::Clear,
        1.0,
        &Config::default(),
    );
    assert_eq!(o, Rgba::new(0.0, 0.0, 0.0, 0.0));
}

#[test]
fn zero_alpha_inputs_are_safe() {
    let z = Rgba::new(0.0, 0.0, 0.0, 0.0);
    let cfg = Config {
        input_form: Form::Premultiplied,
        ..Config::default()
    };
    for p in P::ALL {
        let o = composite_pixel(z, z, BlendMode::Multiply, p, 1.0, &cfg);
        assert_eq!(o, z);
    }
}
