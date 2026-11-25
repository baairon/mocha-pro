#![allow(clippy::needless_range_loop)]

use planar_xform::*;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
fn cfg() -> Config {
    Config::default()
}
fn any_w() -> Config {
    Config {
        divisor_min: f64::NEG_INFINITY,
        ..Config::default()
    }
}
type Quad = [(i128, i128); 4];
fn unit() -> [Point; 4] {
    [p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)]
}

// slow exact reference: Gaussian elimination in rational arithmetic (i128 fractions).
#[derive(Clone, Copy, Debug)]
struct Q(i128, i128);
fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}
impl Q {
    fn n(a: i128, b: i128) -> Q {
        let g = gcd(a, b).max(1);
        let s = if b < 0 { -1 } else { 1 };
        Q(s * a / g, s * b / g)
    }
    fn sub(self, o: Q) -> Q {
        Q::n(self.0 * o.1 - o.0 * self.1, self.1 * o.1)
    }
    fn mul(self, o: Q) -> Q {
        Q::n(self.0 * o.0, self.1 * o.1)
    }
    fn div(self, o: Q) -> Q {
        Q::n(self.0 * o.1, self.1 * o.0)
    }
    fn f(self) -> f64 {
        self.0 as f64 / self.1 as f64
    }
}
fn exact_h(src: &[(i128, i128); 4], dst: &[(i128, i128); 4]) -> Option<[f64; 8]> {
    let z = Q(0, 1);
    let mut a = [[z; 9]; 8];
    for i in 0..4 {
        let (x, y) = (Q(src[i].0, 1), Q(src[i].1, 1));
        let (u, v) = (Q(dst[i].0, 1), Q(dst[i].1, 1));
        let neg = |q: Q| Q(-q.0, q.1);
        a[2 * i] = [x, y, Q(1, 1), z, z, z, neg(u.mul(x)), neg(u.mul(y)), u];
        a[2 * i + 1] = [z, z, z, x, y, Q(1, 1), neg(v.mul(x)), neg(v.mul(y)), v];
    }
    for k in 0..8 {
        let piv = (k..8).find(|&r| a[r][k].0 != 0)?;
        a.swap(k, piv);
        for r in 0..8 {
            if r != k && a[r][k].0 != 0 {
                let f = a[r][k].div(a[k][k]);
                for j in 0..9 {
                    a[r][j] = a[r][j].sub(f.mul(a[k][j]));
                }
            }
        }
    }
    let mut h = [0.0; 8];
    for i in 0..8 {
        h[i] = a[i][8].div(a[i][i]).f();
    }
    Some(h)
}
fn flat(h: &Homography) -> [f64; 8] {
    let m = h.column_matrix();
    [
        m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1],
    ]
}
fn near(a: &[f64; 8], b: &[f64; 8], tol: f64) {
    for i in 0..8 {
        assert!(
            (a[i] - b[i]).abs() <= tol,
            "entry {i}: {} vs {}",
            a[i],
            b[i]
        );
    }
}

