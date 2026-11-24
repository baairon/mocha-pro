//! `frame-io`: generic tools for the raw frame streams of the handoff convention. The crate reads,
//! writes, generates and compares frames; it computes no image operation and says nothing about any
//! plug-in. Every pattern is synthetic and every number in it is made up.
//!
//! The stream contract, as the convention states it:
//!
//! - depth 8: packed `rgba`, 4 bytes per pixel, unsigned 8 bit samples;
//! - depth 16: packed `rgba64le`, 8 bytes per pixel, unsigned 16 bit little endian samples;
//! - depth 32: planar `gbrapf32le`, 16 bytes per pixel, four whole planes in the order G, B, R, A
//!   (rows top to bottom), 32 bit little endian floats with 1.0 as full scale;
//! - whole frames back to back, no header, no padding, straight alpha, a frame of at most 4 GiB;
//! - a stream that ends inside a frame is a partial frame.
//!
//! The pieces: `geometry` (depth, exact rational rate, the checks), `sample` (sample addressing),
//! `io` (frame reader and writer over `Read` and `Write`), `pattern` (the synthetic generator),
//! `compare` (the exact comparer at tolerance 0) and `cli` (the two tools as functions, so that tests
//! run them in process). The binaries `frame-gen` and `frame-cmp` are thin wrappers over `cli`.
//!
//! The self test named in the documents is a pass through test: a stream generated here, written and
//! read back, compares equal to itself. It tests the tools, not any effect.

pub mod cli;
pub mod compare;
pub mod error;
pub mod geometry;
pub mod io;
pub mod pattern;
pub mod sample;

pub use compare::{compare, CompareError, Comparison, Difference, FrameVerdict, Side};
pub use error::{FrameError, GeometryError};
pub use geometry::{Depth, Geometry, Rate, MAX_FRAME_BYTES};
pub use io::{FrameReader, FrameWriter};
pub use pattern::{level_to_bits, Pattern, PatternError, SplitMix64};
pub use sample::{locate_sample, read_sample, sample_offset, write_sample, CHANNEL_NAMES};
