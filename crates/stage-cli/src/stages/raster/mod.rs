//! Stages that turn outlines into coverage.
//!
//! One `pub mod` line and one line in [`stages`] per stage. Three modules serve both stages:
//! [`shape`] reads the shape layer, [`coverage`] holds the parameters of the rasteriser and the
//! frame loop, and [`carry`] writes the one channel matte into the four channel frame.

pub mod carry;
pub mod coverage;
pub mod shape;

pub mod raster_contour;
pub mod raster_polygon;

use crate::registry::Stage;

/// The stages of this family, in manifest order.
pub fn stages() -> Vec<&'static dyn Stage> {
    vec![
        &raster_polygon::RasterPolygon,
        &raster_contour::RasterContour,
    ]
}
