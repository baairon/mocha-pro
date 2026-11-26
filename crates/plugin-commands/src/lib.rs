#![forbid(unsafe_code)]

//! Host command dispatch of the plug-in side as pure data and pure functions.
//!
//! Values that belong to the original's parameter rows, interface rows and
//! texts are not stored here. The caller supplies them as a [`RoleTable`], a
//! typed value keyed by role words: host index to role, position to role, text
//! by role. [`RoleTable::sample`] is a made up table for tests, examples and
//! documentation.
//!
//! No host structures and no host calls.

use std::fmt;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExamples;

/// Returned where the rules are silent or where a row is not rebuilt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The rules do not state the outcome.
    Unspecified(&'static str),
    /// The row, host index or command is not rebuilt: no host return code is
    /// claimed and no role is named.
    NotRebuilt,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unspecified(what) => write!(f, "unspecified: {what}"),
            Error::NotRebuilt => write!(f, "not rebuilt"),
        }
    }
}

impl std::error::Error for Error {}

/// A value that may be absent. The crate's own role word type for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot<T> {
    /// No value.
    Empty,
    /// A value.
    Held(T),
}

impl<T> Slot<T> {
    /// True when there is no value.
    pub fn is_empty(&self) -> bool {
        matches!(self, Slot::Empty)
    }

    /// True when there is a value.
    pub fn is_held(&self) -> bool {
        matches!(self, Slot::Held(_))
    }

    /// The value; panics when empty.
    pub fn unwrap(self) -> T {
        match self {
            Slot::Held(v) => v,
            Slot::Empty => panic!("empty slot"),
        }
    }

    /// The value, or the given error when empty.
    pub fn ok_or<E>(self, e: E) -> Result<T, E> {
        match self {
            Slot::Held(v) => Ok(v),
            Slot::Empty => Err(e),
        }
    }
}

// ------------------------------------------------------------ host constants

/// Host interface return code: success.
pub const OK: i32 = 0;
/// Host interface return code: a required argument was NULL.
pub const ERR_NULL_ARGS: i32 = 512;
/// Host interface return code: the launch function failed.
pub const ERR_LAUNCH: i32 = 515;
/// Host interface return code: a mandatory suite was refused.
pub const ERR_SUITE: i32 = 516;
/// Host interface return code: an engine failure mapped by the wrapper.
pub const ERR_ENGINE: i32 = 517;
/// Host interface return code: a NULL drawing suite set.
pub const ERR_DRAWBOT_NULL: i32 = 514;

/// Host command codes of the commands that have a handler.
pub const CMD_ABOUT: u32 = 0;
pub const CMD_GLOBAL_SETUP: u32 = 1;
pub const CMD_GLOBAL_SETDOWN: u32 = 3;
pub const CMD_PARAMS_SETUP: u32 = 4;
pub const CMD_SEQUENCE_SETUP: u32 = 5;
pub const CMD_SEQUENCE_RESETUP: u32 = 6;
pub const CMD_SEQUENCE_SETDOWN: u32 = 8;
pub const CMD_DO_DIALOG: u32 = 9;
pub const CMD_RENDER: u32 = 11;
pub const CMD_USER_CHANGED_PARAM: u32 = 13;
pub const CMD_UPDATE_PARAMS_UI: u32 = 14;
pub const CMD_EVENT: u32 = 15;
pub const CMD_ARBITRARY_CALLBACK: u32 = 22;
pub const CMD_SMART_PRE_RENDER: u32 = 23;
pub const CMD_SMART_RENDER: u32 = 24;
/// Codes a conforming host never sends, given the declared flags.
pub const CMD_NEVER_SENT: [u32; 3] = [7, 16, 18];

/// Interface flag bits of a parameter's ui flags.
pub const PUI_DISABLED: u32 = 0x20;
pub const PUI_INVISIBLE: u32 = 0x200;

/// Event types the event handler acts on, and the control area code.
pub const EVENT_DO_CLICK: i32 = 2;
pub const EVENT_DRAG: i32 = 3;
pub const EVENT_DRAW: i32 = 4;
pub const EVENT_ADJUST_CURSOR: i32 = 9;
pub const AREA_CONTROL: i32 = 2;
/// Event out flags: handled, and handled with an immediate update.
pub const EVT_HANDLED: u32 = 1;
pub const EVT_HANDLED_UPDATE: u32 = 9;

