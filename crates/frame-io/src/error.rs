//! Error types of the crate.

use std::fmt;

/// A geometry that the convention does not allow, or a value that cannot be a geometry field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeometryError {
    /// Width is zero.
    ZeroWidth,
    /// Height is zero.
    ZeroHeight,
    /// A depth other than 8, 16 or 32.
    UnsupportedDepth(u32),
    /// A frame rate that is not a positive fraction `N/D`.
    BadRate(String),
    /// A frame larger than 4 GiB; the size in bytes is carried.
    FrameTooLarge(u128),
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeometryError::ZeroWidth => write!(f, "width must be at least 1"),
            GeometryError::ZeroHeight => write!(f, "height must be at least 1"),
            GeometryError::UnsupportedDepth(d) => {
                write!(f, "depth {d} is not one of 8, 16 or 32")
            }
            GeometryError::BadRate(s) => {
                write!(
                    f,
                    "frame rate {s:?} is not an exact fraction N/D with N and D at least 1"
                )
            }
            GeometryError::FrameTooLarge(b) => {
                write!(f, "a frame of {b} bytes is larger than 4 GiB")
            }
        }
    }
}

impl std::error::Error for GeometryError {}

/// A failure of a frame reader or writer.
#[derive(Debug)]
pub enum FrameError {
    /// The stream ended inside a frame: `frame` is its index, `got` the bytes read, `want` the
    /// frame size.
    PartialFrame { frame: u64, got: u64, want: u64 },
    /// A frame handed to a writer has the wrong length.
    BadFrameLength { got: u64, want: u64 },
    /// The stream holds a number of frames other than the declared one.
    CountMismatch { expected: u64, found: u64 },
    /// An input or output failure.
    Io(std::io::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::PartialFrame { frame, got, want } => write!(
                f,
                "the stream ends inside frame {frame}: {got} of {want} bytes"
            ),
            FrameError::BadFrameLength { got, want } => {
                write!(
                    f,
                    "a frame of {got} bytes was given, the geometry needs {want}"
                )
            }
            FrameError::CountMismatch { expected, found } => {
                write!(
                    f,
                    "the stream holds {found} frames, {expected} were declared"
                )
            }
            FrameError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<std::io::Error> for FrameError {
    fn from(e: std::io::Error) -> Self {
        FrameError::Io(e)
    }
}