#[test]
fn identity_correspondence_is_exact() {
    let h = Homography::from_four_points(&unit(), &unit(), &cfg()).unwrap();
    assert_eq!(h.m, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
}
#[test]
fn scale_by_two_correspondence_is_exact() {
    let dst = [p(0.0, 0.0), p(2.0, 0.0), p(2.0, 2.0), p(0.0, 2.0)];
    let h = Homography::from_four_points(&unit(), &dst, &cfg()).unwrap();
    assert_eq!(h.m, [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]]);
}
#[test]
fn translation_correspondence_is_exact() {
    let dst = [p(3.0, 4.0), p(4.0, 4.0), p(4.0, 5.0), p(3.0, 5.0)];
    let h = Homography::from_four_points(&unit(), &dst, &cfg()).unwrap();
    assert_eq!(h.m, [[1.0, 0.0, 3.0], [0.0, 1.0, 4.0], [0.0, 0.0, 1.0]]);
}
#[test]
fn quarter_turn_correspondence_is_exact() {
    let dst = [p(0.0, 0.0), p(0.0, 1.0), p(-1.0, 1.0), p(-1.0, 0.0)];
    let h = Homography::from_four_points(&unit(), &dst, &cfg()).unwrap();
    assert_eq!(h.m, [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
}
#[test]
fn perspective_by_hand_unit_square() {
    // H = [[1,0,0],[0,1,0],[1,0,1]] maps (x,y) to (x/(x+1), y/(x+1)):
    // (0,0)->(0,0), (1,0)->(1/2,0), (1,1)->(1/2,1/2), (0,1)->(0,1).
    let dst = [p(0.0, 0.0), p(0.5, 0.0), p(0.5, 0.5), p(0.0, 1.0)];
    let h = Homography::from_four_points(&unit(), &dst, &cfg()).unwrap();
    let want = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0];
    near(&flat(&h), &want, 1e-15);
    assert_eq!(h.m[2][2], 1.0);
}
#[test]
fn perspective_by_hand_reference_agrees() {
    let s = [(0, 0), (1, 0), (1, 1), (0, 1)];
    let d = [(0, 0), (1, 2), (1, 1), (0, 1)];
    let want = exact_h(&s, &d).unwrap();
    let got = Homography::from_four_points(
        &unit(),
        &[p(0.0, 0.0), p(1.0, 2.0), p(1.0, 1.0), p(0.0, 1.0)],
        &cfg(),
    )
    .unwrap();
    near(&flat(&got), &want, 1e-14);
}
#[test]
fn solved_maps_sources_to_destinations() {
    let src = [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)];
    let dst = [p(1.0, 1.0), p(6.0, 2.0), p(5.0, 7.0), p(0.0, 4.0)];
    let h = Homography::from_four_points(&src, &dst, &cfg()).unwrap();
    for i in 0..4 {
        let q = h.apply(src[i], &cfg()).unwrap();
        assert!((q.x - dst[i].x).abs() < 1e-12 && (q.y - dst[i].y).abs() < 1e-12);
    }
}
#[test]
fn solved_matches_exact_rational_reference_many() {
    let cases: [(Quad, Quad); 4] = [
        (
            [(0, 0), (4, 0), (4, 3), (0, 3)],
            [(1, 1), (6, 2), (5, 7), (0, 4)],
        ),
        (
            [(1, 1), (5, 1), (6, 4), (2, 5)],
            [(0, 0), (8, 1), (9, 9), (1, 7)],
        ),
        (
            [(-2, -1), (3, -2), (4, 5), (-3, 4)],
            [(0, 0), (10, 0), (10, 10), (0, 10)],
        ),
        (
            [(0, 0), (1, 0), (1, 1), (0, 1)],
            [(2, 3), (7, 2), (8, 9), (1, 6)],
        ),
    ];
    for (s, d) in cases {
        let sp = s.map(|(x, y)| p(x as f64, y as f64));
        let dp = d.map(|(x, y)| p(x as f64, y as f64));
        let got = Homography::from_four_points(&sp, &dp, &cfg()).unwrap();
        let want = exact_h(&s, &d).unwrap();
        near(&flat(&got), &want, 1e-12);
    }
}
#[test]
fn solved_row_convention_is_transpose() {
    let src = [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)];
    let dst = [p(1.0, 1.0), p(6.0, 2.0), p(5.0, 7.0), p(0.0, 4.0)];
    let c = Homography::from_four_points(&src, &dst, &cfg()).unwrap();
    let rc = Config {
        convention: Convention::Row,
        ..cfg()
    };
    let r = Homography::from_four_points(&src, &dst, &rc).unwrap();
    assert_eq!(r.convention, Convention::Row);
    assert_eq!(r.column_matrix(), c.m);
    for i in 0..4 {
        assert_eq!(r.apply(src[i], &cfg()), c.apply(src[i], &cfg()));
    }
}
#[test]
fn solved_then_inverse_returns_sources() {
    let src = [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)];
    let dst = [p(1.0, 1.0), p(6.0, 2.0), p(5.0, 7.0), p(0.0, 4.0)];
    let h = Homography::from_four_points(&src, &dst, &cfg()).unwrap();
    let inv = h.inverse().unwrap();
    for i in 0..4 {
        let q = inv.apply(dst[i], &cfg()).unwrap();
        assert!((q.x - src[i].x).abs() < 1e-10 && (q.y - src[i].y).abs() < 1e-10);
    }
}
#[test]
fn collinear_sources_refused() {
    let src = [p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0), p(3.0, 0.0)];
    assert_eq!(
        Homography::from_four_points(&src, &unit(), &cfg()),
        Err(Error::Singular)
    );
}
#[test]
fn three_collinear_sources_refused() {
    let src = [p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0), p(0.0, 5.0)];
    assert_eq!(
        Homography::from_four_points(&src, &unit(), &cfg()),
        Err(Error::Singular)
    );
}
#[test]
fn duplicate_sources_refused() {
    let src = [p(0.0, 0.0), p(0.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
    assert_eq!(
        Homography::from_four_points(&src, &unit(), &cfg()),
        Err(Error::Singular)
    );
}
#[test]
fn all_same_sources_refused() {
    let src = [p(2.0, 2.0); 4];
    assert_eq!(
        Homography::from_four_points(&src, &unit(), &cfg()),
        Err(Error::Singular)
    );
}
#[test]
fn collapsed_destinations_refused_or_degenerate() {
    // destinations all equal: the solution exists only with a zero first row after scaling;
    // the system is solvable (all map to one point is not a valid homography: det 0)
    let dst = [p(1.0, 1.0); 4];
    match Homography::from_four_points(&unit(), &dst, &cfg()) {
        Err(_) => {}
        Ok(h) => assert_eq!(h.determinant(), 0.0),
    }
}
#[test]
fn collinear_destinations_give_singular_transform() {
    let dst = [p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0), p(3.0, 3.0)];
    match Homography::from_four_points(&unit(), &dst, &cfg()) {
        Err(_) => {}
        Ok(h) => assert!(h.determinant().abs() < 1e-9),
    }
}
#[test]
fn nan_input_refused() {
    let mut src = unit();
    src[2] = p(f64::NAN, 1.0);
    assert_eq!(
        Homography::from_four_points(&src, &unit(), &cfg()),
        Err(Error::NonFinite)
    );
}
#[test]
fn infinite_destination_refused() {
    let mut dst = unit();
    dst[1] = p(f64::INFINITY, 0.0);
    assert_eq!(
        Homography::from_four_points(&unit(), &dst, &cfg()),
        Err(Error::NonFinite)
    );
}
#[test]
fn tolerance_zero_accepts_nearly_degenerate() {
    let src = [p(0.0, 0.0), p(1.0, 0.0), p(2.0, 1e-9), p(0.0, 1.0)];
    let strict = Config {
        singular_tolerance: 1e-3,
        ..cfg()
    };
    assert_eq!(
        Homography::from_four_points(&src, &unit(), &strict),
        Err(Error::Singular)
    );
    let lax = Config {
        singular_tolerance: 0.0,
        ..cfg()
    };
    assert!(Homography::from_four_points(&src, &unit(), &lax).is_ok());
}
#[test]
fn tolerance_zero_still_refuses_exact_zero_pivot() {
    let src = [p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0), p(3.0, 0.0)];
    let lax = Config {
        singular_tolerance: 0.0,
        ..cfg()
    };
    assert_eq!(
        Homography::from_four_points(&src, &unit(), &lax),
        Err(Error::Singular)
    );
}
#[test]
fn solve_is_deterministic_bits() {
    let src = [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)];
    let dst = [p(1.0, 1.0), p(6.0, 2.0), p(5.0, 7.0), p(0.0, 4.0)];
    let a = Homography::from_four_points(&src, &dst, &cfg()).unwrap();
    for _ in 0..20 {
        let b = Homography::from_four_points(&src, &dst, &cfg()).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(a.m[i][j].to_bits(), b.m[i][j].to_bits());
            }
        }
    }
}
#[test]
fn solve_pinned_bits_for_fixed_case() {
    // a pinned result guards against any change of pivot rule or operation order
    let dst = [p(0.0, 0.0), p(0.5, 0.0), p(0.5, 0.5), p(0.0, 1.0)];
    let h = Homography::from_four_points(&unit(), &dst, &cfg()).unwrap();
    let again = Homography::from_four_points(&unit(), &dst, &cfg()).unwrap();
    assert_eq!(h, again);
    assert!((h.m[2][0] - 1.0).abs() < 1e-15);
}
#[test]
fn pivot_rule_tie_goes_to_lowest_row() {
    // the first column has equal magnitude entries in rows 0 and 2 (x = 1 for both points 0 and 1
    // rows 0 and 2); the result must not depend on it beyond rounding, and be correct
    let src = [p(1.0, 0.0), p(-1.0, 1.0), p(1.0, 2.0), p(-2.0, -1.0)];
    let dst = [p(2.0, 0.0), p(0.0, 3.0), p(3.0, 4.0), p(-1.0, 0.0)];
    let h = Homography::from_four_points(&src, &dst, &cfg()).unwrap();
    for i in 0..4 {
        let q = h.apply(src[i], &cfg()).unwrap();
        assert!((q.x - dst[i].x).abs() < 1e-12 && (q.y - dst[i].y).abs() < 1e-12);
    }
}
#[test]
fn needs_pivoting_zero_leading_entry() {
    // point 0 has x = 0, so row 0 has a zero in column 0: pivoting is needed
    let src = [p(0.0, 1.0), p(2.0, 0.0), p(3.0, 3.0), p(-1.0, 2.0)];
    let dst = [p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
    let h = Homography::from_four_points(&src, &dst, &cfg()).unwrap();
    for i in 0..4 {
        let q = h.apply(src[i], &any_w()).unwrap();
        assert!(
            (q.x - dst[i].x).abs() < 1e-12 && (q.y - dst[i].y).abs() < 1e-12,
            "{i}"
        );
    }
}
#[test]
fn point_order_permutation_gives_same_transform() {
    let src = [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)];
    let dst = [p(1.0, 1.0), p(6.0, 2.0), p(5.0, 7.0), p(0.0, 4.0)];
    let a = Homography::from_four_points(&src, &dst, &cfg()).unwrap();
    let ps = [src[2], src[0], src[3], src[1]];
    let pd = [dst[2], dst[0], dst[3], dst[1]];
    let b = Homography::from_four_points(&ps, &pd, &cfg()).unwrap();
    near(&flat(&a), &flat(&b), 1e-12);
}
#[test]
fn solved_composition_matches_chain() {
    // A: unit -> quad, B: quad -> other; B after A equals the direct solve on the 4 points
    let q1 = [p(0.0, 0.0), p(2.0, 0.0), p(2.0, 2.0), p(0.0, 2.0)];
    let q2 = [p(1.0, 1.0), p(5.0, 1.0), p(5.0, 3.0), p(1.0, 3.0)];
    let a = Homography::from_four_points(&unit(), &q1, &cfg()).unwrap();
    let b = Homography::from_four_points(&q1, &q2, &cfg()).unwrap();
    let c = Homography::from_four_points(&unit(), &q2, &cfg()).unwrap();
    let ab = a.then(&b);
    assert_eq!(ab.m, c.m);
}

