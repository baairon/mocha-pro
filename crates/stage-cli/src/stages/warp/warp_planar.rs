//! `warp-planar`: a generic stage. It warps each frame by a planar projective transform, a 3 by
//! 3 matrix, with the `planar-xform` crate for the coordinates and the `resample` crate for the
//! samples. It computes a published method under explicit configuration and claims nothing
//! about any product.
//!
//! The nine entries `m00` to `m22` are read as `m[row][column]` in the vector convention of
//! `convention`. Each may be keyed, so a key file with the tracks `m00` to `m22` gives a matrix
//! per frame. What the matrix maps, where it acts and how a bad divisor is judged are the
//! convention parameters of [`super::planar`]; the samples follow [`super::sampler`].

use std::io::{Read, Write};

use planar_xform::Homography;

use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::params::Values;
use crate::registry::{
    FrameSpec, Geometry, Kind, Label, LayerDef, ParamDef, Plan, Stage, StageDef,
};

use super::planar::{self, CONVENTIONS};
use super::sampler::{self, float, SHARED};

/// The planar warp stage.
pub struct WarpPlanar;

const ENTRY: (f64, f64) = (-1e9, 1e9);

const fn entry(
    id: &'static str,
    name: &'static str,
    default: f64,
    about: &'static str,
) -> ParamDef {
    float(id, name, (default, ENTRY.0, ENTRY.1), true, about)
}

const MATRIX: [ParamDef; 9] = [
    entry(
        "m00",
        "m00",
        1.0,
        "Row 0, column 0 of the matrix; may be keyed.",
    ),
    entry(
        "m01",
        "m01",
        0.0,
        "Row 0, column 1 of the matrix; may be keyed.",
    ),
    entry(
        "m02",
        "m02",
        0.0,
        "Row 0, column 2 of the matrix; may be keyed.",
    ),
    entry(
        "m10",
        "m10",
        0.0,
        "Row 1, column 0 of the matrix; may be keyed.",
    ),
    entry(
        "m11",
        "m11",
        1.0,
        "Row 1, column 1 of the matrix; may be keyed.",
    ),
    entry(
        "m12",
        "m12",
        0.0,
        "Row 1, column 2 of the matrix; may be keyed.",
    ),
    entry(
        "m20",
        "m20",
        0.0,
        "Row 2, column 0 of the matrix; may be keyed.",
    ),
    entry(
        "m21",
        "m21",
        0.0,
        "Row 2, column 1 of the matrix; may be keyed.",
    ),
    entry(
        "m22",
        "m22",
        1.0,
        "Row 2, column 2 of the matrix; may be keyed.",
    ),
];

const OWN: [ParamDef; 9 + CONVENTIONS] = sampler::join(
    MATRIX,
    planar::conventions(
        "output-to-source",
        "Which way the matrix goes: from an output position to the source position it reads, or from a source position to where it lands; the second is inverted before use.",
        "pixel",
        "Where the matrix acts: on pixel positions, or on the unit square of each frame.",
    ),
);

static PARAMS: [ParamDef; SHARED + 9 + CONVENTIONS] = sampler::params(OWN);

static LAYERS: [LayerDef; 1] = [sampler::keys_layer(
    "A key file with a value per frame for the matrix entries m00 to m22.",
)];

/// What the stage declares. Every default is the default of `planar_xform::Config` or of
/// `resample::Config`; the default matrix is the identity.
pub static DEF: StageDef = StageDef {
    id: "warp-planar",
    name: "Planar warp",
    about: "Warps each frame by a planar projective transform given as a 3 by 3 matrix (Hartley and Zisserman 2003), sampled with a selectable reconstruction filter under explicit vector, divisor, coordinate and pixel centre rules.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Declared,
    depths: &DEPTHS,
    params: &PARAMS,
    layers: &LAYERS,
};

/// The matrix of a set of values, in the convention the values name.
pub fn matrix(values: &Values) -> Homography {
    let m = |id: &str| values.float(id);
    Homography::from_matrix(
        [
            [m("m00"), m("m01"), m("m02")],
            [m("m10"), m("m11"), m("m12")],
            [m("m20"), m("m21"), m("m22")],
        ],
        planar::config(values).convention,
    )
}

impl Stage for WarpPlanar {
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
            planar::source_map(matrix(values), values, src, dst, offset)
                .map_err(|e| planar::refuse(index, "the matrix", e))
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

    #[test]
    fn the_defaults_are_the_identity_and_the_defaults_of_the_crates() {
        let defaults = Values::defaults(&DEF);
        assert_eq!(matrix(&defaults), Homography::identity(Convention::Column));
        assert_eq!(planar::config(&defaults), Config::default());
        assert_eq!(sampler::config(&defaults), resample::Config::default());
        assert_eq!(defaults.choice("maps"), "output-to-source");
        assert_eq!(defaults.choice("coords"), "pixel");
    }

    #[test]
    fn the_entries_are_read_row_by_row_in_the_named_convention() {
        let v = values(&[
            ("m00", "1"),
            ("m01", "2"),
            ("m02", "3"),
            ("m10", "4"),
            ("m11", "5"),
            ("m12", "6"),
            ("m20", "7"),
            ("m21", "8"),
            ("m22", "9"),
            ("convention", "row"),
        ]);
        let h = matrix(&v);
        assert_eq!(h.m, [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);
        assert_eq!(h.convention, Convention::Row);
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
            ["m00", "m01", "m02", "m10", "m11", "m12", "m20", "m21", "m22"]
        );
    }
}
