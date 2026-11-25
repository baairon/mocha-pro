//! `raster-contour`: a generic stage. It flattens cubic Bezier and X-spline contours to polylines
//! with the `contour-eval` crate, rasterises them to a coverage matte with the `coverage-raster`
//! crate, every choice of both methods given as a parameter, and carries the matte in the frame.
//! It computes published methods under explicit configuration and claims nothing about any
//! product.
//!
//! - The contours come from `--layer shape=PATH`, a file of the form `contours-1`
//!   ([`super::shape`]). Without the layer there is no contour and the coverage is 0 everywhere.
//! - The contours are flattened once, before the first frame: `flatten` and its constants give
//!   the rule (`contour_eval::Flatten`), and eight more parameters map one to one to
//!   `contour_eval::Config`. The polyline of a contour is one ring; the rings of a polygon are
//!   filled together. An open contour is filled as the ring of its polyline.
//! - From there the stage is `raster-polygon`: the parameters of [`super::coverage`], the keyed
//!   `offset_x` and `offset_y`, which move every point of the polylines, and `carry` and `apply`.
//!
//! A contour that `contour_eval::flatten` refuses is `unsupported`, with the indexes of its
//! polygon and of the contour: the control polygon flatness test on an X-spline contour, or a
//! stored weight that `weight_map` takes outside the range of its variant.
//!
//! `contour_eval::Flatten` has no default of its own. The defaults here are stated starting
//! values: the uniform rule with 16 segments per span.

use std::io::{Read, Write};

use contour_eval::{Config, Contour, EvalForm, FlatTest, Flatten, OpenEnd, Precision, TangentMode};
use coverage_raster::Polygon;

use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::params::Values;
use crate::registry::{FrameSpec, Geometry, Kind, Label, ParamDef, Plan, Stage, StageDef};

use super::coverage::{self, checkbox, choice, float, int};
use super::shape;

/// The contour rasterisation stage.
pub struct RasterContour;

/// The words of `weight_map`, in the order of `contour_eval::Config::WEIGHT_MAPS`.
const WEIGHT_MAPS: [&str; 4] = [
    "identity",
    "unit-to-signed",
    "hundred-to-unit",
    "hundred-to-signed",
];

/// The parameters of the flattening. Every default of the last eight is the value of
/// `contour_eval::Config::canonical()`.
const FLATTENING: [ParamDef; 14] = [
    choice(
        "flatten",
        "Flatten",
        "uniform",
        &["uniform", "subdivide", "by-length"],
        "The rule that turns a span into segments: a fixed count, midpoint subdivision to a flatness bound, or a count from the length of the span (Flatten).",
    ),
    int(
        "segments",
        "Segments",
        (16, 1, 4096),
        "The segments per span; read when flatten is uniform (Flatten::Uniform).",
    ),
    float(
        "tolerance",
        "Tolerance",
        (0.25, 0.0, 1.0e6),
        false,
        "The largest distance the flatness test accepts, in pixels; read when flatten is subdivide (Flatten::Subdivide tol).",
    ),
    int(
        "max_depth",
        "Depth limit",
        (10, 0, 24),
        "The deepest subdivision; read when flatten is subdivide (Flatten::Subdivide max_depth).",
    ),
    float(
        "step",
        "Step",
        (1.0, 1.0e-6, 1.0e6),
        false,
        "The length one segment stands for, in pixels; read when flatten is by-length (Flatten::ByLength step).",
    ),
    int(
        "length_samples",
        "Length samples",
        (16, 1, 4096),
        "The parameter steps of the length estimate of a span; read when flatten is by-length (Flatten::ByLength samples).",
    ),
    choice(
        "form",
        "Evaluation form",
        "bernstein",
        &["bernstein", "de-casteljau", "horner"],
        "The evaluation form of a cubic Bezier span (Config::form).",
    ),
    choice(
        "eval_precision",
        "Evaluation precision",
        "f64",
        &["f32", "f64"],
        "The number type inside an evaluation (Config::precision).",
    ),
    choice(
        "tangents",
        "Tangents",
        "relative",
        &["relative", "absolute"],
        "Whether a Bezier tangent handle is an offset from its anchor or a position (Config::tangents).",
    ),
    choice(
        "open_end",
        "Open end",
        "repeat-end",
        &["repeat-end", "repeat-end-sharp"],
        "How the ends of an open X-spline contour are extended (Config::open_end).",
    ),
    checkbox(
        "closing_span",
        "Closing span",
        true,
        "Whether a closed contour has the span from its last point back to its first (Config::closing_span).",
    ),
    checkbox(
        "repeat_first",
        "Repeat first",
        false,
        "Whether the polyline of a closed contour ends with a copy of its first point (Config::repeat_first).",
    ),
    choice(
        "flat_test",
        "Flatness test",
        "sampled",
        &["control-polygon", "sampled"],
        "The flatness test of the subdivide rule; control-polygon takes Bezier contours only (Config::flat_test).",
    ),
    choice(
        "weight_map",
        "Weight map",
        "identity",
        &WEIGHT_MAPS,
        "The map from a stored X-spline weight w to the shape parameter: w, 2w - 1, w / 100 or w / 50 - 1 (Config::weight_map, the four maps of Config::WEIGHT_MAPS).",
    ),
];

