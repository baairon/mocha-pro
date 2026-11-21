//! One mask frame and its file form: a binary graymap (PGM, magic `P5`; see
//! the Netpbm PGM format description) with one canonical header only, so that equal
//! samples always give equal file bytes and equal hashes.
//!
//! ```text
//! P5 LF width SP height LF maxval LF raster
//! ```
//!
//! `width` and `height` are decimal, 1 to 5 digits, no leading zero, value 1 to 65535. `maxval` is
//! exactly `255` (depth 8, one byte per sample) or exactly `65535` (depth 16, two bytes per sample,
//! most significant first). The raster is row by row from the top, each row from the left, and the
//! file ends with the last sample. A sample `s` means coverage `s / maxval`.

/// The sample depth of a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    Eight,
    Sixteen,
}

impl Depth {
    /// The largest sample value, which is also the `maxval` of the header.
    pub fn maxval(self) -> u16 {
        match self {
            Depth::Eight => 255,
            Depth::Sixteen => 65535,
        }
    }

    /// Bits per sample.
    pub fn bits(self) -> u32 {
        match self {
            Depth::Eight => 8,
            Depth::Sixteen => 16,
        }
    }

    /// Bytes per sample in a file.
    pub fn bytes_per_sample(self) -> usize {
        match self {
            Depth::Eight => 1,
            Depth::Sixteen => 2,
        }
    }

    /// The depth for 8 or 16 bits, otherwise `None`.
    pub fn from_bits(bits: u32) -> Option<Depth> {
        match bits {
            8 => Some(Depth::Eight),
            16 => Some(Depth::Sixteen),
            _ => None,
        }
    }
}

/// The largest width or height of a frame.
pub const MAX_SIDE: u32 = 65535;

/// Why a frame cannot be built from its parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// A width or height of 0 or above `MAX_SIDE`.
    Side,
    /// The sample list is not `width * height` long.
    SampleCount { expected: usize, got: usize },
    /// A sample above the `maxval` of the depth.
    SampleRange { index: usize },
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Side => write!(f, "width or height out of range"),
            FrameError::SampleCount { expected, got } => {
                write!(f, "expected {expected} samples, got {got}")
            }
            FrameError::SampleRange { index } => write!(f, "sample {index} above maxval"),
        }
    }
}

impl std::error::Error for FrameError {}

/// Why bytes are not a canonical frame file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PgmError {
    /// The header is not the canonical header.
    Header,
    /// The file ends before the last sample.
    Truncated,
    /// The file has bytes after the last sample.
    Trailing,
}

impl std::fmt::Display for PgmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PgmError::Header => "header is not canonical",
            PgmError::Truncated => "file is shorter than its header and samples",
            PgmError::Trailing => "file has bytes after its samples",
        })
    }
}

impl std::error::Error for PgmError {}

/// A parsed canonical header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub width: u32,
    pub height: u32,
    pub depth: Depth,
    /// The header length in bytes, which is the offset of the raster.
    pub len: usize,
}

impl Header {
    /// The raster length in bytes.
    pub fn raster_len(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth.bytes_per_sample() as u64
    }
}

/// Reads a decimal of 1 to 5 digits, no leading zero, followed at once by `terminator`; returns the
/// value and the position after the terminator.
fn decimal(bytes: &[u8], pos: usize, terminator: u8) -> Option<(u32, usize)> {
    let run = bytes[pos.min(bytes.len())..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if run == 0 || run > 5 || bytes[pos] == b'0' {
        return None;
    }
    if bytes.get(pos + run) != Some(&terminator) {
        return None;
    }
    let mut value = 0u32;
    for &b in &bytes[pos..pos + run] {
        value = value * 10 + (b - b'0') as u32;
    }
    Some((value, pos + run + 1))
}

/// Parses the canonical header at the start of `bytes`.
pub fn parse_header(bytes: &[u8]) -> Result<Header, PgmError> {
    if !bytes.starts_with(b"P5\n") {
        return Err(PgmError::Header);
    }
    let (width, pos) = decimal(bytes, 3, b' ').ok_or(PgmError::Header)?;
    let (height, pos) = decimal(bytes, pos, b'\n').ok_or(PgmError::Header)?;
    let (maxval, len) = decimal(bytes, pos, b'\n').ok_or(PgmError::Header)?;
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(PgmError::Header);
    }
    let depth = match maxval {
        255 => Depth::Eight,
        65535 => Depth::Sixteen,
        _ => return Err(PgmError::Header),
    };
    Ok(Header {
        width,
        height,
        depth,
        len,
    })
}

/// One frame: a width, a height, a depth and `width * height` samples, row by row from the top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    width: u32,
    height: u32,
    depth: Depth,
    samples: Vec<u16>,
}

impl Frame {
    /// Builds a frame, checking the sides, the sample count and the sample range, in that order.
    pub fn new(
        width: u32,
        height: u32,
        depth: Depth,
        samples: Vec<u16>,
    ) -> Result<Frame, FrameError> {
        if width == 0 || width > MAX_SIDE || height == 0 || height > MAX_SIDE {
            return Err(FrameError::Side);
        }
        let expected = width as usize * height as usize;
        if samples.len() != expected {
            return Err(FrameError::SampleCount {
                expected,
                got: samples.len(),
            });
        }
        if let Some(index) = samples.iter().position(|&s| s > depth.maxval()) {
            return Err(FrameError::SampleRange { index });
        }
        Ok(Frame {
            width,
            height,
            depth,
            samples,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn depth(&self) -> Depth {
        self.depth
    }

    pub fn samples(&self) -> &[u16] {
        &self.samples
    }

    /// The canonical file bytes: the header, then the raster.
    pub fn to_pgm(&self) -> Vec<u8> {
        let mut out = format!(
            "P5\n{} {}\n{}\n",
            self.width,
            self.height,
            self.depth.maxval()
        )
        .into_bytes();
        match self.depth {
            Depth::Eight => out.extend(self.samples.iter().map(|&s| s as u8)),
            Depth::Sixteen => {
                for &s in &self.samples {
                    out.extend_from_slice(&s.to_be_bytes());
                }
            }
        }
        out
    }

    /// Reads canonical file bytes; any other spelling is refused.
    pub fn from_pgm(bytes: &[u8]) -> Result<Frame, PgmError> {
        let header = parse_header(bytes)?;
        let want = header.len as u64 + header.raster_len();
        let have = bytes.len() as u64;
        if have < want {
            return Err(PgmError::Truncated);
        }
        if have > want {
            return Err(PgmError::Trailing);
        }
        let raster = &bytes[header.len..];
        let samples: Vec<u16> = match header.depth {
            Depth::Eight => raster.iter().map(|&b| b as u16).collect(),
            Depth::Sixteen => (0..raster.len() / 2)
                .map(|i| u16::from_be_bytes([raster[2 * i], raster[2 * i + 1]]))
                .collect(),
        };
        Ok(Frame {
            width: header.width,
            height: header.height,
            depth: header.depth,
            samples,
        })
    }
}
