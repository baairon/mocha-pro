//! Error type of the crate.

use std::fmt;

/// Everything the crate can refuse.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// A configuration field is out of its documented range; the text names the field.
    Config(String),
    /// An image or region has unusable dimensions or data.
    Image(String),
    /// A 3 by 3 pose cannot be inverted or normalised.
    Singular,
    /// A frame index is outside the sequence.
    NoSuchFrame(usize),
    /// A tracking call needs a pose at a frame that has none.
    NoPose(usize),
    /// The template has too few usable pixels for the chosen model.
    TooFewPixels,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Config(s) => write!(f, "bad configuration: {s}"),
            Error::Image(s) => write!(f, "bad image or region: {s}"),
            Error::Singular => write!(f, "singular pose"),
            Error::NoSuchFrame(i) => write!(f, "no frame {i}"),
            Error::NoPose(i) => write!(f, "frame {i} has no pose"),
            Error::TooFewPixels => write!(f, "too few usable template pixels"),
        }
    }
}

impl std::error::Error for Error {}
