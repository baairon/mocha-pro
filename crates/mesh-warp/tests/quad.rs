use mesh_warp::*;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn unit() -> QuadPatch {
    QuadPatch {
        corners: [v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0), v(0.0, 1.0)],
    }
}

fn square2() -> QuadPatch {
    QuadPatch {
        corners: [v(0.0, 0.0), v(2.0, 0.0), v(2.0, 2.0), v(0.0, 2.0)],
    }
}

/// Non affine patch: P00 (0,0), P10 (2,0), P11 (3,2), P01 (0,1); g = (1,1).
fn skew() -> QuadPatch {
    QuadPatch {
        corners: [v(0.0, 0.0), v(2.0, 0.0), v(3.0, 2.0), v(0.0, 1.0)],
    }
}

fn d() -> QuadConfig {
    QuadConfig::default()
}

#[test]
fn forward_corners_exact() {
    let q = skew();
    assert_eq!(q.forward(0.0, 0.0), v(0.0, 0.0));
    assert_eq!(q.forward(1.0, 0.0), v(2.0, 0.0));
    assert_eq!(q.forward(1.0, 1.0), v(3.0, 2.0));
    assert_eq!(q.forward(0.0, 1.0), v(0.0, 1.0));
}

#[test]
fn forward_centre_hand_value() {
    assert_eq!(skew().forward(0.5, 0.5), v(1.25, 0.75));
}

#[test]
fn forward_edge_midpoints() {
    let q = skew();
    assert_eq!(q.forward(0.5, 0.0), v(1.0, 0.0));
    assert_eq!(q.forward(1.0, 0.5), v(2.5, 1.0));
    assert_eq!(q.forward(0.5, 1.0), v(1.5, 1.5));
    assert_eq!(q.forward(0.0, 0.5), v(0.0, 0.5));
}

#[test]
fn inverse_unit_square_identity() {
    let q = unit();
    for i in 0..=8 {
        for j in 0..=8 {
            let p = v(i as f64 / 8.0, j as f64 / 8.0);
            assert_eq!(q.inverse(p, &d()), Some(p));
        }
    }
}

#[test]
fn inverse_affine_square_corners_and_centre() {
    let q = square2();
    assert_eq!(q.inverse(v(0.0, 0.0), &d()), Some(v(0.0, 0.0)));
    assert_eq!(q.inverse(v(2.0, 0.0), &d()), Some(v(1.0, 0.0)));
    assert_eq!(q.inverse(v(2.0, 2.0), &d()), Some(v(1.0, 1.0)));
    assert_eq!(q.inverse(v(0.0, 2.0), &d()), Some(v(0.0, 1.0)));
    assert_eq!(q.inverse(v(1.0, 1.0), &d()), Some(v(0.5, 0.5)));
}

#[test]
fn inverse_skew_corners() {
    let q = skew();
    assert_eq!(q.inverse(v(0.0, 0.0), &d()), Some(v(0.0, 0.0)));
    assert_eq!(q.inverse(v(2.0, 0.0), &d()), Some(v(1.0, 0.0)));
    assert_eq!(q.inverse(v(3.0, 2.0), &d()), Some(v(1.0, 1.0)));
    assert_eq!(q.inverse(v(0.0, 1.0), &d()), Some(v(0.0, 1.0)));
}

#[test]
fn inverse_skew_centre() {
    assert_eq!(skew().inverse(v(1.25, 0.75), &d()), Some(v(0.5, 0.5)));
}

#[test]
fn inverse_skew_centre_bits() {
    let r = skew().inverse(v(1.25, 0.75), &d()).unwrap();
    assert_eq!(
        (r.x.to_bits(), r.y.to_bits()),
        (0x3FE0_0000_0000_0000, 0x3FE0_0000_0000_0000)
    );
}

#[test]
fn root_minus_and_plus_at_corner() {
    let q = skew();
    let minus = QuadConfig {
        root: RootChoice::Minus,
        accept_outside: true,
        ..d()
    };
    let plus = QuadConfig {
        root: RootChoice::Plus,
        ..d()
    };
    assert_eq!(q.inverse(v(2.0, 0.0), &minus), Some(v(-1.0, -4.0)));
    assert_eq!(q.inverse(v(2.0, 0.0), &plus), Some(v(1.0, 0.0)));
}

#[test]
fn root_minus_rejected_without_outside() {
    let minus = QuadConfig {
        root: RootChoice::Minus,
        ..d()
    };
    assert_eq!(skew().inverse(v(2.0, 0.0), &minus), None);
}

