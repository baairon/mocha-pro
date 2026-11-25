//! Version 2 table registration: the exact sequence of host calls, every field of every
//! definition (as bytes), reset modes, num_params and the error paths. Mock host, made up data.

mod common;

use common::*;
use effect_shell::layout::*;
use effect_shell::plan::Plan;
use std::ffi::c_void;

const SAMPLE: &str = include_str!("../sample/sample_v2.txt");
const REFCON: u64 = 0x00c0_ffee_0011_2233;

fn sample() -> Plan {
    Plan::parse(SAMPLE).expect("sample v2 parses")
}

fn run_params(plan: &Plan, i: &mut InData, o: &mut OutData) -> PfErr {
    unsafe { effect_shell::dispatch(CMD_PARAMS_SETUP, plan, i, o, 0) }
}

/// An independent byte image of a definition (176 bytes).
struct Img(Vec<u8>);

impl Img {
    fn new(fill: u8) -> Img {
        Img(vec![fill; 176])
    }
    fn put(&mut self, off: usize, b: &[u8]) {
        self.0[off..off + b.len()].copy_from_slice(b);
    }
    fn i32(&mut self, off: usize, v: i32) -> &mut Img {
        self.put(off, &v.to_le_bytes());
        self
    }
    fn i16(&mut self, off: usize, v: i16) -> &mut Img {
        self.put(off, &v.to_le_bytes());
        self
    }
    fn u8(&mut self, off: usize, v: u8) -> &mut Img {
        self.0[off] = v;
        self
    }
    fn f32(&mut self, off: usize, v: f32) -> &mut Img {
        self.put(off, &v.to_le_bytes());
        self
    }
    fn f64(&mut self, off: usize, v: f64) -> &mut Img {
        self.put(off, &v.to_le_bytes());
        self
    }
    fn u64(&mut self, off: usize, v: u64) -> &mut Img {
        self.put(off, &v.to_le_bytes());
        self
    }
    /// id, ui flags, type, name (bytes and one NUL), flags
    fn head(&mut self, id: i32, ui: u32, ty: i32, name: &str, flags: u32) -> &mut Img {
        self.i32(0, id);
        self.i32(4, ui as i32);
        self.i32(12, ty);
        self.put(16, name.as_bytes());
        self.0[16 + name.len()] = 0;
        self.i32(48, flags as i32);
        self
    }
    /// pointer slots are masked on both sides
    fn masked(&mut self, offs: &[usize]) -> &mut Img {
        for o in offs {
            self.put(*o, &[0u8; 8]);
        }
        self
    }
}

fn actual(d: &ParamDef, ptrs: &[usize]) -> Vec<u8> {
    let mut b = bytes_of(d);
    for o in ptrs {
        b[*o..*o + 8].copy_from_slice(&[0u8; 8]);
    }
    b
}

