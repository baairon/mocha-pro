# Parity

This page defines parity for the rewrite, describes how it is checked against the original Mocha Pro 2024 plug-in, and reports what the checks have found. The [README](../README.md#parity) gives the summary; the [parity log](parity-log.md) gives the history by date.

## Summary

**<!--f:status_line-->method and preparation only: 0 cases compared<!--/f-->.** The repository holds the method, the tools that turn result files into the figures on this page, the rewrite's side of the first step, the interface pairing (`crates/effect-params`), and the rewrite's side of a second prepared check, the shape data check (`crates/shape-model` and `crates/shape-canon`). The sections below that describe a check are the specification of that check. What a run measures goes into a result file, and every figure on this page is generated from those files.

**The interface comes first.** Before any output is compared, the parameters the original registers with its host are read in the emulator and paired with a prediction that was sealed before the original first ran, and the original is booted twice to show that it is deterministic under the harness ([below](#the-interface-pairing-and-calibration)). Neither step carries a parity label: they show that the judge runs and that the two sides describe the same controls, not that any output agrees.

**Shape data is predicted before any run as well.** For each case of the shape data check, the text the rewrite predicts for a shape at a list of times was written and sealed before any run of the original, to be compared byte for byte with what the original reports ([below](#the-shape-data-check)). The rules behind the prediction are hypotheses, labelled as such, and no case has been compared.

**Matches comes first, Verified after it.** Each needs a named rung, all in-contract cases compared, no difference, and at least one call record compared, with every compared record equal on the two sides; Verified adds that the mutants exposed the errors planted in the rewrite ([status rule](#the-status-rule)). An independent repeat of the whole check, on fresh copies, then fills the Re-run column.

## Status

Result files: <!--f:units_total-->0<!--/f-->. The status, one part per rung: <!--f:status_line-->method and preparation only: 0 cases compared<!--/f-->. The table is generated from each unit's result file, `parity/<unit>/results/summary.json`, and from the re-run file beside it, `summary.json.rerun.json`, if an independent re-run produced one.

<!--gen:units-->
No result file exists yet, so no unit has a row. A unit is Not started until its `summary.json` is committed.
<!--/gen:units-->

How to read a row:

- **Rung** is the rung of the measurement ladder the file was measured at, and **Rung label** is the label the rule gives its numbers. The label reads none in every case [the status rule](#the-status-rule) gives none, among them while no case is compared, while the tolerance is not 0, and for the rungs the rule does not compute yet.
- **Cases compared** sets the compared cases against the in-contract ones. The rule wants the two equal, so one in-contract case without a reference output holds the row below Matches.
- **Differing** gives cases, then values. **Call records matching** sets the equal records against the compared ones, and **Mutants killed** the exposed mutants against all of them; a mutant is exposed only when the comparison with the original shows it.
- **Re-run** is yes when an independent re-run dated after the result matches it, no with the number of differences, stale when the re-run is not dated after the result or either date cannot be read, and none when there is no re-run file.
- The totals lines count units per rung. Counts are never added across rungs.

## The bar

One check defines parity; nothing else on this page does.

- **The setup.** The original plug-in file runs unmodified in a CPU emulator, Unicorn 2.1.4 through Qiling 1.4.6 on Linux, the versions [twixtor's harness](https://github.com/baairon/twixtor/blob/8087d06ac43023f954de2cc58583b132a3243ff4/parity/harness/README.md#running-it) names, inside an emulated Windows process and behind a mock After Effects host.
- **Identical inputs** reach the original and the rewrite, stand-ins included.
- **No difference is allowed.** Bytes and integers must be equal, and floats equal as bit patterns at a tolerance of 0, so -0 and 0 count as different. How a NaN is compared is chosen once per unit and written in its result file (`comparator.nan_rule`).
- **Three checks per unit:**
  - a [grid](#the-grid-and-the-out-of-contract-rule) of no fewer than <!--f:case_floor-->150<!--/f--> in-contract cases, edge values among them;
  - the [call records](#the-call-record-check);
  - the [mutants](#mutation-checks).
- **Stand-ins.** When the original needs something the harness cannot provide as After Effects or Windows would, the two sides get one shared, labelled [stand-in](#stand-ins-and-logic-parity), and agreement that depends on it is logic parity.

References come from the original inside the emulator, and the emulator follows the architecture manual. Neither real processors nor After Effects on a real machine take part in the bar.

## How parity is measured

This section is the specification. A result file records what was run and with which commands.

1. **Units.** A unit is one measured part of the plug-in's behaviour behind its host interface, checked on its own: an id, the entry the host calls, and the rewrite code that answers it.
2. **The interface pairing.** Once for the plug-in, before any unit: its registered parameters are read in the emulator and paired with the rewrite's sealed prediction, and the original is booted twice ([below](#the-interface-pairing-and-calibration)).
3. **Test cases.** A deterministic generator writes each unit's test set: no fewer than <!--f:case_floor-->150<!--/f--> in-contract cases, edge values among them, plus a list of candidates that may fall outside the contract.
4. **The original side.** The harness loads the unchanged plug-in into the emulator and takes it through the host's sequence of calls, setup and then one call per case, translating the case's inputs on the way in and the result on the way out. What comes out is the reference for that case.
5. **The rewrite side.** The rewrite is given the same case.
6. **The output check.** The two results are compared at a tolerance of 0.
7. **The call-record check.** The requests the original sends to its host are set against the reads of the rewrite ([below](#the-call-record-check)).
8. **Mutants.** Every copy of the rewrite broken on purpose has to alter at least one compared case ([below](#mutation-checks)).
9. **Out of contract.** Cases the original leaves undefined are judged and kept on a separate list ([below](#the-grid-and-the-out-of-contract-rule)).
10. **Finding causes.** Nothing changes until a difference has a cause.
    - The harness is suspected first: a host answer, an emulator setting.
    - The rewrite comes second, and it changes only to do what the original was seen doing.
    - A difference whose cause is not shown stays in the count.
    - Causes are hunted by running the original on crafted inputs, one setting changed at a time.
11. **Verification.** The [status rule](#the-status-rule) decides Verified. After that, an independent repeat on fresh copies, which makes the original's outputs again instead of reusing them, writes the re-run file.

The heavy work, builds, emulation and grids, happens on one Linux machine, which keeps the original's files, the references, the call records and the inputs. The repository receives only the small result files, once there are any, and the tools that read them.

### The interface pairing and calibration

The first comparison is of parameter tables, not of outputs. When the plug-in sets itself up, it registers its parameters with the host: for each one an id, a kind, a name, a default, a range and flags. The harness reads that table from the emulated original. Before the original first runs, the rewrite's prediction of the same table is written and sealed, with the SHA-256 of the file and a UTC time, so that no prediction can be adjusted to what the run shows. A sealed file is never edited, and a row changed after the original's output has been seen is void.

The pairing then matches each predicted row with at most one observed row: by the host parameter id where both rows carry one, otherwise by the exact name, then by a normalized name, otherwise not at all. Defaults, ranges and flags are compared as bits, never as printed digits, and neither side is rewritten before the comparison. A prediction left unknown is never read as zero; it is not scored. The pairing passes only when nothing is unmatched on either side, and the field differences are counted beside it. `crates/effect-params` holds the rewrite's side of this: the table format and the comparison ([reference: the pairing](reference.md#the-pairing)).

The calibration boots the original twice, in separate processes, and compares the two call records and the two dumps of what it registered, byte for byte. A difference there means the harness is not deterministic, and nothing it produces is used as a reference until the cause is found.

Equal tables do not mean equal output: two parameters can share a name and a range and still render differently, and only the grids can tell. The pairing and calibration figures of every result file that has them:

<!--gen:pairing-->
No result file records an interface pairing or a calibration yet.
<!--/gen:pairing-->

### The shape data check

The second prepared check compares shape data, not rendered output. A case describes one shape, a contour of Bezier control points stored as keyframes, and a list of query times. For each case the rewrite predicts, with `shape-predict`, a text dump of the points at every query time and, where the case asks for them, the rectangle around their anchor positions and four meta values ([reference: the dump form](reference.md#the-dump-form)). The predicted dumps were written and sealed before any run of the original, and they are kept private, as the cases are. On the original's side the harness, which is not in this repository, is built to have the original build the same shape from the same case and to write what it reports in the same text; the two texts are then compared byte for byte. Every float other than NaN keeps all its bits in that text, so equal text means equal values, with -0 and 0 apart; a NaN is written without its sign or payload, so NaNs are compared as a class. A value predicted as `UNKNOWN` is never counted as a correct prediction.

The predictions rest on hypotheses, not on measurements: how the points are kept at a key, blended between keys and held outside them, and how the rectangle is spanned and its ties are broken. Each is labelled INFERRED in the code and pinned by a test ([internals: the hypotheses](internals/shape-crates.md#the-hypotheses)). Only a compared case can confirm or refute one, and no case has been compared. Like the pairing, this check is not the bar: it tests what the rewrite predicts the original reports about a shape, not that any rendered output agrees.

### The grid and the out-of-contract rule

Each unit's grid needs at least <!--f:case_floor-->150<!--/f--> in-contract cases that were compared, with edge values in the parameters and in the inputs: 0, -0, denormals, the infinities, NaN and the ends of every range, at odd and tiny sizes besides the usual ones.

The contract holds the cases whose behaviour the original defines. The rest go on the Out of contract list, where none is a pass or a failure and none counts toward the floor. Only the original's own behaviour moves a case off the contract: it refuses the case, stops on it, is handed a value beyond the range it registers, or gives an output it does not define. Until it is judged against what the original accepts, such a case is only a candidate.

### The call-record check

Every request of the original reaches the mock host and is written down: the inputs it checks out, the times it asks for, the parameters it reads. The rewrite writes its own reads in the same form, and the two lists are compared case by case. The reason is that equal output can hide unequal input: a rewrite that fetches another frame, or samples a parameter at another time, may still produce the same bytes when no case in the grid tells the two apart. Turning the record on must leave the rewrite's output unchanged, and equal output hashes with and without it show that.

### Mutation checks

A grid that has never failed has not shown that it can. So the rewrite is copied and broken on purpose, one small error per copy: two operands swapped, a rounding changed, a term left out, two weights exchanged. Each such mutant must make at least one compared case differ from the original, and only a difference from the original's output counts as a kill. Mutants are built again from the code as it stands before any count. One that no case exposes is a survivor, settled either by a case added to expose it or by a written argument, checked by a named reviewer, that it can never change an output.

### Stand-ins and logic parity

Some inputs of the original cannot be produced the way After Effects or Windows would produce them. Then both sides read one labelled stand-in, the result file lists it under `stand_ins` or `harness_stand_ins`, and agreement that depends on it is logic parity: it shows that the two programs treat that value alike, and nothing about what a real host would give. Stand-ins are taken from the host's public documentation and never from the rewrite's habits, since a stand-in copied from the rewrite would make every case that reads it prove itself.

### The status rule

`tools/status.py` applies the rule to the numbers of each result file; `tools/gen_docs.py` writes its answers into these pages.

| Status | Means |
|---|---|
| Verified | as Matches, and the survivor rule holds: at least ten mutants, at least nine in ten killed outright, and every survivor caught by an added case or argued equivalent with a named reviewer |
| Matches | a rung that is not null; at least <!--f:case_floor-->150<!--/f--> cases compared and every in-contract case compared; a tolerance of 0; 0 differing cases and 0 differing values; at least one call record compared, and every compared call record matching |
| In progress | a valid result file exists, and neither rule holds yet |
| Blocked | something outside the unit stops it, named in `blockers`; checked right after the file is found valid, because a blocked unit does not match |
| Not started | no result file; the unit has no row |
| invalid result file | the file contradicts itself: identical and differing cases do not add up to the compared ones, or more cases are compared than are in contract. It gets no status |

Two of these conditions close gaps that the case counts alone would leave open. A call-record check with 0 records compared is unmet, however the cases came out. A result file whose rung is null, one that carries only the interface pairing and the calibration, is never a parity unit: it cannot reach Matches or Verified, and the totals print it on a line of its own, never inside the count of a rung.

The rung label is computed apart from the status. There is no label when the tolerance is not 0 or no case is compared. A call-record rung is labelled when call records were compared and all match. An output rung is labelled only with 0 differing cases and values, and its full label needs the evidence floor of the method: enough cases and values for the size of a case, every branch taken both ways or unreached with a reason, every allowed value class present, every listed arithmetic site killed by a mutant, the survivor rule, a control build that matches the release build, and no suspended stand-in. A file that misses a floor item gets the thin form of the label with the unmet items listed. The exact numbers are in `tools/status.py`. A re-run counts only when it is dated after the result.

**Rows that are read, not compared.** A unit may state, in a `rows_basis` block of its result file, that some of its rows were only read from a specification and that the rest were seen in a partial observation of the original. Such a unit reads `Read and partly observed`, carries the fixed label `partial observation, not a result of record`, has no rung label and counts toward no total of matching units, whatever its other fields say.

### Wording

- Every factual line is MEASURED, which names the result file that holds the number, or INFERRED, which means specified or reasoned.
- The allowed claim is "matches the emulated original", always with the rung, the counts and the judge.
- Never used before a rung's gates pass: a word that says the work is at its end, "identical to the original", "matches the original" while the judge is an emulator, a share of a product, or a count added across rungs.
- When a claim falls, the record says what is withdrawn, what still stands and why.

`tools/check_docs.py` checks these rules on every page.

## What the results do not cover

- **A grid is a sample.** A match on chosen cases is strong evidence for exactly those inputs and no proof for the rest.
- **References come from the emulated original.** Where the original depends on a real host, the two sides share a [stand-in](#stand-ins-and-assumptions).
- **Cases without a reference.** An in-contract case that has no reference output cannot be compared, and it holds its unit below Matches; the gap shows in the Cases compared column.
- **One instruction-set route.** The emulated CPU has no AVX tier. Where the original dispatches on CPU features, the path for that set is the only one that runs; every result carries the qualifier that CPUs with other tiers run a different body, which is not reproduced.

### Out of contract

A case the original refuses, stops on, or leaves partly undefined is counted under `cases.out_of_contract` in the result file, apart from the compared cases, and never as a pass or a failure. A value the original never initialises is not defined behaviour: it is judged out of contract on evidence, listed case by case, and never encoded in the rewrite.

### Stand-ins and assumptions

The stand-ins and qualifiers each result file states are listed in the [per-unit notes](#per-unit-notes). Every assumption that changes output bits, semantics or what a unit is gets an id, what it is, why it matters and the options, and it is decided by a reviewer other than the one who wrote the code. A result file that rests on one cites its id, so it shows in the notes.

The harness-wide stand-ins the harness is built to apply to every run. The harness is not in this repository, so this list describes its design and cannot be checked from it:

- **Unknown host calls fault.** A call with no answer stops the run and names itself. None is answered with success or with 0.
- **The emulated CPU.** A fixed x86-64 feature set with no AVX tier. A vector instruction outside it, or a feature query the table does not list, stops the run.
- **Floating point start state.** The Windows defaults: every exception masked, round to nearest, no flush to zero.
- **Stacks and threads.** The main thread's stack is 1 MiB, the stand-in value of [twixtor's harness](https://github.com/baairon/twixtor/blob/8087d06ac43023f954de2cc58583b132a3243ff4/parity/harness/README.md#stand-ins) for a host value that is not known, and threads switch only when one waits, in one reproducible order.
- **Time and randomness.** Never the machine's own: a time read stops the run until a fixed clock is registered, and random bytes come from a seeded stream.
- **Host services.** A service on the declared list of absent ones is answered "not found"; a request with no declared answer stops the run.
- **Licence, activation and machine identity.** The licence module gets only the boundary answers listed in [README: left out](../README.md#left-out), none of them a licensed state or a machine identity value; any other call it makes to the host stops the run, and so does a licence or activation call from any other part of the original.

### The NaN rule

Given two NaN operands, the emulator and a real x86 processor may keep different ones, and processor families also disagree on the NaN that an invalid operation creates. Since the references are made in the emulator, the rewrite's arithmetic takes the emulator's choice by default, and a result says so where a real processor could give another payload or sign. Which rule a unit uses, and whether NaNs are compared bit for bit or as a class at a tolerance of 0, is written in its result file.

### Frames

A unit whose outputs are frames may carry an optional `frames` block in its result file: the geometry set compared (width, height and depth, with the frames compared and identical at each), the totals, and a `reference_kind` of either `emulated original` or `owner supplied frames`. The comparator applies per depth. Depth 8 and 16 samples are unsigned integers: the tolerance counts integer code steps, and the NaN and signed zero rules do not apply because such a sample cannot hold either. Depth 32 samples are floats: the tolerance counts float ulps and the result file's `nan_rule` and `signed_zero` apply as written.

Frames compared against `owner supplied frames` are never a judgement by the emulated original. Such a unit reads `Owner supplied frames`, never reaches Matches or Verified whatever its counts, prints the fixed label "owner supplied frames, not the original, not a parity result", and is listed on its own line apart from the rung lines. The block states counts only; the tools computing it are generic and say nothing about any plug-in.

### Caveats

- **Platform math.** A routine such as a power or a logarithm runs inside the emulator on the original's side and as the rewrite's own code on the other. A last-bit difference there appears as a differing value and is reported like any other.
- **Real data.** The inputs are synthetic and small; real footage and real projects are not exercised.
- **Mutants are chosen by the authors.** A kill count depends on which mutants were written, and an equivalence argument is reasoning, not measurement.
- **Stand-ins prove logic only.** When the true data arrives, some stand-in values may turn out wrong, though many may never reach an output.

## Root causes found

Each difference ever found between the original and the rewrite goes here with its cause, harness or rewrite, the change it led to and the units it touched. The list begins with the first compared case. The rewrite keeps only the rules and the arithmetic needed to match; none of the original's code, binaries or documentation is in it.

## Reproducing the results

The original plug-in, the Windows system libraries for the emulator's file system, Adobe's After Effects SDK, the harness, the predicted tables, and the cases of the shape data check with their predicted dumps are not in the repository. [Reproduce](reproduce.md) says what each one is for and where it comes from, and lists the steps in order.

## Terms

| Term | Meaning |
|---|---|
| unit | one measured part of the plug-in's behaviour, checked on its own: an id, an entry, the rewrite code that answers it; the totals lines call it a module |
| judge | the engine that runs the original: here an emulator that follows the architecture manual, not silicon |
| result file | the machine-written record of one unit at one rung, in the fields of `docs/data/result_schema.json` |
| rung | a level of the measurement ladder: what was compared, and how much it lets a claim say |
| predicted table | the rewrite's table of the plug-in's parameters, written and sealed before the original runs |
| pairing | the match of the predicted table with the table the original registers, name by name, fields as bits |
| calibration | two boots of the original that must give the same call record and the same dump |
| shape case | a text file that describes one contour stored as keyframes and the times at which to query it, read by `shape-predict` |
| predicted dump | the text the rewrite predicts for a shape case, in the dump form of the reference, written and sealed before the original runs |
| hypothesis | a rule of the rewrite that no measurement backs yet; the code labels it INFERRED |
| in contract | a case whose behaviour the original defines |
| out of contract | a case whose behaviour the original leaves undefined; counted apart, neither pass nor failure |
| reference | what the original outputs for a case, produced in the emulator |
| call record | the list of the original's requests to its host, set against the list of the rewrite's reads |
| mutant | a copy of the rewrite with an error put in on purpose, which the grid has to expose |
| survivor | a mutant that no compared case exposed |
| kill | a mutant that makes at least one compared case differ from the original's output |
| stand-in | a labelled value that both sides read where the harness cannot give what a host would |
| logic parity | agreement that depends on a stand-in |
| single judge | a result from one engine, not cross-checked on a second |

## Per-unit notes

Taken from each unit's result file: its blockers, stand-ins, logic-parity count, qualifiers, surviving mutants and the paths it leaves unrebuilt.

<!--gen:notes-->
No result file exists yet, so there are no notes.
<!--/gen:notes-->