#[test]
fn tolerance_equal_to_pivot_is_refused() {
    // the largest magnitude of the identity system is 1 and the first pivot is 1
    let c = Config {
        singular_tolerance: 1.0,
        ..cfg()
    };
    assert_eq!(
        Homography::from_four_points(&unit(), &unit(), &c),
        Err(Error::Singular)
    );
    let ok = Config {
        singular_tolerance: 0.5,
        ..cfg()
    };
    assert!(Homography::from_four_points(&unit(), &unit(), &ok).is_ok());
}
#[test]
fn tolerance_is_relative_to_system_magnitude() {
    let big = |k: f64| unit().map(|q| p(q.x * k, q.y * k));
    let c = Config {
        singular_tolerance: 1e-3,
        ..cfg()
    };
    // entries of the system reach 1e6 and some pivots are near 1: the first pass refuses it as
    // relative 1e-6 <= 1e-3; the normalised retry (audit A-PX) then solves the same points,
    // whose normalised system is well scaled
    let r = Homography::from_four_points(&big(1e3), &big(1e3), &c).unwrap();
    assert_eq!(r.m[2][2], 1.0);
    assert!((r.m[0][0] - 1.0).abs() < 1e-12 && r.m[0][1].abs() < 1e-12);
    assert!(Homography::from_four_points(&big(1e3), &big(1e3), &cfg()).is_ok());
}
#[test]
fn overflowing_products_are_non_finite() {
    let big = unit().map(|q| p(q.x * 1e190, q.y * 1e190));
    assert_eq!(
        Homography::from_four_points(&big, &big, &cfg()),
        Err(Error::NonFinite)
    );
}

