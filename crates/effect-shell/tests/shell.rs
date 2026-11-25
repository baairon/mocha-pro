//! Tests drive the entry point and the dispatcher with a mock host written here (made up data).

mod common;

use effect_shell::layout::*;
use effect_shell::plan::Plan;
use std::cell::RefCell;
use std::ffi::{c_void, CStr};
use std::mem::{offset_of, size_of};
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

#[derive(Clone)]
struct Rec {
    effect_ref: usize,
    index: i32,
    def: ParamDef,
    text: String,
}

thread_local! {
    static CALLS: RefCell<Vec<Rec>> = const { RefCell::new(Vec::new()) };
    static PANIC_ON_ADD: RefCell<bool> = const { RefCell::new(false) };
    static FAIL_AT: RefCell<Option<(usize, i32)>> = const { RefCell::new(None) };
}

unsafe fn text_of(p: *const u8) -> String {
    if p.is_null() {
        "<null>".into()
    } else {
        CStr::from_ptr(p as *const _).to_string_lossy().into_owned()
    }
}

unsafe extern "C-unwind" fn mock_add(r: *mut c_void, index: i32, def: *mut ParamDef) -> PfErr {
    if PANIC_ON_ADD.with(|p| *p.borrow()) {
        panic!("mock host callback panics");
    }
    let d = *def;
    let text = match d.param_type {
        x if x == ParamType::Popup as i32 => text_of(d.u.pd.namesptr),
        x if x == ParamType::Checkbox as i32 => text_of(d.u.bd.nameptr),
        x if x == ParamType::Button as i32 => text_of(d.u.button_d.namesptr),
        _ => String::new(),
    };
    let n = CALLS.with(|c| {
        c.borrow_mut().push(Rec {
            effect_ref: r as usize,
            index,
            def: d,
            text,
        });
        c.borrow().len()
    });
    FAIL_AT.with(|f| match *f.borrow() {
        Some((at, code)) if at == n => code,
        _ => 0,
    })
}

fn zero_in() -> InData {
    unsafe { std::mem::zeroed() }
}
fn zero_out() -> OutData {
    unsafe { std::mem::zeroed() }
}

fn host_in(appl: u32) -> InData {
    let mut i = zero_in();
    i.inter.add_param = Some(mock_add);
    i.effect_ref = 0x1234 as *mut c_void;
    i.appl_id = appl;
    common::reset(common::Cfg::default());
    common::install_suites(&mut i);
    CALLS.with(|c| c.borrow_mut().clear());
    FAIL_AT.with(|f| *f.borrow_mut() = None);
    PANIC_ON_ADD.with(|p| *p.borrow_mut() = false);
    i
}

fn calls() -> Vec<Rec> {
    CALLS.with(|c| c.borrow().clone())
}

fn name_of(d: &ParamDef) -> String {
    let end = d.name.iter().position(|&b| b == 0).unwrap_or(32);
    String::from_utf8_lossy(&d.name[..end]).into_owned()
}

const T: &str = "\t";

fn table(rows: &[&str]) -> Plan {
    let mut s = String::from("table v1\n");
    for r in rows {
        s.push_str(&r.replace("<T>", T));
        s.push('\n');
    }
    Plan::parse(&s).unwrap()
}

unsafe fn run(cmd: i32, plan: &Plan, i: &mut InData, o: &mut OutData, flags: u64) -> PfErr {
    effect_shell::dispatch(cmd, plan, i, o, flags)
}

