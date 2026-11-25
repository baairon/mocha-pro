//! What the image stages share: the short forms of a parameter definition, one channel of a
//! decoded frame as a buffer of its own, and the samples of a raw frame as bit patterns.
//!
//! A channel buffer is row major with one sample per pixel, which is the form the `soft-edge`
//! crate takes. A bit pattern is the integer of an 8 or 16 bit sample and `f32::to_bits` of a
//! float sample, which is the form the `pixel-format` crate takes.

use crate::pixels;
use crate::registry::{ParamDef, ParamKind};

/// A choice parameter that is not keyed.
pub(crate) const fn choice(
    id: &'static str,
    name: &'static str,
    default: &'static str,
    choices: &'static [&'static str],
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Choice { default, choices },
        keyed: false,
        about,
    }
}

/// A float parameter; `range` is the default, the lowest and the highest value.
pub(crate) const fn float(
    id: &'static str,
    name: &'static str,
    range: (f64, f64, f64),
    keyed: bool,
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Float {
            default: range.0,
            min: range.1,
            max: range.2,
        },
        keyed,
        about,
    }
}

/// A whole number parameter; `range` is the default, the lowest and the highest value.
pub(crate) const fn int(
    id: &'static str,
    name: &'static str,
    range: (i64, i64, i64),
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Int {
            default: range.0,
            min: range.1,
            max: range.2,
        },
        keyed: false,
        about,
    }
}

/// The index of the sample of pixel `pixel` (row major) and channel `channel` in a frame of
/// `pixel_count` pixels, in the layout of `depth`: packed for depth 8 and 16, planar for 32.
pub(crate) fn sample_index(depth: u32, pixel_count: usize, pixel: usize, channel: usize) -> usize {
    match pixels::layout(depth) {
        pixels::Layout::Packed => pixel * pixels::CHANNELS + channel,
        pixels::Layout::Planar => channel * pixel_count + pixel,
    }
}

/// One channel of a decoded frame as a row major buffer with one sample per pixel.
pub(crate) fn take_plane(samples: &[f32], depth: u32, channel: usize) -> Vec<f32> {
    let pixel_count = samples.len() / pixels::CHANNELS;
    (0..pixel_count)
        .map(|pixel| samples[sample_index(depth, pixel_count, pixel, channel)])
        .collect()
}

/// Write a channel buffer back into a decoded frame. The other channels are not touched.
pub(crate) fn put_plane(samples: &mut [f32], depth: u32, channel: usize, plane: &[f32]) {
    let pixel_count = samples.len() / pixels::CHANNELS;
    for (pixel, value) in plane.iter().enumerate().take(pixel_count) {
        samples[sample_index(depth, pixel_count, pixel, channel)] = *value;
    }
}

/// The samples of one raw frame as bit patterns, in stream order.
pub(crate) fn read_bits(frame: &[u8], depth: u32) -> Vec<u32> {
    match depth {
        8 => frame.iter().map(|b| u32::from(*b)).collect(),
        16 => frame
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u32::from(u16::from_le_bytes([c[0], c[1]])))
            .collect(),
        _ => frame
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
    }
}

/// Write bit patterns as one raw frame, replacing the content of `out`. An integer above the
/// highest value of an 8 or 16 bit sample is stored as that highest value.
pub(crate) fn write_bits(bits: &[u32], depth: u32, out: &mut Vec<u8>) {
    out.clear();
    match depth {
        8 => out.extend(bits.iter().map(|v| u8::try_from(*v).unwrap_or(u8::MAX))),
        16 => {
            for v in bits {
                out.extend_from_slice(&u16::try_from(*v).unwrap_or(u16::MAX).to_le_bytes());
            }
        }
        _ => {
            for v in bits {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plane_is_one_channel_in_both_layouts() {
        // two pixels: packed R, G, B, A per pixel
        let packed = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        assert_eq!(take_plane(&packed, 8, 0), [1.0, 5.0]);
        assert_eq!(take_plane(&packed, 16, pixels::ALPHA), [4.0, 8.0]);
        // two pixels: four planes of two samples
        assert_eq!(take_plane(&packed, 32, 0), [1.0, 2.0]);
        assert_eq!(take_plane(&packed, 32, pixels::ALPHA), [7.0, 8.0]);
    }

    #[test]
    fn a_plane_goes_back_where_it_came_from() {
        for depth in [8, 16, 32] {
            let mut samples: Vec<f32> = (0..12).map(|i| i as f32).collect();
            let before = samples.clone();
            let plane = take_plane(&samples, depth, 2);
            put_plane(&mut samples, depth, 2, &plane);
            assert_eq!(samples, before);
            put_plane(&mut samples, depth, 2, &[90.0, 91.0, 92.0]);
            assert_eq!(take_plane(&samples, depth, 2), [90.0, 91.0, 92.0]);
            assert_eq!(
                take_plane(&samples, depth, 1),
                take_plane(&before, depth, 1)
            );
        }
    }

    #[test]
    fn bit_patterns_round_trip_at_every_depth() {
        let frame: Vec<u8> = (0..16u8).map(|i| i.wrapping_mul(37)).collect();
        for depth in [8, 16, 32] {
            let mut back = Vec::new();
            write_bits(&read_bits(&frame, depth), depth, &mut back);
            assert_eq!(back, frame, "depth {depth}");
        }
        assert_eq!(read_bits(&[0x34, 0x12], 16), [0x1234]);
        assert_eq!(read_bits(&1.0f32.to_le_bytes(), 32), [1.0f32.to_bits()]);
    }

    #[test]
    fn an_integer_above_the_sample_type_is_stored_as_its_highest_value() {
        let mut out = Vec::new();
        write_bits(&[256, 7], 8, &mut out);
        assert_eq!(out, [255, 7]);
        write_bits(&[70000], 16, &mut out);
        assert_eq!(out, [0xff, 0xff]);
    }
}
