use mesh_warp::*;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn tri(src: [(f64, f64); 3], dst: [(f64, f64); 3]) -> Mesh {
    Mesh {
        vertices: (0..3)
            .map(|i| Vertex {
                src: v(src[i].0, src[i].1),
                dst: v(dst[i].0, dst[i].1),
            })
            .collect(),
        triangles: vec![[0, 1, 2]],
    }
}

fn scale_tri() -> Mesh {
    tri(
        [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
        [(0.0, 0.0), (2.0, 0.0), (0.0, 2.0)],
    )
}

fn cfg() -> Config {
    Config::default()
}

fn with(f: impl Fn(&mut Config)) -> Config {
    let mut c = cfg();
    f(&mut c);
    c
}

fn grid(c: usize, r: usize, d: Diagonal) -> Mesh {
    grid_mesh(c, r, 0.0, 0.0, c as f64, r as f64, d)
}

/// Deterministic generator of values in [0, 1).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ---- identity and translation

#[test]
fn identity_inverse_exact_all_diagonals() {
    for d in [Diagonal::Forward, Diagonal::Back, Diagonal::Alternating] {
        let w = Warp::new(grid(4, 4, d), &cfg()).unwrap();
        for i in 0..=16 {
            for j in 0..=16 {
                let p = v(i as f64 / 4.0, j as f64 / 4.0);
                assert_eq!(w.inverse(p), Some(p), "{d:?} {p:?}");
            }
        }
    }
}

#[test]
fn identity_forward_exact() {
    let w = Warp::new(grid(3, 2, Diagonal::Forward), &cfg()).unwrap();
    for i in 0..=12 {
        for j in 0..=8 {
            let p = v(i as f64 / 4.0, j as f64 / 4.0);
            assert_eq!(w.forward(p), Some(p));
        }
    }
}

#[test]
fn identity_cramer_form_exact() {
    let c = with(|c| c.bary = BaryForm::Cramer);
    let w = Warp::new(grid(4, 4, Diagonal::Back), &c).unwrap();
    for i in 0..=16 {
        let p = v(i as f64 / 4.0, 4.0 - i as f64 / 8.0);
        assert_eq!(w.inverse(p), Some(p));
    }
}

#[test]
fn translated_mesh_inverse() {
    let mut m = grid(4, 4, Diagonal::Alternating);
    m.translate_dst(3.0, -2.0);
    let w = Warp::new(m, &cfg()).unwrap();
    for i in 0..=8 {
        for j in 0..=8 {
            let s = v(i as f64 / 2.0, j as f64 / 2.0);
            assert_eq!(w.inverse(v(s.x + 3.0, s.y - 2.0)), Some(s));
        }
    }
}

#[test]
fn translated_mesh_forward() {
    let mut m = grid(2, 2, Diagonal::Forward);
    m.translate_dst(-5.0, 7.0);
    let w = Warp::new(m, &cfg()).unwrap();
    assert_eq!(w.forward(v(1.5, 0.5)), Some(v(-3.5, 7.5)));
    assert_eq!(w.forward(v(0.0, 0.0)), Some(v(-5.0, 7.0)));
}

#[test]
fn translated_mesh_old_position_is_outside() {
    let mut m = grid(2, 2, Diagonal::Forward);
    m.translate_dst(10.0, 0.0);
    let w = Warp::new(m, &cfg()).unwrap();
    assert_eq!(w.inverse(v(1.0, 1.0)), None);
}

// ---- one triangle, hand computed affine map

#[test]
fn affine_inverse_hand_value() {
    let w = Warp::new(scale_tri(), &cfg()).unwrap();
    let r = w.inverse(v(1.0, 1.0)).unwrap();
    assert_eq!(r, v(0.5, 0.25 * 2.0));
    assert_eq!(r.x.to_bits(), 0x3FE0_0000_0000_0000);
}

#[test]
fn affine_inverse_hand_value_y() {
    let w = Warp::new(scale_tri(), &cfg()).unwrap();
    let r = w.inverse(v(1.0, 0.5)).unwrap();
    assert_eq!(r.x, 0.5);
    assert_eq!(r.y, 0.25);
    assert_eq!(r.y.to_bits(), 0x3FD0_0000_0000_0000);
}

#[test]
fn affine_forward_hand_value() {
    let w = Warp::new(scale_tri(), &cfg()).unwrap();
    assert_eq!(w.forward(v(0.5, 0.25)), Some(v(1.0, 0.5)));
}

#[test]
fn affine_nonuniform_scale_and_shear() {
    // src (0,0),(1,0),(0,1) to dst (1,1),(3,1),(2,4): x' = 1 + 2 s + 1 t, y' = 1 + 0 s + 3 t.
    let m = tri(
        [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
        [(1.0, 1.0), (3.0, 1.0), (2.0, 4.0)],
    );
    let w = Warp::new(m, &cfg()).unwrap();
    assert_eq!(w.forward(v(0.25, 0.5)), Some(v(2.0, 2.5)));
    assert_eq!(w.inverse(v(2.0, 2.5)), Some(v(0.25, 0.5)));
}

#[test]
fn vertices_map_to_vertices() {
    let w = Warp::new(scale_tri(), &cfg()).unwrap();
    assert_eq!(w.inverse(v(0.0, 0.0)), Some(v(0.0, 0.0)));
    assert_eq!(w.inverse(v(2.0, 0.0)), Some(v(1.0, 0.0)));
    assert_eq!(w.inverse(v(0.0, 2.0)), Some(v(0.0, 1.0)));
}

#[test]
fn clockwise_triangle_works() {
    let m = tri(
        [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0)],
        [(0.0, 0.0), (0.0, 2.0), (2.0, 0.0)],
    );
    let w = Warp::new(m, &cfg()).unwrap();
    assert_eq!(w.inverse(v(1.0, 0.5)), Some(v(0.5, 0.25)));
}

#[test]
fn mirrored_mesh_inverse() {
    let m = tri(
        [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
        [(0.0, 0.0), (-1.0, 0.0), (0.0, 1.0)],
    );
    let w = Warp::new(m, &cfg()).unwrap();
    assert_eq!(w.inverse(v(-0.5, 0.25)), Some(v(0.5, 0.25)));
}

// ---- barycentric

#[test]
fn bary_vertices_both_forms() {
    let (a, b, c) = (v(0.0, 0.0), v(3.0, 0.0), v(0.0, 3.0));
    for f in [BaryForm::EdgeFunctions, BaryForm::Cramer] {
        assert_eq!(barycentric(a, b, c, a, f), Some([1.0, 0.0, 0.0]));
        assert_eq!(barycentric(a, b, c, b, f), Some([0.0, 1.0, 0.0]));
        assert_eq!(barycentric(a, b, c, c, f), Some([0.0, 0.0, 1.0]));
    }
}

#[test]
fn bary_edge_midpoint_both_forms() {
    let (a, b, c) = (v(0.0, 0.0), v(3.0, 0.0), v(0.0, 3.0));
    for f in [BaryForm::EdgeFunctions, BaryForm::Cramer] {
        assert_eq!(barycentric(a, b, c, v(1.5, 1.5), f), Some([0.0, 0.5, 0.5]));
        assert_eq!(barycentric(a, b, c, v(0.0, 1.5), f), Some([0.5, 0.0, 0.5]));
    }
}

#[test]
fn bary_degenerate_is_none() {
    let (a, b, c) = (v(0.0, 0.0), v(1.0, 1.0), v(2.0, 2.0));
    assert_eq!(barycentric(a, b, c, a, BaryForm::Cramer), None);
    assert_eq!(barycentric(a, b, c, a, BaryForm::EdgeFunctions), None);
}

#[test]
fn bary_negative_outside() {
    let (a, b, c) = (v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0));
    assert_eq!(
        barycentric(a, b, c, v(-1.0, 0.0), BaryForm::EdgeFunctions),
        Some([2.0, -1.0, 0.0])
    );
    assert_eq!(
        barycentric(a, b, c, v(-1.0, 0.0), BaryForm::Cramer),
        Some([2.0, -1.0, 0.0])
    );
}

#[test]
fn bary_sums_to_one_near() {
    let (a, b, c) = (v(0.3, 0.1), v(4.2, 0.7), v(1.1, 3.9));
    for f in [BaryForm::EdgeFunctions, BaryForm::Cramer] {
        let l = barycentric(a, b, c, v(1.7, 1.3), f).unwrap();
        assert!((l[0] + l[1] + l[2] - 1.0).abs() < 1e-14);
    }
}

#[test]
fn bary_sign_independent_of_winding() {
    let (a, b, c) = (v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0));
    let p = v(0.25, 0.25);
    let l1 = barycentric(a, b, c, p, BaryForm::EdgeFunctions).unwrap();
    let l2 = barycentric(a, c, b, p, BaryForm::EdgeFunctions).unwrap();
    assert_eq!(l1, [0.5, 0.25, 0.25]);
    assert_eq!(l2, [0.5, 0.25, 0.25]);
}

