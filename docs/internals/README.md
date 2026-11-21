# Internals

This page is for anyone who changes or extends the code with nothing but this repository at hand: the parts and where they live, how a table turns into a verdict, the rules the code keeps, each check with the command that runs it and what it should print, and what lives outside. The two shape crates have a page of their own, [Internals of the shape crates](shape-crates.md); this page gives their place among the other parts and their share of the checks. The user reference is the [reference](../reference.md); the method is in [parity](../parity.md). One internals page per unit joins these as units are rebuilt.

## Contents

- [Shared architecture](#shared-architecture)
- [From a table file to a verdict](#from-a-table-file-to-a-verdict)
- [Rules the code keeps](#rules-the-code-keeps)
- [Checks](#checks)
- [What lives outside the repository](#what-lives-outside-the-repository)
- [Changing the code safely](#changing-the-code-safely)

## Shared architecture

| Part | Path | Role |
|---|---|---|
| Parameter kinds | `crates/effect-params/src/kind.rs` | `ParamKind`, one variant per kind, with the lower-case text name the table format uses (`as_str`, `parse`, the list `ALL`). |
| Parameter flags | `crates/effect-params/src/flags.rs` | `ParamFlags`, a 32-bit set with this crate's own names for its bits; `contains`, `union`, and `diff` for the bits set in exactly one of two sets. |
| Table format | `crates/effect-params/src/table.rs` | `Cell` (unknown, integer or float, compared by `bits_eq`), `ParamDef` (one parameter), `Table` with `parse` and `serialize`, and `ParseError` with the line of the first error. The module comment is the format's definition. |
| Pairing | `crates/effect-params/src/compare.rs` | `normalize_name`, `pair_tables`, and the result types `Pairing`, `ParamVerdict`, `MatchClass` and `FieldVerdict`. |
| Re-exports | `crates/effect-params/src/lib.rs` | The public names of the four modules at the crate root. |
| Tests | `crates/effect-params/tests/edges.rs`, `crates/effect-params/tests/signs_and_gaps.rs`, `crates/effect-params/tests/table_and_pairing.rs` | The format, the kinds, the flags, the normalization and the pairing, on invented names only, and the example table of the [reference](../reference.md#the-table-format); `edges.rs` pins their edges: the text of parse errors and of the serializer, the bounds of the cells, each normalization rule, the claim rules of the pairing, and the fixed flag and kind values; `signs_and_gaps.rs` pins the refusal of signs and of other bytes that are not hexadecimal digits in `b:` and flags cells, with the error texts, and more edges of the float cells, the verdict names, the normalization and the name and kind cells. |
| Shape model | `crates/shape-model/src/lib.rs` | `Contour` with its keyframes of control points, and the points and the bounding rectangle at a query time, by rules labelled as hypotheses ([shape crates](shape-crates.md)). |
| Dump, case form and predictor | `crates/shape-canon/src/` | The canonical dump text and its hexadecimal float text, the case form, and the binary `shape-predict` ([shape crates](shape-crates.md#code-map)). |
| Shape tests | `crates/shape-canon/tests/canon.rs`, `crates/shape-canon/tests/cli.rs`, `crates/shape-canon/tests/interface_contract.rs`, `crates/shape-canon/tests/model_hypotheses.rs` | The number text, the model, the dump and the case form, the exit codes and standard error forms of `shape-predict`, its interface file against the binary, and one test per hypothesis, on invented values only. |
| Mask sequence form and validator | `crates/mask-seq/src/` | SHA-256 written from FIPS 180-4 (`sha256.rs`), the canonical frame file (`pgm.rs`), the `mask-1` manifest (`manifest.rs`), writing a sequence folder and checking one (`validate.rs`), and the binary `mask-validate` (`main.rs`) ([reference: the mask sequence form](../reference.md#the-mask-sequence-form)). |
| Mask sequence tests | `crates/mask-seq/tests/cli.rs`, `crates/mask-seq/tests/interface_contract.rs`, `crates/mask-seq/tests/manifest.rs`, `crates/mask-seq/tests/pgm.rs`, `crates/mask-seq/tests/sha256.rs`, `crates/mask-seq/tests/validate.rs`, with shared helpers in `crates/mask-seq/tests/common/mod.rs` | The hash against NIST test vectors, the frame file, the manifest, each reason word and the order of the checks, the exit codes and streams of `mask-validate`, and its interface file against the binary and the library, on invented values apart from the test vectors. |
| Status rule | `tools/status.py` | Computes a unit's status, rung label and re-run state from its result file. |
| Generated pages | `tools/gen_docs.py` | Writes every figure and table between markers in `README.md` and `docs/`, and `docs/data/facts.json`. |
| Page checks | `tools/check_docs.py` | The rules every page and text file keeps ([reference: page checks](../reference.md#page-checks)). |
| Tool tests | `tools/test_status.py`, `tools/test_gen_docs.py`, `tools/test_check_docs.py` | One test file per tool, on invented fixtures in temporary directories. |
| Result file schema | `docs/data/result_schema.json` | The fields a result file carries. |
| Interface of `shape-predict` | `docs/data/shape_predict_interface.json` | The arguments, streams, dump form name and exit codes of `shape-predict` in a machine-readable form, for programs that run it ([reference: shape-predict](../reference.md#shape-predict)). |
| Interface of `mask-validate` | `docs/data/mask_validate_interface.json` | The argument, streams, names, reason words and exit codes of `mask-validate` in a machine-readable form, for programs that run it ([reference: mask-validate](../reference.md#mask-validate)). |
| Continuous integration | `.github/workflows/ci.yml` | The build and tests of the crates, the tool tests and the two page tools on Linux, macOS and Windows; formatting and lints on Linux. |
| Line endings | `.gitattributes` | Text files are checked out with LF endings on every system, so the page checks read the same bytes everywhere. |

## From a table file to a verdict

1. `Table::parse` reads the predicted table: it skips blank lines and comment lines, requires the magic line `table v1`, and turns each row into a `ParamDef`. A value written in decimal (`f:`) is parsed into a float once, on load.
2. `pair_tables` takes the observed table, the parameters the original registered, as a `Table` too, so both sides are compared in one representation.
3. `pair_tables` claims exact name matches first, in the predicted table's order, then normalized ones; an observed row is claimed at most once.
4. For each claimed pair, every field gets a verdict: the kind `Same` or `Differ`, and the default, minimum, maximum and flags `Same`, `Differ`, `Unscored` (the prediction is unknown) or `ObservedUnknown`. Differing flag sets also give `flag_diff`.
5. The `Pairing` holds one verdict per predicted row and the names of the observed rows no prediction claimed. The caller decides whether the pairing passes; the method's rule is that nothing stays unmatched on either side ([parity: the interface pairing](../parity.md#the-interface-pairing-and-calibration)).
6. `Table::serialize` writes a table back with every float as its exact bits (`b:`). Text that the serializer produced therefore comes back byte for byte after a parse and a second serialize. Text written by hand need not: the serializer drops comments and blank lines, turns `f:` cells into `b:` cells and writes integers and flags in a form of its own, so the text can change while every value read from it is kept. The test `serialized_table_round_trips_but_hand_written_text_does_not` checks the first statement on a table the serializer wrote, and the second on a hand-written table with a comment, a blank line and an `f:` cell.

## Rules the code keeps

These are the rules of `effect-params`. The rules of the shape crates are hypotheses about the original, and [their page](shape-crates.md#the-hypotheses) lists them together with the crates' own choices.

- **Unknown is not zero.** A cell that is `?` is `Cell::Unknown` and stays so; a prediction that is unknown is `Unscored`, never counted as a match or a difference.
- **Floats are compared as bits.** `Cell::bits_eq` compares floats by `to_bits`, so -0 and 0 differ and two NaNs are equal only when every bit is the same. Integers compare by value, and an integer cell never equals a float cell.
- **No rewriting before a comparison.** Neither table is changed to make a field compare; a decimal value is turned into bits once, when it is read, and never re-printed for a comparison.
- **One partner per row.** A predicted row gets at most one observed row and an observed row is claimed once, with exact names before normalized ones.
- **Flags as one set.** Flag sets are compared whole; `diff` and `flag_diff` give the bits that differ.
- **Nothing of the plug-in.** The code and the tests hold no name, value, default or flag of the original; every test name is invented, and the flag names are this crate's own.
- **No dependencies and no `unsafe`.** The crate uses the standard library only.

## Checks

Run from the repository root. These are the commands of [reference: tests](../reference.md#tests).

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

Where only `python3` is installed, use `python3` for `python`. The counts in the right column are those of the [test runs](../parity-log.md#test-runs-2026-11-18-with-the-hexadecimal-digit-check); a change that adds a test or a page changes them.

| Command | Expected result |
|---|---|
| `cargo test --locked --no-fail-fast` | exit 0; the test binaries report `test result: ok.` with 46 passed for `crates/effect-params/tests/edges.rs`, 21 for `crates/effect-params/tests/signs_and_gaps.rs`, 18 for `crates/effect-params/tests/table_and_pairing.rs`, 10 for `crates/shape-canon/tests/canon.rs`, 7 for `crates/shape-canon/tests/cli.rs`, 2 for `crates/shape-canon/tests/interface_contract.rs`, 7 for `crates/shape-canon/tests/model_hypotheses.rs`, 6 for `crates/mask-seq/tests/cli.rs`, 3 for `crates/mask-seq/tests/interface_contract.rs`, 8 for `crates/mask-seq/tests/manifest.rs`, 7 for `crates/mask-seq/tests/pgm.rs`, 5 for `crates/mask-seq/tests/sha256.rs` and 15 for `crates/mask-seq/tests/validate.rs`, 0 failed in each |
| `cargo fmt --all -- --check` | exit 0, no output |
| `cargo clippy --locked --all-targets -- -D warnings` | exit 0, no warning |
| `python -m compileall -q tools` | exit 0, no output |
| `python -I tools/test_status.py` | exit 0; `Ran 52 tests`, then `OK` |
| `python -I tools/test_gen_docs.py` | exit 0; `Ran 13 tests`, then `OK` |
| `python -I tools/test_check_docs.py` | exit 0; `Ran 32 tests`, then `OK (skipped=1)` while no term list is named |
| `python -I tools/gen_docs.py --check` | exit 0; the line `gen_docs --check: 8 outputs, 0 drifted, 0 unknown keys` |
| `python -I tools/check_docs.py` | exit 0; a NOT RUN line for each of the two term list checks, and the last line `check_docs: 7 pages, 0 findings, 2 checks not run` |

## What lives outside the repository

The original plug-in files, the Windows system libraries, the After Effects SDK and every layout made from it, the emulator harness with its register of host answers, the call records, the reference outputs, the predicted parameter tables, and the cases of the shape data check with their predicted dumps ([reproduce: inputs you supply](../reproduce.md#what-needs-inputs-you-supply)). Nothing in the repository needs them to build or to pass its checks.

## Changing the code safely

- **The format.** `serialize` and `parse` must still round trip every bit, which `round_trip_keeps_every_bit` checks, and the serializer's own text must still come back unchanged, which `serialized_table_round_trips_but_hand_written_text_does_not` checks. The example on the reference page is read by `example_parses`, so a format change that the example no longer meets fails `cargo test` until the page is updated. A changed format should get a new magic line, so that a file of the old format is refused rather than misread.
- **The pairing.** A change to `normalize_name` or to the claiming order changes which rows pair. It should come with a test that fails on the old behaviour, and a pairing already reported against a sealed prediction keeps the rule it was scored under.
- **The status rule.** A change to `tools/status.py` changes what every generated page says. Run `python -I tools/test_status.py`, then `python -I tools/gen_docs.py`, then `--check` and `tools/check_docs.py`; a page that changes says what changed and why in the [parity log](../parity-log.md).
- **Names.** Every name in a test or a comment stays invented. A name of the plug-in enters the code only with the unit that needs it for its interface.
- **The shape crates.** A change to a rule of the shape model changes the predictions, and a change to the dump form changes every dump; [their page](shape-crates.md#changing-the-code-safely) says what such a change has to keep.
- **The mask sequence form.** `example_exact_bytes` holds the example manifest, built from two invented frames, and `reference_example_is_canonical` holds its copy on the reference page, so a change to the manifest, to the frame file or to the hash fails `cargo test` until the example and the page change with it. A changed form should get a new `format` value, so that a manifest of the old form is refused (`manifest_format`) rather than misread. The interface file follows the binary: a change to an exit code, a report line or a reason word fails `interface_contract.rs` until the file says the same.
