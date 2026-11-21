//! A sign or any other non digit byte in a `b:` cell or in the digits of a flags cell is
//! refused with the text of a bad digit, and the narrow cases the edge tests leave open:
//! special float values, verdict names as written, the whole-cell rules of the format.

use effect_params::*;

fn row(cells: [&str; 6]) -> String {
    format!("table v1\n{}\n", cells.join("\t"))
}

fn err(text: &str) -> (usize, String) {
    let e = Table::parse(text).unwrap_err();
    (e.line, e.message)
}

fn with(name: &str, d: Cell) -> ParamDef {
    ParamDef {
        name: name.into(),
        kind: ParamKind::Float,
        default: d,
        min: Cell::Unknown,
        max: Cell::Unknown,
        flags: None,
    }
}

fn plain(names: &[&str]) -> Table {
    Table {
        params: names
            .iter()
            .map(|n| ParamDef::new(n, ParamKind::Int))
            .collect(),
    }
}

fn bits_msg(cell: &str) -> (usize, String) {
    (
        2,
        format!("bad bits {cell:?}: invalid digit found in string"),
    )
}

const BAD_FLAGS: &str = "bad flags: invalid digit found in string";

#[test]
fn a_plus_sign_in_a_bit_cell_is_refused() {
    let cell = "b:+00000000000000a";
    assert_eq!(
        err(&row(["A", "float", cell, "?", "?", "?"])),
        bits_msg(cell)
    );
    let cell = "b:+000000000000000";
    assert_eq!(
        err(&row(["A", "float", "?", cell, "?", "?"])),
        bits_msg(cell)
    );
    let cell = "b:3ff000000000000+";
    assert_eq!(
        err(&row(["A", "float", "?", "?", cell, "?"])),
        bits_msg(cell)
    );
}

#[test]
fn a_plus_sign_in_a_flags_cell_is_refused() {
    assert_eq!(
        err(&row(["A", "int", "?", "?", "?", "0x+1"])),
        (2, BAD_FLAGS.to_string())
    );
    assert_eq!(
        err(&row(["A", "int", "?", "?", "?", "0x1+"])),
        (2, BAD_FLAGS.to_string())
    );
    assert_eq!(
        err(&row(["A", "int", "?", "?", "?", "0x+"])),
        (2, BAD_FLAGS.to_string())
    );
}

#[test]
fn doubled_signs_are_refused_with_the_same_text() {
    let cell = "b:++00000000000000";
    assert_eq!(
        err(&row(["A", "float", cell, "?", "?", "?"])),
        bits_msg(cell)
    );
    let cell = "b:+-0000000000000a";
    assert_eq!(
        err(&row(["A", "float", cell, "?", "?", "?"])),
        bits_msg(cell)
    );
    for f in ["0x++1", "0x-1", "0x+-1", "0x--1", "0x-"] {
        assert_eq!(
            err(&row(["A", "int", "?", "?", "?", f])),
            (2, BAD_FLAGS.to_string()),
            "{f:?}"
        );
    }
}

#[test]
fn a_minus_sign_in_a_bit_cell_is_refused() {
    for cell in [
        "b:-00000000000000a",
        "b:-000000000000000",
        "b:00000000000000-0",
    ] {
        assert_eq!(
            err(&row(["A", "float", cell, "?", "?", "?"])),
            bits_msg(cell)
        );
    }
}

#[test]
fn an_inner_sign_a_space_or_a_letter_past_f_is_refused() {
    for cell in [
        "b:3ff0000+00000000",
        "b:3ff 000000000000",
        "b:3ff000000000000 ",
        "b: 3ff000000000000",
        "b:3ff00000000000g0",
        "b:3ff00000000x0000",
    ] {
        assert_eq!(
            err(&row(["A", "float", cell, "?", "?", "?"])),
            bits_msg(cell)
        );
    }
    for f in ["0x1+1", "0x1 1", "0x g", "0xg", "0x1\u{0}", "0x1.0"] {
        assert_eq!(
            err(&row(["A", "int", "?", "?", "?", f])),
            (2, BAD_FLAGS.to_string()),
            "{f:?}"
        );
    }
}

