use crate::config::{Config, Guard, GuardOrder, SoftLight};
use crate::real::Real;

/// The twelve Porter and Duff operators (Porter and Duff 1984, section 3 and table 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PorterDuff {
    Clear,
    Src,
    Dst,
    SrcOver,
    DstOver,
    SrcIn,
    DstIn,
    SrcOut,
    DstOut,
    SrcAtop,
    DstAtop,
    Xor,
}

impl PorterDuff {
    pub const ALL: [PorterDuff; 12] = [
        PorterDuff::Clear,
        PorterDuff::Src,
        PorterDuff::Dst,
        PorterDuff::SrcOver,
        PorterDuff::DstOver,
        PorterDuff::SrcIn,
        PorterDuff::DstIn,
        PorterDuff::SrcOut,
        PorterDuff::DstOut,
        PorterDuff::SrcAtop,
        PorterDuff::DstAtop,
        PorterDuff::Xor,
    ];

    /// The fractions `(Fa, Fb)` of the source and the backdrop that are kept, given the source
    /// alpha `a_s` and the backdrop alpha `a_b` (Porter and Duff 1984, table 1; W3C Compositing
    /// and Blending Level 1, section 9.2 table of operators).
    pub fn factors<T: Real>(self, a_s: T, a_b: T) -> (T, T) {
        let (z, o) = (T::ZERO, T::ONE);
        match self {
            PorterDuff::Clear => (z, z),
            PorterDuff::Src => (o, z),
            PorterDuff::Dst => (z, o),
            PorterDuff::SrcOver => (o, o - a_s),
            PorterDuff::DstOver => (o - a_b, o),
            PorterDuff::SrcIn => (a_b, z),
            PorterDuff::DstIn => (z, a_s),
            PorterDuff::SrcOut => (o - a_b, z),
            PorterDuff::DstOut => (z, o - a_s),
            PorterDuff::SrcAtop => (a_b, o - a_s),
            PorterDuff::DstAtop => (o - a_b, a_s),
            PorterDuff::Xor => (o - a_b, o - a_s),
        }
    }
}

/// Blend modes. The twelve separable W3C modes of section 9.1 (normal 9.1.1 through exclusion),
/// `Add` and `Subtract` (plain `cb + cs` and `cb - cs`, a generic extension), and the four non
/// separable modes of section 10.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Add,
    Subtract,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendMode {
    pub const ALL: [BlendMode; 18] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Add,
        BlendMode::Subtract,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
    ];

    pub fn is_separable(self) -> bool {
        !matches!(
            self,
            BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity
        )
    }
}

fn guard_value<T: Real>(g: Guard) -> T {
    match g {
        Guard::Zero => T::ZERO,
        Guard::One => T::ONE,
    }
}

fn multiply<T: Real>(cb: T, cs: T) -> T {
    cb * cs
}

fn screen<T: Real>(cb: T, cs: T) -> T {
    cb + cs - cb * cs
}

fn hard_light<T: Real>(cb: T, cs: T) -> T {
    let half = T::from_f64(0.5);
    if cs <= half {
        multiply(cb, cs + cs)
    } else {
        screen(cb, cs + cs - T::ONE)
    }
}

/// W3C section 9.1.3 `D(x)`.
fn soft_d<T: Real>(x: T) -> T {
    if x <= T::from_f64(0.25) {
        ((T::from_f64(16.0) * x - T::from_f64(12.0)) * x + T::from_f64(4.0)) * x
    } else {
        x.sqrt()
    }
}

fn soft_light<T: Real>(cb: T, cs: T, v: SoftLight) -> T {
    let half = T::from_f64(0.5);
    let two_cs = cs + cs;
    if cs <= half {
        cb - (T::ONE - two_cs) * cb * (T::ONE - cb)
    } else {
        let d = match v {
            SoftLight::W3c => soft_d(cb),
            SoftLight::Legacy => cb.sqrt(),
        };
        cb + (two_cs - T::ONE) * (d - cb)
    }
}

/// W3C section 9.1.2 colour dodge, with the guard values and the test order of the configuration.
fn color_dodge<T: Real>(cb: T, cs: T, cfg: &Config) -> T {
    let by_backdrop = cb == T::ZERO;
    let by_source = cs == T::ONE;
    let bv = guard_value(cfg.dodge_backdrop_zero);
    let sv = guard_value(cfg.dodge_source_one);
    let hit = match cfg.guard_order {
        GuardOrder::BackdropFirst if by_backdrop => Some(bv),
        GuardOrder::BackdropFirst if by_source => Some(sv),
        GuardOrder::SourceFirst if by_source => Some(sv),
        GuardOrder::SourceFirst if by_backdrop => Some(bv),
        _ => None,
    };
    match hit {
        Some(v) => v,
        None => T::ONE.min(cb / (T::ONE - cs)),
    }
}

