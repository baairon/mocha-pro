//! The registered unit table, in its own test binary because the table is process wide: a table
//! registered here is the one `registered_units` returns and the one the exported entry point
//! routes unit commands to. The handler is made up.
use effect_shell::layout::*;
use effect_shell::render_layout::*;
use effect_shell::units::*;
use std::ptr::null_mut;

unsafe fn answer(a: &UnitArgs) -> PfErr {
    7000 + a.cmd
}

static TABLE: UnitTable = UnitTable {
    render: Some(answer),
    ..UnitTable::EMPTY
};

#[test]
fn a_registered_table_is_returned_and_used_by_the_entry_point() {
    assert!(registered_units().handler(CMD_RENDER).is_none());
    register_units(&TABLE);
    let got = registered_units();
    assert!(std::ptr::eq(got, &TABLE));
    let f = got
        .handler(CMD_RENDER)
        .expect("the registered render handler");
    let args = UnitArgs {
        cmd: CMD_RENDER,
        in_data: null_mut(),
        out_data: null_mut(),
        params: null_mut(),
        output: null_mut(),
        extra: null_mut(),
    };
    assert_eq!(unsafe { f(&args) }, 7000 + CMD_RENDER);
    assert!(got.handler(CMD_SMART_RENDER).is_none());
    #[cfg(not(feature = "external-entry"))]
    {
        let e = unsafe {
            effect_shell::EffectMain(
                CMD_RENDER,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };
        assert_eq!(e, 7000 + CMD_RENDER);
    }
}
