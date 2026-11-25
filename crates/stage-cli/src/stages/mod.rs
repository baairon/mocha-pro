//! The stages of the provider.
//!
//! A stage is one module that implements [`crate::registry::Stage`] and one line in the list of
//! its family module. The families keep the manifest in a stable order: the self test first,
//! then image operations, warps, rasterisation and data.
//!
//! | Module | Holds |
//! |---|---|
//! | [`selftest_copy`] | the self test stage |
//! | [`image`] | stages that compute an image from an image: resampling, depth, alpha, edges |
//! | [`warp`] | stages that move pixels under a mapping |
//! | [`raster`] | stages that turn outlines into coverage |
//! | [`data`] | stages whose result is a data file beside the frames |

pub mod data;
pub mod image;
pub mod raster;
pub mod selftest_copy;
pub mod warp;

use crate::registry::{Provider, Stage};
use crate::stage::{version_text, PROGRAM, PROVIDER_ID};

/// The provider with every stage of this repository, in manifest order.
pub fn provider() -> Provider {
    let mut stages: Vec<&'static dyn Stage> = vec![&selftest_copy::SelfTestCopy];
    stages.extend(image::stages());
    stages.extend(warp::stages());
    stages.extend(raster::stages());
    stages.extend(data::stages());
    Provider {
        id: PROVIDER_ID,
        program: PROGRAM,
        version: version_text(),
        stages,
    }
}