const PARAMS: [ParamDef; 29] = coverage::join(coverage::PARAMS, FLATTENING);

/// What the stage declares.
pub static DEF: StageDef = StageDef {
    id: "raster-contour",
    name: "Raster contour",
    about: "Flattens cubic Bezier and X-spline contours (Farin 2002; Blanc and Schlick 1995) from a shape file to polylines under an explicit flattening rule and evaluation configuration, rasterises them to a coverage matte as raster-polygon does, and carries the matte in the frame as the carry parameter says.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Preserve,
    depths: &DEPTHS,
    params: &PARAMS,
    layers: &[shape::CONTOURS_LAYER, coverage::KEYS_LAYER],
};

/// The flattening rule for a set of values.
pub fn rule(values: &Values) -> Flatten {
    match values.choice("flatten") {
        "subdivide" => Flatten::Subdivide {
            tol: values.float("tolerance"),
            max_depth: values.int("max_depth") as u32,
        },
        "by-length" => Flatten::ByLength {
            step: values.float("step"),
            samples: values.int("length_samples") as u32,
        },
        _ => Flatten::Uniform(values.int("segments") as u32),
    }
}

/// The evaluation configuration for a set of values.
pub fn config(values: &Values) -> Config {
    let weight_map = values.choice("weight_map");
    let weight_map = WEIGHT_MAPS
        .iter()
        .position(|word| *word == weight_map)
        .map_or(Config::WEIGHT_MAPS[0], |at| Config::WEIGHT_MAPS[at]);
    Config {
        form: match values.choice("form") {
            "de-casteljau" => EvalForm::DeCasteljau,
            "horner" => EvalForm::Horner,
            _ => EvalForm::Bernstein,
        },
        precision: match values.choice("eval_precision") {
            "f32" => Precision::F32,
            _ => Precision::F64,
        },
        tangents: match values.choice("tangents") {
            "absolute" => TangentMode::Absolute,
            _ => TangentMode::Relative,
        },
        weight_map,
        open_end: match values.choice("open_end") {
            "repeat-end-sharp" => OpenEnd::RepeatEndSharp,
            _ => OpenEnd::RepeatEnd,
        },
        closing_span: values.flag("closing_span"),
        repeat_first: values.flag("repeat_first"),
        flat_test: match values.choice("flat_test") {
            "control-polygon" => FlatTest::ControlPolygon,
            _ => FlatTest::Sampled,
        },
    }
}

/// Flatten every contour of every polygon: one ring per contour, in file order. A contour the
/// crate refuses is `unsupported`.
pub fn flatten(
    polygons: &[Vec<Contour>],
    rule: &Flatten,
    cfg: &Config,
) -> Result<Vec<Polygon>, StageError> {
    let mut out = Vec::with_capacity(polygons.len());
    for (p, contours) in polygons.iter().enumerate() {
        let mut rings = Vec::with_capacity(contours.len());
        for (c, contour) in contours.iter().enumerate() {
            let ring = contour_eval::flatten(contour, rule, cfg)
                .map_err(|e| StageError::unsupported(format!("contour {c} of polygon {p}: {e}")))?;
            rings.push(ring);
        }
        out.push(Polygon(rings));
    }
    Ok(out)
}

impl Stage for RasterContour {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    fn output_size(&self, input: &FrameSpec, values: &Values) -> Result<(u32, u32), StageError> {
        coverage::check(input, values)?;
        Ok((input.width, input.height))
    }

    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError> {
        let contours = shape::contours(plan)?;
        let polygons = flatten(&contours, &rule(&plan.values), &config(&plan.values))?;
        coverage::run(plan, input, output, &polygons)
    }
}

#[cfg(test)]
mod tests {
    use contour_eval::{Kind as ContourKind, Point, WeightMap, XVariant};

    use super::*;
    use crate::error::Code;

    fn values(pairs: &[(&str, &str)]) -> Values {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Values::parse(&DEF, &pairs).unwrap()
    }

    #[test]
    fn the_defaults_are_the_defaults_of_both_crates() {
        let defaults = Values::defaults(&DEF);
        assert_eq!(config(&defaults), Config::canonical());
        assert_eq!(
            coverage::config(&defaults, 8),
            coverage_raster::Config::canonical()
        );
        assert_eq!(rule(&defaults), Flatten::Uniform(16));
    }

