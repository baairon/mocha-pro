//! Reorientation of equirectangular images as a pure coordinate map in `f64`.
//!
//! The crate has no dependency and no unsafe code. Every result is built only from addition,
//! subtraction, multiplication, division, square root, `floor` and `round`, which IEEE 754 fixes, and
//! never from a fused multiply add or a libm transcendental, so results are bit identical on every
//! platform. Sine, cosine, arc tangent and arc sine are implemented here ([`trig`]).
//!
//! Angles are expressed in turns (1.0 is a full circle) so that quarter and half turns are exact.
//!
//! Public sources for the definitions used:
//! - J. P. Snyder, Map Projections: A Working Manual, USGS Professional Paper 1395, 1987,
//!   section "Equirectangular Projection" (plate carree, pages 90 to 91): x = R (lon - lon0) cos(lat1),
//!   y = R lat, so longitude and latitude are linear in the image coordinates.
//! - Any text on rotation matrices, for example G. Strang, Introduction to Linear Algebra, section 2.6
//!   (rotation in a plane) and the elementary rotations about the coordinate axes.
//! - Taylor series of sine, cosine and arc tangent, for example M. Abramowitz and I. A. Stegun,
//!   Handbook of Mathematical Functions, 1964, sections 4.3.65, 4.3.66 and 4.4.42, and the arc tangent
//!   identity atan(x) = pi/4 + atan((x - 1)/(x + 1)) of section 4.4.34.

pub mod config;
pub mod cube;
pub mod equirect;
pub mod rotation;
pub mod stereo;
pub mod trig;

pub use config::{
    AngleSign, AxisOrder, Config, LonDirection, NorthEdge, PixelCenter, PoleRows, RotationUse,
};
pub use cube::{CubeConfig, CubeGrid, Face};
pub use equirect::{inverse_map, LonLat, PixelPos, Size};
pub use rotation::{Angle, Mat3};
pub use stereo::{Rect, StereoLayout};