#[test]
fn layout_sizes_and_offsets_match_the_layout_tables() {
    assert_eq!(size_of::<InteractCallbacks>(), 176);
    assert_eq!(size_of::<InData>(), 408);
    assert_eq!(size_of::<OutData>(), 408);
    assert_eq!(size_of::<ParamDef>(), 176);
    assert_eq!(size_of::<ParamUnion>(), 120);
    assert_eq!(offset_of!(InteractCallbacks, add_param), 16);
    assert_eq!(offset_of!(InteractCallbacks, register_ui), 40);
    assert_eq!(offset_of!(InData, utils), 176);
    assert_eq!(offset_of!(InData, effect_ref), 184);
    assert_eq!(offset_of!(InData, appl_id), 204);
    assert_eq!(offset_of!(InData, time_scale), 240);
    assert_eq!(offset_of!(InData, in_flags), 308);
    assert_eq!(offset_of!(InData, global_data), 312);
    assert_eq!(offset_of!(InData, src_snd), 352);
    assert_eq!(offset_of!(InData, pica_basic), 384);
    assert_eq!(offset_of!(InData, shutter_phase), 400);
    assert_eq!(offset_of!(OutData, my_version), 0);
    assert_eq!(offset_of!(OutData, global_data), 40);
    assert_eq!(offset_of!(OutData, num_params), 48);
    assert_eq!(offset_of!(OutData, out_flags), 96);
    assert_eq!(offset_of!(OutData, return_msg), 100);
    assert_eq!(offset_of!(OutData, dest_snd), 368);
    assert_eq!(offset_of!(OutData, out_flags2), 400);
    assert_eq!(offset_of!(ParamDef, ui_flags), 4);
    assert_eq!(offset_of!(ParamDef, param_type), 12);
    assert_eq!(offset_of!(ParamDef, name), 16);
    assert_eq!(offset_of!(ParamDef, flags), 48);
    assert_eq!(offset_of!(ParamDef, u), 56);
    assert_eq!(size_of::<FloatSliderDef>(), 88);
    assert_eq!(offset_of!(FloatSliderDef, valid_min), 48);
    assert_eq!(offset_of!(FloatSliderDef, dephault), 64);
    assert_eq!(offset_of!(FloatSliderDef, fs_flags), 72);
    assert_eq!(offset_of!(FloatSliderDef, curve_tolerance), 76);
    assert_eq!(size_of::<SliderDef>(), 88);
    assert_eq!(size_of::<FixedSliderDef>(), 92);
    assert_eq!(offset_of!(PopupDef, namesptr), 8);
    assert_eq!(offset_of!(CheckBoxDef, nameptr), 8);
    assert_eq!(offset_of!(PointDef, restrict_bounds), 11);
    assert_eq!(offset_of!(PointDef, y_dephault), 16);
    assert_eq!(size_of::<ArbitraryDef>(), 32);
    assert_eq!(offset_of!(ArbitraryDef, dephault), 8);
}

#[test]
fn command_and_type_numbers() {
    assert_eq!((CMD_ABOUT, CMD_GLOBAL_SETUP, CMD_PARAMS_SETUP), (0, 1, 4));
    assert_eq!(ParamType::Layer as i32, 0);
    assert_eq!(ParamType::Checkbox as i32, 4);
    assert_eq!(ParamType::Popup as i32, 7);
    assert_eq!(ParamType::FloatSlider as i32, 10);
    assert_eq!(ParamType::GroupStart as i32, 13);
    assert_eq!(ParamType::GroupEnd as i32, 14);
    assert_eq!(ParamType::Button as i32, 15);
    assert_eq!(APPL_ID_AE, 0x46585443);
    assert_eq!(
        (
            PARAM_FLAG_SUPERVISE,
            PARAM_FLAG_START_COLLAPSED,
            PARAM_FLAG_CANNOT_TIME_VARY
        ),
        (0x40, 0x20, 0x2)
    );
}

#[test]
fn global_setup_sets_the_three_fields_from_the_plan() {
    let plan = Plan::parse(
        &"table v1\n#@global<T>my_version=0x00030002<T>out_flags=0xAB<T>out_flags2=77\nA<T>int<T>?<T>?<T>?<T>?\n".replace("<T>", T),
    )
    .unwrap();
    let mut i = host_in(0);
    let mut o = zero_out();
    o.num_params = 99;
    assert_eq!(
        unsafe { run(CMD_GLOBAL_SETUP, &plan, &mut i, &mut o, 0) },
        0
    );
    assert_eq!(o.my_version, 0x0003_0002);
    assert_eq!(o.out_flags, 0xAB);
    assert_eq!(o.out_flags2, 77);
    assert_eq!(o.num_params, 99, "GLOBAL_SETUP never writes num_params");
    assert!(calls().is_empty(), "GLOBAL_SETUP makes no host call");
}

#[test]
fn about_writes_a_terminated_message_and_truncates() {
    let long = "x".repeat(400);
    let plan = Plan::parse(&format!("table v1\n#@global{T}about={long}\n")).unwrap();
    let mut i = host_in(0);
    let mut o = zero_out();
    assert_eq!(unsafe { run(CMD_ABOUT, &plan, &mut i, &mut o, 0) }, 0);
    assert_eq!(o.return_msg[254], b'x');
    assert_eq!(o.return_msg[255], 0);
    assert_eq!(o.my_version, 0);
    let plan = Plan::parse(&format!("table v1\n#@global{T}about=hi\n")).unwrap();
    assert_eq!(unsafe { run(CMD_ABOUT, &plan, &mut i, &mut o, 0) }, 0);
    assert_eq!(&o.return_msg[..3], b"hi\0");
    assert!(
        o.return_msg[3..].iter().all(|&b| b == 0),
        "old text is cleared"
    );
}

