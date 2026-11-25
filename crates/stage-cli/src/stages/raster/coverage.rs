//! What the two rasterisation stages share: the parameters of the rasteriser, their map to
//! `coverage_raster::Config`, and the frame loop.
//!
//! Per frame the loop moves every point of the polygons by `offset_x` and `offset_y`, calls
//! `coverage_raster::rasterise_buffer` on a target of the frame size with its origin at 0, 0,
//! and hands the plane to [`super::carry`]. The plane is computed again only when an offset
//! differs from the frame before, so a stream without a key file rasterises once.
//!
//! How the plane becomes samples:
//!
//! - `quantise=on`: the rasteriser converts the coverage to codes of the depth of the stream
//!   under `quantise_scale` and `quantise_rounding`, and a code is its value divided by full
//!   scale, the rule of [`crate::pixels`]. At depth 32 the coverage is rounded to `f32` and the
//!   two parameters are not read.
//! - `quantise=off`: the coverage is rounded to `f32`, and the rule of [`crate::pixels`] alone
//!   takes it to the depth of the stream.
//!
//! At the defaults and depth 8 the configuration equals `coverage_raster::Config::canonical()`.

use std::io::{Read, Write};

use coverage_raster::{
    rasterise_buffer, Buffer, Combine, Config, Depth, FillRule, Method, Pattern, Polygon,
    Precision, Quantise, Rounding, Scale, Target, MAX_PIXELS, MAX_SIDE,
};

use crate::error::{Code, StageError};
use crate::keys;
use crate::params::Values;
use crate::pixels;
use crate::registry::{FrameSpec, LayerDef, ParamDef, ParamKind, Plan};
use crate::run::map_frames;

use super::carry;

/// A choice parameter that is not keyed.
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

/// A float parameter: the default, the lowest and the highest value.
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

/// A whole number parameter: the default, the lowest and the highest value.
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

/// Two parameter lists as one, the first list first.
pub const fn join<const A: usize, const B: usize, const N: usize>(
    a: [ParamDef; A],
    b: [ParamDef; B],
) -> [ParamDef; N] {
    assert!(A > 0 && A + B == N);
    let mut out = [a[0]; N];
    let mut i = 0;
    while i < A {
        out[i] = a[i];
        i += 1;
    }
    let mut j = 0;
    while j < B {
        out[A + j] = b[j];
        j += 1;
    }
    out
}

/// The parameters of the rasteriser. Every default is the value of
/// `coverage_raster::Config::canonical()`.
pub const PARAMS: [ParamDef; 15] = [
    carry::CARRY,
    carry::APPLY,
    choice(
        "method",
        "Method",
        "exact-area",
        &["exact-area", "point", "samples-grid", "samples-rotated"],
        "How the coverage of a pixel is found: the exact area, one sample at the pixel centre, or the fraction of a sample pattern (Config::method).",
    ),
    int(
        "samples",
        "Samples",
        (4, 1, 64),
        "The size of the sample pattern: samples-grid takes samples by samples points per pixel, samples-rotated takes samples points.",
    ),
    choice(
        "fill",
        "Fill rule",
        "non-zero",
        &["non-zero", "even-odd"],
        "Which winding numbers count as inside (Config::fill).",
    ),
    float(
        "centre",
        "Pixel centre",
        (0.5, 0.0, 1.0),
        false,
        "Pixel i has its centre at coordinate i + centre (Config::centre).",
    ),
    checkbox(
        "y_down",
        "Rows grow with y",
        true,
        "Whether row 0 of the frame is at the smallest y, or at the largest (Config::y_down).",
    ),
    choice(
        "combine",
        "Combine",
        "max",
        &["max", "add-clamp", "over", "replace"],
        "How the coverages of several polygons become one plane, in file order (Config::combine).",
    ),
    choice(
        "precision",
        "Precision",
        "double",
        &["double", "single"],
        "Whether a coverage value stays a double or is rounded to single precision after each step (Config::precision).",
    ),
    int(
        "snap_bits",
        "Snap bits",
        (0, 0, 30),
        "0 leaves coordinates as they are; n from 1 to 30 rounds every coordinate to a multiple of 2^-n first (Config::snap_bits).",
    ),
    choice(
        "quantise",
        "Quantise",
        "on",
        &["off", "on"],
        "Whether the rasteriser converts the coverage to a code of the depth of the stream (Config::quantise).",
    ),
    choice(
        "quantise_scale",
        "Quantise scale",
        "full",
        &["full", "power-of-two"],
        "The factor from coverage 1 to a code: the largest value of the depth, or that value plus one; read when quantise is on (Quantise::scale).",
    ),
    choice(
        "quantise_rounding",
        "Quantise rounding",
        "nearest-half-up",
        &["nearest-half-up", "nearest-even", "floor", "ceil"],
        "How a scaled coverage becomes a whole code; read when quantise is on (Quantise::rounding).",
    ),
    float(
        "offset_x",
        "Offset X",
        (0.0, -1.0e9, 1.0e9),
        true,
        "Added to the x of every point before the rasteriser runs, in pixels; may be keyed.",
    ),
    float(
        "offset_y",
        "Offset Y",
        (0.0, -1.0e9, 1.0e9),
        true,
        "Added to the y of every point before the rasteriser runs, in pixels; may be keyed.",
    ),
];

