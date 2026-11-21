//! Writing a sequence folder and checking one. `validate` reads a folder (or a manifest path) and
//! reports the first fault it finds, in a fixed order: the manifest line by line; then each frame
//! in index order (present, readable, header, size, depth, length, hash); then frame-named files
//! that the manifest does not list. Details name a file or a manifest line, never a path, so
//! a report does not depend on where the folder is.

use crate::manifest::{
    frame_name, Manifest, ManifestErrorKind, MANIFEST_NAME, MAX_FRAMES, MAX_INDEX,
};
use crate::pgm::{parse_header, Depth, Frame};
use crate::sha256::digest;
use std::path::Path;

/// The stable reason words of an invalid sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    ManifestMissing,
    ManifestSyntax,
    ManifestFormat,
    ManifestValue,
    FrameMissing,
    FrameHeader,
    FrameSize,
    FrameDepth,
    FrameTruncated,
    FrameTrailing,
    FrameHash,
    FrameExtra,
    FrameRead,
}

impl Reason {
    /// Every reason, in the order of the interface file.
    pub const ALL: [Reason; 13] = [
        Reason::ManifestMissing,
        Reason::ManifestSyntax,
        Reason::ManifestFormat,
        Reason::ManifestValue,
        Reason::FrameMissing,
        Reason::FrameHeader,
        Reason::FrameSize,
        Reason::FrameDepth,
        Reason::FrameTruncated,
        Reason::FrameTrailing,
        Reason::FrameHash,
        Reason::FrameExtra,
        Reason::FrameRead,
    ];

    /// The reason word as printed.
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::ManifestMissing => "manifest_missing",
            Reason::ManifestSyntax => "manifest_syntax",
            Reason::ManifestFormat => "manifest_format",
            Reason::ManifestValue => "manifest_value",
            Reason::FrameMissing => "frame_missing",
            Reason::FrameHeader => "frame_header",
            Reason::FrameSize => "frame_size",
            Reason::FrameDepth => "frame_depth",
            Reason::FrameTruncated => "frame_truncated",
            Reason::FrameTrailing => "frame_trailing",
            Reason::FrameHash => "frame_hash",
            Reason::FrameExtra => "frame_extra",
            Reason::FrameRead => "frame_read",
        }
    }
}

/// The first fault of an invalid sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invalid {
    pub reason: Reason,
    pub detail: String,
}

impl Invalid {
    /// The report line, without a line feed.
    pub fn line(&self) -> String {
        format!("error: {}: {}", self.reason.as_str(), self.detail)
    }
}

/// What a valid sequence holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    pub frames: u32,
    pub first: u32,
    pub width: u32,
    pub height: u32,
    pub depth: Depth,
}

impl Summary {
    /// The success line, without a line feed.
    pub fn line(&self) -> String {
        format!(
            "ok format=mask-1 frames={} first={} width={} height={} depth={}",
            self.frames,
            self.first,
            self.width,
            self.height,
            self.depth.bits()
        )
    }
}

/// The result of `validate`.
#[derive(Debug)]
pub enum Outcome {
    Valid(Summary),
    Invalid(Invalid),
    /// The argument or the manifest file could not be read at all.
    Unreadable(std::io::Error),
}

fn invalid(reason: Reason, detail: &str) -> Outcome {
    Outcome::Invalid(Invalid {
        reason,
        detail: detail.to_string(),
    })
}