fn ptr_at(d: &ParamDef, off: usize) -> usize {
    let b = bytes_of(d);
    usize::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

#[test]
fn global_setup_answers_the_header_words() {
    let plan = sample();
    let mut o = out();
    let mut i = host(APPL_ID_AE);
    reset(Cfg::default());
    let e = unsafe { effect_shell::dispatch(CMD_GLOBAL_SETUP, &plan, &mut i, &mut o, 0) };
    assert_eq!(e, 0);
    assert_eq!(o.my_version, 0x0003_0002);
    assert_eq!(o.out_flags, 0x2b0);
    assert_eq!(o.out_flags2, 0x1e0);
    assert_eq!(o.num_params, 0);
    assert!(events().is_empty(), "GLOBAL_SETUP makes no host call");
}

#[test]
fn the_exact_host_call_sequence_for_the_sample() {
    let plan = sample();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let add = |id: i32| format!("add_param ref=0x1234 index=-1 id={id}");
    let mut want: Vec<String> = Vec::new();
    for s in [
        "acquire PF Handle Suite 2",
        "new_handle 4",
        "lock 1",
        "unlock 1",
        "release PF Handle Suite 2",
    ] {
        want.push(s.into());
    }
    want.push(add(4101));
    for s in [
        "acquire PF Handle Suite 2",
        "new_handle 0",
        "release PF Handle Suite 2",
    ] {
        want.push(s.into());
    }
    for id in 4102..=4108 {
        want.push(add(id));
    }
    want.push("acquire PF Effect UI Suite 1".into());
    want.push("set_options_name ref=0x1234 \"Sample options...\"".into());
    want.push("release PF Effect UI Suite 1".into());
    for id in 4109..=4114 {
        want.push(add(id));
    }
    want.push("register_ui ref=0x1234 events=3".into());
    assert_eq!(events(), want);
    assert_eq!(o.num_params, 15);
    assert_eq!(defs().len(), 14);
    assert_eq!(handle_bytes(1), vec![0xa5, 0xb6, 0xc7, 0xd8]);
    assert_eq!(handle_bytes(2), Vec::<u8>::new());
    let ui = UI_INFO.with(|u| u.borrow().clone());
    assert_eq!(
        ui,
        vec![CustomUiInfo {
            events: 3,
            ..CustomUiInfo::default()
        }]
    );
}

#[test]
fn every_byte_of_every_definition() {
    let plan = sample();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let d = defs();
    let t = texts();
    // 1 arbitrary with four data bytes: id, handle, null value, the refcon option
    let mut e = Img::new(0);
    e.head(4101, 0x78, 11, "Blob One", 0x71)
        .i16(56, 4101)
        .u64(80, REFCON);
    assert_eq!(actual(&d[0], &[64]), e.masked(&[64]).0);
    assert_eq!(ptr_at(&d[0], 64), 1);
    assert_eq!(ptr_at(&d[0], 72), 0);
    // 2 arbitrary with no data
    let mut e = Img::new(0);
    e.head(4102, 0x78, 11, "Blob Two", 0x72)
        .i16(56, 4102)
        .u64(80, REFCON);
    assert_eq!(actual(&d[1], &[64]), e.masked(&[64]).0);
    assert_eq!(ptr_at(&d[1], 64), 2);
    // 3 checkbox with a label
    let mut e = Img::new(0);
    e.head(4103, 0x61, 4, "Flag A", 0x51).i32(56, 1).u8(60, 1);
    assert_eq!(actual(&d[2], &[64]), e.masked(&[64]).0);
    assert_eq!(t[2], "tick");
    // 4 popup, four choices (the separator entry counts), stated default 2
    let mut e = Img::new(0);
    e.head(4104, 0, 7, "Mode", 0x73)
        .i32(56, 2)
        .i16(60, 4)
        .i16(62, 2);
    assert_eq!(actual(&d[3], &[64]), e.masked(&[64]).0);
    assert_eq!(t[3], "Alpha|Beta|(-|Gamma");
    // 5 layer default none
    let mut e = Img::new(0);
    e.head(4105, 0, 0, "Src", 0x7d);
    assert_eq!(actual(&d[4], &[]), e.0);
    // 6 float slider
    let mut e = Img::new(0);
    e.head(4106, 0, 10, "Gain", 0x63)
        .f64(56, 12.5)
        .f32(104, -5.0)
        .f32(108, 50.0)
        .f32(112, -2.0)
        .f32(116, 20.0)
        .f32(120, 12.5)
        .i16(124, 3)
        .i16(126, 0x41);
    assert_eq!(actual(&d[5], &[]), e.0);
    // 7 point (16.16 fixed)
    let mut e = Img::new(0);
    e.head(4107, 0, 6, "Spot", 0x65)
        .i32(56, 25 * 65536)
        .i32(60, 75 * 65536)
        .i32(68, 25 * 65536)
        .i32(72, 75 * 65536);
    assert_eq!(actual(&d[6], &[]), e.0);
    // 8 angle
    let mut e = Img::new(0);
    e.head(4108, 0, 3, "Dial", 0)
        .i32(56, 45 * 65536)
        .i32(60, 45 * 65536)
        .i32(64, -90 * 65536)
        .i32(68, 90 * 65536);
    assert_eq!(actual(&d[7], &[]), e.0);
    // 9 button with an empty name and a label
    let mut e = Img::new(0);
    e.head(4109, 6, 15, "", 0x6d);
    assert_eq!(actual(&d[8], &[64]), e.masked(&[64]).0);
    assert_eq!(t[8], "Press Me");
    // 10 group, 11 group end, 12 no data with a size
    let mut e = Img::new(0);
    e.head(4110, 0, 13, "Box", 0);
    assert_eq!(actual(&d[9], &[]), e.0);
    let mut e = Img::new(0);
    e.head(4111, 0, 14, "", 0);
    assert_eq!(actual(&d[10], &[]), e.0);
    let mut e = Img::new(0);
    e.head(4112, 0xa, 9, "Pad", 0).i16(8, 61).i16(10, 57);
    assert_eq!(actual(&d[11], &[]), e.0);
    // 13 integer slider: valid range from min and max, slider range defaults to them
    let mut e = Img::new(0);
    e.head(4113, 0, 1, "Count", 0x79)
        .i32(56, 7)
        .i32(124, 0)
        .i32(128, 20)
        .i32(132, 0)
        .i32(136, 20)
        .i32(140, 7);
    assert_eq!(actual(&d[12], &[]), e.0);
    // 14 color
    let mut e = Img::new(0);
    e.head(4114, 0, 5, "Tint", 0)
        .u8(56, 0xff)
        .u8(59, 0xff)
        .u8(60, 0xff)
        .u8(63, 0xff);
    assert_eq!(actual(&d[13], &[]), e.0);
}

#[test]
fn a_layer_default_that_is_an_index_is_written_and_none_is_zero() {
    let text = "table v2\n\
        i:1\tL1\tlayer\ti:3\t?\t?\t?\t0x0\t0x0\t-\n\
        i:2\tL2\tlayer\tnone\t?\t?\t?\t0x0\t0x0\t-\n";
    let plan = Plan::parse(text).unwrap();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let d = defs();
    assert_eq!(bytes_of(&d[0])[172..176], 3i32.to_le_bytes());
    assert_eq!(bytes_of(&d[1])[172..176], 0i32.to_le_bytes());
}

#[test]
fn a_stated_popup_default_of_zero_is_not_clamped() {
    let text = "table v2\n\
        i:1\tP\tpopup\ti:0\t?\t?\t?\t0x0\t0x0\tc:a|b|c\n\
        i:2\tQ\tpopup\ti:9\t?\t?\t?\t0x0\t0x0\tc:a|b|c\n";
    let plan = Plan::parse(text).unwrap();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let d = defs();
    unsafe {
        assert_eq!(
            (d[0].u.pd.value, d[0].u.pd.dephault, d[0].u.pd.num_choices),
            (0, 0, 3)
        );
        assert_eq!((d[1].u.pd.value, d[1].u.pd.dephault), (9, 9));
    }
}

const REUSE: &str = "table v2\n\
    #@shell\treset=MODE\tfill=0xab\n\
    i:1\tLongNameHere\tcheckbox\ti:1\t?\t?\t?\t0x5\t0x6\tl:x\n\
    ?\tAb\tgroup\t?\t?\t?\t?\t?\t?\t?\n";

#[test]
fn reset_each_refills_the_structure_and_unstated_fields_hold_the_fill_byte() {
    let plan = Plan::parse(&REUSE.replace("MODE", "each")).unwrap();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let d = defs();
    // row 2 states only a name and a kind: everything else is the fill byte
    let mut e = Img::new(0xab);
    e.i32(12, 13);
    e.put(16, b"Ab");
    e.0[18] = 0;
    assert_eq!(bytes_of(&d[1]), e.0);
    // row 1 leaves the union bytes it does not write at the fill byte
    let b = bytes_of(&d[0]);
    assert!(
        b[8..12].iter().all(|x| *x == 0xab),
        "ui size words not stated"
    );
    assert_eq!(b[28], 0, "the name ends with one NUL");
    assert_eq!(
        b[29], 0xab,
        "and the rest of the name field is the fill byte"
    );
    assert_eq!(b[56 + 5], 0xab);
    assert_eq!(b[56 + 6], 0xab);
}

#[test]
fn reset_never_keeps_what_the_previous_row_left() {
    let plan = Plan::parse(&REUSE.replace("MODE", "never")).unwrap();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let d = defs();
    let a = bytes_of(&d[0]);
    let b = bytes_of(&d[1]);
    // row 2: id, flags and ui flags carry over, the type and the name are its own
    assert_eq!(b[0..4], 1i32.to_le_bytes());
    assert_eq!(b[4..8], 6i32.to_le_bytes());
    assert_eq!(b[12..16], 13i32.to_le_bytes());
    assert_eq!(b[48..52], 5i32.to_le_bytes());
    // the name is a string copy: "Ab", NUL, then the old bytes of "LongNameHere" from index 3
    assert_eq!(&b[16..19], b"Ab\0");
    assert_eq!(&b[19..28], b"gNameHere");
    assert_eq!(b[28], 0);
    assert_eq!(b[29], 0xab);
    // the union is the previous row's, pointer included (and it still points at live text)
    assert_eq!(b[56..], a[56..]);
    assert_eq!(texts()[0], "x");
}

#[test]
fn the_fill_byte_applies_to_the_first_row_in_never_mode() {
    let text = "table v2\n#@shell\treset=never\tfill=0x5c\n?\tG\tgroup\t?\t?\t?\t?\t?\t?\t?\n";
    let plan = Plan::parse(text).unwrap();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let b = bytes_of(&defs()[0]);
    assert_eq!(b[0..4], [0x5c; 4]);
    assert_eq!(b[12..16], 13i32.to_le_bytes());
    assert_eq!(&b[16..18], b"G\0");
    assert!(b[18..].iter().all(|x| *x == 0x5c));
}

#[test]
fn a_name_of_31_bytes_is_exact_and_32_is_refused() {
    let n31 = "n".repeat(31);
    let ok = format!("table v2\ni:1\t{n31}\tgroup\t?\t?\t?\t?\t?\t?\t?\n");
    let plan = Plan::parse(&ok).unwrap();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let b = bytes_of(&defs()[0]);
    assert_eq!(&b[16..47], n31.as_bytes());
    assert_eq!(b[47], 0);
    let bad = format!(
        "table v2\ni:1\t{}\tgroup\t?\t?\t?\t?\t?\t?\t?\n",
        "n".repeat(32)
    );
    let e = Plan::parse(&bad).unwrap_err();
    assert!(e.message.contains("32 byte field"), "{e}");
    assert_eq!(e.line, 2);
}

#[test]
fn plan_errors_of_the_version_two_directives_and_cells() {
    let head = "table v2\n";
    let row = "i:1\tG\tgroup\t?\t?\t?\t?\t?\t?\t?\n";
    for (bad, want) in [
        ("#@shell\treset=sometimes\n", "bad reset mode"),
        ("#@shell\tfill=256\n", "fill byte out of range"),
        ("#@shell\tfill=-1\n", "fill byte out of range"),
        ("#@shell\tcolor=1\n", "unknown shell key"),
        ("#@shell\trefcon=zz\n", "bad number for refcon"),
        ("#@shell\tregister_ui=q\n", "bad number for register_ui"),
    ] {
        let e = Plan::parse(&format!("{head}{bad}{row}")).unwrap_err();
        assert!(e.message.contains(want), "{want}: {e}");
        assert_eq!(e.line, 2);
    }
    let big = "i:4294967296\tG\tgroup\t?\t?\t?\t?\t?\t?\t?\n";
    let e = Plan::parse(&format!("{head}{big}")).unwrap_err();
    assert!(e.message.contains("does not fit 32 bits"), "{e}");
    let disp = "i:1\tS\tfloat\t?\t?\t?\t?\t?\t?\ts:2:0x10000:0000000000000000:0000000000000000\n";
    let e = Plan::parse(&format!("{head}{disp}")).unwrap_err();
    assert!(e.message.contains("does not fit 16 bits"), "{e}");
}

#[test]
fn a_refused_handle_suite_gives_the_callback_error_and_no_add_for_that_row() {
    let plan = sample();
    for cfg in [
        Cfg {
            handle_acquire_err: 9,
            ..Cfg::default()
        },
        Cfg {
            handle_null_table: true,
            ..Cfg::default()
        },
    ] {
        reset(cfg);
        let mut i = host(APPL_ID_AE);
        let mut o = out();
        assert_eq!(run_params(&plan, &mut i, &mut o), ERR_BAD_CALLBACK_PARAM);
        assert_eq!(events(), vec!["acquire PF Handle Suite 2".to_string()]);
        assert_eq!(o.num_params, 0);
    }
}

#[test]
fn no_basic_suite_at_all_gives_the_callback_error() {
    let plan = sample();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    i.pica_basic = std::ptr::null_mut();
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), ERR_BAD_CALLBACK_PARAM);
    assert!(events().is_empty());
    // a basic suite without an acquire member
    let mut empty = SpBasicSuite {
        acquire_suite: None,
        release_suite: None,
        is_equal: None,
        allocate_block: None,
        free_block: None,
        reallocate_block: None,
        undefined: None,
    };
    i.pica_basic = &mut empty;
    assert_eq!(run_params(&plan, &mut i, &mut o), ERR_BAD_CALLBACK_PARAM);
    assert!(defs().is_empty());
}

