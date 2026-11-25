//! Integer to float, float to integer and the multiply add rescale.

use crate::mode::{Clamp, Precision, Rounding};

/// Integer to float: `v / denom` in the given precision.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntToFloat {
    /// The divisor, normally the integer value of full scale.
    pub denom: f64,
    /// The precision of the division.
    pub precision: Precision,
}

/// Float to integer: `x * scale + offset`, rounded, then clamped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatToInt {
    /// The multiplier, normally the integer value of full scale.
    pub scale: f64,
    /// The addend applied after the multiply and before the rounding.
    pub offset: f64,
    /// The rounding mode.
    pub rounding: Rounding,
    /// The clamp of the rounded value.
    pub clamp: Clamp,
    /// The precision of the multiply, the add and the rounding.
    pub precision: Precision,
}

/// The generic multiply add round rule on a 16 bit value: `round(v * mul + add)`, saturating to
/// 0 and 65535. Full range to host range is the instance `mul = 32768 / 65535`, `add = 0.5` with
/// `Truncate`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rescale {
    /// The multiplier.
    pub mul: f64,
    /// The addend.
    pub add: f64,
    /// The rounding mode.
    pub rounding: Rounding,
    /// The precision of the multiply, the add and the rounding.
    pub precision: Precision,
}

impl Rescale {
    /// Applies the rule to one value.
    pub fn apply(&self, v: u16) -> u16 {
        let r = self
            .precision
            .mul_add_round(f64::from(v), self.mul, self.add, self.rounding);
        Clamp::None.apply(r, 65535).min(65535) as u16
    }
}

/// Converts an integer to a float.
pub fn int_to_float(v: u32, c: &IntToFloat) -> f32 {
    match c.precision {
        Precision::F32 => (v as f32) / (c.denom as f32),
        Precision::F64 => (f64::from(v) / c.denom) as f32,
    }
}

/// Converts a float to an integer in `0..=max` (`max` is used by the clamps only; with
/// `Clamp::None` the result may exceed it).
pub fn float_to_int(x: f32, max: u32, c: &FloatToInt) -> u32 {
    let r = c
        .precision
        .mul_add_round(f64::from(x), c.scale, c.offset, c.rounding);
    c.clamp.apply(r, max)
}
