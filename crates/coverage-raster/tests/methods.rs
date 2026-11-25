//! Behaviour shared by every method, run once per method.

mod common;
use common::*;
use coverage_raster::*;

macro_rules! per_method {
    ($name:ident, |$c:ident| $body:block) => {
        mod $name {
            use super::*;
            fn body($c: &Config) $body
            #[test]
            fn exact() {
                body(&cfg(Method::ExactArea))
            }
            #[test]
            fn point() {
                body(&cfg(Method::Point))
            }
            #[test]
            fn grid2() {
                body(&cfg(Method::Samples(Pattern::Grid(2))))
            }
            #[test]
            fn grid3() {
                body(&cfg(Method::Samples(Pattern::Grid(3))))
            }
            #[test]
            fn rotated4() {
                body(&cfg(Method::Samples(Pattern::Rotated(4))))
            }
            #[test]
            fn offsets() {
                body(&cfg(Method::Samples(Pattern::Offsets(vec![
                    (0.25, 0.5),
                    (0.75, 0.5),
                ]))))
            }
        }
    };
}

per_method!(integer_square_is_one_inside_zero_outside, |c| {
    let got = one(rect(1.0, 1.0, 3.0, 3.0), 4, 4, c);
    assert_rows(
        &got,
        &[
            &[0.0, 0.0, 0.0, 0.0],
            &[0.0, 1.0, 1.0, 0.0],
            &[0.0, 1.0, 1.0, 0.0],
            &[0.0, 0.0, 0.0, 0.0],
        ],
    );
});

per_method!(whole_target_square_is_all_ones, |c| {
    let got = one(rect(0.0, 0.0, 3.0, 2.0), 3, 2, c);
    assert!(got.data.iter().all(|&v| v == 1.0));
});

per_method!(polygon_wholly_outside_gives_zeros, |c| {
    for p in [
        rect(10.0, 10.0, 12.0, 12.0),
        rect(-5.0, 0.0, -1.0, 3.0),
        rect(0.0, -5.0, 3.0, -1.0),
        rect(3.0, 0.0, 6.0, 3.0),
    ] {
        let got = one(p, 3, 3, c);
        assert!(got.data.iter().all(|&v| v == 0.0));
    }
});

per_method!(clockwise_equals_counter_clockwise, |c| {
    let a = one(rect(0.0, 0.0, 2.0, 2.0), 3, 3, c);
    let b = one(rect_cw(0.0, 0.0, 2.0, 2.0), 3, 3, c);
    assert_eq!(a, b);
    let mut e = c.clone();
    e.fill = FillRule::EvenOdd;
    assert_eq!(one(rect_cw(0.0, 0.0, 2.0, 2.0), 3, 3, &e), a);
});

per_method!(partly_outside_square_is_clipped, |c| {
    let got = one(rect(-1.0, -1.0, 1.0, 1.0), 2, 2, c);
    assert_rows(&got, &[&[1.0, 0.0], &[0.0, 0.0]]);
    let got = one(rect(1.0, 1.0, 5.0, 5.0), 2, 2, c);
    assert_rows(&got, &[&[0.0, 0.0], &[0.0, 1.0]]);
});

per_method!(empty_inputs_give_zeros, |c| {
    assert!(run(&[], 3, 3, c).data.iter().all(|&v| v == 0.0));
    assert!(one(Polygon(vec![]), 3, 3, c).data.iter().all(|&v| v == 0.0));
    assert!(one(Polygon(vec![vec![]]), 3, 3, c)
        .data
        .iter()
        .all(|&v| v == 0.0));
});

per_method!(point_and_line_rings_cover_nothing, |c| {
    for ring in [
        vec![(1.0, 1.0)],
        vec![(1.0, 1.0), (1.0, 1.0), (1.0, 1.0)],
        vec![(0.0, 0.0), (2.0, 2.0)],
        vec![(0.0, 0.5), (3.0, 0.5), (1.0, 0.5)],
    ] {
        let got = one(Polygon::from_ring(ring), 3, 3, c);
        assert!(got.data.iter().all(|&v| v == 0.0));
    }
});

per_method!(horizontal_only_rectangle_covers_nothing, |c| {
    let got = one(rect(0.0, 1.0, 3.0, 1.0), 3, 3, c);
    assert!(got.data.iter().all(|&v| v == 0.0));
});

per_method!(zero_size_target_is_empty, |c| {
    let got = run(&[rect(0.0, 0.0, 1.0, 1.0)], 0, 5, c);
    assert!(got.data.is_empty());
    let got = run(&[rect(0.0, 0.0, 1.0, 1.0)], 5, 0, c);
    assert!(got.data.is_empty());
});

per_method!(repeated_run_is_identical, |c| {
    let p = tri((0.3, 0.2), (3.7, 0.9), (1.1, 2.8));
    let a = one(p.clone(), 4, 4, c);
    let b = one(p, 4, 4, c);
    assert_eq!(a, b);
});

per_method!(duplicate_consecutive_points_change_nothing, |c| {
    let a = one(rect(0.0, 0.0, 2.0, 2.0), 3, 3, c);
    let b = one(
        Polygon::from_ring(vec![
            (0.0, 0.0),
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 0.0),
            (2.0, 2.0),
            (0.0, 2.0),
            (0.0, 2.0),
        ]),
        3,
        3,
        c,
    );
    assert_eq!(a, b);
});

per_method!(closing_point_repeated_changes_nothing, |c| {
    let a = one(rect(0.0, 0.0, 2.0, 2.0), 3, 3, c);
    let b = one(
        Polygon::from_ring(vec![
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 2.0),
            (0.0, 2.0),
            (0.0, 0.0),
        ]),
        3,
        3,
        c,
    );
    assert_eq!(a, b);
});

#[test]
fn point_equals_grid_one() {
    let mut r = Lcg(7);
    for _ in 0..40 {
        let p = tri(
            (r.eighth(), r.eighth()),
            (r.eighth(), r.eighth()),
            (r.eighth(), r.eighth()),
        );
        for centre in [0.0, 0.5] {
            let mut a = cfg(Method::Point);
            let mut b = cfg(Method::Samples(Pattern::Grid(1)));
            a.centre = centre;
            b.centre = centre;
            assert_eq!(one(p.clone(), 8, 8, &a), one(p.clone(), 8, 8, &b));
        }
    }
}