#[test]
fn unknown_command_is_ignored() {
    let plan = table(&["A<T>int<T>?<T>?<T>?<T>?"]);
    let mut i = host_in(0);
    let mut o = zero_out();
    assert_eq!(unsafe { run(77, &plan, &mut i, &mut o, 0) }, 0);
    assert_eq!(unsafe { run(2, &plan, &mut i, &mut o, 0) }, 0);
    assert!(calls().is_empty());
    assert_eq!(o.num_params, 0);
}

#[test]
fn null_records_and_missing_callback_are_refused() {
    let plan = table(&["A<T>int<T>?<T>?<T>?<T>?"]);
    let mut i = host_in(0);
    let mut o = zero_out();
    unsafe {
        assert_eq!(
            effect_shell::dispatch(1, &plan, std::ptr::null_mut(), &mut o, 0),
            516
        );
        assert_eq!(
            effect_shell::dispatch(1, &plan, &mut i, std::ptr::null_mut(), 0),
            516
        );
    }
    i.inter.add_param = None;
    assert_eq!(
        unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) },
        516
    );
    assert_eq!(o.num_params, 0);
}

#[test]
fn params_setup_adds_in_table_order_with_index_minus_one() {
    let plan = table(&[
        "One<T>int<T>i:1<T>?<T>?<T>?",
        "Two<T>float<T>f:2<T>?<T>?<T>?",
        "Three<T>checkbox<T>i:0<T>?<T>?<T>?",
    ]);
    let mut i = host_in(0);
    let mut o = zero_out();
    assert_eq!(
        unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) },
        0
    );
    let c = calls();
    assert_eq!(c.len(), 3);
    let names: Vec<String> = c.iter().map(|r| name_of(&r.def)).collect();
    assert_eq!(names, ["One", "Two", "Three"]);
    assert!(c.iter().all(|r| r.index == -1));
    assert!(c.iter().all(|r| r.effect_ref == 0x1234));
    assert_eq!(c[0].def.param_type, ParamType::Slider as i32);
    assert_eq!(c[1].def.param_type, ParamType::FloatSlider as i32);
    assert_eq!(c[2].def.param_type, ParamType::Checkbox as i32);
    assert_eq!(o.num_params, 4, "three added plus the input layer");
}

#[test]
fn a_host_error_stops_the_setup_and_is_returned_unchanged() {
    let plan = table(&[
        "One<T>int<T>?<T>?<T>?<T>?",
        "Two<T>int<T>?<T>?<T>?<T>?",
        "Three<T>int<T>?<T>?<T>?<T>?",
    ]);
    let mut i = host_in(0);
    FAIL_AT.with(|f| *f.borrow_mut() = Some((2, 1234)));
    let mut o = zero_out();
    assert_eq!(
        unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) },
        1234
    );
    assert_eq!(calls().len(), 2, "nothing is retried or continued");
    assert_eq!(o.num_params, 0, "num_params only on success");
}

#[test]
fn empty_table_registers_nothing_and_counts_the_input_layer() {
    let plan = table(&[]);
    let mut i = host_in(0);
    let mut o = zero_out();
    assert_eq!(
        unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) },
        0
    );
    assert!(calls().is_empty());
    assert_eq!(o.num_params, 1);
}

