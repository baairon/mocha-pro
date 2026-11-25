//! `key-eval`: evaluation of keyframed values at a time in f64.
//!
//! A [`KeyList`] holds keys with a time, a value of `N` components and an interpolation [`Kind`] (hold,
//! linear, cubic Hermite, cubic Bezier). [`KeyList::evaluate`] returns the value at a time under a
//! [`Config`]. Every choice a calibration may need to vary is a field of [`Config`], and
//! [`Config::enumerate`] lists the whole configuration space.
//!
//! Only add, subtract, multiply, divide and square root are used, never a fused multiply add, and every
//! loop runs a fixed number of times, so the results are the same on every platform that follows IEEE 754.
//! The crate has no dependency and no unsafe code.

#![forbid(unsafe_code)]

pub mod config;
pub mod eval;
pub mod key;
pub mod solve;

pub use config::{
    Axes, Clamp, Coincident, Config, Empty, HandleMode, NewtonStart, Outside, Side, Snap, Solver,
    VectorMode,
};
pub use key::{Key, KeyError, KeyList, Kind};
