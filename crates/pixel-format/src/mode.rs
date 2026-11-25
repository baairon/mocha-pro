//! The choice types: depth, rounding, precision, clamp and the order of premultiply and conversion.

use frame_io::Depth as Storage;

/// A sample depth. `max` of `U16` is the integer value that stands for full scale (32768 for the
/// host range of the After Effects SDK, 65535 for the full range of the type).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Depth {
    /// 8 bit, 0 to 255.
    U8,
    /// 16 bit, 0 to `max`.
    U16 { max: u16 },
    /// 32 bit float, nominal 0 to 1.
    F32,
}

impl Depth {
    /// The integer value of full scale: 255, `max`, or 0 for float (which has none).
    pub fn max_value(self) -> u32 {
        match self {
            Depth::U8 => 255,
            Depth::U16 { max } => u32::from(max),
            Depth::F32 => 0,
        }
    }

    /// True for the float depth.
    pub fn is_float(self) -> bool {
        matches!(self, Depth::F32)
    }

    /// Bytes of one sample: 1, 2 or 4 (taken from the storage depth of `frame_io`).
    pub fn bytes_per_sample(self) -> usize {
        self.storage().bytes_per_sample()
    }

    /// The storage depth of `frame_io` with the same width.
    pub fn storage(self) -> Storage {
        match self {
            Depth::U8 => Storage::Eight,
            Depth::U16 { .. } => Storage::Sixteen,
            Depth::F32 => Storage::ThirtyTwo,
        }
    }

    /// The depth for a storage depth; `u16_max` is used when the storage depth is 16 bit.
    pub fn from_storage(storage: Storage, u16_max: u16) -> Depth {
        match storage {
            Storage::Eight => Depth::U8,
            Storage::Sixteen => Depth::U16 { max: u16_max },
            Storage::ThirtyTwo => Depth::F32,
        }
    }
}

/// How a real number becomes an integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rounding {
    /// `floor(x + 0.5)`: ties go up.
    HalfUp,
    /// IEEE roundTiesToEven.
    TiesEven,
    /// Toward zero.
    Truncate,
    /// Toward negative infinity.
    Floor,
    /// Toward positive infinity.
    Ceil,
}

impl Rounding {
    /// Every mode, in canonical order.
    pub const ALL: [Rounding; 5] = [
        Rounding::HalfUp,
        Rounding::TiesEven,
        Rounding::Truncate,
        Rounding::Floor,
        Rounding::Ceil,
    ];

    /// The canonical name.
    pub fn name(self) -> &'static str {
        match self {
            Rounding::HalfUp => "half_up",
            Rounding::TiesEven => "ties_even",
            Rounding::Truncate => "truncate",
            Rounding::Floor => "floor",
            Rounding::Ceil => "ceil",
        }
    }

    /// Parses a canonical name.
    pub fn parse(s: &str) -> Option<Rounding> {
        Rounding::ALL.into_iter().find(|r| r.name() == s)
    }

    /// Rounds an `f64` to an integral `f64` (infinities and NaN pass through).
    pub fn round_f64(self, x: f64) -> f64 {
        match self {
            Rounding::HalfUp => (x + 0.5).floor(),
            Rounding::TiesEven => x.round_ties_even(),
            Rounding::Truncate => x.trunc(),
            Rounding::Floor => x.floor(),
            Rounding::Ceil => x.ceil(),
        }
    }

    /// Rounds an `f32` to an integral `f32`; the half up step `x + 0.5` is itself performed in `f32`.
    pub fn round_f32(self, x: f32) -> f32 {
        match self {
            Rounding::HalfUp => (x + 0.5).floor(),
            Rounding::TiesEven => x.round_ties_even(),
            Rounding::Truncate => x.trunc(),
            Rounding::Floor => x.floor(),
            Rounding::Ceil => x.ceil(),
        }
    }
}

/// The floating point type in which the arithmetic of a conversion is performed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Precision {
    /// Every operand is narrowed to `f32` and every operation result is an `f32`.
    F32,
    /// Every operation is performed in `f64`.
    F64,
}

impl Precision {
    /// Both precisions, in canonical order.
    pub const ALL: [Precision; 2] = [Precision::F32, Precision::F64];

    /// The canonical name.
    pub fn name(self) -> &'static str {
        match self {
            Precision::F32 => "f32",
            Precision::F64 => "f64",
        }
    }

    /// Parses a canonical name.
    pub fn parse(s: &str) -> Option<Precision> {
        Precision::ALL.into_iter().find(|p| p.name() == s)
    }

    /// `x * mul + add` rounded by `rounding`, with the multiply and the add as two separate
    /// operations (no fused multiply add), in this precision. The result is widened to `f64`.
    pub fn mul_add_round(self, x: f64, mul: f64, add: f64, rounding: Rounding) -> f64 {
        match self {
            Precision::F32 => {
                let product = (x as f32) * (mul as f32);
                let sum = product + (add as f32);
                f64::from(rounding.round_f32(sum))
            }
            Precision::F64 => rounding.round_f64(x * mul + add),
        }
    }
}

/// What happens to a rounded result that lies outside 0 to the maximum, or is NaN.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Clamp {
    /// No range clamp: the rounded value is cast to `u32` saturating at 0 and `u32::MAX`; NaN
    /// becomes 0.
    None,
    /// Clamp to 0 and the maximum with `min` then `max` of IEEE minNum and maxNum: NaN becomes the
    /// maximum.
    Range,
    /// Clamp to 0 and the maximum; NaN becomes 0.
    RangeNanZero,
}

impl Clamp {
    /// Every clamp, in canonical order.
    pub const ALL: [Clamp; 3] = [Clamp::None, Clamp::Range, Clamp::RangeNanZero];

    /// The canonical name.
    pub fn name(self) -> &'static str {
        match self {
            Clamp::None => "none",
            Clamp::Range => "range",
            Clamp::RangeNanZero => "range_nan_zero",
        }
    }

    /// Parses a canonical name.
    pub fn parse(s: &str) -> Option<Clamp> {
        Clamp::ALL.into_iter().find(|c| c.name() == s)
    }

    /// Applies the clamp to a rounded value and returns the integer.
    pub fn apply(self, rounded: f64, max: u32) -> u32 {
        match self {
            Clamp::None => rounded as u32,
            Clamp::Range => rounded.min(f64::from(max)).max(0.0) as u32,
            Clamp::RangeNanZero => {
                if rounded.is_nan() {
                    0
                } else {
                    rounded.min(f64::from(max)).max(0.0) as u32
                }
            }
        }
    }
}

/// Whether a pixel conversion premultiplies at the source depth or at the target depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PremulOrder {
    /// Convert every channel to the target depth, then apply the alpha operation there.
    ConvertFirst,
    /// Apply the alpha operation at the source depth, then convert every channel.
    AlphaFirst,
}

impl PremulOrder {
    /// Both orders, in canonical order.
    pub const ALL: [PremulOrder; 2] = [PremulOrder::ConvertFirst, PremulOrder::AlphaFirst];

    /// The canonical name.
    pub fn name(self) -> &'static str {
        match self {
            PremulOrder::ConvertFirst => "convert_first",
            PremulOrder::AlphaFirst => "alpha_first",
        }
    }

    /// Parses a canonical name.
    pub fn parse(s: &str) -> Option<PremulOrder> {
        PremulOrder::ALL.into_iter().find(|o| o.name() == s)
    }
}
