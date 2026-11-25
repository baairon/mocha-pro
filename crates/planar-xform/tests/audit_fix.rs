//! Regression tests from the independent audit A-PX: each case failed on the first version.
use planar_xform::*;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn close(a: Point, b: Point, tol: f64) -> bool {
    (a.x - b.x).abs() <= tol * (1.0 + b.x.abs()) && (a.y - b.y).abs() <= tol * (1.0 + b.y.abs())
}

#[test]
fn offset_source_and_destination_points_are_solved() {
    // Exact rational solution exists (checked by an independent Python Fraction solver).
    let cfg = Config::default();
    let s = [
        p(999990.0, 1000008.0),
        p(1000001.0, 1000002.0),
        p(1000001.0, 999993.0),
        p(1000009.0, 1000002.0),
    ];
    let d = [
        p(999995.0, 1000007.0),
        p(1000002.0, 999991.0),
        p(1000008.0, 1000004.0),
        p(1000002.0, 999998.0),
    ];
    let h = Homography::from_four_points(&s, &d, &cfg).expect("well posed correspondences");
    for i in 0..4 {
        // the divisor may be negative for some points of a correspondence set, so no guard
        let [x, y, w] = h.apply_homogeneous(s[i]);
        assert!(w != 0.0);
        assert!(
            close(p(x / w, y / w), d[i], 1e-9),
            "{:?} vs {:?}",
            (x / w, y / w),
            d[i]
        );
    }
}

#[test]
fn offset_destination_points_are_solved() {
    let cfg = Config::default();
    let off = 1.0e7;
    let s = [p(0.0, 0.0), p(4.0, 0.0), p(5.0, 3.0), p(0.0, 3.0)];
    let d = [
        p(off, off),
        p(off + 8.0, off),
        p(off + 9.0, off + 6.0),
        p(off, off + 6.0),
    ];
    let h = Homography::from_four_points(&s, &d, &cfg).expect("well posed correspondences");
    for i in 0..4 {
        let q = h.apply(s[i], &cfg).unwrap();
        assert!(close(q, d[i], 1e-6), "{:?} vs {:?}", q, d[i]);
    }
}

#[test]
fn tiny_unit_square_is_solved() {
    let cfg = Config::default();
    let k = 1e-9;
    let s = [p(0.0, 0.0), p(k, 0.0), p(k, k), p(0.0, k)];
    let d = [
        p(0.0, 0.0),
        p(2.0 * k, 0.0),
        p(2.0 * k, 2.0 * k),
        p(0.0, 2.0 * k),
    ];
    let h = Homography::from_four_points(&s, &d, &cfg).expect("scale by two");
    let q = h.apply(p(0.5 * k, 0.25 * k), &cfg).unwrap();
    assert!(
        (q.x - k).abs() < 1e-15 && (q.y - 0.5 * k).abs() < 1e-15,
        "{:?}",
        q
    );
}

#[test]
fn origin_at_infinity_is_solved_and_scaled_by_largest_entry() {
    let cfg = Config::default();
    let m = [[1.0, 0.0, 1.0], [0.0, 1.0, 1.0], [1.0, 1.0, 0.0]];
    let truth = Homography::from_matrix(m, Convention::Column);
    let s = [p(1.0, 0.0), p(0.0, 2.0), p(2.0, 3.0), p(-1.0, 2.0)];
    let d = s.map(|q| truth.apply(q, &cfg).unwrap());
    let h = Homography::from_four_points(&s, &d, &cfg).expect("a valid correspondence set");
    for i in 0..4 {
        assert!(close(h.apply(s[i], &cfg).unwrap(), d[i], 1e-9));
    }
    assert!(h.m[2][2].abs() < 1e-9);
}

#[test]
fn collinear_sources_with_generic_destinations_are_refused() {
    let cfg = Config::default();
    // first three sources on the line y = 7 - x... (8,-13), (-19,-12), (62,-15) are collinear
    let s = [p(8.0, -13.0), p(-19.0, -12.0), p(62.0, -15.0), p(-2.0, 7.0)];
    let d = [p(-7.0, -13.0), p(17.0, 12.0), p(-17.0, 12.0), p(2.0, 17.0)];
    assert_eq!(
        Homography::from_four_points(&s, &d, &cfg),
        Err(Error::Singular)
    );
    let s = [p(-1.0, 9.0), p(4.0, 3.0), p(9.0, -3.0), p(15.0, -8.0)];
    let d = [
        p(-8.0, -13.0),
        p(-3.0, -8.0),
        p(-12.0, 18.0),
        p(-14.0, -6.0),
    ];
    assert_eq!(
        Homography::from_four_points(&s, &d, &cfg),
        Err(Error::Singular)
    );
}

#[test]
fn inverse_with_overflowing_entries_is_non_finite() {
    let t = Homography::from_matrix(
        [[1e190, 0.0, 0.0], [0.0, 1e190, 0.0], [0.0, 0.0, 1e-190]],
        Convention::Column,
    );
    assert_eq!(t.inverse(), Err(Error::NonFinite));
}

#[test]
fn inverse_of_tiny_and_huge_uniform_scales_exists() {
    for (k, back) in [(1e-160, 1e160), (1e160, 1e-160), (1e-190, 1e190)] {
        let t = Homography::from_matrix(
            [[k, 0.0, 0.0], [0.0, k, 0.0], [0.0, 0.0, k]],
            Convention::Column,
        );
        let inv = t.inverse().expect("a nonsingular matrix");
        for i in 0..3 {
            assert!((inv.m[i][i] / back - 1.0).abs() < 1e-12, "{:?}", inv.m);
        }
    }
    let t = Homography::from_matrix(
        [[1e-171, 0.0, 0.0], [0.0, 1e-171, 0.0], [0.0, 0.0, 1e-171]],
        Convention::Row,
    );
    let inv = t.inverse().expect("nonsingular");
    assert_eq!(inv.convention, Convention::Row);
    assert!((inv.m[0][0] / 1e171 - 1.0).abs() < 1e-12 && inv.m[0][1] == 0.0);
}

#[test]
fn true_singular_matrix_stays_singular() {
    let t = Homography::from_matrix(
        [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 0.0, 1.0]],
        Convention::Column,
    );
    assert_eq!(t.inverse(), Err(Error::Singular));
    let z = Homography::from_matrix([[0.0; 3]; 3], Convention::Column);
    assert_eq!(z.inverse(), Err(Error::Singular));
}
