//! Exact area mode against hand computed areas.

mod common;
use common::*;
use coverage_raster::*;

#[test]
fn rectangle_on_half_integers() {
    let got = one(rect(0.5, 0.5, 2.5, 1.5), 3, 2, &exact());
    assert_rows(&got, &[&[0.25, 0.5, 0.25], &[0.25, 0.5, 0.25]]);
}

#[test]
fn rectangle_on_quarter_positions() {
    let got = one(rect(0.25, 0.0, 1.75, 1.0), 2, 1, &exact());
    assert_rows(&got, &[&[0.75, 0.75]]);
}

#[test]
fn rectangle_inside_one_pixel() {
    let got = one(rect(0.25, 0.5, 0.75, 1.0), 2, 2, &exact());
    assert_rows(&got, &[&[0.25, 0.0], &[0.0, 0.0]]);
}

#[test]
fn right_triangle_legs_two() {
    let got = one(tri((0.0, 0.0), (2.0, 0.0), (0.0, 2.0)), 2, 2, &exact());
    assert_rows(&got, &[&[1.0, 0.5], &[0.5, 0.0]]);
}

#[test]
fn right_triangle_legs_four() {
    let got = one(tri((0.0, 0.0), (4.0, 0.0), (0.0, 4.0)), 4, 4, &exact());
    assert_rows(
        &got,
        &[
            &[1.0, 1.0, 1.0, 0.5],
            &[1.0, 1.0, 0.5, 0.0],
            &[1.0, 0.5, 0.0, 0.0],
            &[0.5, 0.0, 0.0, 0.0],
        ],
    );
}

#[test]
fn triangle_with_slope_one_half() {
    let got = one(tri((0.0, 0.0), (4.0, 0.0), (0.0, 2.0)), 4, 2, &exact());
    assert_rows(&got, &[&[1.0, 1.0, 0.75, 0.25], &[0.75, 0.25, 0.0, 0.0]]);
}

#[test]
fn triangle_with_slope_one_half_second_row() {
    // Row 1 is y in [1, 2]: the edge y = 2 - x / 2 is above y = 1 for x < 2.
    let got = one(tri((0.0, 0.0), (4.0, 0.0), (0.0, 2.0)), 4, 2, &exact());
    assert_eq!(got.at(0, 1), 0.75);
    assert_eq!(got.at(1, 1), 0.25);
}

#[test]
fn triangle_sum_equals_area() {
    let got = one(tri((0.0, 0.0), (4.0, 0.0), (0.0, 4.0)), 4, 4, &exact());
    assert_eq!(got.sum(), 8.0);
}

#[test]
fn dyadic_rectangles_match_overlap_product() {
    let mut r = Lcg(99);
    for _ in 0..300 {
        let (a, b) = (r.eighth(), r.eighth());
        let (c, d) = (r.eighth(), r.eighth());
        let (x0, x1) = (a.min(b), a.max(b));
        let (y0, y1) = (c.min(d), c.max(d));
        let got = one(rect(x0, y0, x1, y1), 8, 8, &exact());
        for j in 0..8u32 {
            for i in 0..8u32 {
                let ox = (x1.min(i as f64 + 1.0) - x0.max(i as f64)).max(0.0);
                let oy = (y1.min(j as f64 + 1.0) - y0.max(j as f64)).max(0.0);
                assert_eq!(
                    got.at(i, j),
                    ox * oy,
                    "rect {x0} {y0} {x1} {y1} pixel {i},{j}"
                );
            }
        }
    }
}

#[test]
fn dyadic_rectangle_sum_equals_area_exactly() {
    let mut r = Lcg(5);
    for _ in 0..90 {
        let (a, b) = (r.eighth(), r.eighth());
        let (c, d) = (r.eighth(), r.eighth());
        let (x0, x1) = (a.min(b), a.max(b));
        let (y0, y1) = (c.min(d), c.max(d));
        let got = one(rect(x0, y0, x1, y1), 8, 8, &exact());
        assert_eq!(got.sum(), (x1 - x0) * (y1 - y0));
    }
}

