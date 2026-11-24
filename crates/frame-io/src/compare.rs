//! The exact comparer: two streams of one geometry compared as bit patterns at tolerance 0.
//! Samples are compared as their stored bytes, so at depth 32 negative zero differs from zero, two
//! NaNs with different payloads differ, and two NaNs with the same bit pattern are equal.

use crate::error::FrameError;
use crate::geometry::Geometry;
use crate::io::FrameReader;
use crate::sample::{read_sample, CHANNEL_NAMES};
use std::fmt;
use std::io::Read;

/// Which of the two streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The first stream.
    A,
    /// The second stream.
    B,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::A => write!(f, "a"),
            Side::B => write!(f, "b"),
        }
    }
}

/// A failure while reading one of the streams.
#[derive(Debug)]
pub struct CompareError {
    /// The stream that failed.
    pub side: Side,
    /// The failure.
    pub error: FrameError,
}

impl fmt::Display for CompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stream {}: {}", self.side, self.error)
    }
}

impl std::error::Error for CompareError {}

/// The first differing sample: frame, pixel, channel and both raw bit patterns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Difference {
    /// Frame index.
    pub frame: u64,
    /// Column.
    pub x: u32,
    /// Row.
    pub y: u32,
    /// Channel number: 0 R, 1 G, 2 B, 3 A.
    pub channel: usize,
    /// Bit pattern in stream A.
    pub a: u32,
    /// Bit pattern in stream B.
    pub b: u32,
}

impl Difference {
    /// The channel letter.
    pub fn channel_name(&self) -> char {
        CHANNEL_NAMES[self.channel]
    }
}

/// The verdict on one frame present in both streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameVerdict {
    /// Frame index.
    pub frame: u64,
    /// True when every byte is equal.
    pub equal: bool,
    /// Samples that differ in this frame.
    pub samples_different: u64,
}

/// The result of a comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comparison {
    /// Frames in stream A.
    pub frames_a: u64,
    /// Frames in stream B.
    pub frames_b: u64,
    /// Frames present in both and compared.
    pub frames_compared: u64,
    /// Compared frames that are equal.
    pub frames_equal: u64,
    /// Compared frames that differ.
    pub frames_different: u64,
    /// Samples that differ, over all compared frames.
    pub samples_different: u64,
    /// The first differing sample, in frame order, then row, column and channel R, G, B, A.
    pub first: Option<Difference>,
}

impl Comparison {
    /// True when both streams hold the same number of frames and every frame is equal.
    pub fn is_equal(&self) -> bool {
        self.frames_a == self.frames_b && self.frames_different == 0
    }
}

fn first_difference(g: &Geometry, frame: u64, a: &[u8], b: &[u8]) -> Option<Difference> {
    for y in 0..g.height {
        for x in 0..g.width {
            for channel in 0..4 {
                let va = read_sample(g, a, x, y, channel);
                let vb = read_sample(g, b, x, y, channel);
                if va != vb {
                    return Some(Difference {
                        frame,
                        x,
                        y,
                        channel,
                        a: va,
                        b: vb,
                    });
                }
            }
        }
    }
    None
}

fn count_different(g: &Geometry, a: &[u8], b: &[u8]) -> u64 {
    let n = g.depth.bytes_per_sample();
    a.chunks_exact(n)
        .zip(b.chunks_exact(n))
        .filter(|(x, y)| x != y)
        .count() as u64
}

/// Compares two streams of geometry `g`. `on_frame` is called once per frame present in both. A
/// stream that ends inside a frame, or whose frame count differs from a declared `g.frames`, is an
/// error that names the stream. Frames of the longer stream past the end of the shorter one are
/// counted and make the streams unequal.
pub fn compare<RA: Read, RB: Read>(
    g: &Geometry,
    a: RA,
    b: RB,
    mut on_frame: impl FnMut(&FrameVerdict),
) -> Result<Comparison, CompareError> {
    let mut ra = FrameReader::new(a, *g);
    let mut rb = FrameReader::new(b, *g);
    let (mut fa, mut fb) = (Vec::new(), Vec::new());
    let mut out = Comparison {
        frames_a: 0,
        frames_b: 0,
        frames_compared: 0,
        frames_equal: 0,
        frames_different: 0,
        samples_different: 0,
        first: None,
    };
    let err = |side, error| CompareError { side, error };
    loop {
        let has_a = ra.read_frame(&mut fa).map_err(|e| err(Side::A, e))?;
        let has_b = rb.read_frame(&mut fb).map_err(|e| err(Side::B, e))?;
        if has_a {
            out.frames_a += 1;
        }
        if has_b {
            out.frames_b += 1;
        }
        if !(has_a && has_b) {
            // One stream ended or both did: count what the longer one still holds.
            while has_a && ra.read_frame(&mut fa).map_err(|e| err(Side::A, e))? {
                out.frames_a += 1;
            }
            while has_b && rb.read_frame(&mut fb).map_err(|e| err(Side::B, e))? {
                out.frames_b += 1;
            }
            break;
        }
        let frame = out.frames_compared;
        out.frames_compared += 1;
        let equal = fa == fb;
        let different = if equal {
            0
        } else {
            count_different(g, &fa, &fb)
        };
        if equal {
            out.frames_equal += 1;
        } else {
            out.frames_different += 1;
            out.samples_different += different;
            if out.first.is_none() {
                out.first = first_difference(g, frame, &fa, &fb);
            }
        }
        on_frame(&FrameVerdict {
            frame,
            equal,
            samples_different: different,
        });
    }
    g.check_count(out.frames_a).map_err(|e| err(Side::A, e))?;
    g.check_count(out.frames_b).map_err(|e| err(Side::B, e))?;
    Ok(out)
}
