//! Sample addressing inside one frame. Samples are named in the order R, G, B, A (channel 0 to 3)
//! whatever the layout; depth 8 and 16 store them packed per pixel, depth 32 stores four planes in
//! the order G, B, R, A. A sample is passed around as its raw bit pattern in a `u32`.

use crate::geometry::{Depth, Geometry};

/// Channel letters by channel number.
pub const CHANNEL_NAMES: [char; 4] = ['R', 'G', 'B', 'A'];

/// Plane number of each channel at depth 32 (G, B, R, A are planes 0 to 3).
const PLANE_OF_CHANNEL: [usize; 4] = [2, 0, 1, 3];
/// Channel number of each plane at depth 32.
const CHANNEL_OF_PLANE: [usize; 4] = [1, 2, 0, 3];

/// Byte offset of the sample of channel `chan` at column `x`, row `y`.
pub fn sample_offset(g: &Geometry, x: u32, y: u32, chan: usize) -> usize {
    let pixel = y as usize * g.width as usize + x as usize;
    let bps = g.depth.bytes_per_sample();
    if g.depth.is_planar() {
        (PLANE_OF_CHANNEL[chan] * g.pixels() as usize + pixel) * bps
    } else {
        (pixel * 4 + chan) * bps
    }
}

/// The raw bit pattern of one sample (little endian for 16 and 32 bit).
pub fn read_sample(g: &Geometry, frame: &[u8], x: u32, y: u32, chan: usize) -> u32 {
    let o = sample_offset(g, x, y, chan);
    match g.depth {
        Depth::Eight => u32::from(frame[o]),
        Depth::Sixteen => u32::from(u16::from_le_bytes([frame[o], frame[o + 1]])),
        Depth::ThirtyTwo => {
            u32::from_le_bytes([frame[o], frame[o + 1], frame[o + 2], frame[o + 3]])
        }
    }
}

/// Stores a raw bit pattern; at depth 8 and 16 the bits above the depth are ignored.
pub fn write_sample(g: &Geometry, frame: &mut [u8], x: u32, y: u32, chan: usize, bits: u32) {
    let o = sample_offset(g, x, y, chan);
    match g.depth {
        Depth::Eight => frame[o] = bits as u8,
        Depth::Sixteen => frame[o..o + 2].copy_from_slice(&(bits as u16).to_le_bytes()),
        Depth::ThirtyTwo => frame[o..o + 4].copy_from_slice(&bits.to_le_bytes()),
    }
}

/// Column, row and channel of the sample at storage index `index` (the byte offset divided by the
/// sample size). The inverse of `sample_offset`.
pub fn locate_sample(g: &Geometry, index: u64) -> (u32, u32, usize) {
    let (pixel, chan) = if g.depth.is_planar() {
        let per_plane = g.pixels();
        (
            index % per_plane,
            CHANNEL_OF_PLANE[(index / per_plane) as usize],
        )
    } else {
        (index / 4, (index % 4) as usize)
    };
    let w = u64::from(g.width);
    ((pixel % w) as u32, (pixel / w) as u32, chan)
}