#[test]
fn a_handle_suite_without_a_lock_member_still_releases_the_suite() {
    static NO_LOCK: HandleSuite1 = HandleSuite1 {
        new_handle: Some(nl_new),
        lock_handle: None,
        unlock_handle: None,
        dispose_handle: None,
        get_handle_size: None,
        resize_handle: None,
    };
    unsafe extern "C" fn nl_new(size: u64) -> *mut c_void {
        log(format!("new_handle {size}"));
        0x10usize as *mut c_void
    }
    unsafe extern "C" fn nl_acquire(name: *const u8, v: i32, out: *mut *const c_void) -> i32 {
        log(format!("acquire {} {v}", text_of(name)));
        *out = &NO_LOCK as *const _ as *const c_void;
        0
    }
    static BASIC2: SpBasicSuite = SpBasicSuite {
        acquire_suite: Some(nl_acquire),
        release_suite: Some(nl_release),
        is_equal: None,
        allocate_block: None,
        free_block: None,
        reallocate_block: None,
        undefined: None,
    };
    unsafe extern "C" fn nl_release(name: *const u8, v: i32) -> i32 {
        log(format!("release {} {v}", text_of(name)));
        0
    }
    let plan = sample();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    i.pica_basic = &BASIC2 as *const _ as *mut _;
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), ERR_BAD_CALLBACK_PARAM);
    assert_eq!(
        events(),
        vec![
            "acquire PF Handle Suite 2".to_string(),
            "new_handle 4".to_string(),
            "release PF Handle Suite 2".to_string()
        ]
    );
}

