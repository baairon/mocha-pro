//! `warp-four-point`: a generic stage. It solves a planar projective transform from four point
//! pairs with `planar_xform::Homography::from_four_points` and then warps each frame as
//! `warp-planar` does. It computes a published method under explicit configuration and claims
//! nothing about any product.
//!
//! The four pairs. The first point of each pair is a corner of a frame, in the order of the
//! pixels: the corner at the first pixel of the first row, the corner after the last pixel of
//! the first row, the corner after the last pixel of the last row, the corner at the first
//! pixel of the last row. The second point is `(x0, y0)` to `(x3, y3)`, in the same order. Both
//! are in the coordinates that `coords` names:
//!
//! - `unit` (the default): the unit square of the frame, so the corners are `(0, 0)`, `(1, 0)`,
//!   `(1, 1)`, `(0, 1)` when `y_axis` is `down`, and the defaults of the eight parameters are
//!   the corners of the frame whatever its size.
//! - `pixel`: pixel positions with x multiplied by `pixel_aspect`. The corners of a frame of w
//!   by h pixels are `(0, 0)`, `(w, 0)`, `(w, h)`, `(0, h)` when `xform_center` is `half`. The
//!   defaults of the eight parameters are not the corners of a frame here, so all eight are
//!   given.
//!
//! `maps` says which frame the corners belong to. `source-to-output` (the default): the corners
//! of the source frame go to the eight values, and the solved transform is inverted before use.
//! `output-to-source`: the corners of the output frame read the source at the eight values, and
//! the solved transform is used as it is.
//!
//! A set of corners the solve refuses (three on a line, two equal) is `unsupported`.

use std::io::{Read, Write};

use planar_xform::{Error, Homography, Point, YAxis};

use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::params::Values;
use crate::registry::{
    FrameSpec, Geometry, Kind, Label, LayerDef, ParamDef, Plan, Stage, StageDef,
};

use super::planar::{self, CONVENTIONS};
use super::sampler::{self, float, SHARED};

/// The four point warp stage.
pub struct WarpFourPoint;

const fn corner(
    id: &'static str,
    name: &'static str,
    default: f64,
    about: &'static str,
) -> ParamDef {
    float(id, name, (default, -1e9, 1e9), true, about)
}

const CORNERS: [ParamDef; 8] = [
    corner("x0", "Corner 0 x", 0.0, "The x coordinate paired with the corner at the first pixel of the first row; may be keyed."),
    corner("y0", "Corner 0 y", 0.0, "The y coordinate paired with the corner at the first pixel of the first row; may be keyed."),
    corner("x1", "Corner 1 x", 1.0, "The x coordinate paired with the corner after the last pixel of the first row; may be keyed."),
    corner("y1", "Corner 1 y", 0.0, "The y coordinate paired with the corner after the last pixel of the first row; may be keyed."),
    corner("x2", "Corner 2 x", 1.0, "The x coordinate paired with the corner after the last pixel of the last row; may be keyed."),
    corner("y2", "Corner 2 y", 1.0, "The y coordinate paired with the corner after the last pixel of the last row; may be keyed."),
    corner("x3", "Corner 3 x", 0.0, "The x coordinate paired with the corner at the first pixel of the last row; may be keyed."),
    corner("y3", "Corner 3 y", 1.0, "The y coordinate paired with the corner at the first pixel of the last row; may be keyed."),
];

const OWN: [ParamDef; 8 + CONVENTIONS] = sampler::join(
    CORNERS,
    planar::conventions(
        "source-to-output",
        "Which frame the four corners belong to: the corners of the source go to the eight values, or the corners of the output read the source at the eight values.",
        "unit",
        "The coordinates of the eight values: the unit square of the frame, or pixel positions.",
    ),
);

static PARAMS: [ParamDef; SHARED + 8 + CONVENTIONS] = sampler::params(OWN);

static LAYERS: [LayerDef; 1] = [sampler::keys_layer(
    "A key file with a value per frame for the corner coordinates x0, y0 to x3, y3.",
)];

/// What the stage declares. Every default is the default of `planar_xform::Config` or of
/// `resample::Config`, except the two choices the module text names.
pub static DEF: StageDef = StageDef {
    id: "warp-four-point",
    name: "Four point warp",
    about: "Warps each frame by the planar projective transform that takes the four corners of a frame to four given points, solved by the direct linear transformation (Hartley and Zisserman 2003) and sampled with a selectable reconstruction filter.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Declared,
    depths: &DEPTHS,
    params: &PARAMS,
    layers: &LAYERS,
};

/// The four corners of a frame of `size` pixels and the four points of the values, both in the
/// coordinates that `coords` names.
pub fn pairs(values: &Values, size: (u32, u32)) -> ([Point; 4], [Point; 4]) {
    let cfg = planar::config(values);
    let (w, h) = (f64::from(size.0), f64::from(size.1));
    let (corners, aspect) = if values.choice("coords") == "pixel" {
        // The edge of the frame lies half a pixel before the centre of its first pixel.
        let c = 0.5 - planar::matrix_offset(&cfg);
        let a = cfg.pixel_aspect;
        let frame = [(-c, -c), (w - c, -c), (w - c, h - c), (-c, h - c)];
        (frame.map(|(x, y)| Point::new(x * a, y)), a)
    } else {
        let square = match cfg.y_axis {
            YAxis::Down => [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
            YAxis::Up => [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)],
        };
        (square.map(|(u, v)| Point::new(u, v)), 1.0)
    };
    let given = [("x0", "y0"), ("x1", "y1"), ("x2", "y2"), ("x3", "y3")]
        .map(|(x, y)| Point::new(values.float(x) * aspect, values.float(y)));
    (corners, given)
}

