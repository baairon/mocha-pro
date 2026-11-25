//! `blend-ops`: per pixel compositing operators on RGBA values, in `f32` or `f64`.
//!
//! Sources of the published definitions:
//! - Porter and Duff, "Compositing Digital Images", Computer Graphics 18(3), SIGGRAPH 1984,
//!   section 3 (the operators and their fractions `Fa` and `Fb`, table 1).
//! - W3C, "Compositing and Blending Level 1", Candidate Recommendation 2015 (the Recommendation
//!   text of 2024 keeps the formulas): section 5.1 (general formula), section 9.1 (separable blend
//!   modes), section 10 (non separable modes, `Lum`, `ClipColor`, `SetLum`, `Sat`, `SetSat`),
//!   sections 8 and 9 (backdrop weighted source colour and Porter and Duff compositing).
//! - ISO 32000-1:2008, Document management, Portable document format, section 11.3.5 (blend mode
//!   functions), for the older soft light variant that uses the plain square root.
//!
//! Arithmetic uses only add, subtract, multiply, divide and square root, in the order written in
//! each function, with no fused multiply add and no library transcendental, so results are bit
//! identical on every platform that implements IEEE 754 binary32 and binary64. NaN inputs are not
//! supported and give unspecified (but deterministic and safe) output.

mod buffer;
mod config;
mod modes;
mod real;

pub use buffer::{composite_buffers, BufferError};
pub use config::{BackdropMix, Clamp, Config, Form, Guard, GuardOrder, OpacityPoint, SoftLight};
pub use modes::{
    blend_channel, blend_rgb, clip_color, lum, sat, set_lum, set_sat, BlendMode, PorterDuff,
};
pub use real::Real;

/// One RGBA value. Whether the colour channels are premultiplied is stated by [`Form`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

impl<T: Real> Rgba<T> {
    pub fn new(r: T, g: T, b: T, a: T) -> Self {
        Rgba { r, g, b, a }
    }
    pub fn rgb(&self) -> [T; 3] {
        [self.r, self.g, self.b]
    }
}

fn to_straight<T: Real>(p: Rgba<T>, form: Form) -> Rgba<T> {
    match form {
        Form::Straight => p,
        Form::Premultiplied => {
            if p.a == T::ZERO {
                Rgba::new(T::ZERO, T::ZERO, T::ZERO, p.a)
            } else {
                Rgba::new(p.r / p.a, p.g / p.a, p.b / p.a, p.a)
            }
        }
    }
}

fn clamp01<T: Real>(x: T) -> T {
    if x < T::ZERO {
        T::ZERO
    } else if x > T::ONE {
        T::ONE
    } else {
        x
    }
}

/// Composites `src` over `dst` with a blend mode, a Porter and Duff operator, an opacity and a
/// configuration. Order of operations (fixed here because the published formula leaves it open):
/// 1. both inputs are converted to straight form (alpha 0 gives colour 0) and, with
///    `Clamp::InputAndOutput`, clamped to 0..1;
/// 2. with `OpacityPoint::OnSourceAlpha`, `as = as * opacity`;
/// 3. the blend function `B` is applied to the straight colours, and the source colour becomes
///    `Cs' = (1 - ab) * Cs + ab * B(Cb, Cs)` (or `B` alone with `BackdropMix::Off`);
/// 4. `x = as * Fa`, `y = ab * Fb`, premultiplied result `co = x * Cs' + y * Cb`, `ao = x + y`;
/// 5. with `OpacityPoint::OnResult`, `co = co * opacity + (ab * Cb) * (1 - opacity)` and the same
///    for alpha (premultiplied domain);
/// 6. conversion to the output form (division by `ao`, 0 when `ao` is 0), then clamping to 0..1
///    unless `Clamp::None`.
pub fn composite_pixel<T: Real>(
    src: Rgba<T>,
    dst: Rgba<T>,
    blend: BlendMode,
    pd: PorterDuff,
    opacity: T,
    cfg: &Config,
) -> Rgba<T> {
    let mut s = to_straight(src, cfg.input_form);
    let mut b = to_straight(dst, cfg.input_form);
    if cfg.clamp == Clamp::InputAndOutput {
        for p in [&mut s, &mut b] {
            *p = Rgba::new(clamp01(p.r), clamp01(p.g), clamp01(p.b), clamp01(p.a));
        }
    }
    let mut a_s = s.a;
    if cfg.opacity_point == OpacityPoint::OnSourceAlpha {
        a_s = a_s * opacity;
    }
    let a_b = b.a;
    let bl = blend_rgb(blend, b.rgb(), s.rgb(), cfg);
    let cs = s.rgb();
    let cb = b.rgb();
    let (fa, fb) = pd.factors(a_s, a_b);
    let x = a_s * fa;
    let y = a_b * fb;
    let mut co = [T::ZERO; 3];
    for i in 0..3 {
        let cs2 = match cfg.backdrop_mix {
            BackdropMix::W3c => (T::ONE - a_b) * cs[i] + a_b * bl[i],
            BackdropMix::Off => bl[i],
        };
        co[i] = x * cs2 + y * cb[i];
    }
    let mut ao = x + y;
    if cfg.opacity_point == OpacityPoint::OnResult {
        let keep = T::ONE - opacity;
        for i in 0..3 {
            co[i] = co[i] * opacity + (a_b * cb[i]) * keep;
        }
        ao = ao * opacity + a_b * keep;
    }
    let mut out = [co[0], co[1], co[2]];
    if cfg.output_form == Form::Straight {
        for c in out.iter_mut() {
            *c = if ao == T::ZERO { T::ZERO } else { *c / ao };
        }
    }
    let mut res = Rgba::new(out[0], out[1], out[2], ao);
    if cfg.clamp != Clamp::None {
        res = Rgba::new(
            clamp01(res.r),
            clamp01(res.g),
            clamp01(res.b),
            clamp01(res.a),
        );
    }
    res
}
