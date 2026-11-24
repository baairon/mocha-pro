mod common;

use common::geom;
use frame_io::{locate_sample, read_sample, sample_offset, write_sample, Depth, CHANNEL_NAMES};

#[test]
fn packed_depth_8_stores_r_g_b_a_per_pixel() {
    let g = geom(3, 2, Depth::Eight);
    assert_eq!(sample_offset(&g, 0, 0, 0), 0);
    assert_eq!(sample_offset(&g, 0, 0, 3), 3);
    assert_eq!(sample_offset(&g, 1, 0, 0), 4);
    assert_eq!(sample_offset(&g, 0, 1, 0), 12);
    assert_eq!(sample_offset(&g, 2, 1, 3), 23);
}

#[test]
fn packed_depth_16_is_little_endian() {
    let g = geom(2, 1, Depth::Sixteen);
    let mut f = vec![0u8; g.frame_bytes() as usize];
    write_sample(&g, &mut f, 1, 0, 1, 0x1234);
    assert_eq!(sample_offset(&g, 1, 0, 1), 10);
    assert_eq!(&f[10..12], &[0x34, 0x12]);
    assert_eq!(read_sample(&g, &f, 1, 0, 1), 0x1234);
}

#[test]
fn depth_32_is_planar_in_the_order_g_b_r_a() {
    let g = geom(3, 2, Depth::ThirtyTwo);
    let plane = 6 * 4;
    assert_eq!(sample_offset(&g, 0, 0, 1), 0);
    assert_eq!(sample_offset(&g, 0, 0, 2), plane);
    assert_eq!(sample_offset(&g, 0, 0, 0), 2 * plane);
    assert_eq!(sample_offset(&g, 0, 0, 3), 3 * plane);
    assert_eq!(sample_offset(&g, 2, 1, 3), 3 * plane + 5 * 4);
}

#[test]
fn depth_32_samples_keep_their_bit_pattern() {
    let g = geom(2, 2, Depth::ThirtyTwo);
    let mut f = vec![0u8; g.frame_bytes() as usize];
    let nan = f32::from_bits(0x7fc0_0001);
    write_sample(&g, &mut f, 1, 1, 0, nan.to_bits());
    write_sample(&g, &mut f, 0, 1, 3, (-0.0f32).to_bits());
    assert_eq!(read_sample(&g, &f, 1, 1, 0), 0x7fc0_0001);
    assert_eq!(read_sample(&g, &f, 0, 1, 3), 0x8000_0000);
    assert_eq!(read_sample(&g, &f, 0, 0, 0), 0);
}

#[test]
fn writing_depth_8_ignores_the_high_bits() {
    let g = geom(1, 1, Depth::Eight);
    let mut f = vec![0u8; 4];
    write_sample(&g, &mut f, 0, 0, 2, 0x1ff);
    assert_eq!(f, vec![0, 0, 0xff, 0]);
}

#[test]
fn locate_sample_inverts_sample_offset() {
    for depth in [Depth::Eight, Depth::Sixteen, Depth::ThirtyTwo] {
        let g = geom(4, 3, depth);
        let bps = depth.bytes_per_sample();
        for y in 0..3 {
            for x in 0..4 {
                for c in 0..4 {
                    let index = (sample_offset(&g, x, y, c) / bps) as u64;
                    assert_eq!(locate_sample(&g, index), (x, y, c), "{depth:?} {x} {y} {c}");
                }
            }
        }
    }
}

#[test]
fn channel_letters() {
    assert_eq!(CHANNEL_NAMES, ['R', 'G', 'B', 'A']);
}