#[test]
fn a_null_default_handle_is_stored_and_the_setup_goes_on() {
    let plan = sample();
    reset(Cfg {
        null_handle: true,
        ..Cfg::default()
    });
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let d = defs();
    assert_eq!(ptr_at(&d[0], 64), 0);
    assert!(!events().iter().any(|e| e.starts_with("lock")));
    assert_eq!(o.num_params, 15);
}

#[test]
fn a_null_lock_result_is_not_written_through_and_the_setup_goes_on() {
    let plan = sample();
    reset(Cfg {
        null_lock: true,
        ..Cfg::default()
    });
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    assert!(events().iter().any(|e| e.starts_with("lock")));
    assert!(events().iter().any(|e| e.starts_with("unlock")));
    assert_eq!(o.num_params, 15);
}

#[test]
fn an_add_param_error_stops_everything_and_is_returned_unchanged() {
    let plan = sample();
    reset(Cfg {
        add_fail: vec![(5, 7771)],
        ..Cfg::default()
    });
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 7771);
    assert_eq!(defs().len(), 5);
    let ev = events();
    assert!(!ev.iter().any(|e| e.starts_with("register_ui")));
    assert!(!ev.iter().any(|e| e.contains("Effect UI")));
    assert_eq!(o.num_params, 0);
}

