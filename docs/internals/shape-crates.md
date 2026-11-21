# Internals of the shape crates

This page covers `crates/shape-model` and `crates/shape-canon`: what they compute, where each part lives, how a case becomes a dump, the hypotheses with the code and the test behind each, the checks that run from this repository, what lives outside it, and what a change has to keep. The user reference starts at [reference: the shape model](../reference.md#the-shape-model); the check the two crates serve is described in [parity: the shape data check](../parity.md#the-shape-data-check). The architecture of the rest of the repository is in [Internals](README.md).

## Contents

- [What they compute](#what-they-compute)
- [Code map](#code-map)
- [From a case to a dump](#from-a-case-to-a-dump)
- [The hypotheses](#the-hypotheses)
- [Checks](#checks)
- [What lives outside the repository](#what-lives-outside-the-repository)
- [Changing the code safely](#changing-the-code-safely)

## What they compute

Given a case, the two crates compute the text the rewrite predicts for it: for one contour of Bezier control points stored as keyframes, the points at each query time, and, when the case asks for them, the rectangle around their anchor positions and four meta values. `shape-model` holds the shape and answers two questions, the points at a time and the rectangle at a time. `shape-canon` reads the case, asks the model, and writes every answer in one canonical text. The binary `shape-predict` joins the two for one case file.

The answers rest on hypotheses. The original has not been run on any case of the shape data check, so nothing here is measured against it, and every rule about how the original's shape data behaves is labelled INFERRED in the code. A hypothesis can be confirmed or refuted only when the shape data check compares cases, and no case has been compared.

## Code map

| Part | Path | Role |
|---|---|---|
| Point, keyframe, rectangle | `crates/shape-model/src/lib.rs`: `PointData`, `Keyframe`, `Rect` | the stored values: per point two flags, the anchor position, two optional numbers and two tangents; a key time with its points; left, top, width and height |
| Contour | the same file: `Contour::new`, `keys`, `point_count`, `ModelError` | the keyframes of one shape, checked when it is built |
| Query | the same file: `Contour::points_at`, `lerp`, and the private `blend` and `lerp_opt` | the points at a time: a hold, a key's own points or a blend |
| Rectangle | the same file: `Contour::bounds_at` | the rectangle around the anchor positions at a time |
| Number text | `crates/shape-canon/src/hexfloat.rs`: `fmt_hex`, `parse_hex` | the one canonical hexadecimal text of a 64-bit float, and its reader |
| Dump writer | `crates/shape-canon/src/lib.rs`: `DumpWriter` (`record`, `point`, `finish`), `fmt_bool`, `fmt_opt` | one `key=value` record per line, and the eight records of a point |
| Dump | the same file: `render_dump`, `predict`, `UNKNOWN` | the records in their fixed order over contours and query times; a case to its bytes |
| Case form | `crates/shape-canon/src/case.rs`: `Case::parse`, `Case`, `Meta`, `CaseError` | the case text to keys, query times, the two switches and the meta values |
| Command | `crates/shape-canon/src/main.rs` | `shape-predict CASE`: read, parse, predict, write to standard output |
| Tests | `crates/shape-canon/tests/canon.rs`, `crates/shape-canon/tests/cli.rs`, `crates/shape-canon/tests/interface_contract.rs`, `crates/shape-canon/tests/model_hypotheses.rs` | the number text, the model, the dump and the case form; the exit codes and standard error forms of `shape-predict`, run as a child process, and its interface file checked against the binary; the hypotheses pinned one by one |

## From a case to a dump

1. `shape-predict` reads the file named by its first argument as UTF-8 text. A missing argument or a file it cannot read ends the run with exit code 2.
2. `Case::parse` reads the records line by line: the switches `bounds` and `meta`, the query times in their order, each `key` record starting a `Keyframe` and each `pt` record adding a `PointData` to the last one, and the meta values as text. Every number goes through `parse_hex`. The first bad line stops it with a `CaseError` that names the line; a case without `case=1` fails after the last line, reported at line 0.
3. `predict` builds a `Contour` from the keys with `Contour::new`, which checks that there is a key, that every key time is finite, that every key has the same number of points and that the key times strictly increase.
4. `render_dump` writes the header records and then, for each query time in order, the time, each point that `points_at` gives, through `DumpWriter::point`, and, when the case asks for it, the rectangle that `bounds_at` gives. The meta section follows, with the four values only when the case asks for them; a value the case does not give is `UNKNOWN`, the placeholder for a value the predictor has no basis for.
5. Every float goes through `fmt_hex`, every optional number through `fmt_opt` (`none` when it is absent) and every flag through `fmt_bool`. `finish` gives the bytes; `shape-predict` writes them to standard output and exits 0. A refused case exits 1 with the reason on standard error.

`points_at(t)` decides in this order. A NaN `t`, or a `t` at or before the first key time, gives the first key's points; a `t` at or after the last key time gives the last key's points. Otherwise it finds the last key `a` whose time is at or before `t` and the key `b` after it, returns the points of `a` when `t` equals its time, and else blends each point of `a` with the same point of `b` at the weight `u = (t - a.time) / (b.time - a.time)`. `bounds_at(t)` takes those points and keeps the smallest and the largest `x` and `y`, comparing with strict `<` and `>`.

## The hypotheses

Every rule in this table is about how the original's shape data behaves, and none has been measured. Each is labelled INFERRED in the module comment of `crates/shape-model/src/lib.rs`, and each has a test that holds the crate to it, so a change to a rule fails a test until the test changes with it.

| Hypothesis (INFERRED) | Applied in | Pinned by |
|---|---|---|
| At a key time, the points are the stored ones. | `points_at`: the return of a key's own points when `t` equals its time, and the holds at the first and the last key | `interpolation_holds_and_lines` (at both keys); `negative_zero_at_inner_key_kept` (a negative zero at the first key and at an inner key) |
| Between two keys each number is blended linearly in time. | `blend` and `lerp`, at the weight `u` above | `interpolation_holds_and_lines` (halfway); `quarter_point_not_symmetric` (at a quarter and at three quarters); `three_keys_pick_segment` (the segment among three keys) |
| The blend is computed as `a + (b - a) * u`, with the weight measured from the earlier key. | `lerp` | `quarter_point_not_symmetric`, which compares the bits of the result with that expression on a segment 3 long |
| The two flags, and an optional number that either key lacks, keep the earlier key's value between keys. | `blend` for the flags, `lerp_opt` for the optional numbers | `flags_hold_earlier_key` (both flags); `interpolation_holds_and_lines` (one flag, and an optional number the earlier key lacks) |
| Before the first key and after the last, the nearest key's points hold. | `points_at`: the comparisons with the first and the last key time | `interpolation_holds_and_lines` |
| The rectangle spans the anchor positions of all points. | `bounds_at` | `bounds_span_anchors` |
| Of equal extreme coordinates the rectangle keeps the first point's value, which decides the sign of a zero extreme. | `bounds_at`: the strict comparisons | `bounds_signed_zero` |

For two rules, the rounding form and the tie rule, the comments of their tests add that they are hypotheses of the crate that have not been compared with the original. The rounding form matters because `a + (b - a) * u` and other forms of a linear blend, such as `a * (1 - u) + b * u`, can differ in the last bit. The tie rule matters because -0 and 0 compare equal while their texts differ. As the comment of `lerp` says, the form gives `a` exactly at `u` = 0, but gives `b` at `u` = 1 only when `b - a` is exact, so a weight that rounds to 1 between two keys need not give the later key's value.

Choices of the crates that are not hypotheses about the original:

- A NaN query time gives the first key's points (`points_at`; `nan_query_holds_first`).
- A contour whose keys hold no point has a rectangle of four zeros (`bounds_at`; no test covers it).
- A NaN is written `nan`, with no sign and no payload, so the dump tells NaNs apart only as a class (`fmt_hex`; `hex_known_values`).
- The record names, their order and the `format=shape-1` line are this crate's own form, not a file layout of the original (`render_dump`; `predict_exact_bytes`). The harness side of the check is built to write what the original reports in the same form, so a difference between the two texts is a difference of values, not of layout.

## Checks

Run from the repository root. Both crates are covered by the commands of [Internals: checks](README.md#checks); the first command below runs only the tests of these two.

| Command | Expected result |
|---|---|
| `cargo test --locked --no-fail-fast -p shape-canon` | exit 0; `tests/model_hypotheses.rs` reports `test result: ok. 7 passed; 0 failed`, `tests/canon.rs` reports `test result: ok. 10 passed; 0 failed`, `tests/cli.rs` reports `test result: ok. 7 passed; 0 failed`, and `tests/interface_contract.rs` reports `test result: ok. 2 passed; 0 failed` |
| `cargo run --locked --release -p shape-canon --bin shape-predict -- CASE` | for a valid case, exit 0 and its dump on standard output; for a refused one, exit 1 and the reason on standard error ([reference: shape-predict](../reference.md#shape-predict)) |

`shape-model` has no test of its own: `cargo test -p shape-model` builds it and reports 0 tests, and `tests/canon.rs` and `tests/model_hypotheses.rs` exercise it through its public types.

## What lives outside the repository

The cases of the shape data check and the dumps predicted for them, which were written and sealed before any run of the original and are kept private, and the harness side of the check, which is built to have the original build the same shape from each case and to write what it reports in the dump form. Nothing in the repository needs them to build or to pass its checks.

The tests of the two crates are also probed with deliberately broken copies of the crates, one planted error per copy, to see whether a test notices. Those runs and their records are kept outside the repository, so this page gives no count for them.

## Changing the code safely

- **A hypothesis.** A change to `points_at`, `bounds_at`, `blend`, `lerp_opt` or `lerp` changes the predictions. Change the list in the module comment and the test that pins the rule in the same change, and keep the rule labelled INFERRED: only a compared case turns a hypothesis into a measurement. Predictions already sealed are never edited. New ones are written and sealed apart, and a prediction changed after the original's output has been seen does not count.
- **The dump form.** `predict_exact_bytes` holds a whole dump, so a change to the records, their order or the number text fails it until the test changes. A changed form should get a new `format` value, so that a dump of the old form is never read as one of the new.
- **The number text.** `fmt_hex` has to keep one text per value and every bit of every value but NaN; `hex_known_values`, `hex_round_trips_bits`, `hex_rejects_noncanonical` and `hex_reads_minus_zero_exponent` check it. `parse_hex` also reads an exponent written `-0` ([reference: the dump form](../reference.md#the-dump-form)); a change that refuses it changes which case files are accepted, never a dump.
- **Names.** The names of the fields and of the records are this rewrite's own, and every value in the tests is invented. A name of the original enters the code only with the unit that needs it for its interface.
