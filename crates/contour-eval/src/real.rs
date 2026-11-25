//! The number type of an evaluation: `f32` or `f64`, chosen by [`crate::Precision`].
//!
//! Only add, subtract, multiply, divide and negate are used by the evaluators, the operations
//! whose results IEEE 754 fixes. No fused multiply add is used and no libm function is called.

use std::ops::{Add, Div, Mul, Neg, Sub};

pub(crate) trait Real:
    Copy
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    fn from_f64(x: f64) -> Self;
    fn to_f64(self) -> f64;
}

impl Real for f64 {
    fn from_f64(x: f64) -> Self {
        x
    }
    fn to_f64(self) -> f64 {
        self
    }
}

impl Real for f32 {
    fn from_f64(x: f64) -> Self {
        x as f32
    }
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
}
