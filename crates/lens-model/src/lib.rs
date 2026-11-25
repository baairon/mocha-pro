//! `lens-model`: lens distortion models as pure coordinate maps in `f64`.
//!
//! The models are the polynomial radial model with up to three radial terms and two tangential
//! terms (Brown, "Decentering Distortion of Lenses", Photogrammetric Engineering 32(3), 1966; Brown,
//! "Close-Range Camera Calibration", Photogrammetric Engineering 37(8), 1971, section on radial and
//! decentering distortion), the one parameter division model (Fitzgibbon, "Simultaneous linear
//! estimation of multiple view geometry and lens distortion", CVPR 2001, section 2), and an
//! squeezed variant with a separate squeeze per axis, which is defined in this crate.
//!
//! Every convention is a field of [`Config`]: the model, the normalisation, the distortion centre,
//! the pixel aspect, which direction the model formula maps, and the inverse method with its fixed
//! iteration count. [`ConfigSpace`] enumerates the space of those choices.
//!
//! Results are bit identical on every platform: only addition, subtraction, multiplication,
//! division and square root are used, and no fused multiply add. The inverse iterations run a fixed
//! number of steps with no data dependent exit. The crate has no dependency and no unsafe code.

pub mod config;
pub mod model;
pub mod resample;

pub use config::{Config, ConfigSpace, ModelMaps, Normalisation};
pub use model::{InverseMethod, LensError, Model};
pub use resample::{Bounds, Direction};
