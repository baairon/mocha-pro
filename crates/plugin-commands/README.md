# plugin-commands

The decision rules of the plug-in side command handlers of the effect rewrite, as pure functions. The crate is a std only library with no dependency and no unsafe code, it reads no file and no environment variable, and it makes no host call: the caller performs the host calls (suites, engine runs, dialogs, drawing) and passes their results in.

## What it does

- **Dispatch** (`command_from_code`, `never_sent`, `command_refusal`, `thrown_return`, `engine_wrapper_return`): the command codes that have a handler of their own, the default return for every other code and for codes above the table, the codes that a conforming host never sends given the declared flags, and the return of a command after a thrown error or an engine result.
- **Parameter changes** (`param_action`, `sentinel_step`, `sentinel_recorded`, `preset_load`, `preset_load_outcome`, `target_moved_sets_free`): the action of a changed host index by its role, the state that two rows feed, the writes of a preset to its target rows in their order, and the outcome of those writes.
- **Interface state** (`update_ui`, `selector_c_mode`, `enable_rule_flags`, `shown_rule_flags`, `update_needs_host_call`): which dependent rows are enabled and shown, and whether the host has to be called at all.
- **Events** (`event_kind`, `hover_hit`, `click_hit`, `adjust_cursor_hover`, `do_click_effect`, `drag_effect`, `draw_plan`): the event types acted on, the hit boxes of the control row, and the effects of a click, a drag release and a draw.
- **Launch and about command** (`guard_state`, `guard_shows_dialog`, `downsample_ratio`, `launch_return`, `launch_writes_message`, `error_stream_text`, `about_outcome`, `about_text`, `about_refused_return`): the guard dialog, the return of the launch, and the outcome, fallback text and return of the about command.
- **Sequences** (`setdown_skips_registry` and a second helper): whether a sequence id skips the registry at set down, and the id that the second helper makes from an id text.

Every point the rules leave open is a field of `Readings` with a stated default, so that a judged frame settles it by changing one field, apart from the form of the id that the second sequence helper makes, which is the one such rule kept in code. `dialog_answer`, and `param_action` for a host index without a role, answer `Error::NotRebuilt`, which claims no host return code and names no role; an outcome the rules do not state is `Error::Unspecified`.

## Supplying the table

The crate holds no value that belongs to the effect's parameter rows, interface rows or resources. The caller passes a `RoleTable`: host index to role, selector position to role, bitmap and dialog ids, the preset list and the texts, all keyed by role. `RoleTable::check` refuses a table that lists one host index twice or one role other than `ParamRole::NoAction` on two indices. `RoleTable::sample()` is a made up table for tests, examples and documentation; a caller with its own values builds its own table and passes it to the same functions.

```rust
use plugin_commands::{param_action, ParamAction, RoleTable};

let table = RoleTable::sample();
let index = table.params[0].0;
assert_eq!(param_action(&table, index), Ok(ParamAction::SentinelTracker));
```

## Tests

`cargo test -p plugin-commands` runs `tests/spec.rs`: the dispatch of the codes with a handler, of the codes without one and of the codes above the table; the parameter actions, the state that two rows feed and every preset, on the made up sample; the interface state against an independent reference over a grid of inputs; the edges of the hit boxes; the effects of a click, a drag and a draw; the guard, launch and about command cases; the sequence helpers; and the refusals of `RoleTable::check`. The example above runs as a doc test. Apart from the constants of the specification that they pin, their values are invented.