#[test]
fn every_kind_fills_its_union_member() {
    let rows = [
        "S<T>int<T>i:7<T>i:-5<T>i:50<T>0x0",
        "F<T>float<T>f:2.5<T>i:-10<T>i:90<T>0x0",
        "C<T>checkbox<T>i:1<T>?<T>?<T>0x0",
        "L<T>layer<T>i:-1<T>?<T>?<T>0x0",
        "A<T>angle<T>f:90<T>i:-360<T>i:360<T>0x0",
        "K<T>color<T>i:4278255360<T>?<T>?<T>0x0",
        "G<T>group<T>?<T>?<T>?<T>0x0",
    ];
    let plan = table(&rows);
    let mut i = host_in(0);
    let mut o = zero_out();
    assert_eq!(
        unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) },
        0
    );
    let c = calls();
    unsafe {
        let s = c[0].def.u.sd;
        assert_eq!(
            (s.value, s.dephault, s.valid_min, s.valid_max),
            (7, 7, -5, 50)
        );
        assert_eq!((s.slider_min, s.slider_max), (-5, 50));
        let f = c[1].def.u.fs_d;
        assert_eq!((f.value, f.dephault), (2.5, 2.5));
        assert_eq!((f.valid_min, f.valid_max), (-10.0, 90.0));
        assert_eq!((f.slider_min, f.slider_max), (-10.0, 90.0));
        let b = c[2].def.u.bd;
        assert_eq!((b.value, b.dephault), (1, 1));
        assert_eq!(c[3].def.u.ld.dephault, -1);
        let a = c[4].def.u.ad;
        assert_eq!((a.value, a.dephault), (90 * 65536, 90 * 65536));
        assert_eq!((a.valid_min, a.valid_max), (-360 * 65536, 360 * 65536));
        let k = c[5].def.u.cd;
        assert_eq!(k.value, [0xFF, 0x00, 0xFF, 0x00]);
        assert_eq!(k.dephault, [0xFF, 0x00, 0xFF, 0x00]);
    }
    assert_eq!(c[6].def.param_type, ParamType::GroupStart as i32);
}

#[test]
fn unknown_range_cells_give_wide_defaults_not_zero() {
    let plan = table(&["S<T>int<T>?<T>?<T>?<T>?", "F<T>float<T>?<T>?<T>?<T>?"]);
    let mut i = host_in(0);
    let mut o = zero_out();
    unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) };
    let c = calls();
    unsafe {
        assert_eq!(c[0].def.u.sd.valid_min, i32::MIN);
        assert_eq!(c[0].def.u.sd.valid_max, i32::MAX);
        assert_eq!(c[1].def.u.fs_d.valid_min, f32::MIN);
        assert_eq!(c[1].def.u.fs_d.valid_max, f32::MAX);
        assert_eq!(c[1].def.u.fs_d.dephault, 0.0);
    }
}

#[test]
fn directives_set_ids_flags_sliders_points_buttons_and_floats() {
    let plan = table(&[
        "#@<T>id=0x7a1<T>ui=0x34",
        "A<T>checkbox<T>i:0<T>?<T>?<T>0x1",
        "#@<T>id=7<T>label=Go Now<T>sdkflags=0x44<T>ui=0x36",
        "B<T>button<T>?<T>?<T>?<T>0x0",
        "#@<T>precision=2<T>display=1<T>fs_flags=1<T>tol=0.05<T>slider=0,10",
        "F<T>float<T>f:1<T>i:0<T>i:350<T>0x8",
        "#@<T>xy=25,75<T>restrict=1",
        "P<T>point<T>?<T>?<T>?<T>0x0",
        "#@<T>type=fixslider<T>precision=3<T>display=2<T>slider=1,2",
        "X<T>float<T>f:1.5<T>i:0<T>i:5<T>0x0",
        "#@<T>type=arb<T>id=0x7a2",
        "R<T>int<T>?<T>?<T>?<T>0x0",
        "#@<T>type=no_data<T>ui_w=90<T>ui_h=23<T>ui=8",
        "N<T>int<T>?<T>?<T>?<T>0x0",
        "#@<T>type=group_end<T>id=9",
        "E<T>group<T>?<T>?<T>?<T>0x0",
    ]);
    let mut i = host_in(0);
    let mut o = zero_out();
    assert_eq!(
        unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) },
        0
    );
    let c = calls();
    assert_eq!(c[0].def.uu_id, 0x7a1);
    assert_eq!(c[0].def.ui_flags, 0x34);
    assert_eq!(
        c[0].def.flags, PARAM_FLAG_SUPERVISE,
        "SUPERVISED maps to the supervise flag word"
    );
    assert_eq!(c[1].def.uu_id, 7);
    assert_eq!(c[1].def.flags, 0x44, "sdkflags replaces the mapped word");
    assert_eq!(c[1].def.ui_flags, 0x36);
    assert_eq!(c[1].text, "Go Now");
    assert_eq!(
        c[2].def.flags, PARAM_FLAG_CANNOT_TIME_VARY,
        "NOT_ANIMATABLE maps to the cannot time vary flag word"
    );
    unsafe {
        let f = c[2].def.u.fs_d;
        assert_eq!((f.precision, f.display_flags, f.fs_flags), (2, 1, 1));
        assert_eq!(f.curve_tolerance, 0.05f32);
        assert_eq!((f.slider_min, f.slider_max), (0.0, 10.0));
        assert_eq!((f.valid_min, f.valid_max), (0.0, 350.0));
        let p = c[3].def.u.td;
        assert_eq!((p.x_value, p.y_value), (25 * 65536, 75 * 65536));
        assert_eq!((p.x_dephault, p.y_dephault), (25 * 65536, 75 * 65536));
        assert_eq!(p.restrict_bounds, 1);
        let x = c[4].def.u.fd;
        assert_eq!(c[4].def.param_type, ParamType::FixSlider as i32);
        assert_eq!((x.value, x.dephault), (98304, 98304));
        assert_eq!((x.precision, x.display_flags), (3, 2));
        assert_eq!((x.slider_min, x.slider_max), (65536, 131072));
        assert_eq!(c[5].def.param_type, ParamType::ArbitraryData as i32);
        assert_eq!(c[5].def.u.arb_d.id, 0x7a2);
    }
    assert_eq!(
        (c[6].def.ui_width, c[6].def.ui_height, c[6].def.ui_flags),
        (90, 23, 8)
    );
    assert_eq!(c[6].def.param_type, ParamType::NoData as i32);
    assert_eq!(c[7].def.param_type, ParamType::GroupEnd as i32);
    assert_eq!(c[7].def.uu_id, 9);
    assert_eq!(o.num_params, 9);
}

