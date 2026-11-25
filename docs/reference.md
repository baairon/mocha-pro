# Reference

This page documents everything the repository holds: the crates with their file formats, the pairing and the predictor command, the build, the documentation tools and the tests. It lives apart so that the README stays short. The [README](../README.md) covers what the project is and its parity status, and [parity](parity.md) covers the measurement.

The repository holds eighteen Rust crates: `crates/effect-params`, which defines parameter kinds and flags, reads and writes a table of parameter definitions, and pairs a predicted table with an observed one; `crates/shape-model`, the rewrite's model of shape data; `crates/shape-canon`, which writes the model's state as a canonical text dump, reads a case in a plain text form, and builds the command `shape-predict`; `crates/mask-seq`, which defines `mask-1`, the repository's own container form for a sequence of one-channel mask frames, writes and checks such a sequence, and builds the command `mask-validate`; `crates/frame-io`, which reads and writes the raw frame streams of the handoff convention, generates synthetic frames, compares two streams as bit patterns, and builds the commands `frame-gen` and `frame-cmp`; `crates/stage-cli`, which builds `mocha-stage`, a stage program in that convention with one self test stage; seven generic libraries, `crates/pixel-format` (sample depth conversion and alpha premultiplication), `crates/contour-eval` (Bezier and X-spline contours), `crates/coverage-raster` (polygon coverage), `crates/planar-xform` (a 3 by 3 projective transform), `crates/resample` (image resampling), `crates/lens-model` (lens distortion maps) and `crates/soft-edge` (blurs and a feathered ramp over the distance to an edge), each with every choice it could make held as explicit configuration; and five further generic libraries, each with an explicit configuration: `crates/sphere-reorient`, which maps the coordinates of equirectangular, cube map and stereo layout images through a rotation; `crates/mesh-warp`, which warps coordinates by a triangle mesh or a bilinear quad; `crates/blend-ops`, which composites RGBA pixels by the Porter and Duff operators and the blend modes of the W3C; `crates/key-eval`, which evaluates values given by keys at a time; `crates/planar-track`, which tracks a planar region through a sequence of grey level images by direct image alignment. Beside them are three Python tools, `tools/status.py`, `tools/gen_docs.py` and `tools/check_docs.py`, which compute the status of result files, write the generated figures of the documentation and check the pages, each with a test file beside it; the convention's `contracts/` folder, held as twixtor publishes it; the documentation in `docs/`, with the result file schema in `docs/data/result_schema.json`; and the CI workflow, `.github/workflows/ci.yml`.

## Contents

