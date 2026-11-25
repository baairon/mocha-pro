# mocha-pro

An independent open source reimplementation of the Mocha Pro 2024 plug-in, written in Rust. Its output is to be judged case by case against the original plug-in running unmodified in an emulator, and the status computed from the result files reads: <!--f:status_line-->method and preparation only: 0 cases compared<!--/f--> ([parity](#parity)). It is not affiliated with or endorsed by any other company, product or library.

The repository holds the method, the tools that compute and check its status, and six Rust crates. It reimplements no output of the plug-in yet: the one frame stage it offers is a self test that copies frames.

## Contents

- [What it is](#what-it-is)
- [Parity](#parity)
  - [Status](#status)
  - [How to run the verification](#how-to-run-the-verification)
  - [Assumptions and stand-ins](#assumptions-and-stand-ins)
- [Limits](#limits)
  - [Left out](#left-out)
- [Legal](#legal)
- [License](#license)

## What it is

mocha-pro is meant to rebuild a plug-in from what the original is observed to do and from specifications written from static reads of its code ([Legal](#legal)). Its scope is the plug-in, its render engine and its planar tracker, each capability to be rebuilt as a stage program for command line use; the interactive interface of the product's standalone editor is not rebuilt ([Left out](#left-out)). The aim is output equal to the original's on every compared case, from new code that contains none of the original's; the planar tracker is an exception, to be compared within a tolerance ([Limits](#limits)). The judge is the original itself: its unmodified plug-in file runs in a CPU emulator behind a mock host, the same inputs go to it and to the rewrite, and the two outputs are compared with no tolerance (the planar tracker excepted, as stated). Parity is claimed one unit at a time, a unit being one measured part of the plug-in's behaviour, never for the whole product at once. Every status is computed from result files by a tool; none is typed by hand.

What is in the repository:

- **`crates/effect-params`**, a Rust library with no dependencies. It holds parameter kinds and flags as plain types, a text format for a table of parameter definitions with a loader and a serializer, and the pairing that matches a predicted table with an observed one, by parameter id where both tables carry one and by name otherwise, with defaults, ranges and flags compared as bits. It is the rewrite's side of the first step of the method, the interface pairing ([reference: the crates](docs/reference.md#the-crates)). It contains no parameter of the plug-in: its tests use invented names.
- **`crates/shape-model` and `crates/shape-canon`**, the rewrite's side of a second prepared check, the [shape data check](docs/parity.md#the-shape-data-check). `shape-model` stores a shape as one contour of Bezier control points per keyframe and gives its points and the rectangle around them at any query time. Its rules are hypotheses, labelled as such in the code, since none of them has been measured. `shape-canon` reads a case in a plain text form and writes what the rewrite predicts for it as a canonical text dump, every float in a hexadecimal text that keeps all its bits, NaN aside; it also builds `shape-predict`, the command that does this for one case file ([reference: the shape model](docs/reference.md#the-shape-model)). Their tests use invented values only.
- **`crates/mask-seq`**, a crate with no dependencies for `mask-1`, a container form of this repository's own for a sequence of one-channel mask frames: one folder that holds the frames as PGM files with one canonical header, at 8 or 16 bits per sample, and a manifest, `manifest.txt`, that lists them with the SHA-256 of each file. Its library writes such a folder and checks one, and it builds `mask-validate`, the command that checks one sequence and reports the first fault it finds ([reference: the mask sequence form](docs/reference.md#the-mask-sequence-form)). The crate stores and checks frames that it is given; nothing in the repository computes a mask. Its tests use invented values, apart from six published NIST test vectors for SHA-256.
- **`crates/frame-io`**, a crate with no dependencies for the raw frame contract of the [handoff convention](docs/handoff.md): whole frames with no header at 8, 16 or 32 bits per channel. Its library reads and writes such streams, generates synthetic frames that are the same on every platform, and compares two streams as bit patterns with no tolerance. It builds `frame-gen` and `frame-cmp`, the commands that write a pattern and compare two streams ([reference: the frame contract](docs/reference.md#the-frame-contract)). It computes nothing of the plug-in.
- **`crates/stage-cli`**, a crate with no dependencies that builds `mocha-stage`, a stage program in the handoff convention of twixtor and sapphire. It offers one stage, `selftest-copy`, a self test that copies frames unchanged and reimplements nothing. The repository holds the convention's `contracts/` folder byte for byte as twixtor publishes it at commit 8087d06 and does not hold its conformance tool, which was run from a twixtor checkout at that commit against the built program and passed ([reference: `mocha-stage`](docs/reference.md#mocha-stage)).
- **The status tools.** `tools/status.py` computes a unit's status from its result file, `tools/gen_docs.py` writes every parity figure and table of these pages from the result files, and `tools/check_docs.py` checks the pages ([reference: documentation tools](docs/reference.md#documentation-tools)).
- **The documentation** in `docs/`.

Nothing of the original is in it: no code, binary, listing, layout, table or text of the plug-in, of its documentation or of the host's SDK. The emulator harness that runs the original, and the inputs it needs, stay outside the repository ([reproduce](docs/reproduce.md)).

You need a stable Rust toolchain and Python 3. From the repository root:

```sh
cargo test
python -I tools/gen_docs.py --check
python -I tools/check_docs.py
```

Where only `python3` is installed, run the same commands with `python3` in place of `python`.

The first command builds the six crates and runs their tests. The second regenerates every figure of the documentation in memory and exits 1 when a page is stale. The third reads every page for stale figures, broken links, the status wording and hexadecimal addresses, and every text file for characters outside ASCII, dash characters and secrets, and exits 1 on any finding. Two more of its checks, one for words about how the repository was made and one for names that belong outside it, take their terms from a list kept outside the repository; without that list each prints NOT RUN and adds no finding. `effect-params` and `shape-model` are libraries only; `shape-canon` is a library that also builds the command `shape-predict`, which prints the predicted dump of one case file ([reference: shape-predict](docs/reference.md#shape-predict)); and `mask-seq` is a library that also builds the command `mask-validate`, which checks one mask sequence ([reference: mask-validate](docs/reference.md#mask-validate)).

**Continuous integration.** `.github/workflows/ci.yml` runs on every push to `main`, on every pull request, and when started by hand. Its test job runs on Linux, macOS and Windows with the stable Rust toolchain and Python 3.12: it builds the crates, runs their tests, compiles `tools/`, runs the tests of the three tools, then `gen_docs.py --check` and `check_docs.py`, with a 30 minute limit. A second job, on Linux with a 15 minute limit, checks the formatting with `cargo fmt` and runs `cargo clippy` with every warning an error. Every cargo command except `cargo fmt` passes `--locked`. Each action is pinned to a full commit SHA, with the version it came from in a comment: `actions/checkout` v7.0.1, `actions/setup-python` v7.1.0, `Swatinem/rust-cache` v2.9.3, and `dtolnay/rust-toolchain` at its `master` branch as of 2026-11-17. The workflow may only read the repository, and it is the only workflow. The term list is not in CI, so there the two checks that need it print NOT RUN and every other check runs. A new push to a pull request cancels its run in flight; each push to `main` keeps a run of its own.

Everything else is in the [reference](docs/reference.md): the crate map, the table format, the pairing, the shape model, the case and dump forms, the mask sequence form, the frame contract, the stage program, the build, the tools and the tests. A script that needs a fact should begin at its [sources of truth](docs/reference.md#sources-of-truth). Each remaining page has one subject:

- [parity](docs/parity.md): the definition of parity, the way it is measured, the status rule and the limits of what a result says; the [parity log](docs/parity-log.md) keeps the history by date.
- [reproduce](docs/reproduce.md): which checks need nothing but this repository, and which inputs you bring yourself.
- [internals](docs/internals/README.md): how the crates are put together, the rules their code keeps, and what a change has to leave passing; the two shape crates, with their hypotheses, have [a page of their own](docs/internals/shape-crates.md).
- [handoff](docs/handoff.md): where the repository stands against the handoff convention that twixtor and sapphire share: the stage program and its self test stage, the frame tools, the file forms a peer can exchange, the exit codes, and which steps of joining are taken.

## Parity

**<!--f:status_line-->method and preparation only: 0 cases compared<!--/f-->.** No parity label is claimed anywhere in this repository until a unit's result file meets the [status rule](docs/parity.md#the-status-rule).

The check follows [twixtor](https://github.com/baairon/twixtor) and [sapphire](https://github.com/baairon/sapphire), where the judge is the original's own code run in an emulator. Here the plug-in file, left as shipped, is loaded into a CPU emulator (Unicorn, driven through Qiling), and a mock host takes the place of After Effects on Windows. The rewrite gets the very inputs the original gets, and the two outputs have to agree exactly (the planar tracker is an exception, [Limits](#limits)): bytes and integers by value, floats as bit patterns at a tolerance of 0, so that -0 and 0 differ. How a NaN is compared is fixed for each unit and written in its result file.

Each unit is checked three ways:

- **The grid:** a test set of no fewer than <!--f:case_floor-->150<!--/f--> in-contract cases, edge values included.
- **The call record:** every request either side makes of its host, compared one call at a time.
- **The mutants:** copies of the rewrite with errors put in on purpose; the grid has to expose each one.

Before any output is compared, the interface is paired. The parameters the original registers with its host are read in the emulator and matched with the rewrite's prediction of them, which is written and sealed before the original first runs. The original is also booted twice, to show that the same inputs give the same call record and the same dump. Neither step carries a parity label ([parity: the interface pairing](docs/parity.md#the-interface-pairing-and-calibration)).

Shape data is prepared the same way. For each case of the shape data check, the text dump the rewrite predicts for a shape was written and sealed before any run of the original and is kept private; the harness is built to have the original report the same shape in the same text, and the two are to be compared byte for byte. No case of it has run ([parity: the shape data check](docs/parity.md#the-shape-data-check)).

Where any result falls short:

- **A grid is a sample.** It holds the inputs someone chose, never all inputs.
- **The reference comes from the emulated original.** The emulator follows the architecture manual, so a reference says nothing about silicon or about After Effects on a real machine. Where the original leans on its host, both sides read the same labelled stand-in ([below](#assumptions-and-stand-ins)).
- **One judge.** A result rests on one emulator unless its result file says that a second engine checked it.

The method, the status rule and the definitions are in [parity](docs/parity.md).

### Status

Every figure and table in this section is generated from the result files, `parity/<unit>/results/summary.json`, by `tools/gen_docs.py`, with the rule of `tools/status.py`. A result file never states its own status.

Result files: <!--f:units_total-->0<!--/f-->. The status, one part per rung, since units are never counted across rungs: <!--f:status_line-->method and preparation only: 0 cases compared<!--/f-->.

- **Matches:** a named rung; all in-contract cases compared, no fewer than <!--f:case_floor-->150<!--/f-->, not one differing at a tolerance of 0; and at least one call record compared, with every compared record equal on the two sides. A result file without a rung, one that holds only an interface pairing and a calibration, is not a parity unit and never matches.
- **Verified** adds the survivor rule: at least ten mutants, at least nine in ten of them caught outright, and every one that survives either caught by an added case or argued equivalent with a named reviewer.

<!--gen:units-->
No result file exists yet, so no unit has a row. A unit is Not started until its `summary.json` is committed.
<!--/gen:units-->

How to read the table, once it has rows:

- **Cases compared** gives two numbers, "X of Y": the cases compared, then the unit's in-contract cases.
- **Rung label** is computed from the same file by the [status rule](docs/parity.md#the-status-rule), and reads none in every case that rule gives none, for example while no case is compared or the tolerance is not 0.
- **Re-run** is the state of an independent re-run on fresh copies: yes, no, stale or none. Only yes counts as re-run verified.

The label block heads every unit's record. Each line is computed from the result files, or says that nothing was measured:

<!--gen:label-->
```text
Status: none (method and preparation only: 0 cases compared)   [computed from 0 result files; re-run: none]
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
   - Adobe does not allow its After Effects SDK to be passed on, so get your own copy from Adobe. The harness compiles the host's structure layouts from its headers, and those layout files never enter the repository.
   - The harness starts from the public harness of [twixtor](https://github.com/baairon/twixtor) (`parity/harness/` at commit 8087d06), on Qiling 1.4.6 and Unicorn 2.1.4, the versions [its README](https://github.com/baairon/twixtor/blob/8087d06ac43023f954de2cc58583b132a3243ff4/parity/harness/README.md#running-it) names, with the changes this plug-in needs. Those changes are not published, because they carry layouts compiled from the SDK and tables made from the original's files.
   - The predicted parameter tables are written from the product's documentation and stay private, like every name and value taken from it. To repeat the pairing, write your own in the [table format](docs/reference.md#the-table-format) and seal it before the first run.
3. Pair the interface: boot the original under the harness, read the parameters it registers, and pair them with the predicted table; nothing may stay unmatched on either side. Boot it a second time in a fresh process and compare the two call records and dumps byte for byte.
4. For each unit, run the original over the unit's test set, which gives the reference outputs, then run the rewrite over those cases, recording the host calls of both sides.
5. Compare with a tolerance of 0 and write the unit's result file in the fields of `docs/data/result_schema.json`.
6. Regenerate the tables above with `python -I tools/gen_docs.py`; with `--check` it exits 1 when a page no longer matches the result files.

### Assumptions and stand-ins

Where After Effects or Windows would provide something the harness cannot, the harness hands one labelled value to both sides: a stand-in. A result that depends on one is logic parity, not parity. Every answer the mock host gives is a row of the harness's host-answer register, with the public documentation it follows and a status: a stand-in until a run measures its effect, or open, in which case the call faults. The harness is not in this repository, so the harness-wide choices below describe its design and cannot be checked from it:

- **Unknown host calls fault.** A call the harness has no answer for stops the run and names itself; none is answered with success or with 0.
- **The emulated CPU** presents a fixed x86-64 feature set without the AVX tiers. A vector instruction outside that set, or a feature query the table does not list, stops the run. The CPU starts in the Windows floating point state, and the main thread runs on 1 MiB of stack, the stand-in value of [twixtor's harness](https://github.com/baairon/twixtor/blob/8087d06ac43023f954de2cc58583b132a3243ff4/parity/harness/README.md#stand-ins) for a host value that is not known.
- **Threads** switch only when one waits, in one reproducible order.
- **Time and randomness** never come from the machine that runs the emulator: a time read stops the run until a fixed clock is registered, and random bytes come from a seeded stream.
- **Host services.** A service on the declared list of absent ones is answered "not found"; a request with no declared answer stops the run.

The stand-ins a result rests on are named in its result file and listed in [parity: stand-ins and assumptions](docs/parity.md#stand-ins-and-assumptions).

## Limits

What the method cannot show, whatever the results say:

| Limit | What it means |
|---|---|
| Nothing compared yet | The status reads <!--f:status_line-->method and preparation only: 0 cases compared<!--/f-->. There is no reference output and no label, and the crates render nothing. |
| One judge | Every reference comes from one emulator. An emulator bug is common to both sides of a comparison: it can make a wrong rewrite look right, or a right one look wrong. Until a routine that the original implements twice has run on a second engine, every result reads "single judge, not cross-checked". |
| Emulator, not silicon | The emulator follows the architecture manual. Where real processors differ from it (estimate instructions, NaN payloads, which feature tier a dispatcher picks), nothing is claimed. |
| Instruction-set route | The emulated CPU has no AVX tier. Where the original picks its code by CPU features, only the path for that set runs, and every result carries the qualifier that CPUs with other tiers run a different body, which is not reproduced. |
| The stand-in register | Where the host would answer, the harness answers from its register, each row following public documentation. Agreement on a stand-in says the two programs treat that answer alike; it says nothing of what a real host would return. |
| The licence gate | The harness answers the original's licence module only as [Left out](#left-out) lists, and none of those answers tells the original that it is licensed. Which modules the original then offers is read from its own run, never assumed. A unit that the original gates behind its licence state stops or runs as unlicensed: a stop is recorded as Blocked, an output that looks unlicensed is never used as a reference, and every result from a run with these answers names them as a limit of the judge. An output that looks unlicensed is never used as a whole frame reference. Where the original marks each frame its engine delivers with an overlay, only the samples outside the overlay are to be compared: its extent is to be measured from the original's own frames before any comparison, and the excluded samples are to be counted in every such result, which is to have its own status below Matches. This is a rule of method that no tool computes yet ([parity: frames that carry an overlay](docs/parity.md#frames-that-carry-an-overlay)). |
| The licence dependent modules | The modules that the original gates behind its licence state are to be rebuilt from public methods and from a written specification of each module's arithmetic. Whether the emulated original renders each of these modules without any licence or feature question being answered is not known: it is to be measured module by module, and no render command of the original has yet run in the emulator. Where a module renders there, its frames are to be compared outside the overlay, under the status "matches the emulated original outside the overlay footprint", which is below Matches ([parity: frames that carry an overlay](docs/parity.md#frames-that-carry-an-overlay)). Where a feature question stands in front of a module's output, no answer is given and the module is compared only with frames rendered on a licensed installation and supplied by the owner, under the status Owner supplied frames, which is not a parity result ([parity: frames supplied by the owner](docs/parity.md#frames-supplied-by-the-owner)). No route adds any answer to those that [Left out](#left-out) lists. |
| The planar tracker | The original's planar tracker is an iterative solver with two paths, a graphics processor path and a processor path, and its code holds switches whose names show that they exist to manage a difference between the two; the size of that difference has not been measured. A whole track is therefore expected to be comparable only within a stated tolerance on the positions of the tracked surface's corners, not exactly. Single steps on fixed inputs, such as one level of the image pyramid, one evaluation of the error, one damped solver step from a given state and the chaining of per frame motions to a reference frame, are expected to be comparable exactly once their number types are matched. The result schema and `tools/status.py` cannot yet state a tolerance on corner positions (the schema's `tolerance_ulp` counts code steps or ulps of a sample), so this tolerance is a rule of method only ([parity: what the results do not cover](docs/parity.md#what-the-results-do-not-cover)), and a result at a tolerance other than 0 has no rung label under the status rule. No track has been compared, and how a reference track of the original is to be obtained is not settled. |
| Shape hypotheses | The rules of `shape-model` (how points are kept at keys, blended between them and held outside them, and how their rectangle is spanned) are hypotheses that no compared case has tested. A predicted dump built on them says nothing about the original until a case is compared. |
| A sample, not a proof | A grid covers chosen, synthetic and small inputs; real footage and real projects are not exercised. Mutants are chosen by the authors, so a kill count depends on which were written. |
| One self test stage | `mocha-stage` follows the handoff convention that twixtor and sapphire share, and its one stage, `selftest-copy`, copies frames unchanged. No stage reimplements any output of the plug-in, and none is to be offered before its unit has been compared. The [handoff](docs/handoff.md) page lists what exists and what is absent. |

### Left out

Never part of this project: the licence, activation and machine identity code of the original. It is never read or analysed: where a read of the original's other code reaches a call into it, only that call and what the caller does with the answer are recorded ([Legal](#legal)). Nothing in it is changed on disk; it runs as the original's own code, and only inside the emulator. So that the original's engine can load at all, the mock host answers calls that the licence module makes, at the boundary between the emulated program and the host, in these ways only: a request to load or find a library the emulated machine holds gets that library, and a later release of it is accepted; a plain question about the emulated machine's Windows folders or environment gets the declared answer that every program in the emulator gets; a request for anything the emulated machine does not hold gets the documented Windows not found or not available failure; and a question from the module that identifies the machine gets its documented failure, never a value, real or invented. The module's own start-up routine is not run when the engine loads. No answer tells the original that it is licensed, and no licence server or network is involved. Any other call that the licence module makes to the host stops the run at that call, and so does a licence or activation call that any other part of the original makes to the host. No machine identity question, from any part of the original, gets a value from a real machine. A unit that cannot go on without more stops there; its result file says so. The emulated process has no network: every network call faults.

The modules that the original gates behind its licence state are not left out: they are part of the scope, and what their comparison rests on is stated under [Limits](#limits). The interactive interface of the product's standalone editor is left out: it is not rebuilt, and the capabilities in scope are to be offered as stage programs for command line use.

## Legal

- **Independent work.** This project is written independently and contains none of Mocha Pro's code, binaries, assets or documentation.
- **How the match is made.** The outputs are to be matched by running the original plug-in, unmodified, in an emulator on test inputs, for interoperability. So far it has been run there only for the first step of the method, the interface pairing and the calibration boots. Its files were first examined only statically (headers, import and export tables, resources and strings), and the product's documentation was read to predict its interface and to plan the shape data check. Since then parts of its code have also been read statically, without running it: the plug-in's handling of its parameter setup, its render commands and its arbitrary data, and, in its engine module, the entry path, the reader of the project data and stages of the render path with their arithmetic; and its planar tracker, read mostly in the product's separate tracking and numerics files. Each read is written up as a specification of method, interfaces and formats, with no code excerpt and no instruction listing, and kept outside this repository; code here that rests on one is written from it and from what the emulator shows, not from the original's code. Where a read path reaches a licence, activation or machine identity check, only the call and what its caller does with the answer were recorded, and the check itself was not read ([Left out](#left-out)).
- **No affiliation.** It is not affiliated with, endorsed by or sponsored by any other company, product or library.
- **Trademarks.** Product and library names belong to their owners and appear here only to identify them. Mocha Pro is named only to say what this project reimplements.
- **Interoperability.** Parameter names and values of the plug-in are to be used only where settings must carry over between the two and outputs must be matched; this version holds none of them.
- **No warranty.** Everything here is provided as is under the MIT License, without warranty of any kind and without liability.
- **Your responsibility.** You need the rights to the media and the software you process, and you stay bound by the licences of any software you use alongside this project, the original plug-in and the host's SDK included.
- **Rights holders.** If you believe something here infringes your rights, email [hi@bairon.dev](mailto:hi@bairon.dev) or [open an issue](https://github.com/baairon/mocha-pro/issues). Every concern gets a prompt answer in good faith.

## License

MIT, see [LICENSE](LICENSE).
