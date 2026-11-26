//! Tests of the unit table hook with made up handlers: routing, null pointers reaching the
//! handler, panics, the default case and the registered table.
use effect_shell::arb_layout::CMD_ARBITRARY_CALLBACK;
use effect_shell::layout::*;
use effect_shell::render_layout::*;
use effect_shell::units::*;
use std::cell::RefCell;
use std::ptr::null_mut;

thread_local! {
    static SEEN: RefCell<Vec<(i32, bool, bool, bool)>> = const { RefCell::new(Vec::new()) };
}

unsafe fn rec(a: &UnitArgs) -> PfErr {
    SEEN.with(|s| {
        s.borrow_mut().push((
            a.cmd,
            a.in_data.is_null(),
            a.params.is_null(),
            a.extra.is_null(),
        ))
    });
    100 + a.cmd
}
unsafe fn boom(_a: &UnitArgs) -> PfErr {
    panic!("handler panics")
}

fn full() -> UnitTable {
    UnitTable {
        global_setdown: Some(rec),
        sequence_setup: Some(rec),
        sequence_setdown: Some(rec),
        render: Some(rec),
        pre_render: Some(rec),
        smart_render: Some(rec),
        arbitrary: Some(rec),
    }
}

fn plan() -> effect_shell::plan::Plan {
    effect_shell::plan::Plan::parse("table v1\nOne\tcheckbox\ti:0\t?\t?\t?\n").unwrap()
}

unsafe fn call(
    t: &UnitTable,
    cmd: i32,
    in_data: *mut InData,
    extra: *mut std::ffi::c_void,
) -> PfErr {
    let p = plan();
    effect_shell::entry_with_units(
        Ok(&p),
        t,
        cmd,
        in_data,
        null_mut(),
        null_mut(),
        null_mut(),
        extra,
        0,
    )
}

#[test]
fn command_numbers_are_the_sdk_ones() {
    assert_eq!(
        [
            CMD_GLOBAL_SETDOWN,
            CMD_SEQUENCE_SETUP,
            CMD_SEQUENCE_SETDOWN,
            CMD_RENDER,
            CMD_SMART_PRE_RENDER,
            CMD_SMART_RENDER
        ],
        [3, 5, 8, 11, 23, 24]
    );
}

#[test]
fn every_unit_command_reaches_its_handler_with_raw_null_pointers() {
    SEEN.with(|s| s.borrow_mut().clear());
    let t = full();
    for cmd in [3, 5, 8, 11, 23, 24] {
        assert_eq!(unsafe { call(&t, cmd, null_mut(), null_mut()) }, 100 + cmd);
    }
    let seen = SEEN.with(|s| s.borrow().clone());
    assert_eq!(seen.len(), 6);
    assert!(seen.iter().all(|s| s.1 && s.2 && s.3));
}

#[test]
fn a_missing_handler_is_the_default_case() {
    let t = UnitTable {
        render: Some(rec),
        ..UnitTable::EMPTY
    };
    unsafe {
        assert_eq!(
            call(&t, CMD_SMART_RENDER, null_mut(), null_mut()),
            ERR_BAD_CALLBACK_PARAM,
            "falls to dispatch, which refuses a null in_data"
        );
        assert_eq!(
            call(&UnitTable::EMPTY, CMD_RENDER, null_mut(), null_mut()),
            ERR_BAD_CALLBACK_PARAM
        );
        assert_eq!(
            call(&t, CMD_RENDER, null_mut(), null_mut()),
            100 + CMD_RENDER
        );
    }
}

#[test]
fn unit_commands_without_a_handler_return_zero_with_records() {
    // SAFETY: zero is valid for both records.
    let mut ind: InData = unsafe { std::mem::zeroed() };
    let mut outd: OutData = unsafe { std::mem::zeroed() };
    let p = plan();
    for cmd in [3, 5, 8, 11, 23, 24, 10, 12] {
        let e = unsafe {
            effect_shell::entry_with_units(
                Ok(&p),
                &UnitTable::EMPTY,
                cmd,
                &mut ind,
                &mut outd,
                null_mut(),
                null_mut(),
                null_mut(),
                0,
            )
        };
        assert_eq!(e, 0, "cmd {cmd}");
    }
}

#[test]
fn a_panicking_handler_gives_internal_struct_damaged() {
    let t = UnitTable {
        render: Some(boom),
        ..UnitTable::EMPTY
    };
    assert_eq!(
        unsafe { call(&t, CMD_RENDER, null_mut(), null_mut()) },
        ERR_INTERNAL_STRUCT_DAMAGED
    );
}

#[test]
fn other_commands_take_the_dispatch_path() {
    let t = full();
    // GLOBAL_SETUP with null records is refused by dispatch, not routed to a handler
    assert_eq!(
        unsafe { call(&t, CMD_GLOBAL_SETUP, null_mut(), null_mut()) },
        ERR_BAD_CALLBACK_PARAM
    );
}

#[test]
fn the_registered_table_defaults_to_empty() {
    assert!(registered_units().handler(CMD_RENDER).is_none());
}

#[test]
fn rect_types_are_exported() {
    let r = Rect {
        left: 0,
        top: 0,
        right: 2,
        bottom: 2,
    };
    assert!(!r.is_empty());
}

#[test]
fn the_arbitrary_data_callback_command_reaches_its_handler() {
    SEEN.with(|s| s.borrow_mut().clear());
    assert_eq!(CMD_ARBITRARY_CALLBACK, 22);
    let t = full();
    assert_eq!(
        unsafe { call(&t, CMD_ARBITRARY_CALLBACK, null_mut(), null_mut()) },
        122
    );
    let none = UnitTable {
        arbitrary: None,
        ..full()
    };
    assert_eq!(
        unsafe { call(&none, CMD_ARBITRARY_CALLBACK, null_mut(), null_mut()) },
        ERR_BAD_CALLBACK_PARAM,
        "no handler: the default dispatch refuses a null in_data"
    );
}