// ---- validation

#[test]
fn validate_ok() {
    assert_eq!(grid(2, 2, Diagonal::Forward).validate(&cfg()), Ok(()));
}

#[test]
fn validate_no_triangles() {
    let m = Mesh {
        vertices: vec![],
        triangles: vec![],
    };
    assert_eq!(m.validate(&cfg()), Err(MeshError::NoTriangles));
}

#[test]
fn validate_index_out_of_range() {
    let mut m = scale_tri();
    m.triangles.push([0, 1, 3]);
    assert_eq!(
        m.validate(&cfg()),
        Err(MeshError::IndexOutOfRange {
            triangle: 1,
            index: 3
        })
    );
}

#[test]
fn validate_index_equal_len_is_out() {
    let mut m = scale_tri();
    m.triangles[0] = [0, 1, 3];
    assert!(matches!(
        m.validate(&cfg()),
        Err(MeshError::IndexOutOfRange { index: 3, .. })
    ));
    m.triangles[0] = [0, 1, 2];
    assert_eq!(m.validate(&cfg()), Ok(()));
}

#[test]
fn validate_non_finite() {
    for k in 0..4 {
        let mut m = scale_tri();
        let bad = if k == 3 { f64::INFINITY } else { f64::NAN };
        match k {
            0 => m.vertices[1].src.x = bad,
            1 => m.vertices[1].src.y = bad,
            2 => m.vertices[1].dst.x = bad,
            _ => m.vertices[1].dst.y = bad,
        }
        assert_eq!(m.validate(&cfg()), Err(MeshError::NonFinite { vertex: 1 }));
    }
}