#[test]
fn an_error_on_a_deferred_row_is_returned_at_the_end_without_the_final_steps() {
    let plan = sample();
    // the group row (4110) is call 10 and is marked defer_err
    reset(Cfg {
        add_fail: vec![(10, 3131)],
        ..Cfg::default()
    });
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 3131);
    assert_eq!(defs().len(), 14, "the setup went on after the error");
    assert!(!events().iter().any(|e| e.starts_with("register_ui")));
    assert_eq!(o.num_params, 0);
}

#[test]
fn the_registration_code_is_returned_and_num_params_is_still_written() {
    let plan = sample();
    reset(Cfg {
        register_ui_err: 641,
        ..Cfg::default()
    });
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 641);
    assert_eq!(o.num_params, 15);
    // a host without the callback
    reset(Cfg::default());
    i.inter.register_ui = None;
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), ERR_BAD_CALLBACK_PARAM);
}

#[test]
fn a_plan_without_the_registration_option_makes_no_register_call() {
    let text = SAMPLE.replace("\tregister_ui=3", "");
    let plan = Plan::parse(&text).unwrap();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    assert!(!events().iter().any(|e| e.starts_with("register_ui")));
    assert_eq!(o.num_params, 15);
}

#[test]
fn a_refused_effect_ui_suite_is_tolerated() {
    let plan = sample();
    reset(Cfg {
        refuse_ui: true,
        ..Cfg::default()
    });
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let ev = events();
    assert!(ev.contains(&"acquire PF Effect UI Suite 1".to_string()));
    assert!(!ev.iter().any(|e| e.starts_with("set_options_name")));
    assert!(!ev.contains(&"release PF Effect UI Suite 1".to_string()));
    assert_eq!(defs().len(), 14);
}