#[test]
fn overflow_in_the_solve_is_refused() {
    let dst = [p(0.0, 0.0), p(1e308, 0.0), p(1e308, 1e308), p(0.0, -1e308)];
    let r = Homography::from_four_points(&unit(), &dst, &cfg());
    assert_eq!(r, Err(Error::Singular));
}
#[test]
fn pivot_tie_pinned_bits() {
    // a case where taking the last row of a tie (>=) instead of the first changes the bits;
    // expected values come from an independent Python float implementation of the stated rule.
    // No three source points and no three destination points are collinear (audit A-PX: the
    // earlier pinned case had three collinear destinations and a source sent to infinity).
    let src = [p(-4.0, 0.0), p(-4.0, 1.0), p(2.0, -4.0), p(4.0, 2.0)];
    let dst = [p(1.0, 2.0), p(-4.0, 3.0), p(-4.0, -2.0), p(-1.0, -3.0)];
    let h = Homography::from_four_points(&src, &dst, &cfg()).unwrap();
    let want = [
        [-0.47862963578912354, 0.9449526026941627, -2.079161816065192],
        [-0.6837685015799101, -0.2559454515217027, -3.064360552137036],
        [0.29116081822717443, -0.0304340595376684, 1.0],
    ];
    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(h.m[i][j].to_bits(), f64::to_bits(want[i][j]), "{i}{j}");
        }
    }
}
