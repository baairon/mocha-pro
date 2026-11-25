//! The frame loops: whole frames in, whole frames out. [`copy_frames`] writes each frame as it was
//! read; [`map_frames`] hands each frame to a stage and writes what the stage makes of it.

use std::io::{self, Read, Write};

use crate::error::{Code, StageError};

/// Copy whole frames of `frame_bytes` from `input` to `output` and return how many were copied.
///
/// A frame is written only once it has been read whole, so a stream that ends inside a frame
/// leaves every earlier frame written and the partial one not written; that is `partial_frame`
/// (exit 4). A write to a closed pipe is `broken_pipe` (exit 4), any other write failure is
/// `stage_failed` (exit 1) and a read failure is `bad_input`. `expect` is the `--frames` count:
/// a different count is `bad_input`.
pub fn copy_frames<R: Read + ?Sized, W: Write + ?Sized>(
    input: &mut R,
    output: &mut W,
    frame_bytes: usize,
    expect: Option<u64>,
) -> Result<u64, StageError> {
    let mut buf = vec![0u8; frame_bytes];
    let mut count: u64 = 0;
    let looped = (|| -> Result<(), StageError> {
        loop {
            let mut got = 0usize;
            while got < buf.len() {
                match input.read(&mut buf[got..]) {
                    Ok(0) => break,
                    Ok(n) => got += n,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(e) => {
                        return Err(StageError::bad_input(format!(
                            "frame {count}: cannot read the input: {e}"
                        )))
                    }
                }
            }
            if got == 0 {
                return Ok(());
            }
            if got < buf.len() {
                return Err(StageError::new(
                    Code::PartialFrame,
                    format!(
                        "frame {count}: the input ends inside a frame ({got} of {} bytes) after {count} whole frames",
                        buf.len()
                    ),
                ));
            }
            output.write_all(&buf).map_err(|e| write_error(count, &e))?;
            count += 1;
        }
    })();
    let flushed = output.flush();
    looped?;
    flushed.map_err(|e| write_error(count, &e))?;
    if let Some(want) = expect {
        if want != count {
            return Err(StageError::bad_input(format!(
                "the input holds {count} frames, but --frames says {want}"
            )));
        }
    }
    Ok(count)
}

fn write_error(index: u64, e: &io::Error) -> StageError {
    let code = if e.kind() == io::ErrorKind::BrokenPipe {
        Code::BrokenPipe
    } else {
        Code::StageFailed
    };
    StageError::new(code, format!("frame {index}: cannot write the output: {e}"))
}