/// The value of the process wide integer when no index is recorded.
pub const SENTINEL_VACANT: u32 = 1 << 31;

/// The answer for every code that has no handler of its own.
pub const DEFAULT_RETURN: i32 = 0;

// ------------------------------------------------------------------ the table

/// Roles a host parameter index can play in the change rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamRole {
    /// A row the caller lists as acting on nothing.
    NoAction,
    /// One of the two rows that feed the sentinel tracker.
    SentinelA,
    SentinelB,
    /// Rows that start an engine action of their own.
    ActionA,
    ActionB,
    ActionC,
    ActionD,
    ActionE,
    ActionF,
    /// The selector that loads a preset into the three target rows.
    PresetSelector,
    /// The three target rows a preset writes, in write order.
    PresetTargetA,
    PresetTargetB,
    PresetTargetC,
}

/// A preset of the three target rows, single precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

/// Resource ids of the two bitmaps of the control row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bitmaps {
    pub normal: u32,
    pub hover: u32,
}

/// Results of the guard dialog, by meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogResults {
    /// Go on and suppress the dialog from now on.
    pub go_quiet: i32,
    /// Go on.
    pub go: i32,
    /// Go on at full resolution.
    pub go_full: i32,
}

/// Positions of selector b that matter to the interface rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectorBPositions {
    /// The position for which the two dependent rows a and b stay enabled.
    pub single: i32,
    /// The two positions for which dependent row c stays enabled.
    pub pair: [i32; 2],
}

/// Mode codes of [`RoleTable::selector_c_modes`]. Any other nonzero code is an
/// "other" mode.
pub const MODE_PLAIN: i32 = 0;
pub const MODE_PAIRED: i32 = 1;

/// Texts by role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texts {
    /// Written to the error stream when the launch fails.
    pub error_stream: String,
    /// Template of the fallback text of the first command. The placeholders
    /// `{name}`, `{num_a}` and `{num_b}` are replaced by [`about_text`].
    pub about_template: String,
}

/// Everything the original's rows and resources contribute, supplied by the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct RoleTable {
    /// Host index to role. An index that is not listed has no role.
    pub params: Vec<(i32, ParamRole)>,
    /// Presets, selector position 1 first.
    pub presets: Vec<Preset>,
    /// The selector position that means "free" (loads nothing).
    pub free_position: i32,
    /// Width and height of the control row hit box.
    pub control_size: (i32, i32),
    pub bitmaps: Bitmaps,
    pub dialog_results: DialogResults,
    pub selector_b_positions: SelectorBPositions,
    /// Position of selector c to mode code. A position that is not
    /// listed gives [`MODE_PLAIN`].
    pub selector_c_modes: Vec<(i32, i32)>,
    /// The engine capability bit mask that turns the paired rows on.
    pub paired_capability_mask: u64,
    pub texts: Texts,
}

impl RoleTable {
    /// A made up table for tests, examples and documentation. Its values are
    /// invented and equal nothing of the original.
    pub fn sample() -> RoleTable {
        RoleTable {
            params: vec![
                (2701, ParamRole::SentinelA),
                (2702, ParamRole::SentinelB),
                (2703, ParamRole::ActionA),
                (2704, ParamRole::ActionB),
                (2705, ParamRole::PresetSelector),
                (2706, ParamRole::PresetTargetA),
                (2707, ParamRole::PresetTargetB),
                (2708, ParamRole::PresetTargetC),
                (2709, ParamRole::ActionC),
                (2710, ParamRole::ActionD),
                (2711, ParamRole::ActionE),
                (2712, ParamRole::ActionF),
                (2713, ParamRole::NoAction),
                (2714, ParamRole::NoAction),
            ],
            presets: vec![
                Preset {
                    a: 1.5,
                    b: 2.25,
                    c: 71.0,
                },
                Preset {
                    a: -3.5,
                    b: 4.75,
                    c: 72.0,
                },
                Preset {
                    a: 5.5,
                    b: -6.25,
                    c: 73.0,
                },
                Preset {
                    a: 7.5,
                    b: 8.75,
                    c: -74.0,
                },
                Preset {
                    a: 0.0,
                    b: 9.25,
                    c: 75.0,
                },
            ],
            free_position: 3011,
            control_size: (151, 61),
            bitmaps: Bitmaps {
                normal: 4101,
                hover: 4102,
            },
            dialog_results: DialogResults {
                go_quiet: 5101,
                go: 5102,
                go_full: 5103,
            },
            selector_b_positions: SelectorBPositions {
                single: 6101,
                pair: [6102, 6103],
            },
            selector_c_modes: vec![
                (7101, MODE_PLAIN),
                (7102, MODE_PAIRED),
                (7103, 2),
                (7104, 3),
                (7105, MODE_PLAIN),
                (7106, MODE_PAIRED),
            ],
            paired_capability_mask: 1 << 40,
            texts: Texts {
                error_stream: "sample: the interface did not start.".to_string(),
                about_template: "sample {name} / {num_a} / {num_b}".to_string(),
            },
        }
    }