/// The index of a name of the exact form `frame_` + 6 digits + `.pgm`.
fn index_of(name: &str) -> Option<u32> {
    let digits = name.strip_prefix("frame_")?.strip_suffix(".pgm")?;
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Checks the sequence named by `arg`: a folder that holds `manifest.txt`, or a manifest file.
pub fn validate(arg: &Path) -> Outcome {
    let meta = match std::fs::metadata(arg) {
        Ok(m) => m,
        Err(e) => return Outcome::Unreadable(e),
    };
    let (dir, bytes) = if meta.is_dir() {
        match std::fs::read(arg.join(MANIFEST_NAME)) {
            Ok(b) => (arg.to_path_buf(), b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return invalid(Reason::ManifestMissing, MANIFEST_NAME)
            }
            Err(e) => return Outcome::Unreadable(e),
        }
    } else {
        let dir = arg
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf();
        match std::fs::read(arg) {
            Ok(b) => (dir, b),
            Err(e) => return Outcome::Unreadable(e),
        }
    };
    let manifest = match Manifest::parse(&bytes) {
        Ok(m) => m,
        Err(e) => {
            let reason = match e.kind {
                ManifestErrorKind::Syntax => Reason::ManifestSyntax,
                ManifestErrorKind::Format => Reason::ManifestFormat,
                ManifestErrorKind::Value => Reason::ManifestValue,
            };
            return invalid(reason, &format!("line {}", e.line));
        }
    };
    for (k, want) in manifest.hashes().iter().enumerate() {
        let name = frame_name(manifest.first() + k as u32);
        let path = dir.join(&name);
        match std::fs::metadata(&path) {
            Ok(m) if m.is_file() => {}
            _ => return invalid(Reason::FrameMissing, &name),
        }
        let file = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => return invalid(Reason::FrameRead, &name),
        };
        let header = match parse_header(&file) {
            Ok(h) => h,
            Err(_) => return invalid(Reason::FrameHeader, &name),
        };
        if header.width != manifest.width() || header.height != manifest.height() {
            return invalid(Reason::FrameSize, &name);
        }
        if header.depth != manifest.depth() {
            return invalid(Reason::FrameDepth, &name);
        }
        let want_len = header.len as u64 + header.raster_len();
        if (file.len() as u64) < want_len {
            return invalid(Reason::FrameTruncated, &name);
        }
        if file.len() as u64 > want_len {
            return invalid(Reason::FrameTrailing, &name);
        }
        if digest(&file) != *want {
            return invalid(Reason::FrameHash, &name);
        }
    }
    let listing = match std::fs::read_dir(&dir) {
        Ok(l) => l,
        Err(_) => return invalid(Reason::FrameRead, "."),
    };
    let first = manifest.first();
    let last = first + manifest.frames() - 1;
    let mut extra: Option<String> = None;
    for entry in listing {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => return invalid(Reason::FrameRead, "."),
        };
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };
        if !(name.starts_with("frame_") && name.ends_with(".pgm")) {
            continue;
        }
        if in_sequence(&name, first, last) {
            continue;
        }
        if extra.as_ref().is_none_or(|e| name < *e) {
            extra = Some(name);
        }
    }
    if let Some(name) = extra {
        return invalid(Reason::FrameExtra, &name);
    }
    Outcome::Valid(Summary {
        frames: manifest.frames(),
        first,
        width: manifest.width(),
        height: manifest.height(),
        depth: manifest.depth(),
    })
}

/// True when `name` is the file name of a frame index from `first` to `last`.
fn in_sequence(name: &str, first: u32, last: u32) -> bool {
    index_of(name).is_some_and(|i| i >= first && i <= last)
}

/// Why a sequence could not be written.
#[derive(Debug)]
pub enum SequenceError {
    /// No frames were given.
    Empty,
    /// A frame whose width, height or depth differs from frame 0.
    Unequal {
        index: usize,
    },
    /// The indexes would pass 999999, or there are more than 1000000 frames.
    Range,
    Io(std::io::Error),
}

impl std::fmt::Display for SequenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SequenceError::Empty => write!(f, "no frames"),
            SequenceError::Unequal { index } => write!(f, "frame {index} differs from frame 0"),
            SequenceError::Range => write!(f, "frame indexes out of range"),
            SequenceError::Io(e) => write!(f, "write failed: {e}"),
        }
    }
}

impl std::error::Error for SequenceError {}

/// Writes `frames` as frame files with indexes from `first`, then the manifest last, into `dir`
/// (created if absent). Files of the same names are overwritten; no other file is touched.
pub fn write_sequence(dir: &Path, first: u32, frames: &[Frame]) -> Result<Manifest, SequenceError> {
    let head = frames.first().ok_or(SequenceError::Empty)?;
    if let Some(index) = frames.iter().position(|f| {
        f.width() != head.width() || f.height() != head.height() || f.depth() != head.depth()
    }) {
        return Err(SequenceError::Unequal { index });
    }
    if frames.len() > MAX_FRAMES as usize
        || first > MAX_INDEX
        || first as u64 + frames.len() as u64 - 1 > MAX_INDEX as u64
    {
        return Err(SequenceError::Range);
    }
    let files: Vec<Vec<u8>> = frames.iter().map(Frame::to_pgm).collect();
    let hashes = files.iter().map(|f| digest(f)).collect();
    let manifest = Manifest::new(first, head.width(), head.height(), head.depth(), hashes)
        .map_err(|_| SequenceError::Range)?;
    std::fs::create_dir_all(dir).map_err(SequenceError::Io)?;
    for (k, bytes) in files.iter().enumerate() {
        std::fs::write(dir.join(frame_name(first + k as u32)), bytes).map_err(SequenceError::Io)?;
    }
    std::fs::write(dir.join(MANIFEST_NAME), manifest.to_bytes()).map_err(SequenceError::Io)?;
    Ok(manifest)
}