#[test]
fn engine_flags_select_popup_lists_and_defaults() {
    let plan = table(&[
        "#@<T>choices=Aa|Bb|Cc|Dd<T>choice_bits=-,2,-,5",
        "M<T>popup<T>i:4<T>?<T>?<T>0x0",
    ]);
    let cases: [(u64, &str, i16, i16); 4] = [
        (0, "Aa|Cc", 2, 2),
        (1 << 2, "Aa|Bb|Cc", 3, 3),
        (1 << 5, "Aa|Cc|Dd", 3, 3),
        ((1 << 2) | (1 << 5), "Aa|Bb|Cc|Dd", 4, 4),
    ];
    for (flags, names, n, def) in cases {
        let mut i = host_in(0);
        let mut o = zero_out();
        assert_eq!(
            unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, flags) },
            0
        );
        let c = calls();
        let p = unsafe { c[0].def.u.pd };
        assert_eq!(c[0].text, names, "flags {flags:#x}");
        assert_eq!((p.num_choices, p.dephault, p.value as i16), (n, def, def));
    }
}

#[test]
fn popup_default_is_clamped_and_empty_list_gives_zero() {
    let plan = table(&[
        "#@<T>choices=A|B<T>sdkflags=0x32",
        "Hi<T>popup<T>i:9<T>?<T>?<T>?",
        "#@<T>choices=A|B",
        "Lo<T>popup<T>i:-3<T>?<T>?<T>?",
        "#@<T>choices=A|B<T>choice_bits=40,41",
        "None<T>popup<T>i:2<T>?<T>?<T>?",
    ]);
    let mut i = host_in(0);
    let mut o = zero_out();
    unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) };
    let c = calls();
    unsafe {
        assert_eq!(c[0].def.u.pd.dephault, 2);
        assert_eq!(c[0].def.flags, 0x32);
        assert_eq!(c[1].def.u.pd.dephault, 1);
        assert_eq!(c[2].def.u.pd.num_choices, 0);
        assert_eq!(c[2].def.u.pd.dephault, 0);
        assert_eq!(c[2].def.u.pd.value, 0);
    }
    assert_eq!(c[2].text, "");
}

#[test]
fn visibility_bit_gates_the_invisible_ui_flag() {
    let plan = table(&[
        "#@<T>vis_bit=8<T>ui=0x50",
        "Gated<T>checkbox<T>i:0<T>?<T>?<T>0x0",
        "Plain<T>checkbox<T>i:0<T>?<T>?<T>0x0",
        "#@<T>vis_bit=63",
        "High<T>checkbox<T>i:0<T>?<T>?<T>0x0",
    ]);
    for (flags, g, p, h) in [
        (0u64, 0x250u32, 0u32, 0x200u32),
        (1 << 8, 0x50, 0, 0x200),
        (1 << 7, 0x250, 0, 0x200),
        (1 << 9, 0x250, 0, 0x200),
        (1 << 63, 0x250, 0, 0),
        (u64::MAX, 0x50, 0, 0),
    ] {
        let mut i = host_in(0);
        let mut o = zero_out();
        unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, flags) };
        let c = calls();
        assert_eq!(c[0].def.ui_flags, g, "flags {flags:#x}");
        assert_eq!(c[1].def.ui_flags, p, "flags {flags:#x}");
        assert_eq!(c[2].def.ui_flags, h, "flags {flags:#x}");
    }
}