#[test]
fn diagonal_unit_slope_triangles_are_exact() {
    let mut r = Lcg(21);
    for _ in 0..90 {
        let x = (r.next() % 6) as f64;
        let y = (r.next() % 6) as f64;
        let s = 1.0 + (r.next() % 2) as f64;
        let p = tri((x, y), (x + s, y), (x, y + s));
        let got = one(p, 8, 8, &exact());
        assert_eq!(got.sum(), s * s * 0.5);
    }
}

#[test]
fn hole_under_nonzero_same_orientation_is_filled() {
    let p = Polygon(vec![
        vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
        vec![(1.0, 1.0), (3.0, 1.0), (3.0, 3.0), (1.0, 3.0)],
    ]);
    let got = one(p, 4, 4, &exact());
    assert!(got.data.iter().all(|&v| v == 1.0));
}

#[test]
fn hole_under_even_odd_is_empty() {
    let p = Polygon(vec![
        vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
        vec![(1.0, 1.0), (3.0, 1.0), (3.0, 3.0), (1.0, 3.0)],
    ]);
    let mut c = exact();
    c.fill = FillRule::EvenOdd;
    let got = one(p, 4, 4, &c);
    assert_rows(
        &got,
        &[
            &[1.0, 1.0, 1.0, 1.0],
            &[1.0, 0.0, 0.0, 1.0],
            &[1.0, 0.0, 0.0, 1.0],
            &[1.0, 1.0, 1.0, 1.0],
        ],
    );
}

#[test]
fn hole_with_opposite_orientation_is_empty_under_both_rules() {
    let p = Polygon(vec![
        vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
        vec![(1.0, 1.0), (1.0, 3.0), (3.0, 3.0), (3.0, 1.0)],
    ]);
    for fill in [FillRule::NonZero, FillRule::EvenOdd] {
        let mut c = exact();
        c.fill = fill;
        let got = one(p.clone(), 4, 4, &c);
        assert_eq!(got.at(1, 1), 0.0);
        assert_eq!(got.at(2, 2), 0.0);
        assert_eq!(got.at(0, 0), 1.0);
        assert_eq!(got.sum(), 12.0);
    }
}

#[test]
fn hole_at_half_positions() {
    let p = Polygon(vec![
        vec![(0.0, 0.0), (3.0, 0.0), (3.0, 3.0), (0.0, 3.0)],
        vec![(0.5, 0.5), (0.5, 2.5), (2.5, 2.5), (2.5, 0.5)],
    ]);
    let got = one(p, 3, 3, &exact());
    assert_rows(
        &got,
        &[&[0.75, 0.5, 0.75], &[0.5, 0.0, 0.5], &[0.75, 0.5, 0.75]],
    );
}

#[test]
fn doubly_wound_square_differs_between_rules() {
    let ring = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
    let p = Polygon(vec![ring.clone(), ring]);
    let mut c = exact();
    let nz = one(p.clone(), 2, 2, &c);
    c.fill = FillRule::EvenOdd;
    let eo = one(p, 2, 2, &c);
    assert!(nz.data.iter().all(|&v| v == 1.0));
    assert!(eo.data.iter().all(|&v| v == 0.0));
}

#[test]
fn bowtie_has_two_half_triangles_under_both_rules() {
    let p = Polygon::from_ring(vec![(0.0, 0.0), (2.0, 2.0), (2.0, 0.0), (0.0, 2.0)]);
    for fill in [FillRule::NonZero, FillRule::EvenOdd] {
        let mut c = exact();
        c.fill = fill;
        let got = one(p.clone(), 2, 2, &c);
        assert_rows(&got, &[&[0.5, 0.5], &[0.5, 0.5]]);
    }
}

#[test]
fn pentagram_centre_differs_between_rules() {
    // A five pointed star drawn as one self crossing ring: points 0, 2, 4, 1, 3 of a regular
    // pentagon scaled to a 40 wide frame. The central pentagon has winding 2.
    let pts: Vec<(f64, f64)> = (0..5)
        .map(|k| {
            let a = std::f64::consts::PI * 2.0 * (k as f64) / 5.0 - std::f64::consts::FRAC_PI_2;
            (20.0 + 18.0 * a.cos(), 20.0 + 18.0 * a.sin())
        })
        .collect();
    let order = [0, 2, 4, 1, 3];
    let ring: Vec<(f64, f64)> = order.iter().map(|&k| pts[k]).collect();
    let p = Polygon::from_ring(ring);
    let mut c = exact();
    let nz = one(p.clone(), 40, 40, &c);
    c.fill = FillRule::EvenOdd;
    let eo = one(p, 40, 40, &c);
    assert_eq!(nz.at(20, 20), 1.0);
    assert_eq!(eo.at(20, 20), 0.0);
    assert!(nz.sum() > eo.sum() + 10.0);
}

