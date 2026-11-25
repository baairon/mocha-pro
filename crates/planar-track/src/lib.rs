//! `planar-track`: direct intensity based tracking of a planar region through a grey level image
//! sequence, written from published methods in plain IEEE arithmetic.
//!
//! The pieces: [`image`] (pixel centre convention, bilinear sampling, gradients), [`pyramid`]
//! (Gaussian pyramid), [`model`] (motion models and poses), [`region`] (the tracked region),
//! [`edt`] (distance transform for boundary weights), [`align`] (the Lucas-Kanade family with
//! Levenberg-Marquardt damping for one level), [`ncc`] (coarse correlation search), [`config`]
//! (every choice as a field, and the enumeration of the discrete choices) and [`tracker`]
//! (frame to frame tracking and the correction API).
//!
//! Determinism: only IEEE addition, subtraction, multiplication, division, square root, floor and
//! ceiling are used, in the order written, with no fused multiply add and no library
//! transcendental function; results are bit for bit the same on every platform. There is no
//! unsafe code and no thread.

#![forbid(unsafe_code)]

pub mod align;
pub mod config;
pub mod edt;
pub mod error;
pub mod image;
pub mod linalg;
pub mod model;
pub mod ncc;
pub mod pyramid;
pub mod region;
pub mod tracker;

pub use align::StopReason;
pub use config::{
    Acceptance, BoundaryWeight, Config, ConfigSpace, Damping, Illumination, LevelParams, LmConfig,
    NccConfig, SelectionConfig, UpdateRule, Velocity,
};
pub use error::Error;
pub use image::Image;
pub use model::{Dof, Pose};
pub use pyramid::{Pyramid, PyramidConfig};
pub use region::Region;
pub use tracker::{FrameResult, PoseSource, Tracker};
