//! Frames as `f32` samples, for the stages that compute in floating point.
//!
//! The rule is fixed and is part of every stage that uses it:
//!
//! - depth 8 and 16: an integer sample `v` becomes `v / max` (`max` is 255 or 65535), computed in
//!   f64 and rounded once to f32. Back, a sample `x` becomes `floor(clamp(x, 0, 1) * max + 0.5)`
//!   in f64; a sample that is not a number becomes 0. The buffer is packed R, G, B, A.
//! - depth 32: the samples are the stream's own little endian floats, unchanged in both
//!   directions and never clamped. The buffer is the four planes G, B, R, A.
//!
//! In both layouts the alpha channel has index 3.

/// How the samples of a decoded frame are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Interleaved R, G, B, A: index `(y * width + x) * 4 + c`.
    Packed,
    /// Four planes G, B, R, A: index `c * width * height + y * width + x`.
    Planar,
}

/// The index of the alpha channel in both layouts.
pub const ALPHA: usize = 3;
/// The number of channels of a frame.
pub const CHANNELS: usize = 4;

/// The layout [`decode`] gives for a depth.
pub fn layout(depth: u32) -> Layout {
    if depth == 32 {
        Layout::Planar
    } else {
        Layout::Packed
    }
}

/// Decode one raw frame of depth 8, 16 or 32 into `f32` samples.
pub fn decode(frame: &[u8], depth: u32) -> Vec<f32> {
    match depth {
        8 => frame
            .iter()
            .map(|b| (f64::from(*b) / 255.0) as f32)
            .collect(),
        16 => frame
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| (f64::from(u16::from_le_bytes([c[0], c[1]])) / 65535.0) as f32)
            .collect(),
        _ => frame
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
    }
}

fn to_int(x: f32, max: f64) -> f64 {
    let x = f64::from(x);
    if x.is_nan() {
        return 0.0;
    }
    (x.clamp(0.0, 1.0) * max + 0.5).floor()
}

/// Encode `f32` samples as one raw frame of depth 8, 16 or 32, replacing the content of `out`.
pub fn encode(samples: &[f32], depth: u32, out: &mut Vec<u8>) {
    out.clear();
    match depth {
        8 => out.extend(samples.iter().map(|x| to_int(*x, 255.0) as u8)),
        16 => {
            for x in samples {
                out.extend_from_slice(&(to_int(*x, 65535.0) as u16).to_le_bytes());
            }
        }
        _ => {
            for x in samples {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_eight_bit_value_survives_the_round_trip() {
        let frame: Vec<u8> = (0..=255u8).collect();
        let mut back = Vec::new();
        encode(&decode(&frame, 8), 8, &mut back);
        assert_eq!(back, frame);
    }

    #[test]
    fn every_sixteen_bit_value_survives_the_round_trip() {
        let frame: Vec<u8> = (0..=u16::MAX).flat_map(|v| v.to_le_bytes()).collect();
        let mut back = Vec::new();
        encode(&decode(&frame, 16), 16, &mut back);
        assert_eq!(back, frame);
    }

    #[test]
    fn float_frames_pass_unchanged() {
        let values = [
            0.0f32,
            -0.0,
            1.0,
            2.5,
            -7.25,
            f32::MIN_POSITIVE,
            f32::INFINITY,
        ];
        let frame: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut back = Vec::new();
        encode(&decode(&frame, 32), 32, &mut back);
        assert_eq!(back, frame);
    }

    #[test]
    fn full_scale_is_one() {
        assert_eq!(decode(&[0, 255], 8), [0.0, 1.0]);
        assert_eq!(decode(&[0xff, 0xff, 0, 0], 16), [1.0, 0.0]);
    }

    #[test]
    fn integers_clamp_and_round_half_up() {
        let mut out = Vec::new();
        encode(&[-1.0, 2.0, f32::NAN, 0.5, 0.25], 8, &mut out);
        assert_eq!(out, [0, 255, 0, 128, 64]);
        encode(&[1.5, 0.5], 16, &mut out);
        assert_eq!(out, [0xff, 0xff, 0x00, 0x80]);
    }

    #[test]
    fn layouts_by_depth() {
        assert_eq!(layout(8), Layout::Packed);
        assert_eq!(layout(16), Layout::Packed);
        assert_eq!(layout(32), Layout::Planar);
    }
}