#[test]
fn root_minus_centre_is_outside_root() {
    // roots at the centre are -1.25 (minus) and 0.5 (plus).
    let q = skew();
    let minus = QuadConfig {
        root: RootChoice::Minus,
        accept_outside: true,
        ..d()
    };
    assert_eq!(q.inverse(v(1.25, 0.75), &minus).map(|r| r.x), Some(-1.25));
    let plus = QuadConfig {
        root: RootChoice::Plus,
        ..d()
    };
    assert_eq!(q.inverse(v(1.25, 0.75), &plus), Some(v(0.5, 0.5)));
}

#[test]
fn no_real_root_is_none_in_all_configs() {
    for c in quad_config_space() {
        assert_eq!(skew().inverse(v(-2.0, -1.0), &c), None);
    }
}

#[test]
fn outside_point_rejected_by_default_accepted_when_asked() {
    let q = square2();
    assert_eq!(q.inverse(v(4.0, 1.0), &d()), None);
    let acc = QuadConfig {
        accept_outside: true,
        ..d()
    };
    assert_eq!(q.inverse(v(4.0, 1.0), &acc), Some(v(2.0, 0.5)));
    assert_eq!(q.inverse(v(-2.0, 3.0), &acc), Some(v(-1.0, 1.5)));
}

#[test]
fn v_outside_unit_rejected() {
    assert_eq!(square2().inverse(v(1.0, 3.0), &d()), None);
    assert_eq!(unit().inverse(v(0.5, -0.25), &d()), None);
}

#[test]
fn first_in_unit_nearest_candidate_when_none_inside() {
    // skew patch, point left of the quad: candidates are both outside; the nearer is returned.
    let acc = QuadConfig {
        accept_outside: true,
        ..d()
    };
    let r = skew().inverse(v(-0.5, 0.25), &acc);
    assert!(r.is_some());
    let r = r.unwrap();
    let f = skew().forward(r.x, r.y);
    assert!((f.x + 0.5).abs() < 1e-12 && (f.y - 0.25).abs() < 1e-12);
}

#[test]
fn linear_eps_forces_linear_branch() {
    let c = QuadConfig {
        linear_eps: 1e9,
        ..d()
    };
    // centre: B = 1.5, C = -1.25, u = 1.25 / 1.5
    let r = skew()
        .inverse(
            v(1.25, 0.75),
            &QuadConfig {
                accept_outside: true,
                ..c
            },
        )
        .unwrap();
    assert_eq!(r.x, 1.25 / 1.5);
}

#[test]
fn linear_branch_zero_b_is_none() {
    // degenerate patch collapsed to a point: e = f = g = 0.
    let q = QuadPatch {
        corners: [v(1.0, 1.0); 4],
    };
    assert_eq!(q.inverse(v(1.0, 1.0), &d()), None);
}

#[test]
fn collapsed_edge_patch_still_inverts() {
    // P10 = P11 (a triangle like patch): (0,0),(2,0),(2,0),(0,2)
    let q = QuadPatch {
        corners: [v(0.0, 0.0), v(2.0, 0.0), v(2.0, 0.0), v(0.0, 2.0)],
    };
    let r = q.inverse(v(1.0, 0.5), &d()).unwrap();
    let f = q.forward(r.x, r.y);
    assert!((f.x - 1.0).abs() < 1e-12 && (f.y - 0.5).abs() < 1e-12);
}

fn newton(q: &QuadPatch, p: Vec2) -> Vec2 {
    // Independent slow reference: Newton iteration on P(u,v) - p from the centre.
    let (mut u, mut w) = (0.5, 0.5);
    for _ in 0..60 {
        let f = q.forward(u, w);
        let (rx, ry) = (f.x - p.x, f.y - p.y);
        let h = 1e-6;
        let fu = q.forward(u + h, w);
        let fv = q.forward(u, w + h);
        let (a, b, c, e) = (
            (fu.x - f.x) / h,
            (fv.x - f.x) / h,
            (fu.y - f.y) / h,
            (fv.y - f.y) / h,
        );
        let det = a * e - b * c;
        u -= (e * rx - b * ry) / det;
        w -= (-c * rx + a * ry) / det;
    }
    v(u, w)
}

#[test]
fn inverse_matches_newton_reference_grid() {
    let patches = [
        skew(),
        QuadPatch {
            corners: [v(0.0, 0.0), v(4.0, 1.0), v(5.0, 5.0), v(-1.0, 3.0)],
        },
        QuadPatch {
            corners: [v(1.0, 1.0), v(1.0, 5.0), v(6.0, 4.0), v(4.0, 0.5)],
        },
    ];
    for q in &patches {
        for i in 1..10 {
            for j in 1..10 {
                let (u, w) = (i as f64 / 10.0, j as f64 / 10.0);
                let p = q.forward(u, w);
                let got = q.inverse(p, &d()).expect("inside");
                let want = newton(q, p);
                assert!((got.x - want.x).abs() < 1e-6 && (got.y - want.y).abs() < 1e-6);
                assert!(
                    (got.x - u).abs() < 1e-9 && (got.y - w).abs() < 1e-9,
                    "{q:?} {u} {w} {got:?}"
                );
            }
        }
    }
}

