# planar-xform

A 3 by 3 planar projective transform (a homography) in `f64`, with every convention stated and
every choice that a caller may want to vary held in one `Config` value. The crate uses only the
standard library, has no unsafe code and takes no dependency.

## What it offers

- `Homography`: a matrix `m[row][col]` plus a vector convention (`Column`: `p' = M p`, `Row`:
  `p' = p M`). Constructors: `from_matrix`, `identity`, `translation`, `scale`,
  `rotation_quarter_turns` (exact entries 0, 1 and -1) and `rotation_cos_sin` (the cosine and
  sine are used as given, with no normalisation).
- `then`: composition, `self` first and `next` second (column: `M = Mnext * Mself`; row:
  `M = Mself * Mnext`).
- `inverse`: the adjugate divided by the determinant. A zero determinant is `Singular`, a
  non-finite one is `NonFinite`.
- `apply`: the image of a point with the homogeneous divide. A divisor that is not greater than
  `Config::divisor_min` goes to `Config::guard`: return `None`, divide by a stated value, or
  return the undivided numerators.
- `from_four_points`: the direct linear solution of the 8 by 8 system with `h22 = 1`, by
  Gaussian elimination with partial pivoting (largest magnitude in the column among the
  remaining rows, the lowest row on a tie). A pivot of magnitude at most
  `singular_tolerance` times the largest magnitude of the system is refused.
- `map_rect` and `map_rect_bounds`: the image quad of a rectangle and its bounding box.
- `PixelSpace`: conversions between the normalised unit square and pixel coordinates, with
  integer or half integer pixel centres, a Y axis that runs down or up, and a pixel aspect.
- `Config::enumerate`: the product of every discrete choice, for a caller that wants to try
  each one against reference data.

The default `Config` claims nothing about any application: column vectors, `None` on a bad
divisor with a minimum of zero, half integer centres, Y down, square pixels, tolerance `1e-12`.

## Exactness

Only addition, subtraction, multiplication, division and comparison are used, in a fixed
order, and never a fused multiply add. Equal inputs therefore give equal bits on every
platform. Exact values (identity, integer translations, scales by powers of two, quarter turns)
stay exact.

## Sources

- R. Hartley and A. Zisserman, "Multiple View Geometry in Computer Vision", 2nd edition,
  Cambridge University Press, 2003: section 2.3 (projective transformations), section 2.4.1
  (isometries) and section 4.1 (the direct linear transformation, with four point
  correspondences).
- G. Strang, "Linear Algebra and Its Applications", 4th edition, 2006, section 4.4 (cofactors,
  the adjugate and the inverse).
- IEEE Std 754-2019, "IEEE Standard for Floating-Point Arithmetic", for the fixed results of
  the basic operations.

## Tests

The tests use made up data and hand computed expectations, an independent slow solver in
rational arithmetic, a pinned bit pattern from a separate floating point implementation, and
checks of every configuration choice.