#[test]
fn pentagram_nonzero_area_is_the_pentagon_plus_points() {
    // The non zero area of a pentagram is the area of its hull of five points minus nothing inside
    // the star's outline; it equals the exact polygon area computed by the independent reference
    // within the stated bound.
    let pts: Vec<(f64, f64)> = (0..5)
        .map(|k| {
            let a = std::f64::consts::PI * 2.0 * (k as f64) / 5.0 - std::f64::consts::FRAC_PI_2;
            (20.0 + 18.0 * a.cos(), 20.0 + 18.0 * a.sin())
        })
        .collect();
    let ring: Vec<(f64, f64)> = [0, 2, 4, 1, 3].iter().map(|&k| pts[k]).collect();
    let p = Polygon::from_ring(ring);
    let got = one(p.clone(), 40, 40, &exact());
    let want = brute(&p, 40, 40, 16, false);
    let total_bound: f64 = (0..40)
        .flat_map(|j| (0..40).map(move |i| (i, j)))
        .map(|(i, j)| brute_bound(&p, i, j, 16))
        .sum();
    let diff: f64 = got
        .data
        .iter()
        .zip(want.iter())
        .map(|(a, b)| (a - b).abs())
        .sum();
    assert!(diff <= total_bound, "{diff} > {total_bound}");
}

#[test]
fn edge_crossing_pixel_corners_and_integer_lines() {
    let got = one(tri((0.0, 0.0), (3.0, 3.0), (0.0, 3.0)), 3, 3, &exact());
    assert_rows(
        &got,
        &[&[0.5, 0.0, 0.0], &[1.0, 0.5, 0.0], &[1.0, 1.0, 0.5]],
    );
}

#[test]
fn thin_sliver_has_small_exact_area() {
    let got = one(rect(0.0, 0.0, 1.0, 0.0625), 1, 1, &exact());
    assert_eq!(got.at(0, 0), 0.0625);
}

#[test]
fn shared_edge_polygons_sum_to_one_under_add() {
    let mut c = exact();
    c.combine = Combine::AddClamp;
    let got = run(
        &[rect(0.0, 0.0, 0.5, 1.0), rect(0.5, 0.0, 1.0, 1.0)],
        1,
        1,
        &c,
    );
    assert_eq!(got.at(0, 0), 1.0);
}

#[test]
fn star_of_david_like_overlap_clamps_to_one() {
    let got = one(
        Polygon(vec![
            vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)],
            vec![(1.0, 1.0), (3.0, 1.0), (3.0, 3.0), (1.0, 3.0)],
        ]),
        3,
        3,
        &exact(),
    );
    assert!(got.data.iter().all(|&v| (0.0..=1.0).contains(&v)));
    assert_eq!(got.at(1, 1), 1.0);
    assert_eq!(got.sum(), 7.0);
}

#[test]
fn clamp_free_values_stay_in_unit_range_for_random_triangles() {
    let mut r = Lcg(3);
    for _ in 0..200 {
        let p = tri(
            (r.eighth(), r.eighth()),
            (r.eighth(), r.eighth()),
            (r.eighth(), r.eighth()),
        );
        let got = one(p, 8, 8, &exact());
        assert!(got.data.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }
}

#[test]
fn random_triangle_sums_match_shoelace_within_a_tight_bound() {
    let mut r = Lcg(11);
    for _ in 0..200 {
        let p = tri(
            (r.eighth(), r.eighth()),
            (r.eighth(), r.eighth()),
            (r.eighth(), r.eighth()),
        );
        let got = one(p.clone(), 8, 8, &exact());
        let want = shoelace(&p).abs();
        assert!((got.sum() - want).abs() <= 1e-12, "{} vs {want}", got.sum());
    }
}
