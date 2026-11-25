//! A generic After Effects effect plug-in shell. It exports the host entry point, dispatches the
//! commands ABOUT, GLOBAL_SETUP and PARAMS_SETUP and registers every parameter of a table given at
//! build time (the environment variable EFFECT_SHELL_TABLE, read by build.rs). The crate ships only a
//! made up sample table. No engine is involved: the engine flags word is an input. See README.md
//! for the table format, the registration steps and the Windows build.
//!
//! With the cargo feature `freestanding` (and `--no-default-features`) the crate is `no_std` plus
//! `alloc` over a small static arena, with its own panic handler and memory functions, and the
//! Windows cdylib then imports nothing from the system: see README.md, section "Freestanding
//! build". The default build is the ordinary std build and is unchanged.

#![cfg_attr(feature = "freestanding", no_std)]

#[cfg(all(feature = "freestanding", feature = "std"))]
compile_error!(
    "the cargo feature `freestanding` is no_std plus alloc: build with --no-default-features --features freestanding"
);

#[cfg(feature = "freestanding")]
extern crate alloc;

#[cfg(feature = "freestanding")]
pub mod freestanding;
pub mod layout;
pub mod pipl;
pub mod plan;
pub mod render_layout;
pub mod setup;
pub mod units;

use core::sync::atomic::{AtomicU64, Ordering};
use layout::*;
use plan::Plan;
#[cfg(not(feature = "freestanding"))]
use std::sync::OnceLock;
use units::{run_unit, UnitArgs, UnitTable};

static TABLE_TEXT: &str = include_str!(concat!(env!("OUT_DIR"), "/table.txt"));
static ENGINE_FLAGS: AtomicU64 = AtomicU64::new(0);
#[cfg(not(feature = "freestanding"))]
static PLAN: OnceLock<Result<Plan, plan::PlanError>> = OnceLock::new();

/// Sets the engine flags word that selects lists and ui flags in PARAMS_SETUP.
pub fn set_engine_flags(v: u64) {
    ENGINE_FLAGS.store(v, Ordering::SeqCst);
}

pub fn engine_flags() -> u64 {
    ENGINE_FLAGS.load(Ordering::SeqCst)
}

/// The plan compiled in at build time.
#[cfg(not(feature = "freestanding"))]
pub fn built_in_plan() -> Result<&'static Plan, &'static plan::PlanError> {
    PLAN.get_or_init(|| Plan::parse(TABLE_TEXT)).as_ref()
}

/// Dispatch one command over a plan and an explicit engine flags word.
///
/// # Safety
/// Pointers must be null or valid for the record they name; a null in_data or out_data returns
/// `ERR_BAD_CALLBACK_PARAM`.
pub unsafe fn dispatch(
    cmd: i32,
    plan: &Plan,
    in_data: *mut InData,
    out_data: *mut OutData,
    engine_flags: u64,
) -> PfErr {
    if in_data.is_null() || out_data.is_null() {
        return ERR_BAD_CALLBACK_PARAM;
    }
    let (inp, out) = (&*in_data, &mut *out_data);
    match cmd {
        CMD_ABOUT => setup::about(plan, out),
        CMD_GLOBAL_SETUP => setup::global_setup(plan, out),
        CMD_PARAMS_SETUP => setup::params_setup(plan, inp, out, engine_flags),
        _ => ERR_NONE,
    }
}

/// The entry logic over an explicit plan result: a plan that failed to parse gives
/// `ERR_INTERNAL_STRUCT_DAMAGED`, and so does a panic inside the dispatch.
///
/// # Safety
/// Same as [`dispatch`].
pub unsafe fn entry_with(
    plan: Result<&Plan, &plan::PlanError>,
    cmd: i32,
    in_data: *mut InData,
    out_data: *mut OutData,
    engine_flags: u64,
) -> PfErr {
    let plan = match plan {
        Ok(p) => p,
        Err(_) => return ERR_INTERNAL_STRUCT_DAMAGED,
    };
    #[cfg(feature = "freestanding")]
    {
        // panic=abort: a panic never returns here, so there is nothing to catch.
        dispatch(cmd, plan, in_data, out_data, engine_flags)
    }
    #[cfg(not(feature = "freestanding"))]
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispatch(cmd, plan, in_data, out_data, engine_flags)
    })) {
        Ok(e) => e,
        Err(_) => ERR_INTERNAL_STRUCT_DAMAGED,
    }
}

