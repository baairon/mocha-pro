//! `warp-mesh`: a generic stage. It warps each frame by a triangle mesh, with the `mesh-warp`
//! crate for the coordinates and the `resample` crate for the samples. It computes a published
//! method under explicit configuration and claims nothing about any product.
//!
//! The mesh comes from `--layer mesh=PATH`, a file of the form `mesh-1` ([`super::mesh_form`]):
//! vertices with a position in the source frame and a position in the output frame, and
//! triangles. Each output position is located in the output positions of the mesh and reads the
//! source at the same barycentric coordinates (`mesh_warp::Warp::inverse`). A position outside
//! every triangle follows `outside`; with `none` it is zero.
//!
//! Rules of the stage:
//!
//! - The file is read once, before the first frame, and one mesh serves every frame. A file
//!   that cannot be read or does not follow the form is `bad_input`. A mesh the configuration
//!   refuses (a triangle of no area without `allow_degenerate`, mixed winding with `winding`
//!   `consistent`) is `unsupported`. Both end the run before a frame is read.
//! - Without the layer the stage maps every position to itself.
//! - The file places pixel i with its centre at i + 0.5. When the resampler places it at i
//!   (`center` is `corner`), a position is moved by half a pixel before the map and back after
//!   it.
//! - The point location parameter is `mesh_tie`, because `tie` is the rule of the nearest filter.

use std::io::{Read, Write};

use mesh_warp::{BaryForm, Config, GridRes, MeshError, Outside, TieRule, Vec2, Warp, Winding};

use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::params::Values;
use crate::registry::{
    FrameSpec, Geometry, Kind, Label, LayerDef, ParamDef, Plan, Stage, StageDef,
};

use super::mesh_form;
use super::sampler::{self, checkbox, choice, int, SHARED};

/// The mesh warp stage.
pub struct WarpMesh;

const OWN: [ParamDef; 8] = [
    choice("mesh_tie", "Shared edge rule", "lowest-index", &["lowest-index", "pineda"], "Which triangle owns a point on a shared edge or vertex: the containing triangle of lowest index, or the one the fill rule of Pineda 1988 names (Config::tie)."),
    choice("outside", "Outside rule", "none", &["none", "clamp-nearest", "extend-affine"], "What a point in no triangle maps to: nothing, the nearest point of the mesh, or the extended map of the nearest triangle (Config::outside)."),
    choice("winding", "Winding", "any", &["any", "consistent"], "Whether triangles of both orientations are taken or all must turn the same way (Config::winding)."),
    choice("bary", "Barycentric form", "edge-functions", &["edge-functions", "cramer"], "The formula of the barycentric coordinates, which matters for rounding only (Config::bary)."),
    checkbox("allow_degenerate", "Allow degenerate", false, "Whether triangles of no area are taken and skipped or refused (Config::allow_degenerate)."),
    choice("grid", "Location grid", "per-triangle-sqrt", &["per-triangle-sqrt", "fixed"], "The size of the grid that speeds up point location: the square root of the triangle count per side, or grid_cols by grid_rows (Config::grid)."),
    int("grid_cols", "Grid columns", (1, 1, 4096), "The columns of the location grid; read when grid is fixed."),
    int("grid_rows", "Grid rows", (1, 1, 4096), "The rows of the location grid; read when grid is fixed."),
];

static PARAMS: [ParamDef; SHARED + 8] = sampler::params(OWN);

static LAYERS: [LayerDef; 1] = [LayerDef {
    id: mesh_form::LAYER_ID,
    optional: true,
    form: mesh_form::FORM,
    about: "A mesh file: vertices with a source and an output position in pixels, and triangles. Without it every position maps to itself.",
}];

/// What the stage declares. Every default is the default of `mesh_warp::Config` or of
/// `resample::Config`.
pub static DEF: StageDef = StageDef {
    id: "warp-mesh",
    name: "Mesh warp",
    about: "Warps each frame by a triangle mesh with barycentric interpolation inside each triangle (Wolberg 1990) under an explicit shared edge, outside, winding and degenerate triangle rule, sampled with a selectable reconstruction filter.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Declared,
    depths: &DEPTHS,
    params: &PARAMS,
    layers: &LAYERS,
};

/// The configuration of `mesh_warp` for a set of values.
pub fn config(values: &Values) -> Config {
    Config {
        tie: match values.choice("mesh_tie") {
            "pineda" => TieRule::TopLeft,
            _ => TieRule::LowestIndex,
        },
        outside: match values.choice("outside") {
            "clamp-nearest" => Outside::ClampNearest,
            "extend-affine" => Outside::ExtendAffine,
            _ => Outside::None,
        },
        winding: match values.choice("winding") {
            "consistent" => Winding::Consistent,
            _ => Winding::Any,
        },
        bary: match values.choice("bary") {
            "cramer" => BaryForm::Cramer,
            _ => BaryForm::EdgeFunctions,
        },
        allow_degenerate: values.flag("allow_degenerate"),
        grid: match values.choice("grid") {
            "fixed" => GridRes::Fixed(
                values.int("grid_cols") as usize,
                values.int("grid_rows") as usize,
            ),
            _ => GridRes::PerTriangleSqrt,
        },
    }
}

