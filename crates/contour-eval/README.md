# contour-eval

Generic evaluation of planar contours made of cubic Bezier spans or X-spline segments, and flattening of a contour to a polyline. The crate uses the standard library only and holds no data of any program: its tests use made up points.

## What it does

A `Contour` has a `Kind` (`Bezier` or `XSpline { variant }`), a closed flag and a list of `Point` values. A point carries a position, two tangent handles (Bezier) and a stored weight `s` (X-spline); fields a kind does not use are ignored for that kind.

- `eval(&contour, span, t, &config)` gives the point of span `span` at parameter `t` in [0, 1].
- `flatten(&contour, &rule, &config)` gives a polyline under one of three rules: `Uniform(n)`, `Subdivide { tol, max_depth }` and `ByLength { step, samples }`.
- `bezier_split` cuts a cubic Bezier span in two by de Casteljau subdivision.
- `f_blend`, `g_blend`, `h_blend` and `xspline_weights` expose the X-spline blending functions and the four weights of a segment.
- Every refusal is a typed `ContourError`; no input makes the crate panic. A contour with fewer than two points, a non-finite used field, a span index or parameter out of range, a shape parameter outside the variant's range and a bad flattening constant are all refused.

## Published definitions

- Cubic Bezier: G. Farin, "Curves and Surfaces for CAGD: A Practical Guide", 5th edition, Morgan Kaufmann, 2002. The Bernstein form is chapter 4; the de Casteljau algorithm and subdivision are chapter 3. B(t) = (1-t)^3 P0 + 3 (1-t)^2 t P1 + 3 (1-t) t^2 P2 + t^3 P3. Three evaluation forms are offered (Bernstein, de Casteljau, Horner); at t = 0 and t = 1 every form returns the end points unchanged.
- X-splines: C. Blanc and C. Schlick, "X-Splines: A Spline Model Designed for the End-User", Proceedings of SIGGRAPH 1995, pp. 377 to 386. A segment between P1 and P2 is a weighted mean of P0, P1, P2 and P3, the weights come from the blending functions F, G and H of the paper, and they are divided by their sum. A shape parameter s above 0 gives an approximating point, below 0 an interpolating point, and 0 a sharp point on the curve. The general variant takes s in [-1, 1]; the basic variant takes s in [0, 1].
- Flatness test: J. Lane and R. Riesenfeld, "A Theoretical Development for the Computer Generation and Display of Piecewise Polynomial Surfaces", IEEE Transactions on Pattern Analysis and Machine Intelligence 2(1), 1980. The control polygon test keeps a Bezier span when both inner control points lie within the tolerance of the chord between its end points.

The page of the X-spline paper was not available in machine readable form to the author of this crate. The blending polynomials in `src/xspline.rs` therefore follow the reference implementation of the paper that the xfig drawing program ships, and are not verified against the paper itself. The tests check them against what the paper states: a sharp point at s = 0, the point (P0 + 4 P1 + P2) / 6 of the uniform cubic B-spline at the start of a segment for s = 1 at both ends (and the mirrored point at its end), weights close to the B-spline weights inside the segment, interpolation for s <= 0, non-negative weights for the basic variant, negative weights for the general variant, and continuity of adjacent segments.

## What the papers leave open

Each of these is a field of `Config`. `Config::canonical()` is a neutral default and claims nothing about any other program; `Config::enumerate()` lists 768 combinations in a fixed order.

| Choice | Field | Values |
|---|---|---|
| Evaluation form of a Bezier span | `form` | `Bernstein` (default), `DeCasteljau`, `Horner` |
| Number type | `precision` | `F64` (default), `F32` |
| Tangent handles | `tangents` | `Relative` offsets (default), `Absolute` positions |
| Stored weight to shape parameter | `weight_map` | s = a w + b; identity by default; four maps listed in `Config::WEIGHT_MAPS` |
| X-spline variant | `Kind::XSpline { variant }` | `Basic`, `General` |
| Ends of an open X-spline contour | `open_end` | `RepeatEnd` (default), `RepeatEndSharp` |
| Closing span of a closed contour | `closing_span` | on by default; off makes a closed contour behave as an open one |
| Repeated first point of a closed polyline | `repeat_first` | off by default |
| Subdivision flatness test | `flat_test` | `Sampled` (default), `ControlPolygon` (Bezier only) |

Flattening constants are arguments of the rule, not of the configuration: the segment count, the tolerance and depth, or the step and the number of length samples. Limits: depth at most 24, at most 4096 segments per span.

The `Sampled` test compares the curve points at 1/4, 1/2 and 3/4 of an interval with its chord. It is a sample test, not a bound; the `ControlPolygon` test is a bound on the distance of the curve from the polyline.

## Floating point

Results are the same on Linux, macOS and Windows. The evaluation path uses add, subtract, multiply, divide and negate, the operations IEEE 754 fixes, and a square root in the length estimate of `ByLength`. It calls no libm function and uses no fused multiply add; the order of operations is written out in the source. `F32` computes in `f32` and returns `f64` values that are exactly the `f32` results.

## Relation to shape-model

`shape-model` stores keyframed control points of a contour and blends them in time; it does not evaluate curves. The types here are curve level: a `Point` here corresponds to one `PointData` of one key (anchor, the two tangents, and a weight). `shape-model` is unchanged and does not depend on this crate. A later change could make `shape-model` depend on `contour-eval` and add a function that maps the points of a key to a `Contour`; the dependency would run from `shape-model` to this crate, never the reverse, and the mapping of the two optional edge numbers and the two flags of `PointData` to a weight and a closed flag would be a configuration of that adapter.

## Tests

`cargo test -p contour-eval` runs 132 tests: vectors written as bit patterns and computed in exact rational arithmetic (`tests/vectors.rs`), Bezier properties (`tests/bezier.rs`), X-spline properties (`tests/xspline.rs`), flattening (`tests/flatten.rs` and the unit tests in `src/flatten.rs`), refusals (`tests/errors.rs`) the configuration space (`tests/config.rs`) and the segment audit (`tests/audit_segment.rs`).
