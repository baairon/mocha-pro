use std::ops::{Add, Div, Mul, Sub};

/// The scalar type of the operators: `f32` or `f64`. Only IEEE 754 operations are used.
pub trait Real:
    Copy
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
{
    const ZERO: Self;
    const ONE: Self;
    /// Converts a constant that is exactly representable or correctly rounded (`as` cast).
    fn from_f64(x: f64) -> Self;
    /// IEEE 754 square root.
    fn sqrt(self) -> Self;
    fn min(self, o: Self) -> Self {
        if o < self {
            o
        } else {
            self
        }
    }
    fn max(self, o: Self) -> Self {
        if o > self {
            o
        } else {
            self
        }
    }
    fn abs(self) -> Self {
        if self < Self::ZERO {
            Self::ZERO - self
        } else {
            self
        }
    }
}

impl Real for f64 {
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    fn from_f64(x: f64) -> f64 {
        x
    }
    fn sqrt(self) -> f64 {
        f64::sqrt(self)
    }
}

impl Real for f32 {
    const ZERO: f32 = 0.0;
    const ONE: f32 = 1.0;
    fn from_f64(x: f64) -> f32 {
        x as f32
    }
    fn sqrt(self) -> f32 {
        f32::sqrt(self)
    }
}