    /// The role of a host index, if it has one.
    pub fn role_of(&self, index: i32) -> Slot<ParamRole> {
        match self.params.iter().find(|(i, _)| *i == index) {
            Some((_, r)) => Slot::Held(*r),
            None => Slot::Empty,
        }
    }

    /// Refuses a table that lists one host index twice, or one role other than
    /// `ParamRole::NoAction` on two indices.
    /// A caller that loads a table from data calls this before using it.
    pub fn check(&self) -> Result<(), Error> {
        for (n, (i, r)) in self.params.iter().enumerate() {
            let rest = &self.params[n + 1..];
            if rest.iter().any(|(j, _)| j == i) {
                return Err(Error::Unspecified("a host index is listed twice"));
            }
            if *r != ParamRole::NoAction && rest.iter().any(|(_, q)| q == r) {
                return Err(Error::Unspecified("a role is listed on two indices"));
            }
        }
        Ok(())
    }

    /// The host index of a role, if the table lists it.
    pub fn index_of(&self, role: ParamRole) -> Slot<i32> {
        match self.params.iter().find(|(_, r)| *r == role) {
            Some((i, _)) => Slot::Held(*i),
            None => Slot::Empty,
        }
    }
}

// ---------------------------------------------------------------- dispatch

/// Toggles for every open point; a judged frame can settle one without a code change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Readings {
    /// A thrown host error becomes the command return (inferred, medium).
    pub thrown_code_is_returned: bool,
    /// A refused ANSI suite in ABOUT returns the suite error (inferred, medium).
    pub about_throw_returns_code: bool,
    /// An out of range preset selector position is a no-op (true) or unspecified (false).
    pub preset_out_of_range_noop: bool,
    /// Scale of the first drawing transform (inferred, medium).
    pub draw_scale: f32,
    /// The sentinel tracker is active at all (inferred, low).
    pub sentinel_tracker_enabled: bool,
}

impl Default for Readings {
    fn default() -> Self {
        Readings {
            thrown_code_is_returned: true,
            about_throw_returns_code: true,
            preset_out_of_range_noop: false,
            draw_scale: 0.5,
            sentinel_tracker_enabled: true,
        }
    }
}

/// Commands with a handler of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    About,
    GlobalSetup,
    GlobalSetdown,
    ParamsSetup,
    SequenceSetup,
    SequenceResetup,
    SequenceSetdown,
    DoDialog,
    Render,
    UserChangedParam,
    UpdateParamsUi,
    Event,
    ArbitraryCallback,
    SmartPreRender,
    SmartRender,
}

/// Codes with no handler of their own, and codes above the table, fall to the default.
pub fn command_from_code(code: u32) -> Slot<Command> {
    Slot::Held(match code {
        CMD_ABOUT => Command::About,
        CMD_GLOBAL_SETUP => Command::GlobalSetup,
        CMD_GLOBAL_SETDOWN => Command::GlobalSetdown,
        CMD_PARAMS_SETUP => Command::ParamsSetup,
        CMD_SEQUENCE_SETUP => Command::SequenceSetup,
        CMD_SEQUENCE_RESETUP => Command::SequenceResetup,
        CMD_SEQUENCE_SETDOWN => Command::SequenceSetdown,
        CMD_DO_DIALOG => Command::DoDialog,
        CMD_RENDER => Command::Render,
        CMD_USER_CHANGED_PARAM => Command::UserChangedParam,
        CMD_UPDATE_PARAMS_UI => Command::UpdateParamsUi,
        CMD_EVENT => Command::Event,
        CMD_ARBITRARY_CALLBACK => Command::ArbitraryCallback,
        CMD_SMART_PRE_RENDER => Command::SmartPreRender,
        CMD_SMART_RENDER => Command::SmartRender,
        _ => return Slot::Empty,
    })
}

