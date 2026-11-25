//! The typed errors of the crate.

use std::fmt;

/// Why a rasterisation was refused. No input makes the crate panic; each refusal is one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A coordinate is NaN or infinite. The indexes name the polygon, the ring and the point.
    NonFinite {
        polygon: usize,
        ring: usize,
        point: usize,
    },
    /// A coordinate has an absolute value above [`crate::MAX_COORD`].
    OutOfRange {
        polygon: usize,
        ring: usize,
        point: usize,
    },
    /// A setting is outside its documented range; the text says which.
    BadConfig(&'static str),
    /// The target is larger than [`crate::MAX_SIDE`] on a side or [`crate::MAX_PIXELS`] in area, or its origin is out of range.
    BadTarget,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NonFinite {
                polygon,
                ring,
                point,
            } => write!(
                f,
                "non-finite coordinate at polygon {polygon}, ring {ring}, point {point}"
            ),
            Error::OutOfRange {
                polygon,
                ring,
                point,
            } => write!(
                f,
                "coordinate out of range at polygon {polygon}, ring {ring}, point {point}"
            ),
            Error::BadConfig(why) => write!(f, "bad configuration: {why}"),
            Error::BadTarget => write!(f, "bad target rectangle"),
        }
    }
}

impl std::error::Error for Error {}
