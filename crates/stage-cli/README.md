# stage-cli

`mocha-stage`: a generic frame stage program for the handoff convention (how ffmpeg, twixtor and sapphire pass raw frames to each other: `docs/handoff.md` of the twixtor repository, with the contracts in its `contracts/` folder).

This is step 1 of "How a third plugin joins". The program offers `selftest-copy`, a self test that copies frames unchanged and reimplements nothing, and thirteen generic stages. A generic stage computes a published method under explicit configuration with a crate of this workspace, carries the family `generic` and a description that ends with the sentence that it is not compared with any product, and claims nothing about any plug-in. A stage that reproduces the plug-in is added only after its unit has been compared, and then carries that unit's status; the provider has none, and a test refuses one.

## Stages

| Stage | Module | Crates | Kind | Geometry | Parameters | Layers |
|---|---|---|---|---|---|---|
| `selftest-copy` | `src/stages/selftest_copy.rs` | none | `frame` | `preserve` | 0 (0 keyed) | none |
| `resample-scale` | `src/stages/image/resample_scale.rs` | resample | `frame` | `declared` | 16 (2 keyed) | `keys` (`keys-1`) |
| `depth-quantise` | `src/stages/image/depth_quantise.rs` | pixel-format | `frame` | `preserve` | 5 (0 keyed) | none |
| `alpha-premultiply` | `src/stages/image/alpha_premultiply.rs` | pixel-format | `frame` | `preserve` | 6 (0 keyed) | none |
| `soft-edge-blur` | `src/stages/image/soft_edge_blur.rs` | soft-edge | `frame` | `preserve` | 11 (1 keyed) | `keys` (`keys-1`) |
| `soft-edge-feather` | `src/stages/image/soft_edge_feather.rs` | soft-edge | `frame` | `preserve` | 6 (1 keyed) | `keys` (`keys-1`) |
| `warp-planar` | `src/stages/warp/warp_planar.rs` | planar-xform, resample | `frame` | `declared` | 32 (9 keyed) | `keys` (`keys-1`) |
| `warp-four-point` | `src/stages/warp/warp_four_point.rs` | planar-xform, resample | `frame` | `declared` | 31 (8 keyed) | `keys` (`keys-1`) |
| `warp-lens` | `src/stages/warp/warp_lens.rs` | lens-model, resample | `frame` | `declared` | 32 (8 keyed) | `keys` (`keys-1`) |
| `warp-sphere` | `src/stages/warp/warp_sphere.rs` | sphere-reorient, resample | `frame` | `declared` | 28 (3 keyed) | `keys` (`keys-1`) |
| `warp-mesh` | `src/stages/warp/warp_mesh.rs` | mesh-warp, resample | `frame` | `declared` | 22 (0 keyed) | `mesh` (`mesh-1`) |
| `raster-polygon` | `src/stages/raster/raster_polygon.rs` | coverage-raster | `frame` | `preserve` | 15 (2 keyed) | `shape` (`rings-1`), `keys` (`keys-1`) |
| `raster-contour` | `src/stages/raster/raster_contour.rs` | contour-eval, coverage-raster | `frame` | `preserve` | 29 (2 keyed) | `shape` (`contours-1`), `keys` (`keys-1`) |
| `track-planar` | `src/stages/data/track_planar.rs` | planar-track | `temporal` | `preserve` | 48 (0 keyed) | none |

Every stage takes depths 8, 16 and 32 and keeps the depth of the stream; spool is `false` for every stage; every parameter has a default and every layer is optional. `track-planar` is `temporal`: it writes every frame as it was read and writes its poses to the `keys-1` file named by its text parameter `data`, and a table to the file named by `report`. The parameters, layers, layer forms and exit codes of every stage are in `docs/reference.md` (section `mocha-stage`) and in the interface files `docs/data/stage_<id>_interface.json`.

A stage is one module under `src/stages/` that implements `registry::Stage` with a `StageDef`, and one line in the list of its family module (`image`, `warp`, `raster`, `data`). The manifest entry, the parameter and layer checks and the refusals are generated from the `StageDef`.

