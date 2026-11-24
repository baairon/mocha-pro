//! The frame loop: whole frames in, the same whole frames out.

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
