# Reproduce

Which checks run on this repository by itself, which inputs have to come from outside it, and how the evidence is produced again from them. The short form is the README's [How to run the verification](../README.md#how-to-run-the-verification).

## What reruns here

Without any outside input the crates build and test, the predictor writes the dump of any case you write, the documentation regenerates and checks, and the status of any result file can be computed:

| Command | Checks |
|---|---|
| `cargo test --locked --no-fail-fast` | the six crates build, and their tests pass |
| `cargo fmt --all -- --check` and `cargo clippy --locked --all-targets -- -D warnings` | formatting and lints, every warning an error |
| `python -I tools/test_status.py`, `test_gen_docs.py` and `test_check_docs.py` | the tests of the three tools |
| `python -I tools/gen_docs.py --check` | every generated figure and table equals what the result files give; exits 1 when a page is stale |
| `python -I tools/check_docs.py` | the figures, links, wording and hexadecimal addresses of every page, and ASCII, dashes and secrets in every text file; the two checks that need the term list kept outside the repository print NOT RUN |
| `python -I tools/status.py FILE...` | the status, rung label and re-run state of the result files named |
| `cargo run --locked --release -p shape-canon --bin shape-predict -- CASE` | the dump the rewrite predicts for a case written in the [case form](reference.md#the-case-form) |
| `cargo run --locked --release -p mask-seq --bin mask-validate -- SEQUENCE` | whether a folder of mask frames, or its manifest, is a valid sequence of the [mask sequence form](reference.md#the-mask-sequence-form), and the first fault if it is not |

Where only `python3` is installed, use `python3` for `python` in these commands.

Every figure in the documentation is written from the result files under `parity/`; `docs/data/facts.json` lists each one with its source. Without the inputs below, a result file can be read and its figures regenerated, but nothing in it can be measured again.

## What needs inputs you supply

None of these is in the repository, and none ever will be:

| Item | Used for | Where it comes from |
|---|---|---|
| the original plug-in files | the judge: they run unmodified in the emulator and give every reference output | your own installation of Mocha Pro 2024; the harness is built to map them read-only and to check their SHA-256 before each boot |
| the Windows system libraries the loader reports missing | the emulator's root file system | your own Windows installation, listed in one file and never committed |
| Adobe's After Effects SDK | the mock host: the harness compiles the layouts of the host's structures from its headers | Adobe; it is not redistributable, so obtain it yourself |
| the emulator | running the original | Qiling 1.4.6 on Unicorn 2.1.4, the versions [twixtor's harness](https://github.com/baairon/twixtor/blob/8087d06ac43023f954de2cc58583b132a3243ff4/parity/harness/README.md#running-it) names, installed in a Python virtual environment of your own; the repository does not depend on it |
| the harness | loading the original, the mock host, the call record, the register of host answers, the dump of what the original registers, and the side of the shape data check that has the original build each case's shape and write what it reports | it starts from the public harness of [twixtor](https://github.com/baairon/twixtor), `parity/harness/` at commit 8087d06; the changes for this plug-in are kept private because they carry layouts compiled from the SDK and tables made from the original's files |
| the predicted parameter tables | the interface pairing | written from the product's documentation and sealed before the first run; private, like every name and value taken from that documentation. Write your own in the [table format](reference.md#the-table-format) |
| the shape cases and their predicted dumps | the [shape data check](parity.md#the-shape-data-check) | written for this project; the dumps were written and sealed before any run of the original, and both stay private. Write your own cases in the [case form](reference.md#the-case-form) and predict them with `shape-predict` |

## Re-creating the evidence

With those inputs in place, in this order. Each step that produces a figure writes it into a result file, with the exact commands in its `commands` field.

| Step | Action |
|---|---|
| 1. predict | write the predicted parameter table and seal it: the SHA-256 of the file and a UTC time, recorded before the original first runs; a sealed file is never edited |
| 2. calibrate | boot the original under the harness twice, in separate processes, and compare the two call records and the two dumps of what it registered, byte for byte |
| 3. pair | pair the dumped parameters with the sealed table ([reference: the pairing](reference.md#the-pairing)); nothing may stay unmatched on either side |
| 4. references | for each unit, run the original on the unit's test set in the harness, and keep its outputs and its call record |
| 5. rewrite | run the rewrite over those cases and record its reads |
| 6. compare | compare the outputs with a tolerance of 0 and the call records case by case, run the mutants, and write `parity/<unit>/results/summary.json` in the fields of `docs/data/result_schema.json` |
| 7. regenerate | `python -I tools/gen_docs.py` writes every figure and table of the documentation from the result files |
| 8. re-run | on fresh copies, with the original's outputs made again rather than reused, repeat steps 4 to 6 and write the re-run file beside the result file |

The shape data check follows the same order. Its cases and their predicted dumps are written and sealed before the original first runs, like the table of step 1; after the calibration, the harness has the original build each case's shape and write what it reports, and that text is compared byte for byte with the predicted dump.

The original runs in the emulator on a Linux machine. Keep one emulator process alive across the cases of a unit rather than booting it for each case, and check that a case run after an error in the plug-in gives the same output as it does after a fresh boot.