/// The transform of a set of values for a source and an output size: the corners of the frame
/// that `maps` names to the four given points.
pub fn solve(values: &Values, src: (u32, u32), dst: (u32, u32)) -> Result<Homography, Error> {
    let size = match values.choice("maps") {
        "output-to-source" => dst,
        _ => src,
    };
    let (corners, given) = pairs(values, size);
    Homography::from_four_points(&corners, &given, &planar::config(values))
}

impl Stage for WarpFourPoint {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    fn output_size(&self, input: &FrameSpec, values: &Values) -> Result<(u32, u32), StageError> {
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
        sampler::warp_frames(plan, input, output, |index, values| {
            solve(values, src, dst)
                .and_then(|solved| planar::source_map(solved, values, src, dst, offset))
                .map_err(|e| planar::refuse(index, "the transform of the four corners", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use planar_xform::{Config, Convention};

    fn values(pairs: &[(&str, &str)]) -> Values {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Values::parse(&DEF, &pairs).unwrap()
    }

    fn points(list: [(f64, f64); 4]) -> [Point; 4] {
        list.map(|(x, y)| Point::new(x, y))
    }

    #[test]
    fn the_defaults_are_the_corners_of_the_frame_and_the_defaults_of_the_crates() {
        let defaults = Values::defaults(&DEF);
        assert_eq!(planar::config(&defaults), Config::default());
        assert_eq!(sampler::config(&defaults), resample::Config::default());
        assert_eq!(defaults.choice("maps"), "source-to-output");
        assert_eq!(defaults.choice("coords"), "unit");
        for size in [(4, 2), (7, 5), (1920, 1080)] {
            let (corners, given) = pairs(&defaults, size);
            assert_eq!(corners, given, "{size:?}");
            assert_eq!(
                corners,
                points([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)])
            );
        }
        let solved = solve(&defaults, (4, 2), (4, 2)).unwrap();
        assert_eq!(solved.m, Homography::identity(Convention::Column).m);
    }

    #[test]
    fn the_corners_follow_the_coordinates_and_the_pixel_rules() {
        let pixel = values(&[("coords", "pixel")]);
        assert_eq!(
            pairs(&pixel, (4, 2)).0,
            points([(0.0, 0.0), (4.0, 0.0), (4.0, 2.0), (0.0, 2.0)])
        );
        let integer = values(&[("coords", "pixel"), ("xform_center", "integer")]);
        assert_eq!(
            pairs(&integer, (4, 2)).0,
            points([(-0.5, -0.5), (3.5, -0.5), (3.5, 1.5), (-0.5, 1.5)])
        );
        let wide = values(&[("coords", "pixel"), ("pixel_aspect", "2"), ("x1", "3")]);
        let (corners, given) = pairs(&wide, (4, 2));
        assert_eq!(
            corners,
            points([(0.0, 0.0), (8.0, 0.0), (8.0, 2.0), (0.0, 2.0)])
        );
        assert_eq!(given[1], Point::new(6.0, 0.0));
        let up = values(&[("y_axis", "up")]);
        assert_eq!(
            pairs(&up, (4, 2)).0,
            points([(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)])
        );
    }

    #[test]
    fn maps_names_the_frame_of_the_corners() {
        let forward = values(&[("coords", "pixel")]);
        let back = values(&[("coords", "pixel"), ("maps", "output-to-source")]);
        // The given points are the unit square; the frame differs.
        let from_source = solve(&forward, (4, 2), (8, 4)).unwrap();
        let cfg = Config::default();
        assert_eq!(
            from_source.apply(Point::new(4.0, 2.0), &cfg),
            Some(Point::new(1.0, 1.0))
        );
        let from_output = solve(&back, (4, 2), (8, 4)).unwrap();
        assert_eq!(
            from_output.apply(Point::new(8.0, 4.0), &cfg),
            Some(Point::new(1.0, 1.0))
        );
    }

    #[test]
    fn corners_on_a_line_are_refused() {
        let flat = values(&[("y2", "0"), ("y3", "0")]);
        assert_eq!(solve(&flat, (4, 2), (4, 2)).err(), Some(Error::Singular));
    }

    #[test]
    fn the_definition_holds_the_shared_parameters_first() {
        assert_eq!(&DEF.params[..SHARED], &sampler::SHARED_PARAMS[..]);
        assert_eq!(DEF.params.len(), 31);
        let keyed: Vec<&str> = DEF
            .params
            .iter()
            .filter(|p| p.keyed)
            .map(|p| p.id)
            .collect();
        assert_eq!(keyed, ["x0", "y0", "x1", "y1", "x2", "y2", "x3", "y3"]);
    }
}
