# Handoff

[twixtor](https://github.com/baairon/twixtor) and [sapphire](https://github.com/baairon/sapphire) share a handoff convention: a raw frame contract, stage commands that read frames on standard input and write them on standard output, a registry by which the tools find each other, and a conformance suite that every stage has to pass. Its definition is [twixtor's `docs/handoff.md`](https://github.com/baairon/twixtor/blob/main/docs/handoff.md), which both repositories hold byte for byte.

This page states where this repository stands against that convention. In one sentence: it is a stage provider with one stage, and that stage is a self test. `mocha-stage` prints a stage manifest and runs `selftest-copy`, which copies frames unchanged, so that the plumbing between the tools can be checked before any unit of this repository computes a frame. No stage reimplements any output of the plug-in. Beside the stage program there are two frame tools, a set of file forms, and two older command line programs with exit codes of their own.

## Contents

- [The stage program](#the-stage-program)
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
| `mocha-stage apply selftest-copy --width W --height H --depth 8\|16\|32 --fps N[/D] [--frames N]` | reads whole raw frames on standard input and writes them unchanged on standard output |
| the same with `--describe` | prints the geometry answer of `contracts/describe.schema.json` and reads no input |

The manifest names the provider `mocha_pro` (the schema's pattern for an id allows no hyphen) and one stage:

| Stage | Kind | Geometry | Depths | Parameters | Layers | What it computes |
|---|---|---|---|---|---|---|
| `selftest-copy` | `frame` | `preserve` | 8, 16, 32 | none | none | nothing: each input frame is written as it was read |

The exit codes are the convention's:

| Exit | Code word | When |
|---|---|---|
| 0 | none | every input frame was written, or the list or the description was printed |
| 1 | `stage_failed` | a write failed for a reason other than a closed pipe |
| 2 | `usage` | a bad or missing option, a terminal on standard input or output |
| 3 | `unsupported` | an unknown stage, a depth other than 8, 16 or 32, any `--param` or `--layer` |
| 4 | `bad_input`, `partial_frame`, `broken_pipe` | a failed read, a frame count other than `--frames`, input that ends inside a frame, a reader that went away |

An error is one line on standard error, `error: <code>: <message>`. The [reference](reference.md#mocha-stage) and `crates/stage-cli/README.md` give the full behaviour, the order of the checks included.

A stage that computes something of the plug-in is added only after its unit has been compared with the original under the [parity](parity.md) rules, and it then carries that unit's status. Until then the manifest lists the self test alone.

## Frames

The frame contract is the convention's: whole frames back to back, with no header and no padding, in straight alpha.

| Depth | Layout | Bytes per pixel |
|---|---|---|
| 8 | packed `rgba` | 4 |
| 16 | packed `rgba64le` | 8 |
| 32 | planar `gbrapf32le`, the planes in the order G, B, R, A, with 1.0 at full scale | 16 |

A frame is at most 4 GiB. Input that ends inside a frame is a partial frame and an error. `crates/frame-io` holds the contract as a library ([reference: the frame contract](reference.md#the-frame-contract)); `crates/stage-cli` repeats the three sizes so that the stage program depends on no other crate.

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
| 1 | `list --json` prints a stage manifest, and `apply STAGE` reads frames on standard input and writes frames on standard output with the stage exit codes; `--describe` answers the geometry. | Taken for one stage, `selftest-copy`, which reimplements nothing. |
| 2 | The `contracts/` folder and `tools/conformance.py` are held byte for byte. | Taken in part. `contracts/` is held as twixtor publishes it at commit 8087d06 and is not edited here. `tools/conformance.py` is not held. |
| 3 | One entry in `contracts/tool.json` names the tool, its markers, its binaries and its providers, and `handoff.py sync` gives every peer the same registry. | Not taken. The registry is held as published, with no entry for this repository. A tool that only provides stages may skip this step and be found through a provider file. |
| 4 | `conformance.py --stage` passes for every stage. | Taken for `selftest-copy`: the conformance tool, run from a twixtor checkout at commit 8087d06 against the built `mocha-stage` on Linux with `python -I tools/conformance.py --stage "<path to>/mocha-stage apply selftest-copy" --full` inside the checkout, runs levels 1 and 3 and passes (270 and 33 checks). |
| 5 | `handoff.py stages --json` lists the stages and `handoff.py doctor` reports no difference between peers. | Not taken. This repository does not hold `handoff.py`, and no peer has been run against it. |

With `--stage`, the conformance tool runs level 1 (the stage matrix: depths, sizes, frame counts and alpha modes) and level 3 (failure injection under a deadline, with the error exits of the stage). Levels 0, 2, 4 and 5 need the registry, a peer, the wrappers or ffmpeg, and have not been run here.

## Finding the provider

A peer finds a provider that has no registry entry in one of two ways the convention defines: `--provider FILE`, with a `*.stage.json` manifest, or the `HANDOFF_PROVIDERS` folders. This repository ships no `*.stage.json` file yet; the manifest is what `mocha-stage list --json` prints, and a provider file can be written from it. Until one is shipped and checked from a peer's side, discovery is a [known gap](#known-gaps).

## What can be exchanged as files

Each form is a plain file with a specification in the [reference](reference.md) and tests that pin it. None of them carries a name, a value or a text of the original.

| Form | What it holds | Read or written by | Specified in |
|---|---|---|---|
| `table v1`, `table v2` | A parameter table as text: one row per parameter, six tab separated cells in version 1 (a seventh, the extra cell, is optional) and ten in version 2. Floats are written as bit patterns, an unknown cell as `?`. | The `effect-params` library: loader, serializer and pairing. It has no command line program. | [The table format](reference.md#the-table-format), [The pairing](reference.md#the-pairing) |
| Shape case | A text file that states one shape case: contours, points at keys, and the times to report. | Read by `shape-predict`. | [The case form](reference.md#the-case-form) |
| `shape-1` dump | The predicted report for a case, as text with floats in a canonical hexadecimal form. | Written by `shape-predict`. | [The dump form](reference.md#the-dump-form) |
| `mask-1` sequence | A folder with `manifest.txt` and one binary PGM file per frame, one channel at depth 8 or 16, with a SHA-256 for every frame in the manifest. | Written by the `mask-seq` library, checked by `mask-validate`. | [The mask sequence form](reference.md#the-mask-sequence-form) |
| Raw frame file | A frame stream of the contract above, stored as a file. | Written by `frame-gen`, compared by `frame-cmp`, read and written by `mocha-stage`. | [The frame contract](reference.md#the-frame-contract) |
| Result file | The measured outcome of one unit: rung, judge, comparator, counts, mutants, stand-ins, and for a unit of frames the geometries compared. | Read by `tools/status.py` and `tools/gen_docs.py`. No result file exists yet. | `docs/data/result_schema.json`, [Status of one result file](reference.md#status-of-one-result-file) |

## The two file programs

`shape-predict` and `mask-validate` are older than the stage program and are not stages. Both take one path, read no standard input, and are deterministic.

| Program | Argument | Standard output | Exit codes |
|---|---|---|---|
| `shape-predict CASE` | a case file | the `shape-1` dump | 0 dump written, 1 case refused, 2 no argument or unreadable file |
| `mask-validate SEQUENCE` | a sequence folder or its `manifest.txt` | one line that begins `ok format=mask-1` | 0 valid, 1 invalid, 2 usage or unreadable argument |

Their codes are their own and are not mapped to the convention's: here 1 means that the input was read and refused, which the convention reports as 4, and 2 also covers an unreadable file. A wrapper must not read an exit code of these two programs as a code of the convention, and neither may be registered as a stage. Every exit code, stream and reason word is in `docs/data/shape_predict_interface.json` and `docs/data/mask_validate_interface.json`, and a test of each crate runs the program against its file.

## Masks

The `mask-1` form is not the frame contract. It holds one channel, one file per frame, each with a PGM header, at depth 8 or 16, with a manifest that names the geometry and a hash for every frame. It is stored on disk and checked as a whole; it is never piped. No converter between `mask-1` and a frame stream is provided: a stage that offered masks to the convention would have to choose how one channel is carried in a four channel frame, and that choice has not been made.

## Known gaps

Measured against the convention:

| Gap | State |
|---|---|
| A stage that computes | None. The one stage copies frames. |
| Parameters and layers | The stage takes neither; any `--param` or `--layer` is refused as unsupported. |
| Declared geometry | The stage preserves geometry. No stage declares another. |
| Discovery | No registry entry, no markers, no `*.stage.json` file. |
| Packets and `handoff.py` | Not held and not run. |
| Reports and the stall watch | `mocha-stage` writes no report. The wrappers of the convention have not been run around it. |
| The conformance tool | Not held here, and not run by continuous integration. |
| Conformance levels 0, 2, 4 and 5 | Not run. |
| Conformance on Windows | Not run. The stage program has not been built or tested on Windows by this repository's CI yet; it passes `cargo clippy` for the target `x86_64-pc-windows-gnu` on Linux. |
| ffmpeg | Never used here. |
| A peer | Neither twixtor nor sapphire has been chained with `mocha-stage`. |

## Platforms

Continuous integration builds the crates and runs their tests and the tests of the three Python tools on Linux, macOS and Windows. It does not run the convention's conformance tool. The tools need Python 3 and its standard library only and run under `python -I`; the convention's conformance tool asks for Python 3.12. No program here starts a child process, so the process group and kill paths that the convention requires of a wrapper have no counterpart.

## Not offered

The licence, activation and machine identity code of the original is never part of this project ([Left out](../README.md#left-out)), and no stage will ever stand in for it.
