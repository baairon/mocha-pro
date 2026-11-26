#![allow(clippy::field_reassign_with_default)]
mod support;
use mod_remove::mix::*;
use mod_remove::source_index::*;
use mod_remove::*;
use support::*;

fn row_sidx() -> SourceIndex {
    // x 0..7 one row: sources 1 1 0 0 0 1 1
    let w = Win::new(0, 0, 7, 1);
    let v = [1u16, 1, 0, 0, 0, 1, 1];
    let mut p = SourceIndex::new(w, 1);
    for (x, &s) in v.iter().enumerate() {
        p.set(0, x as i32, 0, s);
    }
    p
}
fn row(m: &Matte) -> Vec<u8> {
    m.data.clone()
}

#[test]
fn mix_init_marks_source() {
    let p = row_sidx();
    let mut m = Matte::new(p.win, 9);
    matte_seed(&mut m, &p, 0, 0);
    assert_eq!(row(&m), vec![0, 0, 255, 255, 255, 0, 0]);
}
#[test]
fn mix_fill_sets_constant() {
    let mut m = Matte::new(Win::new(0, 0, 3, 1), 1);
    mix_fill(&mut m, 77);
    assert_eq!(row(&m), vec![77, 77, 77]);
}
#[test]
fn ramp_down_width_three() {
    let p = row_sidx();
    let mut m = Matte::new(p.win, 0);
    matte_seed(&mut m, &p, 0, 0);
    mix_ramp(&mut m, &p, 0, 3, Dir::Down);
    assert_eq!(row(&m), vec![85, 170, 255, 255, 255, 170, 85]);
}
#[test]
fn ramp_width_one_is_noop() {
    let p = row_sidx();
    let mut m = Matte::new(p.win, 0);
    matte_seed(&mut m, &p, 0, 0);
    mix_ramp(&mut m, &p, 0, 1, Dir::Down);
    assert_eq!(row(&m), vec![0, 0, 255, 255, 255, 0, 0]);
}
#[test]
fn ramp_delta_minimum_one() {
    let p = row_sidx();
    let mut m = Matte::new(p.win, 0);
    matte_seed(&mut m, &p, 0, 0);
    mix_ramp(&mut m, &p, 0, 300, Dir::Down);
    assert_eq!(row(&m), vec![253, 254, 255, 255, 255, 254, 253]);
}
#[test]
fn ramp_integer_quotient_width_four() {
    let p = row_sidx();
    let mut m = Matte::new(p.win, 0);
    matte_seed(&mut m, &p, 0, 0);
    mix_ramp(&mut m, &p, 0, 4, Dir::Down);
    // delta 63: 192 then 129 (3 steps, only two rings fit)
    assert_eq!(row(&m), vec![129, 192, 255, 255, 255, 192, 129]);
}
#[test]
fn ramp_does_not_enter_none_pixels() {
    let mut p = row_sidx();
    p.set(0, 1, 0, NO_SOURCE);
    let mut m = Matte::new(p.win, 0);
    matte_seed(&mut m, &p, 0, 0);
    mix_ramp(&mut m, &p, 0, 3, Dir::Down);
    assert_eq!(row(&m), vec![0, 0, 255, 255, 255, 170, 85]);
}
#[test]
fn ramp_up_direction_hand_case() {
    let p = {
        let mut p = SourceIndex::new(Win::new(0, 0, 7, 1), 1);
        for x in 0..7 {
            p.set(0, x, 0, 0);
        }
        p
    };
    let mut m = Matte::new(p.win, 0);
    for x in 2..5 {
        m.set(x, 0, 255);
    }
    mix_ramp(&mut m, &p, 0, 3, Dir::Up);
    assert_eq!(row(&m), vec![0, 0, 85, 170, 85, 0, 0]);
}
#[test]
fn ramp_outside_sidx_window_untouched() {
    let p = row_sidx();
    let mut m = Matte::new(Win::new(-2, 0, 11, 1), 0);
    matte_seed(&mut m, &p, 0, 0);
    mix_ramp(&mut m, &p, 0, 3, Dir::Down);
    assert_eq!(m.get(-1, 0), 0);
    assert_eq!(m.get(1, 0), 170);
}
#[test]
fn mask_out_outside_and_zero_from() {
    let w = Win::new(0, 0, 4, 1);
    let mut s = src_of(const_img(w, [0.0; 4]));
    s.h = translate(2.0, 0.0);
    s.cover.set(2, 0, 0);
    let mut m = Matte::new(w, 255);
    matte_mask_out(&mut m, &Camera::identity(), &s).unwrap();
    // x=0 -> 2 (zero from), x=1 -> 3 ok, x=2 -> 4 outside, x=3 -> 5 outside
    assert_eq!(row(&m), vec![0, 255, 0, 0]);
}
#[test]
fn mask_out_keeps_zero_values_zero() {
    let w = Win::new(0, 0, 2, 1);
    let s = src_of(const_img(w, [0.0; 4]));
    let mut m = Matte::new(w, 0);
    matte_mask_out(&mut m, &Camera::identity(), &s).unwrap();
    assert_eq!(row(&m), vec![0, 0]);
}
fn sin_zero(_: f32) -> f32 {
    0.0
}
fn sin_probe(d: f32) -> f32 {
    (d + 0.5) / 1234567.0
}
#[test]
fn scatter_total_zero_skipped_and_single_kept() {
    let w = Win::new(0, 0, 2, 1);
    let mut mats = vec![Matte::new(w, 0), Matte::new(w, 0)];
    mats[1].set(1, 0, 99);
    matte_scatter(&mut mats, w, 0.0, sin_zero);
    assert_eq!((mats[0].get(0, 0), mats[1].get(0, 0)), (0, 0));
    assert_eq!(mats[1].get(1, 0), 99);
}
#[test]
fn scatter_r_zero_keeps_first_nonzero() {
    let w = Win::new(0, 0, 1, 1);
    let mut mats = vec![Matte::new(w, 100), Matte::new(w, 155)];
    matte_scatter(&mut mats, w, 0.0, sin_zero);
    assert_eq!((mats[0].get(0, 0), mats[1].get(0, 0)), (100, 0));
}
#[test]
fn scatter_uses_d_formula() {
    // pixel (3,2): d = 3 + 20000 = 20003, r = 20003 % 255 = 113, running total 100 then 255
    let w = Win::new(0, 0, 4, 3);
    let mut mats = vec![Matte::new(w, 100), Matte::new(w, 155)];
    matte_scatter(&mut mats, w, 0.0, sin_probe);
    assert_eq!((mats[0].get(3, 2), mats[1].get(3, 2)), (0, 155));
}
#[test]
fn scatter_seed_enters_d() {
    let w = Win::new(0, 0, 4, 3);
    let mut mats = vec![Matte::new(w, 100), Matte::new(w, 155)];
    matte_scatter(&mut mats, w, 1.0, sin_probe);
    let d: f32 = 3.0f32 + 10000.0f32 * 2.0f32 + 100000000.0f32 * 1.0f32;
    let r = (((d + 0.5) / 1234567.0f32) * 1234567.0f32).abs() as u32 % 255;
    let keep_first = r < 100;
    assert_eq!(mats[0].get(3, 2) != 0, keep_first);
    assert_eq!(mats[1].get(3, 2) != 0, !keep_first);
}
#[test]
fn build_mix_matte_end_to_end_seam() {
    let w = Win::new(0, 0, 8, 1);
    let mut p = SourceIndex::new(w, 1);
    for x in 0..8 {
        p.set(0, x, 0, if x < 4 { 0 } else { 1 });
    }
    let s = src_of(const_img(w, [0.0; 4]));
    let mut r = FillConfig::default();
    r.ramp_len = 3;
    let m = build_mix_matte(w, &p, 0, 1, &Camera::identity(), &s, &r).unwrap();
    assert_eq!(row(&m), vec![0, 0, 85, 170, 255, 255, 255, 255]);
    r.ramp_phases = config::RampPhases::UpOnly;
    let m2 = build_mix_matte(w, &p, 0, 1, &Camera::identity(), &s, &r).unwrap();
    assert_ne!(row(&m), row(&m2));
}
#[test]
fn mix_window_back_map_unspecified() {
    assert!(matches!(
        mix_window_from_source_window(Win::new(0, 0, 1, 1)),
        Err(RemoveError::Unspecified(_))
    ));
}

