//! A table of command handlers from another crate, which the shell calls for the arbitrary data and
//! sequence, render and setdown commands when the table holds one; without a handler the command
//! falls to the default case and returns 0. The table carries no plug-in fact.

use crate::arb_layout::CMD_ARBITRARY_CALLBACK;
use crate::layout::*;
use crate::render_layout::*;
use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, Ordering};

/// The raw arguments of one command, as the host passed them.
pub struct UnitArgs {
    pub cmd: i32,
    pub in_data: *mut InData,
    pub out_data: *mut OutData,
    pub params: *mut *mut ParamDef,
    pub output: *mut c_void,
    pub extra: *mut c_void,
}

/// One command handler. It receives the raw pointers (they may be null) and owns every check.
pub type UnitFn = unsafe fn(&UnitArgs) -> PfErr;

/// The handlers by command. A `None` entry means the default case.
#[derive(Clone, Copy, Default)]
pub struct UnitTable {
    pub global_setdown: Option<UnitFn>,
    pub sequence_setup: Option<UnitFn>,
    pub sequence_setdown: Option<UnitFn>,
    pub render: Option<UnitFn>,
    pub pre_render: Option<UnitFn>,
    pub smart_render: Option<UnitFn>,
    /// The arbitrary data callback command (command 22).
    pub arbitrary: Option<UnitFn>,
}

impl UnitTable {
    pub const EMPTY: UnitTable = UnitTable {
        global_setdown: None,
        sequence_setup: None,
        sequence_setdown: None,
        render: None,
        pre_render: None,
        smart_render: None,
        arbitrary: None,
    };

    /// The handler for a command number, if the command is one the table serves.
    pub fn handler(&self, cmd: i32) -> Option<UnitFn> {
        match cmd {
            CMD_GLOBAL_SETDOWN => self.global_setdown,
            CMD_SEQUENCE_SETUP => self.sequence_setup,
            CMD_SEQUENCE_SETDOWN => self.sequence_setdown,
            CMD_RENDER => self.render,
            CMD_SMART_PRE_RENDER => self.pre_render,
            CMD_SMART_RENDER => self.smart_render,
            CMD_ARBITRARY_CALLBACK => self.arbitrary,
            _ => None,
        }
    }
}

static REGISTERED: AtomicPtr<UnitTable> = AtomicPtr::new(core::ptr::null_mut());

/// Registers the table the built in entry point uses. The table must live for the whole process.
pub fn register_units(table: &'static UnitTable) {
    REGISTERED.store(
        table as *const UnitTable as *mut UnitTable,
        Ordering::SeqCst,
    );
}

/// The registered table, or the empty table.
pub fn registered_units() -> &'static UnitTable {
    let p = REGISTERED.load(Ordering::SeqCst);
    if p.is_null() {
        &UnitTable::EMPTY
    } else {
        // SAFETY: only `register_units` stores here, with a 'static reference.
        unsafe { &*p }
    }
}

/// Runs the handler of a unit command. Returns `None` when the command is not a unit command or
/// the table has no handler for it (default case, error code 0 by the caller).
///
/// # Safety
/// Pointers must be null or valid for the record they name.
pub unsafe fn run_unit(table: &UnitTable, args: &UnitArgs) -> Option<PfErr> {
    table.handler(args.cmd).map(|f| f(args))
}
