# sphere-reorient

Reorientation of equirectangular (latitude and longitude) images as a pure coordinate map in `f64`. The crate has no dependency, no unsafe code and no platform specific code.

## What it does

- `equirect`: pixel to longitude and latitude, to a unit direction vector, through a 3 by 3 rotation, back to longitude and latitude, and to an output pixel coordinate. The seam is wrapped in longitude and the poles are clamped in latitude; `PixelPos` reports whether either happened. `inverse_map` returns the closure a resampler needs: output pixel coordinate in, source pixel coordinate out.
- `rotation`: `Mat3` and `Angle`. A rotation comes from a matrix the caller supplies, or from yaw, pitch and roll given as (cos, sin) pairs, in turns, or in degrees.
- `cube`: six cube faces in a layout chosen by configuration (grid shape, face order, quarter turn of each face), with the mapping between a direction and a pixel of the layout image.
- `stereo`: the over under and side by side layouts as a rectangle split, with an optional exchange of the two eyes.
- `trig`: the sine, cosine, arc tangent and arc sine used by the crate.
- `config`: `Config`, every convention in one place, and `Config::all()`, the enumeration of the configuration space (1536 configurations). `CubeConfig::all()` enumerates the cube layouts (11520).

Angles are in turns (1.0 is a full circle), so quarter and half turns are exact.

## Conventions and defaults

Every convention is a field of `Config`. The defaults are one arbitrary point of the space and claim nothing.

| field | choices | default |
|-------|---------|---------|
| `lon_zero` | fraction of the width where longitude is zero | 0.5 |
| `lon_direction` | `IncreasingRight`, `IncreasingLeft` | `IncreasingRight` |
| `pixel_center` | `Centre` (sample at `(i + 0.5) / w`), `Corner` (sample at `i / w`) | `Centre` |
| `pole_rows` | `OnPole` (`v = y / (h - 1)`), `Inset` (`v = (y + 0.5) / h`) | `Inset` |
| `north` | `Top`, `Bottom` | `Top` |
| `axis_order` | the six orders of yaw, pitch, roll | `YawPitchRoll` |
| `angle_sign` | `Positive`, `Negated` | `Positive` |
| `rotation_use` | `Direct` (source direction `R d`), `Transposed` (`R^T d`) | `Direct` |

Vectors are `[x, y, z]` with y up; longitude zero and latitude zero is `[0, 0, 1]`, and longitude grows toward `[1, 0, 0]`. Yaw turns about y, pitch about x and roll about z, right handed.

## Bit identical results

Results use only addition, subtraction, multiplication, division, square root, `floor` and `round`, which IEEE 754 fixes. There is no fused multiply add and no libm function. The four transcendental functions are implemented in `trig`:

- Sine and cosine of `t` turns: `f = t - round(t)` (exact, so the sign and the relative precision of a small `t` of either sign survive), `q = 4 f`, `n = round(q)`, `r = q - n` (exact), `a = r * pi/2` with `|a| <= pi/4`, Maclaurin series in nested form with 10 terms each (truncation error below 1e-22), then an exact quadrant rotation by `n mod 4`. Degrees are divided by 360 to give turns.
- Arc tangent: octant reduction with the exact constants 0.125, 0.25 and 0.5 turn, the identity `atan(a) = pi/4 + atan((a - 1)/(a + 1))` for `a > tan(pi/8)`, and a 25 term Maclaurin series of `atan z` for `|z| <= 0.4143`. Measured against the platform function over a grid in the tests, the difference stays below 1e-15 turns.
- Arc sine: `atan2(s, sqrt((1 - s)(1 + s)))` with `s` clamped to `[-1, 1]`.

## Sources

- J. P. Snyder, Map Projections: A Working Manual, USGS Professional Paper 1395, 1987, the equirectangular projection (plate carree): longitude and latitude are linear in the map coordinates.
- J. P. Snyder, 1987, the gnomonic projection, for the central projection of a direction onto a cube face plane.
- G. Strang, Introduction to Linear Algebra, section 2.6, and any text on rotation matrices, for the elementary rotations about the coordinate axes.
- M. Abramowitz and I. A. Stegun, Handbook of Mathematical Functions, 1964, sections 4.3.65 and 4.3.66 (sine and cosine series) and 4.4.34 and 4.4.42 (arc tangent identity and series).

## Tests

`cargo test -p sphere-reorient` runs 126 tests: exact values for identity, quarter and half turns, seam columns and pole rows; round trips over every configuration; comparisons with the platform functions inside the tests only; and pinned bit patterns for determinism.