/// Like [`entry_with`] for the full argument list and a table of unit handlers. A command the
/// table serves goes to its handler, which receives the raw pointers (null checks are the
/// handler's); every other command takes the [`dispatch`] path. With the std build a panic gives
/// `ERR_INTERNAL_STRUCT_DAMAGED`; the freestanding build is panic=abort and has nothing to catch.
///
/// # Safety
/// Same as [`dispatch`].
#[allow(clippy::too_many_arguments)]
pub unsafe fn entry_with_units(
    plan: Result<&Plan, &plan::PlanError>,
    units: &UnitTable,
    cmd: i32,
    in_data: *mut InData,
    out_data: *mut OutData,
    params: *mut *mut ParamDef,
    output: *mut core::ffi::c_void,
    extra: *mut core::ffi::c_void,
    engine_flags: u64,
) -> PfErr {
    let args = UnitArgs {
        cmd,
        in_data,
        out_data,
        params,
        output,
        extra,
    };
    if units.handler(cmd).is_some() {
        #[cfg(feature = "freestanding")]
        {
            return run_unit(units, &args).unwrap_or(ERR_NONE);
        }
        #[cfg(not(feature = "freestanding"))]
        return match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_unit(units, &args)
        })) {
            Ok(Some(e)) => e,
            Ok(None) => ERR_NONE,
            Err(_) => ERR_INTERNAL_STRUCT_DAMAGED,
        };
    }
    entry_with(plan, cmd, in_data, out_data, engine_flags)
}

/// The entry logic of the built in plan over a table of unit handlers, for the crate that owns
/// the table and exports its own `EffectMain` (feature `external-entry`). The std build builds the
/// plan once and guards the whole call against a panic; the freestanding build parses the plan for
/// every call (no `OnceLock` without std) and is panic=abort, so there is nothing to guard.
///
/// # Safety
/// Called by the host with valid records; the pointers follow [`dispatch`] and the unit handlers.
pub unsafe fn entry_built_in(
    units: &UnitTable,
    cmd: i32,
    in_data: *mut InData,
    out_data: *mut OutData,
    params: *mut *mut ParamDef,
    output: *mut core::ffi::c_void,
    extra: *mut core::ffi::c_void,
) -> PfErr {
    #[cfg(not(feature = "freestanding"))]
    {
        // The plan is built (once) inside the guard too: a panic there must not reach the host either.
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_with_units(
                built_in_plan(),
                units,
                cmd,
                in_data,
                out_data,
                params,
                output,
                extra,
                engine_flags(),
            )
        }))
        .unwrap_or(ERR_INTERNAL_STRUCT_DAMAGED)
    }
    #[cfg(feature = "freestanding")]
    {
        // No OnceLock without std: the plan is parsed for a call that needs it and dropped
        // afterwards, then the arena is emptied (the plan is not reclaimed otherwise). A command
        // that is neither a setup command nor served by the unit table answers success without
        // parsing, so it allocates nothing and cannot race. panic=abort: there is no unwinding to
        // guard.
        if units.handler(cmd).is_none()
            && !matches!(cmd, CMD_ABOUT | CMD_GLOBAL_SETUP | CMD_PARAMS_SETUP)
        {
            return ERR_NONE;
        }
        let r = {
            let plan = Plan::parse(TABLE_TEXT);
            entry_with_units(
                plan.as_ref(),
                units,
                cmd,
                in_data,
                out_data,
                params,
                output,
                extra,
                engine_flags(),
            )
        };
        // SAFETY: the plan and every other allocation of this call were dropped in the block above,
        // and no allocation lives in a static. The host calls these commands from one thread. A unit
        // handler must not keep an arena allocation beyond its call (it receives no allocator state
        // and the table holds function pointers only).
        freestanding::reset();
        r
    }
}

/// The host entry point.
///
/// # Safety
/// Called by the host with valid records.
///
/// With the cargo feature `external-entry` the shell does not export this symbol: another crate
/// exports its own `EffectMain` and calls [`entry_built_in`] or [`entry_with_units`].
#[cfg(not(feature = "external-entry"))]
#[no_mangle]
pub unsafe extern "C" fn EffectMain(
    cmd: i32,
    in_data: *mut InData,
    out_data: *mut OutData,
    params: *mut *mut ParamDef,
    output: *mut core::ffi::c_void,
    extra: *mut core::ffi::c_void,
) -> PfErr {
    entry_built_in(
        units::registered_units(),
        cmd,
        in_data,
        out_data,
        params,
        output,
        extra,
    )
}
