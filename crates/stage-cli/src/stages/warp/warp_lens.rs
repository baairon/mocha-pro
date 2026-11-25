//! `warp-lens`: a generic stage. It applies or removes a lens distortion with the `lens-model`
//! crate for the coordinates and the `resample` crate for the samples. It computes a published
//! method under explicit configuration and claims nothing about any product.
//!
//! The models are those of the crate: the polynomial radial model with two tangential terms
//! (Brown 1966, 1971), the one parameter division model (Fitzgibbon 2001) and the squeezed
//! variant the crate defines. Each output position reads the source at
//! `lens_model::Config::source_map` for the chosen direction. A position the map does not reach
//! is zero.
//!
//! Three rules of the stage:
//!
//! - The image of `lens_model::Config` is the input frame: `size` is its width and height.
//! - `centre_x` and `centre_y` are fractions of the width and the height of the input frame, so
//!   the default of 0.5 is the centre of the frame whatever its size. The centre in pixels is
//!   `(width * centre_x, height * centre_y)`.
//! - The output canvas is centred on the input frame: output position `p` is the frame position
//!   `p - ((output width - input width) / 2, (output height - input height) / 2)`. With the
//!   declared size at its default the two are equal.
//!
//! The crate places pixel i with its centre at i + 0.5. When the resampler places it at i
//! (`center` is `corner`), a position is moved by half a pixel before the map and back after it.

use std::io::{Read, Write};

use lens_model::{Config, Direction, InverseMethod, Model, ModelMaps, Normalisation};

use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::params::Values;
use crate::registry::{
    FrameSpec, Geometry, Kind, Label, LayerDef, ParamDef, Plan, Stage, StageDef,
};

use super::sampler::{self, choice, float, int, SHARED};

/// The lens warp stage.
pub struct WarpLens;

const COEFFICIENT: (f64, f64, f64) = (0.0, -100.0, 100.0);

const OWN: [ParamDef; 18] = [
    choice("direction", "Direction", "apply", &["apply", "remove"], "Whether the stage makes a distorted frame from an undistorted one or the reverse (Direction)."),
    choice("model", "Model", "radial", &["radial", "division", "squeezed"], "The distortion model: the radial polynomial with tangential terms, the division model, or the radial polynomial on squeezed axes (Model)."),
    float("k1", "k1", COEFFICIENT, true, "The first radial coefficient; read by radial and squeezed; may be keyed."),
    float("k2", "k2", COEFFICIENT, true, "The second radial coefficient; read by radial and squeezed; may be keyed."),
    float("k3", "k3", COEFFICIENT, true, "The third radial coefficient; read by radial and squeezed; may be keyed."),
    float("p1", "p1", COEFFICIENT, true, "The first tangential coefficient; read by radial; may be keyed."),
    float("p2", "p2", COEFFICIENT, true, "The second tangential coefficient; read by radial; may be keyed."),
    float("lambda", "lambda", COEFFICIENT, true, "The parameter of the division model; read by division; may be keyed."),
    float("squeeze_x", "Squeeze x", (1.0, 0.01, 100.0), false, "The factor on x inside the radius; read by squeezed."),
    float("squeeze_y", "Squeeze y", (1.0, 0.01, 100.0), false, "The factor on y inside the radius; read by squeezed."),
    choice("normalisation", "Normalisation", "half-width", &["half-width", "half-height", "half-diagonal", "focal-length"], "What pixel offsets from the centre are divided by (Config::normalisation)."),
    float("focal_length", "Focal length in pixels", (1.0, 1e-6, 1e9), false, "The divisor when normalisation is focal-length."),
    float("centre_x", "Centre x", (0.5, -100.0, 100.0), true, "The distortion centre as a fraction of the input width (Config::centre); may be keyed."),
    float("centre_y", "Centre y", (0.5, -100.0, 100.0), true, "The distortion centre as a fraction of the input height (Config::centre); may be keyed."),
    float("pixel_aspect", "Pixel aspect", (1.0, 0.01, 100.0), false, "The width of a pixel over its height (Config::pixel_aspect)."),
    choice("maps", "Model maps", "undistorted-to-distorted", &["undistorted-to-distorted", "distorted-to-undistorted"], "Which way the formula of the model goes (Config::maps)."),
    choice("inverse_method", "Inverse method", "newton", &["fixed-point", "newton", "closed-form"], "How the inverse of the formula is computed where the stage needs it (Config::inverse_method); closed-form exists for division only."),
    int("inverse_iterations", "Inverse iterations", (8, 0, 1024), "The fixed step count of the iterative inverse methods (Config::inverse_iterations)."),
];

static PARAMS: [ParamDef; SHARED + 18] = sampler::params(OWN);

static LAYERS: [LayerDef; 1] = [sampler::keys_layer(
    "A key file with a value per frame for k1, k2, k3, p1, p2, lambda, centre_x and centre_y.",
)];

