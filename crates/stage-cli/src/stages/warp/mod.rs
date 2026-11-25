//! Stages that move pixels under a mapping.
//!
//! One `pub mod` line and one entry in [`stages`] per stage, under a line that names its id.
//! [`sampler`] holds what every warp shares (the output size, the parameters of the resampler,
//! the frame loop), [`planar`] what the two planar stages share and [`mesh_form`] the mesh
//! file of `warp-mesh`.

pub mod mesh_form;
pub mod planar;
pub mod sampler;
pub mod warp_four_point;
pub mod warp_lens;
pub mod warp_mesh;
pub mod warp_planar;
pub mod warp_sphere;

use crate::registry::Stage;

/// The stages of this family, in manifest order.
pub fn stages() -> Vec<&'static dyn Stage> {
    vec![
        // warp-planar
        &warp_planar::WarpPlanar,
        // warp-four-point
        &warp_four_point::WarpFourPoint,
        // warp-lens
        &warp_lens::WarpLens,
        // warp-sphere
        &warp_sphere::WarpSphere,
        // warp-mesh
        &warp_mesh::WarpMesh,
    ]
}
