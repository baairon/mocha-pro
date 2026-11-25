# blend-ops

Per pixel compositing operators on RGBA values in `f32` or `f64`, written with `std` only and
no `unsafe` code. Every convention that the published formulas leave open is a field of
`Config`, and `Config::enumerate()` lists the whole configuration space (3072 entries).

## Operators

- The twelve Porter and Duff operators: clear, src, dst, src over, dst over, src in, dst in,
  src out, dst out, src atop, dst atop, xor. Source: T. Porter and T. Duff, "Compositing Digital
  Images", Computer Graphics 18(3), SIGGRAPH 1984, section 3 and its table of fractions.
- The twelve separable blend modes of W3C, "Compositing and Blending Level 1" (Candidate
  Recommendation, 2015; Recommendation text, 2024), section 9.1: normal, multiply, screen,
  overlay, darken, lighten, colour dodge, colour burn, hard light, soft light, difference,
  exclusion. Plain add (`cb + cs`) and subtract (`cb - cs`) are generic extensions.
- The four non separable modes of the same recommendation, section 10: hue, saturation, colour,
  luminosity, with `Lum`, `ClipColor`, `SetLum`, `Sat` and `SetSat` as defined there. When the
  divisor of `ClipColor` is zero the colour becomes `Lum` (the recommendation does not say).
- The general compositing formula of section 5.1 and 9.2 of the recommendation, including the
  backdrop weighted source colour `Cs' = (1 - ab) * Cs + ab * B(Cb, Cs)`.
- Soft light: the W3C function (section 9.1.3, cubic `D(x)` up to 0.25) or the older function
  of ISO 32000-1:2008, section 11.3.5, where `D(x)` is the square root everywhere.

## Configuration

`Config` has: input form and output form (straight or premultiplied); opacity point (on the
source alpha before blending, or on the result, mixing it with the premultiplied backdrop);
clamping (none, output, input and output); soft light variant; the four division guard values of
dodge and burn (each `Zero` or `One`, defaults as in the recommendation); the order in which the
two guard conditions are tested; and whether the backdrop weighted mix is applied. The default is
straight in and out, opacity on the source alpha, no clamping, W3C soft light, W3C guard values
tested backdrop first, backdrop weighted mix on.

## Order of operations

`composite_pixel` converts to straight form (alpha 0 gives colour 0), optionally clamps, applies
the opacity if it is on the source alpha, blends, forms `x = as * Fa`, `y = ab * Fb`,
`co = x * Cs' + y * Cb`, `ao = x + y`, applies the opacity if it is on the result
(`co * opacity + (ab * Cb) * (1 - opacity)`), converts to the output form and clamps.
`composite_buffers` does this for interleaved RGBA slices with an optional one channel matte that
multiplies the source alpha.

## Numeric contract

Only addition, subtraction, multiplication, division and the IEEE square root are used, in the
order written in the source, with no fused multiply add and no library function, so results are
bit identical on every platform with IEEE 754 binary32 and binary64. NaN inputs are not supported.
