//! `mask-seq`: a container form, `mask-1`, for a sequence of one-channel mask frames, and a
//! validator for it. The crate stores and checks frames that it is given; it computes no frame.
//!
//! A sequence is one folder holding a manifest, `manifest.txt`, and the frame files it lists. Each
//! frame is a binary graymap (PGM, `P5`) with one canonical header, 8 or 16 bits per sample, whose
//! sample `s` means coverage `s / maxval` with no transfer curve. The manifest is canonical text:
//! the form name, the frame count, the first index, the width, height and depth, the frame naming,
//! and the SHA-256 of every frame file, so that equal samples give equal bytes and equal hashes.
//! Indexes are contiguous from the first one, and the frame of index 123 is the file `frame_000123.pgm`.
//!
//! The pieces are `sha256` (the hash, written from FIPS 180-4), `pgm` (frames and their file form),
//! `manifest` (the manifest text) and `validate` (writing a folder and checking one). The binary
//! `mask-validate` runs `validate` on one argument.

pub mod manifest;
pub mod pgm;
pub mod sha256;
pub mod validate;

pub use manifest::{
    frame_name, Manifest, ManifestError, ManifestErrorKind, FORMAT, MANIFEST_NAME, NAMING,
};
pub use pgm::{Depth, Frame, FrameError, Header, PgmError};
pub use sha256::{digest, from_hex, to_hex, Sha256};
pub use validate::{validate, write_sequence, Invalid, Outcome, Reason, SequenceError, Summary};
