# Handoff

[twixtor](https://github.com/baairon/twixtor) and [sapphire](https://github.com/baairon/sapphire) share a handoff convention: a raw frame contract, stage commands that read frames on standard input and write them on standard output, a registry by which the tools find each other, and a conformance suite that every stage has to pass. Its definition is [twixtor's `docs/handoff.md`](https://github.com/baairon/twixtor/blob/main/docs/handoff.md), which both repositories hold byte for byte.

This page states where this repository stands against that convention. In one sentence: it is a stage provider with a self test stage and generic stages. `mocha-stage` prints a stage manifest and runs `selftest-copy`, which copies frames unchanged, so that the plumbing between the tools can be checked, and thirteen generic stages, each of which computes a published method under explicit configuration with a crate of this repository. A generic stage is not compared with any product and claims nothing about the plug-in. No stage reimplements any output of the plug-in. Beside the stage program there are two frame tools, a set of file forms, and two command line programs with exit codes of their own.

## Contents

- [The stage program](#the-stage-program)
- [Layer forms](#layer-forms)
- [Frames](#frames)
- [The frame tools](#the-frame-tools)
- [What joining takes](#what-joining-takes)
- [Finding the provider](#finding-the-provider)
- [What can be exchanged as files](#what-can-be-exchanged-as-files)
- [The two file programs](#the-two-file-programs)
- [Masks](#masks)
- [Known gaps](#known-gaps)
- [Platforms](#platforms)
- [Not offered](#not-offered)

## The stage program

`crates/stage-cli` builds `mocha-stage`. It speaks contract version `frames/2`.

| Command | Does |
|---|---|
| `mocha-stage list --json` | prints the stage manifest, one line of JSON that validates against `contracts/stage-manifest.schema.json` |
| `mocha-stage list` | prints one stage id per line |
| `mocha-stage apply STAGE --width W --height H --depth 8\|16\|32 --fps N[/D] [--frames N] [--param K=V]... [--layer ID=PATH]...` | reads whole raw frames on standard input and writes the frames of the stage on standard output |
| the same with `--describe` | prints the geometry answer of `contracts/describe.schema.json` and reads no input |
| `mocha-stage provider [--program PATH]` | prints the manifest as a provider file |

The manifest names the provider `mocha_pro` (the schema's pattern for an id allows no hyphen) and these stages. The `family` key of each entry is `selftest` or `generic`:

| Stage | Family | Kind | Geometry | Depths | Parameters | Layers | What it computes |
|---|---|---|---|---|---|---|---|
| `selftest-copy` | `selftest` | `frame` | `preserve` | 8, 16, 32 | none | none | nothing: each input frame is written as it was read |
| `resample-scale` | `generic` | `frame` | `declared` | 8, 16, 32 | 16; keyed: `shift_x`, `shift_y` | `keys` (`keys-1`, optional) | each frame resampled to the given width and height with a nearest, bilinear, Keys, Mitchell and Netravali, Lanczos or box filter (`resample`) |
| `depth-quantise` | `generic` | `frame` | `preserve` | 8, 16, 32 | 5 | none | every sample snapped to an 8 bit or 16 bit integer grid and converted back; the stream keeps its depth (`pixel-format`) |
| `alpha-premultiply` | `generic` | `frame` | `preserve` | 8, 16, 32 | 6 | none | colour multiplied by alpha or divided by it (`pixel-format`); with `op=premultiply` the output is premultiplied |
| `soft-edge-blur` | `generic` | `frame` | `preserve` | 8, 16, 32 | 11; keyed: `radius` | `keys` (`keys-1`, optional) | the alpha channel, or all four channels, under a box, repeated box or Gaussian blur (`soft-edge`) |
| `soft-edge-feather` | `generic` | `frame` | `preserve` | 8, 16, 32 | 6; keyed: `width` | `keys` (`keys-1`, optional) | alpha replaced by a ramp over the signed distance to the contour where alpha crosses a threshold; colour passes (`soft-edge`) |
| `warp-planar` | `generic` | `frame` | `declared` | 8, 16, 32 | 32; keyed: `m00`, `m01`, `m02`, `m10`, `m11`, `m12`, `m20`, `m21`, `m22` | `keys` (`keys-1`, optional) | each frame under a 3 by 3 projective matrix (`planar-xform`, `resample`) |
| `warp-four-point` | `generic` | `frame` | `declared` | 8, 16, 32 | 31; keyed: `x0`, `y0`, `x1`, `y1`, `x2`, `y2`, `x3`, `y3` | `keys` (`keys-1`, optional) | each frame under the projective transform that takes the corners of a frame to four points (`planar-xform`, `resample`) |
| `warp-lens` | `generic` | `frame` | `declared` | 8, 16, 32 | 32; keyed: `k1`, `k2`, `k3`, `p1`, `p2`, `lambda`, `centre_x`, `centre_y` | `keys` (`keys-1`, optional) | a radial, division or squeezed lens distortion applied to each frame or removed from it (`lens-model`, `resample`) |
| `warp-sphere` | `generic` | `frame` | `declared` | 8, 16, 32 | 28; keyed: `yaw`, `pitch`, `roll` | `keys` (`keys-1`, optional) | each full sphere frame turned by yaw, pitch and roll, between equirectangular and cube face layouts, mono or stereo (`sphere-reorient`, `resample`) |
| `warp-mesh` | `generic` | `frame` | `declared` | 8, 16, 32 | 22 | `mesh` (`mesh-1`, optional) | each frame under a triangle mesh with barycentric interpolation inside each triangle (`mesh-warp`, `resample`) |
| `raster-polygon` | `generic` | `frame` | `preserve` | 8, 16, 32 | 15; keyed: `offset_x`, `offset_y` | `shape` (`rings-1`, optional), `keys` (`keys-1`, optional) | the closed polygons of a shape file rasterised to a coverage matte, carried in the frame (`coverage-raster`) |
| `raster-contour` | `generic` | `frame` | `preserve` | 8, 16, 32 | 29; keyed: `offset_x`, `offset_y` | `shape` (`contours-1`, optional), `keys` (`keys-1`, optional) | the Bezier and X-spline contours of a shape file flattened and rasterised to a coverage matte, carried in the frame (`contour-eval`, `coverage-raster`) |
| `track-planar` | `generic` | `temporal` | `preserve` | 8, 16, 32 | 48; files written: `data`, `report` | none | the frames unchanged; a planar region tracked through them, one pose per frame written to a `keys-1` file (`planar-track`) |

Every stage takes depths 8, 16 and 32 and writes the depth it reads. A `declared` stage computes its output width and height from its parameters (`width` and `height`, where 0 keeps the input side) and answers them with `--describe`; no stage changes the frame count. Every parameter has a default and every layer is optional, so each stage runs with no `--param` and no `--layer`: at their defaults `resample-scale`, the two soft edge stages and the five warps write the input frames again, `track-planar` passes them, and the raster stages, with no shape file, write coverage 0 into the alpha channel. A parameter that may be keyed takes a value per frame from the `keys` layer. The [reference](reference.md#the-stages) lists every parameter of every stage with its kind, default, range or choices, and the exit codes with the rows that pin them.

The exit codes are the convention's:

| Exit | Code word | When |
|---|---|---|
| 0 | none | every input frame was written, or the list or the description was printed |
| 1 | `stage_failed` | a write failed for a reason other than a closed pipe, or a file that `track-planar` writes could not be moved into place |
| 2 | `usage` | a bad or missing option, a parameter or a layer given twice, a layer with an empty path, one path given for both `data` and `report` (`track-planar`), a terminal on standard input or output |
| 3 | `unsupported` | an unknown stage, a depth the stage does not take, a parameter or a layer the stage does not have, a value the stage refuses, an output frame above 4 GiB |
| 4 | `bad_input`, `partial_frame`, `broken_pipe` | a failed read, a frame count other than `--frames`, input that ends inside a frame, a reader that went away, a layer file that cannot be read or parsed |

A value the stage refuses is refused before the first frame where the parameters decide it; a value from a key that the crate refuses at a later frame ends the run there with exit 3. An error is one line on standard error, `error: <code>: <message>`. The [reference](reference.md#mocha-stage) and `crates/stage-cli/README.md` give the full behaviour, the order of the checks included. Each generic stage has an interface file, `docs/data/stage_<id with underscores>_interface.json`, with its arguments, streams and exit codes, and a test of the crate runs the built program for every row of it.

A stage that computes something of the plug-in is added only after its unit has been compared with the original under the [parity](parity.md) rules, and it then carries that unit's status. The manifest lists the self test and generic stages only. A generic stage computes a published method under explicit configuration. Its manifest entry says `"family": "generic"`, and its description ends with the sentence that it is not compared with any product. It is not a unit, carries no status and enters no parity figure.

## Layer forms

A layer is a file named on the command line with `--layer ID=PATH`. Every layer of every stage is optional, and an absent layer is the neutral input of the operation. The forms are text files of this repository's own, each named by its first line; the [reference](reference.md#layer-files) specifies them.

| Form | Layer | Read by | Holds | Without the layer |
|---|---|---|---|---|
| `keys-1` | `keys` | the stages with a keyed parameter | keys per parameter, `hold` or `linear`, at times in frames | every keyed parameter keeps its command line value |
| `rings-1` | `shape` | `raster-polygon` | polygons as rings of points, in pixels of the frame | no polygon: coverage 0 |
| `contours-1` | `shape` | `raster-contour` | polygons as Bezier or X-spline contours of control points | no contour: coverage 0 |
| `mesh-1` | `mesh` | `warp-mesh` | one triangle mesh, with a source and an output position per vertex | the identity mesh: every position maps to itself |

`track-planar` reads no layer. Its parameter `data` names the file it writes: a `keys-1` file with one pose per frame as the matrix entries `m00` to `m22`, the keyed parameters of `warp-planar`, which reads the same form. Its parameter `report` names a table with one row per frame. Both files are written whole or not at all, and with neither parameter the stage writes no file. A shape moves as a whole, through the keyed parameters `offset_x` and `offset_y`; no form carries keys per point.

## Frames

The frame contract is the convention's: whole frames back to back, with no header and no padding, in straight alpha.

| Depth | Layout | Bytes per pixel |
|---|---|---|
| 8 | packed `rgba` | 4 |
| 16 | packed `rgba64le` | 8 |
| 32 | planar `gbrapf32le`, the planes in the order G, B, R, A, with 1.0 at full scale | 16 |

A frame is at most 4 GiB. Input that ends inside a frame is a partial frame and an error. `alpha-premultiply` with `op=premultiply` writes premultiplied colour, which is outside the straight alpha of the contract until a later stage divides by alpha again. `crates/frame-io` holds the contract as a library ([reference: the frame contract](reference.md#the-frame-contract)); `crates/stage-cli` repeats the three sizes; each generic stage depends on the crate that computes it.

## The frame tools

`crates/frame-io` builds two programs that are not stages. They exist so that a frame stream can be made and judged with nothing but this repository.

| Program | Does | Exit codes |
|---|---|---|
| `frame-gen --width W --height H --depth D --frames N [--fps N/D] [--pattern SPEC]` | writes N synthetic frames to standard output: a constant, a ramp, a checkerboard, an alpha ramp or a seeded field, the same bytes on every platform | 0 written, 2 usage, 3 unsupported, 4 bad input |
| `frame-cmp --width W --height H --depth D [--fps N/D] [--frames N] [-v] A B` | compares two streams as bit patterns at a tolerance of 0 and names the first difference by frame, pixel and channel | 0 equal, 1 differ, 2 usage, 3 unsupported, 4 bad input |

Exit code 1 of `frame-cmp` means that two readable streams differ. The convention keeps 1 for a stage that failed, so `frame-cmp` must not be registered or read as a stage. A check of the stage program with the two tools:

```sh
frame-gen --width 64 --height 36 --depth 16 --frames 3 --pattern random:7 > in.raw
mocha-stage apply selftest-copy --width 64 --height 36 --depth 16 --fps 24/1 < in.raw > out.raw
frame-cmp --width 64 --height 36 --depth 16 in.raw out.raw
```

The arguments, streams, patterns and exit codes of both programs are in `docs/data/frame_tools_interface.json`, and a test runs the built programs against that file.

## What joining takes

The convention lists five steps for a further tool.

| Step | What the convention asks | State here |
|---|---|---|
| 1 | `list --json` prints a stage manifest, and `apply STAGE` reads frames on standard input and writes frames on standard output with the stage exit codes; `--describe` answers the geometry. | Taken for `selftest-copy`, which reimplements nothing, and for the thirteen generic stages of the table above. |
| 2 | The `contracts/` folder and `tools/conformance.py` are held byte for byte. | Taken in part. `contracts/` is held as twixtor publishes it at commit 8087d06 and is not edited here. `tools/conformance.py` is not held. |
| 3 | One entry in `contracts/tool.json` names the tool, its markers, its binaries and its providers, and `handoff.py sync` gives every peer the same registry. | Not taken. The registry is held as published, with no entry for this repository. A tool that only provides stages may skip this step and be found through a provider file. |
| 4 | `conformance.py --stage` passes for every stage. | Taken for `selftest-copy`: the conformance tool, run from a twixtor checkout at commit 8087d06 against the built `mocha-stage` on Linux with `python -I tools/conformance.py --stage "<path to>/mocha-stage apply selftest-copy" --full` inside the checkout, runs levels 1 and 3 and passes (270 and 33 checks). Taken in the same way for each generic stage at its defaults, and with parameters that make it compute at the same size, for the raster stages with a shape file and for `warp-mesh` with a mesh file (270 and 33 checks each). With `--stage` the tool checks that a stage keeps the byte count of its input, so a stage with declared geometry is run at the parameters that keep the size; what it writes at another size is checked by the tests of `crates/stage-cli`. |
| 5 | `handoff.py stages --json` lists the stages and `handoff.py doctor` reports no difference between peers. | Not taken. This repository does not hold `handoff.py`, and no peer has been run against it. |

With `--stage`, the conformance tool runs level 1 (the stage matrix: depths, sizes, frame counts and alpha modes) and level 3 (failure injection under a deadline, with the error exits of the stage). Levels 0, 2, 4 and 5 need the registry, a peer, the wrappers or ffmpeg, and have not been run here.

## Finding the provider

A peer finds a provider that has no registry entry in one of two ways the convention defines: `--provider FILE`, with a `*.stage.json` manifest, or the `HANDOFF_PROVIDERS` folders. This repository ships `crates/stage-cli/providers/mocha_pro.stage.json`, which is what `mocha-stage provider` prints for a program named `mocha-stage` on `PATH`; `mocha-stage provider --program PATH` prints the file for a program elsewhere. A test holds the shipped file equal to the command.

| Way | What a peer gives |
|---|---|
| `--provider FILE` | the shipped file, with `mocha-stage` on `PATH` |
| `HANDOFF_PROVIDERS` | the folder `crates/stage-cli/providers`, with `mocha-stage` on `PATH`, or a folder that holds the output of `mocha-stage provider --program /absolute/path/to/mocha-stage` |
| a stage command | `mocha-stage apply STAGE` with the placeholders of the convention, passed to a wrapper as `--stage`, `--pre-stage` or `--post-stage` |

No peer has read the file, so discovery from a peer's side stays a [known gap](#known-gaps).

## What can be exchanged as files

Each form is a plain file with a specification in the [reference](reference.md) and tests that pin it. None of them carries a name, a value or a text of the original.

| Form | What it holds | Read or written by | Specified in |
|---|---|---|---|
| `table v1`, `table v2` | A parameter table as text: one row per parameter, six tab separated cells in version 1 (a seventh, the extra cell, is optional) and ten in version 2. Floats are written as bit patterns, an unknown cell as `?`. | The `effect-params` library: loader, serializer and pairing. It has no command line program. | [The table format](reference.md#the-table-format), [The pairing](reference.md#the-pairing) |
| Shape case | A text file that states one shape case: contours, points at keys, and the times to report. | Read by `shape-predict`. | [The case form](reference.md#the-case-form) |
| `shape-1` dump | The predicted report for a case, as text with floats in a canonical hexadecimal form. | Written by `shape-predict`. | [The dump form](reference.md#the-dump-form) |
| `mask-1` sequence | A folder with `manifest.txt` and one binary PGM file per frame, one channel at depth 8 or 16, with a SHA-256 for every frame in the manifest. | Written by the `mask-seq` library, checked by `mask-validate`. | [The mask sequence form](reference.md#the-mask-sequence-form) |
| `keys-1`, `rings-1`, `contours-1`, `mesh-1` | The layer files of the stages: values of keyed parameters per frame, polygons as rings, polygons as contours, a triangle mesh. A `keys-1` file is also what `track-planar` writes. | Read by the stages of `mocha-stage` that name them; `keys-1` written by `track-planar`. | [Layer files](reference.md#layer-files) |
| `track-report-1` | The table `track-planar` writes with one row per frame: the source of the pose, the stop reason, the iterations, the residual, the valid and inlier fractions, the gain and the offset. | Written by `track-planar`. | [`track-planar`](reference.md#track-planar) |
| Raw frame file | A frame stream of the contract above, stored as a file. | Written by `frame-gen`, compared by `frame-cmp`, read and written by `mocha-stage`. | [The frame contract](reference.md#the-frame-contract) |
| Result file | The measured outcome of one unit: rung, judge, comparator, counts, mutants, stand-ins, and for a unit of frames the geometries compared. | Read by `tools/status.py` and `tools/gen_docs.py`. No unit result file exists. | `docs/data/result_schema.json`, [Status of one result file](reference.md#status-of-one-result-file) |

## The two file programs

`shape-predict` and `mask-validate` are not stages. Both take one path, read no standard input, and are deterministic.

| Program | Argument | Standard output | Exit codes |
|---|---|---|---|
| `shape-predict CASE` | a case file | the `shape-1` dump | 0 dump written, 1 case refused, 2 no argument or unreadable file |
| `mask-validate SEQUENCE` | a sequence folder or its `manifest.txt` | one line that begins `ok format=mask-1` | 0 valid, 1 invalid, 2 usage or unreadable argument |

Their codes are their own and are not mapped to the convention's: here 1 means that the input was read and refused, which the convention reports as 4, and 2 also covers an unreadable file. A wrapper must not read an exit code of these two programs as a code of the convention, and neither may be registered as a stage. Every exit code, stream and reason word is in `docs/data/shape_predict_interface.json` and `docs/data/mask_validate_interface.json`, and a test of each crate runs the program against its file.

## Masks

The `mask-1` form is not the frame contract. It holds one channel, one file per frame, each with a PGM header, at depth 8 or 16, with a manifest that names the geometry and a hash for every frame. It is stored on disk and checked as a whole; it is never piped. No converter between `mask-1` and a frame stream is provided. The raster stages carry a one channel matte in a four channel frame by their parameter `carry` (in the alpha channel, in the three colour channels, or in all four), and none of them writes or reads `mask-1`.

## Known gaps

Measured against the convention:

| Gap | State |
|---|---|
| A stage that computes | The thirteen generic stages. No stage computes anything of the plug-in. |
| Parameters and layers | Typed per stage and listed in the manifest, every one with a default; every layer is optional. The layer forms are `keys-1`, `rings-1`, `contours-1` and `mesh-1`. No stage takes a second frame stream as a layer. |
| Keyed parameters | A value per frame comes from a `keys-1` file with `hold` and `linear` keys. The keyed form of the convention's command line (a value with `@`) is refused as unsupported, and no shape form carries keys per point. |
| Declared geometry | `resample-scale` and the five warps declare their output width and height. No stage changes the frame count. A change of depth cannot be declared under the convention, so no stage changes the depth. |
| Discovery | No registry entry and no markers. A provider file is shipped; no peer has read it. |
| Packets and `handoff.py` | Not held and not run. |
| Reports and the stall watch | `mocha-stage` writes no report. The wrappers of the convention have not been run around it. |
| The conformance tool | Not held here, and not run by continuous integration. |
| Conformance levels 0, 2, 4 and 5 | Not run. |
| Conformance with a frame layer | No stage has a layer of frames. Level 1 varies the frame size, which such a layer would have to follow, so the tool checks a stage of that kind at its defaults only, and its own tests check it with the layer file. |
| Crates without a stage | `blend-ops` and `key-eval` are offered as libraries only: no stage composites two frame streams, and the key files are evaluated by the stage program itself. `effect-params`, `shape-model`, `shape-canon`, `mask-seq`, `frame-io`, `eye-layout`, `mod-remove` and `plugin-commands` work on tables, shapes, mask folders, the frame contract, eye layouts, the frames that replace a layer and the decision rules of the plug-in's commands, not as stages. |
| Conformance on Windows | Not run: the conformance tool is run on Linux only. CI builds the stage program and runs its tests on Windows ([Platforms](#platforms)), and the program passes `cargo clippy` for the target `x86_64-pc-windows-gnu` on Linux. |
| ffmpeg | Never used here. |
| A peer | Neither twixtor nor sapphire has been chained with `mocha-stage`. |
| Comparison | No stage is compared with any product. A stage that reproduces the plug-in is added only after its unit has been compared. |

## Platforms

Continuous integration builds the crates and runs their tests and the tests of the three Python tools on Linux, macOS and Windows. It does not run the convention's conformance tool. The tools need Python 3 and its standard library only and run under `python -I`; the convention's conformance tool asks for Python 3.12. No program here starts a child process, so the process group and kill paths that the convention requires of a wrapper have no counterpart.

## Not offered

The licence, activation and machine identity code of the original is never part of this project ([Left out](../README.md#left-out)), and no stage will ever stand in for it.

The interactive interface of the product's standalone editor is not rebuilt either ([Left out](../README.md#left-out)); the capabilities in scope are to be offered as stages.

Not offered by `mocha-stage`: a stage that changes the depth of the stream, a stage that composites a second frame stream, keys per point inside a shape layer, and a stage that reproduces any output of the plug-in.
