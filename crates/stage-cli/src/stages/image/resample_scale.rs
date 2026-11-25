//! `resample-scale`: a generic stage. It resamples each frame to a given width and height with
//! the `resample` crate, every choice of the method given as a parameter. It computes a
//! published method under explicit configuration and claims nothing about any product.
//!
//! This module is the worked example of a stage with parameters, declared geometry and keyed
//! parameters:
//!
//! - [`DEF`] declares the stage. The manifest entry and every check of `--param` and `--layer`
//!   are generated from it.
//! - [`Stage::output_size`] answers the declared geometry from the command line values.
//! - [`Stage::run`] reads the key file once, then maps frame by frame with
//!   [`crate::run::map_frames`], asking [`Plan::values_at`] for the values of each frame.
//!
//! The mapping: the source coordinate of output coordinate `(x, y)` is
//! `(x * input width / output width + shift_x, y * input height / output height + shift_y)`, in
//! the pixel centre convention of `center`. Samples are converted by the rule of
//! [`crate::pixels`]; integer depths clamp to their range on the way out, depth 32 does not.

use std::io::{Read, Write};

use resample::{
    AccumOrder, AlphaMode, Border, Config, Filter, Footprint, Image, Normalize, PixelCenter,
    TieRule,
};

use crate::error::{Code, StageError};
use crate::frame::DEPTHS;
use crate::keys;
use crate::params::Values;
use crate::pixels;
use crate::registry::{
    FrameSpec, Geometry, Kind, Label, LayerDef, ParamDef, ParamKind, Plan, Stage, StageDef,
};
use crate::run::map_frames;

/// The resample scale stage.
pub struct ResampleScale;

const fn choice(
    id: &'static str,
    name: &'static str,
    default: &'static str,
    choices: &'static [&'static str],
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Choice { default, choices },
        keyed: false,
        about,
    }
}

const fn float(
    id: &'static str,
    name: &'static str,
    range: (f64, f64, f64),
    keyed: bool,
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Float {
            default: range.0,
            min: range.1,
            max: range.2,
        },
        keyed,
        about,
    }
}

const fn int(
    id: &'static str,
    name: &'static str,
    range: (i64, i64, i64),
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Int {
            default: range.0,
            min: range.1,
            max: range.2,
        },
        keyed: false,
        about,
    }
}

/// What the stage declares. Every default is the default of `resample::Config`.
pub static DEF: StageDef = StageDef {
    id: "resample-scale",
    name: "Resample scale",
    about: "Resamples each frame to a given width and height with a selectable reconstruction filter (nearest, bilinear, Keys cubic, Mitchell and Netravali cubic, Lanczos or box) under an explicit border, pixel centre, alpha and normalisation rule.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Declared,
    depths: &DEPTHS,
    params: &[
        int("width", "Width", (0, 0, 65536), "Output width in pixels; 0 keeps the input width."),
        int("height", "Height", (0, 0, 65536), "Output height in pixels; 0 keeps the input height."),
        choice(
            "filter",
            "Filter",
            "bilinear",
            &["nearest", "bilinear", "keys", "mitchell", "lanczos", "box"],
            "The reconstruction filter (Config::filter).",
        ),
        float("keys_a", "Keys a", (-0.5, -1.0, 0.0), false, "The parameter a of the Keys cubic; read when filter is keys."),
        float("mitchell_b", "Mitchell B", (1.0 / 3.0, 0.0, 1.0), false, "The parameter B of the Mitchell and Netravali cubic; read when filter is mitchell."),
        float("mitchell_c", "Mitchell C", (1.0 / 3.0, 0.0, 1.0), false, "The parameter C of the Mitchell and Netravali cubic; read when filter is mitchell."),
        int("lobes", "Lanczos lobes", (3, 1, 8), "The lobes per side of the Lanczos filter; read when filter is lanczos."),
        choice(
            "border",
            "Border",
            "clamp",
            &["zero", "clamp", "repeat", "mirror", "mirror-no-edge"],
            "What a tap outside the image reads (Config::border).",
        ),
        choice("center", "Pixel centre", "half", &["half", "corner"], "Whether pixel i has its centre at i + 0.5 or at i (Config::center)."),
        choice("alpha", "Alpha", "straight", &["straight", "premultiplied"], "Whether colour is interpolated as it is or multiplied by alpha first and divided afterwards (Config::alpha)."),
        choice("normalize", "Normalise", "none", &["none", "sum", "sum-in-bounds"], "Whether the result is divided by the sum of the tap weights (Config::normalize)."),
        choice("tie", "Nearest tie", "half-up", &["half-up", "half-down", "half-even"], "Where the nearest filter goes when the coordinate is halfway between two pixels (Config::tie)."),
        choice("order", "Accumulation order", "row-major", &["row-major", "column-major"], "The order of the accumulation loop, which matters for rounding only (Config::order)."),
        choice("footprint", "Box footprint", "unit", &["unit", "jacobian"], "The size of the box filter in source pixels: one pixel, or the mapped output pixel (Config::footprint)."),
        float("shift_x", "Shift X", (0.0, -65536.0, 65536.0), true, "Added to the source x coordinate, in source pixels; may be keyed."),
        float("shift_y", "Shift Y", (0.0, -65536.0, 65536.0), true, "Added to the source y coordinate, in source pixels; may be keyed."),
    ],
    layers: &[LayerDef {
        id: keys::LAYER_ID,
        optional: true,
        form: keys::FORM,
        about: "A key file with a value per frame for shift_x and shift_y.",
    }],
};