- [Sources of truth](#sources-of-truth)
- [The crates](#the-crates)
  - [Crate map](#crate-map)
  - [The table format](#the-table-format)
  - [The pairing](#the-pairing)
  - [The shape model](#the-shape-model)
  - [The case form](#the-case-form)
  - [The dump form](#the-dump-form)
  - [`shape-predict`](#shape-predict)
  - [The mask sequence form](#the-mask-sequence-form)
  - [`mask-validate`](#mask-validate)
  - [The frame contract](#the-frame-contract)
  - [`frame-gen` and `frame-cmp`](#frame-gen-and-frame-cmp)
  - [`mocha-stage`](#mocha-stage)
  - [The generic crates](#the-generic-crates)
    - [`pixel-format`](#pixel-format)
    - [`contour-eval`](#contour-eval)
    - [`coverage-raster`](#coverage-raster)
    - [`planar-xform`](#planar-xform)
    - [`resample`](#resample)
    - [`lens-model`](#lens-model)
    - [`soft-edge`](#soft-edge)
    - [`sphere-reorient`](#sphere-reorient)
    - [`mesh-warp`](#mesh-warp)
    - [`blend-ops`](#blend-ops)
    - [`key-eval`](#key-eval)
    - [`planar-track`](#planar-track)
- [Build](#build)
- [Documentation tools](#documentation-tools)
  - [Generated figures](#generated-figures)
  - [Page checks](#page-checks)
  - [Status of one result file](#status-of-one-result-file)
- [Tests](#tests)
- [Documentation](#documentation)

## Sources of truth

For scripts: ask the tool or the file, not the prose.

| Question | Ask |
|---|---|
| Where parity stands, per unit | the generated tables of [README: status](../README.md#status) and [parity: status](parity.md#status), from `parity/<unit>/results/summary.json`; `python -I tools/gen_docs.py --check` fails when a page is stale |
| The status of one result file | `python -I tools/status.py FILE...` |
| Every generated figure, with its source | `docs/data/facts.json`, written by `tools/gen_docs.py` |
| The fields of a result file | `docs/data/result_schema.json` |
| The table format | the module comment of `crates/effect-params/src/table.rs`; the loader is right wherever this page disagrees with it |
| How two tables are paired | `pair_tables` and `normalize_name` in `crates/effect-params/src/compare.rs` |
| The rules of the shape model, and which of them are hypotheses | the module comment of `crates/shape-model/src/lib.rs`, and `Contour::points_at` and `Contour::bounds_at` |
| The case form | the module comment of `crates/shape-canon/src/case.rs`; `Case::parse` is right wherever this page disagrees with it |
| The dump form | the module comment of `crates/shape-canon/src/lib.rs` and `render_dump`; the test `predict_exact_bytes` holds one whole dump |
| How a program runs `shape-predict`: its arguments, streams and exit codes | `docs/data/shape_predict_interface.json`, whose exit code rows `crates/shape-canon/tests/interface_contract.rs` checks against the built binary; `crates/shape-canon/src/main.rs` is right wherever the file disagrees with it |
| The mask sequence form: the frame files and the manifest | the module comments of `crates/mask-seq/src/pgm.rs` and `crates/mask-seq/src/manifest.rs`; `Frame::from_pgm` and `Manifest::parse` are right wherever this page disagrees with them |
| How a program runs `mask-validate`: its argument, streams, exit codes and reason words | `docs/data/mask_validate_interface.json`, whose exit code rows and reason words `crates/mask-seq/tests/interface_contract.rs` checks against the built binary and the library; `crates/mask-seq/src/main.rs` and `crates/mask-seq/src/validate.rs` are right wherever the file disagrees with them |

## The crates

`crates/effect-params` holds the types the rewrite uses to describe the parameters of an effect and the comparison that pairs its prediction of a parameter table with the table the original registers. `crates/shape-model` and `crates/shape-canon` are the rewrite's side of the [shape data check](parity.md#the-shape-data-check): the first holds the model of a shape over time, the second turns a case into the text the rewrite predicts for it. `crates/mask-seq` is not part of a check: it defines a container form of the repository's own for a sequence of mask frames, and it stores and checks frames that it is given. `crates/frame-io` and `crates/stage-cli` are the repository's side of the [handoff convention](handoff.md): the first holds the frame contract with a generator and a comparer, the second a stage program with one self test stage, and neither computes anything of the plug-in. The seven generic crates, `crates/pixel-format`, `crates/contour-eval`, `crates/coverage-raster`, `crates/planar-xform`, `crates/resample`, `crates/lens-model` and `crates/soft-edge`, hold conversions, curves, rasterising, geometry, resampling, lens maps and edge operators that are written from published sources, with each choice that the sources leave open as a field of a configuration value; they compute nothing of the plug-in until a configuration has been selected by comparison with the original. `crates/sphere-reorient`, `crates/mesh-warp`, `crates/blend-ops`, `crates/key-eval` and `crates/planar-track` are five further generic crates: each computes from published methods, with every choice those methods leave open made a field of an explicit configuration, and none computes anything of the plug-in until a configuration has been selected by comparison with the original. None of the eighteen has a dependency outside the workspace or any `unsafe` code, none holds a name, a value or a text of the original, and every name and value in their tests is invented, apart from six published NIST test vectors in the SHA-256 tests of `mask-seq`.

### Crate map

Workspace members of `Cargo.toml`: eighteen, version 0.1.0, edition 2021, unpublished. External crates: none; `shape-canon` depends on `shape-model` and `pixel-format` depends on `frame-io`, each by its path in the workspace.

| Member | Builds | Owns | Key modules (`src/`) | Tests |
|---|---|---|---|---|
| `crates/effect-params` | library `effect_params` | Parameter kinds and flags, the table format with its loader and serializer, and the pairing of a predicted table with an observed one | `kind.rs` (`ParamKind`), `flags.rs` (`ParamFlags`), `table.rs` (`Cell`, `ParamDef`, `Table`, `ParseError`, `ValidateError`), `extra.rs` (`Extra`), `compare.rs` (`pair_tables`, `normalize_name`, `Pairing`, `ParamVerdict`, `MatchClass`, `FieldVerdict`, `PairKey`), `lib.rs` (the re-exports) | `tests/edges.rs`, `tests/extra_fields.rs`, `tests/signs_and_gaps.rs`, `tests/table_and_pairing.rs`, `tests/table_v2.rs`, `tests/table_v2_fix.rs` |
| `crates/shape-model` | library `shape_model` | The shape model: control points stored per keyframe, and the points and the bounding rectangle at any query time, by rules labelled as hypotheses | `lib.rs` (`PointData`, `Keyframe`, `Contour`, `Rect`, `ModelError`, `lerp`) | none of its own; the tests of `shape-canon` exercise it |
| `crates/shape-canon` | library `shape_canon`, binary `shape-predict` | The canonical dump text, the hexadecimal float text, the case form and the predictor | `lib.rs` (`DumpWriter`, `render_dump`, `predict`, `UNKNOWN`), `hexfloat.rs` (`fmt_hex`, `parse_hex`), `case.rs` (`Case`, `Meta`, `CaseError`), `main.rs` (`shape-predict`) | `tests/canon.rs`, `tests/cli.rs`, `tests/interface_contract.rs`, `tests/model_hypotheses.rs` |
| `crates/mask-seq` | library `mask_seq`, binary `mask-validate` | The `mask-1` sequence form: SHA-256, the canonical frame file, the manifest, and writing a sequence folder and checking one | `sha256.rs` (`Sha256`, `digest`, `to_hex`, `from_hex`, `K`, `H0`), `pgm.rs` (`Depth`, `Frame`, `Header`, `parse_header`, `FrameError`, `PgmError`), `manifest.rs` (`Manifest`, `ManifestError`, `ManifestErrorKind`, `frame_name`, `FORMAT`, `MANIFEST_NAME`, `NAMING`), `validate.rs` (`validate`, `write_sequence`, `Outcome`, `Reason`, `Invalid`, `Summary`, `SequenceError`), `lib.rs` (the re-exports), `main.rs` (`mask-validate`) | `tests/cli.rs`, `tests/interface_contract.rs`, `tests/manifest.rs`, `tests/pgm.rs`, `tests/sha256.rs`, `tests/validate.rs`, with shared helpers in `tests/common/mod.rs` |
| `crates/frame-io` | library `frame_io`, binaries `frame-gen` and `frame-cmp` | The raw frame contract, the synthetic frame generator and the comparer of two streams | `geometry.rs` (`Depth`, `Geometry`, `Rate`, `MAX_FRAME_BYTES`), `io.rs` (`FrameReader`, `FrameWriter`), `pattern.rs` (`Pattern`, `SplitMix64`), `compare.rs` (`compare`), `sample.rs`, `error.rs` (`GeometryError`, `FrameError`), `cli.rs`, `bin/frame_gen.rs`, `bin/frame_cmp.rs` | `tests/geometry.rs`, `io.rs`, `pattern.rs`, `sample.rs`, `compare.rs`, `cli.rs`, `interface_contract.rs` |
| `crates/stage-cli` | library `stage_cli`, binary `mocha-stage` | The stage program of the handoff convention: the manifest, the argument rules, the self test stage and the stage exit codes | `stage.rs` (the manifest and `selftest-copy`), `args.rs`, `run.rs`, `frame.rs`, `error.rs`, `main.rs` | unit tests in the modules, `tests/cli.rs` on the built binary |
| `crates/pixel-format` | library `pixel_format` | Conversion of one sample between 8 bit, 16 bit and 32 bit float depths, exact integer premultiplication, and the layout of a pixel buffer with a row stride | `mode.rs` (`Depth`, `Rounding`, `Precision`, `Clamp`, `PremulOrder`), `convert.rs` (`int_to_float`, `float_to_int`, `IntToFloat`, `FloatToInt`, `Rescale`), `alpha.rs` (`ZeroAlpha`, `premultiply_int`, `unpremultiply_int`, `premultiply_f32`, `unpremultiply_f32`), `config.rs` (`Config`, `Space`, `AlphaOp`, `ParseError`), `layout.rs` (`Order`, `WorldLayout`, `LayoutError`), `lib.rs` (the re-exports) | `tests/alpha.rs`, `tests/config.rs`, `tests/convert.rs`, `tests/layout.rs` |
| `crates/contour-eval` | library `contour_eval` | The evaluation of a planar contour of cubic Bezier spans or X-spline segments at a parameter, and its flattening to a polyline | `lib.rs` (`Contour`, `Kind`, `Point`, `Config`, `ContourError`, `eval`, `bezier_split`), `bezier.rs` (the Bezier forms), `xspline.rs` (`f_blend`, `g_blend`, `h_blend`, `xspline_weights`), `flatten.rs` (`flatten`, `Flatten`), `real.rs` (the arithmetic at the two precisions) | `tests/audit_segment.rs`, `tests/bezier.rs`, `tests/config.rs`, `tests/errors.rs`, `tests/flatten.rs`, `tests/vectors.rs`, `tests/xspline.rs`, and unit tests in `src/flatten.rs` |
| `crates/coverage-raster` | library `coverage_raster` | The coverage of closed polygons in a target rectangle, by exact area or by sample points, with the quantising of the result | `config.rs` (`Config`, `Space`, `Method`, `Pattern`, `FillRule`, `Combine`, `Quantise`, `Polygon`, `Target`), `edges.rs` (the edge list), `exact.rs` (exact area), `sample.rs` (sample points and `pattern_offsets`), `quant.rs` (`Buffer`, `code`), `error.rs` (`Error`), `lib.rs` (`rasterise`, `rasterise_buffer`, `Coverage`) | `tests/audit_regress.rs`, `tests/combine_quant.rs`, `tests/edge_cases.rs`, `tests/exact_area.rs`, `tests/methods.rs`, `tests/reference.rs`, `tests/sampling.rs`, with shared helpers in `tests/common/`, and unit tests in `src/exact.rs` |
| `crates/planar-xform` | library `planar_xform` | A 3 by 3 planar projective transform in double precision: construction, composition, inverse, application with a divisor guard, the four point solution, and the unit square to pixel conversion | `homography.rs` (`Homography`, `Error`), `pixel.rs` (`PixelSpace`), `lib.rs` (`Config`, `Convention`, `DivisorGuard`, `PixelCenter`, `YAxis`, `Point`, `Rect`) | `tests/audit_fix.rs`, `tests/core.rs`, `tests/pixel.rs`, `tests/solve.rs` |
| `crates/resample` | library `resample` | The resampling of a planar or packed buffer of 32 bit floats under an inverse mapping, with the kernels and the border rules | `config.rs` (`Config`, `ConfigSpace`, `Filter`, `TieRule`, `PixelCenter`, `AlphaMode`, `Normalize`, `AccumOrder`, `Footprint`, `Error`), `engine.rs` (`resample`, `resample_matrix`, `Image`, `Layout`, `Matrix3`), `kernel.rs` (`sinc`, `bilinear`, `keys`, `mitchell`, `lanczos`), `border.rs` (`Border`, `wrap`) | `tests/alpha_config.rs`, `tests/audit_round1.rs`, `tests/borders.rs`, `tests/filters.rs`, `tests/kernels.rs`, `tests/reference.rs`, with shared helpers in `tests/common/` |
| `crates/lens-model` | library `lens_model` | Lens distortion models as coordinate maps in double precision, their inverses, and the maps and bounds a resampler needs | `model.rs` (`Model`, `InverseMethod`, `LensError`), `config.rs` (`Config`, `ConfigSpace`, `ModelMaps`, `Normalisation`), `resample.rs` (`Direction`, `Bounds`), `lib.rs` (the re-exports) | `tests/audit_gaps.rs`, `tests/config.rs`, `tests/model.rs`, `tests/resample.rs` |
| `crates/soft-edge` | library `soft_edge` | Soft edge operators on a one channel buffer of 32 bit floats: box and Gaussian blurs, the exact squared distance transform, and a ramp over the signed distance to the edge of a hard mask | `kernel.rs` (`box_blur`, `box_blur_repeated`, `box_kernel`, `gaussian_blur`, `gaussian_kernel`, `GaussSpec`, `convolve_separable`, `exp_neg`), `edt.rs` (`squared_distance_transform`), `feather.rs` (`signed_distance`, `feather`, `ramp`), `border.rs` (`Border`, `resolve_index`), `config.rs` (`Config`, `Profile`, `Direction`, `FracRule`, `ExpMethod`), `lib.rs` (`Error`) | `tests/audit_regress.rs`, `tests/blur.rs`, `tests/border_cfg.rs`, `tests/distance.rs`, `tests/feather.rs`, `tests/kernels.rs` |
| `crates/sphere-reorient` | library `sphere_reorient` | The coordinate map of equirectangular, cube map and stereo layout images through a rotation, with its own trigonometric functions | `config.rs` (`Config` and its field types), `equirect.rs` (`Size`, `LonLat`, `PixelPos`, the pixel, angle and direction maps, `inverse_map`), `rotation.rs` (`Angle`, `Mat3`), `cube.rs` (`CubeConfig`, `CubeGrid`, `Face`), `stereo.rs` (`StereoLayout`, `Rect`), `trig.rs` (sine, cosine, arc tangent and arc sine) | `tests/audit_reorient.rs`, `tests/equirect.rs`, `tests/layouts.rs`, `tests/rotation.rs`, `tests/trig.rs` |
| `crates/mesh-warp` | library `mesh_warp` | The coordinate map of a triangle mesh and of a bilinear quad patch, with stated point location, tie and outside rules | `lib.rs` (`Vec2`, `Vertex`, the re-exports), `warp.rs` (`Mesh`, `MeshError`, `Config`, `TieRule`, `Outside`, `Winding`, `BaryForm`, `GridRes`, `barycentric`, `Locator`, `Warp`, `grid_mesh`, `Diagonal`, `config_space`), `quad.rs` (`QuadPatch`, `QuadConfig`, `RootChoice`, `quad_config_space`) | `tests/audit.rs`, `tests/quad.rs`, `tests/warp.rs` |
| `crates/blend-ops` | library `blend_ops` | Per pixel compositing of RGBA values in `f32` or `f64`: the Porter and Duff operators, the separable and non separable blend modes, and the configuration of every open convention | `real.rs` (`Real`), `modes.rs` (`PorterDuff`, the enum of blend modes, `blend_channel`, `blend_rgb`, `lum`, `sat`, `set_lum`, `set_sat`, `clip_color`), `config.rs` (`Config`, `Form`, `OpacityPoint`, `Clamp`, `SoftLight`, `Guard`, `GuardOrder`, `BackdropMix`), `buffer.rs` (`composite_buffers`, `BufferError`), `lib.rs` (`Rgba`, `composite_pixel`, the re-exports) | `tests/config.rs`, `tests/corners.rs`, `tests/extra.rs`, `tests/guards.rs`, `tests/pd.rs`, `tests/reference.rs` |
| `crates/key-eval` | library `key_eval` | The value of a list of keys at a time: hold, linear, Hermite and Bezier keys, fixed step curve solvers and the rules outside the keyed range | `key.rs` (`Key`, `KeyList`, `KeyError`), `config.rs` (`Config`, `Solver` and the other field types, `Axes`), `eval.rs` (`floor`, `round_half_up`, `round_half_even`), `solve.rs` (`cbrt`, `solve`) | `tests/bezier.rs`, `tests/config.rs`, `tests/outside.rs`, `tests/scalar.rs`, `tests/solve.rs` |
| `crates/planar-track` | library `planar_track` | Tracking of a planar region through a sequence of grey level images by direct image alignment, with a correction interface | `image.rs`, `pyramid.rs`, `model.rs` (`Dof`, the five motion models, and `Pose`), `region.rs`, `edt.rs`, `linalg.rs`, `align.rs`, `ncc.rs`, `config.rs` (`Config`, `ConfigSpace` and the field types), `tracker.rs` (the tracker type, `FrameResult`), `error.rs` | `tests/audit_fixes.rs`, `tests/audit_kill.rs`, `tests/audit_steps.rs`, `tests/primitives.rs`, `tests/steps.rs`, `tests/tracking.rs`, with shared helpers in `tests/common/mod.rs` |

The types of `effect-params`:

- **`ParamKind`** is a plain enum of parameter kinds. Each kind has a lower-case text name, the name the table format uses, and `ParamKind::parse` maps the name back.
- **`ParamFlags`** is a set of flag bits with this crate's own names, compared as bits: `contains` tests a subset, `union` joins two sets, and `diff` gives the bits set in exactly one of two sets.
- **`Cell`** is a table value that may be unknown: `Unknown`, `Int`, `Float` or `None`, the last being the default of a layer parameter that names no layer (text `none`, version 2 only). Unknown is never the same as zero. `bits_eq` compares two cells by identity: integers by value, floats by their bit patterns, so -0 differs from 0 and NaN payloads count, and an integer never equals a float.
- **`ParamDef`** is one parameter: its name, kind, default, minimum and maximum (each a `Cell`), its flags, its extra cell (`Extra`), its host id and its two raw host words, `host_flags` and `host_ui_flags`. The flags, the id and the host words are `None` when unknown.

### The table format

A table is a UTF-8 text file:

- Blank lines and lines that start with `#` are ignored.
- The first other line is the magic `table v1`. This section describes version 1; [version 2](#the-table-format-version-2) follows below.
- Every row after it has six cells separated by tabs: `name`, `kind`, `default`, `min`, `max`, `flags`, and may have a seventh, `extra`, the extra cell of the kind. A row with another number of cells, an empty name or a kind the crate does not know is an error.
- A value cell is `?` (unknown), `i:` and a decimal integer, `f:` and a decimal float, or `b:` and the float's 64-bit pattern as exactly 16 hexadecimal digits.
- The flags cell is `?` (unknown) or `0x` and the bits in hexadecimal.

`Table::parse` reads a table and reports the first error with its line. `Table::serialize` writes floats as `b:` cells only, so the values of a table keep every bit when it is serialized and read back, -0 and NaN payloads included; `f:` is accepted on input only. The text itself is sure to come back unchanged only when the serializer wrote it: a hand-written table loses its comments and blank lines, its `f:` cells come back as `b:` cells, and its integers and flags are written in the serializer's own form.

An example with two invented parameters, tabs between the cells:

```text
# invented example
table v1
Param A	float	b:3fe0000000000000	f:0	f:2	0x10
Param B	popup	i:1	?	?	?
```

The crate test `example_parses` reads this page when the tests are built, takes the first fenced block whose first line other than comments and blank lines is the magic line, and checks that `Table::parse` accepts it with its two rows, `Param A` and `Param B`. An edit of the example that the loader would refuse fails `cargo test`.

### The table format, version 2

`Table::serialize` writes version 2; `Table::parse` reads both versions, and the magic line (`table v1` or `table v2`) says which. A version 1 file loads unchanged, with the new fields unknown. A version 2 row has exactly ten cells separated by tabs: `id`, `name`, `kind`, `default`, `min`, `max`, `flags`, `host_flags`, `host_ui_flags`, `extra`.

- `id` is `?` (unknown) or `i:` and a decimal integer, the host's parameter id. A known id appears once in a table.
- `name` has the escapes `\\`, `\t`, `\n` and `\r`. It may be empty, but only when the id is known.
- `kind` is any kind name, including `arbitrary`, `nodata`, `group` and `group_end`.
- The default of a `layer` row may be `none`, the layer default that names no layer: it is neither unknown nor the index 0. `none` is refused in every other cell.
- `flags` is the crate's own five bits. `host_flags` and `host_ui_flags` are the raw words of the host registration, each `?` or `0x` and one to eight hexadecimal digits, kept apart from `flags`.
- `extra` is `?` or the extra cell of the kind (popup choices, slider precision and display flags, point pair, button label, arbitrary data bytes).

When both tables carry a known id for a row, the pairing keys the row by that id and reports `PairKey::Id`; otherwise it falls back to the name, exact and then normalized, reports `PairKey::Name`, and never pairs rows whose known ids differ or rows with an empty name. Each matched pair also gets a verdict for the id, the name, the extra cell, `host_flags` and `host_ui_flags`.

### The pairing

`pair_tables(predicted, observed)` matches each predicted row with at most one observed row, in three passes over the predicted table in its own order:

1. **By id.** The first unclaimed observed row that carries the same known id (match class `ById`; rows without a known id skip this pass).
2. **Exact.** The first observed row with the same name, byte for byte, that no other row has claimed and whose known id does not differ.
3. **Normalized.** For a row still without a partner, the first unclaimed observed row whose name is equal after `normalize_name`: lower-cased, outer whitespace trimmed, inner whitespace runs collapsed to one space, and a trailing colon or `...` dropped, repeatedly.
4. Otherwise the row is **unmatched**.

An observed row is claimed once, so a normalized match never takes a row that another predicted row matches exactly. For every matched pair the kind, the default, the minimum, the maximum, the flags, the extra cell and the two host words each get one verdict, and so do the id and the name:

| Verdict | Means |
|---|---|
| `Same` | both known and identical as bits |
| `Differ` | both known and different |
| `Unscored` | the prediction is unknown, so there is nothing to score |
| `ObservedUnknown` | the prediction is known and the observation is not |

When both flag sets are known, `flag_diff` holds the bits set in exactly one of them. The `Pairing` keeps one `ParamVerdict` per predicted row and the names of the observed rows no prediction claimed; `count(MatchClass)` counts the rows of one class. The rule that the pairing passes only when nothing stays unmatched on either side belongs to the method ([parity: the interface pairing](parity.md#the-interface-pairing-and-calibration)); the crate reports the classes and leaves the verdict to the caller.

### The shape model

`crates/shape-model` describes one shape as a `Contour`: a list of `Keyframe`s, each a key time with its control points, the same number of points in every key. A point (`PointData`) has two flags, `cusp` and `enabled`; its anchor position `x` and `y`; two optional numbers, `edge_w` and `edge_r`; and two tangents, `tan_in` and `tan_out`, each a pair of numbers. Every number is a 64-bit float. `Contour::new` refuses a list with no key (`NoKeys`), a key time that is not finite (`NonFiniteTime`), keys with different numbers of points (`PointCountMismatch`) and key times that do not strictly increase (`KeysNotIncreasing`).

`Contour::points_at(t)` gives the points at a query time, and `Contour::bounds_at(t)` the bounding rectangle of their anchor positions, as left, top, width and height (`Rect`): left and top are the smallest `x` and `y`, width and height the distances to the largest. Nothing in the repository has measured how the original treats shape data, so the rules these two follow are hypotheses, labelled INFERRED in the module comment:

- at a key time, the points are the stored ones;
- between two keys, each number is blended linearly in time, as `lerp(a, b, u)` = `a + (b - a) * u`, with the weight `u` measured from the earlier key;
- the two flags, and an optional number that either key lacks, keep the earlier key's value between keys;
- before the first key and after the last, the nearest key's points hold;
- the rectangle spans the anchor positions of all points, and of equal extreme coordinates it keeps the first point's value, which decides the sign of a zero extreme.

Two more choices belong to the crate and are not hypotheses about the original: a NaN query time gives the first key's points, and a contour whose keys hold no point has a rectangle of four zeros. [Internals: the shape crates](internals/shape-crates.md#the-hypotheses) gives each rule with the code that applies it and the test that pins it.

### The case form

A case is a UTF-8 text file with one `key=value` record per line. It describes one contour and the times at which to query it:

| Record | Means |
|---|---|
| `case=1` | required; a case without it is refused |
| `bounds=0` or `bounds=1` | optional, default 0: with 1 the dump gives the rectangle at every query time |
| `meta=0` or `meta=1` | optional, default 0: with 1 the dump gives the four meta values, otherwise its meta section stays empty |
| `time=F` | a query time; repeated, in the order the dump lists them |
| `key=F` | starts a keyframe at time F |
| `pt=cusp,enabled,x,y,edge_w,edge_r,bx,by,fx,fy` | adds a point to the last keyframe: the two flags as 0 or 1, then `x` and `y`, then `edge_w` and `edge_r`, each a number or `none`, then `bx,by`, the tangent `tan_in`, and `fx,fy`, the tangent `tan_out` |
| `meta.length=TEXT`, `meta.rate=TEXT`, `meta.span_start=TEXT`, `meta.span_end=TEXT` | optional: the four meta values, taken as text; each one not given is `UNKNOWN` |

Every number, F included, is written in the hexadecimal float text of [the dump form](#the-dump-form) and read by `parse_hex`. Empty lines are skipped; there are no comment lines. `Case::parse` refuses a line without `=`, an unknown key (a `case` record other than `case=1` among them), a flag other than 0 or 1, a number that `parse_hex` refuses, a `pt` record with other than ten fields, and a `pt` record before the first `key`, each with its line number; a case without `case=1` is reported at line 0. The keys then go through `Contour::new`, so a case with no key, a key time that is not finite, keys with different numbers of points or key times that do not strictly increase are refused as well.

An invented case, the one of the test `predict_exact_bytes`: one key at time 0 with one point, queried at times 0 and 1, with the rectangle and the meta values.

```text
case=1
bounds=1
meta=1
time=0x0.0p+0
time=0x1.0000000000000p+0
key=0x0.0p+0
pt=1,1,0x1.0000000000000p+0,-0x0.0p+0,none,0x1.0000000000000p+1,0x0.0p+0,0x0.0p+0,0x0.0p+0,0x0.0p+0
meta.length=1
```

### The dump form

The dump is what the rewrite predicts for a case, written by `render_dump`: UTF-8 text with LF line ends, one `key=value` record per line, in a fixed order.

1. `format=shape-1`, the name of this form, then `section=shape` and `contours=N`, the number of contours, which is 1 for every case, since a case holds one contour.
2. For each contour `cI`, counted from 0: `cI.points`, the number of points; then for each query time `tK`, counted from 0 in the case's order, the record `cI.tK.time`, the eight records of each point `pJ` in the order `cusp`, `enabled`, `x`, `y`, `edge_w`, `edge_r`, `tan_in`, `tan_out` (named `cI.tK.pJ.cusp` and so on), and, when the case asks for it, `cI.tK.bounds`.
3. `section=meta`, then, when the case asks for them, `project.length`, `project.rate`, `layer.span_start` and `layer.span_end`, each the text the case gives or `UNKNOWN`.

Flags are written 0 or 1, counts in decimal, a missing optional number as `none`, a tangent as its two numbers joined by a comma, and the rectangle as left, top, width and height joined by commas. A number is written by `fmt_hex` in one canonical hexadecimal text: a `-` when the sign bit is set, `0x`, the leading digit (1, or 0 for a subnormal), a point, the 52 fraction bits as 13 lower-case hexadecimal digits, `p`, and the power of two in decimal with its sign and no leading zero (-1022 for a subnormal). Zero is `0x0.0p+0` and negative zero `-0x0.0p+0`; the other special values are `inf`, `-inf` and `nan`. Every value other than NaN has exactly one text and comes back from it with all its bits (the test `hex_round_trips_bits`), so two dumps with the same text hold the same bits, and -0 and 0 differ. A NaN is written `nan` with no sign and no payload, so the dump tells NaNs apart only as a class.

`parse_hex` reads this text back. It refuses other spellings, among them a short fraction, upper-case letters, an exponent without its sign or with a leading zero, and an exponent out of range (the test `hex_rejects_noncanonical`), with one exception: an exponent written `-0` is read like `+0` (the test `hex_reads_minus_zero_exponent`). `fmt_hex` never writes that spelling, so a dump never holds it.

The dump of the case above, as `predict_exact_bytes` checks it:

```text
format=shape-1
section=shape
contours=1
c0.points=1
c0.t0.time=0x0.0p+0
c0.t0.p0.cusp=1
c0.t0.p0.enabled=1
c0.t0.p0.x=0x1.0000000000000p+0
c0.t0.p0.y=-0x0.0p+0
c0.t0.p0.edge_w=none
c0.t0.p0.edge_r=0x1.0000000000000p+1
c0.t0.p0.tan_in=0x0.0p+0,0x0.0p+0
c0.t0.p0.tan_out=0x0.0p+0,0x0.0p+0
c0.t0.bounds=0x1.0000000000000p+0,-0x0.0p+0,0x0.0p+0,0x0.0p+0
c0.t1.time=0x1.0000000000000p+0
c0.t1.p0.cusp=1
c0.t1.p0.enabled=1
c0.t1.p0.x=0x1.0000000000000p+0
c0.t1.p0.y=-0x0.0p+0
c0.t1.p0.edge_w=none
c0.t1.p0.edge_r=0x1.0000000000000p+1
c0.t1.p0.tan_in=0x0.0p+0,0x0.0p+0
c0.t1.p0.tan_out=0x0.0p+0,0x0.0p+0
c0.t1.bounds=0x1.0000000000000p+0,-0x0.0p+0,0x0.0p+0,0x0.0p+0
section=meta
project.length=1
project.rate=UNKNOWN
layer.span_start=UNKNOWN
layer.span_end=UNKNOWN
```

With one key, every query time holds that key's point, so the two times give the same records. The rectangle of a single point has its position as left and top, and zero width and height.

### `shape-predict`

`shape-predict CASE` reads one case file and writes its predicted dump to standard output; arguments after the first are ignored. A dump that is written leaves standard error empty, apart from the progress lines of cargo when it runs under `cargo run`. From the repository root:

```sh
cargo run --locked --release -p shape-canon --bin shape-predict -- CASE
```

After `cargo build --release` the binary is also at `target/release/shape-predict`.

| Exit code | Means |
|---|---|
| 0 | the dump was written |
| 1 | the case is refused (`error: line N: REASON`, or `error: ` and the reason `Contour::new` gives, on stderr), or standard output cannot be written |
| 2 | no file named (the usage line on stderr), or a file that cannot be read as UTF-8 text (`read failed: REASON` on stderr) |

For programs that run `shape-predict`, `docs/data/shape_predict_interface.json` states this interface in a machine-readable form: the arguments, what the command does with each standard stream, the dump form name `shape-1`, six exit code rows, and one exit path that no test pins. Each row is a kind of run with its exit code and what it writes to standard error: a valid case (0), a bad line and a contour that `Contour::new` refuses (1), and no file named, a file that does not exist and a file that is not UTF-8 (2). The path apart is the exit code 1 for a standard output that cannot be written, listed under `unpinned_paths`: part of the dump may have been written by then, and no test exercises it. The test `crates/shape-canon/tests/interface_contract.rs` runs the built binary as each row describes and checks its exit code, its standard error line and whether standard output is empty against the file ([tests](#tests)), so a change to the exit codes or the standard error forms of the binary fails `cargo test` until the file follows it. Of the file, the test reads only the exit code rows and the dump form name.

### The mask sequence form

`mask-1` is a container form of this repository's own, defined by `crates/mask-seq`, for a sequence of mask frames. A sequence is one folder that holds a manifest, `manifest.txt`, and the frame files that the manifest lists. Each frame is an image of one channel whose sample value `s` means a coverage of `s / maxval`: 0 for none and `maxval` for full, with no transfer curve. The frames are numbered by an index, and the indexes of a sequence run without a gap from `first` to `first + frames - 1`. Every frame of a sequence has the same width, height and depth. The folder may hold other files, which are ignored, but a name that begins with `frame_` and ends with `.pgm` must be the file of an index of the sequence ([`mask-validate`](#mask-validate)).

**Frame files.** A frame is stored as a binary graymap, magic `P5`, as the Netpbm PGM format description defines it. `mask-1` admits one spelling of its header only, so that equal samples always give equal file bytes, and so equal hashes:

```text
P5 LF width SP height LF maxval LF samples
```

- `P5` is the two bytes 0x50 0x35, LF is the byte 0x0A and SP the byte 0x20.
- `width` and `height` are decimal, 1 to 5 digits with the first one not `0`, each from 1 to 65535.
- `maxval` is exactly `255` for depth 8, or exactly `65535` for depth 16.
- The samples, `width * height` of them, run row by row from the top and each row from the left: one byte per sample at depth 8, and two bytes per sample at depth 16, the most significant first. Every value from 0 to `maxval` is a valid sample.
- The file ends right after the last sample.

`Frame::from_pgm` reads exactly this form, through `parse_header`, and refuses any other header as `PgmError::Header`: another magic (`P2`, `P6`, or `p5` in lower case), a comment, any other whitespace (a tab, a CR, two spaces, a space before LF, LF in place of SP), a leading `0`, a sign, a sixth digit, a width or height of 0 or above 65535, a `maxval` other than `255` and `65535` (`254`, `256` or `0255`, for example), and a file that ends before the LF after `maxval`. After a good header, a file shorter than the header and its samples is `PgmError::Truncated`, and a longer one `PgmError::Trailing`. `Frame::to_pgm` writes this form only, so a frame comes back unchanged from its bytes, and the bytes of every file the reader accepts come back unchanged from its frame.

**The manifest.** The manifest is ASCII text with LF line ends: one `key=value` record per line, in a fixed order, every line ended by LF, with no empty line, no comment, no CR and no byte outside 0x20 to 0x7E other than LF. Its records, in order:

| Line | Key | Value |
|---|---|---|
| 1 | `format` | `mask-1`, the name of the form |
| 2 | `frames` | the number of frames, 1 to 1000000 |
| 3 | `first` | the first index, 0 to 999999, with `first + frames - 1` at most 999999 |
| 4 | `width` | the width of every frame, 1 to 65535 |
| 5 | `height` | the height of every frame, 1 to 65535 |
| 6 | `depth` | the bits per sample, 8 or 16 |
| 7 | `channels` | 1 |
| 8 | `naming` | `frame_######.pgm` |
| 9 to 8 + `frames` | `frame.I.sha256`, one line for each index I from `first` to `first + frames - 1`, in that order | the SHA-256 of the frame file of index I, as 64 lower-case hexadecimal digits |

A number is written in decimal: `0`, or a digit from 1 to 9 followed by any digits. The frame of index I is the file `frame_`, then I in decimal, zero padded to six digits, then `.pgm`; the six `#` of the `naming` value stand for those digits. Index 0 is the file `frame_000000.pgm`, and index 999999 the file `frame_999999.pgm`.

`Manifest::parse` reads the records in order; the first fault wins and names its line, and a fault in one line is found before any line after it is read. A line that breaks the form is a syntax fault: a missing line or a missing final LF, a line without `=`, a key other than the one that line must hold (so an unknown, repeated or misplaced key), a value outside its grammar (a sign, a leading zero, a space, an upper-case hexadecimal digit, a hash of another length), a byte outside the range above, or a line after the last frame line. A `format` value other than `mask-1` is a format fault. A value in its grammar but outside its range is a value fault, a `naming` value other than `frame_######.pgm` included; a number too large for 64 bits counts as out of range. `Manifest::to_bytes` writes the records in this form only, with nothing else.

The hash of a frame is the SHA-256 of the whole frame file as it is stored, header and samples, written as the 64 lower-case hexadecimal digits that `sha256sum` of GNU coreutils prints for the file. Since the header has one spelling only, equal samples give an equal file and an equal hash. SHA-256 is a fixed public standard, [FIPS 180-4](https://csrc.nist.gov/pubs/fips/180-4/upd1/final), so any tool that prints SHA-256 recomputes the value without this crate. The crate computes it in `crates/mask-seq/src/sha256.rs`, written from the standard, with no dependency: the round constants and the initial hash value are computed when the crate is compiled, from their definitions in the standard, and the tests check the result against published test vectors of the [NIST Cryptographic Algorithm Validation Program](https://csrc.nist.gov/projects/cryptographic-algorithm-validation-program/secure-hashing) ([tests](#tests)).

The manifest below belongs to a sequence of two invented frames numbered from 0, each 3 samples wide and 2 high at depth 8: the first holds 0, 128 and 255 in its top row and 255, 128 and 0 below, the second 0, 0 and 0 above and 255, 255 and 255 below. The test `example_exact_bytes` builds the same text from those two frames, and `reference_example_is_canonical` reads the block from this page and checks that `Manifest::parse` accepts it and that it is the same text, byte for byte, so an edit of the block fails `cargo test`.

```text
format=mask-1
frames=2
first=0
width=3
height=2
depth=8
channels=1
naming=frame_######.pgm
frame.0.sha256=17e174a819c06c75145d49ea023065f4c4216cd19e8a48b0a0b4942f94e6f74d
frame.1.sha256=20f63f9a860eb8170b76e74883f4db9312bdec2d35f41c1236d93158d5a53d8f
```

### `mask-validate`

`mask-validate SEQUENCE` checks one mask sequence and reports the first fault it finds. `SEQUENCE` is a folder that holds `manifest.txt`, or the path of a manifest file of any name, in which case the frame files are read from the folder that holds that file. The command takes exactly one argument and recognises no option, so `--help` is read as a path, and it never reads standard input. From the repository root:

```sh
cargo run --locked --release -p mask-seq --bin mask-validate -- SEQUENCE
```

After `cargo build --release` the binary is also at `target/release/mask-validate`.

A valid sequence gives one line on standard output, `ok format=mask-1 frames=N first=F width=W height=H depth=D` with the values of its manifest, and leaves standard error empty, apart from the progress lines of cargo when it runs under `cargo run`. An invalid one leaves standard output empty and gives one line on standard error, `error: WORD: DETAIL`, where WORD is the reason word of the first fault and DETAIL is `manifest.txt` for `manifest_missing`, `line N` for a fault inside the manifest (`line 1` for `manifest_format`), the file name for a fault of a frame, and `.` for a folder that cannot be listed. A detail never holds a path, so the report does not depend on where the folder is. With no argument, or with more than one, the command writes the usage line `usage: mask-validate <folder|manifest.txt>` on standard error.

| Exit code | Means |
|---|---|
| 0 | the sequence is valid |
| 1 | the sequence is invalid (one `error:` line on stderr), or standard output cannot be written |
| 2 | no argument or more than one (the usage line on stderr), or an argument that cannot be read: a path that does not exist, or a manifest that exists but cannot be read (`read failed: REASON` on stderr, with the reason as the system gives it) |

The checks run in this order, and the first fault wins:

1. A folder is read as the folder of the sequence, with its `manifest.txt` as the manifest; a folder without that file is `manifest_missing`. Any other path is read as the manifest, and the folder that holds it as the folder of the sequence.
2. The manifest, line by line, as [the mask sequence form](#the-mask-sequence-form) gives it.
3. Each frame file, in index order: it is present and a regular file, it can be read, it begins with the canonical header, its width and height and then its depth are those of the manifest, it is neither shorter nor longer than its header and samples, and its SHA-256 is the one the manifest gives for its index.
4. The folder listing: every file name that begins with `frame_` and ends with `.pgm` but is not the file of an index of the sequence is extra, and the smallest of them in byte order is reported. Names are compared byte for byte, so a name such as `frame_000009.PGM` is ignored like any other file, and a name that is not valid UTF-8 is skipped.

Steps 3 and 4 together mean that a valid sequence has no gap on disk and no frame file beyond its indexes. Each frame file is read into memory whole; the largest frame the form allows, 65535 by 65535 samples at depth 16, takes about 8.6 GB.

Every reason word comes with the exit code 1:

| Reason word | Detail | When |
|---|---|---|
| `manifest_missing` | `manifest.txt` | the folder named holds no file `manifest.txt` |
| `manifest_syntax` | `line N` | a line of the manifest breaks the form |
| `manifest_format` | `line 1` | the `format` value is not `mask-1` |
| `manifest_value` | `line N` | a value of the manifest is in its grammar but out of its range |
| `frame_missing` | the file name | a listed frame file is absent or not a regular file |
| `frame_header` | the file name | a frame file does not begin with the canonical header |
| `frame_size` | the file name | the width or height of a frame differs from the manifest's |
| `frame_depth` | the file name | the `maxval` of a frame differs from the manifest's depth |
| `frame_truncated` | the file name | a frame file is shorter than its header and samples |
| `frame_trailing` | the file name | a frame file has bytes after its samples |
| `frame_hash` | the file name | the SHA-256 of a frame file differs from the manifest's |
| `frame_extra` | the file name | the folder holds a file name of the frame form that the manifest does not list |
| `frame_read` | the file name, or `.` | a present frame file, or the folder listing, cannot be read |

For programs that run `mask-validate`, `docs/data/mask_validate_interface.json` states this interface in a machine-readable form: the argument, what the command does with each standard stream, the names `mask-1`, `manifest.txt` and `frame_######.pgm`, the frame form and the hash, the forms of the two report lines, the order of the checks, the 13 reason words, 17 exit code rows, and three exit paths that no test pins. Each row names one kind of run with its exit code, the reason word it reports and what it writes to each output stream: a valid sequence named by its folder and by its manifest (0); one run for every reason word but `frame_read` (1); and no argument, two arguments and a path that does not exist (2). The three paths apart, listed under `unpinned_paths`, are `frame_read`, a manifest that exists but cannot be read (exit 2), and a standard output that cannot be written (exit 1), where part of the `ok` line may have been written: each needs a permission or device setup that differs between Linux, macOS and Windows and does not hold for an account that may read every file, so no test makes them. The file also states that on a file system that ignores case, a frame file whose name differs from the listed one only in case is found under the listed name; no test covers that either.

The test `crates/mask-seq/tests/interface_contract.rs` runs the built binary for each row and checks its exit code, its standard error (empty, its beginning or the whole text, as the row says) and its standard output (empty or the whole text). It checks that the reason words of the file are those of the library's `Reason::ALL`, each with the exit code 1 and the detail of the table above, that the rows report every word but `frame_read`, that `frame_read` is listed under `unpinned_paths`, and that the entry for the report line, `error_line`, names the details of the four manifest reasons; and it checks that the rows use the exit codes 0, 1 and 2 and no other, and that the file gives the three names as the library's `FORMAT`, `MANIFEST_NAME` and `NAMING` do ([tests](#tests)). A change to the exit codes, the report lines or the reason words of the binary therefore fails `cargo test` until the file follows it. Of the file, the test reads only the exit code rows, the reason words with their exit code and detail, three phrases of `error_line`, the `frame_read` entry and the three names. The other entries, and the rest of `error_line`, describe behaviour that the tests of the library hold where a test can (the frame form in `tests/pgm.rs`, the hash in `tests/sha256.rs` and the `frame_hash` row, the order of the checks in `first_fault_wins`), and no test holds the three unpinned paths.

### The frame contract

`crates/frame-io` implements the raw frame contract of the [handoff convention](handoff.md), as [twixtor's `docs/handoff.md`](https://github.com/baairon/twixtor/blob/main/docs/handoff.md) defines it. A stream is whole frames back to back, with no header and no padding, in straight alpha:

| Depth | Layout | Bytes per pixel |
|---|---|---|
| 8 | packed `rgba` | 4 |
| 16 | packed `rgba64le` | 8 |
| 32 | planar `gbrapf32le`: four whole planes in the order G, B, R, A, with 1.0 at full scale | 16 |

A frame is its width times its height times the bytes per pixel, and at most 4 GiB. A stream that ends inside a frame is a partial frame, which is an error. A frame rate is an exact fraction.

The types of the library:

- **`Depth`, `Rate` and `Geometry`** state a stream: the depth, the exact rate, and the width, height and optional frame count, with the checks the convention names.
- **`FrameReader` and `FrameWriter`** read and write whole frames over any reader and writer.
- **`Pattern`** is the synthetic generator: a constant, a horizontal or vertical ramp, a checkerboard, an alpha ramp, or a seeded field from a small generator of the crate's own (`SplitMix64`), so that a pattern is the same bytes on every platform.
- **`compare`** tests two streams of one geometry as bit patterns at a tolerance of 0. At depth 32 a negative zero differs from zero, and two NaNs with different payloads differ. It reports each frame as equal or names the first difference by frame, pixel and channel with both raw values.

The crate computes nothing of the plug-in: it carries frames, makes test frames and compares.

### `frame-gen` and `frame-cmp`

```text
frame-gen --width W --height H --depth 8|16|32 --frames N [--fps N/D] [--pattern SPEC]
frame-cmp --width W --height H --depth 8|16|32 [--fps N/D] [--frames N] [-v] A B
```

`frame-gen` writes N whole frames to standard output. `SPEC` is `constant:R,G,B,A`, `hramp`, `vramp`, `checker:CELL`, `random:SEED` or `aramp`; without it the pattern is `hramp`. `frame-cmp` compares two files, one of which may be `-` for standard input, and writes its report to standard output, the first difference named by frame, x, y and channel. Both write an error as one line, `error: CODE: MESSAGE`, to standard error.

| Exit code | `frame-gen` | `frame-cmp` |
|---|---|---|
| 0 | the frames were written | the streams are equal |
| 1 | not used | the streams differ |
| 2 | usage | usage |
| 3 | unsupported: a depth other than 8, 16 or 32, or a frame above 4 GiB | the same |
| 4 | bad input, a closed output pipe included | bad input: an unreadable file, a partial frame, a count mismatch or a closed output pipe |

The convention keeps exit code 1 for a stage that failed. `frame-cmp` is not a stage, and its 1 says only that two readable streams differ. `docs/data/frame_tools_interface.json` states the arguments, streams and exit codes of both programs in a machine-readable form, and `crates/frame-io/tests/interface_contract.rs` runs the two binaries against it.

### `mocha-stage`

`crates/stage-cli` builds `mocha-stage`, a stage program in the handoff convention:

```text
mocha-stage list [--json]
mocha-stage apply selftest-copy --width W --height H --depth 8|16|32 --fps N[/D] [--frames N]
mocha-stage apply selftest-copy --width W --height H --depth 8|16|32 --fps N[/D] --describe
```

`list --json` prints a stage manifest that validates against `contracts/stage-manifest.schema.json`: the provider id `mocha_pro` (the schema's pattern for an id has no hyphen), the crate version, and one stage. That stage, `selftest-copy`, is a frame stage with preserved geometry at depths 8, 16 and 32 and no parameters. `apply` reads raw frames on standard input and writes the same frames to standard output; with `--describe` it prints the geometry answer and reads no input. The stage is a self test of the plumbing. It reimplements nothing of the plug-in, and no stage that does is to be added before its unit has been compared ([parity](parity.md)).

| Exit code | Means |
|---|---|
| 0 | all frames were processed, or the list or the description was printed |
| 1 | the stage failed |
| 2 | usage, a terminal on standard input or output included |
| 3 | unsupported: an unknown stage, a depth other than 8, 16 or 32, or any `--param` or `--layer` |
| 4 | bad input, a partial frame and a closed pipe included |

An error is one line on standard error, `error: <code>: <message>`, with the code words of the convention.

The `contracts/` folder is held byte for byte as twixtor publishes it at commit 8087d06 and is not edited here. The convention's conformance tool is not held here. Run from a twixtor checkout at that commit against the built `mocha-stage` on Linux, with `python -I tools/conformance.py --stage "<path to>/mocha-stage apply selftest-copy" --full` inside the checkout, it runs levels 1 and 3, the two that need neither ffmpeg nor the convention's control tool, and gives level 1: 270 passed, level 3: 33 passed, exit 0. The [handoff](handoff.md) page states which steps of joining the convention this covers.

### The generic crates

Seven libraries hold computations that a rebuild of an effect is likely to need, written from published sources and not from the plug-in. They share four rules. Each takes its inputs as plain numbers and slices and has no dependency outside the workspace, `pixel-format` using `frame-io` for its storage depths. Each states in its types every choice that its sources leave open, as a field of one `Config` value with a documented neutral default that claims nothing about any program, and lists the combinations of those fields in a fixed order (`enumerate`, or `ConfigSpace` in `resample` and `lens-model`), so that a search over conventions can index them. Each uses add, subtract, multiply, divide and compare in a fixed order, with no fused multiply add and no library transcendental function, apart from the square roots that `contour-eval`, `lens-model` and `soft-edge` name, so a result has the same bits on every platform. Each refuses bad input with a typed error and does not panic, with one exception: `squared_distance_transform` of `soft-edge` panics on a feature slice whose length is not the width times the height, or on a side above 2^20. None of them computes anything of the plug-in until a configuration has been selected by comparison with the original, and none holds a name, a value or a text of the original: every number in their tests is made up.

#### `pixel-format`

`crates/pixel-format` converts one sample between 8 bit integers, 16 bit integers and 32 bit floats, and applies and removes alpha premultiplication.

- **`Depth`** is `U8`, `U16 { max }` or `F32`, with the full scale of the 16 bit depth carried in the value. It maps to and from the storage depth of `frame-io`.
- **`int_to_float`, `float_to_int` and `Rescale`** hold one conversion each: a division by a denominator, a multiply and add followed by rounding and a clamp, and a multiply add round rule on a 16 bit value that saturates at 0 and 65535.
- **`premultiply_int` and `unpremultiply_int`** compute colour times alpha over the maximum, and colour times the maximum over alpha, exactly in integers, so every rounding mode is exact at every tie; `premultiply_f32` and `unpremultiply_f32` are the float forms.
- **`Config`** gives `convert_sample`, `convert_pixel`, `canonical`, `parse` and `enumerate` over a `Space`. `WorldLayout` and `Order` give pixel access with a row stride in bytes and a byte origin, and an offset that overflows is `LayoutError::Overflow`.

The fields of `Config` are `Rounding` (half up, ties to even, truncate, floor, ceiling), `Precision` (`F32` or `F64`), `Clamp` (none, range, range with NaN to zero), `ZeroAlpha` (zero colour or kept colour at alpha 0), `PremulOrder` (convert first or alpha first) and the full scale at 16 bit, any value. `Config::canonical` writes the fields as one line of `key=value` pairs and `Config::parse` accepts only that text. `Space::full()` enumerates 240 choices with distinct canonical lines.

Cited sources: the After Effects SDK Guide, pixel formats, for the 8 bit range 0 to 255, the 16 bit range 0 to 32768, float channels nominally 0 to 1 and unclamped, the channel orders ARGB and BGRA, and colour premultiplied by default; IEEE Std 754-2019 section 4.3, for the four rounding directions it names (to nearest with ties to even, toward zero, toward positive and toward negative); half up is added as a fifth. Left open, each as a field or a rule pinned by a test and none claimed for any program: the rounding mode, whether the arithmetic runs in single or double precision, what a NaN or an out of range value becomes, what colour a pixel of alpha 0 keeps, whether alpha is applied before or after the depth changes, and the full scale of the 16 bit depth. Integer to integer conversion goes through a float, and the alpha operation is never applied to the alpha sample.

#### `contour-eval`

`crates/contour-eval` evaluates a planar contour at a parameter and flattens it to a polyline.

- **`Contour`** holds a `Kind` (`Bezier`, or `XSpline` with a `Basic` or `General` variant), a closed flag and a list of `Point` values, each with a position, two tangent handles and a stored weight; a field that a kind does not use is ignored. `span_count`, `bezier_span` and `validate` read and check it.
- **`eval`** gives the point of one span at a parameter from 0 to 1. `bezier_split` cuts a Bezier span in two. `f_blend`, `g_blend`, `h_blend` and `xspline_weights` expose the blending functions and the four weights of a segment.
- **`flatten`** takes a `Flatten` rule: `Uniform(n)`, `Subdivide { tol, max_depth }` or `ByLength { step, samples }`. The depth is at most 24 and a span gives at most 4096 segments.
- **`ContourError`** names every refusal: too few points, a non finite field, a span or parameter out of range, a shape parameter outside its range, a zero weight sum, a non finite result, a bad flattening constant, and a flatness test or a call that the kind of the contour does not support.

The fields of `Config` are the evaluation form of a Bezier span (`Bernstein`, `DeCasteljau`, `Horner`), the precision (`F32` or `F64`), the tangents (`Relative` offsets or `Absolute` positions), `weight_map` (a map from a stored weight to a shape parameter, four of them listed), `open_end` (`RepeatEnd` or `RepeatEndSharp`), `closing_span`, `repeat_first` and `flat_test` (`Sampled` or `ControlPolygon`). `Config::enumerate` lists 768 combinations.

Cited sources: G. Farin, "Curves and Surfaces for CAGD", 5th edition, 2002, for the Bezier form and its subdivision; C. Blanc and C. Schlick, "X-Splines: A Spline Model Designed for the End-User", SIGGRAPH 1995, pages 377 to 386; J. Lane and R. Riesenfeld, "A Theoretical Development for the Computer Generation and Display of Piecewise Polynomial Surfaces", IEEE Transactions on Pattern Analysis and Machine Intelligence 2(1), 1980, for the control polygon flatness test. The paper was not available as text, so the X-spline polynomials follow the reference implementation that a public drawing program ships and are not read from the paper's own text: the tests check them against what the paper states, and they agree with four public implementations of one lineage. Left open: the evaluation form, the number type, how handles are stored, how a stored weight becomes a shape parameter, how an open contour ends, whether a closed one has its closing span and a repeated first point, and the flatness test. The `Sampled` test looks at three points of an interval and is a sample, not a bound.

#### `coverage-raster`

`crates/coverage-raster` turns closed polygons into the coverage of the pixels of a target rectangle.

- **`Polygon`** is a list of closed rings of `(f64, f64)` points that are filled together under one fill rule, so a hole is a ring inside another. **`Target`** is an integer origin, a width and a height.
- **`rasterise`** returns a `Coverage` plane of values from 0 to 1, and **`rasterise_buffer`** returns a `Buffer` of 8 bit, 16 bit or 32 bit float samples.
- **`Error`** refuses a NaN or infinite coordinate, a coordinate above 2^50 in magnitude, a setting out of range and a target above 65536 on a side or 2^26 pixels.

The fields of `Config` are the `Method` (exact area, one point, a regular grid of n by n samples, n rotated samples one per row and column, or an explicit list of offsets from 0 to 1), the `FillRule` (non-zero or even odd), the pixel centre, the row direction, the snapping of coordinates to a power of two, the sample depth, the scale, the rounding (half up, half even, floor, ceiling), how several polygons `Combine` (maximum, sum clamped to 1, over, replace) and the arithmetic precision. `Space::default().enumerate()` lists 7616 configurations with distinct labels.

Cited sources: T. Duff, "Polygon Scan Conversion by Exact Area Calculation", in Raster Imaging and Digital Typography, 1989; the winding rules of section 5 of the OpenGL Utility Library 1.3 specification; the top left tie rule of the Direct3D 11 rasterization rules. Exact area is computed by slabs between the heights where edges start, end or cross, which keeps it exact for self intersecting input under both fill rules. It is exact when the vertices are dyadic and no two edges cross inside a pixel row; otherwise its error per slab is of the order of the largest coordinate times 2^-53 (inferred, not measured), and the tests measure it against the shoelace area. Left open: the method, the fill rule, where a pixel centre lies, which way rows run, the quantising, and how polygons combine. The crate evaluates no curve and applies no feather or gamma: a caller flattens its contours first.

#### `planar-xform`

`crates/planar-xform` holds a 3 by 3 projective transform in double precision.

- **`Homography`** is a matrix `m[row][col]` with a `Convention` (`Column` or `Row`). It is built by `from_matrix`, `identity`, `translation`, `scale`, `rotation_quarter_turns` or `rotation_cos_sin`; `then` composes with the first transform applied first; `inverse` returns the adjugate over the determinant or `Error::Singular` or `Error::NonFinite`.
- **`apply`** maps a point with the homogeneous divide. A divisor not above `divisor_min` goes to the `DivisorGuard`: `ReturnNone`, `Clamp(value)` or `PassThrough`. `map_rect` and `map_rect_bounds` give the image of a rectangle and its bounding box.
- **`from_four_points`** solves the 8 by 8 system by Gaussian elimination with partial pivoting, taking the lowest row on a tie, and refuses a pivot below a relative tolerance.
- **`PixelSpace`** converts between the unit square and pixel or physical coordinates, and **`Point`** and **`Rect`** are the plain types.

The fields of `Config` are the vector convention, the divisor guard and minimum, the `PixelCenter` (`Integer` or `Half`), the `YAxis` (`Down` or `Up`), the pixel aspect and the tolerance of the solve. `Config::enumerate(clamps, aspects)` lists the products of the discrete choices with the clamp values and aspects that the caller gives.

Cited sources: R. Hartley and A. Zisserman, "Multiple View Geometry in Computer Vision", 2nd edition, 2003, sections 2.2.3, 2.3, 2.4.1, 4.1 and 4.4; G. Strang, "Linear Algebra and Its Applications", 4th edition, 2006, section 4.4; IEEE Std 754-2019. Left open: whether points are columns or rows, what happens at a divisor of zero or near it, where a pixel centre lies, which way the Y axis runs and the pixel aspect.

#### `resample`

`crates/resample` resamples an `Image` of 32 bit floats, packed or planar with any channel count, under an inverse mapping.

- **`resample`** takes the source, the output size, a closure from an output coordinate to a source coordinate and a `Config`. **`resample_matrix`** takes a `Matrix3` instead, and a zero `w` gives a zero pixel. Coordinates are `f64`, accumulation is in `f64`, and the result is rounded to `f32` once. A coordinate that is not finite or above 2^30 in magnitude gives a zero pixel.
- **`kernel`** exposes `sinc` (a fixed nine term series with a stated absolute error of 1e-12), `bilinear`, `keys`, `mitchell` and `lanczos`; **`wrap`** gives the border index rule.
- A tap of weight exactly zero is skipped, so a non finite sample reaches only the pixels that use it, and the interpolating kernels return exactly 1 and 0 at integer offsets, so an identity mapping returns its input.

The fields of `Config` are `filter` (`Nearest`, `Bilinear`, `Keys { a }`, `Mitchell { b, c }`, `Lanczos { lobes }`, `Box`), `tie`, `center`, `border` (`Zero`, `Clamp`, `Repeat`, `Mirror`, `MirrorNoEdge`), `alpha` and `alpha_channel` (straight or premultiplied), `normalize` (none, the sum of all taps, the sum of the taps inside the image), `order` (row major or column major), `footprint` (`Unit`, `Jacobian`, `Fixed { w, h }`) and `min_extent`. `ConfigSpace::standard()` lists 3960 combinations.

Cited sources: R. Keys, "Cubic Convolution Interpolation for Digital Image Processing", IEEE Transactions on Acoustics, Speech, and Signal Processing 29(6), 1981; D. Mitchell and A. Netravali, "Reconstruction Filters in Computer Graphics", SIGGRAPH 88, section 4; C. Duchon, "Lanczos Filtering in One and Two Dimensions", Journal of Applied Meteorology 18, 1979; K. Turkowski, "Filters for Common Resampling Tasks", Graphics Gems I, 1990. Left open: the filter and its parameters, which way an exact half goes, where a pixel centre lies, what lies beyond the border, how alpha is interpolated, whether weights are renormalised, the order of the two dimensional sum and the footprint of the box filter. The effect of the order of summation shows after the single rounding only in rare cases (inferred, not measured). The crate converts no sample type.

#### `lens-model`

`crates/lens-model` holds lens distortion as maps of coordinates in double precision.

- **`Model`** is `Radial` (terms k1 to k3 and p1, p2), `Division` (one term lambda) or `Squeezed` (k1 to k3 and a squeeze per axis). `apply` maps a point, `invert` finds the preimage by an `InverseMethod` (`FixedPoint`, `Newton` or `ClosedForm`) with a fixed number of iterations and no convergence test, and `closed_form` gives the division inverse. `LensError` names a refusal, a non finite result included.
- **`Config`** converts between pixels and normalised coordinates and gives `to_distorted`, `to_undistorted`, `source_map(Direction)`, the closure that `resample` needs for either direction, `target_map`, and `mapped_bounds(Direction, steps)`, the bounding box of the mapped rectangle sampled along its border.

The fields of `Config` are the model, the `Normalisation` (half width, half height, half diagonal or a focal length in pixels), the size, the centre in pixels, the pixel aspect, `ModelMaps` (which direction the formula maps), the inverse method and the iteration count. `ConfigSpace` lists the axes and `enumerate` returns every combination.

Cited sources: D. C. Brown, "Decentering Distortion of Lenses", Photogrammetric Engineering 32(3), 1966; D. C. Brown, "Close-Range Camera Calibration", Photogrammetric Engineering 37(8), 1971; A. W. Fitzgibbon, "Simultaneous linear estimation of multiple view geometry and lens distortion", CVPR 2001, section 2. The two tangential terms are numbered as the crate documents, p1 being Brown's second term and p2 his first. The closed form inverse of the division model and the squeezed variant are defined in the crate and are not taken from a source. Left open: the model, the scale that normalises a coordinate, the centre, the aspect, which direction the formula maps and the inverse method.

#### `soft-edge`

`crates/soft-edge` offers soft edge operators on a one channel `f32` buffer given as a row major slice with a width and a height.

- **`box_blur`, `box_blur_repeated` and `box_kernel`** blur with an integer or fractional radius, by the `EndTaps` rule (two end taps of the fractional weight) or the `Lerp` rule (a mix of the kernels of the two neighbouring radii).
- **`gaussian_blur` and `gaussian_kernel`** take a `GaussSpec`, a sigma or a radius, truncate at a multiple of sigma and normalise the weights. `exp_neg` evaluates the exponential by a fixed term series, as `Series` or as `Recurrence`. `convolve_separable` applies a given kernel.
- **`squared_distance_transform`** gives exact squared distances as integers in linear time. **`signed_distance`** and **`feather`** give a ramp over the signed distance to the zero crossing of a hard mask, inward, outward or centred, with a `Profile` of `Linear`, `Smoothstep` or `Cosine` through `ramp`.
- **`resolve_index`** applies a `Border` of `Zero`, `Clamp`, `Mirror` or `MirrorNoRepeat`. **`Error`** is `SizeMismatch` or `BadParameter`.

The fields of `Config` are the border, the fractional radius rule, the truncation factor, the exponential method and its term count, the radius to sigma factor, the profile and direction of the ramp, the inside threshold and the edge offset. `Config::enumerate` lists 288 combinations of the discrete choices.

Cited sources: P. F. Felzenszwalb and D. P. Huttenlocher, "Distance Transforms of Sampled Functions", Theory of Computing 8(19), 2012, sections 4 and 5; R. Szeliski, "Computer Vision: Algorithms and Applications", 2nd edition, 2022, section 3.2; M. Abramowitz and I. Stegun, "Handbook of Mathematical Functions", 1964, sections 4.2 and 4.3; the smoothstep definition of the Khronos Group's "The OpenGL Shading Language" specification. The section numbers are cited from memory and are not checked against the sources. Left open: the border rule, the fractional radius rule, the truncation, how the exponential is evaluated, the radius to sigma factor, the ramp profile and direction, the threshold and the edge offset.

Five further crates compute from published methods and from nothing else: `sphere-reorient`, `mesh-warp`, `blend-ops`, `key-eval` and `planar-track`. Each one is generic: where the published method leaves a choice open, such as a pixel centre, the side of a tie, a rounding rule or the order of two tests, the choice is a field of an explicit configuration type with a documented default, and the crate can list the configurations it accepts. A default is one point of that list and claims nothing about the original. None of the six computes anything of the plug-in until a configuration has been selected by comparing its output with the original's under the [parity](parity.md) rules, and none has been. Each crate's own `README.md` gives its sources with their sections, and the documentation comments of its configuration type give the meaning of every field.

All five compute in IEEE 754 floating point with addition, subtraction, multiplication, division, comparison and square root, plus `floor`, `round` and the ceiling where a crate says so, with no fused multiply add and no transcendental function of the platform's maths library. On that reading of the code a run gives the same bits on every platform that follows IEEE 754; CI runs their tests on Linux, macOS and Windows.

#### `sphere-reorient`

`crates/sphere-reorient` maps image coordinates of a spherical image in `f64`: a pixel of an equirectangular image to a longitude and a latitude, those to a unit direction, a direction through a 3 by 3 rotation, and the way back to a pixel, so that a resampler can reorient the image. It maps coordinates only and reads no pixel. Angles are in turns, 1.0 being a full circle, so that quarter and half turns are exact.

- **`equirect`**: `pixel_to_lonlat`, `lonlat_to_pixel`, `lonlat_to_dir`, `dir_to_lonlat`, `pixel_to_dir` and `dir_to_pixel`, with the types `Size`, `LonLat` and `PixelPos`; a `PixelPos` reports whether the seam was wrapped in longitude or a pole clamped in latitude. `inverse_map` returns the closure a resampler needs, from an output pixel to a source pixel.
- **`rotation`**: `Angle` and `Mat3`. A rotation is a matrix the caller gives, or comes from yaw, pitch and roll (`from_yaw_pitch_roll`, `about_x`, `about_y`, `about_z`), with `mul`, `transpose`, `apply` and `det`.
- **`cube`**: six faces in a layout image, `CubeConfig` with a grid of six columns, six rows, three by two or two by three (`CubeGrid`), the face held in each slot and a quarter turn of each, and the maps between a direction and a pixel of the layout.
- **`stereo`**: `StereoLayout`, one view or two views over and under or side by side, as rectangles of the image (`Rect`), with an optional exchange of the two eyes.
- **`trig`**: the sine, cosine, arc tangent and arc sine the crate uses, written as series with a fixed number of terms.

`Config` holds the conventions of the equirectangular map:

| Field | Choices |
|---|---|
| `lon_zero` | the fraction of the width where longitude is zero |
| `lon_direction` | longitude increasing to the right or to the left |
| `pixel_center` | a pixel sampled at its centre or at its corner |
| `pole_rows` | the first and last rows on the poles, or inset by half a row |
| `north` | north at the top or at the bottom |
| `axis_order` | the six orders of yaw, pitch and roll |
| `angle_sign` | angles as given or negated |
| `rotation_use` | the rotation applied as given or transposed |

`Config::all()` lists the 1536 configurations of its discrete fields and `CubeConfig::all()` the 11520 cube layouts. Sources: J. P. Snyder, Map Projections: A Working Manual, 1987, for the equirectangular and the gnomonic projections; G. Strang, Introduction to Linear Algebra, section 2.6, for the rotations about the axes; M. Abramowitz and I. A. Stegun, Handbook of Mathematical Functions, 1964, sections 4.3.65, 4.3.66, 4.4.34 and 4.4.42, for the series.

Left open: the resampling of pixels, the filter and the treatment of the seam and the poles in a resampler, the layout of any particular file, and which point of the configuration space, if any, the original uses.

#### `mesh-warp`

`crates/mesh-warp` maps coordinates by a triangle mesh in `f64`. Each vertex carries a source and a destination position; a point is located in a triangle of one space and carried to the other by its barycentric weights. It maps coordinates only and reads no pixel.

- **`Mesh`**, with `Vertex` and `Vec2`: the vertices and the triangle index list. `validate` checks the indices, finite values, degenerate triangles and, when asked, one winding; errors are `MeshError`. `grid_mesh` builds a regular grid with a configurable diagonal (`Diagonal`: forward, back or alternating).
- **`Locator`**: a uniform grid over the triangles of one space that finds the triangle of a point.
- **`Warp`**: a validated mesh with a locator in each space. `inverse` maps a destination point to its source position, `forward` the reverse, and `inverse_closure` returns the closure a resampler needs.
- **`QuadPatch`**: a bilinear patch with its forward map and its inverse, which solves the quadratic in one parameter with the IEEE square root.

`Config` holds the tie rule for a point on a shared edge or vertex (`TieRule`: the lowest index, or the top left ownership rule), the rule for a point in no triangle (`Outside`: none, the nearest point of the nearest triangle, or the affine map of the nearest triangle extended), the winding check (`Winding`), the barycentric formula and its order of operations (`BaryForm`: edge functions or Cramer's rule), whether degenerate triangles are allowed, and the grid size of the locator (`GridRes`). `config_space()` lists 96 configurations. `QuadConfig` holds the root choice (`RootChoice`: minus, plus, or the first in the unit square), whether a point outside the unit square is accepted, and the threshold below which the quadratic is treated as linear; `quad_config_space()` lists 6. Sources: G. Wolberg, Digital Image Warping, 1990, chapter 3 and its section on mesh warping; J. Pineda, A parallel algorithm for polygon rasterization, SIGGRAPH 1988, for the edge functions and the top left rule; C. Ericson, Real-Time Collision Detection, 2005, sections 3.4 and 5.1.

Left open, on a reading of the code: the resampling of pixels; a tolerance at the border of a quad patch, so that with outside points refused a point within rounding of the border can be refused; a check that products of vertex coordinates stay finite, since `validate` checks the coordinates themselves; and which configuration, if any, the original uses.

#### `blend-ops`

`crates/blend-ops` composites RGBA pixels in `f32` or `f64`, the scalar type being chosen through the trait `Real`.

- **`PorterDuff`**: the twelve operators of Porter and Duff, from clear to xor, with their fractions (`factors`).
- **The blend modes**: an enum of eighteen modes. Twelve are the separable modes of the W3C recommendation, from normal to exclusion; plain add and subtract are two generic extensions; hue, saturation, colour and luminosity are its non separable modes, with `lum`, `sat`, `set_lum`, `set_sat` and `clip_color` as it defines them. `is_separable` tells the two kinds apart.
- **`composite_pixel`** composites one `Rgba` pixel, and `composite_buffers` interleaved RGBA slices with an optional one channel matte, refusing slices that hold no whole number of pixels or describe different numbers of pixels with `BufferError`. `blend_channel` and `blend_rgb` give the blend function alone.

`Config` holds the input and output form, straight or premultiplied (`Form`); where the opacity acts, on the source alpha or on the result (`OpacityPoint`); clamping of none, the output, or input and output (`Clamp`); the soft light function of the W3C or the older one of ISO 32000-1 (`SoftLight`); the four guard values of colour dodge and colour burn (`Guard`) and the order of their two tests (`GuardOrder`); and whether the backdrop weighted source colour of the general formula is used (`BackdropMix`). `Config::enumerate()` lists the 3072 configurations, `Config::SPACE_SIZE`. Sources: T. Porter and T. Duff, Compositing Digital Images, SIGGRAPH 1984, section 3; W3C, Compositing and Blending Level 1, sections 5.1, 9.1, 9.2 and 10; ISO 32000-1:2008, section 11.3.5.

Left open: input that holds a NaN, which the crate does not support; the colour of a pixel whose divisor in `ClipColor` is zero, which the recommendation leaves open and the crate sets to its luminosity; colour spaces and transfer curves, since the crate blends the numbers it is given; and which configuration, if any, the original uses.

#### `key-eval`

`crates/key-eval` gives the value of a list of keys at a time, in `f64`. A key has a time, a value of N components and an interpolation kind, which governs the segment that starts at it.

- **`Key`**: `Key::hold`, `Key::linear`, `Key::hermite`, with an in and an out slope per component, and `Key::bezier`, with an in and an out handle per component.
- **`KeyList`**: `KeyList::new(keys, &Config)` sorts the keys by time with a stable sort, refuses a field that is not finite (`KeyError`), and applies the rule for keys of one time; `evaluate(&Config, t)` returns the N components at time `t`.
- **Helpers**: `solve::cbrt`, a cube root with a fixed number of Newton steps, `solve::solve`, the curve parameter of a Bezier time curve, and `eval::floor`, `eval::round_half_up` and `eval::round_half_even`.

`Config` has twelve fields: the side a hold takes at a key time and the side a jump takes (`hold_side`, `jump_side`); snapping of the time to frames and the frame count per unit (`snap`, `frames_per_unit`); the components handled one by one or jointly (`vector_mode`); handles as points or as offsets (`handle_mode`); the clamp of handles, none, in time, or scaling both (`clamp`); the solver of the Bezier time curve (`solver`: the closed form of Cardano with a number of Newton polish steps, bisection with a number of steps, or Newton with a number of steps and a start); the rule before the first key and after the last (`before`, `after`: hold, linear, cycle, cycle with offset or mirror); keys of one time (`coincident`: keep the first, keep the last, or a jump); and the value of an empty list (`empty`). Every solver does a fixed amount of work. `Config::axes()` and `Config::enumerate()` give the 144000 configurations of the standard axes. Sources: G. Farin, Curves and Surfaces for CAGD, A Practical Guide, fifth edition, 2002; Cardano, Ars Magna, 1545, chapter 11, and W. H. Press and others, Numerical Recipes, third edition, 2007, sections 5.6, 9.2 and 9.4; F. N. Fritsch and R. E. Carlson, Monotone piecewise cubic interpolation, SIAM Journal on Numerical Analysis 17(2), 1980; and the Blender manual on F-curves, their interpolation and extrapolation, and the cycles modifier.

The clamp rules, the side rules at key times, the rules for keys of one time and the snapping rules are choices of this crate, stated in the documentation of `Config`, and claim to match no program. Left open: the time base and units of any file of keys, and which configuration, if any, the original uses.

#### `planar-track`

`crates/planar-track` tracks a planar region through a sequence of grey level images by direct image alignment: for each frame it finds the planar motion that makes the warped frame agree with a fixed template over the region, by minimising the sum of squared grey level differences coarse to fine over a Gaussian pyramid. Every method in it is taken from the published literature and written from its description, and every choice is a field of an explicit configuration.

- **Images and pyramid** (`image`, `pyramid`): grey images of `f64` with a stated pixel centre, bilinear sampling, central difference gradients, and a Gaussian pyramid from a binomial kernel.
- **Models** (`model`): translation (2 parameters), translation and scale (3), similarity (4), affine (6) and a projective homography (8), each a subspace of one 3 by 3 pose matrix (`Pose`).
- **Region and weights** (`region`, `edt`): a rectangle, a convex quadrilateral or a mask, the selection of the pixels of largest gradient, and boundary weights from a linear time Euclidean distance transform.
- **Alignment** (`align`, `linalg`): one pyramid level with four update rules (forward additive, forward compositional, inverse compositional, efficient second order minimisation), Levenberg or Marquardt damping, and a gain and offset illumination model, solved jointly or in turn.
- **Search** (`ncc`): an optional coarse search over integer translations by normalised cross correlation.
- **Tracking** (`tracker`): the tracker type tracks frame to frame forward and backward, with an optional constant velocity prediction, and reports for every frame a `FrameResult`: the pose and its source, the root mean square residual, the inlier and valid fractions, the iterations, whether the finest level converged and why it stopped, the gain and offset, and the correlation score. Its correction functions are `set_pose`, `clear_pose`, `solve_frame`, `track(from, to)`, `track_forward`, `track_backward` and `predict`: a pose set by hand is kept, the frames after it are solved again from it, and a corrected pose is not extrapolated from the tracked pose before it.

`Config` holds the pyramid, the model (`dof`), the update rule, the illumination model, the damping, the limits of each level (`level_params`), the pixel selection, the search, the velocity, the inlier threshold and the least number of valid pixels; `Config::validate` checks every range. `ConfigSpace` lists the 5760 configurations of eight discrete dimensions: model 5, rule 4, illumination 3, damping 2, acceptance test 3, boundary weight 4, motion prediction 2 and search 2. Sources, cited in the crate's `README.md` with their sections: B. D. Lucas and T. Kanade, 1981; S. Baker and I. Matthews, Lucas-Kanade 20 Years On, 2004; S. Benhimane and E. Malis, 2004; K. Levenberg, 1944, and D. W. Marquardt, 1963; P. J. Burt and E. H. Adelson, 1983; J.-Y. Bouguet, 2001; G. D. Hager and P. N. Belhumeur, 1998; P. F. Felzenszwalb and D. P. Huttenlocher, 2012; J. P. Lewis, 1995, and R. Brunelli, 2009.

Its tests compare its results on a sequence with the truth within a stated tolerance, never as exact bits: they render an invented textured plane through known homographies and require the corners of the region within 0.15 pixels of the truth, for each of the five models and each of the four rules. A result of that kind is not a parity result: under the [status rule](parity.md#the-status-rule) a result at any tolerance other than 0 has no rung label, and how a track is to be compared with the original is stated in [parity: what the results do not cover](parity.md#what-the-results-do-not-cover). Left open: an update of the template during tracking, a robust loss, occlusion handling beyond the weights and the inlier report, a correlation search finer than one pixel, colour images, and which configuration, if any, the original uses.

## Build

Run from the repository root with a stable Rust toolchain. The manifests set no minimum Rust version; the crates are tested with Rust 1.94.0, and CI installs the stable toolchain. No crate depends on anything outside the workspace, so the build needs no network.

```sh
cargo build --release
cargo test
```

`cargo build --release` builds the eighteen libraries and the binaries `shape-predict`, `mask-validate`, `frame-gen`, `frame-cmp` and `mocha-stage` into `target/release/`; the generic crates build no binary. `cargo test` builds them and runs their tests. `CARGO_TARGET_DIR` moves the build output. The Python tools need Python 3 and nothing outside its standard library.

## Documentation tools

All three run from the repository root with `python -I`, which keeps the user's site packages and the current folder out of the import path; where only `python3` is installed, the same commands run with `python3 -I`.

### Generated figures

`tools/gen_docs.py` takes the result files and writes every generated figure and table in `README.md` and under `docs/`, plus `docs/data/facts.json`, which gives each figure's value and source.

- A figure sits between an opening comment `<!--f:` KEY `-->` and a closing `<!--/f-->`, and a table or block between `<!--gen:` KEY `-->` and `<!--/gen:` KEY `-->` (spaced out here so that this line is no marker); the generator replaces only the text between the two.
- Its inputs are `parity/<unit>/results/summary.json`, one file per unit and rung, and the re-run file beside it, `summary.json.rerun.json`, when there is one. With no result file it writes the status line "method and preparation only: 0 cases compared" and carries no parity label.
- `--check` builds everything in memory, writes nothing and runs no process.

| Exit code | Means |
|---|---|
| 0 | no unknown key; with `--check`, also no stale page |
| 1 | with `--check`, a stale page; in either mode, a marker that names an unknown key |
| 2 | usage |
| 3 | a result file or a re-run file cannot be read, is not valid JSON, lacks a field or names another unit |

| Key | Kind | Source |
|---|---|---|
| `status_line` | figure | the status line: the empty form, or one part per rung from the totals lines of `tools/status.py` |
| `units_total` | figure | the number of result files |
| `case_floor` | figure | `CASE_FLOOR` in `tools/status.py` |
| `units` | table | one row per result file, then the totals lines per rung |
| `label` | block | the label block that heads a unit's record: the empty form, or one block per unit |
| `pairing` | table | the `pairing` and `calibration` fields of every result file that carries them |
| `notes` | table | the blockers, stand-ins, logic parity, qualifiers, surviving mutants and not rebuilt lists of every result file |

### Page checks

`tools/check_docs.py` checks `README.md` and every page under `docs/`, and `ascii`, `dashes` and `secrets` read every text file of the repository: the files ending in `.md`, `.py`, `.rs`, `.toml`, `.json`, `.tsv`, `.yml`, `.yaml`, `.txt` or `.lock`, and the files named `LICENSE`, `.gitignore` or `.gitattributes`, outside `.git`, `target`, `node_modules` and `__pycache__`. The checks of page prose skip fenced code and HTML comments. `--list NAME` prints every finding of one check, or the NOT RUN line for a check that has no term list to read.

| Check | Finds |
|---|---|
| `regen` | a generated figure or table that differs from a fresh generation, or an unknown key |
| `links` | a relative link or `#anchor` that does not resolve |
| `ascii` | a character outside printable ASCII, or a control character other than tab, LF and CR, in any text file |
| `dashes` | a dash character from U+2010 to U+2015, or U+2212, in any text file |
| `wording` | a word that claims a final state, "bit exact" without a rung, "bit for bit" without a rung, tolerance or judge on the same line, a share written per hundred, a merged count phrase |
| `hexaddr` | a hexadecimal address in page prose: `0x` and five or more hexadecimal digits |
| `secrets` | in any text file: a private key block, a GitHub token or another API key, a secret assigned in the text, a home directory path, or an email address outside `bairon.dev`, `users.noreply.github.com` and the example domains |
| `process` | words that say how the repository was made rather than what it does, from the term list |
| `scope` | names that belong outside the repository, from the term list |

The last two checks have no terms of their own. They take the rows of a term list that is kept outside the repository, named by `--private-terms PATH` or by the environment variable `MOCHA_PRIVATE_TERMS`; the option wins when both are given. Each row of the list applies either to every text file, code spans, fenced blocks and comments included, or to page prose only. A finding of either check prints the file, the line and the name of the row, never the text it matched. With no list named, each of the two prints a NOT RUN line, adds no finding, and the summary line counts it among the checks not run; this is how they run in CI. `--require-private` refuses to run at all unless a list is named.

| Exit code | Means |
|---|---|
| 0 | no check has a finding; a check that did not run adds none |
| 1 | at least one finding |
| 2 | usage; a term list that is missing, unreadable, empty or malformed (the message names the file and, for a bad row, its line number, never the row's text); or `--require-private` with no list named |

### Status of one result file

`python -I tools/status.py FILE...` prints one line for each result file: its unit, its status, its rung label with a note (the reason there is no label, or the floor items a thin label misses), the allowed wording when the unit matches, its blockers, and its re-run state from `FILE.rerun.json` when that file exists. A file that contradicts itself prints as an invalid result file. One totals line per rung follows, and a result file whose rung is null gets a totals line of its own that calls it not a parity unit. With no file it prints `Not started (no result file)`. The rule it applies is the [status rule](parity.md#the-status-rule).

Two kinds of result file are never a parity unit, whatever rung they name. A file with a `rows_basis` block, whose rows are in part only read from a specification and in part seen in a partial observation of the original, prints `Read and partly observed` with the fixed label `partial observation, not a result of record`. A file whose `frames` block names `owner supplied frames` as its reference prints `Owner supplied frames` with the fixed label `owner supplied frames, not the original, not a parity result`. Neither reaches Matches or Verified, and each has a totals line of its own.

| Exit code | Means |
|---|---|
| 0 | the lines were printed, `Not started (no result file)` included |
| 2 | usage: `-h` or `--help` prints the usage line on stderr |
| 3 | a result file or its re-run file cannot be read, is not UTF-8 or is not valid JSON; `status: cannot read FILE: REASON` goes to stderr and nothing is printed on stdout |

## Tests

From the repository root:

```sh
cargo test --locked --no-fail-fast
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
python -m compileall -q tools
python -I tools/test_status.py
python -I tools/test_gen_docs.py
python -I tools/test_check_docs.py
python -I tools/gen_docs.py --check
python -I tools/check_docs.py
```

- **The crate.** `crates/effect-params/tests/table_and_pairing.rs` checks that a table survives a round trip with every bit kept (-0, infinity, unknown cells, empty flags), that decimal cells and comments are read, that malformed input is refused (no magic line, too few or too many cells, an unknown kind, bad value or flag cells, an empty name), that kind names round trip, the flag helpers and the distinct single bits of the flag constants, the normalization rules, every match class and field verdict, that exact names claim before normalized ones and that a row is claimed once, that an unknown observation and a kind mismatch are reported, that an integer cell never equals a float cell, that integer signs and high flag bits are compared, and that an unknown predicted flag set is not scored. Two of its tests tie the format to its text: `example_parses` loads the example of [the table format](#the-table-format) from this page, and `serialized_table_round_trips_but_hand_written_text_does_not` checks that a table written by the serializer is the same text byte for byte after a parse and a second serialize, while a hand-written table with a comment, a blank line and an `f:` cell comes back as different text. `crates/effect-params/tests/edges.rs` pins the same rules at their edges: the text and line number of parse errors, comment and blank lines counted, and their display as `line N: message`; the length of a bit cell, 16 bytes after `b:`, `?` alone as the unknown cell, a value or flags cell with a space in or around it refused, and a flags cell that refuses an upper-case `0X`, a bare `0x`, a minus sign and a value wider than 32 bits; names kept as written, spaces included, and kinds refused with a capital letter or with a space before or after; which lines are blank, comments or the magic line, that one carriage return at the end of a line is dropped and a second is not, and that an empty table is valid; the exact text of the serializer, NaN payloads included; `f:` cells read to their decimal value in double precision, `f:-0.0`, `f:inf` and `f:-inf` included, and integer cells up to the largest signed 64-bit integer and not one past it; `bits_eq` true only for two unknown cells or for two cells of the same type with the same bits, NaN payloads and the sign of zero included, so that two NaN cells of different payloads differ in a pairing, and `is_unknown` true for the unknown cell only, so that a NaN cell is known and scored; each rule of `normalize_name` on its own, its lower case for letters beyond ASCII, and that it drops only a trailing colon or `...`, never a leading one or another mark; that of two predicted rows of one name, exact or normalized, only the first claims the one observed row, and that of two observed rows of one name the first is claimed, in both passes; that an exact pair is kept when a normalized one is also possible, that names that differ in case only pair as normalized, that a verdict keeps the predicted name as written, that the observed rows no prediction claimed are listed in their order, and that empty tables pair to nothing; that the default, the minimum and the maximum are each judged against the same field, so that a swapped minimum and maximum differ on both, and the verdicts of unknown cells, of flags with their difference, a set that holds the other one differing as well, and of kinds; the values of the flag constants, all 32 bits of a flag set, and the set operations of `ParamFlags`; and the order and names of the kinds, which parse as whole lower-case words only, and a round trip of every kind through a table. It builds every table in memory and reads no file. `crates/effect-params/tests/signs_and_gaps.rs` pins the refusal of bytes that are not hexadecimal digits: that a `b:` cell refuses, in the 16 bytes after `b:`, a plus or a minus sign at the start or among them, two signs, a space, a letter past `f`, an `x`, and a letter or a digit outside ASCII, with its line and the text `bad bits "<cell>": invalid digit found in string`; that a flags cell refuses after `0x` a plus sign before, among or after its digits or alone, a minus sign before its digits or alone, two signs, a space, a letter past `f`, a NUL byte, a dot, a second `0x`, and a letter or a digit outside ASCII, with its line and the text `bad flags: invalid digit found in string`; and that of a bad `b:` cell, a bad integer cell and a bad flags cell in one row, the first is reported. It also pins the texts of other faults: `bit cell needs 16 hex digits: "<cell>"` for a `b:` cell of 0, 15 or 17 digits, `bad flags: cannot parse integer from empty string` for a bare `0x`, `bad flags: number too large to fit in target type` for a flags value wider than 32 bits, and `bad flags cell "<cell>"` for a flags cell that does not open with the lower-case `0x`, such as `0X1`, `x1` or `00x1`. It checks that these are read: 16 zeros in a `b:` cell, a flags cell with all 32 bits set, a flags cell of 16 digits with leading zeros, upper-case digits in both cells, which the serializer writes back in lower case, and every hexadecimal digit. Its other tests pin narrow cases: a float cell of the smallest positive normal or subnormal value, of the largest or the smallest finite value or of the machine epsilon is known; two opposite infinities differ in a pairing, and two equal ones are the same; `f:nan` reads as a NaN, and `f:1e-310` and `f:-1e-310` to their subnormal values; a normalized verdict and an unmatched one keep the predicted name as written, spaces and a trailing colon included; `normalize_name` keeps a hyphen, also before a trailing colon that it drops; a carriage return inside a name stays in that name; and a kind cell with a space or a NUL byte after the kind is refused as an unknown kind. It also builds every table in memory and reads no file. No test here involves the original.
- **The table format, version 2.** `crates/effect-params/tests/extra_fields.rs`, `tests/table_v2.rs` and `tests/table_v2_fix.rs` check the extra cell of every kind, the ten cells of a version 2 row, the escapes, the refusals with their line numbers, that a version 1 file still loads, that the serializer refuses a table its own loader would refuse, and the pairing by id.
- **The frame crate.** The seven test files of `crates/frame-io` check the sizes and limits of a geometry, reading and writing whole frames with the partial frame error, every pattern at every depth, the sample accessors, the comparer on equal and differing streams with negative zero and NaN payloads, the two binaries with their exit codes, and the binaries against `docs/data/frame_tools_interface.json`. Their frames are invented.
- **The stage crate.** `crates/stage-cli` has unit tests in its modules and `tests/cli.rs`, which runs the built `mocha-stage`: the manifest, every argument rule, the copied frames at the three depths, the partial frame, and each exit code.
- **The generic crates.** Each of the seven has test files of its own, on made up data with hand computed expectations. `crates/pixel-format/tests/convert.rs`, `alpha.rs`, `config.rs` and `layout.rs` sweep every integer input of the three depths under each rounding and precision, the midpoints and maxima, NaN payloads, infinities, negative zero and subnormals under each clamp, the premultiply ties, the 240 enumerated choices with canonical text that parses back, and the strides, origins and overflow of the layout. `crates/contour-eval/tests/vectors.rs`, `bezier.rs`, `xspline.rs`, `flatten.rs`, `errors.rs`, `config.rs` and `audit_segment.rs`, with the unit tests of `src/flatten.rs`, check vectors written as bit patterns and computed in exact rational arithmetic, the Bezier and X-spline properties, the three flattening rules, every refusal and the 768 configurations. `crates/coverage-raster/tests/exact_area.rs`, `methods.rs`, `sampling.rs`, `edge_cases.rs`, `combine_quant.rs`, `reference.rs` and `audit_regress.rs`, with the unit tests of `src/exact.rs`, check rectangles, triangles, holes, self intersection and either orientation under both fill rules, the sample methods and the tie rule, degenerate and out of range input, the combine and quantise rules, an independent supersampling reference and the 7616 configurations. `crates/planar-xform/tests/core.rs`, `solve.rs`, `pixel.rs` and `audit_fix.rs` check composition, the inverse, the divisor guards, the four point solve against an independent solver in rational arithmetic, the pixel conversions and every configuration choice. `crates/resample/tests/filters.rs`, `kernels.rs`, `borders.rs`, `alpha_config.rs`, `reference.rs` and `audit_round1.rs` check kernel values worked by hand, the sinc error bound, every border rule, identity mappings for each filter, translations and scaling, the tie rules, the box footprints, alpha, normalisation, the configuration space and an independent slow reference. `crates/lens-model/tests/model.rs`, `config.rs`, `resample.rs` and `audit_gaps.rs` check the maps on dyadic inputs, the inverses against an independent bisection and the closed form of the division model, the conversions and the mapped bounds. `crates/soft-edge/tests/kernels.rs`, `blur.rs`, `distance.rs`, `feather.rs`, `border_cfg.rs` and `audit_regress.rs` check hand computed kernels, single lit pixels, half plane edges, the distance transform against brute force, each ramp profile and direction, the border rules and the refusal of a size that does not fit. No test here involves the original.
- **The shape crates.** `shape-model` has no test file of its own; the four test files of `shape-canon` exercise both crates. `crates/shape-canon/tests/canon.rs` checks the hexadecimal text of known values (`hex_known_values`), that it gives back the bits of zeros of both signs, normal and subnormal values, the largest value and an infinity (`hex_round_trips_bits`), and that other spellings are refused (`hex_rejects_noncanonical`), and that an exponent written -0 is read like +0 and never written (`hex_reads_minus_zero_exponent`); that `Contour::new` refuses bad keys (`model_validates_keys`); the hold before, at and after the keys and the blend halfway between two keys, with the earlier key's flag and missing optional number kept (`interpolation_holds_and_lines`); the rectangle of three points (`bounds_span_anchors`); the whole dump of the case of [the case form](#the-case-form), byte for byte (`predict_exact_bytes`); the empty meta section (`meta_off_leaves_section_empty`); and the refusals of the case form (`case_parse_errors`). `crates/shape-canon/tests/model_hypotheses.rs` pins the hypotheses more tightly: the blend at a quarter and at three quarters of a segment, and the rounding of `a + (b - a) * u` on a span that is not a power of two, to the last bit (`quarter_point_not_symmetric`); the choice of segment among three keys (`three_keys_pick_segment`); the NaN query time (`nan_query_holds_first`); a negative zero at the first key and at an inner key, kept (`negative_zero_at_inner_key_kept`); both flags held from the earlier key (`flags_hold_earlier_key`); the sign of a zero extreme, and of a zero width and height, when extremes tie (`bounds_signed_zero`); and the text of a negative zero query time (`negative_zero_query_time_text`). Comments in `quarter_point_not_symmetric` and `bounds_signed_zero` say that the rounding form and the tie rule they pin are hypotheses of the crate that have not been compared with the original. `crates/shape-canon/tests/cli.rs` runs the binary as a child process on invented case files that it writes into a fresh folder under the system temporary directory, and checks the exit codes and the standard error forms of [`shape-predict`](#shape-predict): exit 0 with the dump that `predict` gives for the same case on standard output and nothing on standard error (`valid_case_exit_0_dump_on_stdout`), and the same output when a second argument is given (`second_argument_is_ignored`); exit 1 with `error: line ` for a bad line (`bad_line_exit_1`), and with `error: ` but not `error: line ` for a case that `Contour::new` refuses (`refused_contour_exit_1`); exit 2 with exactly the usage line when no file is named (`no_argument_exit_2_usage`), and with `read failed: ` for a file that does not exist (`missing_file_exit_2`) or is not UTF-8 (`non_utf8_file_exit_2`); every refused run leaves standard output empty, and the exit code 1 for a standard output that cannot be written is not exercised. `crates/shape-canon/tests/interface_contract.rs` holds the [interface file](#shape-predict) to the binary. It reads the file as plain text and, for each of the six exit code rows, makes the run that the row describes, in the same way as `tests/cli.rs` and with invented input, and checks the exit code, the standard error line (empty, its beginning or the whole line, as the row says) and that standard output is empty on every refused run and not on the valid one (`exit_code_rows_match_the_binary`); it also checks that the file lists the exit codes 0, 1 and 2 and no other and names the dump form `shape-1`, and that the dump of a valid case begins with `format=shape-1` (`only_documented_exit_codes_and_dump_name`). No test here involves the original.
- **The mask sequence crate.** Three of the six test files of `mask-seq`, `cli.rs`, `validate.rs` and `interface_contract.rs`, share the helpers of `crates/mask-seq/tests/common/mod.rs`, which write an invented sequence of three frames, 4 by 2 at depth 8, into a fresh folder under the system temporary directory and change it as each run of the interface file needs; the other three work on bytes in memory (`manifest.rs` also reads this page), and no frame file is committed. `crates/mask-seq/tests/sha256.rs` checks the hash against six published test vectors of the NIST Cryptographic Algorithm Validation Program: five messages of 1, 55, 56, 63 and 64 bytes, at the edges of the padding, from `SHA256ShortMsg.rsp` (`cavp_short_messages`), and one of 163 bytes, three blocks, from `SHA256LongMsg.rsp` (`cavp_long_message`). It also checks that a message fed in two parts at every split point, for every length up to 130 bytes, or in chunks of five sizes, gives the hash of the whole (`split_updates_equal_one_shot`); four of the computed constants against the values FIPS 180-4 gives (`derived_constants_spot_values`); and the hexadecimal text, with its refusals of 63 and 65 digits, upper case and other characters (`hex_text`). `crates/mask-seq/tests/pgm.rs` checks the round trip of frames at depth 8 and 16, with the exact bytes of one 16-bit frame (`round_trip_8_bit`, `round_trip_16_bit`); that bytes and frames come back unchanged through each other, sides of 65535 included (`canonical_bytes_identity`); 26 headers that the reader refuses (`refuses_noncanonical_headers`); a file one byte short or one byte long, a header with no samples and 16-bit samples with an odd byte count (`refuses_truncated_and_trailing`); the checks of `Frame::new` (`frame_new_checks`); and the values of the two depths (`depth_values`). `crates/mask-seq/tests/manifest.rs` checks that the two invented frames of [the mask sequence form](#the-mask-sequence-form) give exactly its example manifest (`example_exact_bytes`); reads that example from this page and checks that it is the same text and that `Manifest::parse` accepts it (`reference_example_is_canonical`); checks the round trip of a manifest of the last two indexes at depth 16 and a width of 65535 (`round_trip`), 17 refusals of the form, each with its line (`refuses_syntax`), 16 values out of range or naming another form, each with its kind and line (`refuses_format_and_values`), and that the first fault in line order wins, in `Manifest::new` as well (`first_fault_in_line_order`). `crates/mask-seq/tests/validate.rs` runs the library's `validate` on folders it writes: every reason word but `frame_read`, with the detail of its interface row (`each_reason_word`); the same `ok` line for a folder and for its manifest path (`valid_folder_and_manifest_path`); a path that does not exist, which cannot be read (`missing_path_is_unreadable`); four pairs of faults, of which the earlier check wins (`first_fault_wins`); other files ignored, and the smallest extra frame name reported (`extra_frames_sorted_other_files_ignored`); 16-bit frames whose indexes end at 999999 (`nonzero_first_and_16_bit`); the refusals of `write_sequence`, which leave no manifest behind, and the manifest it returns, equal to the one it writes (`write_sequence_refusals`); and, running the binary as well, with exit 1, empty standard output and exactly the `error:` line: a frame that differs from the manifest in its width only and one that differs in its height only (`size_fault_in_one_side_only`), a frame file name below the first index and one with seven digits, each extra (`frame_named_files_outside_the_range_are_extra`), and a folder in place of a frame file, which counts as missing (`folder_in_place_of_a_frame_is_missing`). `crates/mask-seq/tests/cli.rs` runs the binary as a child process and checks exit 0 with the library's `ok` line on standard output and nothing on standard error, for a folder and for its manifest path (`valid_folder_exit_0`, `valid_manifest_path_exit_0`); exit 1 with the library's `error:` line, alone, on standard error for a frame whose hash differs (`invalid_exit_1_one_line`); exit 2 with exactly the usage line for no argument and for two (`no_argument_exit_2_usage`, `two_arguments_exit_2_usage`), and with `read failed: ` for a path that does not exist (`missing_path_exit_2`); standard output stays empty on every refused run. `crates/mask-seq/tests/interface_contract.rs` holds the [interface file](#mask-validate) to the binary and the library: it makes each of the 17 runs of the file and checks the exit code and both streams (`exit_code_rows_equal_the_binary`), checks the reason words against `Reason::ALL`, with the exit code and the detail of each, and the details that `error_line` names for the four manifest reasons (`reason_words_equal_the_library`), and checks the exit codes and the three names (`documented_codes_and_forms`). The exit paths of `frame_read`, of a manifest that exists but cannot be read and of a standard output that cannot be written are not exercised. No test here involves the original.
- **The sphere crate.** The five test files of `crates/sphere-reorient` check exact values for the identity, quarter and half turns, the seam columns and the pole rows (`equirect.rs`); the rotations, their products, transposes and determinants (`rotation.rs`); the cube and stereo layouts, every cube layout of `CubeConfig::all()` included (`layouts.rs`); the series of `trig` against the platform functions, inside the tests only, and the symmetry of sine and cosine for negative angles (`trig.rs`, `audit_reorient.rs`); round trips over every configuration of `Config::all()`; and pinned bit patterns. `audit_reorient.rs` also checks that the angles of a direction do not change with its length. Their values are invented.
- **The mesh crate.** `crates/mesh-warp/tests/warp.rs` checks values worked by hand (the identity, an integer translation, one affine triangle), the tie rules on shared edges and vertices under each rule against an integer reference of the ownership rule, the outside rules, the validation of a mesh, a slow barycentric reference, and the same results on a second run; `tests/quad.rs` checks the quad patch at its corners and centre and its inverse against a Newton reference; `tests/audit.rs` checks that a point on a segment shared by two triangles is found in one of them, the inverse of a quad that is a parallelogram up to rounding, the order of operations of the two barycentric forms and the left to right sum of the interpolation, the location and listing of triangles on non square fixed grids, that the linear threshold of the quad is inclusive, and the choice between two roots when a point lies outside the unit square. Their meshes are invented.
- **The blend crate.** The six test files of `crates/blend-ops` check the twelve operators of Porter and Duff (`pd.rs`); every mode with source and backdrop at 0 and 1, with pinned bit patterns, and the two soft light functions (`corners.rs`); the guard values of dodge and burn and the order of their tests (`guards.rs`); the configuration space, the two opacity points and the two forms (`config.rs`); values against a reference written from the W3C formulas, and `lum`, `sat`, `set_lum`, `set_sat` and `clip_color` on values worked by hand (`reference.rs`); and asymmetric alphas, signed zeros, values outside the unit range and the matte of a buffer (`extra.rs`). Their pixels are invented.
- **The key crate.** The five test files of `crates/key-eval` check every key kind at a quarter, a half and three quarters of a two key segment with values worked by hand in rational arithmetic (`scalar.rs`, `bezier.rs`); the solvers and the cube root, the closed form near a vanishing cubic coefficient included, against slow references (`solve.rs`); every rule before the first key and after the last (`outside.rs`); and the configuration space, with a test that walks all 144000 configurations and compares a pinned digest (`config.rs`). Their keys are invented.
- **The tracking crate.** The six test files of `crates/planar-track` check the images, the sampler, the gradients, the pyramid, the motion models and the distance transform against brute force on small fixed images (`primitives.rs`); single steps of each update rule by exact equality with values worked by hand (`steps.rs`); the normal equations of every rule and model against independent references and the damped solve against exact rational elimination (`audit_steps.rs`); the order in which each rule composes its step, the acceptance tests, the residual and the correlation search (`audit_kill.rs`); the tracking of an invented textured plane rendered through known homographies, with the corners of the region within 0.15 pixels of the truth for each model and each rule, changes of brightness taken up by the illumination model, a large motion found by the search, and a lost track repaired by the correction functions (`tracking.rs`); and the forward additive rule on pyramid levels of unequal horizontal and vertical scale, and a corrected pose that is not extrapolated from a tracked one (`audit_fixes.rs`). The texture is generated by the tests and is the same on every run.
- **The tools.** Each tool has a test file beside it, whose fixtures are invented and built in temporary directories. `tools/test_status.py` checks the status rule on invented result files: every status and rung label, the case floors, the survivor rule, the invalid files, the re-run states, the totals per rung, an empty call-record check and a result file with no rung, that no forbidden wording is printed, and the exit codes 0, 2 and 3. `tools/test_gen_docs.py` checks that `--check` passes on the repository, and on a scratch copy of the pages that a hand-edited figure or block is found, that write mode repairs it, that unknown keys are counted, and the exit codes 1, 2 and 3. `tools/test_check_docs.py` checks the two term list checks on a scratch repository with an invented term list: the NOT RUN line, the exit code 2 cases, findings in a page, in a code file and inside a code span, the prose-only rows, the option against the environment variable, and that a finding never prints the matched text; and it checks `hexaddr` and `secrets`. One of its tests reads the term list named by `MOCHA_PRIVATE_TERMS` and checks the shipped tools against it; with no list named it is skipped.
- **The pages.** `tools/gen_docs.py --check` exits 1 on a stale figure, and `tools/check_docs.py` on a page or a text file that breaks a rule.
- **Continuous integration.** `.github/workflows/ci.yml` runs the commands above on every push to `main`, on every pull request, and when started by hand. The test job runs on Linux, macOS and Windows with the stable Rust toolchain and Python 3.12: `cargo build --locked --all-targets` first, then every other command above except formatting and lints. Each of those steps runs even when an earlier one failed (the tests of the crates only once the build has passed), so a single run shows every failing step; the job has a 30 minute limit. The lint job runs `cargo fmt --all -- --check` and `cargo clippy --locked --all-targets -- -D warnings` on Linux, with a 15 minute limit. Every action is pinned to a full commit SHA, with the version it was resolved from in a comment. In CI no term list is named, so `check_docs.py` prints NOT RUN for its two list checks. A new push to a pull request cancels its run in flight; each push to `main` keeps a run of its own. The workflow does not run the convention's conformance tool.

## Documentation

This file specifies what the repository holds. The other files each cover one concern.

| File | Specifies |
|---|---|
| [`README.md`](../README.md) | What the project is, the generated status, how the verification runs, the limits, what is left out, the legal notes. |
| [`docs/parity.md`](parity.md) | The definition and the measurement of parity: the bar, the harness, the pairing of the interface and the calibration, grids and the contract, call records, mutants, stand-ins, the specifications written from static reads, the status rule, the wording, the limits of a result, and the generated status, pairing and notes tables. |
| [`docs/reproduce.md`](reproduce.md) | The checks that need only this repository, the inputs you bring, and the steps that produce the evidence again. |
| [`docs/internals/README.md`](internals/README.md) | How the crates are put together, the rules their code keeps, the checks that cover them, and what a change has to leave passing. |
| [`docs/internals/shape-crates.md`](internals/shape-crates.md) | The two shape crates in detail: what they compute, the hypotheses with the code and the tests behind each, and what a change to them has to keep. |
| [`docs/handoff.md`](handoff.md) | Where the repository stands against the handoff convention of twixtor and sapphire: the stage program and its self test stage, the frame tools, the file forms that can be exchanged, the exit codes, the steps of joining and the known gaps. |
| `docs/data/result_schema.json` | The fields of a result file. |
| `docs/data/shape_predict_interface.json` | The arguments, streams, dump form name and exit codes of `shape-predict`, in a machine-readable form for programs that run it. |
| `docs/data/mask_validate_interface.json` | The argument, streams, names, reason words and exit codes of `mask-validate`, in a machine-readable form for programs that run it. |
| `docs/data/frame_tools_interface.json` | The arguments, streams, patterns and exit codes of `frame-gen` and `frame-cmp`, in a machine-readable form for programs that run them. |
| `contracts/` | The schemas, the tool registry and the conformance cases of the handoff convention, held byte for byte as twixtor publishes them at commit 8087d06. |
| `docs/data/facts.json` | Every generated figure with its value and source, written by `tools/gen_docs.py`. |