/// The key file layer of the two stages.
pub const KEYS_LAYER: LayerDef = LayerDef {
    id: keys::LAYER_ID,
    optional: true,
    form: keys::FORM,
    about: "A key file with a value per frame for offset_x and offset_y.",
};

/// The configuration of the rasteriser for a set of values and the depth of the stream.
pub fn config(values: &Values, depth: u32) -> Config {
    let samples = values.int("samples") as u32;
    let quantise = (values.choice("quantise") == "on").then(|| Quantise {
        depth: match depth {
            8 => Depth::Bits8,
            16 => Depth::Bits16,
            _ => Depth::Float32,
        },
        scale: match values.choice("quantise_scale") {
            "power-of-two" => Scale::PowerOfTwo,
            _ => Scale::Full,
        },
        rounding: match values.choice("quantise_rounding") {
            "nearest-even" => Rounding::NearestEven,
            "floor" => Rounding::Floor,
            "ceil" => Rounding::Ceil,
            _ => Rounding::NearestHalfUp,
        },
    });
    Config {
        method: match values.choice("method") {
            "point" => Method::Point,
            "samples-grid" => Method::Samples(Pattern::Grid(samples)),
            "samples-rotated" => Method::Samples(Pattern::Rotated(samples)),
            _ => Method::ExactArea,
        },
        fill: match values.choice("fill") {
            "even-odd" => FillRule::EvenOdd,
            _ => FillRule::NonZero,
        },
        centre: values.float("centre"),
        y_down: values.flag("y_down"),
        quantise,
        combine: match values.choice("combine") {
            "add-clamp" => Combine::AddClamp,
            "over" => Combine::Over,
            "replace" => Combine::Replace,
            _ => Combine::Max,
        },
        precision: match values.choice("precision") {
            "single" => Precision::Single,
            _ => Precision::Double,
        },
        snap_bits: match values.int("snap_bits") {
            0 => None,
            bits => Some(bits as u32),
        },
    }
}

/// Refuse what the rasteriser does not take: a frame above its limits or a configuration
/// outside its ranges. Both are `unsupported`, and both are judged before a frame is read.
pub fn check(input: &FrameSpec, values: &Values) -> Result<(), StageError> {
    let pixels = u64::from(input.width) * u64::from(input.height);
    if input.width > MAX_SIDE || input.height > MAX_SIDE || pixels > MAX_PIXELS {
        return Err(StageError::unsupported(format!(
            "a {}x{} frame is more than the rasteriser takes: {MAX_SIDE} on a side and {MAX_PIXELS} pixels",
            input.width, input.height
        )));
    }
    config(values, input.depth)
        .validate()
        .map_err(|e| StageError::unsupported(e.to_string()))
}

/// The polygons with every point moved by `(dx, dy)`.
pub fn moved(polygons: &[Polygon], dx: f64, dy: f64) -> Vec<Polygon> {
    let ring = |ring: &Vec<(f64, f64)>| ring.iter().map(|&(x, y)| (x + dx, y + dy)).collect();
    polygons
        .iter()
        .map(|polygon| Polygon(polygon.0.iter().map(ring).collect()))
        .collect()
}

/// A buffer of the rasteriser as `f32` samples: a code divided by full scale, a float as it is.
pub fn samples(buffer: Buffer) -> Vec<f32> {
    match buffer {
        Buffer::U8(codes) => codes
            .iter()
            .map(|c| (f64::from(*c) / 255.0) as f32)
            .collect(),
        Buffer::U16(codes) => codes
            .iter()
            .map(|c| (f64::from(*c) / 65535.0) as f32)
            .collect(),
        Buffer::F32(values) => values,
    }
}

/// The frame loop of a rasterisation stage: rasterise `polygons` at the offsets of each frame
/// and carry the plane in the frame. It holds one input frame, one output frame and one plane,
/// whatever the number of frames.
pub fn run(
    plan: &Plan,
    input: &mut dyn Read,
    output: &mut dyn Write,
    polygons: &[Polygon],
) -> Result<u64, StageError> {
    let keys = plan.keys()?;
    let depth = plan.input.depth;
    let cfg = config(&plan.values, depth);
    let (carry, apply) = (carry::carry(&plan.values), carry::apply(&plan.values));
    let target = Target::new(plan.input.width, plan.input.height);
    let layout = pixels::layout(depth);
    let frame_bytes = plan.input.buffer_len()?;
    let mut plane: Option<(u64, u64, Vec<f32>)> = None;
    map_frames(
        input,
        output,
        frame_bytes,
        frame_bytes,
        plan.frames,
        |index, frame, out| {
            let values = plan.values_at(&keys, index);
            let (dx, dy) = (values.float("offset_x"), values.float("offset_y"));
            let at = (dx.to_bits(), dy.to_bits());
            if !matches!(&plane, Some((x, y, _)) if (*x, *y) == at) {
                let buffer =
                    rasterise_buffer(&moved(polygons, dx, dy), &target, &cfg).map_err(|e| {
                        StageError::new(Code::StageFailed, format!("frame {index}: {e}"))
                    })?;
                plane = Some((at.0, at.1, samples(buffer)));
            }
            let mut decoded = pixels::decode(frame, depth);
            if let Some((_, _, coverage)) = &plane {
                carry::write(&mut decoded, layout, coverage, carry, apply);
            }
            pixels::encode(&decoded, depth, out);
            Ok(())
        },
    )
}