#[test]
fn validate_degenerate_dst_refused_and_allowed() {
    let m = tri(
        [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
        [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)],
    );
    assert_eq!(
        m.validate(&cfg()),
        Err(MeshError::Degenerate { triangle: 0 })
    );
    assert_eq!(m.validate(&with(|c| c.allow_degenerate = true)), Ok(()));
}

#[test]
fn validate_degenerate_src_refused() {
    let m = tri(
        [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)],
        [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
    );
    assert_eq!(
        m.validate(&cfg()),
        Err(MeshError::Degenerate { triangle: 0 })
    );
}

#[test]
fn validate_winding_consistent_rejects_mixed() {
    let mut m = grid(2, 1, Diagonal::Forward);
    m.triangles[1] = [m.triangles[1][0], m.triangles[1][2], m.triangles[1][1]];
    let c = with(|c| c.winding = Winding::Consistent);
    assert_eq!(
        m.validate(&c),
        Err(MeshError::InconsistentWinding { triangle: 1 })
    );
    assert_eq!(m.validate(&cfg()), Ok(()));
}

#[test]
fn validate_winding_consistent_accepts_flipped_all() {
    let mut m = grid(2, 2, Diagonal::Back);
    for t in &mut m.triangles {
        *t = [t[0], t[2], t[1]];
    }
    assert_eq!(
        m.validate(&with(|c| c.winding = Winding::Consistent)),
        Ok(())
    );
}

#[test]
fn validate_winding_checks_dst_space() {
    let mut m = grid(2, 1, Diagonal::Forward);
    // flip one triangle in dst only
    let t = m.triangles[0];
    m.vertices[t[1]].dst = v(0.0, 0.0);
    m.vertices[t[0]].dst = v(1.0, 0.0);
    let c = with(|c| c.winding = Winding::Consistent);
    assert!(matches!(
        m.validate(&c),
        Err(MeshError::InconsistentWinding { .. })
    ));
}

#[test]
fn warp_new_propagates_error() {
    let m = Mesh {
        vertices: vec![],
        triangles: vec![],
    };
    assert_eq!(Warp::new(m, &cfg()).err(), Some(MeshError::NoTriangles));
}

#[test]
fn degenerate_triangle_ignored_by_location() {
    let mut m = grid(1, 1, Diagonal::Forward);
    m.vertices.push(Vertex {
        src: v(0.5, 0.5),
        dst: v(0.5, 0.5),
    });
    m.triangles.insert(0, [4, 4, 4]);
    let w = Warp::new(m, &with(|c| c.allow_degenerate = true)).unwrap();
    assert_eq!(w.locate_dst(v(0.75, 0.25)).unwrap().triangle, 1);
}

// ---- grid mesh

#[test]
fn grid_vertex_positions() {
    let m = grid_mesh(3, 2, 10.0, 20.0, 3.0, 4.0, Diagonal::Forward);
    assert_eq!(m.vertices.len(), 12);
    assert_eq!(m.vertices[0].src, v(10.0, 20.0));
    assert_eq!(m.vertices[1].src, v(11.0, 20.0));
    assert_eq!(m.vertices[4].src, v(10.0, 22.0));
    assert_eq!(m.vertices[11].dst, v(13.0, 24.0));
    let t = grid_mesh(3, 1, 0.0, 0.0, 1.0, 1.0, Diagonal::Forward);
    assert_eq!(t.vertices[1].src.x, (1.0f64 * 1.0) / 3.0);
}

#[test]
fn grid_triangle_lists_forward() {
    let m = grid(1, 1, Diagonal::Forward);
    assert_eq!(m.triangles, vec![[0, 1, 3], [0, 3, 2]]);
}