#[test]
fn a_non_ascii_digit_is_refused() {
    // Sixteen bytes: one two-byte letter and fourteen digits.
    let cell = format!("b:\u{e9}{}", "0".repeat(14));
    assert_eq!(
        err(&row(["A", "float", &cell, "?", "?", "?"])),
        bits_msg(&cell)
    );
    // Arabic-Indic digit one is not an ASCII digit.
    let cell = format!("b:{}\u{661}{}", "0".repeat(7), "0".repeat(7));
    assert_eq!(
        err(&row(["A", "float", &cell, "?", "?", "?"])),
        bits_msg(&cell)
    );
    for f in ["0x\u{661}", "0x1\u{e9}", "0x\u{ff11}", "0x\u{ff21}"] {
        assert_eq!(
            err(&row(["A", "int", "?", "?", "?", f])),
            (2, BAD_FLAGS.to_string()),
            "{f:?}"
        );
    }
}

#[test]
fn the_digit_count_of_a_bit_cell_is_exact() {
    assert_eq!(
        err(&row(["A", "float", "b:", "?", "?", "?"])).1,
        "bit cell needs 16 hex digits: \"b:\""
    );
    let short = format!("b:{}", "0".repeat(15));
    assert_eq!(
        err(&row(["A", "float", &short, "?", "?", "?"])).1,
        format!("bit cell needs 16 hex digits: {short:?}")
    );
    let long = format!("b:{}", "0".repeat(17));
    assert_eq!(
        err(&row(["A", "float", &long, "?", "?", "?"])).1,
        format!("bit cell needs 16 hex digits: {long:?}")
    );
    let ok = Table::parse(&row([
        "A",
        "float",
        &format!("b:{}", "0".repeat(16)),
        "?",
        "?",
        "?",
    ]));
    assert!(ok.unwrap().params[0].default.bits_eq(&Cell::Float(0.0)));
}

#[test]
fn an_empty_flags_body_keeps_its_own_text() {
    assert_eq!(
        err(&row(["A", "int", "?", "?", "?", "0x"])),
        (
            2,
            "bad flags: cannot parse integer from empty string".to_string()
        )
    );
}

#[test]
fn a_flags_body_wider_than_thirty_two_bits_keeps_its_own_text() {
    assert_eq!(
        err(&row(["A", "int", "?", "?", "?", "0x100000000"])),
        (
            2,
            "bad flags: number too large to fit in target type".to_string()
        )
    );
    let t = Table::parse(&row(["A", "int", "?", "?", "?", "0xffffffff"])).unwrap();
    assert_eq!(t.params[0].flags, Some(ParamFlags(u32::MAX)));
}

#[test]
fn only_the_lower_case_prefix_opens_a_flags_cell() {
    for f in ["x1", "X1", "0X1", "00x1", "1", "ff", "0 x1"] {
        assert_eq!(
            err(&row(["A", "int", "?", "?", "?", f])),
            (2, format!("bad flags cell {f:?}")),
            "{f:?}"
        );
    }
}

#[test]
fn leading_zeros_are_read_as_written() {
    let t = Table::parse(&row(["A", "int", "?", "?", "?", "0x0000000000000001"])).unwrap();
    assert_eq!(t.params[0].flags, Some(ParamFlags(1)));
    let t = Table::parse(&row(["A", "int", "?", "?", "?", "0x00"])).unwrap();
    assert_eq!(t.params[0].flags, Some(ParamFlags(0)));
}

#[test]
fn upper_case_digits_stay_accepted() {
    let t = Table::parse(&row([
        "A",
        "float",
        "b:3FF0000000000000",
        "?",
        "?",
        "0xAbC",
    ]))
    .unwrap();
    assert!(t.params[0].default.bits_eq(&Cell::Float(1.0)));
    assert_eq!(t.params[0].flags, Some(ParamFlags(0xabc)));
    assert!(t.serialize().contains("b:3ff0000000000000\t?\t?\t0xabc\n"));
}

