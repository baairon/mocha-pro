//! Parameter definitions, the predicted-table file format and the pairing comparison.

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
