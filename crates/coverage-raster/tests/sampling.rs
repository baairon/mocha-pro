//! Point and supersample modes: tie rule, patterns, centre and direction conventions.

mod common;
use common::*;
use coverage_raster::*;

fn at_centre_zero(m: Method) -> Config {
    let mut c = cfg(m);
    c.centre = 0.0;
    c
}

#[test]
fn point_tie_rule_left_edge_inside_right_edge_outside() {
    // Centre 0.0: pixel i has its sample at coordinate i. Samples at 1 and 2 are inside [1, 3); 3 is
    // on the right edge and 0 is outside.
    let got = one(
        rect(1.0, 1.0, 3.0, 3.0),
        5,
        5,
        &at_centre_zero(Method::Point),
    );
    for j in 0..5u32 {
        for i in 0..5u32 {
            let want = if (1..=2).contains(&i) && (1..=2).contains(&j) {
                1.0
            } else {
                0.0
            };
            assert_eq!(got.at(i, j), want, "pixel {i},{j}");
        }
    }
}

#[test]
fn point_tie_rule_top_inside_bottom_outside() {
    let got = one(
        rect(0.0, 1.0, 4.0, 2.0),
        4,
        4,
        &at_centre_zero(Method::Point),
    );
    for i in 0..4 {
        assert_eq!(got.at(i, 0), 0.0);
        assert_eq!(got.at(i, 1), 1.0);
        assert_eq!(got.at(i, 2), 0.0);
    }
}

#[test]
fn point_tie_rule_same_for_clockwise_ring() {
    let a = one(
        rect(1.0, 1.0, 3.0, 3.0),
        5,
        5,
        &at_centre_zero(Method::Point),
    );
    let b = one(
        rect_cw(1.0, 1.0, 3.0, 3.0),
        5,
        5,
        &at_centre_zero(Method::Point),
    );
    assert_eq!(a, b);
}

#[test]
fn point_tie_rule_holds_with_flipped_rows() {
    let mut c = at_centre_zero(Method::Point);
    c.y_down = false;
    // Rows flip: buffer row r has its centre at y = 4 - r. The tie rule is applied in buffer
    // space, where the top is the smaller row: the rectangle spans buffer rows 1 to 3 (edges at
    // y = 3 and y = 1), so the top edge (y = 3, row 1) is inside and the bottom edge (y = 1,
    // row 3) is outside.
    let got = one(rect(1.0, 1.0, 3.0, 3.0), 5, 5, &c);
    for j in 0..5u32 {
        let inside_row = j == 1 || j == 2;
        for i in 0..5u32 {
            let want = if inside_row && (1..=2).contains(&i) {
                1.0
            } else {
                0.0
            };
            assert_eq!(got.at(i, j), want, "pixel {i},{j}");
        }
    }
}

#[test]
fn grid2_half_pixel_edge_is_half() {
    let got = one(
        rect(0.0, 0.0, 1.5, 1.0),
        2,
        1,
        &cfg(Method::Samples(Pattern::Grid(2))),
    );
    assert_rows(&got, &[&[1.0, 0.5]]);
}

#[test]
fn exact_and_grid2_agree_on_half_pixel_edges() {
    let p = rect(0.5, 0.0, 1.5, 2.0);
    let a = one(p.clone(), 2, 2, &exact());
    let b = one(p, 2, 2, &cfg(Method::Samples(Pattern::Grid(2))));
    assert_eq!(a, b);
}

#[test]
fn grid2_shifted_square_is_a_quarter_at_a_corner() {
    let got = one(
        rect(0.5, 0.5, 1.5, 1.5),
        2,
        2,
        &cfg(Method::Samples(Pattern::Grid(2))),
    );
    assert_rows(&got, &[&[0.25, 0.25], &[0.25, 0.25]]);
}

#[test]
fn grid4_hypotenuse_ties_count_as_outside() {
    // Pixel (1, 0) of the triangle with legs 2: samples at (1 + a, b) with a, b in 1/8, 3/8, 5/8,
    // 7/8. Inside when a + b < 1, strictly (the hypotenuse is a right edge): 3 + 2 + 1 + 0 of 16.
    let got = one(
        tri((0.0, 0.0), (2.0, 0.0), (0.0, 2.0)),
        2,
        2,
        &cfg(Method::Samples(Pattern::Grid(4))),
    );
    assert_eq!(got.at(0, 0), 1.0);
    assert_eq!(got.at(1, 0), 6.0 / 16.0);
    assert_eq!(got.at(0, 1), 6.0 / 16.0);
    assert_eq!(got.at(1, 1), 0.0);
}

#[test]
fn rotated_four_offsets_are_listed_by_the_definition() {
    assert_eq!(
        pattern_offsets(&Pattern::Rotated(4)),
        vec![
            (0.125, 0.125),
            (0.375, 0.875),
            (0.625, 0.625),
            (0.875, 0.375)
        ]
    );
}

#[test]
fn rotated_five_uses_stride_two() {
    let o = pattern_offsets(&Pattern::Rotated(5));
    let ys: Vec<f64> = o.iter().map(|p| p.1).collect();
    assert_eq!(ys, vec![0.1, 0.5, 0.9, 0.3, 0.7]);
}

#[test]
fn rotated_one_is_the_centre() {
    assert_eq!(pattern_offsets(&Pattern::Rotated(1)), vec![(0.5, 0.5)]);
}

