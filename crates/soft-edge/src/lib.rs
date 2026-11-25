//! `soft-edge`: soft edge operators on a one channel `f32` coverage buffer, in plain Rust with no
//! dependency.
//!
//! Buffers are row major slices with an explicit width and height. The operators are a separable
//! box blur with an integer or fractional radius (`box_blur`, `box_blur_repeated`), a separable
//! Gaussian generated from a sigma or a radius (`gaussian_blur`), and a feather that ramps over
//! the signed distance to the zero crossing of a hard mask (`feather`), with a linear, smoothstep
//! or cosine profile, an inward, outward or centred ramp and a zero, clamp or mirror border rule.
//! Every choice a calibration may vary is a field of `Config`, and `Config::enumerate` lists the
//! discrete configuration space. Results use only IEEE 754 add, subtract, multiply, divide and
//! square root, never a fused multiply add, and are bit identical on every platform.
//!
//! The published definitions are cited in the module documents: `kernel`, `feather`, `edt`.

pub mod border;
pub mod config;
pub mod edt;
pub mod feather;
pub mod kernel;

pub use border::{resolve_index, Border};
pub use config::{Config, Direction, ExpMethod, FracRule, Profile};
pub use edt::{squared_distance_transform, NO_FEATURE};
pub use feather::{feather, ramp, signed_distance};
pub use kernel::{
    box_blur, box_blur_repeated, box_kernel, convolve_separable, exp_neg, gaussian_blur,
    gaussian_kernel, GaussSpec,
};

/// Errors of the operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The buffer length is not `width * height`.
    SizeMismatch,
    /// A parameter is out of range; the text says which.
    BadParameter(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SizeMismatch => write!(f, "buffer length is not width * height"),
            Error::BadParameter(m) => write!(f, "bad parameter: {m}"),
        }
    }
}

impl std::error::Error for Error {}
