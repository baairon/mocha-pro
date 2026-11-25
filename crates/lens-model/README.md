# lens-model

Lens distortion models as pure coordinate maps in `f64`, with no dependency and no unsafe code.

## Models

- Polynomial radial and tangential (Brown). With `r2 = x*x + y*y` and
  `g = 1 + k1*r2 + k2*r2^2 + k3*r2^3`: `x' = x*g + 2*p1*x*y + p2*(r2 + 2*x*x)` and
  `y' = y*g + p1*(r2 + 2*y*y) + 2*p2*x*y`.
  The tangential terms use the OpenCV ordering: Brown's own P1 multiplies `r2 + 2*x*x` in `x'` and P2
  multiplies `2*x*y`, so `p1` here is Brown's P2 and `p2` here is Brown's P1.
  Sources: D. C. Brown, "Decentering Distortion of Lenses", Photogrammetric Engineering 32(3), 1966;
  D. C. Brown, "Close-Range Camera Calibration", Photogrammetric Engineering 37(8), 1971, the sections
  on radial and decentering distortion.
- One parameter division model: `q = p / (1 + lambda*|p|^2)`.
  Source: A. W. Fitzgibbon, "Simultaneous linear estimation of multiple view geometry and lens
  distortion", CVPR 2001, section 2. The closed form inverse
  `p = 2*q / (1 + sqrt(1 - 4*lambda*|q|^2))` solves the same relation for the radius and takes the root
  that tends to `q` as `lambda` tends to zero; it is derived in this crate.
- Squeezed variant, defined in this crate: the radius is computed from squeezed coordinates,
  `r2 = (sx*x)^2 + (sy*y)^2`, and the point is scaled by the radial polynomial `g(r2)`.

## Configuration

Every convention is a field of `Config`, and the defaults are arbitrary starting values that claim
nothing: the model, the normalisation (half width, half height, half diagonal, or a focal length in
pixels), the image size, the distortion centre in pixels, the pixel aspect (pixel width over pixel
height), which direction the model formula maps (`ModelMaps`), the inverse method (fixed point,
Newton, closed form) and the fixed iteration count. `ConfigSpace` lists the axes and
`ConfigSpace::enumerate` returns every combination in a fixed order.

Pixel to normalised: `((px - cx) * aspect / s, (py - cy) / s)`, where `s` is the normalisation divisor.

## Maps

- `Model::apply`, `Model::invert` (fixed point `u <- u - (F(u) - d)` from `u = d`; Newton with the
  analytic 2 by 2 Jacobian from `u = d`; closed form for the division model), `Model::closed_form`.
- `Config::to_distorted` and `Config::to_undistorted` on pixel positions.
- `Config::source_map(Direction)`: the closure a resampler needs, for applying distortion to an image
  and for removing it.
- `Config::mapped_bounds(Direction, steps)`: the bounding box of the image rectangle mapped along its
  border, with `steps + 1` samples on each edge.

## Reproducibility

Only addition, subtraction, multiplication, division and square root are used, which IEEE 754 fixes
exactly, and no fused multiply add. The iterative inverses run exactly the requested number of steps
with no convergence test, so the same input gives the same bits on every platform. A Newton step with a
zero or non finite Jacobian determinant leaves the iterate unchanged (a non finite residual is an error).

## Tests

The tests in `tests/` use hand computed values on dyadic inputs, an independent bisection reference,
and the division closed form against the iterations.