#[test]
fn host_only_rows_become_hidden_placeholders_elsewhere_and_keep_the_count() {
    let plan = table(&[
        "#@shell<T>placeholder_name=Stub Row<T>placeholder_flags=0x6c",
        "#@<T>ae_only=1<T>id=5<T>label=Real<T>ui=0x36<T>sdkflags=0x44",
        "Real Button<T>button<T>?<T>?<T>?<T>?",
        "After<T>int<T>?<T>?<T>?<T>?",
    ]);
    let mut i = host_in(APPL_ID_AE);
    let mut o = zero_out();
    unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) };
    let c = calls();
    assert_eq!(name_of(&c[0].def), "Real Button");
    assert_eq!(
        (c[0].def.flags, c[0].def.ui_flags, c[0].text.as_str()),
        (0x44, 0x36, "Real")
    );
    assert_eq!(o.num_params, 3);
    for other in [0u32, u32::from_be_bytes(*b"PrMr")] {
        let mut i = host_in(other);
        let mut o = zero_out();
        unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) };
        let c = calls();
        assert_eq!(c.len(), 2);
        assert_eq!(name_of(&c[0].def), "Stub Row");
        assert_eq!(c[0].def.param_type, ParamType::Button as i32);
        assert_eq!((c[0].def.flags, c[0].def.ui_flags), (0x6c, 0x200));
        assert_eq!(c[0].def.uu_id, 5, "the id is kept");
        assert_eq!(c[0].text, "");
        assert_eq!(o.num_params, 3);
    }
}

#[test]
fn names_are_cut_to_31_bytes_and_terminated() {
    let long = "n".repeat(60);
    let plan = table(&[&format!("{long}<T>int<T>?<T>?<T>?<T>?")]);
    let mut i = host_in(0);
    let mut o = zero_out();
    unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) };
    let c = calls();
    assert_eq!(name_of(&c[0].def).len(), 31);
    assert_eq!(c[0].def.name[31], 0);
}

#[test]
fn flag_mapping_and_hidden_flag() {
    let plan = table(&[
        "A<T>int<T>?<T>?<T>?<T>0x0",
        "B<T>int<T>?<T>?<T>?<T>0x1",
        "C<T>int<T>?<T>?<T>?<T>0x2",
        "D<T>int<T>?<T>?<T>?<T>0x4",
        "E<T>int<T>?<T>?<T>?<T>0x8",
        "F<T>int<T>?<T>?<T>?<T>0x10",
        "G<T>int<T>?<T>?<T>?<T>?",
        "H<T>int<T>?<T>?<T>?<T>0x1f",
    ]);
    let mut i = host_in(0);
    let mut o = zero_out();
    unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) };
    let got: Vec<(u32, u32)> = calls()
        .iter()
        .map(|r| (r.def.flags, r.def.ui_flags))
        .collect();
    assert_eq!(
        got,
        [
            (0, 0),
            (PARAM_FLAG_SUPERVISE, 0),
            (PARAM_FLAG_START_COLLAPSED, 0),
            (0, 0x200),
            (PARAM_FLAG_CANNOT_TIME_VARY, 0),
            (0, 0),
            (0, 0),
            (
                PARAM_FLAG_SUPERVISE | PARAM_FLAG_START_COLLAPSED | PARAM_FLAG_CANNOT_TIME_VARY,
                0x200
            )
        ]
    );
}

