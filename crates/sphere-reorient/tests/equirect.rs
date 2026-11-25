use sphere_reorient::config::*;
use sphere_reorient::equirect::*;
use sphere_reorient::{inverse_map, Angle, Config, LonLat, Mat3, PixelPos, Size};

const S84: Size = Size { w: 8, h: 4 };

fn corner() -> Config {
    Config {
        pixel_center: PixelCenter::Corner,
        ..Config::default()
    }
}

#[test]
fn default_centre_column_is_longitude_zero() {
    let l = pixel_to_lonlat(&Config::default(), S84, 3.5, 1.5);
    assert_eq!(l, LonLat { lon: 0.0, lat: 0.0 });
}

#[test]
fn default_first_column_longitude() {
    let l = pixel_to_lonlat(&Config::default(), S84, 0.0, 1.5);
    assert_eq!(l.lon, -0.4375);
}

#[test]
fn default_rows_latitude() {
    let c = Config::default();
    assert_eq!(pixel_to_lonlat(&c, S84, 3.5, 0.0).lat, 0.1875);
    assert_eq!(pixel_to_lonlat(&c, S84, 3.5, 3.0).lat, -0.1875);
    assert_eq!(pixel_to_lonlat(&c, S84, 3.5, 1.0).lat, 0.0625);
}

#[test]
fn corner_convention_columns() {
    let c = corner();
    assert_eq!(pixel_to_lonlat(&c, S84, 4.0, 1.5).lon, 0.0);
    assert_eq!(pixel_to_lonlat(&c, S84, 0.0, 1.5).lon, -0.5);
    assert_eq!(pixel_to_lonlat(&c, S84, 6.0, 1.5).lon, 0.25);
    assert_eq!(pixel_to_lonlat(&c, S84, 2.0, 1.5).lon, -0.25);
}

#[test]
fn corner_seam_column_wraps_to_minus_half() {
    // Column 8 is the seam repeated: its longitude is the same as column 0.
    let c = corner();
    assert_eq!(pixel_to_lonlat(&c, S84, 8.0, 0.0).lon, -0.5);
}

#[test]
fn lon_zero_at_first_column() {
    let c = Config {
        lon_zero: 0.0,
        ..corner()
    };
    assert_eq!(pixel_to_lonlat(&c, S84, 0.0, 0.0).lon, 0.0);
    assert_eq!(pixel_to_lonlat(&c, S84, 2.0, 0.0).lon, 0.25);
    assert_eq!(pixel_to_lonlat(&c, S84, 6.0, 0.0).lon, -0.25);
}

#[test]
fn lon_zero_quarter() {
    let c = Config {
        lon_zero: 0.25,
        ..corner()
    };
    assert_eq!(pixel_to_lonlat(&c, S84, 2.0, 0.0).lon, 0.0);
    assert_eq!(pixel_to_lonlat(&c, S84, 4.0, 0.0).lon, 0.25);
}

#[test]
fn direction_left_mirrors_longitude() {
    let c = Config {
        lon_direction: LonDirection::IncreasingLeft,
        ..corner()
    };
    assert_eq!(pixel_to_lonlat(&c, S84, 6.0, 0.0).lon, -0.25);
    assert_eq!(pixel_to_lonlat(&c, S84, 2.0, 0.0).lon, 0.25);
    let p = lonlat_to_pixel(
        &c,
        S84,
        LonLat {
            lon: 0.25,
            lat: 0.0,
        },
    );
    assert_eq!(p.x, 2.0);
}

#[test]
fn pole_rows_on_pole() {
    let c = Config {
        pole_rows: PoleRows::OnPole,
        ..Config::default()
    };
    assert_eq!(pixel_to_lonlat(&c, S84, 0.0, 0.0).lat, 0.25);
    assert_eq!(pixel_to_lonlat(&c, S84, 0.0, 3.0).lat, -0.25);
    assert_eq!(pixel_to_lonlat(&c, S84, 0.0, 1.5).lat, 0.0);
}

#[test]
fn north_at_bottom_flips_latitude() {
    let c = Config {
        north: NorthEdge::Bottom,
        pole_rows: PoleRows::OnPole,
        ..Config::default()
    };
    assert_eq!(pixel_to_lonlat(&c, S84, 0.0, 0.0).lat, -0.25);
    assert_eq!(pixel_to_lonlat(&c, S84, 0.0, 3.0).lat, 0.25);
    assert_eq!(
        lonlat_to_pixel(
            &c,
            S84,
            LonLat {
                lon: 0.0,
                lat: 0.25
            }
        )
        .y,
        3.0
    );
}

