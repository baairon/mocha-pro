mod common;

use common::{geom, geom_n, stream};
use frame_io::{compare, write_sample, Depth, FrameError, FrameVerdict, Pattern, Side};

fn run(g: &frame_io::Geometry, a: &[u8], b: &[u8]) -> frame_io::Comparison {
    compare(g, a, b, |_| {}).expect("compare")
}

#[test]
fn equal_streams_are_equal() {
    for depth in [Depth::Eight, Depth::Sixteen, Depth::ThirtyTwo] {
        let g = geom(4, 3, depth);
        let s = stream(&g, &Pattern::Random(1), 3);
        let c = run(&g, &s, &s);
        assert!(c.is_equal());
        assert_eq!((c.frames_a, c.frames_b, c.frames_compared), (3, 3, 3));
        assert_eq!(
            (c.frames_equal, c.frames_different, c.samples_different),
            (3, 0, 0)
        );
        assert_eq!(c.first, None);
    }
}

#[test]
fn two_empty_streams_are_equal() {
    let c = run(&geom(2, 2, Depth::Eight), &[], &[]);
    assert!(c.is_equal());
    assert_eq!(c.frames_compared, 0);
}

#[test]
fn the_first_difference_names_frame_pixel_channel_and_values() {
    let g = geom(3, 2, Depth::Eight);
    let a = stream(&g, &Pattern::HRamp, 3);
    let mut b = a.clone();
    let fb = g.frame_bytes() as usize;
    // Frame 1, pixel (2, 1), channel B.
    write_sample(&g, &mut b[fb..2 * fb], 2, 1, 2, 77);
    // Frame 2 also differs, later.
    write_sample(&g, &mut b[2 * fb..], 0, 0, 0, 9);
    let c = run(&g, &a, &b);
    assert!(!c.is_equal());
    let d = c.first.unwrap();
    assert_eq!(
        (d.frame, d.x, d.y, d.channel, d.channel_name()),
        (1, 2, 1, 2, 'B')
    );
    assert_eq!((d.a, d.b), (255, 77));
    assert_eq!(
        (c.frames_equal, c.frames_different, c.samples_different),
        (1, 2, 2)
    );
}

#[test]
fn the_first_difference_is_in_pixel_order_at_depth_32_not_storage_order() {
    let g = geom(2, 2, Depth::ThirtyTwo);
    let a = stream(&g, &Pattern::Constant([0, 0, 0, 0]), 1);
    let mut b = a.clone();
    // Pixel (0, 0) channel A is stored last, pixel (1, 0) channel G is stored first.
    write_sample(&g, &mut b, 0, 0, 3, 1.0f32.to_bits());
    write_sample(&g, &mut b, 1, 0, 1, 1.0f32.to_bits());
    let d = run(&g, &a, &b).first.unwrap();
    assert_eq!((d.x, d.y, d.channel), (0, 0, 3));
}

#[test]
fn channels_of_one_pixel_are_ordered_r_g_b_a() {
    let g = geom(1, 1, Depth::Sixteen);
    let a = vec![0u8; 8];
    let mut b = a.clone();
    write_sample(&g, &mut b, 0, 0, 1, 5);
    write_sample(&g, &mut b, 0, 0, 3, 5);
    assert_eq!(run(&g, &a, &b).first.unwrap().channel, 1);
}

#[test]
fn negative_zero_and_zero_differ_at_depth_32() {
    let g = geom(1, 1, Depth::ThirtyTwo);
    let a = vec![0u8; 16];
    let mut b = a.clone();
    write_sample(&g, &mut b, 0, 0, 0, (-0.0f32).to_bits());
    let c = run(&g, &a, &b);
    assert!(!c.is_equal());
    let d = c.first.unwrap();
    assert_eq!((d.a, d.b, d.channel), (0, 0x8000_0000, 0));
    assert_eq!(c.samples_different, 1);
}

#[test]
fn nan_with_different_payloads_differ_and_the_same_pattern_is_equal() {
    let g = geom(1, 1, Depth::ThirtyTwo);
    let mut a = vec![0u8; 16];
    let mut b = vec![0u8; 16];
    write_sample(&g, &mut a, 0, 0, 0, 0x7fc0_0001);
    write_sample(&g, &mut b, 0, 0, 0, 0x7fc0_0002);
    let c = run(&g, &a, &b);
    assert!(!c.is_equal());
    assert_eq!(c.first.unwrap().a, 0x7fc0_0001);
    write_sample(&g, &mut b, 0, 0, 0, 0x7fc0_0001);
    assert!(run(&g, &a, &b).is_equal());
}

