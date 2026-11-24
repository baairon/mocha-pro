//! The raw frame contract of the handoff convention, as far as a pass through stage needs it.
//!
//! This repeats the small part of the frame-io crate that this program needs:
//! the three depths, their bytes per pixel and the size of one frame.
//!
//! | Depth | Layout | Bytes per pixel |
//! |---|---|---|
//! | 8 | packed R, G, B, A, unsigned 8 bit | 4 |
//! | 16 | packed R, G, B, A, unsigned 16 bit little endian | 8 |
//! | 32 | four planes G, B, R, A, 32 bit float little endian | 16 |

/// A frame is at most 4 GiB.
pub const MAX_FRAME_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// The depths of the frame contract.
pub const DEPTHS: [u32; 3] = [8, 16, 32];

/// Bytes per pixel of a depth, or `None` for a depth the contract does not know.
pub fn bytes_per_pixel(depth: u32) -> Option<u64> {
    match depth {
        8 => Some(4),
        16 => Some(8),
        32 => Some(16),
        _ => None,
    }
}

/// Bytes of one whole frame, or `None` when the depth is unknown or the size overflows.
pub fn frame_bytes(width: u32, height: u32, depth: u32) -> Option<u64> {
    let bpp = bytes_per_pixel(depth)?;
    u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(bpp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_per_pixel_table() {
        assert_eq!(bytes_per_pixel(8), Some(4));
        assert_eq!(bytes_per_pixel(16), Some(8));
        assert_eq!(bytes_per_pixel(32), Some(16));
        assert_eq!(bytes_per_pixel(12), None);
        assert_eq!(bytes_per_pixel(0), None);
    }

    #[test]
    fn frame_sizes() {
        assert_eq!(frame_bytes(1, 1, 8), Some(4));
        assert_eq!(frame_bytes(3, 2, 16), Some(48));
        assert_eq!(frame_bytes(5, 7, 32), Some(560));
        assert_eq!(frame_bytes(5, 7, 24), None);
    }

    #[test]
    fn four_gib_boundary() {
        assert_eq!(frame_bytes(32768, 32768, 8), Some(MAX_FRAME_BYTES));
        assert_eq!(
            frame_bytes(32768, 32769, 8),
            Some(MAX_FRAME_BYTES + 4 * 32768)
        );
    }

    #[test]
    fn huge_sizes_do_not_overflow() {
        assert_eq!(frame_bytes(u32::MAX, u32::MAX, 8), None);
        assert_eq!(frame_bytes(u32::MAX, u32::MAX, 32), None);
        assert!(frame_bytes(u32::MAX, 1, 32).is_some());
    }

    #[test]
    fn depth_list_matches_the_table() {
        for d in DEPTHS {
            assert!(bytes_per_pixel(d).is_some());
        }
        assert_eq!(DEPTHS.len(), 3);
    }
}