## Commands

| Command | Does |
|---|---|
| `mocha-stage list [--json]` | `--json` prints the stage manifest (one line of JSON, `contracts/stage-manifest.schema.json`); without it one stage id per line |
| `mocha-stage apply STAGE --width W --height H --depth 8\|16\|32 --fps N[/D] [--frames N] [--param K=V]... [--layer ID=PATH]...` | reads whole raw frames on standard input and writes the frames of the stage on standard output |
| `mocha-stage apply STAGE --describe --width W --height H --depth D --fps N[/D] [--frames N] [--param K=V]... [--layer ID=PATH]...` | prints the geometry answer (`contracts/describe.schema.json`) and reads no input |
| `mocha-stage provider [--program PATH]` | prints the manifest as a provider file (`*.stage.json`); `--program` names the program the file points at (default `mocha-stage`, found on `PATH`) |
| `mocha-stage --version`, `--help` | `mocha-stage 0.1.0` (the manifest's provider version), the usage text |

`--width`, `--height`, `--depth` and `--fps` are required; `--opt=value` is accepted next to `--opt value`. `--frames N` is the expected frame count: another count is `bad_input`. `--param K=V` and `--layer ID=PATH` take the ids the manifest lists for the stage, each at most once.

The manifest has the provider `mocha_pro` (command `["mocha-stage"]`, version `mocha-stage 0.1.0`) and the fourteen stages of the table above, in that order. The entry of `selftest-copy` is kind `frame`, spool `false`, geometry `preserve`, depths `8, 16, 32`, no parameters, no layers, family `selftest`, with a name and a description that says it is a self test that copies frames unchanged and reimplements nothing. The provider file `providers/mocha_pro.stage.json` is the output of `mocha-stage provider`; the test `the_shipped_provider_file_is_what_the_program_prints` holds the two equal.

The provider id is `mocha_pro`, not `mocha-pro`: `stage-manifest.schema.json` requires `^[a-z][a-z0-9_]{0,31}$` (and `tool.schema.json` says "Lower case, no hyphen"), so a hyphenated id does not validate. The id is one constant, `PROVIDER_ID` in `src/stage.rs`.

## Exits and errors

| Exit | Code | When here |
|---|---|---|
| 0 | none | every input frame was written (an empty input is zero frames), or the list, the provider file or the description was printed |
| 1 | `stage_failed` | a write failed for a reason other than a closed pipe; a file that `track-planar` writes could not be created, written or moved into place |
| 2 | `usage` | a bad or missing flag, a malformed number or rate, a repeated option, a parameter or a layer given twice, a layer with an empty path, one path given for both `data` and `report` (`track-planar`), an input frame above 4 GiB, a terminal on standard input or output |
| 3 | `unsupported` | an unknown stage, a depth that is a whole number but not 8, 16 or 32, a parameter or a layer the stage does not have, a value outside its range or choices, the keyed form of the command line (`@`), a key the stage cannot take, a configuration the crate refuses (before the first frame, or at the frame of a key), an output frame that is empty or above 4 GiB |
| 4 | `bad_input` | a read failed, `--frames` differs from the count read, a layer file cannot be read or does not follow its form, the reference frame of `track-planar` is not among the frames read |
| 4 | `partial_frame` | the input ends inside a frame: the whole frames before it stay written, the partial one is not |
| 4 | `broken_pipe` | the reader of standard output went away |

Every error is one line on standard error in the convention's text form `error: <code>: <message>`; control characters in the message (a stage name with a line break, for instance) become spaces so it stays one line. Standard output carries nothing on a failure before the first frame. Checks run in this order: command line form (2), stage (3), depth (3), parameters, then layers, each pair in command line order (3 for an unknown id or a refused value, 2 for an id met a second time or an empty layer path), input frame size (2), the plan of the stage with its output size (3); then, without `--describe`, a terminal on standard input or output (2), the layer files (4) and what the crate refuses before the first frame (3).

## `--describe` for a preserve stage

The convention does not reserve `--describe` for declared geometry; it defines what a `preserve` stage's output is. The sentences, from `docs/handoff.md`:

- Geometry table: "`preserve` | width, height, fps and pixel format of the output equal the input; the frame count may change only for a temporal stage that says so through its parameters"
- Geometry table: "`declared` | frames/2: the stage answers `apply STAGE --describe --width W --height H --depth D --fps N/D [--frames N] [--param K=V]...` without reading standard input, with its output width, height, depth, fps and frame count"
- `describe.schema.json`: "A stage with geometry declared answers `apply STAGE --describe ...` with this document on standard output, reads no standard input and exits 0".

Nothing says a preserve stage must refuse the flag. This program answers it, as sapphire does for its own preserve self test stage: the answer is the request (width, height, depth, the rate exactly as given, `frames` as given or `null`), because that is what "equal the input" means. The same refusals as `apply` apply up to the plan of the stage (unknown stage, unsupported depth, parameter: exit 3; bad form: exit 2); `--describe` opens no layer file. A `declared` stage answers the output size it computes from its parameters. If a later reading of the convention reserves the flag, the change is one branch in `execute` in `src/lib.rs`.

## Frame contract (duplicated)

`src/frame.rs` holds the frame sizes of the convention: depth 8 `rgba` 4 bytes per pixel, depth 16 `rgba64le` 8 bytes, depth 32 `gbrapf32le` 16 bytes (four planes G, B, R, A); whole frames back to back, no header, no padding, a frame at most 4 GiB. The frame-io crate holds the same sizes; the code that repeats them here is `src/frame.rs` (`DEPTHS`, `bytes_per_pixel`, `frame_bytes`, `MAX_FRAME_BYTES`) and the frame loop `copy_frames` in `src/run.rs`. The self test needs no pixel layout beyond the frame size; the generic stages read and write samples through `src/pixels.rs` (an integer sample over its full scale as f32, and back with a clamp and a rounding half up; depth 32 as the stream's floats), except `depth-quantise` and `alpha-premultiply`, which work on the bit patterns through `pixel-format`.

## Platforms

- Standard input and output are raw bytes: the Rust standard library does no newline translation on any platform, so the bytes `0x0a`, `0x0d` and `0x1a` pass unchanged on Windows (tested with every byte value, in a test that CI runs on Linux, macOS and Windows; the program also compiles and lints clean for `x86_64-pc-windows-gnu` on Linux).
- A closed reader is a write error, not a signal: Rust binaries ignore SIGPIPE on Unix, and a closed pipe is `BrokenPipe` on Windows too. The program exits 4 with `error: broken_pipe: ...` (tested on Linux).
- A terminal on standard input or output is refused (exit 2) instead of read as text; a Windows console read would otherwise insist on UTF-8.
- Arguments must be valid UTF-8 text (exit 2 otherwise). No platform specific code, no unsafe code, and no dependency outside the workspace.
- Standard output goes through the standard library's line buffered writer; bytes are not changed by it, whole frames are written with one `write_all` each.

## Vendoring

The convention asks a joining tool to hold `contracts/` and `tools/conformance.py` byte for byte. The repository holds `contracts/` as twixtor publishes it at commit 8087d06 and edits nothing in it. It does not hold `tools/conformance.py`. It adds no entry to the registry `contracts/tool.json`: a tool that only provides stages may skip that step and be found through a provider file.

## Checks

- `cargo test -p stage-cli`: 192 unit tests, 34 tests of the built binary (`tests/cli.rs`), 19 of the registry and the shared rules (`tests/framework.rs`), 209 in the twelve `tests/stage_<id>.rs` files, 3 in `tests/stage_raster_interface.rs` and 35 in the eleven `tests/interface_<id>.rs` files, 492 in all, on made up frames, keys, shapes and meshes.
- The convention's own tool, run from a twixtor checkout at commit 8087d06: `python3 -I tools/conformance.py --stage "<path>/mocha-stage apply STAGE"` for each stage (level L1 stage matrix and L3 failure injection; the other levels test wrappers and peers). It passes for every stage at its defaults (L1 270 passed, L3 33 passed).