/// Codes a conforming host never sends, given the declared flags.
pub fn never_sent(code: u32) -> bool {
    CMD_NEVER_SENT.contains(&code)
}

/// The dialog command is not rebuilt: an explicit refusal that claims no host
/// return code.
pub fn dialog_answer() -> Result<i32, Error> {
    Err(Error::NotRebuilt)
}

/// The refusal a command carries, if any: the dialog command is refused (see
/// [`dialog_answer`]); every other command has none.
pub fn command_refusal(c: Command) -> Slot<Error> {
    match c {
        Command::DoDialog => match dialog_answer() {
            Err(e) => Slot::Held(e),
            Ok(_) => Slot::Empty,
        },
        _ => Slot::Empty,
    }
}

/// What a failed command answers with (inferred).
pub fn thrown_return(code: i32, r: &Readings) -> i32 {
    if r.thrown_code_is_returned {
        code
    } else {
        OK
    }
}

/// Engine run wrapper return: zero stays zero, any other engine value is the engine error.
pub fn engine_wrapper_return(engine: i32) -> i32 {
    if engine == OK {
        OK
    } else {
        ERR_ENGINE
    }
}

// -------------------------------------------------------- parameter changes

/// The action of a changed parameter index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamAction {
    /// Returns 0, nothing else.
    None,
    /// Feeds the sentinel tracker.
    SentinelTracker,
    /// Starts an engine action of the caller's choosing.
    ActionA,
    ActionB,
    ActionC,
    ActionD,
    ActionE,
    ActionF,
    /// The preset selector changed.
    PresetChange,
    /// One of the three target rows moved.
    TargetMoved,
}

/// The action of a changed host index. An index without a role is refused: the
/// refusal claims no host return code and names no role.
pub fn param_action(t: &RoleTable, index: i32) -> Result<ParamAction, Error> {
    Ok(match t.role_of(index).ok_or(Error::NotRebuilt)? {
        ParamRole::NoAction => ParamAction::None,
        ParamRole::SentinelA | ParamRole::SentinelB => ParamAction::SentinelTracker,
        ParamRole::ActionA => ParamAction::ActionA,
        ParamRole::ActionB => ParamAction::ActionB,
        ParamRole::ActionC => ParamAction::ActionC,
        ParamRole::ActionD => ParamAction::ActionD,
        ParamRole::ActionE => ParamAction::ActionE,
        ParamRole::ActionF => ParamAction::ActionF,
        ParamRole::PresetSelector => ParamAction::PresetChange,
        ParamRole::PresetTargetA | ParamRole::PresetTargetB | ParamRole::PresetTargetC => {
            ParamAction::TargetMoved
        }
    })
}

/// State machine step of the sentinel tracker (purpose inferred, steps read).
pub fn sentinel_step(state: u32, index: u32, r: &Readings) -> u32 {
    if !r.sentinel_tracker_enabled {
        return state;
    }
    if state == SENTINEL_VACANT {
        index
    } else if state != index {
        SENTINEL_VACANT
    } else {
        state
    }
}

/// The engine boolean: true when the integer holds an index.
pub fn sentinel_recorded(state: u32) -> bool {
    state != SENTINEL_VACANT
}

/// One write of the preset loop: host index and widened value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamWrite {
    pub index: i32,
    pub value: f64,
}

/// Result of the preset selector change.
#[derive(Debug, Clone, PartialEq)]
pub enum PresetLoad {
    /// The free position: nothing is done.
    NoOp,
    /// Writes in target order, each followed by a changed-value mark and an interface update.
    Load([ParamWrite; 3]),
}

/// The selector value is read directly as a one based position in the preset list.
pub fn preset_load(t: &RoleTable, v: i32, r: &Readings) -> Result<PresetLoad, Error> {
    if v == t.free_position {
        return Ok(PresetLoad::NoOp);
    }
    let slot = usize::try_from(v.wrapping_sub(1))
        .ok()
        .filter(|s| *s < t.presets.len());
    let Some(slot) = slot else {
        return if r.preset_out_of_range_noop {
            Ok(PresetLoad::NoOp)
        } else {
            Err(Error::Unspecified(
                "selector position outside the preset table",
            ))
        };
    };
    let p = t.presets[slot];
    let at = |role| t.index_of(role).ok_or(Error::NotRebuilt);
    Ok(PresetLoad::Load([
        ParamWrite {
            index: at(ParamRole::PresetTargetA)?,
            value: f64::from(p.a),
        },
        ParamWrite {
            index: at(ParamRole::PresetTargetB)?,
            value: f64::from(p.b),
        },
        ParamWrite {
            index: at(ParamRole::PresetTargetC)?,
            value: f64::from(p.c),
        },
    ]))
}

