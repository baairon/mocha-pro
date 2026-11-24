//! Depth, exact frame rate and the geometry of a stream, with the checks of the convention.

use crate::error::GeometryError;
use std::fmt;
use std::str::FromStr;

/// The largest frame the convention allows: 4 GiB.
pub const MAX_FRAME_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// A sample depth of the convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Depth {
    /// 8 bit, packed `rgba`.
    Eight,
    /// 16 bit, packed `rgba64le`.
    Sixteen,
    /// 32 bit float, planar `gbrapf32le`.
    ThirtyTwo,
}

impl Depth {
    /// The depth for 8, 16 or 32.
    pub fn from_bits(bits: u32) -> Result<Depth, GeometryError> {
        match bits {
            8 => Ok(Depth::Eight),
            16 => Ok(Depth::Sixteen),
            32 => Ok(Depth::ThirtyTwo),
            other => Err(GeometryError::UnsupportedDepth(other)),
        }
    }

    /// 8, 16 or 32.
    pub fn bits(self) -> u32 {
        match self {
            Depth::Eight => 8,
            Depth::Sixteen => 16,
            Depth::ThirtyTwo => 32,
        }
    }

    /// Bytes of one sample: 1, 2 or 4.
    pub fn bytes_per_sample(self) -> usize {
        match self {
            Depth::Eight => 1,
            Depth::Sixteen => 2,
            Depth::ThirtyTwo => 4,
        }
    }

    /// Bytes of one pixel: 4, 8 or 16.
    pub fn bytes_per_pixel(self) -> usize {
        4 * self.bytes_per_sample()
    }

    /// The ffmpeg pixel format name.
    pub fn pix_fmt(self) -> &'static str {
        match self {
            Depth::Eight => "rgba",
            Depth::Sixteen => "rgba64le",
            Depth::ThirtyTwo => "gbrapf32le",
        }
    }

    /// True for the planar layout (depth 32).
    pub fn is_planar(self) -> bool {
        self == Depth::ThirtyTwo
    }
}

/// An exact frame rate, a fraction of two positive integers such as `30000/1001`. The fraction is
/// kept as written: `60/2` is not rewritten to `30/1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rate {
    /// Numerator, at least 1.
    pub num: u32,
    /// Denominator, at least 1.
    pub den: u32,
}

impl Rate {
    /// A rate from its parts; both must be at least 1.
    pub fn new(num: u32, den: u32) -> Result<Rate, GeometryError> {
        if num == 0 || den == 0 {
            return Err(GeometryError::BadRate(format!("{num}/{den}")));
        }
        Ok(Rate { num, den })
    }

    /// True when both rates are the same value (`60/2` and `30/1` are).
    pub fn same_value(&self, other: &Rate) -> bool {
        u64::from(self.num) * u64::from(other.den) == u64::from(other.num) * u64::from(self.den)
    }
}

impl FromStr for Rate {
    type Err = GeometryError;

    /// `N/D` with decimal digits only: no sign, no space, no decimal point.
    fn from_str(s: &str) -> Result<Rate, GeometryError> {
        let bad = || GeometryError::BadRate(s.to_string());
        let (n, d) = s.split_once('/').ok_or_else(bad)?;
        let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
        if !digits(n) || !digits(d) {
            return Err(bad());
        }
        let num: u32 = n.parse().map_err(|_| bad())?;
        let den: u32 = d.parse().map_err(|_| bad())?;
        Rate::new(num, den).map_err(|_| bad())
    }
}

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

/// The geometry of a stream: size, depth, exact rate and an optional frame count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    /// Width in pixels, at least 1.
    pub width: u32,
    /// Height in pixels, at least 1.
    pub height: u32,
    /// Sample depth.
    pub depth: Depth,
    /// Frame rate.
    pub fps: Rate,
    /// The declared number of frames, when known.
    pub frames: Option<u64>,
}

impl Geometry {
    /// A checked geometry: width and height at least 1, a frame of at most 4 GiB.
    pub fn new(
        width: u32,
        height: u32,
        depth: Depth,
        fps: Rate,
        frames: Option<u64>,
    ) -> Result<Geometry, GeometryError> {
        if width == 0 {
            return Err(GeometryError::ZeroWidth);
        }
        if height == 0 {
            return Err(GeometryError::ZeroHeight);
        }
        let bytes = u128::from(width) * u128::from(height) * depth.bytes_per_pixel() as u128;
        if bytes > u128::from(MAX_FRAME_BYTES) {
            return Err(GeometryError::FrameTooLarge(bytes));
        }
        Ok(Geometry {
            width,
            height,
            depth,
            fps,
            frames,
        })
    }

    /// Pixels in a frame.
    pub fn pixels(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// Bytes of one frame: `width * height * bytes per pixel`.
    pub fn frame_bytes(&self) -> u64 {
        self.pixels() * self.depth.bytes_per_pixel() as u64
    }

    /// Samples in a frame (four per pixel).
    pub fn samples(&self) -> u64 {
        self.pixels() * 4
    }

    /// Bytes of the whole stream when the frame count is declared.
    pub fn stream_bytes(&self) -> Option<u128> {
        self.frames
            .map(|n| u128::from(n) * u128::from(self.frame_bytes()))
    }

    /// Checks a found frame count against the declared one, when there is one.
    pub fn check_count(&self, found: u64) -> Result<(), crate::error::FrameError> {
        match self.frames {
            Some(expected) if expected != found => {
                Err(crate::error::FrameError::CountMismatch { expected, found })
            }
            _ => Ok(()),
        }
    }
}