/// What the stage declares. Every default is the default of `lens_model::Config` for the input
/// frame or of `resample::Config`.
pub static DEF: StageDef = StageDef {
    id: "warp-lens",
    name: "Lens distortion warp",
    about: "Applies or removes a lens distortion on each frame with a radial polynomial model (Brown 1966, 1971), a division model (Fitzgibbon 2001) or a squeezed variant, sampled with a selectable reconstruction filter.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Declared,
    depths: &DEPTHS,
    params: &PARAMS,
    layers: &LAYERS,
};

/// The direction of a set of values.
pub fn direction(values: &Values) -> Direction {
    match values.choice("direction") {
        "remove" => Direction::ToUndistorted,
        _ => Direction::ToDistorted,
    }
}

/// The lens configuration of a set of values for an input frame of `width` by `height`.
pub fn config(values: &Values, width: u32, height: u32) -> Config {
    let k = [values.float("k1"), values.float("k2"), values.float("k3")];
    let (w, h) = (f64::from(width), f64::from(height));
    Config {
        model: match values.choice("model") {
            "division" => Model::Division {
                lambda: values.float("lambda"),
            },
            "squeezed" => Model::Squeezed {
                k,
                squeeze: [values.float("squeeze_x"), values.float("squeeze_y")],
            },
            _ => Model::Radial {
                k,
                p: [values.float("p1"), values.float("p2")],
            },
        },
        normalisation: match values.choice("normalisation") {
            "half-height" => Normalisation::HalfHeight,
            "half-diagonal" => Normalisation::HalfDiagonal,
            "focal-length" => Normalisation::Focal(values.float("focal_length")),
            _ => Normalisation::HalfWidth,
        },
        size: [w, h],
        centre: [w * values.float("centre_x"), h * values.float("centre_y")],
        pixel_aspect: values.float("pixel_aspect"),
        maps: match values.choice("maps") {
            "distorted-to-undistorted" => ModelMaps::DistortedToUndistorted,
            _ => ModelMaps::UndistortedToDistorted,
        },
        inverse_method: match values.choice("inverse_method") {
            "fixed-point" => InverseMethod::FixedPoint,
            "closed-form" => InverseMethod::ClosedForm,
            _ => InverseMethod::Newton,
        },
        inverse_iterations: values.int("inverse_iterations") as u32,
    }
}

/// Whether the map the stage samples through is the inverse of the formula of the model.
pub fn uses_the_inverse(values: &Values) -> bool {
    let apply = values.choice("direction") == "apply";
    let forward = values.choice("maps") == "undistorted-to-distorted";
    apply == forward
}

/// The map of one frame for the resampler: an output position to its source position, both in
/// the coordinates of the resampler, whose pixel centres lie at an index plus `sample_offset`.
pub fn source_map(
    values: &Values,
    src: (u32, u32),
    dst: (u32, u32),
    sample_offset: f64,
) -> impl Fn(f64, f64) -> Option<(f64, f64)> {
    let cfg = config(values, src.0, src.1);
    let dir = direction(values);
    let shift = 0.5 - sample_offset;
    let margin_x = (f64::from(dst.0) - f64::from(src.0)) / 2.0;
    let margin_y = (f64::from(dst.1) - f64::from(src.1)) / 2.0;
    move |x: f64, y: f64| {
        let read = cfg.source_map(dir);
        let q = read([x + shift - margin_x, y + shift - margin_y])?;
        Some((q[0] - shift, q[1] - shift))
    }
}