#[test]
fn plan_parse_errors() {
    let bad = |s: &str| Plan::parse(&s.replace("<T>", T)).unwrap_err().message;
    assert!(bad("table v1\n#@<T>nope=1\nA<T>int<T>?<T>?<T>?<T>?\n").contains("unknown row key"));
    assert!(bad("table v1\n#@global<T>nope=1\n").contains("unknown global key"));
    assert!(bad("table v1\n#@<T>id=1\n").contains("no row after"));
    assert!(bad("table v1\n#@<T>junk\nA<T>int<T>?<T>?<T>?<T>?\n").contains("without '='"));
    assert!(bad("table v1\n#@<T>id=zz\nA<T>int<T>?<T>?<T>?<T>?\n").contains("bad number"));
    assert!(bad("table v1\n#@<T>type=wat\nA<T>int<T>?<T>?<T>?<T>?\n").contains("unknown type"));
    assert!(bad("table v1\n#@<T>vis_bit=64\nA<T>int<T>?<T>?<T>?<T>?\n").contains("out of range"));
    assert!(bad("table v1\n#@<T>vis_bit=-1\nA<T>int<T>?<T>?<T>?<T>?\n").contains("out of range"));
    assert!(
        bad("table v1\n#@<T>choices=a|b<T>choice_bits=-\nA<T>popup<T>?<T>?<T>?<T>?\n")
            .contains("choice_bits has 1 entries for 2")
    );
    assert!(
        bad("table v1\n#@<T>choices=a<T>choice_bits=64\nA<T>popup<T>?<T>?<T>?<T>?\n")
            .contains("out of range")
    );
    assert!(bad("table v1\n#@<T>slider=1\nA<T>int<T>?<T>?<T>?<T>?\n").contains("slider"));
    assert!(bad("table v1\n#@<T>xy=1;2\nA<T>int<T>?<T>?<T>?<T>?\n").contains("xy"));
    assert!(bad("table v1\n#@<T>tol=abc\nA<T>int<T>?<T>?<T>?<T>?\n").contains("tolerance"));
    assert!(bad("A<T>int<T>?<T>?<T>?<T>?\n").contains("table v1"));
    let e = Plan::parse("table v1\nA\tint\n").unwrap_err();
    assert_eq!(e.line, 2);
    assert!(e.to_string().starts_with("line 2:"));
}

#[test]
fn parse_int_forms() {
    use effect_shell::plan::parse_int;
    assert_eq!(parse_int("12"), Some(12));
    assert_eq!(parse_int("-12"), Some(-12));
    assert_eq!(parse_int("0x3F"), Some(63));
    assert_eq!(parse_int("0x3f"), Some(63));
    assert_eq!(parse_int("-0x10"), Some(-16));
    assert_eq!(parse_int(" 5 "), Some(5));
    assert_eq!(parse_int("x"), None);
}

#[test]
fn directive_lines_do_not_disturb_the_effect_params_reader() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/sample/sample_table.txt"
    ))
    .unwrap();
    let t = effect_params::Table::parse(&text).unwrap();
    let p = Plan::parse(&text).unwrap();
    assert_eq!(t.params.len(), p.rows.len());
    for (a, b) in t.params.iter().zip(&p.rows) {
        assert_eq!(a.name, b.name);
    }
}