/// With an error in the loop, later interface updates are skipped but the writes
/// continue; the return is the first error. Input: the error code of each update.
pub fn preset_load_outcome(update_errors: [i32; 3]) -> (i32, [bool; 3]) {
    let mut first = OK;
    let mut called = [false; 3];
    for (i, e) in update_errors.iter().enumerate() {
        if first == OK {
            called[i] = true;
            if *e != OK {
                first = *e;
            }
        }
    }
    (first, called)
}

/// A target row moved: true means the selector is set to the free position and updated.
pub fn target_moved_sets_free(t: &RoleTable, selector_value: i32) -> bool {
    selector_value != t.free_position
}

// -------------------------------------------------------------- interface rules

/// Inputs of the interface rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiInputs {
    /// Value of gate row a.
    pub gate_a: i32,
    /// Position of selector b.
    pub selector_b: i32,
    /// Position of selector c.
    pub selector_c: i32,
    /// Engine capability word.
    pub flags_w: u64,
}

/// State of each row the rules touch (true = disabled, or shown as the field
/// says).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiState {
    /// The two rows disabled while gate row a is on.
    pub gate_dep_a_disabled: bool,
    pub gate_dep_b_disabled: bool,
    /// Dependent row c, disabled unless selector b is one of its two pair positions.
    pub selector_b_dep_c_disabled: bool,
    /// Dependent rows a and b, disabled unless selector b is at its single position.
    pub selector_b_dep_a_disabled: bool,
    pub selector_b_dep_b_disabled: bool,
    /// Empty when the capability bit is clear (the paired rows are untouched).
    pub paired: Slot<Paired>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paired {
    pub dep_a_shown: bool,
    pub dep_b_shown: bool,
    pub dep_c_shown: bool,
}

/// The mode code of a selector c position; unlisted positions give [`MODE_PLAIN`].
pub fn selector_c_mode(t: &RoleTable, selector_value: i32) -> i32 {
    t.selector_c_modes
        .iter()
        .find(|(k, _)| *k == selector_value)
        .map_or(MODE_PLAIN, |(_, m)| *m)
}

/// The pure interface update function.
pub fn update_ui(t: &RoleTable, i: &UiInputs) -> UiState {
    let m = &t.selector_b_positions;
    let paired = if i.flags_w & t.paired_capability_mask != 0 {
        let mode = selector_c_mode(t, i.selector_c);
        Slot::Held(Paired {
            dep_a_shown: mode == MODE_PAIRED,
            dep_b_shown: mode == MODE_PAIRED,
            dep_c_shown: mode != MODE_PLAIN,
        })
    } else {
        Slot::Empty
    };
    UiState {
        gate_dep_a_disabled: i.gate_a != 0,
        gate_dep_b_disabled: i.gate_a != 0,
        selector_b_dep_c_disabled: !m.pair.contains(&i.selector_b),
        selector_b_dep_a_disabled: i.selector_b != m.single,
        selector_b_dep_b_disabled: i.selector_b != m.single,
        paired,
    }
}

/// A host call happens only if the ui flags differ.
pub fn update_needs_host_call(current_ui_flags: u32, wanted: u32) -> bool {
    current_ui_flags != wanted
}

/// Enable rule on the ui flags dword.
pub fn enable_rule_flags(flags: u32, disabled: bool) -> u32 {
    if disabled {
        flags | PUI_DISABLED
    } else {
        flags & !PUI_DISABLED
    }
}

/// Shown rule on the ui flags dword.
pub fn shown_rule_flags(flags: u32, shown: bool) -> u32 {
    if shown {
        flags & !PUI_INVISIBLE
    } else {
        flags | PUI_INVISIBLE
    }
}

// ------------------------------------------------------------------- events

/// Event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    DoClick,
    Drag,
    Draw,
    AdjustCursor,
    /// Returns 0, no host call, event out flags untouched.
    Ignored,
}

