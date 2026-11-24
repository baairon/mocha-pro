mod common;

use common::{geom, stream};
use frame_io::{Depth, FrameError, FrameReader, FrameWriter, Pattern};
use std::io::{Cursor, Read};

#[test]
fn reads_whole_frames_then_a_clean_end() {
    let g = geom(3, 2, Depth::Eight);
    let data = stream(&g, &Pattern::HRamp, 3);
    assert_eq!(data.len(), 3 * 24);
    let mut r = FrameReader::new(Cursor::new(data.clone()), g);
    let mut buf = Vec::new();
    for k in 0..3usize {
        assert!(r.read_frame(&mut buf).unwrap());
        assert_eq!(buf, data[k * 24..(k + 1) * 24]);
        assert_eq!(r.frames_read(), k as u64 + 1);
    }
    assert!(!r.read_frame(&mut buf).unwrap());
    assert_eq!(r.frames_read(), 3);
}

#[test]
fn an_empty_stream_has_no_frames() {
    let g = geom(3, 2, Depth::Sixteen);
    let mut r = FrameReader::new(Cursor::new(Vec::new()), g);
    assert!(!r.read_frame(&mut Vec::new()).unwrap());
}

#[test]
fn a_stream_ending_inside_a_frame_is_a_partial_frame() {
    let g = geom(3, 2, Depth::Eight);
    let mut data = stream(&g, &Pattern::VRamp, 2);
    data.truncate(24 + 23);
    let mut r = FrameReader::new(Cursor::new(data), g);
    let mut buf = Vec::new();
    assert!(r.read_frame(&mut buf).unwrap());
    match r.read_frame(&mut buf) {
        Err(FrameError::PartialFrame { frame, got, want }) => {
            assert_eq!((frame, got, want), (1, 23, 24));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_single_byte_is_a_partial_frame_at_every_depth() {
    for depth in [Depth::Eight, Depth::Sixteen, Depth::ThirtyTwo] {
        let g = geom(2, 2, depth);
        let mut r = FrameReader::new(Cursor::new(vec![7u8]), g);
        assert!(matches!(
            r.read_frame(&mut Vec::new()),
            Err(FrameError::PartialFrame {
                frame: 0,
                got: 1,
                ..
            })
        ));
    }
}

/// A source that hands out one byte per call and interrupts every other call.
struct Dribble {
    data: Vec<u8>,
    pos: usize,
    flip: bool,
}

impl Read for Dribble {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.flip = !self.flip;
        if self.flip {
            return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
        }
        if self.pos >= self.data.len() || buf.is_empty() {
            return Ok(0);
        }
        buf[0] = self.data[self.pos];
        self.pos += 1;
        Ok(1)
    }
}

#[test]
fn short_reads_and_interrupts_do_not_split_frames() {
    let g = geom(2, 2, Depth::Sixteen);
    let data = stream(&g, &Pattern::Random(3), 2);
    let mut r = FrameReader::new(
        Dribble {
            data: data.clone(),
            pos: 0,
            flip: false,
        },
        g,
    );
    let mut buf = Vec::new();
    assert!(r.read_frame(&mut buf).unwrap());
    assert_eq!(buf, data[..32]);
    assert!(r.read_frame(&mut buf).unwrap());
    assert_eq!(buf, data[32..]);
    assert!(!r.read_frame(&mut buf).unwrap());
}

struct Broken;

impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    }
}

#[test]
fn a_read_failure_is_an_io_error_not_a_partial_frame() {
    let mut r = FrameReader::new(Broken, geom(1, 1, Depth::Eight));
    assert!(matches!(
        r.read_frame(&mut Vec::new()),
        Err(FrameError::Io(_))
    ));
}

#[test]
fn the_writer_writes_the_frame_bytes_and_nothing_else() {
    let g = geom(2, 1, Depth::ThirtyTwo);
    let frame = stream(&g, &Pattern::HRamp, 1);
    let mut w = FrameWriter::new(Vec::new(), g);
    w.write_frame(&frame).unwrap();
    w.write_frame(&frame).unwrap();
    assert_eq!(w.frames_written(), 2);
    w.flush().unwrap();
    let out = w.into_inner();
    assert_eq!(out.len(), 64);
    assert_eq!(&out[..32], &frame[..]);
    assert_eq!(&out[32..], &frame[..]);
}

#[test]
fn the_writer_refuses_a_frame_of_the_wrong_length() {
    let g = geom(2, 2, Depth::Eight);
    let mut w = FrameWriter::new(Vec::new(), g);
    for len in [0usize, 15, 17, 32] {
        assert!(matches!(
            w.write_frame(&vec![0u8; len]),
            Err(FrameError::BadFrameLength { want: 16, .. })
        ));
    }
    assert_eq!(w.frames_written(), 0);
    assert!(w.into_inner().is_empty());
}

#[test]
fn write_then_read_round_trips_at_every_depth() {
    for depth in [Depth::Eight, Depth::Sixteen, Depth::ThirtyTwo] {
        let g = geom(5, 3, depth);
        let data = stream(&g, &Pattern::Random(11), 4);
        let mut w = FrameWriter::new(Vec::new(), g);
        for f in data.chunks(g.frame_bytes() as usize) {
            w.write_frame(f).unwrap();
        }
        let bytes = w.into_inner();
        assert_eq!(bytes, data);
        let mut r = FrameReader::new(Cursor::new(bytes), g);
        let mut got = Vec::new();
        let mut buf = Vec::new();
        while r.read_frame(&mut buf).unwrap() {
            got.extend_from_slice(&buf);
        }
        assert_eq!(got, data);
    }
}
