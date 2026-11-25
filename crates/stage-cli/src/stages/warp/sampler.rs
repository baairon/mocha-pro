//! What every warp stage shares: the output size, the twelve parameters of the resampler and
//! the frame loop that samples each frame through a coordinate map.
//!
//! A warp stage is "a crate gives the map from an output position to a source position, and
//! `resample::resample` samples through it". This module holds the half that does not depend on
//! the map:
//!
//! - [`SHARED_PARAMS`]: `width` and `height` (0 keeps the input side) and the twelve parameters
//!   that map one to one onto `resample::Config`, with the ids, defaults and ranges of
//!   `resample-scale`.
//! - [`config`]: the function from the values of a request to `resample::Config`.
//! - [`warp_frames`]: decode, sample through the map of the frame, encode.
//!
//! A map answers `None` where it has no source position. Such a pixel is zero in every channel.

use std::io::{Read, Write};

use resample::{
    AccumOrder, AlphaMode, Border, Config, Filter, Footprint, Image, Normalize, PixelCenter,
    TieRule,
};

use crate::error::{Code, StageError};
use crate::keys;
use crate::params::Values;
use crate::pixels;
use crate::registry::{FrameSpec, LayerDef, ParamDef, ParamKind, Plan};
use crate::run::map_frames;

/// A choice parameter.
pub const fn choice(
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

/// A float parameter; `range` is the default, the lowest and the highest value.
pub const fn float(
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

/// A whole number parameter; `range` is the default, the lowest and the highest value.
pub const fn int(
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

/// A checkbox parameter.
pub const fn checkbox(
    id: &'static str,
    name: &'static str,
    default: bool,
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Checkbox { default },
        keyed: false,
        about,
    }
}

/// The number of shared parameters: `width`, `height` and the twelve of the resampler.
pub const SHARED: usize = 14;

/// The parameters every warp stage starts with. Every default of the twelve is the default of
/// `resample::Config`.
pub const SHARED_PARAMS: [ParamDef; SHARED] = [
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
];

/// Two parameter lists as one, in a constant. `ALL` is the sum of the two lengths; another value
/// stops the build.
pub const fn join<const A: usize, const B: usize, const ALL: usize>(
    first: [ParamDef; A],
    second: [ParamDef; B],
) -> [ParamDef; ALL] {
    assert!(A > 0 && A + B == ALL);
    let mut all = [first[0]; ALL];
    let mut i = 0;
    while i < A {
        all[i] = first[i];
        i += 1;
    }
    let mut j = 0;
    while j < B {
        all[A + j] = second[j];
        j += 1;
    }
    all
}

/// The shared parameters followed by the parameters of one stage.
pub const fn params<const OWN: usize, const ALL: usize>(own: [ParamDef; OWN]) -> [ParamDef; ALL] {
    join(SHARED_PARAMS, own)
}

/// The key file layer of a stage with keyed parameters.
pub const fn keys_layer(about: &'static str) -> LayerDef {
    LayerDef {
        id: keys::LAYER_ID,
        optional: true,
        form: keys::FORM,
        about,
    }
}

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

/// What the resampler adds to a pixel index to form the coordinate it gives to a map: 0.5 when
/// `center` is `half`, 0 when it is `corner`.
pub fn centre_offset(values: &Values) -> f64 {
    config(values).center.offset()
}

fn side(asked: i64, input: u32) -> u32 {
    if asked == 0 {
        input
    } else {
        asked as u32
    }
}

/// The declared output size: `width` by `height`, where 0 keeps the input side.
pub fn output_size(input: &FrameSpec, values: &Values) -> (u32, u32) {
    (
        side(values.int("width"), input.width),
        side(values.int("height"), input.height),
    )
}

/// Read frames, sample each one through the map of its frame and write the result.
///
/// `frame_map` receives the index of a frame and the values of that frame (the command line
/// values, with every keyed parameter that has a track replaced by the value of the track) and
/// answers the map of the frame or a refusal. The map receives an output position in the
/// coordinates of the resampler (a pixel index plus [`centre_offset`]) and answers the source
/// position in the same coordinates, or `None` for a pixel that stays zero.
///
/// The key file is read once, before the first frame. When it has no track, the values of the
/// command line hold for every frame, so the map is asked for once before any frame is read and
/// a refusal ends the run even on an empty input. With tracks, a refusal ends the run at the
/// frame that meets it, after every earlier frame was written.
///
/// One input frame, its decoded samples and one output frame are held at a time.
pub fn warp_frames<F, M>(
    plan: &Plan,
    input: &mut dyn Read,
    output: &mut dyn Write,
    mut frame_map: F,
) -> Result<u64, StageError>
where
    F: FnMut(u64, &Values) -> Result<M, StageError>,
    M: Fn(f64, f64) -> Option<(f64, f64)>,
{
    let keys = plan.keys()?;
    if keys.tracks().is_empty() {
        frame_map(0, &plan.values)?;
    }
    let cfg = config(&plan.values);
    let depth = plan.input.depth;
    let (src_w, src_h) = (plan.input.width as usize, plan.input.height as usize);
    let (dst_w, dst_h) = (plan.output.width as usize, plan.output.height as usize);
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
            let map = frame_map(index, &values)?;
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
                |x, y| map(x, y).unwrap_or((f64::NAN, f64::NAN)),
                &cfg,
            )
            .map_err(|e| StageError::new(Code::StageFailed, format!("frame {index}: {e}")))?;
            pixels::encode(&made, depth, out);
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Geometry, Kind, Label, StageDef};

    static PARAMS: [ParamDef; SHARED + 1] = params([float(
        "extra",
        "Extra",
        (0.0, -1.0, 1.0),
        true,
        "A made up parameter after the shared ones.",
    )]);

    static DEF: StageDef = StageDef {
        id: "sampler-test",
        name: "Sampler test",
        about: "A made up definition that holds the shared parameters.",
        label: Label::Generic,
        kind: Kind::Frame,
        spool: false,
        geometry: Geometry::Declared,
        depths: &crate::frame::DEPTHS,
        params: &PARAMS,
        layers: &[keys_layer("A key file for the made up parameter.")],
    };

    fn set(pairs: &[(&str, &str)]) -> Config {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        config(&Values::parse(&DEF, &pairs).unwrap())
    }

    #[test]
    fn the_defaults_are_the_defaults_of_the_resampler() {
        assert_eq!(config(&Values::defaults(&DEF)), Config::default());
        assert_eq!(centre_offset(&Values::defaults(&DEF)), 0.5);
    }

    #[test]
    fn the_shared_parameters_are_those_of_resample_scale() {
        let scale = crate::stages::image::resample_scale::DEF.params;
        assert_eq!(&scale[..SHARED], &SHARED_PARAMS[..]);
        assert_eq!(&PARAMS[..SHARED], &SHARED_PARAMS[..]);
        assert_eq!(PARAMS[SHARED].id, "extra");
    }

    #[test]
    fn the_configuration_is_that_of_resample_scale_for_every_choice() {
        let scale = &crate::stages::image::resample_scale::DEF;
        for p in &SHARED_PARAMS[2..] {
            let texts: Vec<String> = match p.kind {
                ParamKind::Choice { choices, .. } => {
                    choices.iter().map(|c| c.to_string()).collect()
                }
                ParamKind::Float { min, max, .. } => vec![min.to_string(), max.to_string()],
                ParamKind::Int { min, max, .. } => vec![min.to_string(), max.to_string()],
                _ => Vec::new(),
            };
            for text in texts {
                for filter in ["keys", "mitchell", "lanczos"] {
                    let mut pairs = vec![(p.id.to_string(), text.clone())];
                    if p.id != "filter" {
                        pairs.push(("filter".to_string(), filter.to_string()));
                    }
                    let here = config(&Values::parse(&DEF, &pairs).unwrap());
                    let there = crate::stages::image::resample_scale::config(
                        &Values::parse(scale, &pairs).unwrap(),
                    );
                    assert_eq!(here, there, "{} = {text}", p.id);
                }
            }
        }
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        assert_eq!(set(&[("filter", "nearest")]).filter, Filter::Nearest);
        assert_eq!(set(&[("filter", "box")]).filter, Filter::Box);
        assert_eq!(
            set(&[("filter", "lanczos"), ("lobes", "2")]).filter,
            Filter::Lanczos { lobes: 2 }
        );
        assert_eq!(set(&[("border", "zero")]).border, Border::Zero);
        assert_eq!(set(&[("center", "corner")]).center, PixelCenter::Corner);
        assert_eq!(set(&[("tie", "half-even")]).tie, TieRule::HalfEven);
        assert_eq!(set(&[("normalize", "sum")]).normalize, Normalize::Sum);
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
        assert!(premultiplied.validate(pixels::CHANNELS).is_ok());
        let corner = Values::parse(&DEF, &[("center".to_string(), "corner".to_string())]).unwrap();
        assert_eq!(centre_offset(&corner), 0.0);
    }

    #[test]
    fn a_zero_side_keeps_the_input_side() {
        let input = FrameSpec {
            width: 17,
            height: 9,
            depth: 8,
            frame_bytes: 17 * 9 * 4,
        };
        assert_eq!(output_size(&input, &Values::defaults(&DEF)), (17, 9));
        let pairs = [
            ("width".to_string(), "5".to_string()),
            ("height".to_string(), "40".to_string()),
        ];
        let asked = Values::parse(&DEF, &pairs).unwrap();
        assert_eq!(output_size(&input, &asked), (5, 40));
    }
}