#[test]
fn the_name_call_error_is_ignored_and_the_suite_is_still_released() {
    let plan = sample();
    reset(Cfg {
        set_name_err: 699,
        ..Cfg::default()
    });
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    assert!(events().contains(&"release PF Effect UI Suite 1".to_string()));
}

#[test]
fn premiere_and_a_missing_basic_suite_skip_the_effect_ui_step() {
    let plan = Plan::parse(&SAMPLE.replace("Blob One", "Blob 1")).unwrap();
    reset(Cfg::default());
    let mut i = host(APPL_ID_PREMIERE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    assert!(!events().iter().any(|e| e.contains("Effect UI")));
    // no basic suite: only a plan without an arbitrary row can run, and the step is skipped
    let text = "table v2\n#@\tui_suite=Opt\ni:1\tG\tgroup\t?\t?\t?\t?\t?\t?\t?\n";
    let plan = Plan::parse(text).unwrap();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    i.pica_basic = std::ptr::null_mut();
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    assert!(events().iter().all(|e| !e.contains("Effect UI")));
    assert_eq!(defs().len(), 1);
}

#[test]
fn null_pointers_and_a_missing_add_param() {
    let plan = sample();
    let mut o = out();
    let mut i = host(APPL_ID_AE);
    unsafe {
        assert_eq!(
            effect_shell::dispatch(CMD_PARAMS_SETUP, &plan, std::ptr::null_mut(), &mut o, 0),
            ERR_BAD_CALLBACK_PARAM
        );
        assert_eq!(
            effect_shell::dispatch(CMD_PARAMS_SETUP, &plan, &mut i, std::ptr::null_mut(), 0),
            ERR_BAD_CALLBACK_PARAM
        );
        assert_eq!(
            effect_shell::entry_with(Ok(&plan), CMD_PARAMS_SETUP, std::ptr::null_mut(), &mut o, 0),
            ERR_BAD_CALLBACK_PARAM
        );
    }
    reset(Cfg::default());
    i.inter.add_param = None;
    assert_eq!(run_params(&plan, &mut i, &mut o), ERR_BAD_CALLBACK_PARAM);
    assert!(events().is_empty());
}

#[test]
fn the_entry_function_runs_a_v2_plan_end_to_end() {
    let plan = sample();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    let e = unsafe { effect_shell::entry_with(Ok(&plan), CMD_PARAMS_SETUP, &mut i, &mut o, 0) };
    assert_eq!(e, 0);
    assert_eq!(defs().len(), 14);
    assert_eq!(o.num_params, 15);
}

#[test]
fn the_id_arbitrary_field_uses_the_low_16_bits_and_a_row_without_id_leaves_it() {
    let text = "table v2\n#@shell\tfill=0x77\n?\tA\tarbitrary\t?\t?\t?\t?\t?\t?\td:\n";
    let plan = Plan::parse(text).unwrap();
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    let b = bytes_of(&defs()[0]);
    assert_eq!(b[56..58], [0x77, 0x77]);
    assert_eq!(b[58..60], [0x77, 0x77], "pad is not stated");
}

#[test]
fn the_first_of_several_deferred_errors_is_the_one_returned() {
    let text = "table v2\n\
        #@\tdefer_err=1\n\
        i:1\tA\tgroup\t?\t?\t?\t?\t?\t?\t?\n\
        #@\tdefer_err=1\n\
        i:2\tB\tgroup\t?\t?\t?\t?\t?\t?\t?\n\
        i:3\tC\tgroup\t?\t?\t?\t?\t?\t?\t?\n";
    let plan = Plan::parse(text).unwrap();
    reset(Cfg {
        add_fail: vec![(1, 611), (2, 622)],
        ..Cfg::default()
    });
    let mut i = host(APPL_ID_AE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 611);
    assert_eq!(defs().len(), 3);
    // an error on a row that is not deferred still stops at once
    reset(Cfg {
        add_fail: vec![(3, 633), (1, 611)],
        ..Cfg::default()
    });
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 633);
    assert_eq!(defs().len(), 3);
    reset(Cfg {
        add_fail: vec![(3, 633)],
        ..Cfg::default()
    });
    assert_eq!(run_params(&plan, &mut i, &mut o), 633);
}

