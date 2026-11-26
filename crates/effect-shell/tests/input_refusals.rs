//! Input checks for the plan and description parsers: directive numbers must not wrap silently,
//! a doubled sign is not a number, and a property description line that holds only an id is an
//! error, not a panic.

use effect_shell::pipl::parse_description;
use effect_shell::plan::{parse_int, Plan};

fn table_with(directives: &str) -> String {
    format!("table v1\n{directives}A\tfloat\tf:1\ti:0\ti:10\t0x1\n")
}

#[test]
fn parse_int_refuses_a_second_sign() {
    assert_eq!(parse_int("-5"), Some(-5));
    assert_eq!(parse_int("--5"), None);
    assert_eq!(parse_int("-+5"), None);
    assert_eq!(parse_int("0x-5"), None);
    assert_eq!(parse_int("0x+5"), None);
    assert_eq!(parse_int("-0x-5"), None);
}

#[test]
fn global_numbers_that_do_not_fit_32_bits_are_refused() {
    for key in ["my_version", "out_flags", "out_flags2"] {
        let t = table_with(&format!("#@global\t{key}=0x100000000\n"));
        assert!(Plan::parse(&t).is_err(), "{key} above 32 bits");
        let t = table_with(&format!("#@global\t{key}=-1\n"));
        assert!(Plan::parse(&t).is_err(), "{key} negative");
        let t = table_with(&format!("#@global\t{key}=0xffffffff\n"));
        assert!(Plan::parse(&t).is_ok(), "{key} at the 32 bit limit");
    }
}

#[test]
fn shell_numbers_that_do_not_fit_are_refused() {
    for kv in ["register_ui=0x100000000", "placeholder_flags=4294967296"] {
        let t = table_with(&format!("#@shell\t{kv}\n"));
        assert!(Plan::parse(&t).is_err(), "{kv}");
    }
}

#[test]
fn row_numbers_that_do_not_fit_are_refused() {
    for kv in [
        "id=4294967297",
        "ui=0x100000000",
        "ui_w=65537",
        "ui_h=-32769",
        "sdkflags=0x100000000",
        "precision=65536",
        "display=40000",
        "fs_flags=3000000000",
    ] {
        let t = table_with(&format!("#@\t{kv}\n"));
        assert!(Plan::parse(&t).is_err(), "{kv}");
    }
    let t = table_with(&format!("#@\tid=7\tui_w={}\tui_h={}\n", i16::MAX, i16::MIN));
    let p = Plan::parse(&t).unwrap();
    assert_eq!(p.rows[0].id, Some(7));
    assert_eq!(p.rows[0].ui_width, Some(i16::MAX));
    assert_eq!(p.rows[0].ui_height, Some(i16::MIN));
}

#[test]
fn description_line_with_only_an_id_is_an_error() {
    assert!(parse_description("id=5").is_err());
    assert!(parse_description("kind abcd\nid=5\n").is_err());
}

#[test]
fn refusal_names_the_key_and_the_line() {
    let t = table_with("#@global\tout_flags=0x100000000\n");
    let e = Plan::parse(&t).unwrap_err().to_string();
    assert!(e.contains("out_flags"), "{e}");
    assert!(e.contains("out of range"), "{e}");
}

#[test]
fn values_at_the_limits_still_parse() {
    let t = table_with(
        "#@global\tmy_version=0\tout_flags=0\n#@shell\trefcon=0\tregister_ui=0xffffffff\n",
    );
    assert!(Plan::parse(&t).is_ok());
    let t = table_with(&format!("#@\tui_w={}\tui_h={}\n", i16::MIN, i16::MAX));
    assert!(Plan::parse(&t).is_ok());
    assert!(Plan::parse(&table_with(&format!(
        "#@\tui_w={}\n",
        i32::from(i16::MAX) + 1
    )))
    .is_err());
    assert!(Plan::parse(&table_with("#@\tui_h=-32769\n")).is_err());
}

#[test]
fn description_with_a_second_id_only_line_is_an_error() {
    assert!(parse_description("id=1\nid=2").is_err());
    assert!(parse_description("   id=5   ").is_err());
}

#[test]
fn a_reference_constant_may_use_the_whole_unsigned_range() {
    let top = u64::MAX;
    for (kv, want) in [
        (
            "refcon=0xfedcba9876543210".to_string(),
            0xfedc_ba98_7654_3210_u64,
        ),
        (format!("refcon={top}"), top),
        (format!("refcon=0x{top:x}"), top),
        ("refcon=0".to_string(), 0),
        ("refcon=-1".to_string(), u64::MAX),
        ("refcon=-0x10".to_string(), 0xffff_ffff_ffff_fff0_u64),
        (
            format!("refcon=-{}", i64::MIN.unsigned_abs()),
            i64::MIN as u64,
        ),
        (
            format!("refcon=-0x{:x}", i64::MIN.unsigned_abs()),
            i64::MIN as u64,
        ),
    ] {
        let t = table_with(&format!("#@shell\t{kv}\n"));
        let p = Plan::parse(&t).expect(&kv);
        // The parsed plan prints the stored word in its debug form.
        assert!(
            format!("{p:?}").contains(&format!("refcon: {want},")),
            "{kv}"
        );
    }
    for kv in [
        "refcon=0x10000000000000000",
        "refcon=+1",
        "refcon=--1",
        "refcon=-+1",
        "refcon=0x+1",
        "refcon=0x-1",
        "refcon=-0x-1",
        "refcon=-0x10000000000000000",
    ] {
        let t = table_with(&format!("#@shell\t{kv}\n"));
        assert!(Plan::parse(&t).is_err(), "{kv}");
    }
    let above = i64::MIN.unsigned_abs() + 1;
    for kv in [format!("refcon=-{above}"), format!("refcon=-0x{above:x}")] {
        let t = table_with(&format!("#@shell\t{kv}\n"));
        assert!(Plan::parse(&t).is_err(), "{kv}");
    }
}

#[test]
fn a_row_id_may_use_the_whole_signed_32_bit_range() {
    for (kv, want) in [
        ("id=40001".to_string(), 40001),
        (format!("id={}", i32::MAX), i32::MAX),
        ("id=-40001".to_string(), -40001),
    ] {
        let t = table_with(&format!("#@\t{kv}\n"));
        let p = Plan::parse(&t).expect(&kv);
        assert_eq!(p.rows[0].id, Some(want), "{kv}");
    }
    assert!(Plan::parse(&table_with(&format!(
        "#@\tid={}\n",
        i64::from(i32::MAX) + 1
    )))
    .is_err());
}