pub fn event_kind(e_type: i32) -> EventKind {
    match e_type {
        EVENT_DO_CLICK => EventKind::DoClick,
        EVENT_DRAG => EventKind::Drag,
        EVENT_DRAW => EventKind::Draw,
        EVENT_ADJUST_CURSOR => EventKind::AdjustCursor,
        _ => EventKind::Ignored,
    }
}

/// Hover hit box: the click box inset by one on every side.
pub fn hover_hit(t: &RoleTable, area: i32, left: i32, top: i32, x: i32, y: i32) -> bool {
    let (w, h) = (i64::from(t.control_size.0), i64::from(t.control_size.1));
    let (l, tp, x, y) = (i64::from(left), i64::from(top), i64::from(x), i64::from(y));
    area == AREA_CONTROL && l + 1 < x && x < l + w && tp + 1 < y && y < tp + h
}

/// Click hit box: L < x <= L + width and T < y <= T + height.
pub fn click_hit(t: &RoleTable, area: i32, left: i32, top: i32, x: i32, y: i32) -> bool {
    let (w, h) = (i64::from(t.control_size.0), i64::from(t.control_size.1));
    let (l, tp, x, y) = (i64::from(left), i64::from(top), i64::from(x), i64::from(y));
    area == AREA_CONTROL && l < x && x <= l + w && tp < y && y <= tp + h
}

/// New hover flag (always cleared first, then set on a hit).
pub fn adjust_cursor_hover(t: &RoleTable, area: i32, left: i32, top: i32, x: i32, y: i32) -> bool {
    hover_hit(t, area, left, top, x, y)
}

/// Effects of a click on the event record and state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClickEffect {
    pub send_drag: bool,
    /// New hover flag, empty if unchanged.
    pub hover: Slot<bool>,
    pub invalidate_rect: bool,
    pub evt_out_or: u32,
}

const NO_EFFECT: ClickEffect = ClickEffect {
    send_drag: false,
    hover: Slot::Empty,
    invalidate_rect: false,
    evt_out_or: 0,
};

/// A click: invalidation is skipped on the second host kind (`premiere`).
pub fn do_click_effect(
    t: &RoleTable,
    area: i32,
    left: i32,
    top: i32,
    x: i32,
    y: i32,
    premiere: bool,
) -> ClickEffect {
    if !click_hit(t, area, left, top, x, y) {
        return NO_EFFECT;
    }
    ClickEffect {
        send_drag: true,
        hover: Slot::Held(true),
        invalidate_rect: !premiere,
        evt_out_or: EVT_HANDLED_UPDATE,
    }
}

/// Effects of a drag release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragEffect {
    /// Launch function with mode 0.
    pub launch_mode0: bool,
    pub invalidate_rect: bool,
    /// The change flags of the first two rows are assigned 1.
    pub assign_changed_flags: bool,
    pub hover: Slot<bool>,
    pub evt_out_or: u32,
}

/// Acts only when last_time is nonzero and the click hit test holds.
#[allow(clippy::too_many_arguments)]
pub fn drag_effect(
    t: &RoleTable,
    area: i32,
    left: i32,
    top: i32,
    x: i32,
    y: i32,
    last_time: u32,
    premiere: bool,
) -> DragEffect {
    if last_time == 0 || !click_hit(t, area, left, top, x, y) {
        return DragEffect {
            launch_mode0: false,
            invalidate_rect: false,
            assign_changed_flags: false,
            hover: Slot::Empty,
            evt_out_or: 0,
        };
    }
    DragEffect {
        launch_mode0: true,
        invalidate_rect: !premiere,
        assign_changed_flags: true,
        hover: Slot::Held(false),
        evt_out_or: EVT_HANDLED_UPDATE,
    }
}

/// The decision part of DRAW.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawPlan {
    pub bitmap: u32,
    /// Origin as two f32: twice left, twice top.
    pub origin: [f32; 2],
    /// First transform diag(s, s, 1).
    pub scale: f32,
    /// Second transform diag(2, 2, 1), second host kind only.
    pub restore_scale: Slot<f32>,
    pub evt_out_flags_assign: u32,
}

