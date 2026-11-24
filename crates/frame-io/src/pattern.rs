//! The synthetic frame generator. Every pattern is a pure function of the geometry, the pattern
//! and the frame index, built with integer arithmetic (and, at depth 32, one exact float division
//! of an integer by 65535 or by a power of two), so that the bytes are the same on every platform.
//!
//! Patterns are named by a text form that `FromStr` and `Display` share:
//!
//! - `constant:R,G,B,A`: every pixel has the four levels given (0 to 65535, 65535 is full scale);
//! - `hramp`: R, G and B ramp from level 0 at column 0 to 65535 at the last column, A is full;
//! - `vramp`: the same down the rows;
//! - `checker:CELL`: square cells of CELL pixels (at least 1), black and white with A full; the
//!   colours swap on every frame;
//! - `random:SEED`: every sample is a draw of `SplitMix64` seeded from SEED and the frame index;
//! - `aramp`: R, G, B are the constant levels 49152, 32768, 16384 and A ramps along the columns
//!   (straight alpha: the colour does not change with A).
//!
//! A level maps to a sample by `level_to_bits`: depth 8 is `(level * 255 + 32767) / 65535`, depth 16
//! is the level, depth 32 is `level / 65535` as a float.

use crate::geometry::{Depth, Geometry};
use crate::sample::write_sample;
use std::fmt;
use std::str::FromStr;

/// Full scale level.
pub const FULL: u16 = 65535;

/// The colour of the alpha ramp pattern.
pub const ALPHA_RAMP_RGB: [u16; 3] = [49152, 32768, 16384];

/// The SplitMix64 generator (a 64 bit state, three multiplications per draw). Written here so that
/// the crate has no dependency.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator in the given state.
    pub fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }

    /// The next 64 bit draw.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// A pattern of the generator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    /// One level per channel (R, G, B, A).
    Constant([u16; 4]),
    /// Horizontal ramp.
    HRamp,
    /// Vertical ramp.
    VRamp,
    /// Checkerboard with square cells of the given size.
    Checker(u32),
    /// Seeded pseudo random field.
    Random(u64),
    /// Alpha ramp over a constant colour.
    AlphaRamp,
}

/// A pattern text that does not parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternError(pub String);

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown pattern {:?}", self.0)
    }
}

impl std::error::Error for PatternError {}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

impl FromStr for Pattern {
    type Err = PatternError;

    fn from_str(s: &str) -> Result<Pattern, PatternError> {
        let bad = || PatternError(s.to_string());
        let (name, arg) = match s.split_once(':') {
            Some((n, a)) => (n, Some(a)),
            None => (s, None),
        };
        match (name, arg) {
            ("hramp", None) => Ok(Pattern::HRamp),
            ("vramp", None) => Ok(Pattern::VRamp),
            ("aramp", None) => Ok(Pattern::AlphaRamp),
            ("checker", Some(a)) if digits(a) => {
                let cell: u32 = a.parse().map_err(|_| bad())?;
                if cell == 0 {
                    return Err(bad());
                }
                Ok(Pattern::Checker(cell))
            }
            ("random", Some(a)) if digits(a) => Ok(Pattern::Random(a.parse().map_err(|_| bad())?)),
            ("constant", Some(a)) => {
                let parts: Vec<&str> = a.split(',').collect();
                if parts.len() != 4 || !parts.iter().all(|p| digits(p)) {
                    return Err(bad());
                }
                let mut levels = [0u16; 4];
                for (slot, p) in levels.iter_mut().zip(parts) {
                    *slot = p.parse().map_err(|_| bad())?;
                }
                Ok(Pattern::Constant(levels))
            }
            _ => Err(bad()),
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Constant(l) => write!(f, "constant:{},{},{},{}", l[0], l[1], l[2], l[3]),
            Pattern::HRamp => write!(f, "hramp"),
            Pattern::VRamp => write!(f, "vramp"),
            Pattern::Checker(c) => write!(f, "checker:{c}"),
            Pattern::Random(s) => write!(f, "random:{s}"),
            Pattern::AlphaRamp => write!(f, "aramp"),
        }
    }
}

/// Maps a level (0 to 65535) to the raw bit pattern of a sample of the given depth.
pub fn level_to_bits(depth: Depth, level: u16) -> u32 {
    match depth {
        Depth::Eight => (u32::from(level) * 255 + 32767) / 65535,
        Depth::Sixteen => u32::from(level),
        Depth::ThirtyTwo => (f32::from(level) / 65535.0).to_bits(),
    }
}

/// The level of position `pos` on an axis of `len` positions: 0 at the first, 65535 at the last,
/// rounded to nearest in between; an axis of one position is level 0.
pub fn ramp_level(pos: u32, len: u32) -> u16 {
    if len <= 1 {
        return 0;
    }
    let last = u64::from(len - 1);
    ((u64::from(pos) * 65535 + last / 2) / last) as u16
}

/// A random draw as a sample of the given depth: the top bits of the draw at depth 8 and 16, and
/// 24 bits over 2^24 (a float in [0, 1), exact) at depth 32.
fn random_bits(depth: Depth, draw: u64) -> u32 {
    match depth {
        Depth::Eight => (draw >> 56) as u32,
        Depth::Sixteen => (draw >> 48) as u32,
        Depth::ThirtyTwo => ((draw >> 40) as f32 / 16_777_216.0).to_bits(),
    }
}

impl Pattern {
    /// The four levels of a pixel for the patterns that are level based (not `Random`).
    fn levels(&self, g: &Geometry, frame: u64, x: u32, y: u32) -> [u16; 4] {
        match self {
            Pattern::Constant(l) => *l,
            Pattern::HRamp => {
                let v = ramp_level(x, g.width);
                [v, v, v, FULL]
            }
            Pattern::VRamp => {
                let v = ramp_level(y, g.height);
                [v, v, v, FULL]
            }
            Pattern::Checker(cell) => {
                let parity = (u64::from(x / cell) + u64::from(y / cell) + frame) & 1;
                let v = if parity == 0 { FULL } else { 0 };
                [v, v, v, FULL]
            }
            Pattern::AlphaRamp => [
                ALPHA_RAMP_RGB[0],
                ALPHA_RAMP_RGB[1],
                ALPHA_RAMP_RGB[2],
                ramp_level(x, g.width),
            ],
            Pattern::Random(_) => [0, 0, 0, 0],
        }
    }

    /// Fills `buf` (resized to the frame size) with frame number `frame` of the pattern.
    pub fn fill_frame(&self, g: &Geometry, frame: u64, buf: &mut Vec<u8>) {
        buf.clear();
        buf.resize(g.frame_bytes() as usize, 0);
        if let Pattern::Random(seed) = self {
            let mut rng =
                SplitMix64::new(seed.wrapping_add(frame.wrapping_mul(0xD1B5_4A32_D192_ED03)));
            for y in 0..g.height {
                for x in 0..g.width {
                    for chan in 0..4 {
                        let bits = random_bits(g.depth, rng.next_u64());
                        write_sample(g, buf, x, y, chan, bits);
                    }
                }
            }
            return;
        }
        for y in 0..g.height {
            for x in 0..g.width {
                let levels = self.levels(g, frame, x, y);
                for (chan, level) in levels.iter().enumerate() {
                    write_sample(g, buf, x, y, chan, level_to_bits(g.depth, *level));
                }
            }
        }
    }
}