fn sin_one(_: f32) -> f32 {
    1.0
}
fn sin_hundred_and_a_half(_: f32) -> f32 {
    100.5 / 1234567.0
}
#[test]
fn scatter_multiplier_is_1234567() {
    // r = 1234567 % 200 = 167 selects the second matte; another multiplier selects the first
    let w = Win::new(0, 0, 1, 1);
    let mut mats = vec![Matte::new(w, 100), Matte::new(w, 100)];
    matte_scatter(&mut mats, w, 0.0, sin_one);
    assert_eq!((mats[0].get(0, 0), mats[1].get(0, 0)), (0, 100));
}
#[test]
fn scatter_interval_upper_bound_is_exclusive() {
    // r = 100 lies in the second interval [100, 200)
    let w = Win::new(0, 0, 1, 1);
    let mut mats = vec![Matte::new(w, 100), Matte::new(w, 100)];
    matte_scatter(&mut mats, w, 0.0, sin_hundred_and_a_half);
    assert_eq!((mats[0].get(0, 0), mats[1].get(0, 0)), (0, 100));
}
#[test]
fn ramp_width_two_makes_exactly_one_ring() {
    let p = row_sidx();
    let mut m = Matte::new(p.win, 0);
    matte_seed(&mut m, &p, 0, 0);
    mix_ramp(&mut m, &p, 0, 2, Dir::Down);
    assert_eq!(row(&m), vec![0, 128, 255, 255, 255, 128, 0]);
}