/// The configuration of the resampler for a set of values.
pub fn config(values: &Values) -> Config {
    let filter = match values.choice("filter") {
        "nearest" => Filter::Nearest,
        "keys" => Filter::Keys {
            a: values.float("keys_a"),
        },
        "mitchell" => Filter::Mitchell {
            b: values.float("mitchell_b"),
            c: values.float("mitchell_c"),
        },
        "lanczos" => Filter::Lanczos {
            lobes: values.int("lobes") as u32,
        },
        "box" => Filter::Box,
        _ => Filter::Bilinear,
    };
    let premultiplied = values.choice("alpha") == "premultiplied";
    Config {
        filter,
        tie: match values.choice("tie") {
            "half-down" => TieRule::HalfDown,
            "half-even" => TieRule::HalfEven,
            _ => TieRule::HalfUp,
        },
        center: match values.choice("center") {
            "corner" => PixelCenter::Corner,
            _ => PixelCenter::Half,
        },
        border: match values.choice("border") {
            "zero" => Border::Zero,
            "repeat" => Border::Repeat,
            "mirror" => Border::Mirror,
            "mirror-no-edge" => Border::MirrorNoEdge,
            _ => Border::Clamp,
        },
        alpha: if premultiplied {
            AlphaMode::Premultiplied
        } else {
            AlphaMode::Straight
        },
        alpha_channel: premultiplied.then_some(pixels::ALPHA),
        normalize: match values.choice("normalize") {
            "sum" => Normalize::Sum,
            "sum-in-bounds" => Normalize::SumInBounds,
            _ => Normalize::None,
        },
        order: match values.choice("order") {
            "column-major" => AccumOrder::ColumnMajor,
            _ => AccumOrder::RowMajor,
        },
        footprint: match values.choice("footprint") {
            "jacobian" => Footprint::Jacobian,
            _ => Footprint::Unit,
        },
        min_extent: 1.0,
    }
}

fn side(asked: i64, input: u32) -> u32 {
    if asked == 0 {
        input
    } else {
        asked as u32
    }
}