/// Read whole frames of `in_bytes` from `input`, hand each one with its index to `each`, and
/// write the frame that `each` leaves in its output buffer. Returns how many frames were written.
///
/// The rules are those of [`copy_frames`]: a frame is processed only once it has been read whole,
/// a stream that ends inside a frame is `partial_frame` with every earlier frame written, a write
/// to a closed pipe is `broken_pipe`, any other write failure is `stage_failed`, a read failure
/// is `bad_input`, and an input count other than `expect` is `bad_input`. An error of `each` ends
/// the run with that error. `out_bytes` is the size of an output frame: a stage that leaves
/// another size in the buffer is `stage_failed`, so a frame of the wrong size is never written.
pub fn map_frames<R, W, F>(
    input: &mut R,
    output: &mut W,
    in_bytes: usize,
    out_bytes: usize,
    expect: Option<u64>,
    mut each: F,
) -> Result<u64, StageError>
where
    R: Read + ?Sized,
    W: Write + ?Sized,
    F: FnMut(u64, &[u8], &mut Vec<u8>) -> Result<(), StageError>,
{
    let mut buf = vec![0u8; in_bytes];
    let mut made: Vec<u8> = Vec::with_capacity(out_bytes);
    let mut count: u64 = 0;
    let looped = (|| -> Result<(), StageError> {
        loop {
            let mut got = 0usize;
            while got < buf.len() {
                match input.read(&mut buf[got..]) {
                    Ok(0) => break,
                    Ok(n) => got += n,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(e) => {
                        return Err(StageError::bad_input(format!(
                            "frame {count}: cannot read the input: {e}"
                        )))
                    }
                }
            }
            if got == 0 {
                return Ok(());
            }
            if got < buf.len() {
                return Err(StageError::new(
                    Code::PartialFrame,
                    format!(
                        "frame {count}: the input ends inside a frame ({got} of {} bytes) after {count} whole frames",
                        buf.len()
                    ),
                ));
            }
            each(count, &buf, &mut made)?;
            if made.len() != out_bytes {
                return Err(StageError::new(
                    Code::StageFailed,
                    format!(
                        "frame {count}: the stage made {} bytes for a frame of {out_bytes}",
                        made.len()
                    ),
                ));
            }
            output
                .write_all(&made)
                .map_err(|e| write_error(count, &e))?;
            count += 1;
        }
    })();
    let flushed = output.flush();
    looped?;
    flushed.map_err(|e| write_error(count, &e))?;
    if let Some(want) = expect {
        if want != count {
            return Err(StageError::bad_input(format!(
                "the input holds {count} frames, but --frames says {want}"
            )));
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader that gives at most `step` bytes per call.
    struct Slow<'a> {
        data: &'a [u8],
        step: usize,
        interrupt_first: bool,
    }

    impl Read for Slow<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.interrupt_first {
                self.interrupt_first = false;
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            let n = self.step.min(buf.len()).min(self.data.len());
            buf[..n].copy_from_slice(&self.data[..n]);
            self.data = &self.data[n..];
            Ok(n)
        }
    }

    struct Failing(io::ErrorKind);

    impl Write for Failing {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct FlushFails;

    impl Write for FlushFails {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
    }

    struct ReadFails;

    impl Read for ReadFails {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        }
    }

    fn pattern(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 7 + 3) as u8).collect()
    }

    #[test]
    fn copies_whole_frames() {
        let data = pattern(24);
        let mut out = Vec::new();
        let n = copy_frames(&mut &data[..], &mut out, 8, None).unwrap();
        assert_eq!((n, out), (3, data));
    }

    #[test]
    fn empty_input_is_zero_frames() {
        let mut out = Vec::new();
        assert_eq!(copy_frames(&mut &b""[..], &mut out, 8, None).unwrap(), 0);
        assert!(out.is_empty());
    }

    #[test]
    fn short_reads_still_make_whole_frames() {
        let data = pattern(40);
        let mut out = Vec::new();
        let mut r = Slow {
            data: &data,
            step: 3,
            interrupt_first: true,
        };
        assert_eq!(copy_frames(&mut r, &mut out, 8, None).unwrap(), 5);
        assert_eq!(out, data);
    }

    #[test]
    fn partial_frame_keeps_earlier_frames_and_drops_the_partial() {
        let data = pattern(20);
        let mut out = Vec::new();
        let e = copy_frames(&mut &data[..], &mut out, 8, None).unwrap_err();
        assert_eq!(e.code, Code::PartialFrame);
        assert_eq!(e.exit(), 4);
        assert_eq!(out, data[..16]);
        assert!(e.message.contains("4 of 8"), "{}", e.message);
    }

    #[test]
    fn a_single_byte_is_a_partial_frame() {
        let mut out = Vec::new();
        let e = copy_frames(&mut &b"x"[..], &mut out, 4, None).unwrap_err();
        assert_eq!(e.code, Code::PartialFrame);
        assert!(out.is_empty());
    }

    #[test]
    fn frame_count_expectation() {
        let data = pattern(16);
        assert_eq!(
            copy_frames(&mut &data[..], &mut Vec::new(), 8, Some(2)).unwrap(),
            2
        );
        let e = copy_frames(&mut &data[..], &mut Vec::new(), 8, Some(3)).unwrap_err();
        assert_eq!(e.code, Code::BadInput);
        let e = copy_frames(&mut &data[..], &mut Vec::new(), 8, Some(1)).unwrap_err();
        assert_eq!(e.code, Code::BadInput);
        assert_eq!(
            copy_frames(&mut &b""[..], &mut Vec::new(), 8, Some(0)).unwrap(),
            0
        );
    }

    #[test]
    fn closed_pipe_is_broken_pipe() {
        let data = pattern(16);
        let e = copy_frames(
            &mut &data[..],
            &mut Failing(io::ErrorKind::BrokenPipe),
            8,
            None,
        )
        .unwrap_err();
        assert_eq!((e.code, e.exit()), (Code::BrokenPipe, 4));
    }

    #[test]
    fn other_write_failure_is_stage_failed() {
        let data = pattern(16);
        let e = copy_frames(
            &mut &data[..],
            &mut Failing(io::ErrorKind::PermissionDenied),
            8,
            None,
        )
        .unwrap_err();
        assert_eq!((e.code, e.exit()), (Code::StageFailed, 1));
    }

    #[test]
    fn flush_failure_is_reported() {
        let data = pattern(16);
        let e = copy_frames(&mut &data[..], &mut FlushFails, 8, None).unwrap_err();
        assert_eq!(e.code, Code::BrokenPipe);
    }

    #[test]
    fn read_failure_is_bad_input() {
        let e = copy_frames(&mut ReadFails, &mut Vec::new(), 8, None).unwrap_err();
        assert_eq!((e.code, e.exit()), (Code::BadInput, 4));
    }

    #[test]
    fn every_byte_value_survives() {
        let data: Vec<u8> = (0..=255u8).cycle().take(1024).collect();
        let mut out = Vec::new();
        copy_frames(&mut &data[..], &mut out, 256, None).unwrap();
        assert_eq!(out, data);
    }
}

