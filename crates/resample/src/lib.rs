//! Generic image resampling of `f32` buffers under an inverse mapping.
//!
//! See the crate README for the definitions, the cited sources and the configuration space.

pub mod border;
pub mod config;
pub mod engine;
pub mod kernel;

pub use border::{wrap, Border};
pub use config::{
    AccumOrder, AlphaMode, Config, ConfigSpace, Error, Filter, Footprint, Normalize, PixelCenter,
    TieRule,
};
pub use engine::{resample, resample_matrix, Image, Layout, Matrix3};
