# eye-layout

Eye layouts as pure functions. The crate has no dependency, no unsafe code and no platform specific code.

## What it does

- `RoleTable`: the table that maps a layout selector position to a layout kind (a whole frame, two layers, two eyes stacked, two eyes side by side) and a spherical flag. The caller supplies every row and every separator position; the crate holds no real table. `RoleTable::sample()` is a made up table for tests, examples and documentation.
- `narrow`: the rectangle of one eye inside a packed frame for the two packed layouts, with the odd size extra pixel and the exchange and bottom-up inversions.
- Layer and eye resolution, the clip scale decision, the arguments of the first and second engine call, the preparation error code, and `place_eye`, which copies one eye into its rectangle of the output frame.
- `Readings`: every point the description leaves open is a field with a stated default, so a judged frame settles it by changing one field.

Arithmetic is IEEE double with truncation toward zero for the narrowed sizes; no library call is on that path.

## Supplying the table

```rust
use eye_layout::{Layout, LayoutRow, RoleTable};

let table = RoleTable::new(
    vec![LayoutRow { position: 0, layout: Layout::SideBySide, spherical: false }],
    vec![1],
)
.unwrap();
assert!(table.parse_selector(1).is_ok());
```

The selector value is one based: the position looked up is the value minus one.

## Tests

`cargo test -p eye-layout` runs the vectors of every layout, exchange and size case, the table behaviour on the made up sample, the second call arguments per layout and an integer reference over a range of sizes.