#[test]
fn a_one_unit_difference_is_found_at_tolerance_zero() {
    let g = geom(2, 2, Depth::Sixteen);
    let a = stream(&g, &Pattern::HRamp, 1);
    let mut b = a.clone();
    write_sample(&g, &mut b, 1, 1, 0, 65534);
    let c = run(&g, &a, &b);
    let d = c.first.unwrap();
    assert_eq!((d.a, d.b), (65535, 65534));
}

#[test]
fn counts_of_differing_samples_are_per_sample_not_per_byte() {
    let g = geom(2, 1, Depth::Sixteen);
    let a = vec![0u8; 16];
    let mut b = a.clone();
    // Both bytes of one sample change: one sample.
    b[0] = 1;
    b[1] = 2;
    // One byte of another sample.
    b[6] = 9;
    assert_eq!(run(&g, &a, &b).samples_different, 2);
}

#[test]
fn the_callback_gets_one_verdict_per_common_frame() {
    let g = geom(2, 2, Depth::Eight);
    let a = stream(&g, &Pattern::HRamp, 3);
    let mut b = a.clone();
    b[16] ^= 1;
    b[17] ^= 1;
    let mut seen: Vec<FrameVerdict> = Vec::new();
    compare(&g, &a[..], &b[..], |v| seen.push(*v)).unwrap();
    assert_eq!(
        seen,
        vec![
            FrameVerdict {
                frame: 0,
                equal: true,
                samples_different: 0
            },
            FrameVerdict {
                frame: 1,
                equal: false,
                samples_different: 2
            },
            FrameVerdict {
                frame: 2,
                equal: true,
                samples_different: 0
            },
        ]
    );
}

#[test]
fn a_longer_stream_is_unequal_and_counted() {
    let g = geom(2, 2, Depth::Eight);
    let a = stream(&g, &Pattern::HRamp, 4);
    let b = a[..32].to_vec();
    let c = run(&g, &a, &b);
    assert!(!c.is_equal());
    assert_eq!(
        (c.frames_a, c.frames_b, c.frames_compared, c.frames_equal),
        (4, 2, 2, 2)
    );
    assert_eq!(c.first, None);
    let c = run(&g, &b, &a);
    assert_eq!((c.frames_a, c.frames_b), (2, 4));
    assert!(!c.is_equal());
}

#[test]
fn a_partial_frame_names_its_stream() {
    let g = geom(2, 2, Depth::Eight);
    let a = stream(&g, &Pattern::HRamp, 2);
    let mut b = a.clone();
    b.pop();
    let e = compare(&g, &a[..], &b[..], |_| {}).unwrap_err();
    assert_eq!(e.side, Side::B);
    assert!(matches!(
        e.error,
        FrameError::PartialFrame {
            frame: 1,
            got: 15,
            want: 16
        }
    ));
    let e = compare(&g, &b[..], &a[..], |_| {}).unwrap_err();
    assert_eq!(e.side, Side::A);
}

#[test]
fn a_partial_frame_in_the_longer_tail_is_still_found() {
    let g = geom(2, 2, Depth::Eight);
    let a = stream(&g, &Pattern::HRamp, 3);
    let mut longer = a.clone();
    longer.pop();
    let e = compare(&g, &a[..16], &longer[..], |_| {}).unwrap_err();
    assert_eq!(e.side, Side::B);
}

#[test]
fn a_declared_frame_count_is_enforced_on_both_streams() {
    let g = geom_n(2, 2, Depth::Eight, 3);
    let s3 = stream(&g, &Pattern::HRamp, 3);
    let s2 = s3[..32].to_vec();
    assert!(compare(&g, &s3[..], &s3[..], |_| {}).is_ok());
    let e = compare(&g, &s2[..], &s3[..], |_| {}).unwrap_err();
    assert_eq!(e.side, Side::A);
    assert!(matches!(
        e.error,
        FrameError::CountMismatch {
            expected: 3,
            found: 2
        }
    ));
    let e = compare(&g, &s3[..], &s2[..], |_| {}).unwrap_err();
    assert_eq!(e.side, Side::B);
}
