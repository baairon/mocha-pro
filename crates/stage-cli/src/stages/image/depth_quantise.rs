//! `depth-quantise`: a generic stage. It snaps every sample of each frame to the values an
//! 8 bit or a 16 bit integer grid can hold, with the `pixel-format` crate, every choice of the
//! conversion given as a parameter. It computes a published method under explicit configuration
//! and claims nothing about any product.
//!
//! Each sample is converted from the depth of the stream to the integer depth `levels` and back
//! with `pixel_format::Config::convert_sample`. The stream keeps its depth: a stage of the
//! convention cannot change it. What changes is the set of values the stream carries.
//!
//! The depth of the stream is the depth of the frame contract: 8 bit with full scale 255, 16 bit
//! with full scale 65535, or float. The grid is 8 bit with full scale 255, or 16 bit with the
//! full scale `u16_max`. A sample is handled as a bit pattern, so a float stream is neither
//! clamped nor rounded by the stage itself; the `clamp` rule of the crate decides.

use std::io::{Read, Write};

use pixel_format::{Clamp, Config, Depth, Precision, Rounding};

use super::shared::{choice, int, read_bits, write_bits};
use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::params::Values;
use crate::registry::{Geometry, Kind, Label, Plan, Stage, StageDef};
use crate::run::map_frames;

/// The depth quantise stage.
pub struct DepthQuantise;

/// The full scale of a 16 bit sample of the frame contract.
pub const STREAM_U16_MAX: u16 = u16::MAX;

/// What the stage declares. `rounding`, `precision` and `clamp` have the defaults of
/// `pixel_format::Config::default_choice`; `u16_max` has the full scale of the frame contract.
pub static DEF: StageDef = StageDef {
    id: "depth-quantise",
    name: "Depth quantise",
    about: "Snaps every sample of each frame to the values of an 8 bit or 16 bit integer grid, by a conversion to that depth and back under an explicit rounding, precision and clamp rule; the stream keeps its depth.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Preserve,
    depths: &DEPTHS,
    params: &[
        choice("levels", "Levels", "8", &["8", "16"], "The integer depth of the grid the samples are snapped to."),
        int("u16_max", "16 bit full scale", (65535, 1, 65535), "The integer that stands for full scale on the 16 bit grid (Config::u16_max); read when levels is 16."),
        choice("rounding", "Rounding", "half-up", &["half-up", "ties-even", "truncate", "floor", "ceil"], "How a real number becomes an integer (Config::rounding)."),
        choice("precision", "Precision", "f64", &["f32", "f64"], "The floating point type of the arithmetic (Config::precision)."),
        choice("clamp", "Clamp", "range-nan-zero", &["none", "range", "range-nan-zero"], "What happens to a rounded value outside the grid or to a value that is not a number (Config::clamp)."),
    ],
    layers: &[],
};

/// The rounding a set of values names.
pub(crate) fn rounding(values: &Values) -> Rounding {
    match values.choice("rounding") {
        "ties-even" => Rounding::TiesEven,
        "truncate" => Rounding::Truncate,
        "floor" => Rounding::Floor,
        "ceil" => Rounding::Ceil,
        _ => Rounding::HalfUp,
    }
}

/// The precision a set of values names.
pub(crate) fn precision(values: &Values) -> Precision {
    match values.choice("precision") {
        "f32" => Precision::F32,
        _ => Precision::F64,
    }
}

/// The clamp a set of values names.
pub(crate) fn clamp(values: &Values) -> Clamp {
    match values.choice("clamp") {
        "none" => Clamp::None,
        "range" => Clamp::Range,
        _ => Clamp::RangeNanZero,
    }
}

/// The depth of the frame contract for a `--depth` value.
pub(crate) fn stream_depth(depth: u32) -> Depth {
    match depth {
        8 => Depth::U8,
        16 => Depth::U16 {
            max: STREAM_U16_MAX,
        },
        _ => Depth::F32,
    }
}