#[cfg(test)]
mod map_tests {
    use super::*;

    fn double(_: u64, frame: &[u8], out: &mut Vec<u8>) -> Result<(), StageError> {
        out.clear();
        out.extend_from_slice(frame);
        out.extend_from_slice(frame);
        Ok(())
    }

    #[test]
    fn each_frame_is_mapped_with_its_index() {
        let data: Vec<u8> = (0..12).collect();
        let mut out = Vec::new();
        let mut seen = Vec::new();
        let n = map_frames(&mut &data[..], &mut out, 4, 4, None, |i, frame, made| {
            seen.push(i);
            made.clear();
            made.extend(frame.iter().map(|b| b + i as u8));
            Ok(())
        })
        .unwrap();
        assert_eq!((n, seen), (3, vec![0, 1, 2]));
        assert_eq!(out, [0, 1, 2, 3, 5, 6, 7, 8, 10, 11, 12, 13]);
    }

    #[test]
    fn the_output_frame_may_have_another_size() {
        let data: Vec<u8> = (0..8).collect();
        let mut out = Vec::new();
        assert_eq!(
            map_frames(&mut &data[..], &mut out, 4, 8, Some(2), double).unwrap(),
            2
        );
        assert_eq!(out.len(), 16);
    }

    #[test]
    fn a_frame_of_the_wrong_size_is_never_written() {
        let data: Vec<u8> = (0..8).collect();
        let mut out = Vec::new();
        let e = map_frames(&mut &data[..], &mut out, 4, 4, None, double).unwrap_err();
        assert_eq!((e.code, e.exit()), (Code::StageFailed, 1));
        assert!(out.is_empty());
    }

    #[test]
    fn partial_input_keeps_earlier_frames_and_is_exit_four() {
        let data: Vec<u8> = (0..10).collect();
        let mut out = Vec::new();
        let e = map_frames(&mut &data[..], &mut out, 4, 8, None, double).unwrap_err();
        assert_eq!((e.code, e.exit()), (Code::PartialFrame, 4));
        assert_eq!(out.len(), 16);
        assert!(e.message.contains("2 of 4"), "{}", e.message);
    }

    #[test]
    fn an_error_of_the_stage_ends_the_run() {
        let data: Vec<u8> = (0..8).collect();
        let mut out = Vec::new();
        let e = map_frames(&mut &data[..], &mut out, 4, 4, None, |i, frame, made| {
            if i == 1 {
                return Err(StageError::unsupported("frame 1: refused"));
            }
            made.clear();
            made.extend_from_slice(frame);
            Ok(())
        })
        .unwrap_err();
        assert_eq!(e.code, Code::Unsupported);
        assert_eq!(out, [0, 1, 2, 3]);
    }

    #[test]
    fn empty_input_and_the_frame_count() {
        let mut out = Vec::new();
        assert_eq!(
            map_frames(&mut &b""[..], &mut out, 4, 8, Some(0), double).unwrap(),
            0
        );
        let data: Vec<u8> = (0..8).collect();
        let e = map_frames(&mut &data[..], &mut out, 4, 8, Some(3), double).unwrap_err();
        assert_eq!(e.code, Code::BadInput);
    }

    #[test]
    fn a_closed_reader_is_broken_pipe() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::from(io::ErrorKind::BrokenPipe))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let data: Vec<u8> = (0..8).collect();
        let e = map_frames(&mut &data[..], &mut Closed, 4, 8, None, double).unwrap_err();
        assert_eq!((e.code, e.exit()), (Code::BrokenPipe, 4));
    }
}