impl Stage for ResampleScale {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    fn output_size(&self, input: &FrameSpec, values: &Values) -> Result<(u32, u32), StageError> {
        Ok((
            side(values.int("width"), input.width),
            side(values.int("height"), input.height),
        ))
    }

    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError> {
        let keys = plan.keys()?;
        let cfg = config(&plan.values);
        let depth = plan.input.depth;
        let (src_w, src_h) = (plan.input.width as usize, plan.input.height as usize);
        let (dst_w, dst_h) = (plan.output.width as usize, plan.output.height as usize);
        let (step_x, step_y) = (src_w as f64 / dst_w as f64, src_h as f64 / dst_h as f64);
        let layout = match pixels::layout(depth) {
            pixels::Layout::Packed => resample::Layout::Packed,
            pixels::Layout::Planar => resample::Layout::Planar,
        };
        map_frames(
            input,
            output,
            plan.input.buffer_len()?,
            plan.output.buffer_len()?,
            plan.frames,
            |index, frame, out| {
                let values = plan.values_at(&keys, index);
                let (shift_x, shift_y) = (values.float("shift_x"), values.float("shift_y"));
                let samples = pixels::decode(frame, depth);
                let image = Image {
                    data: &samples,
                    width: src_w,
                    height: src_h,
                    channels: pixels::CHANNELS,
                    layout,
                };
                let made = resample::resample(
                    &image,
                    dst_w,
                    dst_h,
                    |x, y| (x * step_x + shift_x, y * step_y + shift_y),
                    &cfg,
                )
                .map_err(|e| StageError::new(Code::StageFailed, format!("frame {index}: {e}")))?;
                pixels::encode(&made, depth, out);
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_defaults_of_the_resampler() {
        assert_eq!(config(&Values::defaults(&DEF)), Config::default());
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        let set = |pairs: &[(&str, &str)]| {
            let pairs: Vec<(String, String)> = pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            config(&Values::parse(&DEF, &pairs).unwrap())
        };
        assert_eq!(set(&[("filter", "nearest")]).filter, Filter::Nearest);
        assert_eq!(
            set(&[("filter", "keys"), ("keys_a", "-0.75")]).filter,
            Filter::Keys { a: -0.75 }
        );
        assert_eq!(
            set(&[
                ("filter", "mitchell"),
                ("mitchell_b", "1"),
                ("mitchell_c", "0")
            ])
            .filter,
            Filter::Mitchell { b: 1.0, c: 0.0 }
        );
        assert_eq!(
            set(&[("filter", "lanczos"), ("lobes", "2")]).filter,
            Filter::Lanczos { lobes: 2 }
        );
        assert_eq!(set(&[("filter", "box")]).filter, Filter::Box);
        assert_eq!(set(&[("border", "zero")]).border, Border::Zero);
        assert_eq!(set(&[("border", "repeat")]).border, Border::Repeat);
        assert_eq!(set(&[("border", "mirror")]).border, Border::Mirror);
        assert_eq!(
            set(&[("border", "mirror-no-edge")]).border,
            Border::MirrorNoEdge
        );
        assert_eq!(set(&[("center", "corner")]).center, PixelCenter::Corner);
        assert_eq!(set(&[("tie", "half-down")]).tie, TieRule::HalfDown);
        assert_eq!(set(&[("tie", "half-even")]).tie, TieRule::HalfEven);
        assert_eq!(set(&[("normalize", "sum")]).normalize, Normalize::Sum);
        assert_eq!(
            set(&[("normalize", "sum-in-bounds")]).normalize,
            Normalize::SumInBounds
        );
        assert_eq!(
            set(&[("order", "column-major")]).order,
            AccumOrder::ColumnMajor
        );
        assert_eq!(
            set(&[("footprint", "jacobian")]).footprint,
            Footprint::Jacobian
        );
        let premultiplied = set(&[("alpha", "premultiplied")]);
        assert_eq!(premultiplied.alpha, AlphaMode::Premultiplied);
        assert_eq!(premultiplied.alpha_channel, Some(3));
        assert!(premultiplied.validate(4).is_ok());
    }

    #[test]
    fn every_configuration_the_parameters_can_reach_is_valid() {
        for filter in ["nearest", "bilinear", "keys", "mitchell", "lanczos", "box"] {
            for alpha in ["straight", "premultiplied"] {
                let pairs = [
                    ("filter".to_string(), filter.to_string()),
                    ("alpha".to_string(), alpha.to_string()),
                    ("lobes".to_string(), "1".to_string()),
                ];
                let cfg = config(&Values::parse(&DEF, &pairs).unwrap());
                assert!(cfg.validate(pixels::CHANNELS).is_ok(), "{filter} {alpha}");
            }
        }
    }

    #[test]
    fn a_zero_side_keeps_the_input_side() {
        assert_eq!(side(0, 17), 17);
        assert_eq!(side(5, 17), 5);
    }
}
