//! The `mask-1` manifest: ASCII text, LF line ends, one `key=value` record per line in a fixed
//! order, every line ended by LF, no empty line, no comment and no byte outside 0x20 to 0x7E except
//! LF. Its default name is `manifest.txt`.
//!
//! ```text
//! format=mask-1
//! frames=N
//! first=F
//! width=W
//! height=H
//! depth=8 or 16
//! channels=1
//! naming=frame_######.pgm
//! frame.I.sha256=64 lower-case hexadecimal digits   (one line per frame, I from F to F+N-1)
//! ```
//!
//! A decimal is `0` or a digit 1 to 9 followed by digits. The frame of index `I` is the file
//! `frame_` + `I` zero padded to 6 digits + `.pgm`. The hash of a frame is the SHA-256 of the whole
//! frame file. Records are read in order; the first fault wins and names its line.

use crate::pgm::{Depth, MAX_SIDE};
use crate::sha256::{from_hex, to_hex};

/// The form name in the first line.
pub const FORMAT: &str = "mask-1";
/// The default manifest file name.
pub const MANIFEST_NAME: &str = "manifest.txt";
/// The value of the `naming` record.
pub const NAMING: &str = "frame_######.pgm";
/// The most frames in one sequence.
pub const MAX_FRAMES: u32 = 1_000_000;
/// The largest frame index.
pub const MAX_INDEX: u32 = 999_999;

/// The file name of the frame of index `index` (at most `MAX_INDEX`).
pub fn frame_name(index: u32) -> String {
    format!("frame_{index:06}.pgm")
}

/// The kind of a manifest fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestErrorKind {
    /// A line breaks the form.
    Syntax,
    /// The first line names another form.
    Format,
    /// A value is in its grammar but out of its range.
    Value,
}

/// A manifest fault and the line (counted from 1) that holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestError {
    pub kind: ManifestErrorKind,
    pub line: usize,
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let word = match self.kind {
            ManifestErrorKind::Syntax => "manifest_syntax",
            ManifestErrorKind::Format => "manifest_format",
            ManifestErrorKind::Value => "manifest_value",
        };
        write!(f, "{word}: line {}", self.line)
    }
}

impl std::error::Error for ManifestError {}

fn syntax(line: usize) -> ManifestError {
    ManifestError {
        kind: ManifestErrorKind::Syntax,
        line,
    }
}

fn value(line: usize) -> ManifestError {
    ManifestError {
        kind: ManifestErrorKind::Value,
        line,
    }
}

/// A manifest value in its decimal form; `None` is a grammar fault. A number too large for `u64`
/// is `Some(u64::MAX)`, which every range refuses.
fn decimal(v: &[u8]) -> Option<u64> {
    if v.is_empty() || !v.iter().all(u8::is_ascii_digit) || (v[0] == b'0' && v.len() > 1) {
        return None;
    }
    let mut n = 0u64;
    for &b in v {
        n = match n
            .checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as u64))
        {
            Some(n) => n,
            None => return Some(u64::MAX),
        };
    }
    Some(n)
}

/// The value of line `r` read at `*pos`, whose key must be `key`; advances past the line.
fn record<'a>(
    bytes: &'a [u8],
    pos: &mut usize,
    r: usize,
    key: &str,
) -> Result<&'a [u8], ManifestError> {
    if *pos >= bytes.len() {
        return Err(syntax(r));
    }
    let rest = &bytes[*pos..];
    let end = rest.iter().position(|&b| b == b'\n').ok_or(syntax(r))?;
    let line = &rest[..end];
    if line.iter().any(|&b| !(0x20..=0x7e).contains(&b)) {
        return Err(syntax(r));
    }
    let eq = line.iter().position(|&b| b == b'=').ok_or(syntax(r))?;
    if &line[..eq] != key.as_bytes() {
        return Err(syntax(r));
    }
    *pos += end + 1;
    Ok(&rest[eq + 1..end])
}

/// The parsed content of a manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    first: u32,
    width: u32,
    height: u32,
    depth: Depth,
    hashes: Vec<[u8; 32]>,
}