/// Empty when the area is not the control area (nothing happens) or the chosen bitmap is missing.
#[allow(clippy::too_many_arguments)]
pub fn draw_plan(
    t: &RoleTable,
    area: i32,
    hover: bool,
    have_normal: bool,
    have_hover: bool,
    left: i32,
    top: i32,
    premiere: bool,
    r: &Readings,
) -> Slot<DrawPlan> {
    if area != AREA_CONTROL {
        return Slot::Empty;
    }
    let (bitmap, ok) = if hover {
        (t.bitmaps.hover, have_hover)
    } else {
        (t.bitmaps.normal, have_normal)
    };
    if !ok {
        return Slot::Empty;
    }
    Slot::Held(DrawPlan {
        bitmap,
        origin: [2.0f32 * left as f32, 2.0f32 * top as f32],
        scale: r.draw_scale,
        restore_scale: if premiere {
            Slot::Held(2.0)
        } else {
            Slot::Empty
        },
        evt_out_flags_assign: EVT_HANDLED,
    })
}

// ------------------------------------------------------------ launch, about

/// Guard dialog result to (state, quiet flag): state 0 abort, 1 go, 2 go at full resolution.
pub fn guard_state(t: &RoleTable, dialog_result: i32) -> (u8, bool) {
    let d = &t.dialog_results;
    if dialog_result == d.go_quiet {
        (1, true)
    } else if dialog_result == d.go {
        (1, false)
    } else if dialog_result == d.go_full {
        (2, false)
    } else {
        (0, false)
    }
}

/// Ratio threshold under which the guard dialog is shown.
const GUARD_RATIO: f64 = 0.99;

/// The dialog is shown when not suppressed and a ratio is below the threshold.
pub fn guard_shows_dialog(quiet: bool, ratio_x: f64, ratio_y: f64) -> bool {
    !quiet && (ratio_x < GUARD_RATIO || ratio_y < GUARD_RATIO)
}

/// Ratio numerator over denominator, 1.0 when the denominator is 0.
pub fn downsample_ratio(num: i32, den: i32) -> f64 {
    if den == 0 {
        1.0
    } else {
        f64::from(num) / f64::from(den)
    }
}

/// Return of the launch function from the engine value.
pub fn launch_return(engine: i32) -> i32 {
    match engine {
        0 | 1 => OK,
        _ => ERR_LAUNCH,
    }
}

/// Whether the error stream text is written.
pub fn launch_writes_message(engine: i32) -> bool {
    engine != 0 && engine != 1
}

/// The error stream text, a text by role.
pub fn error_stream_text(t: &RoleTable) -> &str {
    &t.texts.error_stream
}

/// ABOUT outcome from the engine result (empty: engine absent or NULL args).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AboutOutcome {
    /// Engine returned 0: return 0, out_data untouched.
    EngineHandled,
    /// Fallback text is written to the return message and 0 returned.
    Fallback,
}

pub fn about_outcome(engine: Slot<i32>) -> AboutOutcome {
    match engine {
        Slot::Held(0) => AboutOutcome::EngineHandled,
        _ => AboutOutcome::Fallback,
    }
}

/// The fallback text, a text by role: the caller's template with its name,
/// major and minor placeholders filled.
pub fn about_text(t: &RoleTable, name: &str, major: i32, minor: i32) -> String {
    let template = &t.texts.about_template;
    let major = major.to_string();
    let minor = minor.to_string();
    let mut out = String::with_capacity(template.len() + name.len());
    let mut rest = template.as_str();
    while let Some(at) = rest.find('{') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let (fill, len) = if tail.starts_with("{name}") {
            (name, "{name}".len())
        } else if tail.starts_with("{num_a}") {
            (major.as_str(), "{num_a}".len())
        } else if tail.starts_with("{num_b}") {
            (minor.as_str(), "{num_b}".len())
        } else {
            ("{", 1)
        };
        out.push_str(fill);
        rest = &tail[len..];
    }
    out.push_str(rest);
    out
}

/// ABOUT return when the ANSI suite is refused.
pub fn about_refused_return(r: &Readings) -> i32 {
    if r.about_throw_returns_code {
        ERR_SUITE
    } else {
        OK
    }
}

// ---------------------------------------------------------------- sequences

/// Ids with an underscore (render only projects) skip the registry.
pub fn setdown_skips_registry(id: &str) -> bool {
    id.contains('_')
}

/// Render only id from a UUID text (inferred, medium).
pub fn render_only_id(uuid_text: &str) -> String {
    uuid_text.replace('-', "_")
}
