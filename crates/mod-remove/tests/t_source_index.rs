#![allow(clippy::field_reassign_with_default)]
mod support;
use mod_remove::source_index::*;
use mod_remove::*;
use support::*;

fn two_src() -> (Matte, Vec<Source>) {
    let w = Win::new(0, 0, 8, 8);
    let fill = rect_matte(w, 2, 2, 5, 5);
    let a = src_of(const_img(w, [1.0, 0.0, 0.0, 1.0]));
    let b = src_of(const_img(w, [0.0, 1.0, 0.0, 1.0]));
    (fill, vec![a, b])
}

#[test]
fn sidx_single_source() {
    let (fill, s) = two_src();
    let mut p = SourceIndex::new(fill.win, 1);
    index_map_update(&mut p, &fill, &s[..1], 0, &Camera::identity()).unwrap();
    assert_eq!(p.get(0, 3, 3), 0);
    assert_eq!(p.get(0, 0, 0), NO_SOURCE);
}
#[test]
fn sidx_zero_cover_mask_gives_none() {
    let (fill, mut s) = two_src();
    s[0].cover.set(3, 3, 0);
    let mut p = SourceIndex::new(fill.win, 1);
    index_map_update(&mut p, &fill, &s[..1], 0, &Camera::identity()).unwrap();
    assert_eq!(p.get(0, 3, 3), NO_SOURCE);
    assert_eq!(p.get(0, 3, 4), 0);
}
#[test]
fn sidx_outside_from_window_gives_none() {
    let (fill, mut s) = two_src();
    s[0].h = translate(4.0, 0.0);
    let mut p = SourceIndex::new(fill.win, 1);
    index_map_update(&mut p, &fill, &s[..1], 0, &Camera::identity()).unwrap();
    assert_eq!(p.get(0, 3, 3), 0); // 3+4 = 7 inside
    assert_eq!(p.get(0, 4, 3), NO_SOURCE); // 8 is outside
}
#[test]
fn sidx_channels_fill_in_source_order() {
    let (fill, s) = two_src();
    let mut p = SourceIndex::new(fill.win, 2);
    index_map_update(&mut p, &fill, &s, 0, &Camera::identity()).unwrap();
    assert_eq!((p.get(0, 3, 3), p.get(1, 3, 3)), (0, 1));
}
#[test]
fn sidx_one_channel_first_source_wins() {
    let (fill, s) = two_src();
    let mut p = SourceIndex::new(fill.win, 1);
    index_map_update(&mut p, &fill, &s, 0, &Camera::identity()).unwrap();
    assert_eq!(p.get(0, 3, 3), 0);
}
#[test]
fn sidx_base_offsets_global_index() {
    let (fill, s) = two_src();
    let mut p = SourceIndex::new(fill.win, 1);
    index_map_update(&mut p, &fill, &s[1..], 10, &Camera::identity()).unwrap();
    assert_eq!(p.get(0, 3, 3), 10);
}
#[test]
fn sidx_index_beyond_16_bits_invalid() {
    let (fill, s) = two_src();
    let mut p = SourceIndex::new(fill.win, 1);
    let e = index_map_update(&mut p, &fill, &s, 65534, &Camera::identity());
    assert!(matches!(e, Err(RemoveError::Invalid(_))));
}
#[test]
fn batch_size_is_ten() {
    assert_eq!(BATCH, 10);
    assert_eq!(NO_SOURCE, 65535);
}
#[test]
fn counts_to_fill_and_per_source() {
    let (fill, mut s) = two_src();
    s[0].cover = rect_matte(Win::new(0, 0, 8, 8), 2, 2, 3, 5);
    let mut p = SourceIndex::new(fill.win, 1);
    index_map_update(&mut p, &fill, &s[..1], 0, &Camera::identity()).unwrap();
    assert_eq!(count_pending(&p, &fill), 8); // x = 4,5 over 4 rows
    assert_eq!(count_per_source(&p, &fill, 2), vec![8, 0]);
}
#[test]
fn compact_index_running_numbers() {
    assert_eq!(compact_index(&[3, 0, 2, 0, 1]), vec![0, -1, 1, -1, 2]);
}
#[test]
fn compact_map_rewrites() {
    let w = Win::new(0, 0, 3, 1);
    let mut p = sidx_with(w, 1, &[(0, 0, 0, 0), (1, 0, 0, 2), (2, 0, 0, 1)]);
    compact_map(&mut p, &[0, -1, 1]);
    assert_eq!(
        (p.get(0, 0, 0), p.get(0, 1, 0), p.get(0, 2, 0)),
        (0, 1, NO_SOURCE)
    );
}
#[test]
fn source_window_single_pixel() {
    let w = Win::new(0, 0, 10, 10);
    let p = sidx_with(w, 1, &[(5, 5, 0, 0)]);
    let s = src_of(const_img(w, [0.0; 4]));
    let win = source_region(&p, 0, 1, &Camera::identity(), &s)
        .unwrap()
        .unwrap();
    assert_eq!(win, Win::new(3, 3, 5, 5));
}
#[test]
fn source_fill_window_translation_and_union() {
    let w = Win::new(0, 0, 10, 10);
    let p = sidx_with(w, 1, &[(5, 5, 0, 0), (7, 5, 0, 0)]);
    let mut s = src_of(const_img(w, [0.0; 4]));
    s.h = translate(1.0, 0.0);
    let win = source_region(&p, 0, 2, &Camera::identity(), &s)
        .unwrap()
        .unwrap();
    // x: pixel 5 gives 4..8, pixel 7 gives 6..10 (after +1), union 4..10 grown 1: 3..11; y: 3..7 grown 1: 2..8
    assert_eq!(win, Win::new(3, 2, 9, 7));
    assert_eq!(
        source_region(&p, 3, 2, &Camera::identity(), &s).unwrap(),
        None
    );
}
#[test]
fn index_and_sample_windows() {
    let w = Win::new(0, 0, 10, 10);
    let p = sidx_with(w, 1, &[(2, 3, 0, 0), (6, 4, 0, 0)]);
    assert_eq!(index_window(&p, 0), Some(Win::new(2, 3, 5, 2)));
    assert_eq!(sampling_window(&p, 0), Some(Win::new(1, 2, 7, 4)));
    assert_eq!(index_window(&p, 1), None);
}
#[test]
fn distortion_map_window_unspecified() {
    let p = SourceIndex::new(Win::new(0, 0, 2, 2), 1);
    assert!(matches!(
        source_frame_window(&p, 0),
        Err(RemoveError::Unspecified(_))
    ));
}
#[test]
fn twelve_sources_batches_and_reduction() {
    let w = Win::new(0, 0, 8, 8);
    let fill = rect_matte(w, 2, 2, 5, 5);
    let srcs: Vec<Source> = (0..12)
        .map(|i| src_of(const_img(w, [i as f32 / 16.0, 0.5, 0.25, 1.0])))
        .collect();
    let r = FillConfig::default();
    let out = run(&fill, &srcs, false, &r);
    // one channel: source 0 takes every pixel, the other eleven are reduced away
    assert_eq!(bits(out.image.at(3, 3)), bits([0.0, 0.5, 0.25, 1.0]));
    let mut r2 = FillConfig::default();
    r2.index_channels = 2;
    let out_b = run(&fill, &srcs, false, &r2);
    // two channels: the first two sources contribute and the weight sum is two
    assert_eq!(out_b.image.at(3, 3)[0], (0.0f32 + 1.0 / 16.0) / 2.0);
    assert_eq!(out_b.weights[out_b.image.win.idx(3, 3)], 2.0);
}
