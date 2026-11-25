# coverage-raster

`coverage-raster` turns closed polygons in floating point coordinates into a one channel coverage buffer. It is a library with no dependency and no platform specific code. It takes plain point lists, so it does not depend on any curve code: a caller flattens its contours first.

## Input and output

- A `Polygon` is a list of closed rings of `(f64, f64)` points. All rings of one polygon are filled together under one fill rule, so a hole is a ring inside another.
- A `Target` is a rectangle of pixels: an integer origin, a width and a height.
- `rasterise` returns a `Coverage` plane of `f64` values from 0 to 1. `rasterise_buffer` returns a `Buffer` of 8 bit, 16 bit or 32 bit float samples.
- Bad input is a typed `Error`: a NaN or infinite coordinate, a coordinate above 2^50 in absolute value, a setting outside its range, or a target that is too large. Nothing panics.

## Settings

Every choice that programs differ on is a field of `Config`, and the crate fixes none of them.

| Setting | Choices |
|---|---|
| Method | exact area; one sample at the pixel centre; a fraction of a sample pattern: a regular `n` by `n` grid, `n` samples one per row and column, or an explicit list of offsets |
| Fill rule | non-zero winding; even odd winding |
| Pixel centre | pixel `i` has its centre at coordinate `i + centre`, with `centre` from -1 to 1 (0.5 puts the pixel corners on integers, 0.0 puts the centres on integers) |
| Row direction | rows grow with `y`, or against it |
| Coordinate snapping | none, or rounding of every coordinate to a multiple of `2^-bits` |
| Sample type | 8 bit, 16 bit, 32 bit float |
| Scale | the largest value of the depth, a power of two, or an explicit factor such as 32768 |
| Rounding | nearest with halves up, nearest with halves to even, floor, ceiling |
| Several polygons | maximum, sum clamped to 1, over, replace |
| Arithmetic width | double, or rounded to single after each step |

`Config::canonical()` is a stated default and claims nothing about any other program: exact area, non-zero winding, centre 0.5, rows growing with `y`, 8 bit scaled by 255 with halves up, maximum, double precision, no snapping. `Space` holds one list per setting, and `Space::enumerate()` returns every combination in a fixed order, so a search over conventions can index them. `Config::label()` gives a stable one line name.

## Methods

Exact area follows T. Duff, "Polygon Scan Conversion by Exact Area Calculation" (Raster Imaging and Digital Typography, 1989), the exact area of a pixel square inside the polygon. The crate computes it by cutting each pixel row into slabs between the heights where edges start, end or cross. Inside a slab no two edges cross, so the sweep with the winding number gives trapezoids bounded by straight lines, and the area of a trapezoid in a pixel column has a closed form. This keeps the result exact for self intersecting input under either fill rule, where a plain sum of signed cell areas is not.

Point and sample coverage use the winding number at each sample. The tie rule is the top left rule: a sample exactly on a left or top edge is inside and a sample exactly on a right or bottom edge is outside, where top is the smaller row of the buffer. The winding rules are those of section 5 of the OpenGL Utility Library 1.3 specification.

## Exactness

All arithmetic is IEEE 754 binary64 with the operations in a fixed order and no fused multiply add, so a result is the same bit for bit on every platform. Exact area is exact when the vertices are multiples of `2^-k` for a small `k` and no two edges cross inside one pixel row. In general its error per pixel is of the order of `M * 2^-53` per slab of the row, where `M` is the largest absolute coordinate in pixel space; the tests measure sums over random triangles against the shoelace area to within 1e-11 for coordinates below 16. This is a statement of order, not a proof. Point and sample coverage are exact counts on their sample points, up to the rounding of an edge's x at a sample height.

## Tests

`cargo test -p coverage-raster` runs the crate's tests. They use invented polygons with hand computed expectations (rectangles at integer, half and quarter positions, right triangles, holes under both fill rules, self intersection, either orientation, polygons outside and partly outside the target, empty and degenerate rings, very large coordinates), an independent slow reference (supersampling at 96 by 96 per pixel with a separate winding test, within a stated bound), a pinned bit pattern of one run, and the configuration space.