/// W3C section 9.1.2 colour burn, with the guard values and the test order of the configuration.
fn color_burn<T: Real>(cb: T, cs: T, cfg: &Config) -> T {
    let by_backdrop = cb == T::ONE;
    let by_source = cs == T::ZERO;
    let bv = guard_value(cfg.burn_backdrop_one);
    let sv = guard_value(cfg.burn_source_zero);
    let hit = match cfg.guard_order {
        GuardOrder::BackdropFirst if by_backdrop => Some(bv),
        GuardOrder::BackdropFirst if by_source => Some(sv),
        GuardOrder::SourceFirst if by_source => Some(sv),
        GuardOrder::SourceFirst if by_backdrop => Some(bv),
        _ => None,
    };
    match hit {
        Some(v) => v,
        None => T::ONE - T::ONE.min((T::ONE - cb) / cs),
    }
}

/// One channel of a separable mode (W3C section 9.1). `cb` is the straight backdrop channel and
/// `cs` the straight source channel. Non separable modes are not channel functions and return
/// the source channel; use [`blend_rgb`].
pub fn blend_channel<T: Real>(mode: BlendMode, cb: T, cs: T, cfg: &Config) -> T {
    match mode {
        BlendMode::Normal => cs,
        BlendMode::Multiply => multiply(cb, cs),
        BlendMode::Screen => screen(cb, cs),
        // Overlay(cb, cs) = HardLight(cs, cb) (W3C section 9.1.2).
        BlendMode::Overlay => hard_light(cs, cb),
        BlendMode::Darken => cb.min(cs),
        BlendMode::Lighten => cb.max(cs),
        BlendMode::ColorDodge => color_dodge(cb, cs, cfg),
        BlendMode::ColorBurn => color_burn(cb, cs, cfg),
        BlendMode::HardLight => hard_light(cb, cs),
        BlendMode::SoftLight => soft_light(cb, cs, cfg.soft_light),
        BlendMode::Difference => (cb - cs).abs(),
        BlendMode::Exclusion => cb + cs - (cb * cs + cb * cs),
        BlendMode::Add => cb + cs,
        BlendMode::Subtract => cb - cs,
        _ => cs,
    }
}

/// `Lum(C) = 0.3 R + 0.59 G + 0.11 B` (W3C section 10, left to right).
pub fn lum<T: Real>(c: [T; 3]) -> T {
    T::from_f64(0.3) * c[0] + T::from_f64(0.59) * c[1] + T::from_f64(0.11) * c[2]
}

/// `ClipColor` (W3C section 10). When the divisor would be zero the colour becomes `Lum`.
pub fn clip_color<T: Real>(c: [T; 3]) -> [T; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut o = c;
    if n < T::ZERO {
        let d = l - n;
        for e in o.iter_mut() {
            *e = if d > T::ZERO { l + (*e - l) * l / d } else { l };
        }
    }
    if x > T::ONE {
        let d = x - l;
        for e in o.iter_mut() {
            *e = if d > T::ZERO {
                l + (*e - l) * (T::ONE - l) / d
            } else {
                l
            };
        }
    }
    o
}

/// `SetLum(C, l)` (W3C section 10).
pub fn set_lum<T: Real>(c: [T; 3], l: T) -> [T; 3] {
    let d = l - lum(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d])
}

/// `Sat(C) = max - min` (W3C section 10).
pub fn sat<T: Real>(c: [T; 3]) -> T {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

/// `SetSat(C, s)` (W3C section 10): the smallest channel becomes 0, the largest `s`, the middle
/// one is scaled; with equal extremes all channels become 0.
pub fn set_sat<T: Real>(c: [T; 3], s: T) -> [T; 3] {
    let mut idx = [0usize, 1, 2];
    if c[idx[0]] > c[idx[1]] {
        idx.swap(0, 1);
    }
    if c[idx[1]] > c[idx[2]] {
        idx.swap(1, 2);
    }
    if c[idx[0]] > c[idx[1]] {
        idx.swap(0, 1);
    }
    let (imin, imid, imax) = (idx[0], idx[1], idx[2]);
    let mut o = [T::ZERO; 3];
    if c[imax] > c[imin] {
        o[imid] = (c[imid] - c[imin]) * s / (c[imax] - c[imin]);
        o[imax] = s;
    }
    o
}

/// The blend function on a whole straight colour: `B(Cb, Cs)` of W3C sections 9.1 and 10.
pub fn blend_rgb<T: Real>(mode: BlendMode, cb: [T; 3], cs: [T; 3], cfg: &Config) -> [T; 3] {
    match mode {
        BlendMode::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
        BlendMode::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
        BlendMode::Color => set_lum(cs, lum(cb)),
        BlendMode::Luminosity => set_lum(cb, lum(cs)),
        _ => [
            blend_channel(mode, cb[0], cs[0], cfg),
            blend_channel(mode, cb[1], cs[1], cfg),
            blend_channel(mode, cb[2], cs[2], cfg),
        ],
    }
}
