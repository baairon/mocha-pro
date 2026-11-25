//! Parameter definitions, the predicted-table file format and the pairing comparison.
//!
//! The crate is `no_std` plus `alloc` when its default feature `std` is turned off (used by the
//! freestanding build of the effect shell); with the feature on it behaves as a normal std crate.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod compare;
pub mod extra;
pub mod flags;
pub mod kind;
pub mod table;

pub use compare::{
    normalize_name, pair_tables, FieldVerdict, MatchClass, PairKey, Pairing, ParamVerdict,
};
pub use extra::Extra;
pub use flags::ParamFlags;
pub use kind::ParamKind;
pub use table::{Cell, ParamDef, ParseError, Table, ValidateError};
