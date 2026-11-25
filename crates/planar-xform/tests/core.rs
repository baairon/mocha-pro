#![allow(clippy::needless_range_loop)]

use planar_xform::*;

const C: Convention = Convention::Column;
const R: Convention = Convention::Row;

fn cfg() -> Config {
    Config::default()
}
fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
fn bits(m: &[[f64; 3]; 3]) -> [[u64; 3]; 3] {
    let mut o = [[0u64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            o[i][j] = m[i][j].to_bits();
        }
    }
    o
}
const ID: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

#[test]
fn identity_matrix_column() {
    assert_eq!(Homography::identity(C).m, ID);
}
#[test]
fn identity_matrix_row() {
    assert_eq!(Homography::identity(R).m, ID);
}
#[test]
fn identity_applies_exactly() {
    let h = Homography::identity(C);
    assert_eq!(h.apply(p(3.25, -7.5), &cfg()), Some(p(3.25, -7.5)));
}
#[test]
fn from_matrix_keeps_entries() {
    let m = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
    let h = Homography::from_matrix(m, R);
    assert_eq!(h.m, m);
    assert_eq!(h.convention, R);
}
#[test]
fn translation_column_layout() {
    let h = Homography::translation(C, 3.0, -2.0);
    assert_eq!(h.m, [[1.0, 0.0, 3.0], [0.0, 1.0, -2.0], [0.0, 0.0, 1.0]]);
}
#[test]
fn translation_row_layout_is_transpose() {
    let h = Homography::translation(R, 3.0, -2.0);
    assert_eq!(h.m, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [3.0, -2.0, 1.0]]);
}
#[test]
fn translation_applies_both_conventions() {
    for c in [C, R] {
        let h = Homography::translation(c, 3.0, -2.0);
        assert_eq!(h.apply(p(1.0, 1.0), &cfg()), Some(p(4.0, -1.0)));
    }
}
#[test]
fn scale_by_two() {
    for c in [C, R] {
        let h = Homography::scale(c, 2.0, 2.0);
        assert_eq!(h.apply(p(1.5, -4.0), &cfg()), Some(p(3.0, -8.0)));
    }
}
#[test]
fn scale_anisotropic() {
    let h = Homography::scale(C, 2.0, 0.5);
    assert_eq!(h.m, [[2.0, 0.0, 0.0], [0.0, 0.5, 0.0], [0.0, 0.0, 1.0]]);
    assert_eq!(h.apply(p(3.0, 3.0), &cfg()), Some(p(6.0, 1.5)));
}
#[test]
fn quarter_turn_one_column_matrix() {
    let h = Homography::rotation_quarter_turns(C, 1);
    assert_eq!(h.m, [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
}
#[test]
fn quarter_turn_one_maps_x_axis_to_y_axis() {
    for c in [C, R] {
        let h = Homography::rotation_quarter_turns(c, 1);
        assert_eq!(h.apply(p(1.0, 0.0), &cfg()), Some(p(0.0, 1.0)));
        assert_eq!(h.apply(p(0.0, 1.0), &cfg()), Some(p(-1.0, 0.0)));
    }
}
#[test]
fn quarter_turn_two() {
    let h = Homography::rotation_quarter_turns(C, 2);
    assert_eq!(h.apply(p(2.0, 3.0), &cfg()), Some(p(-2.0, -3.0)));
}
#[test]
fn quarter_turn_three() {
    let h = Homography::rotation_quarter_turns(C, 3);
    assert_eq!(h.apply(p(1.0, 0.0), &cfg()), Some(p(0.0, -1.0)));
}
#[test]
fn quarter_turn_zero_is_identity() {
    assert_eq!(Homography::rotation_quarter_turns(C, 0).m, ID);
}
#[test]
fn quarter_turn_wraps_modulo_four() {
    for t in -9..=9 {
        let a = Homography::rotation_quarter_turns(C, t);
        let b = Homography::rotation_quarter_turns(C, t.rem_euclid(4));
        assert_eq!(bits(&a.m), bits(&b.m));
    }
}
#[test]
fn quarter_turn_negative_one_equals_three() {
    assert_eq!(
        Homography::rotation_quarter_turns(C, -1).m,
        Homography::rotation_quarter_turns(C, 3).m
    );
}
#[test]
fn four_quarter_turns_compose_to_identity_exactly() {
    let q = Homography::rotation_quarter_turns(C, 1);
    let h = q.then(&q).then(&q).then(&q);
    assert_eq!(h.m, ID);
}
#[test]
fn quarter_turns_add() {
    let q1 = Homography::rotation_quarter_turns(C, 1);
    let q2 = Homography::rotation_quarter_turns(C, 2);
    assert_eq!(q1.then(&q2).m, Homography::rotation_quarter_turns(C, 3).m);
}
#[test]
fn cos_sin_given_by_caller_unnormalised() {
    let h = Homography::rotation_cos_sin(C, 3.0, 4.0);
    assert_eq!(h.m, [[3.0, -4.0, 0.0], [4.0, 3.0, 0.0], [0.0, 0.0, 1.0]]);
    assert_eq!(h.apply(p(1.0, 0.0), &cfg()), Some(p(3.0, 4.0)));
}
#[test]
fn cos_sin_row_layout() {
    let h = Homography::rotation_cos_sin(R, 3.0, 4.0);
    assert_eq!(h.m, [[3.0, 4.0, 0.0], [-4.0, 3.0, 0.0], [0.0, 0.0, 1.0]]);
}
#[test]
fn cos_sin_axis_values() {
    let h = Homography::rotation_cos_sin(C, 0.6, 0.8);
    let q = h.apply(p(5.0, 0.0), &cfg()).unwrap();
    assert!((q.x - 3.0).abs() < 1e-15 && (q.y - 4.0).abs() < 1e-15);
}
#[test]
fn compose_order_column_translate_then_scale() {
    // translate first, then scale: (1,1) -> (2,1) -> (4,2)
    let t = Homography::translation(C, 1.0, 0.0);
    let s = Homography::scale(C, 2.0, 2.0);
    let h = t.then(&s);
    assert_eq!(h.apply(p(1.0, 1.0), &cfg()), Some(p(4.0, 2.0)));
    assert_eq!(h.m, [[2.0, 0.0, 2.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]]);
}
#[test]
fn compose_order_column_scale_then_translate() {
    let t = Homography::translation(C, 1.0, 0.0);
    let s = Homography::scale(C, 2.0, 2.0);
    let h = s.then(&t);
    assert_eq!(h.apply(p(1.0, 1.0), &cfg()), Some(p(3.0, 2.0)));
}
#[test]
fn compose_order_row_translate_then_scale() {
    let t = Homography::translation(R, 1.0, 0.0);
    let s = Homography::scale(R, 2.0, 2.0);
    let h = t.then(&s);
    assert_eq!(h.apply(p(1.0, 1.0), &cfg()), Some(p(4.0, 2.0)));
    assert_eq!(h.m, [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [2.0, 0.0, 1.0]]);
}
#[test]
fn compose_row_and_column_agree_on_points() {
    let q = Homography::rotation_quarter_turns(C, 1);
    let t = Homography::translation(C, 5.0, -3.0);
    let a = q.then(&t);
    let b = q.with_convention(R).then(&t.with_convention(R));
    for pt in [p(0.0, 0.0), p(2.0, 7.0), p(-1.0, 4.0)] {
        assert_eq!(a.apply(pt, &cfg()), b.apply(pt, &cfg()));
    }
    assert_eq!(a.column_matrix(), b.column_matrix());
}
#[test]
fn compose_mixed_conventions_converts_next() {
    let a = Homography::translation(C, 1.0, 2.0);
    let b = Homography::scale(R, 2.0, 2.0);
    let h = a.then(&b);
    assert_eq!(h.convention, C);
    assert_eq!(h.apply(p(0.0, 0.0), &cfg()), Some(p(2.0, 4.0)));
}
#[test]
fn compose_with_identity_is_noop_bits() {
    let h = Homography::from_matrix([[1.5, 0.25, 3.0], [-2.0, 4.0, 0.5], [0.125, 0.0, 1.0]], C);
    let i = Homography::identity(C);
    assert_eq!(bits(&h.then(&i).m), bits(&h.m));
    assert_eq!(bits(&i.then(&h).m), bits(&h.m));
}
#[test]
fn compose_matrix_product_by_hand() {
    let a = Homography::from_matrix([[1.0, 2.0, 0.0], [3.0, 4.0, 0.0], [0.0, 0.0, 1.0]], C);
    let b = Homography::from_matrix([[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], C);
    // a.then(b) = b * a
    assert_eq!(
        a.then(&b).m,
        [[3.0, 4.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 1.0]]
    );
}
#[test]
fn with_convention_round_trip() {
    let h = Homography::from_matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]], C);
    assert_eq!(h.with_convention(C), h);
    let r = h.with_convention(R);
    assert_eq!(r.m, [[1.0, 4.0, 7.0], [2.0, 5.0, 8.0], [3.0, 6.0, 9.0]]);
    assert_eq!(r.with_convention(C), h);
    assert_eq!(r.column_matrix(), h.m);
}
#[test]
fn determinant_by_hand() {
    let h = Homography::from_matrix([[2.0, 0.0, 1.0], [1.0, 3.0, 2.0], [1.0, 1.0, 1.0]], C);
    // 2*(3-2) - 0 + 1*(1-3) = 0
    assert_eq!(h.determinant(), 0.0);
    let g = Homography::from_matrix([[2.0, 0.0, 1.0], [1.0, 3.0, 2.0], [1.0, 1.0, 2.0]], C);
    // 2*(6-2) - 0 + 1*(1-3) = 6
    assert_eq!(g.determinant(), 6.0);
}
#[test]
fn determinant_of_transpose_equal() {
    let h = Homography::from_matrix([[2.0, 0.0, 1.0], [1.0, 3.0, 2.0], [1.0, 1.0, 2.0]], C);
    assert_eq!(h.determinant(), h.with_convention(R).determinant());
}
#[test]
fn adjugate_by_hand() {
    let h = Homography::from_matrix([[2.0, 0.0, 1.0], [1.0, 3.0, 2.0], [1.0, 1.0, 2.0]], C);
    assert_eq!(
        h.adjugate().m,
        [[4.0, 1.0, -3.0], [0.0, 3.0, -3.0], [-2.0, -2.0, 6.0]]
    );
}
#[test]
fn adjugate_times_matrix_is_det_identity() {
    let h = Homography::from_matrix([[2.0, 0.0, 1.0], [1.0, 3.0, 2.0], [1.0, 1.0, 2.0]], C);
    let prod = h.then(&h.adjugate());
    assert_eq!(prod.m, [[6.0, 0.0, 0.0], [0.0, 6.0, 0.0], [0.0, 0.0, 6.0]]);
}
#[test]
fn inverse_of_dyadic_scale_translate_is_exact() {
    let h = Homography::scale(C, 2.0, 4.0).then(&Homography::translation(C, 3.0, -1.0));
    let inv = h.inverse().unwrap();
    assert_eq!(h.then(&inv).m, ID);
    assert_eq!(inv.then(&h).m, ID);
}
#[test]
fn inverse_of_quarter_turn_is_exact() {
    let q = Homography::rotation_quarter_turns(C, 1);
    assert_eq!(
        q.inverse().unwrap().m,
        Homography::rotation_quarter_turns(C, 3).m
    );
}
#[test]
fn inverse_dyadic_perspective_exact() {
    // det = 1, entries dyadic
    let h = Homography::from_matrix([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.5, 0.25, 1.0]], C);
    let inv = h.inverse().unwrap();
    assert_eq!(
        inv.m,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-0.5, -0.25, 1.0]]
    );
    assert_eq!(h.then(&inv).m, ID);
}
#[test]
fn inverse_det_two_dyadic() {
    let h = Homography::from_matrix([[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], C);
    let inv = h.inverse().unwrap();
    assert_eq!(inv.m, [[0.5, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
}
#[test]
fn inverse_row_convention() {
    let h = Homography::scale(R, 2.0, 4.0).then(&Homography::translation(R, 3.0, -1.0));
    let inv = h.inverse().unwrap();
    assert_eq!(inv.convention, R);
    assert_eq!(h.then(&inv).m, ID);
}
#[test]
fn inverse_singular_refused() {
    let h = Homography::from_matrix([[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 0.0, 1.0]], C);
    assert_eq!(h.inverse(), Err(Error::Singular));
}
#[test]
fn inverse_zero_matrix_refused() {
    let h = Homography::from_matrix([[0.0; 3]; 3], C);
    assert_eq!(h.inverse(), Err(Error::Singular));
}
#[test]
fn inverse_non_finite_refused() {
    let h = Homography::from_matrix([[f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], C);
    assert_eq!(h.inverse(), Err(Error::NonFinite));
    let g = Homography::from_matrix(
        [[f64::INFINITY, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        C,
    );
    assert_eq!(g.inverse(), Err(Error::NonFinite));
}
#[test]
fn inverse_involution_dyadic() {
    let h = Homography::from_matrix([[2.0, 0.0, 1.0], [0.0, 4.0, 2.0], [0.0, 0.0, 1.0]], C);
    assert_eq!(h.inverse().unwrap().inverse().unwrap().m, h.m);
}

// apply and guards
fn persp() -> Homography {
    // x' = x, y' = y, w = x + 1
    Homography::from_matrix([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 1.0]], C)
}
#[test]
fn apply_divides_by_w() {
    assert_eq!(persp().apply(p(1.0, 1.0), &cfg()), Some(p(0.5, 0.5)));
    assert_eq!(persp().apply(p(3.0, 2.0), &cfg()), Some(p(0.75, 0.5)));
}
#[test]
fn apply_homogeneous_values() {
    assert_eq!(persp().apply_homogeneous(p(3.0, 2.0)), [3.0, 2.0, 4.0]);
}
#[test]
fn apply_homogeneous_row_values() {
    let h = persp().with_convention(R);
    assert_eq!(h.apply_homogeneous(p(3.0, 2.0)), [3.0, 2.0, 4.0]);
}
#[test]
fn guard_zero_returns_none() {
    assert_eq!(persp().apply(p(-1.0, 1.0), &cfg()), None);
}
#[test]
fn guard_negative_returns_none() {
    assert_eq!(persp().apply(p(-3.0, 1.0), &cfg()), None);
}
#[test]
fn guard_nan_returns_none() {
    assert_eq!(persp().apply(p(f64::NAN, 1.0), &cfg()), None);
}
#[test]
fn guard_clamp_uses_value() {
    let c = Config {
        guard: DivisorGuard::Clamp(0.5),
        ..cfg()
    };
    assert_eq!(persp().apply(p(-3.0, 1.0), &c), Some(p(-6.0, 2.0)));
    assert_eq!(persp().apply(p(-1.0, 1.0), &c), Some(p(-2.0, 2.0)));
}
#[test]
fn guard_clamp_not_used_when_valid() {
    let c = Config {
        guard: DivisorGuard::Clamp(0.5),
        ..cfg()
    };
    assert_eq!(persp().apply(p(1.0, 1.0), &c), Some(p(0.5, 0.5)));
}
#[test]
fn guard_pass_through_returns_numerators() {
    let c = Config {
        guard: DivisorGuard::PassThrough,
        ..cfg()
    };
    assert_eq!(persp().apply(p(-3.0, 1.0), &c), Some(p(-3.0, 1.0)));
}
#[test]
fn guard_divisor_min_is_strict() {
    let c = Config {
        divisor_min: 2.0,
        ..cfg()
    };
    // w = 2 is not greater than 2
    assert_eq!(persp().apply(p(1.0, 1.0), &c), None);
    assert_eq!(persp().apply(p(2.0, 3.0), &c), Some(p(2.0 / 3.0, 1.0)));
}
#[test]
fn guard_divisor_min_negative_allows_negative_w() {
    let c = Config {
        divisor_min: -10.0,
        ..cfg()
    };
    assert_eq!(persp().apply(p(-3.0, 2.0), &c), Some(p(1.5, -1.0)));
}

// rectangles
#[test]
fn rect_corners_order() {
    let r = Rect::new(1.0, 2.0, 3.0, 5.0);
    assert_eq!(
        r.corners(),
        [p(1.0, 2.0), p(3.0, 2.0), p(3.0, 5.0), p(1.0, 5.0)]
    );
}
#[test]
fn rect_bounding_points() {
    let b = Rect::bounding(&[p(1.0, 5.0), p(-2.0, 3.0), p(4.0, -1.0)]).unwrap();
    assert_eq!(b, Rect::new(-2.0, -1.0, 4.0, 5.0));
}
#[test]
fn rect_bounding_empty_is_none() {
    assert_eq!(Rect::bounding(&[]), None);
}
#[test]
fn rect_bounding_single() {
    assert_eq!(
        Rect::bounding(&[p(2.0, 3.0)]),
        Some(Rect::new(2.0, 3.0, 2.0, 3.0))
    );
}
#[test]
fn map_rect_translation() {
    let h = Homography::translation(C, 10.0, 20.0);
    let q = h.map_rect(&Rect::new(0.0, 0.0, 2.0, 1.0), &cfg()).unwrap();
    assert_eq!(
        q,
        [p(10.0, 20.0), p(12.0, 20.0), p(12.0, 21.0), p(10.0, 21.0)]
    );
}
#[test]
fn map_rect_quarter_turn_bounds() {
    let h = Homography::rotation_quarter_turns(C, 1);
    let b = h
        .map_rect_bounds(&Rect::new(0.0, 0.0, 2.0, 1.0), &cfg())
        .unwrap();
    assert_eq!(b, Rect::new(-1.0, 0.0, 0.0, 2.0));
}
#[test]
fn map_rect_perspective() {
    let q = persp()
        .map_rect(&Rect::new(0.0, 0.0, 1.0, 1.0), &cfg())
        .unwrap();
    assert_eq!(q, [p(0.0, 0.0), p(0.5, 0.0), p(0.5, 0.5), p(0.0, 1.0)]);
}
#[test]
fn map_rect_perspective_bounds() {
    let b = persp()
        .map_rect_bounds(&Rect::new(0.0, 0.0, 1.0, 1.0), &cfg())
        .unwrap();
    assert_eq!(b, Rect::new(0.0, 0.0, 0.5, 1.0));
}
#[test]
fn map_rect_none_when_a_corner_is_behind() {
    assert_eq!(
        persp().map_rect(&Rect::new(-2.0, 0.0, 1.0, 1.0), &cfg()),
        None
    );
    assert_eq!(
        persp().map_rect_bounds(&Rect::new(-2.0, 0.0, 1.0, 1.0), &cfg()),
        None
    );
}
#[test]
fn map_rect_clamp_gives_quad() {
    let c = Config {
        guard: DivisorGuard::Clamp(1.0),
        ..cfg()
    };
    let q = persp()
        .map_rect(&Rect::new(-2.0, 0.0, 1.0, 1.0), &c)
        .unwrap();
    assert_eq!(q[0], p(-2.0, 0.0));
}

// config space
#[test]
fn default_config_values() {
    let c = Config::default();
    assert_eq!(c.convention, C);
    assert_eq!(c.guard, DivisorGuard::ReturnNone);
    assert_eq!(c.divisor_min, 0.0);
    assert_eq!(c.center, PixelCenter::Half);
    assert_eq!(c.y_axis, YAxis::Down);
    assert_eq!(c.pixel_aspect, 1.0);
    assert_eq!(c.singular_tolerance, 1e-12);
}
#[test]
fn enumerate_count() {
    let v = Config::enumerate(&[1.0, 1e-6], &[1.0, 2.0, 0.5]);
    assert_eq!(v.len(), 2 * 4 * 2 * 2 * 3);
}
#[test]
fn enumerate_minimal() {
    let v = Config::enumerate(&[], &[1.0]);
    assert_eq!(v.len(), 2 * 2 * 2 * 2);
}
#[test]
fn enumerate_empty_aspects_is_empty() {
    assert!(Config::enumerate(&[1.0], &[]).is_empty());
}
#[test]
fn enumerate_all_distinct() {
    let v = Config::enumerate(&[1.0], &[1.0, 2.0]);
    for i in 0..v.len() {
        for j in (i + 1)..v.len() {
            assert_ne!(v[i], v[j]);
        }
    }
}
#[test]
fn enumerate_first_and_last() {
    let v = Config::enumerate(&[3.0], &[1.0, 2.0]);
    let f = v[0];
    assert_eq!(f.convention, C);
    assert_eq!(f.guard, DivisorGuard::ReturnNone);
    assert_eq!(f.center, PixelCenter::Integer);
    assert_eq!(f.y_axis, YAxis::Down);
    assert_eq!(f.pixel_aspect, 1.0);
    let l = v[v.len() - 1];
    assert_eq!(l.convention, R);
    assert_eq!(l.guard, DivisorGuard::Clamp(3.0));
    assert_eq!(l.center, PixelCenter::Half);
    assert_eq!(l.y_axis, YAxis::Up);
    assert_eq!(l.pixel_aspect, 2.0);
}
#[test]
fn enumerate_covers_every_choice() {
    let v = Config::enumerate(&[3.0], &[1.0]);
    assert!(v.iter().any(|c| c.guard == DivisorGuard::PassThrough));
    assert!(v.iter().any(|c| c.convention == R));
    assert!(v.iter().any(|c| c.center == PixelCenter::Integer));
    assert!(v.iter().any(|c| c.y_axis == YAxis::Up));
    assert!(v
        .iter()
        .all(|c| c.divisor_min == 0.0 && c.singular_tolerance == 1e-12));
}

#[test]
fn row_apply_uses_matrix_corner_entry_for_w() {
    // row layout, m22 = 2: w = x*m02 + y*m12 + 2
    let h = Homography::from_matrix([[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]], R);
    assert_eq!(h.apply(p(3.0, 4.0), &cfg()), Some(p(3.0, 4.0)));
    assert_eq!(h.apply_homogeneous(p(3.0, 4.0)), [6.0, 8.0, 2.0]);
}
#[test]
fn compose_mixed_conventions_asymmetric() {
    let a = Homography::translation(C, 1.0, 2.0);
    let b = Homography::translation(R, 10.0, 20.0);
    let h = a.then(&b);
    assert_eq!(h.apply(p(0.0, 0.0), &cfg()), Some(p(11.0, 22.0)));
    let g = b.then(&a);
    assert_eq!(g.convention, R);
    assert_eq!(g.apply(p(0.0, 0.0), &cfg()), Some(p(11.0, 22.0)));
}
