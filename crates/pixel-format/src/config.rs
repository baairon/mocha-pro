//! One choice of every open setting, its canonical text and the enumeration of a space of choices.

use crate::alpha::{
    premultiply_f32, premultiply_int, unpremultiply_f32, unpremultiply_int, ZeroAlpha,
};
use crate::convert::{float_to_int, int_to_float, FloatToInt, IntToFloat};
use crate::mode::{Clamp, Depth, Precision, PremulOrder, Rounding};
use std::fmt;

/// An operation on colour by alpha inside a pixel conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlphaOp {
    /// Colour is only converted.
    None,
    /// Colour is multiplied by alpha.
    Premultiply,
    /// Colour is divided by alpha.
    Unpremultiply,
}

/// One choice of rounding, precision, clamp, zero alpha rule, premultiply order and the integer
/// value of full scale at 16 bit. (8 bit full scale is 255 by definition of the type.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Config {
    /// Rounding of float to integer, of the integer alpha operations.
    pub rounding: Rounding,
    /// Precision of the float arithmetic.
    pub precision: Precision,
    /// Clamp of float to integer.
    pub clamp: Clamp,
    /// Unpremultiply at alpha 0.
    pub zero_alpha: ZeroAlpha,
    /// Order of the alpha operation and the depth conversion.
    pub order: PremulOrder,
    /// Full scale at 16 bit.
    pub u16_max: u16,
}

/// A refused canonical text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot parse configuration: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

/// The candidate values of each setting; `Config::enumerate` walks their product.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Space {
    /// Candidate roundings.
    pub roundings: Vec<Rounding>,
    /// Candidate precisions.
    pub precisions: Vec<Precision>,
    /// Candidate clamps.
    pub clamps: Vec<Clamp>,
    /// Candidate zero alpha rules.
    pub zero_alphas: Vec<ZeroAlpha>,
    /// Candidate premultiply orders.
    pub orders: Vec<PremulOrder>,
    /// Candidate 16 bit full scale values.
    pub u16_maxes: Vec<u16>,
}

impl Space {
    /// Every value of every setting, with the two usual 16 bit full scales 32768 and 65535.
    pub fn full() -> Space {
        Space {
            roundings: Rounding::ALL.to_vec(),
            precisions: Precision::ALL.to_vec(),
            clamps: Clamp::ALL.to_vec(),
            zero_alphas: ZeroAlpha::ALL.to_vec(),
            orders: PremulOrder::ALL.to_vec(),
            u16_maxes: vec![32768, 65535],
        }
    }
}

impl Config {
    /// A documented starting point: half up, `f64`, range clamp with NaN to zero, zero colour at
    /// alpha 0, convert first, 16 bit full scale 32768. It claims nothing about any product.
    pub fn default_choice() -> Config {
        Config {
            rounding: Rounding::HalfUp,
            precision: Precision::F64,
            clamp: Clamp::RangeNanZero,
            zero_alpha: ZeroAlpha::ZeroColour,
            order: PremulOrder::ConvertFirst,
            u16_max: 32768,
        }
    }

    /// The 16 bit depth of this choice.
    pub fn u16_depth(&self) -> Depth {
        Depth::U16 { max: self.u16_max }
    }

    /// The integer to float rule for full scale `max`.
    pub fn int_to_float_rule(&self, max: u32) -> IntToFloat {
        IntToFloat {
            denom: f64::from(max),
            precision: self.precision,
        }
    }

    /// The float to integer rule for full scale `max`.
    pub fn float_to_int_rule(&self, max: u32) -> FloatToInt {
        FloatToInt {
            scale: f64::from(max),
            offset: 0.0,
            rounding: self.rounding,
            clamp: self.clamp,
            precision: self.precision,
        }
    }

    /// Converts one sample, given and returned as a raw bit pattern (`f32::to_bits` for float),
    /// from one depth to another. Integer to integer goes through a float; float to float is the
    /// identity.
    pub fn convert_sample(&self, from: Depth, to: Depth, bits: u32) -> u32 {
        let as_float = match from {
            Depth::F32 => f32::from_bits(bits),
            _ => int_to_float(bits, &self.int_to_float_rule(from.max_value())),
        };
        match to {
            Depth::F32 => as_float.to_bits(),
            _ => float_to_int(
                as_float,
                to.max_value(),
                &self.float_to_int_rule(to.max_value()),
            ),
        }
    }

