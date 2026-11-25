//! Regression tests of the audit: edge function consistency across a shared segment and the
//! quadratic root choice for a nearly affine patch.

use mesh_warp::*;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

/// Every point of a shared segment lies in a mesh that covers it: no crack between neighbours.
#[test]
fn points_on_shared_segments_are_never_lost() {
    let mut rng = Rng(88172645463325252);
    let mut lost = 0;
    for trial in 0..200 {
        let n = 2 + trial % 4;
        let mesh = grid_mesh(
            n,
            n,
            rng.unit() * 3.0,
            rng.unit() * 3.0,
            1.0 + rng.unit() * 7.0,
            1.0 + rng.unit() * 7.0,
            Diagonal::Alternating,
        );
        for tie in [TieRule::LowestIndex, TieRule::TopLeft] {
            let cfg = Config {
                tie,
                ..Config::default()
            };
            let warp = Warp::new(mesh.clone(), &cfg).unwrap();
            for _ in 0..300 {
                let t = (rng.next() % mesh.triangles.len() as u64) as usize;
                let [a, b, c] = mesh.corners(t, Space::Dst);
                let s = rng.unit();
                let (p, q) = match rng.next() % 3 {
                    0 => (a, b),
                    1 => (b, c),
                    _ => (c, a),
                };
                let pt = v(p.x + s * (q.x - p.x), p.y + s * (q.y - p.y));
                if warp.locate_dst(pt).is_none() {
                    lost += 1;
                }
            }
        }
    }
    assert_eq!(lost, 0);
}

/// Two triangles on opposite sides of a shared segment: a point of the segment is found.
#[test]
fn shared_segment_is_evaluated_identically_from_both_sides() {
    let mut rng = Rng(7);
    let cfg = Config::default();
    let mut checked = 0;
    for _ in 0..20000 {
        let a = v(rng.unit() * 9.0, rng.unit() * 9.0);
        let b = v(rng.unit() * 9.0, rng.unit() * 9.0);
        let c = v(rng.unit() * 9.0, rng.unit() * 9.0);
        let d = v(rng.unit() * 9.0, rng.unit() * 9.0);
        let mesh = Mesh {
            vertices: [a, b, c, d]
                .iter()
                .map(|&p| Vertex { src: p, dst: p })
                .collect(),
            triangles: vec![[0, 1, 2], [1, 0, 3]],
        };
        let (ta, tb) = (mesh.area2(0, Space::Dst), mesh.area2(1, Space::Dst));
        if ta == 0.0 || tb == 0.0 || (ta > 0.0) != (tb > 0.0) {
            continue;
        }
        let warp = Warp::new(mesh, &cfg).unwrap();
        let s = rng.unit();
        let pt = v(a.x + s * (b.x - a.x), a.y + s * (b.y - a.y));
        assert!(
            warp.locate_dst(pt).is_some(),
            "lost {:?} {:?} {:?} {:?} {:?}",
            a,
            b,
            c,
            d,
            pt
        );
        checked += 1;
    }
    assert!(checked > 1000);
}

/// A patch whose corners form a parallelogram up to rounding: the quadratic term is a few ulps.
#[test]
fn near_parallelogram_round_trip() {
    let q = QuadPatch {
        corners: [
            v(0.19522213520061849, 0.7615457835645942),
            v(1.6041317366815222, 0.7730512841037183),
            v(1.1349306074525178, 1.8634051388402673),
            v(-0.273978994028386, 1.8518996383011432),
        ],
    };
    let (u, w) = (0.721504866172506, 0.5900422890699633);
    let uv = q
        .inverse(q.forward(u, w), &QuadConfig::default())
        .expect("inside the patch");
    assert!(
        (uv.x - u).abs() < 1e-9 && (uv.y - w).abs() < 1e-9,
        "{:?}",
        uv
    );
}

#[test]
fn near_parallelogram_random_round_trips() {
    let mut rng = Rng(88172645463325252);
    for _ in 0..5000 {
        let a = v(rng.unit(), rng.unit());
        let e = v(1.0 + rng.unit(), rng.unit() - 0.5);
        let f = v(rng.unit() - 0.5, 1.0 + rng.unit());
        let q = QuadPatch {
            corners: [
                a,
                v(a.x + e.x, a.y + e.y),
                v(a.x + e.x + f.x, a.y + e.y + f.y),
                v(a.x + f.x, a.y + f.y),
            ],
        };
        let (u, w) = (rng.unit(), rng.unit());
        let uv = q
            .inverse(q.forward(u, w), &QuadConfig::default())
            .expect("inside the patch");
        assert!((uv.x - u).abs() < 1e-6 && (uv.y - w).abs() < 1e-6);
    }
}