#[test]
fn rotated_pattern_has_one_sample_per_row_and_column() {
    for n in [2u32, 3, 4, 5, 6, 7, 8, 9, 12, 16] {
        let o = pattern_offsets(&Pattern::Rotated(n));
        let mut xs: Vec<u32> = o.iter().map(|p| (p.0 * n as f64) as u32).collect();
        let mut ys: Vec<u32> = o.iter().map(|p| (p.1 * n as f64) as u32).collect();
        xs.sort_unstable();
        ys.sort_unstable();
        let want: Vec<u32> = (0..n).collect();
        assert_eq!(xs, want, "n {n}");
        assert_eq!(ys, want, "n {n}");
    }
}

#[test]
fn grid_offsets_are_listed_row_by_row() {
    assert_eq!(
        pattern_offsets(&Pattern::Grid(2)),
        vec![(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)]
    );
}

#[test]
fn explicit_offsets_are_used_as_given() {
    // The rectangle ends at x = 1.5; pixel 1 spans [1, 2].
    let at = |ox: f64| {
        let m = Method::Samples(Pattern::Offsets(vec![(ox, 0.5)]));
        one(rect(0.0, 0.0, 1.5, 1.0), 2, 1, &cfg(m)).at(1, 0)
    };
    assert_eq!(at(0.25), 1.0);
    assert_eq!(at(0.75), 0.0);
    assert_eq!(at(0.5), 0.0);
}

#[test]
fn even_odd_and_non_zero_differ_for_samples() {
    let ring = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
    let p = Polygon(vec![ring.clone(), ring]);
    let mut c = cfg(Method::Point);
    let nz = one(p.clone(), 2, 2, &c);
    c.fill = FillRule::EvenOdd;
    let eo = one(p, 2, 2, &c);
    assert_eq!(nz.sum(), 4.0);
    assert_eq!(eo.sum(), 0.0);
}

#[test]
fn hole_under_even_odd_with_samples() {
    let p = Polygon(vec![
        vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
        vec![(1.0, 1.0), (3.0, 1.0), (3.0, 3.0), (1.0, 3.0)],
    ]);
    let mut c = cfg(Method::Point);
    c.fill = FillRule::EvenOdd;
    let got = one(p, 4, 4, &c);
    assert_eq!(got.sum(), 12.0);
    assert_eq!(got.at(1, 1), 0.0);
}

#[test]
fn centre_zero_moves_the_pixel_square_by_half() {
    let p = rect(-0.5, -0.5, 1.5, 1.5);
    let mut c = exact();
    c.centre = 0.0;
    let got = one(p.clone(), 2, 2, &c);
    assert!(got.data.iter().all(|&v| v == 1.0));
    let got = one(p, 2, 2, &exact());
    assert_rows(&got, &[&[1.0, 0.5], &[0.5, 0.25]]);
}

#[test]
fn centre_zero_equals_shifting_the_polygon_by_half() {
    let mut c = exact();
    c.centre = 0.0;
    let a = one(rect(0.25, 0.25, 1.75, 1.0), 2, 2, &c);
    let b = one(rect(0.75, 0.75, 2.25, 1.5), 2, 2, &exact());
    assert_eq!(a, b);
}

#[test]
fn y_up_flips_the_rows() {
    let mut c = exact();
    c.y_down = false;
    let got = one(rect(0.0, 0.0, 2.0, 1.0), 2, 2, &c);
    assert_rows(&got, &[&[0.0, 0.0], &[1.0, 1.0]]);
    let got = one(rect(0.0, 0.0, 2.0, 1.0), 2, 2, &exact());
    assert_rows(&got, &[&[1.0, 1.0], &[0.0, 0.0]]);
}

#[test]
fn y_up_with_centre_zero_has_pixel_centres_on_integers() {
    let mut c = exact();
    c.y_down = false;
    c.centre = 0.0;
    // Row 0 is the largest y: with height 2 its centre is y = 1 and it covers [0.5, 1.5].
    let got = one(rect(-0.5, 0.5, 0.5, 1.5), 1, 2, &c);
    assert_rows(&got, &[&[1.0], &[0.0]]);
}

#[test]
fn target_origin_moves_the_window() {
    let t = Target {
        x: 10,
        y: 20,
        width: 2,
        height: 2,
    };
    let got = rasterise(&[rect(11.0, 21.0, 12.0, 22.0)], &t, &exact()).unwrap();
    assert_rows(&got, &[&[0.0, 0.0], &[0.0, 1.0]]);
}

#[test]
fn negative_target_origin() {
    let t = Target {
        x: -3,
        y: -3,
        width: 2,
        height: 2,
    };
    let got = rasterise(&[rect(-3.0, -3.0, -2.0, -2.0)], &t, &exact()).unwrap();
    assert_rows(&got, &[&[1.0, 0.0], &[0.0, 0.0]]);
}

#[test]
fn snapping_rounds_vertices_to_a_grid() {
    let mut c = exact();
    c.snap_bits = Some(1);
    // 0.3 snaps to 0.5 and 1.2 snaps to 1.0.
    let got = one(rect(0.3, 0.0, 1.2, 1.0), 2, 1, &c);
    assert_rows(&got, &[&[0.5, 0.0]]);
}

#[test]
fn snapping_zero_bits_rounds_to_integers() {
    let mut c = exact();
    c.snap_bits = Some(0);
    let got = one(rect(0.4, 0.6, 2.6, 2.4), 3, 3, &c);
    assert_rows(
        &got,
        &[&[0.0, 0.0, 0.0], &[1.0, 1.0, 1.0], &[0.0, 0.0, 0.0]],
    );
}
