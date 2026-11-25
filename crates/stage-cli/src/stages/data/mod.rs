//! Stages whose result is a data file beside the frames.
//!
//! One `pub mod` line and one line in [`stages`] per stage. [`sink`] is the helper every data
//! stage writes its file through.

pub mod sink;
pub mod track_planar;

use crate::registry::Stage;

/// The stages of this family, in manifest order.
pub fn stages() -> Vec<&'static dyn Stage> {
    vec![&track_planar::TrackPlanar]
}
