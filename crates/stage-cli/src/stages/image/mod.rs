//! Stages that compute an image from an image.
//!
//! One `pub mod` line and one line in [`stages`] per stage.

pub mod alpha_premultiply;
pub mod depth_quantise;
pub mod resample_scale;
mod shared;
pub mod soft_edge_blur;
pub mod soft_edge_feather;

use crate::registry::Stage;

/// The stages of this family, in manifest order.
pub fn stages() -> Vec<&'static dyn Stage> {
    vec![
        &resample_scale::ResampleScale,
        &depth_quantise::DepthQuantise,
        &alpha_premultiply::AlphaPremultiply,
        &soft_edge_blur::SoftEdgeBlur,
        &soft_edge_feather::SoftEdgeFeather,
    ]
}
