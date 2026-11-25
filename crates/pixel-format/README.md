# pixel-format

Generic conversion of pixel samples between integer and floating point depths, with every choice a conversion can make held as explicit configuration. The crate uses only the standard library and the workspace crate `frame-io`. It computes nothing for any particular program, and no default in it is a claim about any program.

## Published definitions followed

- After Effects SDK Guide, pixel formats: 8 bit channels run 0 to 255, 16 bit channels run 0 to 32768 (`PF_MAX_CHAN16`), float channels are nominally 0 to 1 and are not clamped. The channel order in memory is ARGB for After Effects and BGRA for Premiere. Colour is premultiplied by alpha by default.
- IEEE 754-2019 section 4.3: roundTiesToEven, roundTowardZero, roundTowardPositive and roundTowardNegative. Round half up (`floor(x + 0.5)`) is added as a fifth mode.

## What is configuration

Nothing is fixed behind the API. Each of these is a field of `Config`, or an argument of a rule struct:

| Setting | Type | Values |
|---|---|---|
| Rounding | `Rounding` | `HalfUp`, `TiesEven`, `Truncate`, `Floor`, `Ceil` |
| Precision of the arithmetic | `Precision` | `F32`, `F64` |
| Clamp and NaN rule | `Clamp` | `None`, `Range`, `RangeNanZero` |
| Unpremultiply at alpha 0 | `ZeroAlpha` | `ZeroColour`, `KeepColour` |
| Order of the alpha operation and the depth conversion | `PremulOrder` | `ConvertFirst`, `AlphaFirst` |
| Full scale at 16 bit | `u16` | any value, usually 32768 or 65535 |
| Scale, denominator and offset of one conversion | `f64` | any value, in `FloatToInt`, `IntToFloat` and `Rescale` |

`Config::default_choice()` is half up, `F64`, range clamp with NaN to zero, zero colour at alpha 0, convert first, 16 bit full scale 32768. It is a starting point only.

## Public API

`Depth` is `U8`, `U16 { max }` or `F32`. It is reused from `frame-io` for storage: `Depth::storage()` and `Depth::from_storage()` map to and from `frame_io::Depth`, and the bytes per sample come from `frame-io`.

`int_to_float(v, &IntToFloat)` divides by `denom`. `float_to_int(x, max, &FloatToInt)` computes `x * scale + offset`, rounds, then clamps. `Rescale { mul, add, rounding, precision }.apply(v)` is the generic multiply add round rule on a 16 bit value, saturating to 0 and 65535. Full range to host range is the instance `mul = 32768 / 65535`, `add = 0.5`, `Truncate`.

`premultiply_int` and `unpremultiply_int` compute `colour * alpha / max` and `colour * max / alpha` exactly in integers (a 64 bit numerator, no float), so every rounding mode is exact at every tie. `premultiply_f32` and `unpremultiply_f32` are the float forms.

`Config` offers `convert_sample(from, to, bits)`, `convert_pixel(from, to, [r, g, b, a], AlphaOp)`, `canonical()` (one text line), `Config::parse(text)` and `Config::enumerate(&Space)` (each choice once, in a fixed order). `Order` (`Argb`, `Bgra`, `Rgba`) and `WorldLayout { width, height, rowbytes, origin, depth, order }` give pixel access with a row stride in bytes and a byte origin, after the meaning of `PF_EffectWorld`. Samples of 16 and 32 bit are little endian. A float sample travels as its `f32::to_bits`.

## Definitions chosen where the published text is silent

Each is pinned by a test.

- `Precision::F32` narrows every operand to `f32` and rounds every operation result to `f32`; the multiply and the add are two separate operations (no fused multiply add), and the rounding mode runs in `f32`, including the `x + 0.5` step of half up. `Precision::F64` does the same in `f64`.
- `Clamp` acts on the rounded value. `None` casts to `u32` saturating at 0 and `u32::MAX`, so the result may exceed the maximum and NaN becomes 0. `Range` clamps with `min` then `max` (IEEE minNum and maxNum), so NaN becomes the maximum. `RangeNanZero` clamps and maps NaN to 0.
- Integer to integer conversion goes through a float. Float to float is the identity on the bits.
- `convert_pixel` never applies the alpha operation to the alpha sample. With `ConvertFirst` the colour is converted to the target depth and the operation is applied there; with `AlphaFirst` it is applied at the source depth and the result is converted.
- Unpremultiply is capped at the maximum. Alpha of exactly 0 or -0.0 follows `ZeroAlpha`; a subnormal alpha is not zero.
- Rounding of the integer alpha operations is exact over rationals; `Floor` equals `Truncate` there because the operands are not negative.
- `canonical()` is `key=value` pairs joined by `;` in the fixed order rounding, precision, clamp, zero_alpha, order, u16_max. `parse` accepts exactly that form.

## Tests

`cargo test -p pixel-format` runs unit style tests of exact vectors and sweeps: all 256, 32769 and 65536 integer inputs for each rounding and precision (monotone, in range, round trip), the midpoints and the maxima of every rounding mode, NaN with several payloads, infinities, negative zero and subnormals under each clamp, the premultiply tie cases with made up maxima, the enumeration of 240 choices with unique canonical text that parses back, and the layout orders, strides and origins. All data is made up.
