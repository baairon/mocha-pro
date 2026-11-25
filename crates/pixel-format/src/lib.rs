//! `pixel-format`: generic conversion of pixel samples between integer and floating point depths,
//! with every choice that a conversion can make held as explicit configuration.
//!
//! Published definitions the crate follows:
//!
//! - After Effects SDK Guide, pixel formats: 8 bit channels run 0 to 255, 16 bit channels run 0 to
//!   32768, float channels are nominally 0 to 1 and unclamped; the channel order
//!   in memory is ARGB for After Effects and BGRA for Premiere; colour is premultiplied by alpha by
//!   default.
//! - IEEE 754-2019 section 4.3: the rounding directions roundTiesToEven, roundTowardZero,
//!   roundTowardPositive and roundTowardNegative; round half up (`floor(x + 0.5)`) is added.
//!
//! Nothing in the crate has a hidden default. `Config::default_choice` is a documented starting
//! point and states no fact about any product.
//!
//! The pieces: `mode` (the choice types), `convert` (integer to float, float to integer and the
//! multiply add rescale), `alpha` (premultiply and unpremultiply), `config` (one choice of each,
//! its canonical text, its parser and the enumeration of a space of choices) and `layout` (pixel
//! orders and worlds with a row stride). The storage depth of `frame_io` is reused through
//! `Depth::storage` and `Depth::from_storage`.

pub mod alpha;
pub mod config;
pub mod convert;
pub mod layout;
pub mod mode;

pub use alpha::{
    premultiply_f32, premultiply_int, unpremultiply_f32, unpremultiply_int, ZeroAlpha,
};
pub use config::{AlphaOp, Config, ParseError, Space};
pub use convert::{float_to_int, int_to_float, FloatToInt, IntToFloat, Rescale};
pub use layout::{LayoutError, Order, WorldLayout};
pub use mode::{Clamp, Depth, Precision, PremulOrder, Rounding};
