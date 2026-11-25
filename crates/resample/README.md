# resample

Generic image resampling of `f32` buffers under an arbitrary inverse mapping. The crate uses
only the standard library, contains no unsafe code, and has no dependency.

## What it does

`resample(src, dst_w, dst_h, map, &config)` fills a `dst_w` by `dst_h` buffer. For each output
pixel the closure `map` gives the source coordinate, and the configured reconstruction filter
combines the source samples around that coordinate. `resample_matrix` takes a 3 by 3 matrix
(homogeneous coordinates, a zero `w` gives a zero pixel) instead of a closure. Buffers are
packed (interleaved) or planar, any channel count. The caller converts sample types.

Coordinates are `f64`, samples are `f32`, accumulation is in `f64`, and the result is rounded to
`f32` once. Filter outputs are not clamped: cubic and Lanczos kernels can overshoot. An output
pixel whose mapped coordinate is not finite or is larger than 2^30 in magnitude is zero.

## Reproducibility

Only addition, subtraction, multiplication, division, comparison and integer conversion are
used, in a fixed order, with no fused multiply add and no libm function, so results are bit
identical on every platform that follows IEEE 754 binary64 and binary32. The sinc function is
implemented in the crate (see below). The tests include a pinned checksum of an output.

## Configuration space

Every choice that a calibration against reference images may need to vary is a field of
`Config`. The defaults are arbitrary starting points and make no claim about any other
software. `ConfigSpace::standard()` lists a representative product of all axes (3,960
combinations) and `ConfigSpace::all()` enumerates it in a fixed order; the axes are public
vectors and can be replaced.

| Field | Values |
| --- | --- |
| `filter` | `Nearest`, `Bilinear`, `Keys { a }`, `Mitchell { b, c }`, `Lanczos { lobes }`, `Box` |
| `tie` | `HalfUp`, `HalfDown`, `HalfEven`: nearest sampling at an exact half way coordinate |
| `center` | `Half` (pixel `i` centred at `i + 0.5`), `Corner` (at `i`) |
| `border` | `Zero` (transparent, all channels), `Clamp`, `Repeat`, `Mirror` (edge repeated, period `2n`), `MirrorNoEdge` (edge not repeated, period `2n - 2`) |
| `alpha`, `alpha_channel` | `Straight` or `Premultiplied` interpolation of colour with the alpha channel at the given index |
| `normalize` | `None`, `Sum` (divide by the sum of all tap weights), `SumInBounds` (divide by the sum of the weights of taps inside the image) |
| `order` | `RowMajor`, `ColumnMajor`: loop order of the two dimensional accumulation |
| `footprint`, `min_extent` | box filter footprint: `Unit`, `Jacobian` (bounding box of the mapped output pixel from forward differences, at least `min_extent`), `Fixed { w, h }` |

Coordinate convention: output pixel `(X, Y)` is passed to the mapping as `(X + o, Y + o)` with
`o = 0.5` for `Half` and `0` for `Corner`; the returned source coordinate is converted back to
a pixel index space by subtracting `o`. Separable kernels use the taps `floor(u) - r + 1` to
`floor(u) + r` with `r` = 1 (bilinear), 2 (cubic families), `lobes` (Lanczos). The box filter
weights each source pixel by the length of its overlap with the footprint interval, divided by
the footprint width, per axis. The accumulation order only changes rounding of the `f64` sums, which is visible after the
single rounding to `f32` only in rare cases (an estimate, not a proof). A tap whose weight is exactly zero is skipped, so a non finite source sample reaches only the pixels that really use it. Interpolating kernels (Keys, Lanczos, bilinear) return exact 1
and 0 at integer offsets so that an identity mapping reproduces the input bit for bit.

## Published definitions and sources

- Keys, R. G., "Cubic Convolution Interpolation for Digital Image Processing", IEEE
  Transactions on Acoustics, Speech, and Signal Processing 29(6), 1981: the cubic convolution
  kernel with free parameter `a` (the kernel definition and its piecewise cubic form).
  `|t| <= 1`: `(a+2)|t|^3 - (a+3)|t|^2 + 1`; `1 < |t| < 2`: `a|t|^3 - 5a|t|^2 + 8a|t| - 4a`.
- Mitchell, D. P. and Netravali, A. N., "Reconstruction Filters in Computer Graphics",
  Computer Graphics (SIGGRAPH 88 Proceedings) 22(4), 1988, section 4: the two parameter cubic
  family `B`, `C`. `|t| < 1`: `((12-9B-6C)|t|^3 + (-18+12B+6C)|t|^2 + (6-2B)) / 6`;
  `1 <= |t| < 2`: `((-B-6C)|t|^3 + (6B+30C)|t|^2 + (-12B-48C)|t| + (8B+24C)) / 6`.
  `B = 0, C = 0.5` equals Keys with `a = -0.5`.
- Duchon, C. E., "Lanczos Filtering in One and Two Dimensions", Journal of Applied
  Meteorology 18(8), 1979, and Turkowski, K., "Filters for Common Resampling Tasks", Graphics
  Gems I, 1990: the Lanczos window `sinc(t) sinc(t / a)` for `|t| < a`, zero outside.
- Normalised sinc, `sin(pi x) / (pi x)`: its Taylor series, evaluated as described next.

### The sinc implementation

`x = n + r` with `n = floor(x + 0.5)`, `r` in `[-0.5, 0.5]`. Then
`sin(pi x) = (-1)^n sin(pi r)`, and `sin(pi r) = pi r P(z)` with `z = (pi r)^2` and
`P(z) = sum over k = 0..8 of (-z)^k / (2k+1)!`, evaluated as the nested form
`1 - z/(2*3) * (1 - z/(4*5) * (... ))` with exactly 9 terms. The remainder of the series on
`|pi r| <= pi/2` is at most `(pi/2)^18 / 19!`, about 2.8e-14 (the measured maximum against a 60 digit oracle is 2.78e-14);
the stated bound of the function is an absolute error of 1e-12, checked in the tests against the
standard library sine on a dense grid. At non zero integers the result is an exact zero.

## Tests

The tests cover: kernel values worked by hand, partition of unity, evenness, the sinc error
bound, every border rule on a three pixel row, identity mappings for each filter, layout and
border, integer and half pixel translations, scaling by two of a small image for each filter,
nearest tie rules, box footprints, premultiplied against straight colour on hand picked pixels,
normalisation rules, error cases, the configuration space, determinism, and an independent slow
reference (a direct sum over taps with separately written kernels and a library sine).

Run with `cargo test -p resample`.
