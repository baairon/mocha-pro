# effect-shell

A generic After Effects effect plug-in shell. It exports the host entry point `EffectMain`, answers
ABOUT, GLOBAL_SETUP and PARAMS_SETUP from a parameter table that is given at build time, and holds
no engine and no plug-in data. The crate ships only a made up sample table.

## The table

The table is an effect-params text file. Version 2 rows (ten cells, see the effect-params format
notes) state the id, the name, the kind, the raw host flags word, the raw host ui flags word and
the extra cell (popup choices, slider precision and display bits, point, button label, arbitrary
data bytes). The shell writes exactly the stated fields into the definition it hands to the host.
Version 1 tables still load (the older conventions in `src/plan.rs` apply to them).

Lines that start with `#@` are comments for effect-params and directives for the shell:

* `#@global` with tab separated `key=value`: `my_version`, `out_flags`, `out_flags2` (the words
  GLOBAL_SETUP writes to out_data) and `about`.
* `#@shell`: `reset`, `fill`, `refcon`, `register_ui` (see below).
* `#@` before a row: per row directives (`ui_w`, `ui_h`, `ui_suite`, `defer_err`, and the older
  ones listed in `src/plan.rs`).

## What PARAMS_SETUP does

For every row, in table order:

1. When the row has `ui_suite=<text>`: acquire the effect ui suite (version 1) through the basic
   suite of in_data, and when a table comes back call the options button name function with the
   text, then release the suite. A refusal is tolerated, a return code is ignored, and the step is
   skipped in Premiere and when in_data has no basic suite.
2. For an arbitrary data row: acquire the handle suite (version 2), call new_handle with the
   number of stated data bytes (when there are bytes: lock, copy, unlock), release the suite. The
   handle becomes the default of the definition. A refused suite, a null table or a missing member
   gives `ERR_BAD_CALLBACK_PARAM` (516) and nothing more is added.
3. Write the stated fields into the definition structure and call add_param with index -1. A
   nonzero code stops the setup and is returned unchanged. A row marked `defer_err=1` instead
   keeps the first code, the setup goes on, and the kept code is returned after the last row
   without the final steps.

After the last row, when `register_ui=<events>` is set: one register_ui call with a record whose
events word is the value and whose other words are zero; its code becomes the command's return
code. num_params is written (rows plus one for the input layer) in every case that gets here.

## The bytes of the definition

The structure the host sees is reproducible to the byte.

* `reset=each` (default): the 176 byte structure is filled with the `fill` byte (default 0) before
  every row. A field the table does not state keeps the fill byte.
* `reset=never`: the structure is filled once and reused; a field the row does not state keeps
  what the previous row left (a name is written like a string copy, bytes and one NUL, so the tail
  of a longer earlier name stays; strings that earlier rows pointed at stay alive, so a carried
  pointer is still valid). The choice is a table option because a host may hand over a zeroed
  structure or one that keeps earlier values; the table can also state such a word per row.
* Version 2 rows: id, host flags, host ui flags, ui width and height are written only when
  stated. Union scalars with an unknown cell use the conventions of `src/setup.rs` (widest range,
  zero default). The arbitrary data value is NULL and its refcon is the `refcon` option.
* A name of 32 bytes or more is refused at plan parse time (version 1 cuts at 31 bytes).
* A popup default of a version 2 row is written as stated (zero stays zero); version 1 clamps it
  to a 1 based index.

## Build

```
cargo build --release -p effect-shell
```

with the sample table, or with a private table kept outside the repository:

```
EFFECT_SHELL_TABLE=/path/to/private_table.txt cargo build --release -p effect-shell
```

`build.rs` reads the variable and compiles the text in; the variable is tracked, so a change
rebuilds the crate.

For a Windows library (a cdylib that exports `EffectMain`):

```
rustup target add x86_64-pc-windows-gnu
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
EFFECT_SHELL_TABLE=/path/to/private_table.txt \
cargo build --release -p effect-shell --target x86_64-pc-windows-gnu
```

The result is `target/x86_64-pc-windows-gnu/release/effect_shell.dll`; its only export is
`EffectMain`. Do not commit the private table or the built library.

## Freestanding build

The ordinary Windows dll above is a Rust std library: its import table holds the standard library
runtime (about 170 imports), which a small emulator host cannot serve. The cargo feature
`freestanding` builds the same code as `no_std` plus `alloc`: a static 8 MiB arena as the global
allocator, its own panic handler (an undefined instruction), the memory functions (`memcpy`,
`memmove`, `memset`, `memcmp`, `bcmp`, `strlen`) and a DLL entry that returns 1. The link uses
`-nostartfiles -nodefaultlibs` (set by `build.rs` for this feature on Windows only) and panics must
abort:

