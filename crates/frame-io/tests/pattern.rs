mod common;

use common::{geom, stream};
use frame_io::pattern::ramp_level;
use frame_io::{level_to_bits, Depth, Pattern, SplitMix64};

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h = (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[test]
fn splitmix_matches_the_published_sequence_for_seed_zero() {
    let mut r = SplitMix64::new(0);
    assert_eq!(r.next_u64(), 0xe220_a839_7b1d_cdaf);
    assert_eq!(r.next_u64(), 0x6e78_9e6a_a1b9_65f4);
    assert_eq!(r.next_u64(), 0x06c4_5d18_8009_454f);
    assert_eq!(r.next_u64(), 0xf88b_b8a8_724c_81ec);
}

#[test]
fn level_mapping_per_depth() {
    assert_eq!(level_to_bits(Depth::Eight, 0), 0);
    assert_eq!(level_to_bits(Depth::Eight, 65535), 255);
    assert_eq!(level_to_bits(Depth::Eight, 32768), 128);
    assert_eq!(level_to_bits(Depth::Eight, 257), 1);
    assert_eq!(level_to_bits(Depth::Sixteen, 12345), 12345);
    assert_eq!(level_to_bits(Depth::ThirtyTwo, 0), 0.0f32.to_bits());
    assert_eq!(level_to_bits(Depth::ThirtyTwo, 65535), 1.0f32.to_bits());
    assert_eq!(
        level_to_bits(Depth::ThirtyTwo, 32768),
        (32768.0f32 / 65535.0).to_bits()
    );
}

#[test]
fn ramp_levels_run_from_zero_to_full_scale() {
    assert_eq!(ramp_level(0, 1), 0);
    assert_eq!(ramp_level(0, 5), 0);
    assert_eq!(ramp_level(4, 5), 65535);
    assert_eq!(ramp_level(1, 3), 32768);
    assert_eq!(ramp_level(2, 5), 32768);
    assert_eq!(ramp_level(u32::MAX - 1, u32::MAX), 65535);
}

#[test]
fn constant_depth_8_is_packed() {
    let g = geom(2, 1, Depth::Eight);
    let p: Pattern = "constant:65535,0,32768,65535".parse().unwrap();
    assert_eq!(stream(&g, &p, 1), vec![255, 0, 128, 255, 255, 0, 128, 255]);
}

#[test]
fn constant_depth_16_is_little_endian() {
    let g = geom(1, 1, Depth::Sixteen);
    let p: Pattern = "constant:1,258,65535,513".parse().unwrap();
    assert_eq!(stream(&g, &p, 1), vec![1, 0, 2, 1, 255, 255, 1, 2]);
}

#[test]
fn constant_depth_32_is_planar_g_b_r_a() {
    let g = geom(2, 1, Depth::ThirtyTwo);
    // R full, G zero, B zero, A full.
    let p: Pattern = "constant:65535,0,0,65535".parse().unwrap();
    let one = 1.0f32.to_le_bytes();
    let mut want = vec![0u8; 16]; // planes G and B
    for _ in 0..2 {
        want.extend_from_slice(&one); // plane R
    }
    for _ in 0..2 {
        want.extend_from_slice(&one); // plane A
    }
    assert_eq!(stream(&g, &p, 1), want);
}

#[test]
fn hramp_depth_8() {
    let g = geom(3, 2, Depth::Eight);
    let f = stream(&g, &Pattern::HRamp, 1);
    let row = [0, 0, 0, 255, 128, 128, 128, 255, 255, 255, 255, 255];
    assert_eq!(&f[..12], &row);
    assert_eq!(&f[12..], &row);
}

#[test]
fn vramp_depth_16() {
    let g = geom(2, 3, Depth::Sixteen);
    let f = stream(&g, &Pattern::VRamp, 1);
    let px = |v: u16| {
        let mut b = Vec::new();
        for c in [v, v, v, 65535u16] {
            b.extend_from_slice(&c.to_le_bytes());
        }
        b
    };
    let mut want = Vec::new();
    for v in [0u16, 32768, 65535] {
        want.extend(px(v));
        want.extend(px(v));
    }
    assert_eq!(f, want);
}

#[test]
fn a_ramp_over_one_column_is_level_zero() {
    let g = geom(1, 2, Depth::Eight);
    assert_eq!(
        stream(&g, &Pattern::HRamp, 1),
        vec![0, 0, 0, 255, 0, 0, 0, 255]
    );
}

#[test]
fn checker_cells_and_frame_swap() {
    let g = geom(4, 2, Depth::Eight);
    let f0 = stream(&g, &Pattern::Checker(2), 1);
    let white = [255u8, 255, 255, 255];
    let black = [0u8, 0, 0, 255];
    for y in 0..2 {
        for x in 0..4 {
            let o = (y * 4 + x) * 4;
            let want = if x < 2 { white } else { black };
            assert_eq!(&f0[o..o + 4], &want, "x={x} y={y}");
        }
    }
    let both = stream(&g, &Pattern::Checker(2), 2);
    let f1 = &both[32..];
    assert_eq!(&f1[..4], &black);
    assert_eq!(&f1[8..12], &white);
}

#[test]
fn checker_of_one_pixel_alternates_both_ways() {
    let g = geom(2, 2, Depth::Eight);
    let f = stream(&g, &Pattern::Checker(1), 1);
    assert_eq!(f[0], 255);
    assert_eq!(f[4], 0);
    assert_eq!(f[8], 0);
    assert_eq!(f[12], 255);
}

#[test]
fn alpha_ramp_keeps_straight_colour() {
    let g = geom(3, 1, Depth::Sixteen);
    let f = stream(&g, &Pattern::AlphaRamp, 1);
    for (x, a) in [(0usize, 0u16), (1, 32768), (2, 65535)] {
        let o = x * 8;
        assert_eq!(u16::from_le_bytes([f[o], f[o + 1]]), 49152);
        assert_eq!(u16::from_le_bytes([f[o + 2], f[o + 3]]), 32768);
        assert_eq!(u16::from_le_bytes([f[o + 4], f[o + 5]]), 16384);
        assert_eq!(u16::from_le_bytes([f[o + 6], f[o + 7]]), a);
    }
}

#[test]
fn random_field_starts_with_the_seed_zero_draws() {
    let p = Pattern::Random(0);
    assert_eq!(
        stream(&geom(1, 1, Depth::Eight), &p, 1),
        vec![226, 110, 6, 248]
    );
    let f16 = stream(&geom(1, 1, Depth::Sixteen), &p, 1);
    let got: Vec<u16> = f16
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    assert_eq!(got, vec![57888, 28280, 1732, 63627]);
}

#[test]
fn random_depth_32_values_are_floats_below_one() {
    let g = geom(1, 1, Depth::ThirtyTwo);
    let f = stream(&g, &Pattern::Random(0), 1);
    // Planes G, B, R, A hold the draws of channels G, B, R, A: draw 0 is R, 1 is G, 2 is B, 3 is A.
    let word = |i: usize| u32::from_le_bytes([f[i * 4], f[i * 4 + 1], f[i * 4 + 2], f[i * 4 + 3]]);
    assert_eq!(word(2), 0x3f62_20a8); // R from draw 0
    assert_eq!(word(0), 0x3edc_f13c); // G from draw 1
    assert_eq!(word(1), 0x3cd8_8ba0); // B from draw 2
    assert_eq!(word(3), 0x3f78_8bb8); // A from draw 3
}

#[test]
fn random_frames_use_a_state_of_seed_and_frame_index() {
    let g = geom(1, 1, Depth::Eight);
    let two = stream(&g, &Pattern::Random(5), 2);
    assert_eq!(&two[4..], &[159, 24, 84, 40]);
    assert_ne!(&two[..4], &two[4..]);
}

#[test]
fn random_stream_hashes_are_pinned() {
    let g = geom(5, 3, Depth::Eight);
    let two = stream(&g, &Pattern::Random(7), 3);
    assert_eq!(fnv(&two[..60]), 0x8c06_110b_b04a_5a83);
    assert_eq!(fnv(&two[120..]), 0x9711_62ae_3522_24f7);
}

#[test]
fn every_pattern_is_deterministic_and_seeds_differ() {
    for depth in [Depth::Eight, Depth::Sixteen, Depth::ThirtyTwo] {
        let g = geom(6, 4, depth);
        for text in [
            "hramp",
            "vramp",
            "aramp",
            "checker:3",
            "random:9",
            "constant:1,2,3,4",
        ] {
            let p: Pattern = text.parse().unwrap();
            assert_eq!(stream(&g, &p, 3), stream(&g, &p, 3), "{text}");
        }
        assert_ne!(
            stream(&g, &Pattern::Random(1), 1),
            stream(&g, &Pattern::Random(2), 1)
        );
    }
}

#[test]
fn pattern_text_round_trips() {
    for text in [
        "hramp",
        "vramp",
        "aramp",
        "checker:8",
        "random:18446744073709551615",
        "constant:0,1,2,65535",
    ] {
        let p: Pattern = text.parse().unwrap();
        assert_eq!(p.to_string(), text);
    }
}

#[test]
fn bad_pattern_text_is_refused() {
    for bad in [
        "",
        "ramp",
        "hramp:1",
        "checker",
        "checker:0",
        "checker:-1",
        "checker:x",
        "random",
        "random:",
        "random:-3",
        "random:18446744073709551616",
        "constant",
        "constant:1,2,3",
        "constant:1,2,3,4,5",
        "constant:1,2,3,65536",
        "constant:1,2,3,-4",
        "constant: 1,2,3,4",
        "HRAMP",
    ] {
        assert!(bad.parse::<Pattern>().is_err(), "{bad:?}");
    }
}