#[test]
fn rows_outside_the_image_clamp_to_the_pole_rows() {
    let c = Config::default();
    assert_eq!(
        pixel_to_lonlat(&c, S84, 0.0, -9.0),
        pixel_to_lonlat(&c, S84, 0.0, 0.0)
    );
    assert_eq!(
        pixel_to_lonlat(&c, S84, 0.0, 99.0),
        pixel_to_lonlat(&c, S84, 0.0, 3.0)
    );
}

#[test]
fn single_row_on_pole_is_the_equator() {
    let c = Config {
        pole_rows: PoleRows::OnPole,
        ..Config::default()
    };
    let s = Size { w: 8, h: 1 };
    assert_eq!(pixel_to_lonlat(&c, s, 0.0, 0.0).lat, 0.0);
}

#[test]
fn origin_maps_to_the_image_centre() {
    let p = lonlat_to_pixel(&Config::default(), S84, LonLat { lon: 0.0, lat: 0.0 });
    assert_eq!(
        p,
        PixelPos {
            x: 3.5,
            y: 1.5,
            wrapped: false,
            clamped: false
        }
    );
}

#[test]
fn seam_wraps_in_longitude_centre() {
    let c = Config::default();
    let a = lonlat_to_pixel(
        &c,
        S84,
        LonLat {
            lon: -0.5,
            lat: 0.0,
        },
    );
    assert_eq!((a.x, a.wrapped), (7.5, true));
    let b = lonlat_to_pixel(&c, S84, LonLat { lon: 0.5, lat: 0.0 });
    assert_eq!((b.x, b.wrapped), (7.5, false));
}

#[test]
fn seam_wraps_in_longitude_corner() {
    let c = corner();
    let a = lonlat_to_pixel(
        &c,
        S84,
        LonLat {
            lon: -0.5,
            lat: 0.0,
        },
    );
    assert_eq!((a.x, a.wrapped), (0.0, false));
    let b = lonlat_to_pixel(&c, S84, LonLat { lon: 0.5, lat: 0.0 });
    assert_eq!((b.x, b.wrapped), (0.0, true));
    let d = lonlat_to_pixel(
        &c,
        S84,
        LonLat {
            lon: 0.75,
            lat: 0.0,
        },
    );
    assert_eq!((d.x, d.wrapped), (2.0, true));
}

#[test]
fn pole_rows_clamp_inset() {
    let c = Config::default();
    let n = lonlat_to_pixel(
        &c,
        S84,
        LonLat {
            lon: 0.0,
            lat: 0.25,
        },
    );
    assert_eq!((n.y, n.clamped), (0.0, true));
    let s = lonlat_to_pixel(
        &c,
        S84,
        LonLat {
            lon: 0.0,
            lat: -0.25,
        },
    );
    assert_eq!((s.y, s.clamped), (3.0, true));
}

#[test]
fn pole_rows_land_on_the_edge_rows_on_pole() {
    let c = Config {
        pole_rows: PoleRows::OnPole,
        ..Config::default()
    };
    let n = lonlat_to_pixel(
        &c,
        S84,
        LonLat {
            lon: 0.0,
            lat: 0.25,
        },
    );
    assert_eq!((n.y, n.clamped), (0.0, false));
    let s = lonlat_to_pixel(
        &c,
        S84,
        LonLat {
            lon: 0.0,
            lat: -0.25,
        },
    );
    assert_eq!((s.y, s.clamped), (3.0, false));
}

#[test]
fn latitude_beyond_the_pole_is_clamped() {
    let c = Config {
        pole_rows: PoleRows::OnPole,
        ..Config::default()
    };
    let p = lonlat_to_pixel(&c, S84, LonLat { lon: 0.0, lat: 0.3 });
    assert_eq!((p.y, p.clamped), (0.0, true));
}

#[test]
fn wrap_helpers() {
    assert_eq!(wrap(-0.5, 8.0), 7.5);
    assert_eq!(wrap(8.0, 8.0), 0.0);
    assert_eq!(wrap(16.25, 8.0), 0.25);
    assert_eq!(wrap(0.0, 8.0), 0.0);
    assert_eq!(wrap(-1e-20, 8.0), 0.0);
    assert_eq!(wrap_turns(0.5), -0.5);
    assert_eq!(wrap_turns(-0.5), -0.5);
    assert_eq!(wrap_turns(0.75), -0.25);
    assert_eq!(wrap_turns(1.25), 0.25);
    assert_eq!(wrap_turns(0.0), 0.0);
}