/// The refusal for a mesh the configuration does not take.
pub fn refuse(path: &str, error: &MeshError) -> StageError {
    let why = match error {
        MeshError::NoTriangles => "it has no triangle".to_string(),
        MeshError::IndexOutOfRange { triangle, index } => {
            format!("triangle {triangle} names vertex {index}, which is not in the file")
        }
        MeshError::NonFinite { vertex } => {
            format!("vertex {vertex} has a position that is not finite")
        }
        MeshError::Degenerate { triangle } => {
            format!("triangle {triangle} has no area and allow_degenerate is off")
        }
        MeshError::InconsistentWinding { triangle } => {
            format!("triangle {triangle} turns the other way and winding is consistent")
        }
    };
    StageError::unsupported(format!("mesh file `{path}`: {why}"))
}

/// Read the mesh file of a plan and build the warp, or `None` when the layer was not given.
pub fn warp(plan: &Plan) -> Result<Option<Warp>, StageError> {
    let Some(path) = plan.layers.path(mesh_form::LAYER_ID) else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(path)
        .map_err(|e| StageError::bad_input(format!("cannot read the mesh file `{path}`: {e}")))?;
    let mesh = mesh_form::parse(&text)
        .map_err(|why| StageError::bad_input(format!("mesh file `{path}`: {why}")))?;
    Warp::new(mesh, &config(&plan.values))
        .map(Some)
        .map_err(|e| refuse(path, &e))
}

impl Stage for WarpMesh {
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
        let warp = warp(plan)?;
        let shift = 0.5 - sampler::centre_offset(&plan.values);
        sampler::warp_frames(plan, input, output, |_, _| {
            let warp = warp.as_ref();
            Ok(move |x: f64, y: f64| match warp {
                None => Some((x, y)),
                Some(warp) => warp
                    .inverse(Vec2::new(x + shift, y + shift))
                    .map(|q| (q.x - shift, q.y - shift)),
            })
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
        assert_eq!(config(&defaults), Config::default());
        assert_eq!(sampler::config(&defaults), resample::Config::default());
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        let c = |pairs: &[(&str, &str)]| config(&values(pairs));
        assert_eq!(c(&[("mesh_tie", "pineda")]).tie, TieRule::TopLeft);
        assert_eq!(
            c(&[("outside", "clamp-nearest")]).outside,
            Outside::ClampNearest
        );
        assert_eq!(
            c(&[("outside", "extend-affine")]).outside,
            Outside::ExtendAffine
        );
        assert_eq!(c(&[("winding", "consistent")]).winding, Winding::Consistent);
        assert_eq!(c(&[("bary", "cramer")]).bary, BaryForm::Cramer);
        for word in ["1", "true", "on"] {
            assert!(c(&[("allow_degenerate", word)]).allow_degenerate);
        }
        for word in ["0", "false", "off"] {
            assert!(!c(&[("allow_degenerate", word)]).allow_degenerate);
        }
        assert_eq!(
            c(&[("grid", "fixed"), ("grid_cols", "7"), ("grid_rows", "3")]).grid,
            GridRes::Fixed(7, 3)
        );
        assert_eq!(c(&[("grid", "fixed")]).grid, GridRes::Fixed(1, 1));
        // the two counts are read only when the grid is fixed
        assert_eq!(c(&[("grid_cols", "7")]).grid, GridRes::PerTriangleSqrt);
    }

    #[test]
    fn every_configuration_of_the_crate_can_be_reached() {
        let mut reached = Vec::new();
        for tie in ["lowest-index", "pineda"] {
            for outside in ["none", "clamp-nearest", "extend-affine"] {
                for winding in ["any", "consistent"] {
                    for bary in ["edge-functions", "cramer"] {
                        for degenerate in ["0", "1"] {
                            for grid in ["per-triangle-sqrt", "fixed"] {
                                reached.push(config(&values(&[
                                    ("mesh_tie", tie),
                                    ("outside", outside),
                                    ("winding", winding),
                                    ("bary", bary),
                                    ("allow_degenerate", degenerate),
                                    ("grid", grid),
                                ])));
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(reached, mesh_warp::config_space());
    }

    #[test]
    fn each_mesh_error_has_its_line() {
        let line = |e: MeshError| refuse("m.txt", &e).to_string();
        assert_eq!(
            line(MeshError::Degenerate { triangle: 3 }),
            "error: unsupported: mesh file `m.txt`: triangle 3 has no area and allow_degenerate is off"
        );
        assert_eq!(
            line(MeshError::InconsistentWinding { triangle: 1 }),
            "error: unsupported: mesh file `m.txt`: triangle 1 turns the other way and winding is consistent"
        );
        assert_eq!(
            line(MeshError::NoTriangles),
            "error: unsupported: mesh file `m.txt`: it has no triangle"
        );
        assert_eq!(
            line(MeshError::IndexOutOfRange { triangle: 0, index: 9 }),
            "error: unsupported: mesh file `m.txt`: triangle 0 names vertex 9, which is not in the file"
        );
        assert_eq!(
            line(MeshError::NonFinite { vertex: 2 }),
            "error: unsupported: mesh file `m.txt`: vertex 2 has a position that is not finite"
        );
    }

    #[test]
    fn the_definition_holds_the_shared_parameters_first() {
        assert_eq!(&DEF.params[..SHARED], &sampler::SHARED_PARAMS[..]);
        assert_eq!(DEF.params.len(), 22);
        assert!(DEF.params.iter().all(|p| !p.keyed));
        assert_eq!(DEF.layers.len(), 1);
        assert_eq!((DEF.layers[0].id, DEF.layers[0].form), ("mesh", "mesh-1"));
        assert!(DEF.layers[0].optional);
    }
}