#[test]
fn grid_triangle_lists_back() {
    let m = grid(1, 1, Diagonal::Back);
    assert_eq!(m.triangles, vec![[0, 1, 2], [1, 3, 2]]);
}

#[test]
fn grid_triangle_lists_alternating() {
    let m = grid(2, 2, Diagonal::Alternating);
    assert_eq!(m.triangles.len(), 8);
    assert_eq!(m.triangles[0], [0, 1, 4]);
    assert_eq!(m.triangles[1], [0, 4, 3]);
    assert_eq!(m.triangles[2], [1, 2, 4]);
    assert_eq!(m.triangles[3], [2, 5, 4]);
    assert_eq!(m.triangles[4], [3, 4, 6]);
    assert_eq!(m.triangles[5], [4, 7, 6]);
    assert_eq!(m.triangles[6], [4, 5, 8]);
    assert_eq!(m.triangles[7], [4, 8, 7]);
}

#[test]
fn grid_all_positive_area_all_configs() {
    for d in [Diagonal::Forward, Diagonal::Back, Diagonal::Alternating] {
        let m = grid_mesh(3, 3, -1.0, 2.0, 6.0, 3.0, d);
        for t in 0..m.triangles.len() {
            assert!(m.area2(t, Space::Dst) > 0.0 && m.area2(t, Space::Src) > 0.0);
        }
        assert_eq!(
            m.validate(&with(|c| c.winding = Winding::Consistent)),
            Ok(())
        );
    }
}

#[test]
fn grid_cell_area_sum() {
    let m = grid(3, 2, Diagonal::Alternating);
    let s: f64 = (0..m.triangles.len()).map(|t| m.area2(t, Space::Src)).sum();
    assert_eq!(s, 12.0);
}

// ---- tie rules

#[test]
fn tie_lowest_index_on_diagonal() {
    let w = Warp::new(grid(1, 1, Diagonal::Back), &cfg()).unwrap();
    assert_eq!(w.locate_dst(v(0.5, 0.5)).unwrap().triangle, 0);
}

#[test]
fn tie_top_left_on_diagonal_back() {
    let w = Warp::new(
        grid(1, 1, Diagonal::Back),
        &with(|c| c.tie = TieRule::TopLeft),
    )
    .unwrap();
    assert_eq!(w.locate_dst(v(0.5, 0.5)).unwrap().triangle, 1);
}

#[test]
fn tie_top_left_on_diagonal_forward() {
    let w = Warp::new(
        grid(1, 1, Diagonal::Forward),
        &with(|c| c.tie = TieRule::TopLeft),
    )
    .unwrap();
    assert_eq!(w.locate_dst(v(0.5, 0.5)).unwrap().triangle, 0);
}

#[test]
fn tie_vertex_fallback_to_lowest() {
    let w = Warp::new(
        grid(1, 1, Diagonal::Back),
        &with(|c| c.tie = TieRule::TopLeft),
    )
    .unwrap();
    let l = w.locate_dst(v(1.0, 0.0)).unwrap();
    assert_eq!(l.triangle, 0);
    assert_eq!(l.bary, [0.0, 1.0, 0.0]);
}

#[test]
fn tie_same_map_value_on_shared_edge() {
    for tie in [TieRule::LowestIndex, TieRule::TopLeft] {
        let w = Warp::new(grid(2, 2, Diagonal::Back), &with(|c| c.tie = tie)).unwrap();
        assert_eq!(w.inverse(v(1.0, 0.5)), Some(v(1.0, 0.5)));
    }
}

#[test]
fn tie_horizontal_shared_edge_top_left_picks_lower_row_cell() {
    // rows are y down: the edge y = 1 is the top edge of the triangle below it.
    let w = Warp::new(
        grid_mesh(1, 2, 0.0, 0.0, 1.0, 2.0, Diagonal::Forward),
        &with(|c| c.tie = TieRule::TopLeft),
    )
    .unwrap();
    // triangles 0,1 are row 0 (y in 0..1); 2,3 are row 1.
    assert_eq!(w.locate_dst(v(0.75, 1.0)).unwrap().triangle, 2);
    let w = Warp::new(
        grid_mesh(1, 2, 0.0, 0.0, 1.0, 2.0, Diagonal::Forward),
        &cfg(),
    )
    .unwrap();
    assert_eq!(w.locate_dst(v(0.75, 1.0)).unwrap().triangle, 1);
}

