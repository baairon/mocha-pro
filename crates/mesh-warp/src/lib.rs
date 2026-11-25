//! `mesh-warp`: warping by a triangle mesh as a pure coordinate map in `f64`.
//!
//! A [`Mesh`] holds vertices with a source position and a destination position each, and a
//! triangle index list. A [`Warp`] locates a point in one of the two meshes (a uniform grid
//! acceleration structure, with a stated cell rule and tie rule) and maps it by barycentric
//! interpolation: the inverse map takes a destination point to a source position, the forward
//! map the other way. Behaviour outside the mesh is a [`Config`] field. [`QuadPatch`] is the
//! bilinear quad variant with its exact inverse.
//!
//! Every choice that may need to vary is a field of [`Config`] or [`QuadConfig`]; the defaults
//! claim nothing, and [`config_space`] and [`quad_config_space`] enumerate the choices.
//!
//! Determinism: only addition, subtraction, multiplication, division, comparison, `floor` of an
//! exactly representable integer-valued step and the IEEE square root are used. No fused multiply
//! add and no transcendental function appears, so results are bit identical on every platform.
//!
//! Public sources:
//! - G. Wolberg, "Digital Image Warping", IEEE Computer Society Press, 1990, chapter 3 (spatial
//!   transformations, affine and bilinear maps) and the mesh warping section.
//! - J. Pineda, "A parallel algorithm for polygon rasterization", SIGGRAPH 1988 (edge functions and
//!   the top left ownership rule for shared edges).
//! - C. Ericson, "Real-Time Collision Detection", 2005, section 3.4 (barycentric coordinates).

mod quad;
mod warp;

pub use quad::{quad_config_space, QuadConfig, QuadPatch, RootChoice};
pub use warp::{
    barycentric, config_space, grid_mesh, BaryForm, Config, Diagonal, GridRes, Located, Locator,
    Mesh, MeshError, Outside, Space, TieRule, Warp, Winding,
};

/// A point or vector in the plane.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const fn new(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }
}

/// A mesh vertex: where it sits in the source image and where it sits in the destination image.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vertex {
    pub src: Vec2,
    pub dst: Vec2,
}

/// The planar cross product `a.x * b.y - a.y * b.x`, in that operation order.
pub fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}
