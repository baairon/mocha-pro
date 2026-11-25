//! `alpha-premultiply`: a generic stage. It multiplies the colour of each pixel by its alpha, or
//! divides it by its alpha, with the `pixel-format` crate, every choice of the operation given
//! as a parameter. It computes a published method under explicit configuration and claims
//! nothing about any product.
//!
//! Each pixel is given to `pixel_format::Config::convert_pixel` with the depth of the stream as
//! both the source and the target depth, so the stream keeps its depth. The alpha sample is
//! never changed by the alpha operation. A pixel is handled as four bit patterns, so a float
//! stream is neither clamped nor rounded by the stage itself.
//!
//! The frame contract of the convention carries straight alpha. The output of `op=premultiply`
//! is premultiplied and therefore outside the contract until a later stage divides the colour by
//! alpha again (`op=unpremultiply`). The manifest text of the stage says so.

use std::io::{Read, Write};

use pixel_format::{AlphaOp, Config, PremulOrder, ZeroAlpha};

use super::depth_quantise::{clamp, precision, rounding, stream_depth, STREAM_U16_MAX};
use super::shared::{choice, read_bits, sample_index, write_bits};
use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::params::Values;
use crate::pixels;
use crate::registry::{Geometry, Kind, Label, Plan, Stage, StageDef};
use crate::run::map_frames;

/// The alpha premultiply stage.
pub struct AlphaPremultiply;

/// What the stage declares. Every default of a configuration field is the default of
/// `pixel_format::Config::default_choice`.
pub static DEF: StageDef = StageDef {
    id: "alpha-premultiply",
    name: "Alpha premultiply",
    about: "Multiplies the colour of each pixel by its alpha, or divides it by its alpha, under an explicit rounding, precision, clamp, zero alpha and order rule. The frame contract carries straight alpha: the output of op=premultiply is premultiplied and outside the contract until a later stage divides by alpha again.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Preserve,
    depths: &DEPTHS,
    params: &[
        choice("op", "Operation", "premultiply", &["premultiply", "unpremultiply"], "Whether colour is multiplied by alpha or divided by it (AlphaOp)."),
        choice("rounding", "Rounding", "half-up", &["half-up", "ties-even", "truncate", "floor", "ceil"], "How a real number becomes an integer (Config::rounding)."),
        choice("precision", "Precision", "f64", &["f32", "f64"], "The floating point type of the arithmetic (Config::precision)."),
        choice("clamp", "Clamp", "range-nan-zero", &["none", "range", "range-nan-zero"], "What happens to a rounded value outside the range or to a value that is not a number (Config::clamp)."),
        choice("zero_alpha", "Zero alpha", "zero-colour", &["zero-colour", "keep-colour"], "What a division does where alpha is 0: the colour becomes 0, or stays (Config::zero_alpha)."),
        choice("order", "Order", "convert-first", &["convert-first", "alpha-first"], "Whether the samples are converted before or after the alpha operation (Config::order)."),
    ],
    layers: &[],
};

/// The configuration of the operation for a set of values. The 16 bit full scale is that of the
/// frame contract.
pub fn config(values: &Values) -> Config {
    Config {
        rounding: rounding(values),
        precision: precision(values),
        clamp: clamp(values),
        zero_alpha: match values.choice("zero_alpha") {
            "keep-colour" => ZeroAlpha::KeepColour,
            _ => ZeroAlpha::ZeroColour,
        },
        order: match values.choice("order") {
            "alpha-first" => PremulOrder::AlphaFirst,
            _ => PremulOrder::ConvertFirst,
        },
        u16_max: STREAM_U16_MAX,
    }
}

/// The alpha operation a set of values names.
pub fn alpha_op(values: &Values) -> AlphaOp {
    match values.choice("op") {
        "unpremultiply" => AlphaOp::Unpremultiply,
        _ => AlphaOp::Premultiply,
    }
}

impl Stage for AlphaPremultiply {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError> {
        let cfg = config(&plan.values);
        let op = alpha_op(&plan.values);
        let depth = plan.input.depth;
        let stream = stream_depth(depth);
        let frame_bytes = plan.input.buffer_len()?;
        map_frames(
            input,
            output,
            frame_bytes,
            frame_bytes,
            plan.frames,
            |_, frame, out| {
                let mut bits = read_bits(frame, depth);
                let pixel_count = bits.len() / pixels::CHANNELS;
                for pixel in 0..pixel_count {
                    let at = |channel| sample_index(depth, pixel_count, pixel, channel);
                    let made = cfg.convert_pixel(
                        stream,
                        stream,
                        [
                            bits[at(0)],
                            bits[at(1)],
                            bits[at(2)],
                            bits[at(pixels::ALPHA)],
                        ],
                        op,
                    );
                    for (channel, value) in made.into_iter().enumerate() {
                        bits[at(channel)] = value;
                    }
                }
                write_bits(&bits, depth, out);
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixel_format::{Clamp, Precision, Rounding};

    fn set(pairs: &[(&str, &str)]) -> Values {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Values::parse(&DEF, &pairs).unwrap()
    }

    #[test]
    fn the_defaults_are_the_starting_point_of_the_crate_at_the_full_scale_of_the_contract() {
        let values = Values::defaults(&DEF);
        assert_eq!(
            config(&values),
            Config {
                u16_max: STREAM_U16_MAX,
                ..Config::default_choice()
            }
        );
        assert_eq!(alpha_op(&values), AlphaOp::Premultiply);
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        assert_eq!(
            alpha_op(&set(&[("op", "premultiply")])),
            AlphaOp::Premultiply
        );
        assert_eq!(
            alpha_op(&set(&[("op", "unpremultiply")])),
            AlphaOp::Unpremultiply
        );
        assert_eq!(
            config(&set(&[("rounding", "ceil")])).rounding,
            Rounding::Ceil
        );
        assert_eq!(
            config(&set(&[("precision", "f32")])).precision,
            Precision::F32
        );
        assert_eq!(config(&set(&[("clamp", "none")])).clamp, Clamp::None);
        assert_eq!(
            config(&set(&[("zero_alpha", "zero-colour")])).zero_alpha,
            ZeroAlpha::ZeroColour
        );
        assert_eq!(
            config(&set(&[("zero_alpha", "keep-colour")])).zero_alpha,
            ZeroAlpha::KeepColour
        );
        assert_eq!(
            config(&set(&[("order", "convert-first")])).order,
            PremulOrder::ConvertFirst
        );
        assert_eq!(
            config(&set(&[("order", "alpha-first")])).order,
            PremulOrder::AlphaFirst
        );
    }

    #[test]
    fn the_manifest_text_says_that_the_output_is_outside_the_contract() {
        assert!(DEF.about.contains("straight alpha"));
        assert!(DEF.about.contains("outside the contract"));
    }
}