#[test]
fn entry_point_runs_the_built_in_sample_table() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let plan = effect_shell::built_in_plan().expect("sample table parses");
    assert_eq!(plan.rows.len(), 16);
    let mut o = zero_out();
    let mut i = host_in(APPL_ID_AE);
    unsafe {
        assert_eq!(
            effect_shell::EffectMain(
                1,
                &mut i,
                &mut o,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            ),
            0
        );
    }
    assert_eq!(o.my_version, 0x0002_0001);
    assert_eq!(o.out_flags, 0x1234);
    assert_eq!(o.out_flags2, 0x400);
    unsafe {
        assert_eq!(
            effect_shell::EffectMain(
                0,
                &mut i,
                &mut o,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            ),
            0
        );
    }
    assert_eq!(&o.return_msg[..6], b"Sample");

    effect_shell::set_engine_flags(0);
    assert_eq!(effect_shell::engine_flags(), 0);
    unsafe {
        assert_eq!(
            effect_shell::EffectMain(
                4,
                &mut i,
                &mut o,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            ),
            0
        );
    }
    let c0 = calls();
    assert_eq!(c0.len(), 16);
    assert_eq!(o.num_params, 17);
    let ids: Vec<i32> = c0.iter().map(|r| r.def.uu_id).collect();
    assert_eq!(ids, (101..=116).collect::<Vec<i32>>());
    // Quality popup (bit 1 gates the row, bits 2 and 3 gate two choices).
    assert_eq!(c0[4].text, "Low|Mid");
    assert_eq!(c0[4].def.ui_flags, 0x200);
    // Gated Box needs bit 4.
    assert_eq!(c0[15].def.ui_flags, 0x200);

    effect_shell::set_engine_flags((1 << 1) | (1 << 3) | (1 << 4));
    let mut i = host_in(APPL_ID_AE);
    let mut o = zero_out();
    unsafe {
        assert_eq!(
            effect_shell::EffectMain(
                4,
                &mut i,
                &mut o,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            ),
            0
        );
    }
    let c1 = calls();
    assert_eq!(c1[4].text, "Low|Mid|Extra");
    assert_eq!(c1[4].def.ui_flags, 0);
    assert_eq!(c1[15].def.ui_flags, 0);
    assert_eq!(unsafe { c1[4].def.u.pd.dephault }, 3);
    // The host only row in a non AE host.
    let mut i = host_in(0);
    let mut o = zero_out();
    unsafe {
        effect_shell::EffectMain(
            4,
            &mut i,
            &mut o,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
    let c2 = calls();
    assert_eq!(name_of(&c2[14].def), "Stub Row");
    assert_eq!(c2.len(), 16);
    effect_shell::set_engine_flags(0);
    // A null record is refused through the exported entry too.
    unsafe {
        assert_eq!(
            effect_shell::EffectMain(
                4,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            ),
            516
        );
    }
}

#[test]
fn sample_table_kinds_cover_the_row_types() {
    let plan = effect_shell::built_in_plan().unwrap();
    use std::collections::HashSet;
    let kinds: HashSet<i32> = plan.rows.iter().map(|r| r.ty as i32).collect();
    for t in [
        ParamType::Slider,
        ParamType::FixSlider,
        ParamType::Angle,
        ParamType::Checkbox,
        ParamType::Color,
        ParamType::Point,
        ParamType::Popup,
        ParamType::NoData,
        ParamType::FloatSlider,
        ParamType::ArbitraryData,
        ParamType::GroupStart,
        ParamType::GroupEnd,
        ParamType::Button,
        ParamType::Layer,
    ] {
        assert!(kinds.contains(&(t as i32)), "{t:?}");
    }
}

#[test]
fn sdkflags_directive_replaces_not_merges() {
    let plan = table(&["#@<T>sdkflags=0x32", "A<T>int<T>?<T>?<T>?<T>0x1"]);
    let mut i = host_in(0);
    let mut o = zero_out();
    unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) };
    assert_eq!(calls()[0].def.flags, 0x32);
}

#[test]
fn a_panic_in_the_host_callback_is_caught_and_reported_as_damaged_struct() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut i = host_in(0);
    let mut o = zero_out();
    PANIC_ON_ADD.with(|p| *p.borrow_mut() = true);
    let err = unsafe {
        effect_shell::EffectMain(
            4,
            &mut i,
            &mut o,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    PANIC_ON_ADD.with(|p| *p.borrow_mut() = false);
    assert_eq!(err, ERR_INTERNAL_STRUCT_DAMAGED);
    assert_eq!(err, 512);
    assert_eq!(o.num_params, 0, "the failed setup registers no count");
}

#[test]
fn a_plan_that_failed_to_parse_gives_the_damaged_struct_code() {
    let bad = Plan::parse("not a table\n");
    assert!(bad.is_err());
    let mut i = host_in(0);
    let mut o = zero_out();
    let err = unsafe { effect_shell::entry_with(bad.as_ref(), 4, &mut i, &mut o, 0) };
    assert_eq!(err, 512);
    assert!(calls().is_empty());
    let good = table(&["A<T>int<T>?<T>?<T>?<T>0x0"]);
    let err = unsafe { effect_shell::entry_with(Ok(&good), 4, &mut i, &mut o, 0) };
    assert_eq!(err, 0);
    assert_eq!(calls().len(), 1);
}

#[test]
fn native_group_end_arbitrary_and_nodata_kinds_map_without_a_type_override() {
    let plan = table(&[
        "G<T>group<T>?<T>?<T>?<T>0x0",
        "G End<T>group_end<T>?<T>?<T>?<T>0x0",
        "Blob<T>arbitrary<T>?<T>?<T>?<T>0x0",
        "Area<T>nodata<T>?<T>?<T>?<T>0x0",
    ]);
    let mut i = host_in(0);
    let mut o = zero_out();
    assert_eq!(
        unsafe { run(CMD_PARAMS_SETUP, &plan, &mut i, &mut o, 0) },
        0
    );
    let c = calls();
    assert_eq!(c.len(), 4);
    assert_eq!(c[0].def.param_type, ParamType::GroupStart as i32);
    assert_eq!(c[1].def.param_type, ParamType::GroupEnd as i32);
    assert_eq!(c[2].def.param_type, ParamType::ArbitraryData as i32);
    assert_eq!(c[3].def.param_type, ParamType::NoData as i32);
}
