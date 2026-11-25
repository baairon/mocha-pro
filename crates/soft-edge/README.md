# soft-edge

`soft-edge` holds soft edge operators for a one channel `f32` coverage buffer. It is written in plain
Rust, uses only the standard library, has no dependency and contains no unsafe code.

## Operators

Buffers are row major slices with an explicit width and height.

- `box_blur` and `box_blur_repeated`: a separable box blur with an integer or fractional radius.
  A radius `r = n + f` (integer part `n`, fraction `f`) gives, by the `EndTaps` rule, `2n + 1` taps
  of weight 1 and two end taps of weight `f` at offsets `n + 1` and `-(n + 1)`, divided by their sum
  `2n + 1 + 2f`. The `Lerp` rule mixes the normalised kernels of radius `n` and `n + 1` with weights
  `1 - f` and `f`.
- `gaussian_blur` and `gaussian_kernel`: a separable Gaussian whose kernel comes from a sigma or from
  a radius (`sigma = radius / radius_to_sigma`). The kernel has half width `ceil(trunc_k * sigma)`, the
  weights are `exp(-x^2 / (2 sigma^2))` and they are normalised to sum to one. The exponential is
  evaluated by this crate, either by a fixed number of Maclaurin terms after halving the argument
  (`Series`) or by one such evaluation followed by a product recurrence (`Recurrence`).
- `squared_distance_transform`: the exact Euclidean distance transform with squared distances in
  integers, by the linear time lower envelope algorithm.
- `signed_distance` and `feather`: a ramp over the signed distance to the zero crossing of a hard mask
  (`sample >= threshold` is inside). The ramp is inward, outward or centred, with a linear,
  smoothstep or cosine profile. The cosine profile `0.5 - 0.5 cos(pi t)` is evaluated as
  `sin^2(pi t / 2)` with a fixed degree 13 polynomial.
- Border rules: `Zero`, `Clamp`, `Mirror` (edge sample repeated once) and `MirrorNoRepeat`.

## Configuration

Every choice that may be varied is a field of `Config`: the border rule, the fractional radius rule,
the truncation factor, the exponential method and its term count, the radius to sigma factor, the
ramp profile and direction, the inside threshold and the edge offset. Defaults are neutral
placeholders. `Config::enumerate` lists the 288 combinations of the discrete choices.

## Determinism

Only addition, subtraction, multiplication, division, square root, floor, ceil and clamp on floats
are used, with no fused multiply add and no library transcendental function, and every sum has a
fixed order. Results are therefore bit identical on Linux, macOS and Windows.

## Sources

- P. F. Felzenszwalb and D. P. Huttenlocher, "Distance Transforms of Sampled Functions", Theory of
  Computing 8(19), 2012, sections 4 and 5 (lower envelope of parabolas, columns then rows).
- R. Szeliski, "Computer Vision: Algorithms and Applications", 2nd edition, 2022, section 3.2
  (linear filtering, box and Gaussian kernels, separable filters).
- M. Abramowitz and I. A. Stegun, "Handbook of Mathematical Functions", 1964, sections 4.2 and 4.3
  (series for the exponential and the sine).
- The Khronos Group, "The OpenGL Shading Language" specification, built-in functions (the
  `smoothstep` definition `t^2 (3 - 2t)`).

## Tests

`cargo test -p soft-edge` runs the unit style tests in `tests/`: hand computed kernels, single lit
pixels, half plane edges, the distance transform against brute force, ramps for each profile and
direction, and determinism across runs.
