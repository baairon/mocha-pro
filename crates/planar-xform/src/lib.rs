//! `planar-xform`: a 3 by 3 planar projective transform (a homography) in `f64`, with every
//! convention stated and every choice that a caller may want to vary held in [`Config`].
//!
//! Sources: R. Hartley and A. Zisserman, "Multiple View Geometry in Computer Vision", 2nd
//! edition, Cambridge University Press, 2003: section 2.3 (projective transformations,
//! homogeneous coordinates and the divide), section 4.1 (the direct linear transformation from
//! point correspondences, with two equations per point and `h22 = 1`), and section 2.2.3
//! (the transform of a point is a matrix product followed by the divide). The inverse by the
//! adjugate is the cofactor formula of any linear algebra text (for example G. Strang,
//! "Linear Algebra and Its Applications", 4th edition, 2006, section 4.4).
//!
//! Bit exactness: only add, subtract, multiply, divide and comparisons are used, in a fixed
//! order, and never a fused multiply add, so equal inputs give equal bits on every platform.
//! There is no unsafe code and there are no dependencies.

mod homography;
mod pixel;

pub use homography::{Error, Homography};
pub use pixel::PixelSpace;

/// Whether a point is a column vector (`p' = M p`) or a row vector (`p' = p M`).
/// Hartley and Zisserman use columns; many graphics texts use rows. The stored matrix of a
/// row convention transform is the transpose of the column one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Convention {
    Column,
    Row,
}

/// What `apply` does when the homogeneous divisor `w` is not greater than `Config::divisor_min`
/// (a zero, negative or NaN divisor with the default minimum).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DivisorGuard {
    /// Return `None`.
    ReturnNone,
    /// Divide by the stated value instead of `w`.
    Clamp(f64),
    /// Return the undivided numerators `(x', y')` as the point.
    PassThrough,
}

/// Where the centre of the pixel of integer index `i` lies in continuous pixel coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PixelCenter {
    /// The centre is at `i`; the image extent runs from `-0.5` to `size - 0.5`.
    Integer,
    /// The centre is at `i + 0.5`; the image extent runs from `0` to `size`.
    Half,
}

/// Direction of the pixel row index against the normalised vertical coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum YAxis {
    /// Row 0 is at normalised `v = 0` and `v` grows with the row index.
    Down,
    /// Row 0 is at normalised `v = 1` and `v` falls as the row index grows.
    Up,
}

/// Every selectable choice. The default claims nothing about any application: column vectors,
/// return none on a bad divisor, a minimum of zero, half integer centres, Y down, square
/// pixels, and a singularity tolerance of `1e-12`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub convention: Convention,
    pub guard: DivisorGuard,
    /// The divisor is valid when it is strictly greater than this value.
    pub divisor_min: f64,
    pub center: PixelCenter,
    pub y_axis: YAxis,
    /// Width of a pixel divided by its height.
    pub pixel_aspect: f64,
    /// Relative pivot tolerance of the four point solve: a pivot whose magnitude is at most
    /// `tolerance` times the largest magnitude of the system is refused.
    pub singular_tolerance: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            convention: Convention::Column,
            guard: DivisorGuard::ReturnNone,
            divisor_min: 0.0,
            center: PixelCenter::Half,
            y_axis: YAxis::Down,
            pixel_aspect: 1.0,
            singular_tolerance: 1e-12,
        }
    }
}

impl Config {
    /// The configuration space: the product of both conventions, the guards (return none,
    /// pass through, and one clamp per entry of `clamps`), both centre conventions, both Y
    /// directions and one pixel aspect per entry of `aspects`, in that nesting order (the
    /// convention varies slowest). Other fields keep their defaults.
    /// The count is `2 * (2 + clamps.len()) * 2 * 2 * aspects.len()`.
    pub fn enumerate(clamps: &[f64], aspects: &[f64]) -> Vec<Config> {
        let mut guards = vec![DivisorGuard::ReturnNone, DivisorGuard::PassThrough];
        guards.extend(clamps.iter().map(|&c| DivisorGuard::Clamp(c)));
        let mut out = Vec::new();
        for convention in [Convention::Column, Convention::Row] {
            for &guard in &guards {
                for center in [PixelCenter::Integer, PixelCenter::Half] {
                    for y_axis in [YAxis::Down, YAxis::Up] {
                        for &pixel_aspect in aspects {
                            out.push(Config {
                                convention,
                                guard,
                                center,
                                y_axis,
                                pixel_aspect,
                                ..Config::default()
                            });
                        }
                    }
                }
            }
        }
        out
    }
}

/// A point in the plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
}

/// An axis aligned rectangle, `x0 <= x1` and `y0 <= y1` expected.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    /// Corners in the order `(x0,y0)`, `(x1,y0)`, `(x1,y1)`, `(x0,y1)`.
    pub fn corners(&self) -> [Point; 4] {
        [
            Point::new(self.x0, self.y0),
            Point::new(self.x1, self.y0),
            Point::new(self.x1, self.y1),
            Point::new(self.x0, self.y1),
        ]
    }

    /// The smallest rectangle holding all the points (`None` for an empty slice).
    pub fn bounding(points: &[Point]) -> Option<Rect> {
        let first = points.first()?;
        let mut r = Rect::new(first.x, first.y, first.x, first.y);
        for p in &points[1..] {
            if p.x < r.x0 {
                r.x0 = p.x;
            }
            if p.x > r.x1 {
                r.x1 = p.x;
            }
            if p.y < r.y0 {
                r.y0 = p.y;
            }
            if p.y > r.y1 {
                r.y1 = p.y;
            }
        }
        Some(r)
    }
}
