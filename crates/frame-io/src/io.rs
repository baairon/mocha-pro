//! Frame readers and writers over any `Read` and `Write`. A stream is whole frames back to back.

use crate::error::FrameError;
use crate::geometry::Geometry;
use std::io::{ErrorKind, Read, Write};

/// Reads whole frames from a byte source.
pub struct FrameReader<R> {
    inner: R,
    geometry: Geometry,
    index: u64,
}

impl<R: Read> FrameReader<R> {
    /// A reader at frame 0.
    pub fn new(inner: R, geometry: Geometry) -> Self {
        FrameReader {
            inner,
            geometry,
            index: 0,
        }
    }

    /// Frames read so far.
    pub fn frames_read(&self) -> u64 {
        self.index
    }

    /// The geometry of the stream.
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    /// Reads the next frame into `buf` (resized to the frame size). Returns `Ok(false)` at a clean
    /// end of stream, and `PartialFrame` when the stream ends inside a frame.
    pub fn read_frame(&mut self, buf: &mut Vec<u8>) -> Result<bool, FrameError> {
        let want = self.geometry.frame_bytes();
        buf.resize(want as usize, 0);
        let mut got = 0usize;
        while got < buf.len() {
            match self.inner.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(FrameError::Io(e)),
            }
        }
        if got == 0 {
            return Ok(false);
        }
        if got < buf.len() {
            return Err(FrameError::PartialFrame {
                frame: self.index,
                got: got as u64,
                want,
            });
        }
        self.index += 1;
        Ok(true)
    }

    /// The source back.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

/// Writes whole frames to a byte sink; nothing but the frame bytes is written.
pub struct FrameWriter<W> {
    inner: W,
    geometry: Geometry,
    index: u64,
}

impl<W: Write> FrameWriter<W> {
    /// A writer at frame 0.
    pub fn new(inner: W, geometry: Geometry) -> Self {
        FrameWriter {
            inner,
            geometry,
            index: 0,
        }
    }

    /// Frames written so far.
    pub fn frames_written(&self) -> u64 {
        self.index
    }

    /// Writes one frame; its length must equal the frame size.
    pub fn write_frame(&mut self, frame: &[u8]) -> Result<(), FrameError> {
        let want = self.geometry.frame_bytes();
        if frame.len() as u64 != want {
            return Err(FrameError::BadFrameLength {
                got: frame.len() as u64,
                want,
            });
        }
        self.inner.write_all(frame)?;
        self.index += 1;
        Ok(())
    }

    /// Flushes the sink.
    pub fn flush(&mut self) -> Result<(), FrameError> {
        self.inner.flush()?;
        Ok(())
    }

    /// The sink back.
    pub fn into_inner(self) -> W {
        self.inner
    }
}