#[test]
fn the_fill_byte_range_is_zero_to_255() {
    let row = "i:1\tG\tgroup\t?\t?\t?\t?\t?\t?\t?\n";
    let plan = Plan::parse(&format!("table v2\n#@shell\tfill=255\n{row}")).unwrap();
    assert_eq!(plan.options.fill, 255);
    let plan = Plan::parse(&format!("table v2\n#@shell\tfill=0\n{row}")).unwrap();
    assert_eq!(plan.options.fill, 0);
    let plan = Plan::parse(&format!(
        "table v2\n#@shell\tfill=0xff\treset=never\tregister_ui=0\n{row}"
    ))
    .unwrap();
    assert_eq!(plan.options.fill, 0xff);
    assert_eq!(plan.options.register_ui, Some(0));
    assert_eq!(plan.options.reset, effect_shell::plan::Reset::Never);
    assert_eq!(plan.version, 2);
}

#[test]
fn a_placeholder_row_makes_no_handle_call_and_a_version_one_table_is_version_one() {
    let text = "table v1\n#@shell\tplaceholder_name=Stub Row\n#@\ttype=arb\tae_only=1\tid=5\nA\tint\t?\t?\t?\t0x0\n";
    let plan = Plan::parse(text).unwrap();
    assert_eq!(plan.version, 1);
    reset(Cfg::default());
    let mut i = host(APPL_ID_PREMIERE);
    let mut o = out();
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    assert!(events().iter().all(|e| !e.contains("Handle")));
    let d = defs();
    assert_eq!(d[0].param_type, ParamType::Button as i32);
    assert_eq!(&d[0].name[..9], b"Stub Row\0");
    // the same plan in After Effects takes the handle suite
    reset(Cfg::default());
    let mut i = host(APPL_ID_AE);
    assert_eq!(run_params(&plan, &mut i, &mut o), 0);
    assert!(events().contains(&"acquire PF Handle Suite 2".to_string()));
}

#[test]
fn the_premiere_code_and_the_registration_record_layout() {
    assert_eq!(APPL_ID_PREMIERE, 0x5072_4d72);
    assert_eq!(APPL_ID_AE, 0x4658_5443);
    assert_eq!(std::mem::size_of::<CustomUiInfo>(), 44);
    assert_eq!(std::mem::offset_of!(CustomUiInfo, events), 4);
    assert_eq!(std::mem::offset_of!(CustomUiInfo, preview_ui_alignment), 40);
    let info = CustomUiInfo {
        events: 0x0102_0304,
        ..CustomUiInfo::default()
    };
    let b: [u8; 44] = unsafe { std::mem::transmute(info) };
    assert_eq!(b[4..8], [4, 3, 2, 1]);
    assert_eq!(b[0..4], [0; 4]);
    assert_eq!(std::mem::size_of::<SpBasicSuite>(), 56);
    assert_eq!(std::mem::size_of::<HandleSuite1>(), 48);
    assert_eq!(std::mem::size_of::<EffectUiSuite1>(), 8);
}
