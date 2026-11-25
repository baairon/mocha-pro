#![allow(clippy::needless_range_loop)]

use planar_xform::*;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
fn sp(center: PixelCenter, y: YAxis, aspect: f64, conv: Convention) -> PixelSpace {
    PixelSpace::new(
        8.0,
        4.0,
        Config {
            center,
            y_axis: y,
            pixel_aspect: aspect,
            convention: conv,
            ..Config::default()
        },
    )
}
fn ap(h: &Homography, x: f64, y: f64) -> Point {
    h.apply(p(x, y), &Config::default()).unwrap()
}
const H: PixelCenter = PixelCenter::Half;
const I: PixelCenter = PixelCenter::Integer;
const D: YAxis = YAxis::Down;
const U: YAxis = YAxis::Up;
const C: Convention = Convention::Column;

#[test]
fn half_down_corners() {
    let s = sp(H, D, 1.0, C);
    assert_eq!(ap(&s.unit_to_pixel(), 0.0, 0.0), p(0.0, 0.0));
    assert_eq!(ap(&s.unit_to_pixel(), 1.0, 1.0), p(8.0, 4.0));
}
#[test]
fn half_down_matrix_exact() {
    let s = sp(H, D, 1.0, C);
    assert_eq!(
        s.unit_to_pixel().m,
        [[8.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 1.0]]
    );
    assert_eq!(
        s.pixel_to_unit().m,
        [[0.125, 0.0, 0.0], [0.0, 0.25, 0.0], [0.0, 0.0, 1.0]]
    );
}
#[test]
fn half_up_corners() {
    let s = sp(H, U, 1.0, C);
    assert_eq!(ap(&s.unit_to_pixel(), 0.0, 0.0), p(0.0, 4.0));
    assert_eq!(ap(&s.unit_to_pixel(), 1.0, 1.0), p(8.0, 0.0));
}
#[test]
fn half_up_matrix_exact() {
    let s = sp(H, U, 1.0, C);
    assert_eq!(
        s.unit_to_pixel().m,
        [[8.0, 0.0, 0.0], [0.0, -4.0, 4.0], [0.0, 0.0, 1.0]]
    );
    assert_eq!(
        s.pixel_to_unit().m,
        [[0.125, 0.0, 0.0], [0.0, -0.25, 1.0], [0.0, 0.0, 1.0]]
    );
}
#[test]
fn integer_down_corners() {
    let s = sp(I, D, 1.0, C);
    assert_eq!(ap(&s.unit_to_pixel(), 0.0, 0.0), p(-0.5, -0.5));
    assert_eq!(ap(&s.unit_to_pixel(), 1.0, 1.0), p(7.5, 3.5));
}
#[test]
fn integer_up_corners() {
    let s = sp(I, U, 1.0, C);
    assert_eq!(ap(&s.unit_to_pixel(), 0.0, 0.0), p(-0.5, 3.5));
    assert_eq!(ap(&s.unit_to_pixel(), 1.0, 1.0), p(7.5, -0.5));
}
#[test]
fn integer_pixel_zero_centre_is_at_zero() {
    let s = sp(I, D, 1.0, C);
    // pixel 0 centre at coordinate 0 -> u = 0.5/8, v = 0.5/4
    assert_eq!(ap(&s.pixel_to_unit(), 0.0, 0.0), p(0.0625, 0.125));
}
#[test]
fn half_pixel_zero_centre_is_at_half() {
    let s = sp(H, D, 1.0, C);
    assert_eq!(ap(&s.pixel_to_unit(), 0.5, 0.5), p(0.0625, 0.125));
}
#[test]
fn half_and_integer_differ_by_half_pixel() {
    let a = sp(H, D, 1.0, C).pixel_to_unit();
    let b = sp(I, D, 1.0, C).pixel_to_unit();
    assert_eq!(ap(&a, 1.5, 2.5), ap(&b, 1.0, 2.0));
}
#[test]
fn centre_of_image_is_half_in_every_variant() {
    for c in [H, I] {
        for y in [D, U] {
            let s = sp(c, y, 1.0, C);
            let k = if c == H { 4.0 } else { 3.5 };
            let kx = if c == H { 4.0 } else { 3.5 };
            let _ = k;
            let q = ap(&s.pixel_to_unit(), kx, if c == H { 2.0 } else { 1.5 });
            assert_eq!(q, p(0.5, 0.5));
        }
    }
}
#[test]
fn round_trip_exact_on_dyadic_all_variants() {
    for cfgs in Config::enumerate(&[], &[1.0, 2.0, 0.5]) {
        let s = PixelSpace::new(8.0, 4.0, cfgs);
        let f = s.unit_to_pixel();
        let b = s.pixel_to_unit();
        for pt in [p(0.0, 0.0), p(0.25, 0.75), p(1.0, 1.0), p(0.5, 0.125)] {
            let c = Config::default();
            let q = f.apply(pt, &c).unwrap();
            assert_eq!(b.apply(q, &c).unwrap(), pt);
        }
    }
}
#[test]
fn closed_form_matches_numeric_inverse() {
    for cfgs in Config::enumerate(&[], &[1.0]) {
        let s = PixelSpace::new(8.0, 4.0, cfgs);
        let inv = s.unit_to_pixel().inverse().unwrap();
        for pt in [p(0.0, 0.0), p(3.0, 2.0), p(7.5, 1.5)] {
            let c = Config::default();
            assert_eq!(inv.apply(pt, &c), s.pixel_to_unit().apply(pt, &c));
        }
    }
}
#[test]
fn row_convention_matrices_are_transposed() {
    let a = sp(H, U, 1.0, C);
    let b = sp(H, U, 1.0, Convention::Row);
    assert_eq!(b.unit_to_pixel().column_matrix(), a.unit_to_pixel().m);
    assert_eq!(
        b.unit_to_pixel().m,
        [[8.0, 0.0, 0.0], [0.0, -4.0, 0.0], [0.0, 4.0, 1.0]]
    );
    assert_eq!(ap(&b.unit_to_pixel(), 1.0, 1.0), p(8.0, 0.0));
}
#[test]
fn aspect_physical_width() {
    let s = sp(H, D, 2.0, C);
    assert_eq!(ap(&s.unit_to_physical(), 1.0, 1.0), p(16.0, 4.0));
    assert_eq!(ap(&s.physical_to_unit(), 16.0, 4.0), p(1.0, 1.0));
}
#[test]
fn aspect_does_not_change_unit_pixel_mapping() {
    let a = sp(H, D, 1.0, C);
    let b = sp(H, D, 2.0, C);
    assert_eq!(a.unit_to_pixel().m, b.unit_to_pixel().m);
}
#[test]
fn aspect_pixel_to_physical_matrix() {
    let s = sp(H, D, 0.5, C);
    assert_eq!(
        s.pixel_to_physical().m,
        [[0.5, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    );
    assert_eq!(
        s.physical_to_pixel().m,
        [[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    );
}
#[test]
fn aspect_integer_up_physical_corner() {
    let s = sp(I, U, 2.0, C);
    assert_eq!(ap(&s.unit_to_physical(), 1.0, 1.0), p(15.0, -0.5));
}
#[test]
fn pixel_rect_to_unit_half_down() {
    let s = sp(H, D, 1.0, C);
    let r = s.pixel_rect_to_unit(&Rect::new(2.0, 1.0, 6.0, 3.0));
    assert_eq!(r, Rect::new(0.25, 0.25, 0.75, 0.75));
}
#[test]
fn pixel_rect_to_unit_half_up_is_sorted() {
    let s = sp(H, U, 1.0, C);
    let r = s.pixel_rect_to_unit(&Rect::new(0.0, 0.0, 4.0, 1.0));
    assert_eq!(r, Rect::new(0.0, 0.75, 0.5, 1.0));
}
#[test]
fn unit_rect_to_pixel_up_is_sorted() {
    let s = sp(H, U, 1.0, C);
    let r = s.unit_rect_to_pixel(&Rect::new(0.0, 0.75, 0.5, 1.0));
    assert_eq!(r, Rect::new(0.0, 0.0, 4.0, 1.0));
}
#[test]
fn unit_rect_to_pixel_integer_down() {
    let s = sp(I, D, 1.0, C);
    let r = s.unit_rect_to_pixel(&Rect::new(0.0, 0.0, 1.0, 1.0));
    assert_eq!(r, Rect::new(-0.5, -0.5, 7.5, 3.5));
}
#[test]
fn rect_round_trip() {
    for c in Config::enumerate(&[], &[1.0]) {
        let s = PixelSpace::new(8.0, 4.0, c);
        let r = Rect::new(1.0, 1.0, 5.0, 3.0);
        assert_eq!(s.unit_rect_to_pixel(&s.pixel_rect_to_unit(&r)), r);
    }
}
#[test]
fn physical_round_trip_pixel() {
    let s = sp(H, D, 4.0, C);
    let q = s
        .physical_to_pixel()
        .apply(p(8.0, 3.0), &Config::default())
        .unwrap();
    assert_eq!(q, p(2.0, 3.0));
    assert_eq!(
        s.pixel_to_physical().apply(q, &Config::default()).unwrap(),
        p(8.0, 3.0)
    );
}
#[test]
fn all_variants_give_distinct_unit_to_pixel_matrices() {
    let v: Vec<_> = Config::enumerate(&[], &[1.0])
        .into_iter()
        .filter(|c| c.convention == C)
        .map(|c| PixelSpace::new(8.0, 4.0, c).unit_to_pixel().m)
        .collect();
    // guards do not change geometry: four geometries, each seen twice
    for m in &v {
        assert_eq!(v.iter().filter(|o| *o == m).count(), 2);
    }
}

#[test]
fn physical_to_unit_order_with_offsets() {
    let s = sp(I, D, 2.0, C);
    assert_eq!(ap(&s.physical_to_unit(), 8.0, 2.0), p(0.5625, 0.625));
    assert_eq!(ap(&s.unit_to_physical(), 0.5625, 0.625), p(8.0, 2.0));
}