```
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
RUSTFLAGS="-C panic=abort" \
EFFECT_SHELL_TABLE=/path/to/private_table.txt \
cargo build --release -p effect-shell --no-default-features --features freestanding \
  --target x86_64-pc-windows-gnu
```

The image imports nothing. The feature is off by default; the std build and its tests are
unchanged. The freestanding build parses the plan on every call (there is no `OnceLock`), has no
`catch_unwind` (a panic aborts) and is meant for emulator images only. `effect-params` is `no_std`
plus `alloc` when its default feature `std` is off.

The arena is emptied after every call of the entry point that parses the plan (about, global setup,
params setup), so a long session cannot exhaust it; any other command returns success without
parsing. Allocation failure (an arena of 8 MiB used up by one call) is an abort, not an error code.
`freestanding` needs `--no-default-features` (with the default `std` feature the crate stops with a
`compile_error!`), and `cargo check` or `clippy` of the feature also needs `RUSTFLAGS="-C panic=abort"`.
The allocator and the memory functions are tested on the host by `tests/arena.rs`, which includes
`src/freestanding.rs` by path.

## Property list (PiPL) resource

A real host reads a plug-in property list from the library's resources before it calls the entry
point. `src/pipl.rs` serialises such a list from a neutral description with no external tool and no
dependency, and `build.rs` embeds it in the Windows library as a resource of type `PiPL`, id 16000,
language 1033 (US English).

The description is a text file named by the environment variable `EFFECT_SHELL_PIPL` (default:
`sample/sample_pipl.txt`, made up). One property per line: typed lines such as `kind`, `name`,
`category`, `entry_win64`, `global_flags`, `global_flags2`, `match_name`, and generic lines
`u32`, `pair`, `code`, `pstring`, `cstring`, `hex` with a four character key; an optional trailing
`id=N` sets the property id. The format is described in the module documentation.

```
EFFECT_SHELL_PIPL=/path/to/private_pipl.txt \
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
cargo build --release -p effect-shell --target x86_64-pc-windows-gnu
```

How it is embedded: build.rs writes a COFF object with the sections `.rsrc$01` (directory tree)
and `.rsrc$02` (data) into `OUT_DIR` and passes it with `cargo:rustc-link-arg-cdylib`; the linker
merges them into the image's `.rsrc`. No resource compiler is needed (`windres` would also work,
but is not used). The object is written for x86_64, x86 and aarch64 Windows targets; only
x86_64-pc-windows-gnu was built and read back. On other targets the description is still parsed
(a broken file fails the build everywhere) and nothing is embedded.

The global flags of the list must equal the flags the plug-in reports in its global setup: a host
refuses a mismatch. The shell does not check this for you; keep the description and the table's
global directive in step.

## What it does not do

* No engine, no rendering and no output of any plug-in. The sequence, render and setdown commands
  go to the unit table (below), which is empty in this crate, so they return 0 and do nothing.
* No result file covers the crate, so it has no part in the status of `docs/parity.md`, and it has
  not been loaded by a real host.
* No engine call; the engine flags word is an input (`set_engine_flags`).
* No `ansi.strcpy` call through the host utilities: names are written directly, so the host sees
  no string copy calls.
* The out_data name field and the options button name are not set by GLOBAL_SETUP.

## Host interface

`src/layout.rs` and `src/render_layout.rs` hold this project's own repr(C) definitions of the
host's structures under the crate's own Rust names. Their sizes, member offsets and constants are
public interface facts of the host's 64 bit plug-in interface, written for interoperability, and
`tests/interface_layout.rs` with the unit tests of `src/render_layout.rs` pins every size and
offset. No text or file of the host's SDK is included.

## Unit table and render layouts

`src/units.rs` lets another crate supply handlers for the commands SEQUENCE_SETUP, SEQUENCE_SETDOWN,
GLOBAL_SETDOWN, RENDER, SMART_PRE_RENDER and SMART_RENDER. A handler receives the raw pointers of
the command (they may be null; the handler owns every check). A command without a handler falls to
the default case and returns 0. `entry_with_units` is the entry logic over a table; a panic in a
handler gives `ERR_INTERNAL_STRUCT_DAMAGED`. The shell's own `EffectMain` uses the table given to
`units::register_units` (the empty table by default). With the cargo feature `external-entry` the
shell does not export `EffectMain`: the crate that owns the table exports its own and calls
`entry_built_in` (the built in plan, in the std and the freestanding profile) or `entry_with_units`.

`src/render_layout.rs` holds the repr(C) definitions the render commands need (rectangles, the
render request, the pre render and smart render extra records and callbacks, the world record, the
utility callback table by slot, the typed handle suite, the checkout and checkin callbacks of the
interact table), with size and offset tests. The crate holds no plug-in data and no engine.
