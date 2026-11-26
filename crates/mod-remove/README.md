# mod-remove

The frames that replace a layer, produced from other frames of a clip. The crate is a std only library with no dependency and no unsafe code, and it reads no file and no environment variable.

## What it does

- **Source index** (`source_index`): for each pixel to fill, the source frames that can supply it, recorded batch by batch, then counted per source and compacted, with the windows that a source covers in the target frame.
- **Weight maps** (`mix`): one soft map per source, grown in steps from the source index and masked to the window and the mask of its source, and a scatter across the maps of a batch when `FillConfig` asks for it.
- **Brightness fit** (`illum`): a correction per source, either interpolated from gains at sample points or a linear fit that is solved, filtered by a trim limit and solved again.
- **Synthesis** (`geom`, `fill`, `spread`): the transfer of a target pixel into each source through its projective map, bilinear samples accumulated under the weight maps and normalised, and a spread pass that fills, from the boundary, the pixels that no source reaches.
- **Frame driver** (`host`): the preconditions of a render, checked in a fixed order, the one pixel expansion of the layer matte, the order of the candidate source frames, which the caller computes with `candidate_times`, the composite of the fill over the frame, and `render_frame`, which runs the preconditions, refuses the carry-over rendering mode as a stage not given, then runs the matte expansion, the synthesis and the composite in that order.

Every point the specification leaves open is a field of `FillConfig` with a stated default reading. Every stage it does not give returns `RemoveError::Unspecified` with the point named.

## What the caller supplies

- The frames, mattes, cameras, projective maps and corrections of a render, through `FrameInputs` and `Source`.
- The rendering mode flag of `FrameInputs`. No `Default` holds it; `FrameInputs::sample()` is a made up set of inputs for tests, examples and documentation, and its flag is the made up `SAMPLE_CARRY_OVER`.
- Any text that a precondition failure must carry, through `PreconditionTexts`. Each `Precondition` has a `Display` text of the crate's own, used when the caller gives none; `PreconditionTexts::sample()` is a made up table.

```rust
use mod_remove::{Precondition, PreconditionTexts, RemoveError};

let mut texts = PreconditionTexts::new();
texts.set(Precondition::EmptyRegion, "made up text");
let e = RemoveError::Precondition(Precondition::EmptyRegion);
assert_eq!(e.precondition_text(&texts).as_deref(), Some("made up text"));
```

## Tests

`cargo test -p mod-remove` runs seven test files, with helpers in `tests/support/mod.rs`: the source index, its counts, compaction and windows (`t_source_index.rs`), the weight maps (`t_mix.rs`), the brightness models (`t_illum.rs`), the transfer and the sampler (`t_geom.rs`), fills of made up frames against values worked by hand and a reference fill (`t_fill.rs`), the spread pass (`t_spread.rs`), and the preconditions, the composite, the caller supplied texts and flag and the stages of the frame driver that return `RemoveError::Unspecified` (`t_host.rs`). Each file also checks the stages of its own part that return `RemoveError::Unspecified`. Apart from the constants of the specification that they pin, their values are invented.
