# stage-cli

`mocha-stage`: a generic frame stage program for the handoff convention (how ffmpeg, twixtor and sapphire pass raw frames to each other: `docs/handoff.md` of the twixtor repository, with the contracts in its `contracts/` folder).

This is step 1 of "How a third plugin joins" for a tool that so far renders nothing of its own. The program offers one stage, `selftest-copy`, a self test that copies frames unchanged and reimplements nothing. It computes, imitates and claims no behaviour of any plug-in; a stage that does is outside the scope of this crate.

## Commands

| Command | Does |
|---|---|
| `mocha-stage list [--json]` | `--json` prints the stage manifest (one line of JSON, `contracts/stage-manifest.schema.json`); without it one stage id per line |
| `mocha-stage apply selftest-copy --width W --height H --depth 8\|16\|32 --fps N[/D] [--frames N]` | reads whole raw frames on standard input and writes them unchanged on standard output |
| `mocha-stage apply selftest-copy --describe --width W --height H --depth D --fps N[/D] [--frames N]` | prints the geometry answer (`contracts/describe.schema.json`) and reads no input |
| `mocha-stage --version`, `--help` | `mocha-stage 0.1.0` (the manifest's provider version), the usage text |

`--width`, `--height`, `--depth` and `--fps` are required; `--opt=value` is accepted next to `--opt value`. `--frames N` is the expected frame count: another count is `bad_input`. A stage takes no `--param` and no `--layer`.

The manifest has the provider `mocha_pro` (command `["mocha-stage"]`, version `mocha-stage 0.1.0`) and exactly one stage: `selftest-copy`, kind `frame`, spool `false`, geometry `preserve`, depths `8, 16, 32`, no parameters, no layers, with a name and a description that says it is a self test that copies frames unchanged and reimplements nothing.

The provider id is `mocha_pro`, not `mocha-pro`: `stage-manifest.schema.json` requires `^[a-z][a-z0-9_]{0,31}$` (and `tool.schema.json` says "Lower case, no hyphen"), so a hyphenated id does not validate. The id is one constant, `PROVIDER_ID` in `src/stage.rs`.

## Exits and errors

| Exit | Code | When here |
|---|---|---|
| 0 | none | every input frame was written (an empty input is zero frames) |
| 1 | `stage_failed` | a write failed for a reason other than a closed pipe |
| 2 | `usage` | a bad or missing flag, a malformed number or rate, a repeated option, a frame above 4 GiB, a terminal on standard input or output |
| 3 | `unsupported` | an unknown stage, a depth that is a whole number but not 8, 16 or 32, any `--param`, any `--layer` |
| 4 | `bad_input` | a read failed, `--frames` differs from the count read |
| 4 | `partial_frame` | the input ends inside a frame: the whole frames before it stay written, the partial one is not |
| 4 | `broken_pipe` | the reader of standard output went away |

Every error is one line on standard error in the convention's text form `error: <code>: <message>`; control characters in the message (a stage name with a line break, for instance) become spaces so it stays one line. Standard output carries nothing on a failure before the first frame. Checks run in this order: command line form (2), stage (3), depth (3), parameters and layers (3), frame size (2).

## `--describe` for a preserve stage

The convention does not reserve `--describe` for declared geometry; it defines what a `preserve` stage's output is. The sentences, from `docs/handoff.md`:

- Geometry table: "`preserve` | width, height, fps and pixel format of the output equal the input; the frame count may change only for a temporal stage that says so through its parameters"
- Geometry table: "`declared` | frames/2: the stage answers `apply STAGE --describe --width W --height H --depth D --fps N/D [--frames N] [--param K=V]...` without reading standard input, with its output width, height, depth, fps and frame count"
- `describe.schema.json`: "A stage with geometry declared answers `apply STAGE --describe ...` with this document on standard output, reads no standard input and exits 0".

Nothing says a preserve stage must refuse the flag. This program answers it, as sapphire does for its own preserve self test stage: the answer is the request (width, height, depth, the rate exactly as given, `frames` as given or `null`), because that is what "equal the input" means. The same refusals as `apply` apply (unknown stage, unsupported depth, parameter: exit 3; bad form: exit 2). If a later reading of the convention reserves the flag, the change is one branch in `execute` in `src/lib.rs`.

## Frame contract (duplicated)

`src/frame.rs` holds the frame sizes of the convention: depth 8 `rgba` 4 bytes per pixel, depth 16 `rgba64le` 8 bytes, depth 32 `gbrapf32le` 16 bytes (four planes G, B, R, A); whole frames back to back, no header, no padding, a frame at most 4 GiB. The frame-io crate holds the same sizes; the code that repeats them here is `src/frame.rs` (`DEPTHS`, `bytes_per_pixel`, `frame_bytes`, `MAX_FRAME_BYTES`) and the frame loop `copy_frames` in `src/run.rs`. A pass through stage needs no pixel layout beyond the frame size.

## Platforms

- Standard input and output are raw bytes: the Rust standard library does no newline translation on any platform, so the bytes `0x0a`, `0x0d` and `0x1a` pass unchanged on Windows (tested with every byte value on Linux; on Windows this follows from the library's behaviour and was not run; the Windows build compiles and lints clean for `x86_64-pc-windows-gnu`, and was not run there).
- A closed reader is a write error, not a signal: Rust binaries ignore SIGPIPE on Unix, and a closed pipe is `BrokenPipe` on Windows too. The program exits 4 with `error: broken_pipe: ...` (tested on Linux).
- A terminal on standard input or output is refused (exit 2) instead of read as text; a Windows console read would otherwise insist on UTF-8.
- Arguments must be valid UTF-8 text (exit 2 otherwise). No platform specific code, no unsafe code, no dependency beyond the standard library.
- Standard output goes through the standard library's line buffered writer; bytes are not changed by it, whole frames are written with one `write_all` each.

## Vendoring

The convention asks a joining tool to hold `contracts/` and `tools/conformance.py` byte for byte. The repository holds `contracts/` as twixtor publishes it at commit 8087d06 and edits nothing in it. It does not hold `tools/conformance.py`. It adds no entry to the registry `contracts/tool.json`: a tool that only provides stages may skip that step and be found through a provider file.

## Checks

- `cargo test -p stage-cli`: 47 unit tests and 34 tests of the built binary (`tests/cli.rs`), on made up frames.
- The convention's own tool, run from a twixtor checkout at commit 8087d06: `python3 -I tools/conformance.py --stage "<path>/mocha-stage apply selftest-copy"` (level L1 stage matrix and L3 failure injection; the other levels test wrappers and peers).
