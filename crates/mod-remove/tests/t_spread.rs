#![allow(clippy::field_reassign_with_default)]
mod support;
use mod_remove::config::SpreadBoundary;
use mod_remove::spread::diffuse_gaps;
use mod_remove::*;
use support::*;

fn setup(filled_cols: &[i32]) -> (Image, Matte, Vec<bool>) {
    let w = Win::new(0, 0, 6, 3);
    let img = img_fn(w, |x, _| [x as f32, 0.0, 0.0, 0.25]);
    let fill = rect_matte(w, 0, 0, 5, 2);
    let mut f = vec![false; w.area()];
    for &c in filled_cols {
        for y in 0..3 {
            f[w.idx(c, y)] = true;
        }
    }
    (img, fill, f)
}

#[test]
fn spread_diffuses_from_boundary_alpha_one() {
    let (mut img, fill, mut f) = setup(&[0, 1]);
    diffuse_gaps(&mut img, &fill, &mut f, &FillConfig::default()).unwrap();
    assert!(f.iter().all(|&b| b));
    // pass 1: column 2 = mean of column 1 = 1.0, alpha 1
    assert_eq!(bits(img.at(2, 1)), bits([1.0, 0.0, 0.0, 1.0]));
    // pass 2: column 3 = mean of column 2 only (column 1 now has no unfilled neighbour)
    assert_eq!(bits(img.at(3, 1)), bits([1.0, 0.0, 0.0, 1.0]));
    assert_eq!(bits(img.at(5, 0)), bits([1.0, 0.0, 0.0, 1.0]));
}
#[test]
fn spread_any_filled_reading_differs() {
    let (mut img, fill, mut f) = setup(&[0, 1]);
    let mut r = FillConfig::default();
    r.spread_boundary = SpreadBoundary::AnyFilled;
    diffuse_gaps(&mut img, &fill, &mut f, &r).unwrap();
    // column 2 = mean of columns 0 and 1 over the 5x5 window rows: all filled pixels, colour 0.5
    assert_eq!(img.at(2, 1)[0], 0.5);
}
#[test]
fn spread_far_pixel_gets_boundary_colour() {
    let (mut img, fill, mut f) = setup(&[0, 1]);
    diffuse_gaps(&mut img, &fill, &mut f, &FillConfig::default()).unwrap();
    assert_eq!(img.at(5, 2)[0], 1.0);
    assert_eq!(img.at(0, 0)[0], 0.0); // already filled pixels are untouched
    assert_eq!(img.at(0, 0)[3], 0.25);
}
#[test]
fn spread_with_nothing_filled_is_refused() {
    let (mut img, fill, mut f) = setup(&[]);
    let e = diffuse_gaps(&mut img, &fill, &mut f, &FillConfig::default());
    assert!(matches!(e, Err(RemoveError::Refused(_))));
}
#[test]
fn spread_nothing_to_do_is_ok() {
    let (mut img, fill, mut f) = setup(&[0, 1, 2, 3, 4, 5]);
    let before = img.clone();
    diffuse_gaps(&mut img, &fill, &mut f, &FillConfig::default()).unwrap();
    assert_eq!(img, before);
}
#[test]
fn spread_level_unspecified() {
    let (mut img, fill, mut f) = setup(&[0]);
    let mut r = FillConfig::default();
    r.spread_level = 2;
    assert!(matches!(
        diffuse_gaps(&mut img, &fill, &mut f, &r),
        Err(RemoveError::Unspecified(_))
    ));
}
#[test]
fn spread_leaves_pixels_outside_fill_matte() {
    let w = Win::new(0, 0, 6, 3);
    let mut img = img_fn(w, |x, _| [x as f32, 0.0, 0.0, 0.25]);
    let fill = rect_matte(w, 0, 0, 3, 2);
    let mut f = vec![false; w.area()];
    for y in 0..3 {
        f[w.idx(0, y)] = true;
    }
    diffuse_gaps(&mut img, &fill, &mut f, &FillConfig::default()).unwrap();
    assert_eq!(bits(img.at(4, 1)), bits([4.0, 0.0, 0.0, 0.25]));
    assert!(!f[w.idx(4, 1)]);
}

#[test]
fn spread_window_reaches_two_rows_away() {
    let w = Win::new(0, 0, 6, 5);
    let mut img = img_fn(w, |x, _| [x as f32, 0.0, 0.0, 0.25]);
    let fill = rect_matte(w, 0, 0, 5, 4);
    let mut f = vec![false; w.area()];
    f[w.idx(1, 1)] = true;
    f[w.idx(2, 2)] = true;
    let mut r = FillConfig::default();
    r.spread_boundary = SpreadBoundary::AnyFilled;
    diffuse_gaps(&mut img, &fill, &mut f, &r).unwrap();
    // pixel (2,0) in pass 1 averages (1,1) and (2,2): the second is two rows below
    assert_eq!(img.at(2, 0)[0], 1.5);
}

#[test]
fn spread_level_one_is_unspecified() {
    let (mut img, fill, mut f) = setup(&[0]);
    let mut r = FillConfig::default();
    r.spread_level = 1;
    assert!(matches!(
        diffuse_gaps(&mut img, &fill, &mut f, &r),
        Err(RemoveError::Unspecified(_))
    ));
}
