//! Round trips, seam and pole cases, independent of the other test files.
use sphere_reorient::cube::{self, CubeConfig};
use sphere_reorient::equirect::*;
use sphere_reorient::trig::*;
use sphere_reorient::*;

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

fn ang(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

#[test]
fn direction_round_trip_100k() {
    let mut r = Rng(12345);
    let mut worst: f64 = 0.0;
    for _ in 0..100_000 {
        let ll = LonLat {
            lon: r.unit() - 0.5,
            lat: (r.unit() - 0.5) * 0.5,
        };
        let d = equirect::lonlat_to_dir(ll);
        let back = equirect::lonlat_to_dir(equirect::dir_to_lonlat(d));
        worst = worst.max(ang(d, back));
        let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        assert!((n - 1.0).abs() < 1e-15, "norm {n}");
    }
    assert!(worst < 1e-15, "worst chord {worst}");
}

#[test]
fn unnormalised_direction_scale_invariant() {
    let ll = equirect::dir_to_lonlat([1.0, 1.0, 1.0]);
    for k in [1e-200, 1e-160, 1e-100, 1e100, 1e160, 1e200] {
        let got = equirect::dir_to_lonlat([k, k, k]);
        assert!(
            (got.lon - ll.lon).abs() < 1e-14,
            "lon at scale {k}: {}",
            got.lon
        );
        assert!(
            (got.lat - ll.lat).abs() < 1e-14,
            "lat at scale {k}: {}",
            got.lat
        );
    }
}

#[test]
fn trig_odd_even_symmetry() {
    let mut r = Rng(777);
    for _ in 0..100_000 {
        let t = r.unit() * 10f64.powi(-((r.next() % 17) as i32));
        let (sp, cp) = sin_cos_turns(t);
        let (sn, cn) = sin_cos_turns(-t);
        assert_eq!(sp, -sn, "sin asymmetric at {t}");
        assert_eq!(cp, cn, "cos asymmetric at {t}");
    }
}

#[test]
fn trig_small_negative_keeps_relative_accuracy() {
    let t = -1e-17;
    let (s, _) = sin_cos_turns(t);
    let want = t * TAU;
    assert!(
        ((s - want) / want).abs() < 1e-14,
        "sin({t} turns) = {s}, want {want}"
    );
}

#[test]
fn pixel_round_trip_all_configs() {
    let size = Size { w: 64, h: 32 };
    for cfg in Config::all() {
        for y in 0..32 {
            for x in 0..64 {
                let p = pixel_to_dir(&cfg, size, f64::from(x), f64::from(y));
                let q = dir_to_pixel(&cfg, size, p);
                assert!(!q.clamped);
                let ex = (q.x - f64::from(x) + 32.0).rem_euclid(64.0) - 32.0;
                let cos_lat = (p[0] * p[0] + p[2] * p[2]).sqrt();
                // on a pole row the column is undefined
                if cos_lat > 1e-9 {
                    assert!(ex.abs() < 1e-8, "{cfg:?} x={x} y={y} got {}", q.x);
                }
                assert!(
                    (q.y - f64::from(y)).abs() < 1e-8,
                    "{cfg:?} x={x} y={y} got y {}",
                    q.y
                );
            }
        }
    }
}

#[test]
fn seam_and_poles() {
    let cfg = Config::default();
    let size = Size { w: 8, h: 4 };
    // direction behind the viewer, both signs of a zero x
    for x in [0.0, -0.0] {
        let ll = dir_to_lonlat([x, 0.0, -1.0]);
        assert_eq!(ll.lon.abs(), 0.5);
        let p = dir_to_pixel(&cfg, size, [x, 0.0, -1.0]);
        assert!(p.x >= 0.0 && p.x < 8.0);
    }
    // straight up and down
    let up = dir_to_pixel(&cfg, size, [0.0, 1.0, 0.0]);
    assert_eq!(up.y, 0.0);
    assert!(up.clamped);
    let dn = dir_to_pixel(&cfg, size, [0.0, -1.0, 0.0]);
    assert_eq!(dn.y, 3.0);
    // longitudes just either side of the seam (lon = 0.5 is column 7.5 of 8 centred samples)
    let e = 1e-12;
    let a = dir_to_pixel(&cfg, size, [e, 0.0, -1.0]);
    let b = dir_to_pixel(&cfg, size, [-e, 0.0, -1.0]);
    assert!((a.x - 7.5).abs() < 1e-9, "a {}", a.x);
    assert!((b.x - 7.5).abs() < 1e-9, "b {}", b.x);
}

#[test]
fn cube_round_trip_all_layouts() {
    for cfg in CubeConfig::all() {
        let (c, r) = cfg.grid.dims();
        let f = 4u32;
        let size = Size { w: c * f, h: r * f };
        for y in 0..size.h {
            for x in 0..size.w {
                let d = cube::pixel_to_dir(&cfg, size, f64::from(x), f64::from(y)).unwrap();
                let p = cube::dir_to_pixel(&cfg, size, d).unwrap();
                assert!(
                    (p.x - f64::from(x)).abs() < 1e-9 && (p.y - f64::from(y)).abs() < 1e-9,
                    "{cfg:?} {x},{y} -> {},{}",
                    p.x,
                    p.y
                );
            }
        }
    }
}

#[test]
fn cube_direction_round_trip_100k() {
    let cfg = CubeConfig::default();
    let size = Size { w: 6 * 512, h: 512 };
    let mut r = Rng(99);
    for _ in 0..100_000 {
        let d = [
            r.unit() * 2.0 - 1.0,
            r.unit() * 2.0 - 1.0,
            r.unit() * 2.0 - 1.0,
        ];
        let p = cube::dir_to_pixel(&cfg, size, d).unwrap();
        let d2 = cube::pixel_to_dir(&cfg, size, p.x, p.y).unwrap();
        let m = d.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let m2 = d2.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let n1 = [d[0] / m, d[1] / m, d[2] / m];
        let n2 = [d2[0] / m2, d2[1] / m2, d2[2] / m2];
        // within one pixel of the face plane (2/512)
        assert!(ang(n1, n2) < 2.0 / 512.0 * 1.5, "{d:?}");
    }
}

#[test]
fn cube_fractional_pixel_picks_cell_by_sample_centre() {
    // Row6, face size 4: the cell border sits at x = 3.5 (pixel centres at integers), so x = 3.75
    // belongs to the second face, near its left edge.
    let cfg = CubeConfig::default();
    let size = Size { w: 24, h: 4 };
    let d = cube::pixel_to_dir(&cfg, size, 3.75, 0.0).unwrap();
    assert_eq!(d, [-1.0, 0.75, -0.875]);
    let d = cube::pixel_to_dir(&cfg, size, 3.25, 0.0).unwrap();
    assert_eq!(d, [1.0, 0.75, -0.875]);
}
