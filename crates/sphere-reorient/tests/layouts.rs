use sphere_reorient::cube::*;
use sphere_reorient::stereo::*;
use sphere_reorient::{Config, CubeConfig, CubeGrid, Face, Size, StereoLayout};
use std::collections::HashSet;

#[test]
fn config_space_size_and_uniqueness() {
    let all = Config::all();
    assert_eq!(all.len(), Config::COUNT);
    let set: HashSet<String> = all.iter().map(|c| format!("{c:?}")).collect();
    assert_eq!(set.len(), Config::COUNT);
}

#[test]
fn config_space_contains_the_default() {
    assert!(Config::all().contains(&Config::default()));
}

#[test]
fn config_space_order_is_fixed() {
    let a = Config::all();
    assert_eq!(a[0].lon_zero, 0.0);
    assert_eq!(a[Config::COUNT - 1].lon_zero, 0.75);
    assert_eq!(a, Config::all());
}

#[test]
fn cube_config_space() {
    let all = CubeConfig::all();
    assert_eq!(all.len(), CubeConfig::COUNT);
    assert!(all.contains(&CubeConfig::default()));
    let orders: HashSet<String> = all.iter().map(|c| format!("{:?}", c.order)).collect();
    assert_eq!(orders.len(), 720);
    let set: HashSet<String> = all.iter().map(|c| format!("{c:?}")).collect();
    assert_eq!(set.len(), CubeConfig::COUNT);
    assert_eq!(all[0].order, Face::ALL);
}

#[test]
fn every_cube_order_holds_each_face_once() {
    for c in CubeConfig::all().iter().step_by(7) {
        for f in Face::ALL {
            assert_eq!(c.order.iter().filter(|&&g| g == f).count(), 1);
        }
    }
}