fn owners_ref(m: &Mesh, x: i64, y: i64) -> Vec<usize> {
    // Independent integer reference of the ownership rule (positive area orientation).
    let mut out = Vec::new();
    for (t, idx) in m.triangles.iter().enumerate() {
        let mut p: Vec<(i64, i64)> = idx
            .iter()
            .map(|&i| (m.vertices[i].dst.x as i64, m.vertices[i].dst.y as i64))
            .collect();
        let a = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[1].1 - p[0].1) * (p[2].0 - p[0].0);
        if a < 0 {
            p.swap(1, 2);
        }
        let mut ok = true;
        for i in 0..3 {
            let (s, e) = (p[i], p[(i + 1) % 3]);
            let f = (e.0 - s.0) * (y - s.1) - (e.1 - s.1) * (x - s.0);
            let (dx, dy) = (e.0 - s.0, e.1 - s.1);
            let owned = dy < 0 || (dy == 0 && dx > 0);
            if !(f > 0 || (f == 0 && owned)) {
                ok = false;
            }
        }
        if ok {
            out.push(t);
        }
    }
    out
}

#[test]
fn tie_top_left_matches_integer_reference_on_lattice() {
    for d in [Diagonal::Forward, Diagonal::Back, Diagonal::Alternating] {
        let m = grid_mesh(4, 3, 0.0, 0.0, 8.0, 6.0, d);
        let w = Warp::new(m.clone(), &with(|c| c.tie = TieRule::TopLeft)).unwrap();
        for y in 0..=6 {
            for x in 0..=8 {
                let own = owners_ref(&m, x, y);
                assert!(own.len() <= 1, "unique owner {d:?} {x} {y}");
                let got = w.locate_dst(v(x as f64, y as f64)).unwrap().triangle;
                if let Some(&o) = own.first() {
                    assert_eq!(got, o, "{d:?} {x} {y}");
                }
            }
        }
    }
}

#[test]
fn tie_top_left_border_points_are_found() {
    let w = Warp::new(
        grid(2, 2, Diagonal::Forward),
        &with(|c| c.tie = TieRule::TopLeft),
    )
    .unwrap();
    for i in 0..=2 {
        for j in 0..=2 {
            assert!(w.locate_dst(v(i as f64, j as f64)).is_some());
        }
    }
}

#[test]
fn tie_lowest_index_is_minimum_of_closed_containment() {
    let m = grid_mesh(3, 3, 0.0, 0.0, 6.0, 6.0, Diagonal::Alternating);
    let w = Warp::new(m.clone(), &cfg()).unwrap();
    for y in 0..=6 {
        for x in 0..=6 {
            let mut first = None;
            for t in 0..m.triangles.len() {
                let [a, b, c] = m.corners(t, Space::Dst);
                let e = |p: Vec2, q: Vec2| {
                    (q.x - p.x) * (y as f64 - p.y) - (q.y - p.y) * (x as f64 - p.x)
                };
                if e(a, b) >= 0.0 && e(b, c) >= 0.0 && e(c, a) >= 0.0 {
                    first = Some(t);
                    break;
                }
            }
            assert_eq!(
                w.locate_dst(v(x as f64, y as f64)).map(|l| l.triangle),
                first
            );
        }
    }
}

// ---- outside rules

#[test]
fn outside_none() {
    let w = Warp::new(scale_tri(), &cfg()).unwrap();
    assert_eq!(w.inverse(v(3.0, 3.0)), None);
    assert_eq!(w.locate_dst(v(-0.1, 0.5)), None);
    assert_eq!(w.forward(v(2.0, 2.0)), None);
}

#[test]
fn outside_clamp_vertex_region() {
    let w = Warp::new(scale_tri(), &with(|c| c.outside = Outside::ClampNearest)).unwrap();
    let l = w.locate_dst(v(3.0, 0.0)).unwrap();
    assert!(l.outside);
    assert_eq!(w.inverse(v(3.0, 0.0)), Some(v(1.0, 0.0)));
    assert_eq!(w.inverse(v(-4.0, -4.0)), Some(v(0.0, 0.0)));
}

#[test]
fn outside_clamp_edge_region() {
    let w = Warp::new(scale_tri(), &with(|c| c.outside = Outside::ClampNearest)).unwrap();
    assert_eq!(w.inverse(v(2.0, 2.0)), Some(v(0.5, 0.5)));
    assert_eq!(w.locate_dst(v(2.0, 2.0)).unwrap().bary, [0.0, 0.5, 0.5]);
    assert_eq!(w.inverse(v(1.0, -3.0)), Some(v(0.5, 0.0)));
}

#[test]
fn outside_extend_affine() {
    let w = Warp::new(scale_tri(), &with(|c| c.outside = Outside::ExtendAffine)).unwrap();
    assert_eq!(w.inverse(v(2.0, 2.0)), Some(v(1.0, 1.0)));
    assert_eq!(w.inverse(v(-2.0, 4.0)), Some(v(-1.0, 2.0)));
    assert!(w.locate_dst(v(2.0, 2.0)).unwrap().outside);
}