    #[test]
    fn the_first_parameters_are_those_of_raster_polygon() {
        let shared = super::super::raster_polygon::DEF.params;
        assert_eq!(&DEF.params[..shared.len()], shared);
        assert_eq!(DEF.params.len(), shared.len() + FLATTENING.len());
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        let set = |pairs: &[(&str, &str)]| config(&values(pairs));
        assert_eq!(set(&[("form", "bernstein")]).form, EvalForm::Bernstein);
        assert_eq!(set(&[("form", "de-casteljau")]).form, EvalForm::DeCasteljau);
        assert_eq!(set(&[("form", "horner")]).form, EvalForm::Horner);
        assert_eq!(set(&[("eval_precision", "f32")]).precision, Precision::F32);
        assert_eq!(set(&[("eval_precision", "f64")]).precision, Precision::F64);
        assert_eq!(
            set(&[("tangents", "relative")]).tangents,
            TangentMode::Relative
        );
        assert_eq!(
            set(&[("tangents", "absolute")]).tangents,
            TangentMode::Absolute
        );
        assert_eq!(
            set(&[("open_end", "repeat-end")]).open_end,
            OpenEnd::RepeatEnd
        );
        assert_eq!(
            set(&[("open_end", "repeat-end-sharp")]).open_end,
            OpenEnd::RepeatEndSharp
        );
        assert!(!set(&[("closing_span", "0")]).closing_span);
        assert!(set(&[("repeat_first", "1")]).repeat_first);
        assert_eq!(
            set(&[("flat_test", "control-polygon")]).flat_test,
            FlatTest::ControlPolygon
        );
        assert_eq!(
            set(&[("flat_test", "sampled")]).flat_test,
            FlatTest::Sampled
        );
        let map = |word| set(&[("weight_map", word)]).weight_map;
        assert_eq!(map("identity"), WeightMap { a: 1.0, b: 0.0 });
        assert_eq!(map("unit-to-signed"), WeightMap { a: 2.0, b: -1.0 });
        assert_eq!(map("hundred-to-unit"), WeightMap { a: 0.01, b: 0.0 });
        assert_eq!(map("hundred-to-signed"), WeightMap { a: 0.02, b: -1.0 });
        for (word, want) in WEIGHT_MAPS.iter().zip(Config::WEIGHT_MAPS) {
            assert_eq!(map(word), want);
        }
    }

    #[test]
    fn every_rule_maps_to_a_flattening() {
        let set = |pairs: &[(&str, &str)]| rule(&values(pairs));
        assert_eq!(set(&[("segments", "3")]), Flatten::Uniform(3));
        assert_eq!(
            set(&[("flatten", "subdivide")]),
            Flatten::Subdivide {
                tol: 0.25,
                max_depth: 10
            }
        );
        assert_eq!(
            set(&[
                ("flatten", "subdivide"),
                ("tolerance", "0.5"),
                ("max_depth", "24")
            ]),
            Flatten::Subdivide {
                tol: 0.5,
                max_depth: 24
            }
        );
        assert_eq!(
            set(&[("flatten", "by-length")]),
            Flatten::ByLength {
                step: 1.0,
                samples: 16
            }
        );
        assert_eq!(
            set(&[
                ("flatten", "by-length"),
                ("step", "2.5"),
                ("length_samples", "4")
            ]),
            Flatten::ByLength {
                step: 2.5,
                samples: 4
            }
        );
    }

    fn square(kind: ContourKind) -> Contour {
        Contour {
            kind,
            closed: true,
            points: vec![
                Point::new(1.0, 1.0),
                Point::new(5.0, 1.0),
                Point::new(5.0, 3.0),
                Point::new(1.0, 3.0),
            ],
        }
    }

    #[test]
    fn a_contour_becomes_one_ring_of_its_polygon() {
        let polygons = [
            vec![square(ContourKind::Bezier), square(ContourKind::Bezier)],
            vec![square(ContourKind::Bezier)],
        ];
        let made = flatten(&polygons, &Flatten::Uniform(1), &Config::canonical()).unwrap();
        assert_eq!(made.len(), 2);
        assert_eq!(made[0].0.len(), 2);
        assert_eq!(
            made[1].0[0],
            [(1.0, 1.0), (5.0, 1.0), (5.0, 3.0), (1.0, 3.0)]
        );
    }

    #[test]
    fn what_the_flattening_refuses_is_unsupported() {
        let spline = ContourKind::XSpline {
            variant: XVariant::Basic,
        };
        let cfg = Config {
            flat_test: FlatTest::ControlPolygon,
            ..Config::canonical()
        };
        let rule = Flatten::Subdivide {
            tol: 0.25,
            max_depth: 4,
        };
        let e = flatten(&[vec![square(spline)]], &rule, &cfg).unwrap_err();
        assert_eq!(e.code, Code::Unsupported);
        assert_eq!(
            e.message,
            "contour 0 of polygon 0: the control polygon flatness test needs a Bezier contour"
        );
        let mut heavy = square(spline);
        heavy.points[2].s = 2.0;
        let e = flatten(
            &[vec![square(spline), heavy]],
            &Flatten::Uniform(2),
            &Config::canonical(),
        )
        .unwrap_err();
        assert_eq!(e.code, Code::Unsupported);
        assert_eq!(
            e.message,
            "contour 1 of polygon 0: point 2 has shape parameter 2 outside the variant's range"
        );
    }
}