#[test]
fn direction_vectors_at_axes_are_exact() {
    let d = |lon, lat| lonlat_to_dir(LonLat { lon, lat });
    assert_eq!(d(0.0, 0.0), [0.0, 0.0, 1.0]);
    assert_eq!(d(0.25, 0.0), [1.0, 0.0, 0.0]);
    assert_eq!(d(0.5, 0.0), [0.0, 0.0, -1.0]);
    assert_eq!(d(-0.25, 0.0), [-1.0, 0.0, 0.0]);
    assert_eq!(d(0.0, 0.25), [0.0, 1.0, 0.0]);
    assert_eq!(d(0.3, -0.25), [0.0, -1.0, 0.0]);
}

#[test]
fn directions_have_unit_length() {
    for i in 0..60 {
        let d = lonlat_to_dir(LonLat {
            lon: -0.5 + f64::from(i) * 0.0167,
            lat: -0.25 + f64::from(i) * 0.0083,
        });
        assert!((d[0] * d[0] + d[1] * d[1] + d[2] * d[2] - 1.0).abs() < 1e-15);
    }
}

#[test]
fn dir_to_lonlat_exact_inputs() {
    assert_eq!(
        dir_to_lonlat([1.0, 0.0, 0.0]),
        LonLat {
            lon: 0.25,
            lat: 0.0
        }
    );
    assert_eq!(
        dir_to_lonlat([0.0, 0.0, -1.0]),
        LonLat { lon: 0.5, lat: 0.0 }
    );
    assert_eq!(
        dir_to_lonlat([-1.0, 0.0, 0.0]),
        LonLat {
            lon: -0.25,
            lat: 0.0
        }
    );
    assert_eq!(
        dir_to_lonlat([0.0, 1.0, 0.0]),
        LonLat {
            lon: 0.0,
            lat: 0.25
        }
    );
    assert_eq!(
        dir_to_lonlat([0.0, -1.0, 0.0]),
        LonLat {
            lon: 0.0,
            lat: -0.25
        }
    );
    assert_eq!(
        dir_to_lonlat([1.0, 1.0, 0.0]),
        LonLat {
            lon: 0.25,
            lat: 0.125
        }
    );
    assert_eq!(
        dir_to_lonlat([0.0, 0.0, 0.0]),
        LonLat { lon: 0.0, lat: 0.0 }
    );
}

#[test]
fn dir_to_lonlat_ignores_length() {
    assert_eq!(
        dir_to_lonlat([0.0, 0.0, 5.0]),
        dir_to_lonlat([0.0, 0.0, 1.0])
    );
    assert_eq!(
        dir_to_lonlat([2.0, 2.0, 2.0]),
        dir_to_lonlat([0.5, 0.5, 0.5])
    );
}

#[test]
fn asin_latitude_agrees_with_atan2_latitude() {
    for i in -9..=9 {
        let l = LonLat {
            lon: 0.1,
            lat: f64::from(i) * 0.0271,
        };
        let d = lonlat_to_dir(l);
        assert!((dir_to_lat_asin(d) - dir_to_lonlat(d).lat).abs() < 2e-15);
    }
}

#[test]
fn lonlat_direction_round_trip_exact_inputs() {
    for (lon, lat) in [
        (0.0, 0.0),
        (0.25, 0.0),
        (-0.25, 0.0),
        (0.5, 0.0),
        (0.0, 0.25),
        (0.0, -0.25),
    ] {
        let back = dir_to_lonlat(lonlat_to_dir(LonLat { lon, lat }));
        if lat == 0.0 {
            assert_eq!(back, LonLat { lon, lat });
        } else {
            assert_eq!(back.lat, lat);
        }
    }
}

#[test]
fn lonlat_direction_round_trip_general() {
    for i in 0..200 {
        let ll = LonLat {
            lon: -0.499 + f64::from(i) * 0.00493,
            lat: -0.249 + f64::from(i) * 0.0024,
        };
        let b = dir_to_lonlat(lonlat_to_dir(ll));
        assert!((b.lon - ll.lon).abs() < 1e-15 && (b.lat - ll.lat).abs() < 1e-15);
    }
}

