# mocha-pro

An independent open source reimplementation of the Mocha Pro 2024 plug-in, written in Rust. Its output is to be judged case by case against the original plug-in running unmodified in an emulator, and the status computed from the result files reads: <!--f:status_line-->method and preparation only: 0 cases compared<!--/f--> ([parity](#parity)). It is not affiliated with or endorsed by any other company, product or library.

The repository holds the method, the tools that compute and check its status, and twenty-one Rust crates. It reimplements no output of the plug-in: its frame stages are a self test that copies frames and generic stages that compute published methods and claim nothing about the plug-in.

## Status

- **Parity.** Unit result files: <!--f:units_total-->0<!--/f-->. The status, one part per rung, since units are never counted across rungs: <!--f:status_line-->method and preparation only: 0 cases compared<!--/f-->. The method is under [Parity](#parity).
- **The original in the emulator.** The original has run in the emulator only as far as the end of its parameter setup, and no render command of it has run in the emulator. [Parity](#parity) states what of it is observed there, and [Limits](#limits) what that observation can show.
- **Pairing and shape data.** `effect-params` holds the rewrite's side of the interface pairing, and `shape-model` and `shape-canon` that of the shape data check; [Parity](#parity) describes both checks.
- **Stages.** `mocha-stage` is the stage program of this repository; [Stages](#stages) lists each of its stages with its family and what it computes.
- **Handoff.** Steps 1 and 4 of joining the handoff convention of twixtor and sapphire are taken, step 2 in part, and steps 3 and 5 are not ([Handoff](#handoff)).
- **Usage.** [Build](#build) builds the programs and [Quick start](#quick-start) runs a stage on synthetic frames; the [reference](docs/reference.md) lists every command, stage, parameter, layer file and exit code.

The figures of the Parity line above and the table below are generated from the unit result files, `parity/<unit>/results/summary.json`, by `tools/gen_docs.py`, with the rule of `tools/status.py`. A result file never states its own status.

<!--gen:units-->
No unit result file exists yet, so no unit has a row. A unit is Not started until its `summary.json` is committed.
<!--/gen:units-->

The statuses Matches and Verified, how to read a row and the label block that heads every unit's record are in [Reading the status](#reading-the-status).

## Contents

- [Status](#status)
- [What it is](#what-it-is)
- [Build](#build)
- [Quick start](#quick-start)
- [Documentation](#documentation)
- [Parity](#parity)
  - [Reading the status](#reading-the-status)
  - [How to run the verification](#how-to-run-the-verification)
  - [Assumptions and stand-ins](#assumptions-and-stand-ins)
- [Stages and crates](#stages-and-crates)
  - [Stages](#stages)
  - [Crate map](#crate-map)
- [Interface](#interface)
  - [Pipeline](#pipeline)
  - [Stage commands](#stage-commands)
  - [Raw frame format](#raw-frame-format)
  - [Layer forms](#layer-forms)
  - [Exit codes](#exit-codes)
  - [Python tools](#python-tools)
- [Handoff](#handoff)
- [Tests](#tests)
  - [Smoke test](#smoke-test)
- [Limits](#limits)
  - [Left out](#left-out)
- [Legal](#legal)
- [License](#license)

## What it is

mocha-pro is meant to rebuild a plug-in from what the original is observed to do and from specifications written from static reads of its code ([Legal](#legal)). Its scope is the plug-in, its render engine and its planar tracker, each capability to be rebuilt as a stage program for command line use; the interactive interface of the product's standalone editor is not rebuilt ([Left out](#left-out)). The aim is output equal to the original's on every compared case, from code of this project's own that contains none of the original's; the planar tracker is an exception, to be compared within a tolerance ([Limits](#limits)).

The judge is the original itself: its unmodified plug-in file runs in a CPU emulator behind a mock host, the same inputs go to it and to the rewrite, and the two outputs are compared with no tolerance (the planar tracker excepted, as stated). Parity is claimed one unit at a time, a unit being one measured part of the plug-in's behaviour, never for the whole product at once. Every status is computed from result files by a tool; none is typed by hand.

What is in the repository, by role ([Crate map](#crate-map) has every crate):

- **The method and its tools.** The repository holds the definition of parity and the rules of a result ([Parity](#parity)), the fields of a result file in `docs/data/result_schema.json` and those of the survey capture comparison file in `docs/data/survey_comparison_schema.json`, and three Python tools: `tools/status.py` computes a unit's status from its result file, `tools/gen_docs.py` writes every parity figure and table of these pages from the result files, and `tools/check_docs.py` checks the pages ([Python tools](#python-tools)).
- **The rewrite's side of the interface pairing and the shape data check.** `crates/effect-params` holds the table format and the pairing for the first step of the method, the interface pairing; `crates/shape-model` and `crates/shape-canon`, with the command `shape-predict`, compute the predicted dump of a shape case for the [shape data check](docs/parity.md#the-shape-data-check).
- **A plug-in shell.** `crates/effect-shell` is the host side shell of an After Effects effect plug-in, with this project's own definitions of the host's structures; it holds no engine.
- **Frames, masks and stages.** `crates/frame-io` holds the raw frame contract of the [handoff convention](docs/handoff.md) with the commands `frame-gen` and `frame-cmp`; `crates/mask-seq` defines `mask-1`, a container form of this repository's own for a sequence of one-channel mask frames, with the command `mask-validate`; and `crates/stage-cli` builds `mocha-stage`, the stage program.
- **Generic crates.** The generic crates compute published methods with every choice held as a field of an explicit configuration ([Crate map](#crate-map)).
- **The documentation** in `docs/` ([Documentation](#documentation)).

Nothing of the original is in it: no code, binary, listing, layout, table or text of the plug-in or of its documentation, and no text or file of the host's SDK. The emulator harness that runs the original, and the inputs it needs, stay outside the repository ([reproduce](docs/reproduce.md)).

## Build

From the repository root, with a stable Rust toolchain:

```sh
cargo build --release
```

This builds every library of the workspace, `effect-shell` also as a dynamic library, and the five binaries of the table below into `target/release/`; the generic crates build no binary. No crate depends on anything outside the workspace, so the build needs no network. `CARGO_TARGET_DIR` moves the build output.

| Binary | Built from | Does |
|---|---|---|
| `mocha-stage` | `crates/stage-cli` | lists its stages, runs one on raw frames, answers `--describe` and prints its provider file |
| `frame-gen` | `crates/frame-io` | writes synthetic frames of the frame contract, the same bytes on every platform |
| `frame-cmp` | `crates/frame-io` | compares two frame streams as bit patterns at a tolerance of 0 and names the first difference |
| `shape-predict` | `crates/shape-canon` | writes the predicted dump of one shape case |
| `mask-validate` | `crates/mask-seq` | checks one `mask-1` sequence and reports the first fault it finds |

The dynamic library of `effect-shell` exports `EffectMain` (on Linux it is `target/release/libeffect_shell.so`); it has not been loaded by a real host. The Python tools need Python 3 and nothing outside its standard library. They run with `python -I`; where only `python3` is installed, run the same commands with `python3` in place of `python`.

## Quick start

After [Build](#build), from the repository root. List the stages, write three synthetic frames, pass them through the self test stage and compare, then ask a generic stage for its output size and run it. Scratch files go to the git ignored `target/demo/`.

```sh
mkdir -p target/demo
target/release/mocha-stage list
target/release/frame-gen --width 64 --height 36 --depth 16 --frames 3 --pattern random:7 > target/demo/in.raw
target/release/mocha-stage apply selftest-copy --width 64 --height 36 --depth 16 --fps 24/1 < target/demo/in.raw > target/demo/out.raw
target/release/frame-cmp --width 64 --height 36 --depth 16 target/demo/in.raw target/demo/out.raw
target/release/mocha-stage apply resample-scale --describe --width 64 --height 36 --depth 16 --fps 24/1 --param width=128 --param height=72
target/release/mocha-stage apply resample-scale --width 64 --height 36 --depth 16 --fps 24/1 --param width=128 --param height=72 --param filter=nearest < target/demo/in.raw > target/demo/wide.raw
```

`mocha-stage list` prints one stage id per line, in the order of the [Stages](#stages) table. `frame-cmp` prints `equal frames=3 width=64 height=36 depth=16` and exits 0: the self test stage wrote the frames it read. The `--describe` command reads no input and prints the geometry answer, `{"ok":true,"contract":"frames/2","stage":"resample-scale","width":128,"height":72,"depth":16,"fps":"24/1","frames":null}`. The last command writes the three frames at 128 by 72 pixels, resampled with the nearest filter. Every stage with its parameters and layers is in the [reference](docs/reference.md#the-stages); [Interface](#interface) gives the forms in short.

## Documentation

| Page | Holds |
|---|---|
| [docs/reference.md](docs/reference.md) | Everything the repository holds: the crate map, the table format and the pairing, `effect-shell`, the shape model with the case and dump forms and `shape-predict`, the mask sequence form and `mask-validate`, the frame contract and the frame tools, `mocha-stage` with every stage, parameter, layer file and exit code, the generic crates with their sources, the build, the documentation tools and the tests. |
| [docs/parity.md](docs/parity.md) | The definition of parity, the way it is measured, the harness that runs the original, the observed registration of the original's parameters, the status rule, the limits of what a result says and the [terms](docs/parity.md#terms) of the method, with the generated status, pairing and notes tables and the counts of the survey capture comparison. |
| [docs/reproduce.md](docs/reproduce.md) | Which checks need nothing but this repository, which inputs you bring yourself, and the steps that produce the evidence again. |
| [docs/handoff.md](docs/handoff.md) | Where the repository stands against the handoff convention that twixtor and sapphire share: the stage program with its self test stage and its generic stages, the layer forms, the frame tools, the file forms a peer can exchange, the exit codes, and which steps of joining are taken. |
| [docs/internals/README.md](docs/internals/README.md) | How the crates are put together, the rules their code keeps, every check with what it should print, and what a change has to leave passing. |
| [docs/internals/shape-crates.md](docs/internals/shape-crates.md) | The two shape crates: what they compute, the hypotheses with the code and the test behind each, and what a change to them has to keep. |
| Crate READMEs | What one crate computes or offers, with its methods, sources and settings, for [`stage-cli`](crates/stage-cli/README.md), [`effect-shell`](crates/effect-shell/README.md), [`pixel-format`](crates/pixel-format/README.md), [`contour-eval`](crates/contour-eval/README.md), [`coverage-raster`](crates/coverage-raster/README.md), [`planar-xform`](crates/planar-xform/README.md), [`resample`](crates/resample/README.md), [`lens-model`](crates/lens-model/README.md), [`soft-edge`](crates/soft-edge/README.md), [`sphere-reorient`](crates/sphere-reorient/README.md), [`mesh-warp`](crates/mesh-warp/README.md), [`blend-ops`](crates/blend-ops/README.md), [`key-eval`](crates/key-eval/README.md), [`planar-track`](crates/planar-track/README.md), [`eye-layout`](crates/eye-layout/README.md) and [`mod-remove`](crates/mod-remove/README.md). |

| File | Holds |
|---|---|
| `docs/data/result_schema.json` | The fields of a result file. |
| `docs/data/survey_comparison_schema.json` | The fields of the survey capture comparison file. |
| `docs/data/facts.json` | Every generated figure with its value and source, written by `tools/gen_docs.py`. |
| `docs/data/stage_<id with underscores>_interface.json` | One file for each generic stage: its arguments, streams, parameters, layers, geometry, sample rule and exit codes, with the rows that pin them. |
| `docs/data/frame_tools_interface.json` | The arguments, streams, patterns and exit codes of `frame-gen` and `frame-cmp`. |
| `docs/data/shape_predict_interface.json` | The arguments, streams, dump form name and exit codes of `shape-predict`. |
| `docs/data/mask_validate_interface.json` | The argument, streams, names, reason words and exit codes of `mask-validate`. |
| `crates/stage-cli/providers/mocha_pro.stage.json` | The provider file: the stage manifest in the form a peer of the convention reads. |
| `contracts/` | The schemas, the tool registry and the conformance cases of the handoff convention, held byte for byte as twixtor publishes them at commit 8087d06. |

For scripts, ask the tool or the file, not the prose: `target/release/mocha-stage list --json` for the stages, `docs/data/facts.json` for every generated figure, `python -I tools/status.py FILE...` for the status of a result file, and the interface files for how a program runs. The [sources of truth](docs/reference.md#sources-of-truth) name the file or command that is right wherever a page disagrees with it.

## Parity

**<!--f:status_line-->method and preparation only: 0 cases compared<!--/f-->.** No parity label is claimed anywhere in this repository until a unit's result file meets the [status rule](docs/parity.md#the-status-rule).

The check follows [twixtor](https://github.com/baairon/twixtor) and [sapphire](https://github.com/baairon/sapphire), where the judge is the original's own code run in an emulator. Here the plug-in file, left as shipped, is loaded into a CPU emulator (Unicorn, driven through Qiling), and a mock host takes the place of After Effects on Windows. The rewrite gets the very inputs the original gets, and the two outputs have to agree exactly (the planar tracker is an exception, [Limits](#limits)): bytes and integers by value, floats as bit patterns at a tolerance of 0, so that -0 and 0 differ. How a NaN is compared is fixed for each unit and written in its result file.

Each unit is checked three ways:

- **The grid:** a test set of no fewer than <!--f:case_floor-->150<!--/f--> in-contract cases, edge values included.
- **The call record:** every request either side makes of its host, compared one call at a time.
- **The mutants:** copies of the rewrite with errors put in on purpose; the grid has to expose each one.

Before any output is compared, the interface is paired. The parameters the original registers with its host are recorded in the emulator and matched field by field with the rewrite's prediction of them, which is to be written and sealed before the registration it is paired with is observed. The original is also booted twice, to show that the same inputs give the same call record and the same dump. Neither step carries a parity label ([parity: the interface pairing](docs/parity.md#the-interface-pairing-and-calibration)).

The original's registration of its parameters is observed. Run in the emulator with its engine, its file unmodified on disk, the original sets itself up and registers its parameters with the mock host, and the harness records them; the record stays outside the repository. That observation comes from one boot on a reduced host whose answers are declared: it is not a result of record and not parity evidence, and a comparison with it carries no parity label. One comparison with it is recorded, a survey capture comparison, never a result of record and never parity evidence; [parity: the survey capture comparison](docs/parity.md#the-survey-capture-comparison) gives its counts and its limits. No result file records a second boot that reaches the end of the parameter setup ([parity: the observed registration](docs/parity.md#the-observed-registration)).

Shape data is prepared like the interface pairing, with a sealed prediction. For each case of the shape data check, the text dump the rewrite predicts for a shape was written and sealed before any run of the original on a shape case and is kept private; the harness is built to have the original report the same shape in the same text, and the two are to be compared byte for byte. No case of it has run ([parity: the shape data check](docs/parity.md#the-shape-data-check)).

Where any result falls short:

- **A grid is a sample.** It holds the inputs someone chose, never all inputs.
- **The reference comes from the emulated original.** The emulator follows the architecture manual, so a reference says nothing about silicon or about After Effects on a real machine. Where the original leans on its host, both sides read the same labelled stand-in ([below](#assumptions-and-stand-ins)).
- **One judge.** A result rests on one emulator unless its result file says that a second engine checked it.

The method, the status rule and the definitions are in [parity](docs/parity.md).

### Reading the status

The table of [Status](#status) gives each unit the status that the [status rule](docs/parity.md#the-status-rule) computes. The two highest statuses are:

- **Matches:** a named rung; all in-contract cases compared, no fewer than <!--f:case_floor-->150<!--/f-->, not one differing at a tolerance of 0; and at least one call record compared, with every compared record equal on the two sides. A result file without a rung, one that holds only an interface pairing and a calibration, is not a parity unit and never matches.
- **Verified** adds the survivor rule: at least ten mutants, at least nine in ten of them caught outright, and every one that survives either caught by an added case or argued equivalent with a named reviewer.

How to read the table, once it has rows:

- **Cases compared** gives two numbers, "X of Y": the cases compared, then the unit's in-contract cases.
- **Rung label** is computed from the same file by the [status rule](docs/parity.md#the-status-rule), and reads none in every case that rule gives none, for example while no case is compared or the tolerance is not 0.
- **Re-run** is the state of an independent re-run on fresh copies: yes, no, stale or none. Only yes counts as re-run verified.

The label block heads every unit's record. Each line is computed from the result files, or says that nothing was measured:

<!--gen:label-->
```text
Status: none (method and preparation only: 0 cases compared)   [computed from 0 unit result files; re-run: none]
Judge: none named; no result file records a compared case
Data: none measured
Identity: none
Route: none
Evidence: 0 cases compared
Estimates: not applicable, nothing compared
Hooks: none
Contract: not applicable, nothing compared
SUSPENDED: none
NOT REBUILT: everything; no unit is rebuilt yet
```
<!--/gen:label-->

The wording allowed for a result is "matches the emulated original", always with the rung, the counts and the judge ([parity: wording](docs/parity.md#wording)).

### How to run the verification

These are the steps a result file records in its `commands` field. They cannot run from this repository alone: the harness and the reference side stay outside it.

1. Build and test the rewrite: `cargo test` ([reference: build](docs/reference.md#build)).
2. Bring the inputs the repository leaves out ([reproduce: inputs you supply](docs/reproduce.md#what-needs-inputs-you-supply)).
   - The original plug-in files come from your own installation of Mocha Pro 2024, and the Windows system libraries the emulator's file system needs come from your own Windows installation. Neither is ever copied here.
   - Adobe does not allow its After Effects SDK, headers included, to be passed on, so get your own copy from Adobe. The harness compiles its own layout files from those headers, and those files never enter the repository.
   - The harness starts from the public harness of [twixtor](https://github.com/baairon/twixtor) (`parity/harness/` at commit 8087d06), on Qiling 1.4.6 and Unicorn 2.1.4, the versions [its README](https://github.com/baairon/twixtor/blob/8087d06ac43023f954de2cc58583b132a3243ff4/parity/harness/README.md#running-it) names, with the changes this plug-in needs. Those changes are not published, because they carry layouts compiled from the SDK and tables made from the original's files.
   - The predicted parameter tables stay private, like every name and value in them. To repeat the pairing, write your own in the [table format](docs/reference.md#the-table-format) and seal it before the registration you pair it with is observed.
3. Pair the interface: boot the original under the harness, read the parameters it registers, and pair them with the predicted table; nothing may stay unmatched on either side. Boot it a second time in a fresh process and compare the two call records and dumps byte for byte.
4. For each unit, run the original over the unit's test set, which gives the reference outputs, then run the rewrite over those cases, recording the host calls of both sides.
5. Compare with a tolerance of 0 and write the unit's result file in the fields of `docs/data/result_schema.json`.
6. Regenerate the tables above with `python -I tools/gen_docs.py`; with `--check` it exits 1 when a page does not match the result files.

### Assumptions and stand-ins

Where After Effects or Windows would provide something the harness cannot, the harness hands one labelled value to both sides: a stand-in. A result that depends on one is logic parity, not parity. Every answer the mock host gives is a row of the harness's host-answer register, with the public documentation it follows and a status: a stand-in until a run measures its effect, or open, in which case the call faults. The harness is not in this repository, so the harness-wide choices below describe its design and cannot be checked from it:

- **Unknown host calls fault.** A call the harness has no answer for stops the run and names itself; none is answered with success or with 0.
- **The emulated CPU** presents a fixed x86-64 feature set without the AVX tiers. A vector instruction outside that set, or a feature query the table does not list, stops the run. The CPU starts in the Windows floating point state, and the main thread runs on 1 MiB of stack, the stand-in value of [twixtor's harness](https://github.com/baairon/twixtor/blob/8087d06ac43023f954de2cc58583b132a3243ff4/parity/harness/README.md#stand-ins) for a host value that is not known.
- **Threads** switch only when one waits, in one reproducible order.
- **Time and randomness** never come from the machine that runs the emulator: a time read stops the run until a fixed clock is registered, and random bytes come from a seeded stream.
- **Host services.** A service on the declared list of absent ones is answered "not found"; a request with no declared answer stops the run.
- **The declared machine.** Questions about the emulated machine's files, folders, environment, registry, locale and desktop are answered from declared models of a fresh machine: declared values, not observations of a real one.

The stand-ins a result rests on are named in its result file and listed in [parity: stand-ins and assumptions](docs/parity.md#stand-ins-and-assumptions).

## Stages and crates

`crates/stage-cli` builds `mocha-stage`, a stage program in the handoff convention of twixtor and sapphire. It speaks contract version `frames/2`, names its provider `mocha_pro` (the schema's pattern for an id allows no hyphen), and ships its manifest as a provider file, `crates/stage-cli/providers/mocha_pro.stage.json`. Each generic stage runs a crate of this workspace; the [reference](docs/reference.md#the-stages) gives every parameter of every stage with its kind, default, range or choices.

### Stages

The Family column gives the `family` key of each manifest entry.

| Stage | Family | Kind, geometry | Layers | What it computes | Crates |
|---|---|---|---|---|---|
| `selftest-copy` | `selftest` | `frame`, `preserve` | none | nothing: each input frame is written as it was read | none |
| `resample-scale` | `generic` | `frame`, `declared` | `keys` | each frame resampled to the given width and height with a nearest, bilinear, Keys, Mitchell and Netravali, Lanczos or box filter | `resample` |
| `depth-quantise` | `generic` | `frame`, `preserve` | none | every sample snapped to an 8 bit or 16 bit integer grid and converted back; the stream keeps its depth | `pixel-format` |
| `alpha-premultiply` | `generic` | `frame`, `preserve` | none | colour multiplied by alpha or divided by it; with `op=premultiply` the output is premultiplied | `pixel-format` |
| `soft-edge-blur` | `generic` | `frame`, `preserve` | `keys` | the alpha channel, or all four channels, under a box, repeated box or Gaussian blur | `soft-edge` |
| `soft-edge-feather` | `generic` | `frame`, `preserve` | `keys` | alpha replaced by a ramp over the signed distance to the contour where alpha crosses a threshold; colour passes | `soft-edge` |
| `warp-planar` | `generic` | `frame`, `declared` | `keys` | each frame under a 3 by 3 projective matrix | `planar-xform`, `resample` |
| `warp-four-point` | `generic` | `frame`, `declared` | `keys` | each frame under the projective transform that takes the corners of a frame to four points | `planar-xform`, `resample` |
| `warp-lens` | `generic` | `frame`, `declared` | `keys` | a radial, division or squeezed lens distortion applied to each frame or removed from it | `lens-model`, `resample` |
| `warp-sphere` | `generic` | `frame`, `declared` | `keys` | each full sphere frame turned by yaw, pitch and roll, between equirectangular and cube face layouts, mono or stereo | `sphere-reorient`, `resample` |
| `warp-mesh` | `generic` | `frame`, `declared` | `mesh` | each frame under a triangle mesh with barycentric interpolation inside each triangle | `mesh-warp`, `resample` |
| `raster-polygon` | `generic` | `frame`, `preserve` | `shape`, `keys` | the closed polygons of a shape file rasterised to a coverage matte, carried in the frame | `coverage-raster` |
| `raster-contour` | `generic` | `frame`, `preserve` | `shape`, `keys` | the Bezier and X-spline contours of a shape file flattened and rasterised to a coverage matte, carried in the frame | `contour-eval`, `coverage-raster` |
| `track-planar` | `generic` | `temporal`, `preserve` | none | the frames unchanged; a planar region tracked through them, one pose per frame written to a `keys-1` file | `planar-track` |

Every stage takes depths 8, 16 and 32 and writes the depth it reads. A `declared` stage computes its output width and height from its parameters (`width` and `height`, where 0 keeps the input side) and answers them with `--describe`; no stage changes the frame count. Every parameter has a default and every layer is optional, so each stage runs with no `--param` and no `--layer`: at their defaults `resample-scale`, the soft edge stages and the warp stages write the input frames again, `track-planar` passes them, and the raster stages, with no shape file, write coverage 0 into the alpha channel.

A generic stage computes a published method under explicit configuration. Its manifest entry says `"family": "generic"`, and its description ends with the sentence that it is not compared with any product. It is not a unit, carries no status and enters no parity figure. A stage that computes something of the plug-in is added only after its unit has been compared with the original under the [parity](docs/parity.md) rules, and it then carries that unit's status.

`blend-ops` and `key-eval` are offered as libraries only: no stage composites two frame streams, and the key files are evaluated by the stage program itself. `effect-params`, `shape-model`, `shape-canon`, `mask-seq`, `frame-io`, `eye-layout` and `mod-remove` work on tables, shapes, mask folders, the frame contract, eye layouts and the frames that replace a layer, not as stages.

### Crate map

Workspace members of `Cargo.toml`: every folder under `crates/`, version 0.1.0, edition 2021, MIT, unpublished. External crates: none. The [reference crate map](docs/reference.md#crate-map) gives each crate's key modules and tests.

| Member | Builds | Holds | Reference |
|---|---|---|---|
| `crates/effect-params` | library | parameter kinds and flags, the table format with its loader and serializer, and the pairing that matches a predicted table with an observed one, by parameter id where both tables carry one and by name otherwise, with defaults, ranges and flags compared as bits | [the pairing](docs/reference.md#the-pairing) |
| `crates/effect-shell` | library, also a dynamic library | the host side shell of an After Effects effect plug-in: the entry point `EffectMain`, a command dispatch that answers ABOUT, GLOBAL_SETUP and PARAMS_SETUP and hands the other commands to a table of handlers that another crate supplies, the registration of a parameter table given at build time, here a made up sample, and this project's own definitions of the host's structures | [`effect-shell`](docs/reference.md#effect-shell) |
| `crates/shape-model` | library | a shape as one contour of Bezier control points per keyframe, with its points and the rectangle around them at any query time ([internals: the hypotheses](docs/internals/shape-crates.md#the-hypotheses)) | [the shape model](docs/reference.md#the-shape-model) |
| `crates/shape-canon` | library, binary `shape-predict` | the case form of a shape case, and the predicted dump in a canonical text form, every float in a hexadecimal text that keeps all its bits, NaN aside | [`shape-predict`](docs/reference.md#shape-predict) |
| `crates/mask-seq` | library, binary `mask-validate` | the `mask-1` form: writing a sequence folder of PGM frames with a manifest of their SHA-256, and checking one | [the mask sequence form](docs/reference.md#the-mask-sequence-form) |
| `crates/frame-io` | library, binaries `frame-gen` and `frame-cmp` | the raw frame contract: reading and writing streams, synthetic frames, and the comparison of two streams as bit patterns with no tolerance | [the frame contract](docs/reference.md#the-frame-contract) |
| `crates/stage-cli` | library, binary `mocha-stage` | the stage program: the registry, the manifest and the provider file, the argument rules, typed parameters and layers, the key file, the stages and the stage exit codes | [`mocha-stage`](docs/reference.md#mocha-stage) |
| `crates/pixel-format` | library | the conversion of one sample between 8 bit integers, 16 bit integers and 32 bit floats, and alpha premultiplication | [`pixel-format`](docs/reference.md#pixel-format) |
| `crates/contour-eval` | library | the evaluation of a planar contour of cubic Bezier spans or X-spline segments at a parameter, and its flattening to a polyline | [`contour-eval`](docs/reference.md#contour-eval) |
| `crates/coverage-raster` | library | the coverage of closed polygons in floating point coordinates, by exact area or by sampling | [`coverage-raster`](docs/reference.md#coverage-raster) |
| `crates/planar-xform` | library | a 3 by 3 planar projective transform in double precision, with the transform that maps four points to four points | [`planar-xform`](docs/reference.md#planar-xform) |
| `crates/resample` | library | the resampling of a buffer of 32 bit floats under an inverse mapping given as a function or a matrix | [`resample`](docs/reference.md#resample) |
| `crates/lens-model` | library | lens distortion as a coordinate map: a radial and tangential polynomial, a one parameter division model and a squeezed variant, with their inverses | [`lens-model`](docs/reference.md#lens-model) |
| `crates/soft-edge` | library | soft edge operators on a one channel buffer: a box blur, a Gaussian blur, the exact squared distance transform, and a ramp over the signed distance to the edge of a hard mask | [`soft-edge`](docs/reference.md#soft-edge) |
| `crates/sphere-reorient` | library | the image coordinates of a spherical image through a rotation, in equirectangular, cube map and stereo layouts | [`sphere-reorient`](docs/reference.md#sphere-reorient) |
| `crates/mesh-warp` | library | coordinates mapped by a triangle mesh and by a bilinear quad patch with its inverse | [`mesh-warp`](docs/reference.md#mesh-warp) |
| `crates/blend-ops` | library | RGBA compositing by the twelve operators of Porter and Duff and the blend modes of the W3C recommendation on compositing and blending | [`blend-ops`](docs/reference.md#blend-ops) |
| `crates/key-eval` | library | values given by keys at a time: hold, linear, Hermite and Bezier keys, and the rules before the first key and after the last | [`key-eval`](docs/reference.md#key-eval) |
| `crates/planar-track` | library | the tracking of a planar region through a sequence of grey level images by direct image alignment, from published methods | [`planar-track`](docs/reference.md#planar-track) |
| `crates/eye-layout` | library | the layout of one or two eyes in a frame as pure functions: a selector table that the caller supplies, the rectangle of one eye in a packed frame, layer and eye resolution, and the arguments of the two engine calls | [`eye-layout`](docs/reference.md#eye-layout) |
| `crates/mod-remove` | library | the frames that replace a layer, produced from other frames of a clip: a source index, soft weight maps, a brightness fit, the synthesis with a spread pass fallback, and a frame driver with its preconditions and composite | [`mod-remove`](docs/reference.md#mod-remove) |

`effect-shell` depends on `effect-params`, `shape-canon` on `shape-model`, `pixel-format` on `frame-io`, and `stage-cli` on the crates its stages run (the Crates column of [Stages](#stages)), each by its path in the workspace. Only `effect-shell` has `unsafe` code, which it uses at the host interface and in its freestanding build. The values that the tests of the crates use are stated under [Tests](#tests).

The generic crates (`pixel-format`, `contour-eval`, `coverage-raster`, `planar-xform`, `resample`, `lens-model`, `soft-edge`, `sphere-reorient`, `mesh-warp`, `blend-ops`, `key-eval`, `planar-track`) hold every choice their published sources leave open as a field of an explicit configuration, with a default that claims nothing about any program. The tests of `planar-track` hold its results on a sequence within a stated tolerance of the truth, never as exact bits; how a track is to be compared with the original is stated under [Limits](#limits).

`eye-layout` holds no selector table of its own: the caller supplies every row and every separator position, and `RoleTable::sample()` is a made up table for tests, examples and documentation.

`mod-remove` takes its rendering mode flag and any precondition text from the caller: the caller supplies the flag of `FrameInputs` and the texts of `PreconditionTexts`, a `Precondition` with no text from the caller carries a `Display` text of the crate's own, the default readings of `FillConfig` are the crate's own, and `FrameInputs::sample()` and `PreconditionTexts::sample()` are made up for tests, examples and documentation.

Outside the workspace: `contracts/` (the convention's folder, held byte for byte as twixtor publishes it at commit 8087d06 and not edited here), `tools/` (the three Python tools and their tests) and `docs/`.

## Interface

`mocha-stage`, `frame-gen` and `frame-cmp` print their usage with `--help`. The [reference](docs/reference.md#mocha-stage) gives the full behaviour, the order of the checks included. The convention itself is [twixtor's `docs/handoff.md`](https://github.com/baairon/twixtor/blob/main/docs/handoff.md).

### Pipeline

No program here starts a child process, and none uses ffmpeg. A stage reads whole frames on standard input and writes them on standard output, so stages chain through pipes. From the repository root after [Build](#build), with scratch files in the git ignored `target/demo/`:

```sh
mkdir -p target/demo
target/release/frame-gen --width 64 --height 36 --depth 8 --frames 2 --pattern checker:4 \
  | target/release/mocha-stage apply soft-edge-blur --width 64 --height 36 --depth 8 --fps 24/1 --param channels=all --param radius=2 \
  | target/release/mocha-stage apply resample-scale --width 64 --height 36 --depth 8 --fps 24/1 --param width=32 --param height=18 > target/demo/small.raw
```

`target/demo/small.raw` then holds two frames of 32 by 18 pixels at depth 8. Data passes from one stage to another through files. `track-planar` writes one pose per frame, as the matrix entries `m00` to `m22`, to the `keys-1` file that its parameter `data` names. Those entries are the keyed parameters of `warp-planar`, which reads that file as its `keys` layer. The block below makes a clip of six synthetic frames, `target/demo/clip.raw`, at 64 by 36 pixels and depth 8, runs `track-planar` on it, and runs `warp-planar` on it with the poses as its `keys` layer:

```sh
target/release/frame-gen --width 64 --height 36 --depth 8 --frames 6 --pattern random:11 > target/demo/clip.raw
target/release/mocha-stage apply track-planar --width 64 --height 36 --depth 8 --fps 24/1 --param data=target/demo/poses.keys --param report=target/demo/track.txt < target/demo/clip.raw > /dev/null
target/release/mocha-stage apply warp-planar --width 64 --height 36 --depth 8 --fps 24/1 --layer keys=target/demo/poses.keys < target/demo/clip.raw > target/demo/warped.raw
```

A wrapper of the convention takes `mocha-stage apply STAGE` with the placeholders of the convention as `--stage`, `--pre-stage` or `--post-stage` ([handoff: finding the provider](docs/handoff.md#finding-the-provider)). Neither twixtor nor sapphire has been chained with `mocha-stage`.

### Stage commands

| Command | Does |
|---|---|
| `mocha-stage list --json` | prints the stage manifest, one line of JSON that validates against `contracts/stage-manifest.schema.json` |
| `mocha-stage list` | prints one stage id per line |
| `mocha-stage apply STAGE --width W --height H --depth 8\|16\|32 --fps N[/D] [--frames N] [--param K=V]... [--layer ID=PATH]...` | reads whole raw frames on standard input and writes the frames of the stage on standard output |
| the same with `--describe` | prints the geometry answer of `contracts/describe.schema.json` and reads no input |
| `mocha-stage provider [--program PATH]` | prints the manifest as a provider file |
| `mocha-stage --version`, `--help` | prints `mocha-stage 0.1.0`, the version text of the manifest, or the usage text |

`--width`, `--height`, `--depth` and `--fps` are required, and `--fps` is `N` or `N/D` with positive whole numbers. `--frames N` is the expected frame count, and a stream with another number of frames is `bad_input`. A parameter is `--param K=V`, at most once per id, and its value is read by its kind: `int`, `float`, `popup`, `checkbox` or `text` ([parameters on the command line](docs/reference.md#parameters-on-the-command-line)). A value that holds `@` is the keyed form of the convention's command line, and every stage here refuses it as `unsupported`: a value that changes per frame comes from a `keys-1` file instead. A layer is `--layer ID=PATH`, at most once per id.

### Raw frame format

The frame contract is the convention's: whole frames back to back, with no header and no padding, in straight alpha.

| Depth | Layout | Bytes per pixel |
|---|---|---|
| 8 | packed `rgba` | 4 |
| 16 | packed `rgba64le` | 8 |
| 32 | planar `gbrapf32le`, the planes in the order G, B, R, A, with 1.0 at full scale | 16 |

A frame is at most 4 GiB. Input that ends inside a frame is a partial frame and an error. `alpha-premultiply` with `op=premultiply` writes premultiplied colour, which is outside the straight alpha of the contract until a later stage divides by alpha again ([reference: the frame contract](docs/reference.md#the-frame-contract)).

### Layer forms

A layer is a file named on the command line with `--layer ID=PATH`. Every layer of every stage is optional, and an absent layer is the neutral input of the operation. The forms are text files of this repository's own, each named by its first line ([reference: layer files](docs/reference.md#layer-files)).

| Form | Layer | Read by | Holds | Without the layer |
|---|---|---|---|---|
| `keys-1` | `keys` | the stages with a keyed parameter | keys per parameter, `hold` or `linear`, at times in frames | every keyed parameter keeps its command line value |
| `rings-1` | `shape` | `raster-polygon` | polygons as rings of points, in pixels of the frame | no polygon: coverage 0 |
| `contours-1` | `shape` | `raster-contour` | polygons as Bezier or X-spline contours of control points | no contour: coverage 0 |
| `mesh-1` | `mesh` | `warp-mesh` | one triangle mesh, with a source and an output position per vertex | the identity mesh: every position maps to itself |

`track-planar` reads no layer. Its parameter `data` names the `keys-1` file it writes, and its parameter `report` a `track-report-1` table with one row per frame; both files are written whole or not at all. The other file forms, the parameter table, the shape case and dump and the `mask-1` sequence among them, are in [handoff: what can be exchanged as files](docs/handoff.md#what-can-be-exchanged-as-files).

### Exit codes

| Exit | `mocha-stage` | `frame-gen` | `frame-cmp` | `shape-predict` | `mask-validate` |
|---|---|---|---|---|---|
| 0 | every input frame was written, or the list, the provider file or the description was printed | the frames were written | the streams are equal | the dump was written | the sequence is valid |
| 1 | `stage_failed` | not used | the streams differ | the case is refused, or standard output cannot be written | the sequence is invalid, or standard output cannot be written |
| 2 | `usage` | usage | usage | no file named, or a file that cannot be read | no argument or more than one, or an argument that cannot be read |
| 3 | `unsupported` | unsupported | unsupported | not used | not used |
| 4 | `bad_input`, `partial_frame`, `broken_pipe` | bad input | bad input | not used | not used |

The codes of `mocha-stage` are the convention's. An error of `mocha-stage` is one line on standard error, `error: <code>: <message>`, and [handoff: the stage program](docs/handoff.md#the-stage-program) says when each code is given. Exit code 1 of `frame-cmp` means that two readable streams differ. The convention keeps 1 for a stage that failed, so `frame-cmp` must not be registered or read as a stage. The codes of `shape-predict` and `mask-validate` are their own and are not mapped to the convention's. For those two, 1 means that the input was read and refused, which the convention reports as 4, or that standard output could not be written; neither may be registered as a stage.

### Python tools

All three run from the repository root with `python -I` and need the standard library only; each has a test file beside it.

| Tool | Does | Exit codes |
|---|---|---|
| `tools/status.py FILE...` | prints the status, rung label and re-run state of the unit result files named, then one totals line per rung, and one fixed line with no status for a survey capture comparison file; with no file, `Not started (no result file)` | 0 printed, 2 usage, 3 a file that cannot be read |
| `tools/gen_docs.py` | writes every generated figure and table of `README.md` and `docs/`, and `docs/data/facts.json`, from the result files; `--check` writes nothing and exits 1 when a page is stale | 0 no drift, 1 drift or an unknown key, 2 usage, 3 a result file that cannot be read, or a survey capture comparison file that breaks its schema |
| `tools/check_docs.py` | reads every page for stale figures, broken links, the status wording and hexadecimal addresses, and every text file for characters outside ASCII, dash characters and secrets | 0 no findings, 1 findings, 2 usage or a term list that cannot be used |

Two more checks of `tools/check_docs.py`, one for words about how the repository was made and one for names that belong outside it, take their terms from a list kept outside the repository; without that list each prints NOT RUN and adds no finding ([reference: page checks](docs/reference.md#page-checks)).

## Handoff

[twixtor](https://github.com/baairon/twixtor) and [sapphire](https://github.com/baairon/sapphire) share a handoff convention: a raw frame contract, stage commands that read frames on standard input and write them on standard output, a registry by which the tools find each other, and a conformance suite that every stage has to pass. This repository is a stage provider ([handoff](docs/handoff.md)). The convention lists five steps for a further tool:

| Step | What the convention asks | State here |
|---|---|---|
| 1 | `list --json` prints a stage manifest, and `apply STAGE` reads frames on standard input and writes frames on standard output with the stage exit codes; `--describe` answers the geometry. | Taken for every stage of the [Stages](#stages) table. |
| 2 | The `contracts/` folder and `tools/conformance.py` are held byte for byte. | Taken in part. `contracts/` is held as twixtor publishes it at commit 8087d06 and is not edited here. `tools/conformance.py` is not held. |
| 3 | One entry in `contracts/tool.json` names the tool, its markers, its binaries and its providers. | Not taken. The registry is held as published, with no entry for this repository. A tool that only provides stages may skip this step and be found through a provider file. |
| 4 | `conformance.py --stage` passes for every stage. | Taken. Run from a twixtor checkout at commit 8087d06 against the built `mocha-stage` on Linux, the conformance tool runs levels 1 and 3 and passes (270 and 33 checks) for each stage at its defaults, and for each generic stage with parameters that make it compute at the same size, for the raster stages with a shape file and for `warp-mesh` with a mesh file. |
| 5 | `handoff.py stages --json` lists the stages and `handoff.py doctor` reports no difference between peers. | Not taken. This repository does not hold `handoff.py`, and no peer has been run against it. |

Levels 0, 2, 4 and 5 of the conformance tool need the registry, a peer, the wrappers or ffmpeg, and have not been run here; continuous integration does not run the conformance tool.

A peer finds a provider that has no registry entry in one of two ways the convention defines:

| Way | What a peer gives |
|---|---|
| `--provider FILE` | the shipped file, `crates/stage-cli/providers/mocha_pro.stage.json`, with `mocha-stage` on `PATH` |
| `HANDOFF_PROVIDERS` | the folder `crates/stage-cli/providers`, with `mocha-stage` on `PATH`, or a folder that holds the output of `mocha-stage provider --program /absolute/path/to/mocha-stage` |

A test holds the shipped file equal to what `mocha-stage provider` prints. No peer has read the file, so discovery from a peer's side stays a gap; [handoff: known gaps](docs/handoff.md#known-gaps) lists it with every other gap measured against the convention.

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

- **The crates.** `cargo test` builds the crates of the workspace and runs their tests: the unit tests, the test files of each crate, and the tests that run the built binaries. Every name and value in them is invented, apart from six published NIST test vectors in the SHA-256 tests of `mask-seq`, the sizes, offsets and constants of the host's interface that the tests of `effect-shell` pin, the constants and default readings of the layout description that the tests of `eye-layout` pin, and the constants of the specification that the tests of `mod-remove` pin, and no test involves the original.
- **The interface files.** For each generic stage, for the two frame tools, for `shape-predict` and for `mask-validate`, a test runs the built program for every row of its interface file under `docs/data/`, so a change to an exit code or a report line fails `cargo test` until the file follows it.
- **The pages.** Two crate tests read `docs/reference.md`: one loads the example of the table format, the other the example manifest of the mask sequence form, so an edit of the table example that the loader would refuse, or any edit of the example manifest, fails `cargo test`. `gen_docs.py --check` exits 1 on a stale figure, and `check_docs.py` on a page or a text file that breaks a rule.
- **The tools.** Each Python tool has a test file beside it, on invented fixtures built in temporary directories. One test of `tools/test_check_docs.py` reads the term list kept outside the repository and is skipped when no list is named.
- **Expected results.** The output each command should print, with the number of tests of every test binary, is in [internals: checks](docs/internals/README.md#checks).

**Continuous integration.** `.github/workflows/ci.yml` runs on every push to `main`, on every pull request, and when started by hand. Its test job runs on Linux, macOS and Windows with the stable Rust toolchain and Python 3.12: it builds the crates, runs their tests, compiles `tools/`, runs the tests of the three tools, then `gen_docs.py --check` and `check_docs.py`, with a 30 minute limit. A second job, on Linux with a 15 minute limit, checks the formatting with `cargo fmt` and runs `cargo clippy` with every warning an error. Every cargo command except `cargo fmt` passes `--locked`. Each action is pinned to a full commit SHA, with the version it came from in a comment: `actions/checkout` v7.0.1, `actions/setup-python` v7.1.0, `Swatinem/rust-cache` v2.9.3, and `dtolnay/rust-toolchain` at a commit of its `master` branch. The workflow may only read the repository, and it is the only workflow. The term list is not in CI, so there the two checks that need it print NOT RUN and every other check runs. A later push to a pull request cancels its run in flight; each push to `main` keeps a run of its own.

### Smoke test

Confirms a build end to end. Run from the repository root after [Build](#build); scratch files go to the git ignored `target/demo/`.

```sh
mkdir -p target/demo
target/release/mocha-stage list --json
target/release/mocha-stage provider > target/demo/mocha_pro.stage.json
cmp target/demo/mocha_pro.stage.json crates/stage-cli/providers/mocha_pro.stage.json
target/release/frame-gen --width 64 --height 36 --depth 32 --frames 3 --pattern aramp > target/demo/in.raw
target/release/mocha-stage apply selftest-copy --width 64 --height 36 --depth 32 --fps 24/1 < target/demo/in.raw | target/release/mocha-stage apply warp-planar --width 64 --height 36 --depth 32 --fps 24/1 > target/demo/out.raw
target/release/frame-cmp --width 64 --height 36 --depth 32 target/demo/in.raw target/demo/out.raw
printf 'case=1\ntime=0x0.0p+0\nkey=0x0.0p+0\npt=1,1,0x1.0000000000000p+0,-0x0.0p+0,none,none,0x0.0p+0,0x0.0p+0,0x0.0p+0,0x0.0p+0\n' > target/demo/case.txt
target/release/shape-predict target/demo/case.txt
python -I tools/status.py
python -I tools/gen_docs.py --check
python -I tools/check_docs.py
```

| Step | Expected |
|---|---|
| `list --json` | one line of JSON with `"contract":"frames/2"`, the provider id `mocha_pro` and every stage of the [Stages](#stages) table |
| `provider` and `cmp` | exit 0 and no output from `cmp`: the program prints the shipped provider file |
| the two stages and `frame-cmp` | `equal frames=3 width=64 height=36 depth=32` and exit 0: `selftest-copy` and `warp-planar` at its defaults write the input frames again |
| `shape-predict` | the predicted dump of the case: `format=shape-1` first, the point at time 0 with `c0.t0.p0.y=-0x0.0p+0`, and `section=meta` last |
| `status.py` | `Not started (no result file)` |
| `gen_docs.py --check` | `gen_docs --check: 8 outputs, 0 drifted, 0 unknown keys` |
| `check_docs.py` | a NOT RUN line for each of the two term list checks, and the last line `check_docs: 7 pages, 0 findings, 2 checks not run` |

## Limits

What the method cannot show, whatever the results say:

| Limit | What it means |
|---|---|
| Nothing compared | The status reads <!--f:status_line-->method and preparation only: 0 cases compared<!--/f-->. There is no reference output and no label, and no crate computes an output of the plug-in. The original's registration of its parameters is observed, and one survey capture comparison with it is recorded; it is not a case, has no parity label and is never parity evidence ([parity: the survey capture comparison](docs/parity.md#the-survey-capture-comparison)). |
| The observed registration | The original's registration of its parameters comes from one boot on a reduced host whose answers are declared. It is not a result of record and not parity evidence, and a comparison with it carries no parity label. Such a comparison can say whether the two tables agree field by field on that host. It says nothing about any output or render, about a second boot, or about what the original registers under other host answers or on a real machine; all of that stays unmeasured ([parity: the observed registration](docs/parity.md#the-observed-registration)). |
| One judge | Every reference comes from one emulator. An emulator bug is common to both sides of a comparison: it can make a wrong rewrite look right, or a right one look wrong. Until a routine that the original implements twice has run on a second engine, every result reads "single judge, not cross-checked". |
| Emulator, not silicon | The emulator follows the architecture manual. Where real processors differ from it (estimate instructions, NaN payloads, which feature tier a dispatcher picks), nothing is claimed. |
| Instruction-set route | The emulated CPU has no AVX tier. Where the original picks its code by CPU features, only the path for that set runs, and every result carries the qualifier that CPUs with other tiers run a different body, which is not reproduced. |
| The stand-in register | Where the host would answer, the harness answers from its register, each row following public documentation. Agreement on a stand-in says the two programs treat that answer alike; it says nothing of what a real host would return. |
| The licence gate | The harness answers the original's licence module only as [Left out](#left-out) lists, and none of those answers tells the original that it is licensed. Which modules the original then offers is read from its own run, never assumed. A unit that the original gates behind its licence state is judged by what the original does in the emulator with these answers and no others: a stop is recorded as Blocked, an output that looks unlicensed is never used as a reference, and every result from a run with these answers names them as a limit of the judge. An output that looks unlicensed is never used as a whole frame reference. Where the original marks each frame its engine delivers with an overlay, only the samples outside the overlay are to be compared: its extent is to be measured from the original's own frames before any comparison, and the excluded samples are to be counted in every such result, which is to have its own status below Matches. This is a rule of method that no tool computes yet ([parity: frames that carry an overlay](docs/parity.md#frames-that-carry-an-overlay)). |
| The licence dependent modules | The modules that the original gates behind its licence state are to be rebuilt from public methods and from a written specification of each module's arithmetic. Whether the emulated original renders each of these modules without any licence or feature question being answered is not known: it is to be measured module by module. The original has run in the emulator only as far as the end of its parameter setup, and no render command of it has run in the emulator. Where a module renders in the emulator, its frames are to be compared outside the overlay, under the status "matches the emulated original outside the overlay footprint", which is below Matches ([parity: frames that carry an overlay](docs/parity.md#frames-that-carry-an-overlay)). Where a feature question stands in front of a module's output, no answer is given and the module is compared only with frames rendered on a licensed installation and supplied by the owner, under the status Owner supplied frames, which is not a parity result ([parity: frames supplied by the owner](docs/parity.md#frames-supplied-by-the-owner)). No route adds any answer to those that [Left out](#left-out) lists. |
| The planar tracker | The original's planar tracker is an iterative solver with two paths, a graphics processor path and a processor path, and its code holds switches whose names show that they exist to manage a difference between the two; the size of that difference has not been measured. A whole track is therefore expected to be comparable only within a stated tolerance on the positions of the tracked surface's corners, not exactly. Single steps on fixed inputs, such as one level of the image pyramid, one evaluation of the error, one damped solver step from a given state and the chaining of per frame motions to a reference frame, are expected to be comparable exactly once their number types are matched. The result schema and `tools/status.py` cannot state a tolerance on corner positions (the schema's `tolerance_ulp` counts code steps or ulps of a sample), so this tolerance is a rule of method only ([parity: what the results do not cover](docs/parity.md#what-the-results-do-not-cover)), and a result at a tolerance other than 0 has no rung label under the status rule. No track has been compared, and how a reference track of the original is to be obtained is not settled. |
| Shape hypotheses | The rules of `shape-model` (how points are kept at keys, blended between them and held outside them, and how their rectangle is spanned) are hypotheses that no compared case has tested. A predicted dump built on them says nothing about the original until a case is compared. |
| A sample, not a proof | A grid covers chosen, synthetic and small inputs; real footage and real projects are not exercised. Mutants are chosen by the authors, so a kill count depends on which were written. |
| Generic stages only | `mocha-stage` follows the handoff convention that twixtor and sapphire share. Its stages are `selftest-copy`, which copies frames unchanged, and thirteen generic stages, each a published method under explicit configuration that is not compared with any product. No stage reimplements any output of the plug-in, and none that does is to be offered before its unit has been compared. The [handoff](docs/handoff.md) page lists what exists and what is absent. |

### Left out

Never part of this project: the licence, activation and machine identity code of the original. It is never read or analysed: where a read of the original's other code reaches a call into it, only that call and what the caller does with the answer are recorded ([Legal](#legal)). Nothing in it is changed on disk; it runs as the original's own code, and only inside the emulator. So that the original's engine can load at all, the mock host answers calls that the licence module makes, at the boundary between the emulated program and the host, in these ways only: a request to load or find a library the emulated machine holds gets that library, and a later release of it is accepted; a plain question about the emulated machine's Windows folders or environment gets the declared answer that every program in the emulator gets; a request for anything the emulated machine does not hold gets the documented Windows not found or not available failure; and a question from the module that identifies the machine gets its documented failure, never a value, real or invented. The module's own start-up routine is not run when the engine loads. No answer tells the original that it is licensed, and no licence server or network is involved. Any other call that the licence module makes to the host stops the run at that call, and so does a licence or activation call that any other part of the original makes to the host. No machine identity question, from any part of the original, gets a value from a real machine. A unit that cannot go on without more stops there; its result file says so. The emulated process has no network: every network call faults.

The modules that the original gates behind its licence state are not left out: they are part of the scope, and what their comparison rests on is stated under [Limits](#limits). The interactive interface of the product's standalone editor is left out: it is not rebuilt, and the capabilities in scope are to be offered as stage programs for command line use.

## Legal

- **Independent work.** This project is written independently and contains none of Mocha Pro's code, binaries, assets or documentation.
- **How the match is made.** The outputs are to be matched by running the original plug-in, unmodified, in an emulator on test inputs, for interoperability. It has been run there only as far as the end of its parameter setup, where it registers its parameters with the host; no render command of it has run there. Its files are examined statically (headers, import and export tables, resources and strings), and the product's documentation is read to predict its interface and to plan the shape data check. Parts of its code are also read statically, without running it: the plug-in's handling of its parameter setup, its render commands and its arbitrary data, and, in its engine module, the entry path, the reader of the project data and stages of the render path with their arithmetic; and its planar tracker, read mostly in the product's separate tracking and numerics files. Each read is written up as a specification of method, interfaces and formats, with no code excerpt and no instruction listing, and kept outside this repository; code here that rests on one is written from it and from what the emulator shows, not from the original's code. Where a read path reaches a licence, activation or machine identity check, only the call and what its caller does with the answer were recorded, and the check itself was not read ([Left out](#left-out)).
- **No affiliation.** It is not affiliated with, endorsed by or sponsored by any other company, product or library.
- **Trademarks.** Product and library names belong to their owners and appear here only to identify them. Mocha Pro is named only to say what this project reimplements.
- **Interoperability.** Parameter names and values of the plug-in are to be used only where settings must carry over between the two and outputs must be matched; this version holds none of them.
- **Host interface.** The plug-in shell's definitions of the host's structures, their sizes, offsets and constants are written by this project for interoperability. No part of Adobe's After Effects SDK is included; to build the harness you need your own copy.
- **No warranty.** Everything here is provided as is under the MIT License, without warranty of any kind and without liability.
- **Your responsibility.** You need the rights to the media and the software you process, and you stay bound by the licences of any software you use alongside this project, the original plug-in and the host's SDK included.
- **Rights holders.** If you believe something here infringes your rights, email [hi@bairon.dev](mailto:hi@bairon.dev) or [open an issue](https://github.com/baairon/mocha-pro/issues). Every concern gets a prompt answer in good faith.

## License

MIT, see [LICENSE](LICENSE).
