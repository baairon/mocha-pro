# Parity log

The order in which the parity results came about. The present state, the method and the terms live in [parity](parity.md); this page is the record by date, and it keeps findings that were later superseded.

## 2026-11-17: the judge

**Which harness.** The judge is of the kind [twixtor](https://github.com/baairon/twixtor) and [sapphire](https://github.com/baairon/sapphire) used: the original's own code, with nothing in it changed, running in an emulator. Here, as in twixtor, a mock host stands around it. A new loader written directly on Unicorn was considered first and set aside. Twixtor's harness, on Qiling 1.4.6 and Unicorn 2.1.4 ([its README](https://github.com/baairon/twixtor/blob/8087d06ac43023f954de2cc58583b132a3243ff4/parity/harness/README.md#running-it)), had already booted and rendered real plug-ins for the same host, while a new loader would have been a part of the judge that no real plug-in had exercised, and a defect in it would be common to both sides of every comparison. The harness here starts from twixtor's at commit 8087d06 and changes it only where this plug-in or the fault rule needs it.

**What changed from twixtor's harness.**
- **Unknown calls fault.** Twixtor's harness lets a Windows function with no hook return 0 and lists that as a fidelity risk ([twixtor: assumptions and stand-ins](https://github.com/baairon/twixtor/blob/8087d06ac43023f954de2cc58583b132a3243ff4/README.md#assumptions-and-stand-ins)). Here a call with no entry in the register of host answers stops the run with its name, and every entry names the public documentation it follows.
- **Some calls stop the run on purpose.** Licence-family and machine-identity calls are never answered, and a message box or a dialog stops the run instead of returning a button.
- **The C runtime runs as its own code,** as it did in twixtor, so formatted output and the runtime's arithmetic come from the runtime libraries themselves, not from a model of them.
- **A fixed CPU.** The emulated processor presents one feature set without AVX. A vector instruction outside it, a feature query the table does not list, and the time-stamp and random-number instructions stop the run.
- **Thread-local storage.** The static thread-local storage of an image that has it is set up before its entry point runs.
- **Host layouts.** The layouts of the host's structures are compiled from the SDK headers, as in twixtor, with two compiler flags added: one keeps the structure types in the debug data, the other finds every header.

**Tests of the harness.** The harness carries unit tests of its own, which run on synthetic plug-in images made for them; none of them loads the original. The harness is not in this repository, so neither those tests nor their results can be checked from it.

## 2026-11-17: the interface

**The rewrite's side.** `crates/effect-params` was written: the parameter types, the table format with its exact-bits float cells, and the pairing with its match classes and field verdicts ([reference: the crates](reference.md#the-crates)). Its tests use invented names only.

**Mutants of the crate's tests.** Part of the method is to plant one error at a time in copies of the crate and to see whether its tests notice. Those runs, and what they found, are kept outside the repository, so this page gives no count for them. They measure the strength of the crate's own tests only. They are not the mutants of the status rule, which are copies of the rewrite judged against the original's outputs.

**The prediction.** The predicted parameter tables were written from the product's documentation and sealed before any run of the original. A cell the documentation does not settle is left unknown, and an unknown cell is not scored. The tables are private ([reproduce: inputs you supply](reproduce.md#what-needs-inputs-you-supply)).

## 2026-11-17: the repository

The documentation, the status tools with their tests, the crate and the CI workflow were prepared for the first version of the repository. When this entry was written the original had not been booted, no result file existed, and no case had been compared.

## 2026-11-17: the shape data

**The rewrite's side.** `crates/shape-model` and `crates/shape-canon` were written for a second prepared check, the [shape data check](parity.md#the-shape-data-check): a model of one shape stored as keyframes of Bezier control points, a canonical text dump of its state at a list of query times, a plain text form for a case, and the command `shape-predict`, which writes the predicted dump of a case ([reference: the shape model](reference.md#the-shape-model)). The field and record names are the rewrite's own, and the tests use invented values only.

**Hypotheses.** How the original keeps the points of a shape at its keys, blends them between keys and holds them outside the keys, and how it spans and ties the rectangle around them, is not known from any measurement. `shape-model` states each of these rules as a hypothesis, labelled INFERRED in its module comment, and a test pins each one. For two of them, the rounding form of the blend and the tie rule of the rectangle, the comments of their tests add that they are hypotheses of the crate that have not been compared with the original ([internals: the hypotheses](internals/shape-crates.md#the-hypotheses)).

**Mutants of the crates' tests.** As for `effect-params`, the tests of the two crates are probed with deliberately broken copies of the crates, one planted error per copy. Those runs, and what they found, are kept outside the repository, so this page gives no count for them.

**The prediction.** The predicted dumps of the shape cases were written and sealed before any run of the original. A value the predictor has no basis for is written `UNKNOWN` and is never counted as a correct prediction. The cases and the dumps are private ([reproduce: inputs you supply](reproduce.md#what-needs-inputs-you-supply)).

When this entry was written the original had not been booted, no result file existed, and no case had been compared.

## 2026-11-17: the first runs

**The original in the emulator.** The original plug-in was started in the emulator, unmodified, under the fault rule, by which a call the harness has no answer for stops the run and names itself. The runs were for the first step of the method, the interface pairing and the calibration ([parity: the interface pairing and calibration](parity.md#the-interface-pairing-and-calibration)). When this entry was written, the code that computes the original's output had not been read or traced; the [static reading](#2026-11-24-the-static-reading) came later.

**Optional Windows components.** Where Windows would provide an optional component that the Microsoft C runtime probes for and that the harness does not model, the harness answers with a declared "not found". The README's list of stand-ins already states this for host services on the declared list of absent ones, and a request with no declared answer still stops the run ([README: assumptions and stand-ins](../README.md#assumptions-and-stand-ins)).

**Further runs.** When this entry was written the work was meant to go on through further runs, each of which would add only declared answers to the harness's register of host answers, every one with the public documentation it follows. The harness and the records of these runs are not in this repository, so none of this can be checked from it.

**CI on macOS and Windows.** On the [second pull request](https://github.com/baairon/mocha-pro/pull/2), the CI workflow ran the shape crates and their tests on macOS and Windows for the first time, and both test jobs passed ([the workflow run](https://github.com/baairon/mocha-pro/actions/runs/44062826004)).

When this entry was written the interface pairing had not been made, no result file existed in the repository, and no case had been compared.

## Test runs, 2026-11-17

On Linux, with Rust 1.94.0, the stable Rust 1.100.0 for the formatting and lint rows as well, and Python 3.12.3. The shape crates had not run on macOS or Windows by then; the CI workflow is where they first meet those two systems. None of these runs compares anything with the original.

| Suite | Result |
|---|---|
| `cargo test --locked --no-fail-fast` (Rust 1.94.0) | 35 pass (18 in `effect-params`, 17 in `shape-canon`), 0 fail, 0 ignored |
| `cargo fmt --all -- --check` (Rust 1.94.0 and 1.100.0) | no change needed |
| `cargo clippy --locked --all-targets -- -D warnings` (Rust 1.94.0 and 1.100.0) | no warning |
| `python -m compileall -q tools` | no error |
| `python -I tools/test_status.py` | 52 tests, OK |
| `python -I tools/test_gen_docs.py` | 13 tests, OK |
| `python -I tools/test_check_docs.py` | 32 tests, OK, 1 skipped: the test that reads a term list skips when none is named |
| `python -I tools/gen_docs.py --check` | 8 outputs, 0 drifted, 0 unknown keys |
| `python -I tools/check_docs.py` | 7 pages, 0 findings, 2 checks not run (no term list named) |

## Test runs, 2026-11-18

On Linux, with Rust 1.94.0, the stable Rust 1.100.0 for the formatting and lint rows as well, and Python 3.12.3, after `crates/shape-canon/tests/cli.rs` was added. Against the [runs of 2026-11-17](#test-runs-2026-11-17), only the row of the crate tests changed: it gains the 7 tests of that file, which run `shape-predict` as a child process. That file had not run on macOS or Windows by then; the CI workflow is where it first meets those two systems. None of these runs compares anything with the original.

| Suite | Result |
|---|---|
| `cargo test --locked --no-fail-fast` (Rust 1.94.0) | 42 pass (18 in `effect-params`, 24 in `shape-canon`), 0 fail, 0 ignored |
| `cargo fmt --all -- --check` (Rust 1.94.0 and 1.100.0) | no change needed |
| `cargo clippy --locked --all-targets -- -D warnings` (Rust 1.94.0 and 1.100.0) | no warning |
| `python -m compileall -q tools` | no error |
| `python -I tools/test_status.py` | 52 tests, OK |
| `python -I tools/test_gen_docs.py` | 13 tests, OK |
| `python -I tools/test_check_docs.py` | 32 tests, OK, 1 skipped: the test that reads a term list skips when none is named |
| `python -I tools/gen_docs.py --check` | 8 outputs, 0 drifted, 0 unknown keys |
| `python -I tools/check_docs.py` | 7 pages, 0 findings, 2 checks not run (no term list named) |

## Test runs, 2026-11-18, with the interface file

On Linux, with Rust 1.94.0, the stable Rust 1.100.0 for the formatting and lint rows as well, and Python 3.12.3, after `docs/data/shape_predict_interface.json` and `crates/shape-canon/tests/interface_contract.rs` were added. Against the [earlier runs of the same day](#test-runs-2026-11-18), only the row of the crate tests changed: it gains the 2 tests of the new test file, which check the interface file against `shape-predict`, run as a child process. The new test file had not run on macOS or Windows by then; the CI workflow is where it first meets those two systems. None of these runs compares anything with the original.

| Suite | Result |
|---|---|
| `cargo test --locked --no-fail-fast` (Rust 1.94.0) | 44 pass (18 in `effect-params`, 26 in `shape-canon`), 0 fail, 0 ignored |
| `cargo fmt --all -- --check` (Rust 1.94.0 and 1.100.0) | no change needed |
| `cargo clippy --locked --all-targets -- -D warnings` (Rust 1.94.0 and 1.100.0) | no warning |
| `python -m compileall -q tools` | no error |
| `python -I tools/test_status.py` | 52 tests, OK |
| `python -I tools/test_gen_docs.py` | 13 tests, OK |
| `python -I tools/test_check_docs.py` | 32 tests, OK, 1 skipped: the test that reads a term list skips when none is named |
| `python -I tools/gen_docs.py --check` | 8 outputs, 0 drifted, 0 unknown keys |
| `python -I tools/check_docs.py` | 7 pages, 0 findings, 2 checks not run (no term list named) |

## Test runs, 2026-11-18, with the mask sequence crate

On Linux, with Rust 1.94.0, the stable Rust 1.100.0 for the formatting and lint rows as well, and Python 3.12.3, after `crates/mask-seq` and `docs/data/mask_validate_interface.json` were added. Against the [runs with the interface file](#test-runs-2026-11-18-with-the-interface-file), only the row of the crate tests changed: it gains the 44 tests of the six test files of the new crate. Those files had not run on macOS or Windows by then; the CI workflow is where they first meet those two systems. None of these runs compares anything with the original.

| Suite | Result |
|---|---|
| `cargo test --locked --no-fail-fast` (Rust 1.94.0) | 88 pass (18 in `effect-params`, 26 in `shape-canon`, 44 in `mask-seq`), 0 fail, 0 ignored |
| `cargo fmt --all -- --check` (Rust 1.94.0 and 1.100.0) | no change needed |
| `cargo clippy --locked --all-targets -- -D warnings` (Rust 1.94.0 and 1.100.0) | no warning |
| `python -m compileall -q tools` | no error |
| `python -I tools/test_status.py` | 52 tests, OK |
| `python -I tools/test_gen_docs.py` | 13 tests, OK |
| `python -I tools/test_check_docs.py` | 32 tests, OK, 1 skipped: the test that reads a term list skips when none is named |
| `python -I tools/gen_docs.py --check` | 8 outputs, 0 drifted, 0 unknown keys |
| `python -I tools/check_docs.py` | 7 pages, 0 findings, 2 checks not run (no term list named) |

## Test runs, 2026-11-18, with the parameter edge tests

On Linux, with Rust 1.94.0, the stable Rust 1.100.0 for the formatting and lint rows as well, and Python 3.12.3, after `crates/effect-params/tests/edges.rs` was added. Against the [runs with the mask sequence crate](#test-runs-2026-11-18-with-the-mask-sequence-crate), only the row of the crate tests changed: it gains the 46 tests of that file, which build every table in memory. That file had not run on macOS or Windows by then; the CI workflow is where it first meets those two systems. None of these runs compares anything with the original.

| Suite | Result |
|---|---|
| `cargo test --locked --no-fail-fast` (Rust 1.94.0) | 134 pass (64 in `effect-params`, 26 in `shape-canon`, 44 in `mask-seq`), 0 fail, 0 ignored |
| `cargo fmt --all -- --check` (Rust 1.94.0 and 1.100.0) | no change needed |
| `cargo clippy --locked --all-targets -- -D warnings` (Rust 1.94.0 and 1.100.0) | no warning |
| `python -m compileall -q tools` | no error |
| `python -I tools/test_status.py` | 52 tests, OK |
| `python -I tools/test_gen_docs.py` | 13 tests, OK |
| `python -I tools/test_check_docs.py` | 32 tests, OK, 1 skipped: the test that reads a term list skips when none is named |
| `python -I tools/gen_docs.py --check` | 8 outputs, 0 drifted, 0 unknown keys |
| `python -I tools/check_docs.py` | 7 pages, 0 findings, 2 checks not run (no term list named) |

## Test runs, 2026-11-18, with the hexadecimal digit check

On Linux, with Rust 1.94.0, the stable Rust 1.100.0 for the formatting and lint rows as well, and Python 3.12.3, after `crates/effect-params/src/table.rs` was changed to refuse a byte that is not a hexadecimal digit in a `b:` cell and after `0x` in a flags cell, and `crates/effect-params/tests/signs_and_gaps.rs` was added. Against the [runs with the parameter edge tests](#test-runs-2026-11-18-with-the-parameter-edge-tests), only the row of the crate tests changed: it gains the 21 tests of that file, which build every table in memory. That file and the changed parser had not run on macOS or Windows by then; the CI workflow is where they first meet those two systems. None of these runs compares anything with the original.

| Suite | Result |
|---|---|
| `cargo test --locked --no-fail-fast` (Rust 1.94.0) | 155 pass (85 in `effect-params`, 26 in `shape-canon`, 44 in `mask-seq`), 0 fail, 0 ignored |
| `cargo fmt --all -- --check` (Rust 1.94.0 and 1.100.0) | no change needed |
| `cargo clippy --locked --all-targets -- -D warnings` (Rust 1.94.0 and 1.100.0) | no warning |
| `python -m compileall -q tools` | no error |
| `python -I tools/test_status.py` | 52 tests, OK |
| `python -I tools/test_gen_docs.py` | 13 tests, OK |
| `python -I tools/test_check_docs.py` | 32 tests, OK, 1 skipped: the test that reads a term list skips when none is named |
| `python -I tools/gen_docs.py --check` | 8 outputs, 0 drifted, 0 unknown keys |
| `python -I tools/check_docs.py` | 7 pages, 0 findings, 2 checks not run (no term list named) |

## Test runs, 2026-11-24, with the frame and stage crates

On Linux, with Rust 1.94.0 and Python 3.12.3, after `crates/frame-io` and `crates/stage-cli` were added, the table format of `crates/effect-params` gained its version 2, the result file schema gained the `rows_basis` and `frames` blocks, and `contracts/` was taken byte for byte from twixtor at commit 8087d06. Against the [runs with the hexadecimal digit check](#test-runs-2026-11-18-with-the-hexadecimal-digit-check), the crate tests grow by the two new crates and by three test files of `effect-params`, the status tests grow from 52 to 96, the documentation has one more generated output and one more page, and the convention's conformance tool runs for the first time, from a twixtor checkout. The new crates had not run on macOS or Windows by then; the CI workflow is where they first meet those two systems. None of these runs compares anything with the original: the one stage copies frames, and its frames are synthetic.

| Suite | Result |
|---|---|
| `cargo test --locked --no-fail-fast` | 385 pass (147 in `effect-params`, 87 in `frame-io`, 44 in `mask-seq`, 26 in `shape-canon`, 81 in `stage-cli`), 0 fail, 0 ignored |
| `cargo fmt --all -- --check` | no change needed |
| `cargo clippy --locked --all-targets -- -D warnings` | no warning |
| `python -m compileall -q tools` | no error |
| `python -I tools/test_status.py` | 96 tests, OK |
| `python -I tools/test_gen_docs.py` | 13 tests, OK |
| `python -I tools/test_check_docs.py` | 32 tests, OK, 1 skipped: the test that reads a term list skips when none is named |
| `python -I tools/gen_docs.py --check` | 9 outputs, 0 drifted, 0 unknown keys |
| `python -I tools/check_docs.py` | 8 pages, 0 findings, 2 checks not run (no term list named) |
| `python -I tools/conformance.py --stage "<path to>/mocha-stage apply selftest-copy" --full`, run inside a twixtor checkout at commit 8087d06 | level 1: 270 passed; level 3: 33 passed; exit 0 |

## 2026-11-24: the static reading

**What was read.** From 2026-11-23 parts of the original's code were read statically, with disassembly tools and without running it: in the plug-in, the handling of its parameter setup, its render commands and its arbitrary data; in the engine module, the entry path, the reader of the project data held in the plug-in's parameters, and stages of the render path with their arithmetic.

**What was written.** Each read was written up as a specification of method, interfaces and formats: the host interactions in their order, what a stage of a render computes with the formulas, number types and rounding it uses, or the form of the project data. A specification holds no code excerpt and no instruction listing. The specifications are kept outside the repository, like the predicted tables, and code here that rests on one is written from it and from what the emulator shows.

**What was not read.** The code of the original's licence, activation and machine identity checks is never read. Where a read path reaches such a check, only the call and what its caller does with the answer were recorded ([README: left out](../README.md#left-out)). When this entry was written, the arithmetic of the modules that the original gates behind its licence state and the code of the product's other files had not been read.

**What it changes.** A specification is not a measurement. A row that is only read from one is not compared, and a unit that rests on such rows carries the label set for them ([parity: the status rule](parity.md#the-status-rule)). When this entry was written, the original had run in the emulator only for the interface pairing and the calibration, and no case had been compared.

## 2026-11-24: the licence boundary

**The licence boundary.** The licence module's library requests are answered from the emulated machine, its plain folder and environment questions get the emulator's declared answers, its other unanswerable calls get the documented Windows not found or not available failure, machine identity questions from it get their documented failure, and its start-up routine is not run during the engine load ([README: left out](../README.md#left-out)). No answer gives a licensed state. This supersedes the line of [2026-11-17](#2026-11-17-the-judge) that such calls are never answered.

## 2026-11-24: the limits and the scope

**What changed.** The statements of limits and scope were brought in line with the static reading ([README: limits](../README.md#limits)). The scope is stated as the plug-in, its render engine and its planar tracker, each capability to be rebuilt as a stage program for command line use, without the interactive interface of the product's standalone editor. For the modules that the original gates behind its licence state, what their comparison rests on is stated by route. The planar tracker is stated as to be compared within a tolerance on the corners of the tracked surface. The rules for a frame that carries an overlay and for frames supplied by the owner were added to the method ([parity: frames that carry an overlay](parity.md#frames-that-carry-an-overlay)).

**What it rests on.** Readings of the original's code, not runs, and the rules of the method. A reading of the planar tracker found an iterative solver with two code paths and switches whose names show that they exist to manage a difference between them; the size of that difference was not read. No reading is a measurement.

**What has not run.** When this entry was written, no render command of the original had run in the emulator, no frame had been compared, no track had been compared, and no frame had been supplied by the owner. No tool computes the status for a frame that carries an overlay yet.

## Test runs, 2026-11-24, with seven generic crates

On Linux, with Rust 1.94.0 and Python 3.12.3, after `crates/pixel-format`, `crates/contour-eval`, `crates/coverage-raster`, `crates/planar-xform`, `crates/resample`, `crates/lens-model` and `crates/soft-edge` were added, with their seven workspace members and their entries in `Cargo.lock`; `pixel-format` has one dependency edge, to `frame-io`. Against the [runs with the frame and stage crates](#test-runs-2026-11-24-with-the-frame-and-stage-crates), only the row of the crate tests changed: it gains the 932 tests of the seven new crates, and the build row is new. The status tests, the generated outputs and the pages are as before. The new crates had not run on macOS or Windows by then; the CI workflow is where they first meet those two systems. None of these runs compares anything with the original: the seven crates compute from published sources with made up inputs, and none has had a configuration selected by comparison.

| Suite | Result |
|---|---|
| `cargo test --locked --no-fail-fast` | 1317 pass (147 in `effect-params`, 87 in `frame-io`, 44 in `mask-seq`, 26 in `shape-canon`, 81 in `stage-cli`, 98 in `pixel-format`, 132 in `contour-eval`, 188 in `coverage-raster`, 136 in `planar-xform`, 136 in `resample`, 104 in `lens-model`, 138 in `soft-edge`), 0 fail, 0 ignored |
| `cargo build --locked --all-targets` | no error |
| `cargo fmt --all -- --check` | no change needed |
| `cargo clippy --locked --all-targets -- -D warnings` | no warning |
| `python -I tools/test_status.py` | 96 tests, OK |
| `python -I tools/test_gen_docs.py` | 13 tests, OK |
| `python -I tools/test_check_docs.py` | 32 tests, OK, 1 skipped: the test that reads a term list skips when none is named |
| `python -I tools/gen_docs.py --check` | 9 outputs, 0 drifted, 0 unknown keys |
| `python -I tools/check_docs.py` | 8 pages, 0 findings, 2 checks not run (no term list named) |
