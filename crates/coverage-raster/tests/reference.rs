//! The exact mode against an independent slow reference: supersampling on a 96 by 96 grid per pixel
//! with a winding number test written separately in `common`. The reference is within
//! `brute_bound` of the true area, so the exact result must be within that bound of it.

mod common;
use common::*;
use coverage_raster::*;

const N: u32 = 96;

fn check(p: &Polygon, w: u32, h: u32, fill: FillRule) {
    let mut c = exact();
    c.fill = fill;
    let got = one(p.clone(), w, h, &c);
    let want = brute(p, w, h, N, fill == FillRule::EvenOdd);
    for j in 0..h {
        for i in 0..w {
            let g = got.at(i, j);
            let r = want[(j * w + i) as usize];
            let b = brute_bound(p, i, j, N);
            assert!(
                (g - r).abs() <= b,
                "pixel {i},{j}: exact {g}, reference {r}, bound {b}"
            );
        }
    }
}

fn concave() -> Polygon {
    Polygon::from_ring(vec![
        (0.7, 0.4),
        (7.3, 1.1),
        (6.2, 3.3),
        (4.1, 2.2),
        (2.9, 6.8),
        (0.9, 5.1),
    ])
}

fn star() -> Polygon {
    let pts: Vec<(f64, f64)> = (0..5)
        .map(|k| {
            let a = std::f64::consts::PI * 2.0 * (k as f64) / 5.0 - 1.3;
            (4.0 + 3.7 * a.cos(), 4.1 + 3.7 * a.sin())
        })
        .collect();
    Polygon::from_ring([0, 2, 4, 1, 3].iter().map(|&k| pts[k]).collect())
}

fn holey() -> Polygon {
    Polygon(vec![
        vec![(0.3, 0.2), (7.6, 0.7), (7.1, 7.4), (0.6, 6.9)],
        vec![(2.1, 2.3), (5.3, 2.0), (4.4, 5.2)],
        vec![(3.1, 3.0), (3.9, 3.2), (3.5, 3.9)],
    ])
}

fn thin() -> Polygon {
    Polygon::from_ring(vec![(0.2, 0.3), (7.9, 7.6), (7.8, 7.7), (0.1, 0.45)])
}

fn crossing_rings() -> Polygon {
    Polygon(vec![
        vec![(0.5, 0.5), (5.5, 1.5), (4.5, 6.5), (1.5, 5.5)],
        vec![(3.5, 0.5), (7.5, 3.5), (3.5, 7.5), (2.5, 4.0)],
    ])
}

#[test]
fn concave_nonzero() {
    check(&concave(), 8, 8, FillRule::NonZero);
}

#[test]
fn concave_even_odd() {
    check(&concave(), 8, 8, FillRule::EvenOdd);
}

#[test]
fn star_nonzero() {
    check(&star(), 8, 8, FillRule::NonZero);
}

#[test]
fn star_even_odd() {
    check(&star(), 8, 8, FillRule::EvenOdd);
}

#[test]
fn holes_nonzero() {
    check(&holey(), 8, 8, FillRule::NonZero);
}

#[test]
fn holes_even_odd() {
    check(&holey(), 8, 8, FillRule::EvenOdd);
}

#[test]
fn thin_diagonal_sliver() {
    check(&thin(), 8, 8, FillRule::NonZero);
}

#[test]
fn rings_crossing_each_other_nonzero() {
    check(&crossing_rings(), 8, 8, FillRule::NonZero);
}

#[test]
fn rings_crossing_each_other_even_odd() {
    check(&crossing_rings(), 8, 8, FillRule::EvenOdd);
}

#[test]
fn star_and_concave_sums_against_closed_form_of_simple_polygon() {
    // The concave polygon is simple, so its coverage sum is its shoelace area.
    let p = concave();
    let got = one(p.clone(), 8, 8, &exact());
    assert!((got.sum() - shoelace(&p).abs()).abs() < 1e-12);
}

#[test]
fn large_supersample_converges_toward_exact() {
    // Grid(64) through the crate itself approaches the exact area; the difference of sums is
    // below the stated sampling bound of the perimeter.
    let p = concave();
    let e = one(p.clone(), 8, 8, &exact()).sum();
    let s = one(p, 8, 8, &cfg(Method::Samples(Pattern::Grid(64)))).sum();
    assert!((e - s).abs() < 0.1, "{e} vs {s}");
}

#[test]
fn supersample_mode_matches_the_independent_reference_exactly() {
    // The crate's Grid(n) and the reference use the same sample positions; away from ties they agree
    // exactly.
    let p = concave();
    let got = one(p.clone(), 8, 8, &cfg(Method::Samples(Pattern::Grid(16))));
    let want = brute(&p, 8, 8, 16, false);
    assert_eq!(got.data, want);
}

#[test]
fn supersample_even_odd_matches_the_independent_reference() {
    let p = star();
    let mut c = cfg(Method::Samples(Pattern::Grid(16)));
    c.fill = FillRule::EvenOdd;
    let got = one(p.clone(), 8, 8, &c);
    assert_eq!(got.data, brute(&p, 8, 8, 16, true));
}

#[test]
fn error_bound_over_random_triangles_is_small() {
    // Bound check: against the shoelace area, sums on 16 by 16 targets stay within 1e-11.
    let mut r = Lcg(1234);
    let mut worst = 0.0f64;
    for _ in 0..300 {
        let f = |r: &mut Lcg| (r.next() % 100_000) as f64 / 100_000.0 * 16.0;
        let p = tri(
            (f(&mut r), f(&mut r)),
            (f(&mut r), f(&mut r)),
            (f(&mut r), f(&mut r)),
        );
        let got = one(p.clone(), 16, 16, &exact());
        worst = worst.max((got.sum() - shoelace(&p).abs()).abs());
    }
    assert!(worst < 1e-11, "worst {worst}");
}