impl Manifest {
    /// Builds a manifest, refusing what the parser would refuse and naming the parser's line.
    pub fn new(
        first: u32,
        width: u32,
        height: u32,
        depth: Depth,
        hashes: Vec<[u8; 32]>,
    ) -> Result<Manifest, ManifestError> {
        let frames = hashes.len() as u64;
        if frames == 0 || frames > MAX_FRAMES as u64 {
            return Err(value(2));
        }
        if first > MAX_INDEX || first as u64 + frames - 1 > MAX_INDEX as u64 {
            return Err(value(3));
        }
        if width == 0 || width > MAX_SIDE {
            return Err(value(4));
        }
        if height == 0 || height > MAX_SIDE {
            return Err(value(5));
        }
        Ok(Manifest {
            first,
            width,
            height,
            depth,
            hashes,
        })
    }

    pub fn frames(&self) -> u32 {
        self.hashes.len() as u32
    }

    pub fn first(&self) -> u32 {
        self.first
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

    pub fn hashes(&self) -> &[[u8; 32]] {
        &self.hashes
    }

    /// The frame file names, in index order.
    pub fn frame_names(&self) -> Vec<String> {
        (0..self.frames())
            .map(|k| frame_name(self.first + k))
            .collect()
    }

    /// Reads a manifest; the first fault wins.
    pub fn parse(bytes: &[u8]) -> Result<Manifest, ManifestError> {
        let mut pos = 0usize;
        let range = |v: &[u8], line: usize, lo: u64, hi: u64| -> Result<u32, ManifestError> {
            let n = decimal(v).ok_or(syntax(line))?;
            if n < lo || n > hi {
                return Err(value(line));
            }
            Ok(n as u32)
        };
        if record(bytes, &mut pos, 1, "format")? != FORMAT.as_bytes() {
            return Err(ManifestError {
                kind: ManifestErrorKind::Format,
                line: 1,
            });
        }
        let frames = range(
            record(bytes, &mut pos, 2, "frames")?,
            2,
            1,
            MAX_FRAMES as u64,
        )?;
        let first = range(record(bytes, &mut pos, 3, "first")?, 3, 0, MAX_INDEX as u64)?;
        if first as u64 + frames as u64 - 1 > MAX_INDEX as u64 {
            return Err(value(3));
        }
        let width = range(record(bytes, &mut pos, 4, "width")?, 4, 1, MAX_SIDE as u64)?;
        let height = range(record(bytes, &mut pos, 5, "height")?, 5, 1, MAX_SIDE as u64)?;
        let depth = range(record(bytes, &mut pos, 6, "depth")?, 6, 8, 16)?;
        let depth = Depth::from_bits(depth).ok_or(value(6))?;
        range(record(bytes, &mut pos, 7, "channels")?, 7, 1, 1)?;
        if record(bytes, &mut pos, 8, "naming")? != NAMING.as_bytes() {
            return Err(value(8));
        }
        let mut hashes = Vec::new();
        for k in 0..frames {
            let line = 9 + k as usize;
            let v = record(
                bytes,
                &mut pos,
                line,
                &format!("frame.{}.sha256", first + k),
            )?;
            let text = std::str::from_utf8(v).map_err(|_| syntax(line))?;
            hashes.push(from_hex(text).ok_or(syntax(line))?);
        }
        if pos < bytes.len() {
            return Err(syntax(9 + frames as usize));
        }
        Ok(Manifest {
            first,
            width,
            height,
            depth,
            hashes,
        })
    }

    /// The canonical manifest text.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut s = format!(
            "format={FORMAT}\nframes={}\nfirst={}\nwidth={}\nheight={}\ndepth={}\nchannels=1\nnaming={NAMING}\n",
            self.frames(),
            self.first,
            self.width,
            self.height,
            self.depth.bits()
        );
        for (k, h) in self.hashes.iter().enumerate() {
            s.push_str(&format!(
                "frame.{}.sha256={}\n",
                self.first as usize + k,
                to_hex(h)
            ));
        }
        s.into_bytes()
    }
}
