# mesh-warp

A generic, dependency free Rust crate that warps by a triangle mesh as a pure coordinate map in `f64`. It maps coordinates only; it reads and writes no pixels.

## What it does

- `Mesh`: vertices that each carry a source position and a destination position, and a triangle index list. `Mesh::validate` checks indices in range, finite values, degenerate triangles (refused unless the configuration allows them) and, optionally, a consistent winding.
- `Warp`: validates a mesh and builds a point locator in each space. `inverse` maps a destination point to a source position, `forward` maps a source point to a destination position, both by barycentric interpolation. `inverse_closure` returns the closure `Fn(f64, f64) -> Option<(f64, f64)>` that a resampler needs.
- `Locator`: a uniform grid over the triangles of one space. The cell of a coordinate is `clamp(floor((v - min) / size), 0, n - 1)` with `size = (max - min) / n`; a triangle is listed in every cell from the cell of its bounding box minimum to the cell of its maximum, in index order.
- Tie rule for a point on a shared edge or vertex: `LowestIndex` (the closed containing triangle of lowest index) or `TopLeft` (the first triangle that owns the point under the top left rule; a border point owned by no triangle falls back to the lowest index).
- Outside rule for a point in no triangle: `None`, `ClampNearest` (the nearest boundary point of the nearest triangle is mapped) or `ExtendAffine` (the affine map of the nearest triangle is extended).
- `barycentric`: two stated formulas and operation orders, `EdgeFunctions` and `Cramer`.
- `grid_mesh`: a regular grid mesh with a configurable diagonal (`Forward`, `Back`, `Alternating`).
- `QuadPatch`: a bilinear quad patch with a forward map and an exact inverse. The inverse solves the quadratic in `u` with a configurable root choice and the IEEE square root.
- `Config` and `QuadConfig` hold every choice. Their defaults claim nothing. `config_space()` enumerates the 96 discrete configurations and `quad_config_space()` the 6 quad ones.

## Determinism

Only addition, subtraction, multiplication, division, comparison, `floor` and the IEEE square root are used. There is no fused multiply add, no transcendental function and no unsafe code, so results are bit identical on Linux, macOS and Windows.

## Public sources

- G. Wolberg, "Digital Image Warping", IEEE Computer Society Press, 1990, chapter 3 (spatial transformations: affine and bilinear maps) and the mesh warping section.
- J. Pineda, "A parallel algorithm for polygon rasterization", Computer Graphics (SIGGRAPH 88), 1988 (edge functions and the top left ownership rule).
- C. Ericson, "Real-Time Collision Detection", Morgan Kaufmann, 2005, section 3.4 (barycentric coordinates) and section 5.1 (closest point on a triangle).

## Tests

`cargo test -p mesh-warp` runs the unit and property tests: hand computed exact values (identity, integer translation, one affine triangle, shared edge ties under each rule, outside points under each rule, quad corners and centre), an integer reference for the ownership rule, a slow barycentric reference, a Newton reference for the quad inverse, and run to run determinism.
