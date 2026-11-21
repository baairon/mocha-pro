//! Frame file form: the canonical header, round trips, refusals. All values are invented.

use mask_seq::pgm::{parse_header, MAX_SIDE};
use mask_seq::{Depth, Frame, FrameError, PgmError};

#[test]
fn round_trip_8_bit() {
    let samples: Vec<u16> = (0..15).map(|i| i * 17).collect();
    let f = Frame::new(5, 3, Depth::Eight, samples).unwrap();
    let b = f.to_pgm();
    assert!(b.starts_with(b"P5\n5 3\n255\n"));
    assert_eq!(b.len(), 26);
    assert_eq!(Frame::from_pgm(&b), Ok(f));
}

#[test]
fn round_trip_16_bit() {
    let f = Frame::new(2, 1, Depth::Sixteen, vec![258, 65535]).unwrap();
    assert_eq!(f.to_pgm(), b"P5\n2 1\n65535\n\x01\x02\xff\xff");
    assert_eq!(Frame::from_pgm(&f.to_pgm()), Ok(f));
    let samples: Vec<u16> = (0..21u32).map(|i| (i * 3121) as u16).collect();
    assert!(samples.iter().any(|&s| s > 60000));
    let g = Frame::new(7, 3, Depth::Sixteen, samples).unwrap();
    assert_eq!(Frame::from_pgm(&g.to_pgm()), Ok(g));
}

#[test]
fn canonical_bytes_identity() {
    let mut frames = Vec::new();
    for depth in [Depth::Eight, Depth::Sixteen] {
        frames.push(Frame::new(1, 1, depth, vec![0]).unwrap());
        frames.push(Frame::new(1, 1, depth, vec![depth.maxval()]).unwrap());
    }
    frames.push(Frame::new(65535, 1, Depth::Eight, vec![7; 65535]).unwrap());
    frames.push(Frame::new(1, 65535, Depth::Eight, vec![9; 65535]).unwrap());
    for w in [3u32, 5, 7] {
        let s: Vec<u16> = (0..w * 2).map(|i| (i * 13 % 256) as u16).collect();
        frames.push(Frame::new(w, 2, Depth::Eight, s).unwrap());
    }
    for f in frames {
        let b = f.to_pgm();
        let back = Frame::from_pgm(&b).unwrap();
        assert_eq!(back, f);
        assert_eq!(back.to_pgm(), b);
    }
}

#[test]
fn refuses_noncanonical_headers() {
    let raster = [0u8; 16];
    let heads: [&[u8]; 26] = [
        b"P2\n4 2\n255\n",
        b"P6\n4 2\n255\n",
        b"p5\n4 2\n255\n",
        b"P5\n# c\n4 2\n255\n",
        b"P5 4 2\n255\n",
        b"P5\n4  2\n255\n",
        b"P5\n4\t2\n255\n",
        b"P5\r\n4 2\r\n255\r\n",
        b"P5\n4\n2\n255\n",
        b"P5\n4 2 \n255\n",
        b"P5\n04 2\n255\n",
        b"P5\n+4 2\n255\n",
        b"P5\n0 2\n255\n",
        b"P5\n4 0\n255\n",
        b"P5\n123456 1\n255\n",
        b"P5\n65536 1\n255\n",
        b"P5\n4 2\n1\n",
        b"P5\n4 2\n254\n",
        b"P5\n4 2\n256\n",
        b"P5\n4 2\n65534\n",
        b"P5\n4 2\n0255\n",
        b"P5\n4 2\n255",
        b"",
        b"P5\n",
        b"P5\n4 2\n255 \n",
        b"P5\n4 65536\n255\n",
    ];
    for h in heads {
        let mut b = h.to_vec();
        b.extend_from_slice(&raster);
        assert_eq!(Frame::from_pgm(&b), Err(PgmError::Header), "{:?}", h);
        assert_eq!(parse_header(&b).err(), Some(PgmError::Header), "{:?}", h);
    }
    assert_eq!(Frame::from_pgm(b""), Err(PgmError::Header));
}

#[test]
fn refuses_truncated_and_trailing() {
    for depth in [Depth::Eight, Depth::Sixteen] {
        let n = depth.bytes_per_sample();
        let f = Frame::new(3, 2, depth, vec![1; 6]).unwrap();
        let b = f.to_pgm();
        assert_eq!(Frame::from_pgm(&b[..b.len() - 1]), Err(PgmError::Truncated));
        let mut more = b.clone();
        more.push(0);
        assert_eq!(Frame::from_pgm(&more), Err(PgmError::Trailing));
        let head = parse_header(&b).unwrap();
        assert_eq!(head.len, b.len() - 6 * n);
        assert_eq!(head.raster_len(), (6 * n) as u64);
        assert_eq!(Frame::from_pgm(&b[..head.len]), Err(PgmError::Truncated));
    }
    // A 16-bit raster with an odd byte count.
    let mut odd = b"P5\n3 1\n65535\n".to_vec();
    odd.extend_from_slice(&[0; 5]);
    assert_eq!(Frame::from_pgm(&odd), Err(PgmError::Truncated));
}

#[test]
fn frame_new_checks() {
    let one = |w, h| Frame::new(w, h, Depth::Eight, vec![0; 4]);
    assert_eq!(one(0, 1), Err(FrameError::Side));
    assert_eq!(one(MAX_SIDE + 1, 1), Err(FrameError::Side));
    assert_eq!(one(1, 0), Err(FrameError::Side));
    assert_eq!(one(1, MAX_SIDE + 1), Err(FrameError::Side));
    assert_eq!(
        one(2, 3),
        Err(FrameError::SampleCount {
            expected: 6,
            got: 4
        })
    );
    assert_eq!(
        Frame::new(2, 2, Depth::Eight, vec![0, 1, 256, 3]),
        Err(FrameError::SampleRange { index: 2 })
    );
    assert!(Frame::new(2, 1, Depth::Sixteen, vec![65535, 0]).is_ok());
}

#[test]
fn depth_values() {
    assert_eq!(Depth::Eight.maxval(), 255);
    assert_eq!(Depth::Sixteen.maxval(), 65535);
    assert_eq!(Depth::Eight.bits(), 8);
    assert_eq!(Depth::Sixteen.bits(), 16);
    assert_eq!(Depth::Eight.bytes_per_sample(), 1);
    assert_eq!(Depth::Sixteen.bytes_per_sample(), 2);
    assert_eq!(Depth::from_bits(8), Some(Depth::Eight));
    assert_eq!(Depth::from_bits(16), Some(Depth::Sixteen));
    for n in [0, 1, 12, 32] {
        assert_eq!(Depth::from_bits(n), None);
    }
}