    /// Applies an alpha operation to the colour samples of a pixel (R, G, B, A) at one depth. The
    /// alpha sample is never changed.
    fn alpha_op(&self, depth: Depth, px: [u32; 4], op: AlphaOp) -> [u32; 4] {
        if op == AlphaOp::None {
            return px;
        }
        let mut out = px;
        for c in out.iter_mut().take(3) {
            *c = match depth {
                Depth::F32 => {
                    let colour = f32::from_bits(*c);
                    let alpha = f32::from_bits(px[3]);
                    let r = match op {
                        AlphaOp::Premultiply => premultiply_f32(colour, alpha, self.precision),
                        _ => unpremultiply_f32(colour, alpha, self.precision, self.zero_alpha),
                    };
                    r.to_bits()
                }
                _ => {
                    let max = depth.max_value();
                    match op {
                        AlphaOp::Premultiply => premultiply_int(*c, px[3], max, self.rounding),
                        _ => unpremultiply_int(*c, px[3], max, self.rounding, self.zero_alpha),
                    }
                }
            };
        }
        out
    }

    /// Converts a pixel (R, G, B, A raw bit patterns) between depths with an alpha operation,
    /// in the order of `self.order`.
    pub fn convert_pixel(&self, from: Depth, to: Depth, px: [u32; 4], op: AlphaOp) -> [u32; 4] {
        let convert_all = |depth_from: Depth, depth_to: Depth, p: [u32; 4]| {
            [
                self.convert_sample(depth_from, depth_to, p[0]),
                self.convert_sample(depth_from, depth_to, p[1]),
                self.convert_sample(depth_from, depth_to, p[2]),
                self.convert_sample(depth_from, depth_to, p[3]),
            ]
        };
        match self.order {
            PremulOrder::ConvertFirst => self.alpha_op(to, convert_all(from, to, px), op),
            PremulOrder::AlphaFirst => convert_all(from, to, self.alpha_op(from, px, op)),
        }
    }

    /// One stable text line: `key=value` pairs in a fixed order joined by `;`.
    pub fn canonical(&self) -> String {
        format!(
            "rounding={};precision={};clamp={};zero_alpha={};order={};u16_max={}",
            self.rounding.name(),
            self.precision.name(),
            self.clamp.name(),
            self.zero_alpha.name(),
            self.order.name(),
            self.u16_max
        )
    }

    /// Parses the output of `canonical` (the exact six keys in the exact order).
    pub fn parse(text: &str) -> Result<Config, ParseError> {
        let parts: Vec<&str> = text.split(';').collect();
        if parts.len() != 6 {
            return Err(ParseError(format!(
                "expected 6 fields, found {}",
                parts.len()
            )));
        }
        let keys = [
            "rounding",
            "precision",
            "clamp",
            "zero_alpha",
            "order",
            "u16_max",
        ];
        let mut values = [""; 6];
        for (i, part) in parts.iter().enumerate() {
            let (k, v) = part
                .split_once('=')
                .ok_or_else(|| ParseError(format!("field {} has no '='", i + 1)))?;
            if k != keys[i] {
                return Err(ParseError(format!(
                    "field {} is '{k}', expected '{}'",
                    i + 1,
                    keys[i]
                )));
            }
            values[i] = v;
        }
        let bad = |what: &str, v: &str| ParseError(format!("bad {what} '{v}'"));
        let max_text = values[5];
        if max_text.is_empty()
            || !max_text.bytes().all(|b| b.is_ascii_digit())
            || (max_text.len() > 1 && max_text.starts_with('0'))
        {
            return Err(bad("u16_max", max_text));
        }
        Ok(Config {
            rounding: Rounding::parse(values[0]).ok_or_else(|| bad("rounding", values[0]))?,
            precision: Precision::parse(values[1]).ok_or_else(|| bad("precision", values[1]))?,
            clamp: Clamp::parse(values[2]).ok_or_else(|| bad("clamp", values[2]))?,
            zero_alpha: ZeroAlpha::parse(values[3]).ok_or_else(|| bad("zero_alpha", values[3]))?,
            order: PremulOrder::parse(values[4]).ok_or_else(|| bad("order", values[4]))?,
            u16_max: max_text.parse().map_err(|_| bad("u16_max", max_text))?,
        })
    }

    /// Every choice of the space, each once, in a fixed order (the first setting varies slowest;
    /// a repeated candidate is skipped).
    pub fn enumerate(space: &Space) -> impl Iterator<Item = Config> {
        let mut out: Vec<Config> = Vec::new();
        for &rounding in &space.roundings {
            for &precision in &space.precisions {
                for &clamp in &space.clamps {
                    for &zero_alpha in &space.zero_alphas {
                        for &order in &space.orders {
                            for &u16_max in &space.u16_maxes {
                                let c = Config {
                                    rounding,
                                    precision,
                                    clamp,
                                    zero_alpha,
                                    order,
                                    u16_max,
                                };
                                if !out.contains(&c) {
                                    out.push(c);
                                }
                            }
                        }
                    }
                }
            }
        }
        out.into_iter()
    }
}