#[test]
fn grid_dims_and_cells() {
    assert_eq!(CubeGrid::Row6.dims(), (6, 1));
    assert_eq!(CubeGrid::Column6.dims(), (1, 6));
    assert_eq!(CubeGrid::Grid3x2.dims(), (3, 2));
    assert_eq!(CubeGrid::Grid2x3.dims(), (2, 3));
    let cells = |g: CubeGrid| (0..6).map(|i| g.cell(i)).collect::<Vec<_>>();
    assert_eq!(
        cells(CubeGrid::Row6),
        vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]
    );
    assert_eq!(
        cells(CubeGrid::Column6),
        vec![(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]
    );
    assert_eq!(
        cells(CubeGrid::Grid3x2),
        vec![(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]
    );
    assert_eq!(
        cells(CubeGrid::Grid2x3),
        vec![(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)]
    );
}

fn cfg(grid: CubeGrid) -> CubeConfig {
    CubeConfig {
        grid,
        ..CubeConfig::default()
    }
}

#[test]
fn face_sizes() {
    let sz = |w, h| Size { w, h };
    assert_eq!(cfg(CubeGrid::Row6).face_size(sz(24, 4)), Some(4));
    assert_eq!(cfg(CubeGrid::Row6).face_size(sz(24, 5)), None);
    assert_eq!(cfg(CubeGrid::Row6).face_size(sz(25, 4)), None);
    assert_eq!(cfg(CubeGrid::Column6).face_size(sz(4, 24)), Some(4));
    assert_eq!(cfg(CubeGrid::Grid3x2).face_size(sz(12, 8)), Some(4));
    assert_eq!(cfg(CubeGrid::Grid3x2).face_size(sz(12, 4)), None);
    assert_eq!(cfg(CubeGrid::Grid2x3).face_size(sz(8, 12)), Some(4));
    assert_eq!(cfg(CubeGrid::Grid2x3).face_size(sz(0, 0)), None);
}

#[test]
fn face_directions_exact() {
    assert_eq!(face_to_dir(Face::PosX, 0.5, 0.25), [1.0, 0.25, -0.5]);
    assert_eq!(face_to_dir(Face::NegX, 0.5, 0.25), [-1.0, 0.25, 0.5]);
    assert_eq!(face_to_dir(Face::PosY, 0.5, 0.25), [0.5, 1.0, -0.25]);
    assert_eq!(face_to_dir(Face::NegY, 0.5, 0.25), [0.5, -1.0, 0.25]);
    assert_eq!(face_to_dir(Face::PosZ, 0.5, 0.25), [0.5, 0.25, 1.0]);
    assert_eq!(face_to_dir(Face::NegZ, 0.5, 0.25), [-0.5, 0.25, -1.0]);
}

#[test]
fn face_round_trip_inside_the_face() {
    for f in Face::ALL {
        for a in [-0.75, -0.5, 0.0, 0.25, 0.5, 0.75] {
            for b in [-0.75, -0.25, 0.0, 0.5, 0.75] {
                let (g, a2, b2) = dir_to_face(face_to_dir(f, a, b));
                assert_eq!((g, a2, b2), (f, a, b));
            }
        }
    }
}

#[test]
fn face_choice_scales_with_length() {
    let (f, a, b) = dir_to_face([0.0, 0.0, 8.0]);
    assert_eq!((f, a, b), (Face::PosZ, 0.0, 0.0));
    assert_eq!(dir_to_face([0.0, 0.0, 0.0]), (Face::PosZ, 0.0, 0.0));
    assert_eq!(dir_to_face([-3.0, 1.0, 2.0]).0, Face::NegX);
    assert_eq!(dir_to_face([0.5, -3.0, 2.0]).0, Face::NegY);
    assert_eq!(dir_to_face([0.5, 3.0, 2.0]).0, Face::PosY);
    assert_eq!(dir_to_face([0.5, 1.0, -2.0]).0, Face::NegZ);
}

#[test]
fn face_ties_prefer_x_then_y_then_z() {
    assert_eq!(dir_to_face([1.0, 1.0, 1.0]).0, Face::PosX);
    assert_eq!(dir_to_face([0.5, 1.0, 1.0]).0, Face::PosY);
    assert_eq!(dir_to_face([0.5, -1.0, -1.0]).0, Face::NegY);
    assert_eq!(dir_to_face([0.5, 0.5, -1.0]).0, Face::NegZ);
}

#[test]
fn cube_pixel_to_dir_centres() {
    let s = Size { w: 24, h: 4 };
    let c = cfg(CubeGrid::Row6);
    assert_eq!(pixel_to_dir(&c, s, 17.5, 1.5), Some([0.0, 0.0, 1.0]));
    assert_eq!(pixel_to_dir(&c, s, 1.5, 1.5), Some([1.0, 0.0, 0.0]));
    assert_eq!(pixel_to_dir(&c, s, 5.5, 1.5), Some([-1.0, 0.0, 0.0]));
}

#[test]
fn cube_pixel_to_dir_corner_pixel() {
    let s = Size { w: 24, h: 4 };
    assert_eq!(
        pixel_to_dir(&cfg(CubeGrid::Row6), s, 16.0, 0.0),
        Some([-0.75, 0.75, 1.0])
    );
}

#[test]
fn cube_pixel_to_dir_needs_square_faces() {
    assert_eq!(
        pixel_to_dir(&cfg(CubeGrid::Row6), Size { w: 24, h: 5 }, 1.0, 1.0),
        None
    );
    assert_eq!(
        dir_to_pixel(&cfg(CubeGrid::Row6), Size { w: 24, h: 5 }, [0.0, 0.0, 1.0]),
        None
    );
}

#[test]
fn cube_dir_to_pixel_centre() {
    let s = Size { w: 24, h: 4 };
    let p = dir_to_pixel(&cfg(CubeGrid::Row6), s, [0.0, 0.0, 1.0]).unwrap();
    assert_eq!((p.x, p.y, p.clamped, p.wrapped), (17.5, 1.5, false, false));
}

#[test]
fn cube_dir_to_pixel_clamps_at_the_face_edge() {
    let s = Size { w: 24, h: 4 };
    let p = dir_to_pixel(&cfg(CubeGrid::Row6), s, [1.0, 0.0, 1.0]).unwrap();
    // x/z = 1 gives u = 1, so lx = 3.5 which is beyond the last pixel centre 3.
    assert!(p.clamped);
    assert!(p.x <= 19.0);
}

#[test]
fn cube_grid_layouts_place_faces_in_cells() {
    let s = Size { w: 12, h: 8 };
    let c = cfg(CubeGrid::Grid3x2);
    let p = dir_to_pixel(&c, s, [0.0, 0.0, 1.0]).unwrap();
    // PosZ is slot 4: cell (1, 1), centre at (4 + 1.5, 4 + 1.5).
    assert_eq!((p.x, p.y), (5.5, 5.5));
    let q = dir_to_pixel(
        &cfg(CubeGrid::Grid2x3),
        Size { w: 8, h: 12 },
        [0.0, 0.0, 1.0],
    )
    .unwrap();
    assert_eq!((q.x, q.y), (1.5, 9.5));
}

#[test]
fn cube_custom_order_moves_the_face() {
    let mut c = cfg(CubeGrid::Row6);
    c.order = [
        Face::PosZ,
        Face::NegZ,
        Face::PosX,
        Face::NegX,
        Face::PosY,
        Face::NegY,
    ];
    let s = Size { w: 24, h: 4 };
    let p = dir_to_pixel(&c, s, [0.0, 0.0, 1.0]).unwrap();
    assert_eq!((p.x, p.y), (1.5, 1.5));
    let d = pixel_to_dir(&c, s, 1.5, 1.5).unwrap();
    assert_eq!(d, [0.0, 0.0, 1.0]);
}

#[test]
fn cube_face_turn_moves_the_corner() {
    let mut c = cfg(CubeGrid::Row6);
    c.turns[4] = 1;
    let s = Size { w: 24, h: 4 };
    let p = dir_to_pixel(&c, s, [-0.75, 0.75, 1.0]).unwrap();
    assert_eq!((p.x, p.y), (19.0, 0.0));
    assert_eq!(pixel_to_dir(&c, s, 19.0, 0.0), Some([-0.75, 0.75, 1.0]));
}

#[test]
fn cube_turn_four_is_no_turn() {
    let mut c = cfg(CubeGrid::Row6);
    c.turns = [4; 6];
    let s = Size { w: 24, h: 4 };
    assert_eq!(
        pixel_to_dir(&c, s, 16.0, 0.0),
        pixel_to_dir(&cfg(CubeGrid::Row6), s, 16.0, 0.0)
    );
}

#[test]
fn cube_round_trip_over_sampled_configurations() {
    for c in CubeConfig::all().iter().step_by(101) {
        let (w, h) = match c.grid {
            CubeGrid::Row6 => (24, 4),
            CubeGrid::Column6 => (4, 24),
            CubeGrid::Grid3x2 => (12, 8),
            CubeGrid::Grid2x3 => (8, 12),
        };
        let s = Size { w, h };
        for y in 0..h {
            for x in 0..w {
                let (x, y) = (f64::from(x), f64::from(y));
                let d = pixel_to_dir(c, s, x, y).unwrap();
                let p = dir_to_pixel(c, s, d).unwrap();
                assert!(
                    (p.x - x).abs() < 1e-12 && (p.y - y).abs() < 1e-12,
                    "{c:?} {x} {y} {p:?}"
                );
            }
        }
    }
}

#[test]
fn stereo_over_under_split() {
    let l = StereoLayout::OverUnder;
    assert_eq!(
        l.rect(10, 7, false, 0),
        Rect {
            x: 0,
            y: 0,
            w: 10,
            h: 3
        }
    );
    assert_eq!(
        l.rect(10, 7, false, 1),
        Rect {
            x: 0,
            y: 3,
            w: 10,
            h: 4
        }
    );
}

#[test]
fn stereo_side_by_side_split() {
    let l = StereoLayout::SideBySide;
    assert_eq!(
        l.rect(7, 4, false, 0),
        Rect {
            x: 0,
            y: 0,
            w: 3,
            h: 4
        }
    );
    assert_eq!(
        l.rect(7, 4, false, 1),
        Rect {
            x: 3,
            y: 0,
            w: 4,
            h: 4
        }
    );
}

#[test]
fn stereo_even_split_is_exact() {
    assert_eq!(
        StereoLayout::SideBySide.rect(8, 4, false, 1),
        Rect {
            x: 4,
            y: 0,
            w: 4,
            h: 4
        }
    );
    assert_eq!(
        StereoLayout::OverUnder.rect(8, 4, false, 1),
        Rect {
            x: 0,
            y: 2,
            w: 8,
            h: 2
        }
    );
}

#[test]
fn stereo_swap_exchanges_eyes() {
    let l = StereoLayout::SideBySide;
    assert_eq!(l.rect(8, 4, true, 0), l.rect(8, 4, false, 1));
    assert_eq!(l.rect(8, 4, true, 1), l.rect(8, 4, false, 0));
}

#[test]
fn stereo_mono_is_the_whole_image() {
    for eye in 0..3 {
        for swap in [false, true] {
            assert_eq!(
                StereoLayout::Mono.rect(9, 5, swap, eye),
                Rect {
                    x: 0,
                    y: 0,
                    w: 9,
                    h: 5
                }
            );
        }
    }
    assert_eq!(StereoLayout::Mono.views(), 1);
    assert_eq!(StereoLayout::OverUnder.views(), 2);
    assert_eq!(StereoLayout::SideBySide.views(), 2);
    assert_eq!(StereoLayout::ALL.len(), 3);
}

#[test]
fn stereo_rects_tile_the_image() {
    for l in [StereoLayout::OverUnder, StereoLayout::SideBySide] {
        for (w, h) in [(8, 6), (9, 7), (1, 1), (2, 3)] {
            let (a, b) = (l.rect(w, h, false, 0), l.rect(w, h, false, 1));
            assert_eq!(a.w * a.h + b.w * b.h, w * h);
        }
    }
}

#[test]
fn rect_local_global_and_contains() {
    let r = Rect {
        x: 4,
        y: 2,
        w: 3,
        h: 2,
    };
    assert_eq!(r.to_local(5.5, 3.0), (1.5, 1.0));
    assert_eq!(r.to_global(1.5, 1.0), (5.5, 3.0));
    assert!(r.contains(4.0, 2.0) && r.contains(6.9, 3.9));
    assert!(
        !r.contains(7.0, 2.0)
            && !r.contains(3.9, 2.0)
            && !r.contains(4.0, 4.0)
            && !r.contains(4.0, 1.9)
    );
}

#[test]
fn stereo_eye_feeds_the_equirect_map() {
    use sphere_reorient::equirect::pixel_to_lonlat;
    let cfg = Config::default();
    let r = StereoLayout::OverUnder.rect(8, 8, false, 1);
    let (lx, ly) = r.to_local(3.5, 4.0);
    let eye = Size { w: r.w, h: r.h };
    let l = pixel_to_lonlat(&cfg, eye, lx, ly);
    assert_eq!((l.lon, l.lat), (0.0, 0.1875));
}
