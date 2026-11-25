//! Coverage to a sample: scale, round, clamp.

use crate::config::{Depth, Quantise, Rounding, Scale};

/// The output buffer in the configured sample type, row by row.
#[derive(Debug, Clone, PartialEq)]
pub enum Buffer {
    U8(Vec<u8>),
    U16(Vec<u16>),
    F32(Vec<f32>),
}

impl Buffer {
    /// The number of samples.
    pub fn len(&self) -> usize {
        match self {
            Buffer::U8(v) => v.len(),
            Buffer::U16(v) => v.len(),
            Buffer::F32(v) => v.len(),
        }
    }

    /// True when there are no samples.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sample `k` as a double (a code is returned as the integer value, not divided).
    pub fn get(&self, k: usize) -> f64 {
        match self {
            Buffer::U8(v) => v[k] as f64,
            Buffer::U16(v) => v[k] as f64,
            Buffer::F32(v) => v[k] as f64,
        }
    }
}

fn round(v: f64, r: Rounding) -> f64 {
    match r {
        // Not `(v + 0.5).floor()`: for v = 0.49999999999999994 the sum rounds to 1.0.
        Rounding::NearestHalfUp => {
            let f = v.floor();
            if v - f >= 0.5 {
                f + 1.0
            } else {
                f
            }
        }
        Rounding::NearestEven => v.round_ties_even(),
        Rounding::Floor => v.floor(),
        Rounding::Ceil => v.ceil(),
    }
}

/// The code for one coverage `v` in 0 to 1: `clamp(round(v * factor), 0, max)`.
pub fn code(v: f64, q: &Quantise) -> u32 {
    let max: f64 = match q.depth {
        Depth::Bits8 => 255.0,
        Depth::Bits16 => 65535.0,
        Depth::Float32 => return 0,
    };
    let factor = match q.scale {
        Scale::Full => max,
        Scale::PowerOfTwo => max + 1.0,
        Scale::Custom(f) => f,
    };
    let c = round(v * factor, q.rounding);
    c.clamp(0.0, max) as u32
}

pub(crate) fn quantise(plane: &[f64], q: Option<&Quantise>) -> Buffer {
    match q {
        None
        | Some(Quantise {
            depth: Depth::Float32,
            ..
        }) => Buffer::F32(plane.iter().map(|&v| v as f32).collect()),
        Some(q) if q.depth == Depth::Bits8 => {
            Buffer::U8(plane.iter().map(|&v| code(v, q) as u8).collect())
        }
        Some(q) => Buffer::U16(plane.iter().map(|&v| code(v, q) as u16).collect()),
    }
}
