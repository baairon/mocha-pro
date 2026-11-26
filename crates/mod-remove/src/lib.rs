//! Clean-room implementation of a frame producer for layer removal, written from a specification.
//! Every stage the specification does not give returns `RemoveError::Unspecified`.

pub mod config;
pub mod fill;
pub mod geom;
pub mod host;
pub mod illum;
pub mod mix;
pub mod source_index;
pub mod spread;
pub mod types;

pub use config::FillConfig;
pub use types::*;