impl Stage for WarpLens {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    fn output_size(&self, input: &FrameSpec, values: &Values) -> Result<(u32, u32), StageError> {
        if values.choice("inverse_method") == "closed-form"
            && values.choice("model") != "division"
            && uses_the_inverse(values)
        {
            return Err(StageError::unsupported(format!(
                "inverse_method closed-form needs model division: {} has no closed form inverse",
                values.choice("model")
            )));
        }
        Ok(sampler::output_size(input, values))
    }

    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError> {
        let src = (plan.input.width, plan.input.height);
        let dst = (plan.output.width, plan.output.height);
        let offset = sampler::centre_offset(&plan.values);
        sampler::warp_frames(plan, input, output, |_, values| {
            Ok(source_map(values, src, dst, offset))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> Values {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Values::parse(&DEF, &pairs).unwrap()
    }

    #[test]
    fn the_defaults_are_the_defaults_of_the_crates() {
        let defaults = Values::defaults(&DEF);
        assert_eq!(config(&defaults, 2, 2), Config::default());
        assert_eq!(
            config(&defaults, 7, 5),
            Config::new(7.0, 5.0, Model::identity())
        );
        assert_eq!(direction(&defaults), Direction::ToDistorted);
        assert_eq!(sampler::config(&defaults), resample::Config::default());
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        let radial = values(&[
            ("k1", "0.5"),
            ("k2", "0.25"),
            ("k3", "-0.125"),
            ("p1", "2"),
            ("p2", "-3"),
        ]);
        assert_eq!(
            config(&radial, 4, 4).model,
            Model::Radial {
                k: [0.5, 0.25, -0.125],
                p: [2.0, -3.0]
            }
        );
        let division = values(&[("model", "division"), ("lambda", "0.75")]);
        assert_eq!(
            config(&division, 4, 4).model,
            Model::Division { lambda: 0.75 }
        );
        let squeezed = values(&[
            ("model", "squeezed"),
            ("k1", "1"),
            ("squeeze_x", "2"),
            ("squeeze_y", "0.5"),
        ]);
        assert_eq!(
            config(&squeezed, 4, 4).model,
            Model::Squeezed {
                k: [1.0, 0.0, 0.0],
                squeeze: [2.0, 0.5]
            }
        );
        let norm = |word: &str| config(&values(&[("normalisation", word)]), 4, 4).normalisation;
        assert_eq!(norm("half-width"), Normalisation::HalfWidth);
        assert_eq!(norm("half-height"), Normalisation::HalfHeight);
        assert_eq!(norm("half-diagonal"), Normalisation::HalfDiagonal);
        let focal = values(&[("normalisation", "focal-length"), ("focal_length", "35")]);
        assert_eq!(
            config(&focal, 4, 4).normalisation,
            Normalisation::Focal(35.0)
        );
        let moved = values(&[
            ("centre_x", "0.25"),
            ("centre_y", "1"),
            ("pixel_aspect", "2"),
        ]);
        let moved = config(&moved, 8, 4);
        assert_eq!(moved.centre, [2.0, 4.0]);
        assert_eq!(moved.size, [8.0, 4.0]);
        assert_eq!(moved.pixel_aspect, 2.0);
        assert_eq!(
            config(&values(&[("maps", "distorted-to-undistorted")]), 4, 4).maps,
            ModelMaps::DistortedToUndistorted
        );
        let method = |word: &str| config(&values(&[("inverse_method", word)]), 4, 4).inverse_method;
        assert_eq!(method("fixed-point"), InverseMethod::FixedPoint);
        assert_eq!(method("newton"), InverseMethod::Newton);
        assert_eq!(method("closed-form"), InverseMethod::ClosedForm);
        assert_eq!(
            config(&values(&[("inverse_iterations", "3")]), 4, 4).inverse_iterations,
            3
        );
        assert_eq!(
            direction(&values(&[("direction", "remove")])),
            Direction::ToUndistorted
        );
    }

    #[test]
    fn every_configuration_the_parameters_can_reach_is_valid() {
        for model in ["radial", "division", "squeezed"] {
            for norm in ["half-width", "half-height", "half-diagonal", "focal-length"] {
                for edge in ["0", "1"] {
                    let pairs = [
                        ("model", model),
                        ("normalisation", norm),
                        (
                            "focal_length",
                            if edge == "0" {
                                "0.000001"
                            } else {
                                "1000000000"
                            },
                        ),
                        ("pixel_aspect", if edge == "0" { "0.01" } else { "100" }),
                        ("centre_x", if edge == "0" { "-100" } else { "100" }),
                    ];
                    assert!(config(&values(&pairs), 1, 65536).validate().is_ok());
                }
            }
        }
    }

    #[test]
    fn the_inverse_is_used_in_two_of_the_four_combinations() {
        assert!(uses_the_inverse(&Values::defaults(&DEF)));
        assert!(!uses_the_inverse(&values(&[("direction", "remove")])));
        assert!(!uses_the_inverse(&values(&[(
            "maps",
            "distorted-to-undistorted"
        )])));
        assert!(uses_the_inverse(&values(&[
            ("direction", "remove"),
            ("maps", "distorted-to-undistorted")
        ])));
    }

    #[test]
    fn the_map_is_the_map_of_the_crate_on_a_centred_canvas() {
        let v = values(&[("direction", "remove"), ("k1", "1")]);
        let cfg = config(&v, 4, 4);
        let same = source_map(&v, (4, 4), (4, 4), 0.5);
        let want = cfg.to_distorted([2.5, 2.5]).unwrap();
        assert_eq!(same(2.5, 2.5), Some((want[0], want[1])));
        assert_eq!(same(2.5, 2.5), Some((2.5625, 2.5625)));
        // An 8 by 6 canvas holds the 4 by 4 frame with margins of 2 and 1.
        let wide = source_map(&v, (4, 4), (8, 6), 0.5);
        assert_eq!(wide(4.5, 3.5), Some((2.5625, 2.5625)));
        // Pixel centres at whole numbers in the resampler: half a pixel there and back.
        let corner = source_map(&v, (4, 4), (4, 4), 0.0);
        assert_eq!(corner(2.0, 2.0), Some((2.0625, 2.0625)));
    }

    #[test]
    fn the_definition_holds_the_shared_parameters_first() {
        assert_eq!(&DEF.params[..SHARED], &sampler::SHARED_PARAMS[..]);
        assert_eq!(DEF.params.len(), 32);
        let keyed: Vec<&str> = DEF
            .params
            .iter()
            .filter(|p| p.keyed)
            .map(|p| p.id)
            .collect();
        assert_eq!(
            keyed,
            ["k1", "k2", "k3", "p1", "p2", "lambda", "centre_x", "centre_y"]
        );
    }
}
