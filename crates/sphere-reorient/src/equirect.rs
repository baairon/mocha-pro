//! Equirectangular (latitude and longitude) maps. Snyder 1987, Equirectangular Projection: longitude
//! and latitude are linear in the image coordinates. Angles are in turns.
//!
//! Pixel coordinates are continuous: an integer `i` addresses the sample of pixel `i` under the
//! conventions of [`Config`]. Direction vectors are `[cos(lat) sin(lon), sin(lat), cos(lat) cos(lon)]`.
//!
//! Seam: a longitude is wrapped, so an output column lies in `[0, width)`. Poles: a row is clamped to
//! `[0, height - 1]`. [`PixelPos`] reports whether either happened.

use crate::config::{Config, LonDirection, NorthEdge, PixelCenter, PoleRows};
use crate::rotation::Mat3;
use crate::trig::{asin_turns, atan2_turns, sin_cos_turns};

/// Image size in pixels, both at least 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

/// Longitude in `[-0.5, 0.5)` turns and latitude in `[-0.25, 0.25]` turns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LonLat {
    pub lon: f64,
    pub lat: f64,
}

/// An output pixel coordinate with the seam and pole handling made explicit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelPos {
    pub x: f64,
    pub y: f64,
    /// The column was outside `[0, width)` before wrapping.
    pub wrapped: bool,
    /// The row was outside `[0, height - 1]` before clamping.
    pub clamped: bool,
}

fn col_offset(cfg: &Config) -> f64 {
    match cfg.pixel_center {
        PixelCenter::Centre => 0.5,
        PixelCenter::Corner => 0.0,
    }
}

/// Wrap `x` into `[0, period)`.
pub fn wrap(x: f64, period: f64) -> f64 {
    let r = x - (x / period).floor() * period;
    if r >= period || r < 0.0 {
        0.0
    } else {
        r
    }
}

/// Wrap a turn count into `[-0.5, 0.5)`.
pub fn wrap_turns(t: f64) -> f64 {
    t - (t + 0.5).floor()
}

fn v_of_row(cfg: &Config, h: u32, y: f64) -> f64 {
    match cfg.pole_rows {
        PoleRows::OnPole => {
            if h <= 1 {
                0.5
            } else {
                y / f64::from(h - 1)
            }
        }
        PoleRows::Inset => (y + 0.5) / f64::from(h),
    }
}

fn row_of_v(cfg: &Config, h: u32, v: f64) -> f64 {
    match cfg.pole_rows {
        PoleRows::OnPole => v * f64::from(h - 1),
        PoleRows::Inset => v * f64::from(h) - 0.5,
    }
}

/// Longitude and latitude of the sample at continuous pixel coordinate (x, y). The row is clamped
/// to `[0, height - 1]` and the longitude is wrapped.
pub fn pixel_to_lonlat(cfg: &Config, size: Size, x: f64, y: f64) -> LonLat {
    let u = (x + col_offset(cfg)) / f64::from(size.w);
    let d = u - cfg.lon_zero;
    let t = match cfg.lon_direction {
        LonDirection::IncreasingRight => d,
        LonDirection::IncreasingLeft => 0.0 - d,
    };
    let yc = y.clamp(0.0, f64::from(size.h - 1));
    let v = v_of_row(cfg, size.h, yc);
    let k = 0.5 - v;
    let lat = match cfg.north {
        NorthEdge::Top => k * 0.5,
        NorthEdge::Bottom => (0.0 - k) * 0.5,
    };
    LonLat {
        lon: wrap_turns(t),
        lat: lat.clamp(-0.25, 0.25),
    }
}

/// Pixel coordinate of a longitude and latitude, wrapping the seam and clamping the poles.
pub fn lonlat_to_pixel(cfg: &Config, size: Size, ll: LonLat) -> PixelPos {
    let t = match cfg.lon_direction {
        LonDirection::IncreasingRight => ll.lon,
        LonDirection::IncreasingLeft => 0.0 - ll.lon,
    };
    let u = cfg.lon_zero + t;
    let w = f64::from(size.w);
    let raw_x = u * w - col_offset(cfg);
    let x = wrap(raw_x, w);
    let wrapped = !(0.0..w).contains(&raw_x);
    let k = ll.lat * 2.0;
    let v = match cfg.north {
        NorthEdge::Top => 0.5 - k,
        NorthEdge::Bottom => 0.5 + k,
    };
    let raw_y = row_of_v(cfg, size.h, v);
    let top = f64::from(size.h - 1);
    let y = raw_y.clamp(0.0, top);
    let clamped = !(0.0..=top).contains(&raw_y);
    PixelPos {
        x,
        y,
        wrapped,
        clamped,
    }
}

/// Unit direction of a longitude and latitude.
pub fn lonlat_to_dir(ll: LonLat) -> [f64; 3] {
    let (slon, clon) = sin_cos_turns(ll.lon);
    let (slat, clat) = sin_cos_turns(ll.lat);
    [clat * slon, slat, clat * clon]
}

/// `sqrt(x^2 + z^2)` without overflow or underflow of the squares (the plain form is used whenever
/// the sum is a normal number far from both limits, so the usual results keep their bits).
fn horizontal_length(x: f64, z: f64) -> f64 {
    let h2 = x * x + z * z;
    if h2 > 1e-280 && h2 < 1e280 {
        return h2.sqrt();
    }
    let m = x.abs().max(z.abs());
    if m == 0.0 || !m.is_finite() {
        return m;
    }
    let (a, b) = (x / m, z / m);
    m * (a * a + b * b).sqrt()
}

/// Longitude and latitude of a direction (it need not be normalised). The zero vector gives (0, 0).
pub fn dir_to_lonlat(d: [f64; 3]) -> LonLat {
    let lon = atan2_turns(d[0], d[2]);
    let lat = atan2_turns(d[1], horizontal_length(d[0], d[2]));
    LonLat { lon, lat }
}

/// Latitude from the vertical component of a unit direction through the arc sine.
pub fn dir_to_lat_asin(d: [f64; 3]) -> f64 {
    asin_turns(d[1])
}

pub fn pixel_to_dir(cfg: &Config, size: Size, x: f64, y: f64) -> [f64; 3] {
    lonlat_to_dir(pixel_to_lonlat(cfg, size, x, y))
}

pub fn dir_to_pixel(cfg: &Config, size: Size, d: [f64; 3]) -> PixelPos {
    lonlat_to_pixel(cfg, size, dir_to_lonlat(d))
}

/// The inverse mapping a resampler needs: for an output pixel coordinate it returns the source
/// coordinate, by output pixel, direction, `rot` (used as `cfg.rotation_use` says), source pixel.
pub fn inverse_map(cfg: Config, dst: Size, src: Size, rot: Mat3) -> impl Fn(f64, f64) -> PixelPos {
    let m = rot.for_use(&cfg);
    move |x, y| {
        let d = m.apply(pixel_to_dir(&cfg, dst, x, y));
        dir_to_pixel(&cfg, src, d)
    }
}