/// The configuration of the conversion for a set of values. The alpha fields keep the defaults
/// of the crate; a sample conversion does not read them.
pub fn config(values: &Values) -> Config {
    Config {
        rounding: rounding(values),
        precision: precision(values),
        clamp: clamp(values),
        u16_max: values.int("u16_max") as u16,
        ..Config::default_choice()
    }
}

/// The integer depth of the grid for a set of values.
pub fn grid(values: &Values) -> Depth {
    match values.choice("levels") {
        "16" => Depth::U16 {
            max: values.int("u16_max") as u16,
        },
        _ => Depth::U8,
    }
}

/// One sample, as a bit pattern of the depth `stream`, snapped to the grid and returned as a bit
/// pattern of the same depth.
pub fn quantise(cfg: &Config, stream: Depth, grid: Depth, bits: u32) -> u32 {
    cfg.convert_sample(grid, stream, cfg.convert_sample(stream, grid, bits))
}

impl Stage for DepthQuantise {
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
        let depth = plan.input.depth;
        let (stream, grid) = (stream_depth(depth), grid(&plan.values));
        let frame_bytes = plan.input.buffer_len()?;
        map_frames(
            input,
            output,
            frame_bytes,
            frame_bytes,
            plan.frames,
            |_, frame, out| {
                let mut bits = read_bits(frame, depth);
                for sample in &mut bits {
                    *sample = quantise(&cfg, stream, grid, *sample);
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
        assert_eq!(grid(&values), Depth::U8);
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        for (word, want) in [
            ("half-up", Rounding::HalfUp),
            ("ties-even", Rounding::TiesEven),
            ("truncate", Rounding::Truncate),
            ("floor", Rounding::Floor),
            ("ceil", Rounding::Ceil),
        ] {
            assert_eq!(config(&set(&[("rounding", word)])).rounding, want);
        }
        for (word, want) in [("f32", Precision::F32), ("f64", Precision::F64)] {
            assert_eq!(config(&set(&[("precision", word)])).precision, want);
        }
        for (word, want) in [
            ("none", Clamp::None),
            ("range", Clamp::Range),
            ("range-nan-zero", Clamp::RangeNanZero),
        ] {
            assert_eq!(config(&set(&[("clamp", word)])).clamp, want);
        }
        assert_eq!(config(&set(&[("u16_max", "32")])).u16_max, 32);
        assert_eq!(grid(&set(&[("levels", "8"), ("u16_max", "32")])), Depth::U8);
        assert_eq!(
            grid(&set(&[("levels", "16"), ("u16_max", "32")])),
            Depth::U16 { max: 32 }
        );
        assert_eq!(grid(&set(&[("levels", "16")])), Depth::U16 { max: 65535 });
    }

    #[test]
    fn the_stream_depths_are_those_of_the_frame_contract() {
        assert_eq!(stream_depth(8), Depth::U8);
        assert_eq!(stream_depth(16), Depth::U16 { max: 65535 });
        assert_eq!(stream_depth(32), Depth::F32);
    }

    #[test]
    fn a_sample_goes_to_the_grid_and_back() {
        let cfg = config(&Values::defaults(&DEF));
        // 16 bit stream, 8 bit grid: 1000 / 65535 * 255 = 3.89, rounds to 4; 4 / 255 * 65535 = 1028
        assert_eq!(quantise(&cfg, stream_depth(16), Depth::U8, 1000), 1028);
        // 8 bit stream, 8 bit grid: every value is on the grid
        for v in 0..=255 {
            assert_eq!(quantise(&cfg, Depth::U8, Depth::U8, v), v);
        }
        // 8 bit stream, a grid of four values: 100 / 255 * 3 = 1.18, rounds to 1; 1 / 3 * 255 = 85
        assert_eq!(quantise(&cfg, Depth::U8, Depth::U16 { max: 3 }, 100), 85);
        // float stream, 8 bit grid: 2.0 clamps to 255, which is 1.0
        assert_eq!(
            quantise(&cfg, Depth::F32, Depth::U8, 2.0f32.to_bits()),
            1.0f32.to_bits()
        );
    }
}