#[test]
fn inverse_closure_matches() {
    let q = skew();
    let c = d();
    let f = q.inverse_closure(&c);
    assert_eq!(f(1.25, 0.75), Some((0.5, 0.5)));
    assert_eq!(f(-2.0, -1.0), None);
}

#[test]
fn quad_config_space_has_six_distinct() {
    let s = quad_config_space();
    assert_eq!(s.len(), 6);
    for i in 0..6 {
        for j in i + 1..6 {
            assert_ne!(s[i], s[j]);
        }
    }
    assert!(s.contains(&QuadConfig::default()));
}

#[test]
fn quad_default_values() {
    let c = QuadConfig::default();
    assert_eq!(c.root, RootChoice::FirstInUnit);
    assert!(!c.accept_outside);
    assert_eq!(c.linear_eps, 0.0);
}

#[test]
fn quad_determinism_runs_equal() {
    let run = || {
        let q = QuadPatch {
            corners: [v(0.0, 0.0), v(4.0, 1.0), v(5.0, 5.0), v(-1.0, 3.0)],
        };
        (1..20)
            .map(|i| q.inverse(q.forward(i as f64 / 20.0, 0.3), &d()).unwrap())
            .flat_map(|r| [r.x.to_bits(), r.y.to_bits()])
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

#[test]
fn forward_weights_order_sum() {
    // all four corners equal: any (u, v) gives the corner (weights sum to one on dyadics).
    let q = QuadPatch {
        corners: [v(3.0, 4.0); 4],
    };
    assert_eq!(q.forward(0.25, 0.75), v(3.0, 4.0));
}

#[test]
fn inverse_tall_patch_uses_y_component() {
    // d = f + u g has the larger y component: v from y.
    let q = QuadPatch {
        corners: [v(0.0, 0.0), v(1.0, 0.0), v(1.0, 4.0), v(0.0, 4.0)],
    };
    assert_eq!(q.inverse(v(0.5, 2.0), &d()), Some(v(0.5, 0.5)));
}

#[test]
fn inverse_wide_patch_uses_x_component() {
    let q = QuadPatch {
        corners: [v(0.0, 0.0), v(4.0, 0.0), v(4.0, 1.0), v(0.0, 1.0)],
    };
    // swapped roles: e along x, f along y; d = (0, 1) uses y; use rotated patch for x.
    let r = QuadPatch {
        corners: [v(0.0, 0.0), v(0.0, 1.0), v(-4.0, 1.0), v(-4.0, 0.0)],
    };
    assert_eq!(q.inverse(v(1.0, 0.5), &d()), Some(v(0.25, 0.5)));
    assert_eq!(r.inverse(v(-2.0, 0.5), &d()), Some(v(0.5, 0.5)));
}

#[test]
fn zero_discriminant_double_root() {
    // skew patch at p = (-2, 0): B = 4, C = 2, A = 2, D = 0, both roots equal -1 (outside unit).
    let acc = QuadConfig {
        accept_outside: true,
        root: RootChoice::Plus,
        ..d()
    };
    let r = skew().inverse(v(-2.0, 0.0), &acc).unwrap();
    assert_eq!(r.x, -1.0);
    let minus = QuadConfig {
        root: RootChoice::Minus,
        ..acc
    };
    assert_eq!(skew().inverse(v(-2.0, 0.0), &minus).unwrap().x, -1.0);
}

#[test]
fn small_negative_discriminant_is_none() {
    // skew patch at p = (-2, -0.1): D = 3.9^2 - 16 = -0.79, no real root, even when outside is accepted.
    let acc = QuadConfig {
        accept_outside: true,
        ..d()
    };
    for root in [RootChoice::Minus, RootChoice::Plus, RootChoice::FirstInUnit] {
        let r = skew().inverse(v(-2.0, -0.1), &QuadConfig { root, ..acc });
        assert_eq!(r, None, "{root:?} {:?}", skew());
    }
}

#[test]
fn first_in_unit_prefers_candidate_nearer_the_unit_square() {
    // roots at p = (-0.5, 0.25) are about -1.1594 and -0.2156; the second is nearer.
    let acc = QuadConfig {
        accept_outside: true,
        ..d()
    };
    let r = skew().inverse(v(-0.5, 0.25), &acc).unwrap();
    assert!((r.x + 0.215635).abs() < 1e-5, "{r:?}");
    assert!((r.y - 0.3187).abs() < 1e-3, "{r:?}");
}
