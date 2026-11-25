//! Premultiply and unpremultiply, exact in integers and plain arithmetic in floats.

use crate::mode::{Precision, Rounding};

/// What unpremultiply does when alpha is zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZeroAlpha {
    /// The colour becomes 0.
    ZeroColour,
    /// The colour is returned unchanged.
    KeepColour,
}

impl ZeroAlpha {
    /// Both rules, in canonical order.
    pub const ALL: [ZeroAlpha; 2] = [ZeroAlpha::ZeroColour, ZeroAlpha::KeepColour];

    /// The canonical name.
    pub fn name(self) -> &'static str {
        match self {
            ZeroAlpha::ZeroColour => "zero_colour",
            ZeroAlpha::KeepColour => "keep_colour",
        }
    }

    /// Parses a canonical name.
    pub fn parse(s: &str) -> Option<ZeroAlpha> {
        ZeroAlpha::ALL.into_iter().find(|z| z.name() == s)
    }
}

/// `num / den` rounded exactly (integers, `den > 0`) by a mode; the operands are non negative, so
/// `Floor` equals `Truncate`.
fn div_round(num: u64, den: u64, rounding: Rounding) -> u64 {
    let q = num / den;
    let r = num % den;
    match rounding {
        Rounding::Truncate | Rounding::Floor => q,
        Rounding::Ceil => q + u64::from(r > 0),
        Rounding::HalfUp => q + u64::from(2 * r >= den),
        Rounding::TiesEven => {
            let twice = 2 * r;
            if twice > den || (twice == den && q % 2 == 1) {
                q + 1
            } else {
                q
            }
        }
    }
}

/// `colour * alpha / max`, rounded exactly. `colour` and `alpha` are in `0..=max`; `max` must be
/// at least 1 (a `max` of 0 returns 0).
pub fn premultiply_int(colour: u32, alpha: u32, max: u32, rounding: Rounding) -> u32 {
    if max == 0 {
        return 0;
    }
    let v = div_round(
        u64::from(colour) * u64::from(alpha),
        u64::from(max),
        rounding,
    );
    v.min(u64::from(u32::MAX)) as u32
}

/// `colour * max / alpha`, rounded exactly and capped at `max`. Alpha 0 follows `zero`.
pub fn unpremultiply_int(
    colour: u32,
    alpha: u32,
    max: u32,
    rounding: Rounding,
    zero: ZeroAlpha,
) -> u32 {
    if alpha == 0 {
        return match zero {
            ZeroAlpha::ZeroColour => 0,
            ZeroAlpha::KeepColour => colour,
        };
    }
    let v = div_round(
        u64::from(colour) * u64::from(max),
        u64::from(alpha),
        rounding,
    );
    v.min(u64::from(max)) as u32
}

/// `colour * alpha` in the given precision.
pub fn premultiply_f32(colour: f32, alpha: f32, precision: Precision) -> f32 {
    match precision {
        Precision::F32 => colour * alpha,
        Precision::F64 => (f64::from(colour) * f64::from(alpha)) as f32,
    }
}

/// `colour / alpha` in the given precision; alpha exactly 0 (either sign) follows `zero`.
pub fn unpremultiply_f32(colour: f32, alpha: f32, precision: Precision, zero: ZeroAlpha) -> f32 {
    if alpha == 0.0 {
        return match zero {
            ZeroAlpha::ZeroColour => 0.0,
            ZeroAlpha::KeepColour => colour,
        };
    }
    match precision {
        Precision::F32 => colour / alpha,
        Precision::F64 => (f64::from(colour) / f64::from(alpha)) as f32,
    }
}