#[test]
fn pixel_round_trip_every_configuration() {
    let size = Size { w: 8, h: 5 };
    for cfg in Config::all() {
        for y in 0..5 {
            for x in 0..8 {
                let (x, y) = (f64::from(x), f64::from(y));
                let p = lonlat_to_pixel(&cfg, size, pixel_to_lonlat(&cfg, size, x, y));
                assert!(
                    (p.x - x).abs() < 1e-9 || (p.x - x).abs() > 8.0 - 1e-9,
                    "{cfg:?} {x} {y} -> {p:?}"
                );
                assert!((p.y - y).abs() < 1e-9, "{cfg:?} {x} {y} -> {p:?}");
            }
        }
    }
}

#[test]
fn pixel_dir_round_trip_default() {
    let size = Size { w: 16, h: 8 };
    let c = Config::default();
    for y in 0..8 {
        for x in 0..16 {
            let (x, y) = (f64::from(x), f64::from(y));
            let p = dir_to_pixel(&c, size, pixel_to_dir(&c, size, x, y));
            assert!((p.x - x).abs() < 1e-12 && (p.y - y).abs() < 1e-12);
            assert!(!p.clamped);
        }
    }
}

#[test]
fn identity_rotation_is_the_identity_map() {
    let size = Size { w: 12, h: 6 };
    for cfg in Config::all().iter().step_by(37) {
        let f = inverse_map(*cfg, size, size, Mat3::IDENTITY);
        for y in 0..6 {
            for x in 0..12 {
                let (x, y) = (f64::from(x), f64::from(y));
                let p = f(x, y);
                assert!((p.y - y).abs() < 1e-9);
                if cfg.pole_rows == PoleRows::OnPole && (y == 0.0 || y == 5.0) {
                    continue; // the longitude of a pole row is undefined
                }
                let dx = (p.x - x).abs();
                assert!(
                    dx < 1e-9 || (12.0 - dx).abs() < 1e-9,
                    "{cfg:?} {x} {y} {p:?}"
                );
            }
        }
    }
}

fn yaw90() -> Mat3 {
    Mat3::about_y(Angle::from_degrees(90.0))
}

fn equator_row(cfg: Config, rot: Mat3) -> Vec<f64> {
    let s = Size { w: 4, h: 5 };
    let f = inverse_map(cfg, s, s, rot);
    (0..4).map(|x| f(f64::from(x), 2.0).x).collect()
}

#[test]
fn quarter_yaw_shifts_columns_by_a_quarter_width_exactly() {
    let cfg = Config {
        pole_rows: PoleRows::OnPole,
        ..corner()
    };
    assert_eq!(equator_row(cfg, yaw90()), vec![1.0, 2.0, 3.0, 0.0]);
}

#[test]
fn quarter_yaw_transposed_shifts_the_other_way() {
    let cfg = Config {
        pole_rows: PoleRows::OnPole,
        rotation_use: RotationUse::Transposed,
        ..corner()
    };
    assert_eq!(equator_row(cfg, yaw90()), vec![3.0, 0.0, 1.0, 2.0]);
}

#[test]
fn quarter_yaw_keeps_rows_exactly() {
    let cfg = Config {
        pole_rows: PoleRows::OnPole,
        ..corner()
    };
    let s = Size { w: 4, h: 5 };
    let f = inverse_map(cfg, s, s, yaw90());
    for x in 0..4 {
        assert_eq!(f(f64::from(x), 2.0).y, 2.0);
    }
}

#[test]
fn quarter_yaw_mirrored_direction_shifts_the_other_way() {
    let cfg = Config {
        pole_rows: PoleRows::OnPole,
        lon_direction: LonDirection::IncreasingLeft,
        ..corner()
    };
    assert_eq!(equator_row(cfg, yaw90()), vec![3.0, 0.0, 1.0, 2.0]);
}

#[test]
fn quarter_yaw_wide_image_shifts_by_a_quarter_width() {
    let cfg = Config {
        pole_rows: PoleRows::Inset,
        ..Config::default()
    };
    let s = Size { w: 64, h: 8 };
    let f = inverse_map(cfg, s, s, yaw90());
    for x in 0..64 {
        let want = (f64::from(x) + 16.0) % 64.0;
        let got = f(f64::from(x), 3.5).x;
        let d = (got - want).abs();
        assert!(d < 1e-12 || (64.0 - d).abs() < 1e-12, "{x} {got} {want}");
    }
}