#[test]
fn outside_flag_false_inside() {
    for o in [Outside::None, Outside::ClampNearest, Outside::ExtendAffine] {
        let w = Warp::new(scale_tri(), &with(|c| c.outside = o)).unwrap();
        assert!(!w.locate_dst(v(0.5, 0.5)).unwrap().outside);
    }
}

#[test]
fn outside_forward_clamp() {
    let w = Warp::new(scale_tri(), &with(|c| c.outside = Outside::ClampNearest)).unwrap();
    assert_eq!(w.forward(v(2.0, 0.0)), Some(v(2.0, 0.0)));
    assert_eq!(w.forward(v(1.0, 1.0)), Some(v(1.0, 1.0)));
}

fn two_far_triangles() -> Mesh {
    Mesh {
        vertices: [
            (0.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (4.0, 0.0),
            (5.0, 0.0),
            (4.0, 1.0),
        ]
        .iter()
        .map(|&(x, y)| Vertex {
            src: v(x, y),
            dst: v(x, y),
        })
        .collect(),
        triangles: vec![[0, 1, 2], [3, 4, 5]],
    }
}

#[test]
fn outside_hole_nearest_picks_closer_triangle() {
    let w = Warp::new(
        two_far_triangles(),
        &with(|c| c.outside = Outside::ClampNearest),
    )
    .unwrap();
    assert_eq!(w.locate_dst(v(1.5, 0.0)).unwrap().triangle, 0);
    assert_eq!(w.locate_dst(v(3.5, 0.0)).unwrap().triangle, 1);
}

#[test]
fn outside_hole_equidistant_goes_to_lowest_index() {
    let w = Warp::new(
        two_far_triangles(),
        &with(|c| c.outside = Outside::ClampNearest),
    )
    .unwrap();
    // x = 2.5 is 1.5 from (1,0) and 1.5 from (4,0).
    let l = w.locate_dst(v(2.5, 0.0)).unwrap();
    assert_eq!(l.triangle, 0);
    assert_eq!(l.bary, [0.0, 1.0, 0.0]);
}

#[test]
fn outside_extend_in_hole() {
    let w = Warp::new(
        two_far_triangles(),
        &with(|c| c.outside = Outside::ExtendAffine),
    )
    .unwrap();
    assert_eq!(w.inverse(v(3.5, 0.0)), Some(v(3.5, 0.0)));
}

// ---- grid structure

#[test]
fn grid_dims_per_triangle_sqrt() {
    let l = Locator::build(&grid(4, 4, Diagonal::Forward), Space::Dst, &cfg());
    assert_eq!(l.grid_dims(), (5, 5));
    let l = Locator::build(&grid(1, 1, Diagonal::Forward), Space::Dst, &cfg());
    assert_eq!(l.grid_dims(), (1, 1));
    let l = Locator::build(&grid(2, 2, Diagonal::Forward), Space::Dst, &cfg());
    assert_eq!(l.grid_dims(), (2, 2));
}

#[test]
fn grid_dims_fixed_and_clamped() {
    let m = grid(2, 2, Diagonal::Forward);
    let l = Locator::build(&m, Space::Dst, &with(|c| c.grid = GridRes::Fixed(3, 2)));
    assert_eq!(l.grid_dims(), (3, 2));
    let l = Locator::build(&m, Space::Dst, &with(|c| c.grid = GridRes::Fixed(0, 0)));
    assert_eq!(l.grid_dims(), (1, 1));
}

#[test]
fn grid_cell_rule_far_point_clamped() {
    let l = Locator::build(&grid(4, 4, Diagonal::Forward), Space::Dst, &cfg());
    assert!(l.cell_len(v(-100.0, -100.0)) >= 1);
    assert_eq!(l.cell_len(v(-100.0, -100.0)), l.cell_len(v(0.0, 0.0)));
    assert_eq!(l.cell_len(v(1e9, 1e9)), l.cell_len(v(4.0, 4.0)));
}

#[test]
fn grid_cell_lists_are_short() {
    let l = Locator::build(&grid(8, 8, Diagonal::Forward), Space::Dst, &cfg());
    for i in 0..8 {
        assert!(l.cell_len(v(i as f64 + 0.5, i as f64 + 0.5)) <= 12);
    }
}

#[test]
fn grid_fixed_one_cell_same_results() {
    let m = grid(3, 3, Diagonal::Alternating);
    let a = Warp::new(m.clone(), &cfg()).unwrap();
    let b = Warp::new(m, &with(|c| c.grid = GridRes::Fixed(1, 1))).unwrap();
    for i in 0..=12 {
        for j in 0..=12 {
            let p = v(i as f64 / 4.0, j as f64 / 4.0);
            assert_eq!(a.inverse(p), b.inverse(p));
        }
    }
}

#[test]
fn locator_space_reported() {
    let l = Locator::build(&grid(1, 1, Diagonal::Forward), Space::Src, &cfg());
    assert_eq!(l.space(), Space::Src);
}

#[test]
fn locator_find_none_outside_box() {
    let l = Locator::build(&grid(2, 2, Diagonal::Forward), Space::Dst, &cfg());
    assert_eq!(l.find(v(5.0, 5.0)), None);
    assert_eq!(l.find(v(-0.5, 1.0)), None);
    assert!(l.find(v(1.0, 1.0)).is_some());
}

#[test]
fn single_point_extent_mesh_does_not_panic() {
    let m = scale_tri();
    let w = Warp::new(m, &with(|c| c.grid = GridRes::Fixed(7, 3))).unwrap();
    assert!(w.inverse(v(0.5, 0.5)).is_some());
}

// ---- reference comparison on a perturbed mesh

fn perturbed() -> Mesh {
    let mut m = grid_mesh(6, 5, 0.0, 0.0, 12.0, 10.0, Diagonal::Alternating);
    let mut r = Lcg(7);
    for vx in &mut m.vertices {
        vx.dst.x += (r.next() - 0.5) * 0.6;
        vx.dst.y += (r.next() - 0.5) * 0.6;
    }
    m
}

fn slow_ref(m: &Mesh, p: Vec2) -> Vec<(usize, Vec2)> {
    // Independent: solve p - a = s (b - a) + t (c - a) by the 2x2 inverse matrix.
    let mut out = Vec::new();
    for t in 0..m.triangles.len() {
        let [a, b, c] = m.corners(t, Space::Dst);
        let [sa, sb, sc] = m.corners(t, Space::Src);
        let (m00, m01, m10, m11) = (b.x - a.x, c.x - a.x, b.y - a.y, c.y - a.y);
        let det = m00 * m11 - m01 * m10;
        let (rx, ry) = (p.x - a.x, p.y - a.y);
        let s = (m11 * rx - m01 * ry) / det;
        let u = (-m10 * rx + m00 * ry) / det;
        if s > 1e-9 && u > 1e-9 && s + u < 1.0 - 1e-9 {
            let q = v(
                sa.x + s * (sb.x - sa.x) + u * (sc.x - sa.x),
                sa.y + s * (sb.y - sa.y) + u * (sc.y - sa.y),
            );
            out.push((t, q));
        }
    }
    out
}

#[test]
fn random_points_match_slow_reference_all_inside_configs() {
    let m = perturbed();
    let mut checked = 0;
    for c in config_space() {
        if c.allow_degenerate {
            continue;
        }
        let w = Warp::new(m.clone(), &c).unwrap();
        let mut r = Lcg(99);
        for _ in 0..200 {
            let p = v(0.7 + r.next() * 10.6, 0.7 + r.next() * 8.6);
            let want = slow_ref(&m, p);
            if want.len() != 1 {
                continue;
            }
            let l = w.locate_dst(p).unwrap();
            assert_eq!(l.triangle, want[0].0);
            let got = w.inverse(p).unwrap();
            assert!((got.x - want[0].1.x).abs() < 1e-9 && (got.y - want[0].1.y).abs() < 1e-9);
            checked += 1;
        }
    }
    assert!(checked > 2000);
}

#[test]
fn forward_then_inverse_roundtrip_perturbed() {
    let m = perturbed();
    let w = Warp::new(m, &cfg()).unwrap();
    let mut r = Lcg(5);
    let mut n = 0;
    for _ in 0..300 {
        let p = v(1.0 + r.next() * 10.0, 1.0 + r.next() * 8.0);
        if let Some(d) = w.forward(p) {
            if let Some(back) = w.inverse(d) {
                assert!((back.x - p.x).abs() < 1e-9 && (back.y - p.y).abs() < 1e-9);
                n += 1;
            }
        }
    }
    assert!(n > 200);
}

#[test]
fn bary_forms_agree_closely() {
    let m = perturbed();
    let a = Warp::new(m.clone(), &cfg()).unwrap();
    let b = Warp::new(m, &with(|c| c.bary = BaryForm::Cramer)).unwrap();
    let mut r = Lcg(3);
    for _ in 0..200 {
        let p = v(1.0 + r.next() * 10.0, 1.0 + r.next() * 8.0);
        let (x, y) = (a.inverse(p).unwrap(), b.inverse(p).unwrap());
        assert!((x.x - y.x).abs() < 1e-12 && (x.y - y.y).abs() < 1e-12);
    }
}

// ---- closure, configuration, determinism

#[test]
fn closure_matches_inverse() {
    let w = Warp::new(scale_tri(), &cfg()).unwrap();
    let f = w.inverse_closure();
    assert_eq!(f(1.0, 1.0), Some((0.5, 0.5)));
    assert_eq!(f(9.0, 9.0), None);
}

#[test]
fn closure_with_clamp_never_none() {
    let w = Warp::new(scale_tri(), &with(|c| c.outside = Outside::ClampNearest)).unwrap();
    let f = w.inverse_closure();
    assert!(f(100.0, -50.0).is_some());
}

#[test]
fn default_config_values() {
    let c = Config::default();
    assert_eq!(c.tie, TieRule::LowestIndex);
    assert_eq!(c.outside, Outside::None);
    assert_eq!(c.winding, Winding::Any);
    assert_eq!(c.bary, BaryForm::EdgeFunctions);
    assert!(!c.allow_degenerate);
    assert_eq!(c.grid, GridRes::PerTriangleSqrt);
}

#[test]
fn config_space_size_and_distinct() {
    let s = config_space();
    assert_eq!(s.len(), 96);
    for i in 0..s.len() {
        for j in i + 1..s.len() {
            assert_ne!(s[i], s[j]);
        }
    }
    assert!(s.contains(&Config::default()));
}

#[test]
fn every_config_builds_and_maps_identity() {
    for c in config_space() {
        let w = Warp::new(grid(2, 2, Diagonal::Forward), &c).unwrap();
        assert_eq!(w.inverse(v(0.5, 1.5)), Some(v(0.5, 1.5)));
    }
}

#[test]
fn determinism_bit_identical_runs() {
    let run = || {
        let w = Warp::new(perturbed(), &cfg()).unwrap();
        let mut r = Lcg(11);
        (0..100)
            .filter_map(|_| w.inverse(v(1.0 + r.next() * 10.0, 1.0 + r.next() * 8.0)))
            .flat_map(|q| [q.x.to_bits(), q.y.to_bits()])
            .collect::<Vec<u64>>()
    };
    let (a, b) = (run(), run());
    assert!(a.len() > 100);
    assert_eq!(a, b);
}

#[test]
fn determinism_fixed_bit_patterns() {
    let w = Warp::new(scale_tri(), &cfg()).unwrap();
    let q = w.inverse(v(1.0, 0.5)).unwrap();
    assert_eq!(
        (q.x.to_bits(), q.y.to_bits()),
        (0x3FE0_0000_0000_0000, 0x3FD0_0000_0000_0000)
    );
    let q = w.forward(v(0.5, 0.25)).unwrap();
    assert_eq!(
        (q.x.to_bits(), q.y.to_bits()),
        (0x3FF0_0000_0000_0000, 0x3FE0_0000_0000_0000)
    );
}

#[test]
fn mesh_accessor_and_translate() {
    let w = Warp::new(grid(1, 1, Diagonal::Forward), &cfg()).unwrap();
    assert_eq!(w.mesh().triangles.len(), 2);
    let mut m = w.mesh().clone();
    m.translate_dst(0.5, 0.25);
    assert_eq!(m.vertices[0].dst, v(0.5, 0.25));
    assert_eq!(m.vertices[0].src, v(0.0, 0.0));
}

#[test]
fn locate_src_uses_source_positions() {
    let mut m = grid(2, 2, Diagonal::Forward);
    m.translate_dst(10.0, 10.0);
    let w = Warp::new(m, &cfg()).unwrap();
    assert!(w.locate_src(v(1.0, 1.0)).is_some());
    assert!(w.locate_dst(v(1.0, 1.0)).is_none());
    assert!(w.locate_dst(v(11.0, 11.0)).is_some());
}

#[test]
fn cross_operation_order() {
    assert_eq!(cross(v(1.0, 2.0), v(3.0, 4.0)), -2.0);
}

#[test]
fn grid_dims_perfect_square_triangle_count() {
    // 4 triangles: n = 2; 18 triangles: n = 4 (not 3 or 5).
    let l = Locator::build(&grid(1, 2, Diagonal::Forward), Space::Dst, &cfg());
    assert_eq!(l.grid_dims(), (2, 2));
    let l = Locator::build(&grid(3, 3, Diagonal::Forward), Space::Dst, &cfg());
    assert_eq!(l.grid_dims(), (4, 4));
}

#[test]
fn outside_clamp_interior_of_edge_fractions() {
    let w = Warp::new(scale_tri(), &with(|c| c.outside = Outside::ClampNearest)).unwrap();
    assert_eq!(w.inverse(v(0.5, -1.0)), Some(v(0.25, 0.0)));
    assert_eq!(w.inverse(v(1.5, -1.0)), Some(v(0.75, 0.0)));
    assert_eq!(w.inverse(v(-1.0, 0.5)), Some(v(0.0, 0.25)));
}
