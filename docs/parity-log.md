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

**The original in the emulator.** The original plug-in was started in the emulator, unmodified, under the fault rule, by which a call the harness has no answer for stops the run and names itself. The runs were for the first step of the method, the interface pairing and the calibration ([parity: the interface pairing and calibration](parity.md#the-interface-pairing-and-calibration)). The code that computes the original's output was not read or traced.

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