#[test]
fn half_turn_pitch_maps_poles_to_poles() {
    let cfg = Config {
        pole_rows: PoleRows::OnPole,
        ..corner()
    };
    let s = Size { w: 4, h: 5 };
    let rot = Mat3::about_x(Angle::from_degrees(180.0));
    let f = inverse_map(cfg, s, s, rot);
    for x in 0..4 {
        let n = f(f64::from(x), 0.0);
        assert_eq!((n.y, n.clamped), (4.0, false));
        let m = f(f64::from(x), 4.0);
        assert_eq!((m.y, m.clamped), (0.0, false));
    }
}

#[test]
fn half_turn_pitch_maps_the_equator_to_itself_mirrored() {
    let cfg = Config {
        pole_rows: PoleRows::OnPole,
        ..corner()
    };
    let s = Size { w: 4, h: 5 };
    let f = inverse_map(cfg, s, s, Mat3::about_x(Angle::from_degrees(180.0)));
    // (x, y, z) -> (x, -y, -z): longitude lon -> 0.5 - lon, the pole-free columns -0.5, -0.25, 0, 0.25 go to 0, -0.25, 0.5, 0.25 turns.
    let got: Vec<f64> = (0..4).map(|x| f(f64::from(x), 2.0).x).collect();
    assert_eq!(got, vec![2.0, 1.0, 0.0, 3.0]);
}

#[test]
fn half_turn_yaw_shifts_by_half_the_width() {
    let cfg = Config {
        pole_rows: PoleRows::OnPole,
        ..corner()
    };
    let rot = Mat3::about_y(Angle::from_degrees(180.0));
    assert_eq!(equator_row(cfg, rot), vec![2.0, 3.0, 0.0, 1.0]);
}

#[test]
fn closure_matches_manual_composition() {
    let cfg = Config {
        axis_order: AxisOrder::RollYawPitch,
        ..Config::default()
    };
    let rot = Mat3::from_yaw_pitch_roll(
        &cfg,
        Angle::from_degrees(20.0),
        Angle::from_degrees(-35.0),
        Angle::from_degrees(50.0),
    );
    let dst = Size { w: 32, h: 16 };
    let src = Size { w: 20, h: 10 };
    let f = inverse_map(cfg, dst, src, rot);
    for (x, y) in [(0.0, 0.0), (5.5, 3.0), (31.0, 15.0), (12.25, 7.75)] {
        let d = rot.apply(pixel_to_dir(&cfg, dst, x, y));
        assert_eq!(f(x, y), dir_to_pixel(&cfg, src, d));
    }
}

#[test]
fn closure_output_stays_in_range() {
    let cfg = Config::default();
    let rot = Mat3::from_yaw_pitch_roll(
        &cfg,
        Angle::from_degrees(77.0),
        Angle::from_degrees(61.0),
        Angle::from_degrees(-12.0),
    );
    let s = Size { w: 24, h: 12 };
    let f = inverse_map(cfg, s, s, rot);
    for y in 0..12 {
        for x in 0..24 {
            let p = f(f64::from(x), f64::from(y));
            assert!((0.0..24.0).contains(&p.x) && (0.0..=11.0).contains(&p.y));
        }
    }
}

#[test]
fn rotation_preserves_the_pixel_count_of_orbits() {
    // Rotating by yaw 90 four times is the identity on the equator row.
    let cfg = Config {
        pole_rows: PoleRows::OnPole,
        ..corner()
    };
    let s = Size { w: 4, h: 5 };
    let f = inverse_map(cfg, s, s, yaw90());
    for x0 in 0..4 {
        let mut x = f64::from(x0);
        for _ in 0..4 {
            x = f(x, 2.0).x;
        }
        assert_eq!(x, f64::from(x0));
    }
}

#[test]
fn results_are_deterministic() {
    let cfg = Config::default();
    let rot = Mat3::from_yaw_pitch_roll(
        &cfg,
        Angle::from_degrees(13.0),
        Angle::from_degrees(29.0),
        Angle::from_degrees(3.0),
    );
    let s = Size { w: 40, h: 20 };
    let run = || {
        let f = inverse_map(cfg, s, s, rot);
        let mut h: u64 = 0xcbf29ce484222325;
        for y in 0..20 {
            for x in 0..40 {
                let p = f(f64::from(x), f64::from(y));
                h = (h ^ p.x.to_bits()).wrapping_mul(0x100000001b3);
                h = (h ^ p.y.to_bits()).wrapping_mul(0x100000001b3);
            }
        }
        h
    };
    assert_eq!(run(), run());
    assert_eq!(run(), PIN_MAP_HASH);
}

const PIN_MAP_HASH: u64 = 15519978956201957532;