// ---- Tests added to close survivors of the audit mutants ----

fn cross2(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn sub2(a: Vec2, b: Vec2) -> Vec2 {
    v(a.x - b.x, a.y - b.y)
}

/// The two barycentric forms follow their documented operation order bit for bit.
#[test]
fn barycentric_forms_follow_the_documented_operation_order() {
    let mut rng = Rng(424242);
    for _ in 0..2000 {
        let mut pt = || v(rng.unit() * 10.0 - 5.0, rng.unit() * 10.0 - 5.0);
        let (a, b, c, p) = (pt(), pt(), pt(), pt());
        let area = cross2(sub2(b, a), sub2(c, a));
        let edge = barycentric(a, b, c, p, BaryForm::EdgeFunctions).unwrap();
        let want = [
            cross2(sub2(c, b), sub2(p, b)) / area,
            cross2(sub2(a, c), sub2(p, c)) / area,
            cross2(sub2(b, a), sub2(p, a)) / area,
        ];
        for i in 0..3 {
            assert_eq!(edge[i].to_bits(), want[i].to_bits());
        }
        let cr = barycentric(a, b, c, p, BaryForm::Cramer).unwrap();
        let lb = cross2(sub2(p, a), sub2(c, a)) / area;
        let lc = cross2(sub2(b, a), sub2(p, a)) / area;
        let want = [(1.0 - lb) - lc, lb, lc];
        for i in 0..3 {
            assert_eq!(cr[i].to_bits(), want[i].to_bits());
        }
    }
}

/// The interpolated position sums its three terms left to right.
#[test]
fn interpolation_sums_left_to_right() {
    let mut rng = Rng(5150);
    for _ in 0..2000 {
        let mut pt = || v(rng.unit() * 10.0 - 5.0, rng.unit() * 10.0 - 5.0);
        let (sa, sb, sc) = (pt(), pt(), pt());
        let (da, db, dc) = (pt(), pt(), pt());
        let mesh = Mesh {
            vertices: vec![
                Vertex { src: sa, dst: da },
                Vertex { src: sb, dst: db },
                Vertex { src: sc, dst: dc },
            ],
            triangles: vec![[0, 1, 2]],
        };
        let cfg = Config {
            outside: Outside::ExtendAffine,
            ..Config::default()
        };
        if mesh.validate(&cfg).is_err() {
            continue;
        }
        let warp = Warp::new(mesh, &cfg).unwrap();
        let p = pt();
        let l = barycentric(da, db, dc, p, BaryForm::EdgeFunctions).unwrap();
        let got = warp.inverse(p).unwrap();
        assert_eq!(
            got.x.to_bits(),
            ((l[0] * sa.x + l[1] * sb.x) + l[2] * sc.x).to_bits()
        );
        assert_eq!(
            got.y.to_bits(),
            ((l[0] * sa.y + l[1] * sb.y) + l[2] * sc.y).to_bits()
        );
    }
}

/// Non square grids: location agrees with brute force and the cell contents follow the stated rule.
#[test]
fn non_square_fixed_grids_locate_and_list_by_the_stated_rule() {
    for (gx, gy) in [(3usize, 2usize), (2, 5), (4, 1), (1, 3)] {
        let (cols, rows) = (6usize, 4usize);
        let mesh = grid_mesh(cols, rows, 0.0, 0.0, 6.0, 4.0, Diagonal::Alternating);
        let cfg = Config {
            grid: GridRes::Fixed(gx, gy),
            ..Config::default()
        };
        let warp = Warp::new(mesh.clone(), &cfg).unwrap();
        let loc = Locator::build(&mesh, Space::Dst, &cfg);
        assert_eq!(loc.grid_dims(), (gx, gy));
        // brute force, lowest index closed containment, on a half integer lattice
        for xi in -2..=14 {
            for yi in -2..=10 {
                let p = v(xi as f64 / 2.0, yi as f64 / 2.0);
                let mut want = None;
                for t in 0..mesh.triangles.len() {
                    let [a, b, c] = mesh.corners(t, Space::Dst);
                    let s = if cross2(sub2(b, a), sub2(c, a)) < 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                    let e = [
                        cross2(sub2(c, b), sub2(p, b)) * s,
                        cross2(sub2(a, c), sub2(p, c)) * s,
                        cross2(sub2(b, a), sub2(p, a)) * s,
                    ];
                    if e.iter().all(|&x| x >= 0.0) {
                        want = Some(t);
                        break;
                    }
                }
                assert_eq!(warp.locate_dst(p).map(|l| l.triangle), want, "{:?}", p);
            }
        }
        // cell contents by the documented cell rule, probed at each cell centre
        let (sx, sy) = (6.0 / gx as f64, 4.0 / gy as f64);
        let cell =
            |val: f64, size: f64, n: usize| ((val / size).floor().max(0.0) as usize).min(n - 1);
        for cy in 0..gy {
            for cx in 0..gx {
                let centre = v((cx as f64 + 0.5) * sx, (cy as f64 + 0.5) * sy);
                let mut want = 0;
                for t in 0..mesh.triangles.len() {
                    let cs = mesh.corners(t, Space::Dst);
                    let xs: Vec<usize> = cs.iter().map(|c| cell(c.x, sx, gx)).collect();
                    let ys: Vec<usize> = cs.iter().map(|c| cell(c.y, sy, gy)).collect();
                    let (x0, x1) = (*xs.iter().min().unwrap(), *xs.iter().max().unwrap());
                    let (y0, y1) = (*ys.iter().min().unwrap(), *ys.iter().max().unwrap());
                    if (x0..=x1).contains(&cx) && (y0..=y1).contains(&cy) {
                        want += 1;
                    }
                }
                assert_eq!(loc.cell_len(centre), want, "grid {gx}x{gy} cell {cx},{cy}");
            }
        }
    }
}

/// `|A| == linear_eps` takes the linear branch.
#[test]
fn linear_eps_is_inclusive() {
    let q = QuadPatch {
        corners: [v(0.0, 0.0), v(2.0, 0.0), v(3.0, 2.0), v(0.0, 1.0)],
    };
    let p = q.forward(0.5, 0.5);
    let lin = QuadConfig {
        linear_eps: 2.0,
        ..QuadConfig::default()
    };
    let uv = q.inverse(p, &lin).unwrap();
    assert!((uv.x - 1.25 / 1.5).abs() < 1e-12, "{:?}", uv);
    let quad = QuadConfig {
        linear_eps: 1.99,
        ..QuadConfig::default()
    };
    let uv = q.inverse(p, &quad).unwrap();
    assert!(
        (uv.x - 0.5).abs() < 1e-12 && (uv.y - 0.5).abs() < 1e-12,
        "{:?}",
        uv
    );
}

/// Both roots outside the unit square: the one with the smaller squared excess is taken.
#[test]
fn first_in_unit_outside_picks_the_smaller_squared_excess() {
    let q = QuadPatch {
        corners: [v(5.0, 4.0), v(7.0, 0.0), v(4.0, 6.0), v(6.0, 8.0)],
    };
    let p = v(-2.0, 6.0);
    let cfg = |root| QuadConfig {
        root,
        accept_outside: true,
        linear_eps: 0.0,
    };
    let minus = q.inverse(p, &cfg(RootChoice::Minus)).unwrap();
    let plus = q.inverse(p, &cfg(RootChoice::Plus)).unwrap();
    assert!((minus.x - 2.5).abs() < 1e-9 && (plus.x + 1.0).abs() < 1e-9);
    // excess squared: minus 2.25 + 1/9, plus 1 + 1; the sums are 1.83 and 2, so only the squares agree
    assert_eq!(q.inverse(p, &cfg(RootChoice::FirstInUnit)), Some(plus));
}

/// Equal excess: the first (minus) root wins.
#[test]
fn first_in_unit_outside_tie_takes_the_minus_root() {
    let q = QuadPatch {
        corners: [v(5.0, 6.0), v(4.0, 4.0), v(9.0, 7.0), v(4.0, 4.0)],
    };
    let p = v(2.0, 2.0);
    let cfg = |root| QuadConfig {
        root,
        accept_outside: true,
        linear_eps: 0.0,
    };
    let minus = q.inverse(p, &cfg(RootChoice::Minus)).unwrap();
    let plus = q.inverse(p, &cfg(RootChoice::Plus)).unwrap();
    assert_ne!(minus, plus);
    assert_eq!(q.inverse(p, &cfg(RootChoice::FirstInUnit)), Some(minus));
}