#[test]
fn every_hexadecimal_digit_is_read() {
    let t = Table::parse(&row([
        "A",
        "float",
        "b:0123456789abcdef",
        "?",
        "?",
        "0x89abcdef",
    ]))
    .unwrap();
    assert!(t.params[0]
        .default
        .bits_eq(&Cell::Float(f64::from_bits(0x0123_4567_89ab_cdef))));
    assert_eq!(t.params[0].flags, Some(ParamFlags(0x89ab_cdef)));
    let t = Table::parse(&row([
        "A",
        "float",
        "b:FEDCBA9876543210",
        "?",
        "?",
        "0x01234567",
    ]))
    .unwrap();
    assert!(t.params[0]
        .default
        .bits_eq(&Cell::Float(f64::from_bits(0xfedc_ba98_7654_3210))));
    assert_eq!(t.params[0].flags, Some(ParamFlags(0x0123_4567)));
}

#[test]
fn a_digit_cell_is_checked_before_the_next_cell() {
    // The first bad cell of the row is the one reported.
    let e = err(&row([
        "A",
        "float",
        "b:+00000000000000a",
        "i:x",
        "?",
        "0x+1",
    ]));
    assert_eq!(e, bits_msg("b:+00000000000000a"));
    let e = err(&row(["A", "float", "?", "?", "?", "0x+1"]));
    assert_eq!(e, (2, BAD_FLAGS.to_string()));
}

// Narrow cases of the format and the pairing. Where the page is silent the current behaviour
// is pinned; where it speaks, the code agrees.

#[test]
fn known_special_floats_are_known() {
    for v in [f64::MIN_POSITIVE, 5e-324, f64::MAX, f64::MIN, f64::EPSILON] {
        assert!(!Cell::Float(v).is_unknown());
    }
}

#[test]
fn opposite_infinities_differ() {
    let one = |c: Cell| Table {
        params: vec![with("Param A", c)],
    };
    let v = &pair_tables(
        &one(Cell::Float(f64::INFINITY)),
        &one(Cell::Float(f64::NEG_INFINITY)),
    )
    .verdicts[0];
    assert_eq!(v.default, Some(FieldVerdict::Differ));
    let v = &pair_tables(
        &one(Cell::Float(f64::INFINITY)),
        &one(Cell::Float(f64::INFINITY)),
    )
    .verdicts[0];
    assert_eq!(v.default, Some(FieldVerdict::Same));
}

#[test]
fn a_repeated_prefix_in_a_flags_cell_is_refused() {
    assert_eq!(
        err(&row(["A", "int", "?", "?", "?", "0x0x1"])),
        (2, BAD_FLAGS.to_string())
    );
}

#[test]
fn a_kind_cell_with_a_stray_byte_is_refused() {
    assert_eq!(
        err(&row(["A", "int ", "?", "?", "?", "?"])).1,
        "unknown kind \"int \""
    );
    assert_eq!(
        err(&row(["A", "int\u{0}", "?", "?", "?", "?"])).1,
        "unknown kind \"int\\0\""
    );
}

#[test]
fn text_float_nan_and_tiny_values_are_kept() {
    let t = Table::parse(&row(["A", "float", "f:nan", "f:1e-310", "f:-1e-310", "?"])).unwrap();
    assert!(matches!(t.params[0].default, Cell::Float(v) if v.is_nan()));
    assert!(t.params[0].min.bits_eq(&Cell::Float(1e-310)));
    assert!(t.params[0].max.bits_eq(&Cell::Float(-1e-310)));
}

#[test]
fn verdict_names_are_as_written() {
    let p = pair_tables(&plain(&[" Param A ", " Other:"]), &plain(&["param a"]));
    assert_eq!(p.verdicts[0].class, MatchClass::Normalized);
    assert_eq!(p.verdicts[0].predicted, " Param A ");
    assert_eq!(p.verdicts[1].class, MatchClass::Unmatched);
    assert_eq!(p.verdicts[1].predicted, " Other:");
    assert_eq!(p.verdicts[1].observed, None);
}

#[test]
fn a_hyphen_is_kept_and_a_carriage_return_stays_inside_a_cell() {
    assert_eq!(normalize_name("Name-"), "name-");
    assert_eq!(normalize_name("Name-:"), "name-");
    let t = Table::parse(&row(["a\rb", "int", "?", "?", "?", "?"])).unwrap();
    assert_eq!(t.params[0].name, "a\rb");
    assert_eq!(t.params.len(), 1);
}
