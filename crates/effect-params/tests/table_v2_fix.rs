//! Delta tests of the version 2 table format: blank and tab lines, the end of file line,
//! strict cells, serializer validation, and unmatched observed rows. Made up data only.

use effect_params::*;

fn perr(text: &str) -> ParseError {
    Table::parse(text).unwrap_err()
}

const V2ROW: &str = "i:7\tGain\tfloat\t?\t?\t?\t?\t?\t?\t?";

fn v2(row: &str) -> String {
    format!("table v2\n{row}\n")
}

fn with_cell(i: usize, v: &str) -> String {
    let mut c: Vec<&str> = V2ROW.split('\t').collect();
    c[i] = v;
    v2(&c.join("\t"))
}

fn def(id: Option<i64>, name: &str, kind: ParamKind) -> ParamDef {
    let mut d = ParamDef::new(name, kind);
    d.id = id;
    d
}

// F1

#[test]
fn f1_a_line_of_only_tabs_is_a_row_with_empty_cells_in_v2() {
    let e = perr("table v2\n\t\t\t\t\t\t\t\t\t\n");
    assert_eq!(e.line, 2);
    assert!(e.message.contains("bad id cell"), "{}", e.message);
    // Any number of tabs, also in the middle of the body.
    let e = perr(&format!("table v2\n{V2ROW}\n\t\n"));
    assert_eq!(e.line, 3);
    assert!(e.message.contains("expected 10 cells"), "{}", e.message);
    let e = perr(&format!("table v2\n{V2ROW}\n \t \n"));
    assert_eq!(
        (e.line, e.message.as_str()),
        (3, "expected 10 cells, found 2")
    );
}

#[test]
fn f1_spaces_only_and_empty_lines_are_still_blank() {
    let t = Table::parse(&format!("table v2\n\n   \n{V2ROW}\n\r\n  \n")).unwrap();
    assert_eq!(t.params.len(), 1);
}

#[test]
fn f1_version_1_and_the_lines_before_the_magic_keep_their_old_leniency() {
    let t = Table::parse("\t\ntable v1\n\t\t\nA\tint\t?\t?\t?\t?\n").unwrap();
    assert_eq!(t.params.len(), 1);
    // The magic line may carry tabs and spaces around it.
    assert!(Table::parse("\ttable v2 \t\n").is_ok());
}

// F2

#[test]
fn f2_missing_magic_reports_the_line_after_the_last_line() {
    assert_eq!(perr("").line, 1);
    assert_eq!(perr("# note\n").line, 2);
    assert_eq!(perr("# a\n# b\n\n").line, 4);
    assert_eq!(perr("# a").line, 2);
    assert_eq!(perr("\n").line, 2);
    assert_eq!(
        perr("").to_string(),
        "line 1: missing \"table v1\" or \"table v2\""
    );
}

#[test]
fn f2_a_present_but_wrong_first_line_is_still_reported_at_its_own_line() {
    assert_eq!(perr("# a\nwrong\n").line, 2);
}

// F3

#[test]
fn f3_v2_integer_cells_refuse_a_plus_sign_and_v1_keeps_it() {
    for (i, cell) in [(0usize, "i:+5"), (3, "i:+5"), (4, "i:+5"), (5, "i:+5")] {
        let m = Table::parse(&with_cell(i, cell)).unwrap_err().message;
        assert!(m.contains("plus sign"), "cell {i}: {m}");
    }
    // The minus sign and zero forms stay.
    assert!(Table::parse(&with_cell(3, "i:-5")).is_ok());
    assert!(Table::parse(&with_cell(0, "i:-0")).is_ok());
    // Version 1 loaded a plus sign before and still does.
    assert!(Table::parse("table v1\nA\tint\ti:+5\t?\t?\t?\n").is_ok());
}

#[test]
fn f3_v2_float_cells_are_finite_and_unsigned_by_plus() {
    for bad in [
        "f:+1.5",
        "f:nan",
        "f:NaN",
        "f:inf",
        "f:-inf",
        "f:infinity",
        "f:1e999",
        "f:-1e999",
    ] {
        assert!(Table::parse(&with_cell(3, bad)).is_err(), "{bad}");
    }
    for good in ["f:1.5", "f:-1.5", "f:1e3", "f:1e+3", "f:.5", "f:5.", "f:-0"] {
        assert!(Table::parse(&with_cell(4, good)).is_ok(), "{good}");
    }
    // Version 1 keeps the standard parser's whole range.
    for old in ["f:+1.5", "f:nan", "f:inf"] {
        assert!(
            Table::parse(&format!("table v1\nA\tfloat\t{old}\t?\t?\t?\n")).is_ok(),
            "{old}"
        );
    }
    // Non finite values stay writable through the bit form.
    let mut d = def(Some(1), "N", ParamKind::Float);
    d.default = Cell::Float(f64::NAN);
    d.min = Cell::Float(f64::NEG_INFINITY);
    let text = Table { params: vec![d] }.serialize();
    let back = Table::parse(&text).unwrap();
    assert!(back.params[0].default.bits_eq(&Cell::Float(f64::NAN)));
}

#[test]
fn f3_crlf_line_ends_and_a_padded_magic_are_accepted_and_the_text_is_lf() {
    let t = Table::parse(&format!("table v2\r\n{V2ROW}\r\n")).unwrap();
    assert_eq!(t.params.len(), 1);
    assert_eq!(t.params[0].extra, Extra::Unknown);
    assert!(Table::parse("  table v2  \n").is_ok());
    assert!(!t.serialize().contains('\r'));
}

#[test]
fn f3_hex_cells_take_either_digit_case_and_a_lower_prefix_only() {
    let t = Table::parse(&with_cell(7, "0xAbCd")).unwrap();
    assert_eq!(t.params[0].host_flags, Some(0xabcd));
    assert!(t.serialize().contains("\t0xabcd\t"));
    assert!(Table::parse(&with_cell(7, "0XAB")).is_err());
    assert!(Table::parse(&with_cell(7, "0x")).is_err());
    assert!(Table::parse(&with_cell(7, "0x123456789")).is_err());
    assert!(Table::parse(&with_cell(7, "0x+1")).is_err());
    // The own flags cell is a u32 of any digit count, leading zeros allowed.
    let t = Table::parse(&with_cell(6, "0x00000000000001")).unwrap();
    assert_eq!(t.params[0].flags, Some(ParamFlags(1)));
    assert!(Table::parse(&with_cell(6, "0x100000000")).is_err());
    // Bit cells: sixteen digits, either case.
    let t = Table::parse(&with_cell(3, "b:3FF0000000000000")).unwrap();
    assert!(t.params[0].default.bits_eq(&Cell::Float(1.0)));
}

#[test]
fn f3_the_version_1_name_cell_has_no_escapes() {
    let t = Table::parse("table v1\nA\\tB\tint\t?\t?\t?\t?\n").unwrap();
    assert_eq!(t.params[0].name, "A\\tB");
    let t = Table::parse(&with_cell(1, "A\\tB")).unwrap();
    assert_eq!(t.params[0].name, "A\tB");
}

// F4

#[test]
fn f4_v2_extra_cells_refuse_the_empty_forms_that_have_an_escape() {
    // An empty choice is written \\e, an empty label l:\\e.
    for bad in [
        "c:a||b",
        "c:|",
        "c:a|",
        "c:|a",
        "l:",
        "s:+2:0x1:3ff0000000000000:3ff0000000000000",
    ] {
        assert!(Table::parse(&with_cell(9, bad)).is_err(), "{bad}");
        assert!(Extra::parse_strict(bad).is_err(), "{bad}");
    }
    for good in ["c:", "c:\\e", "c:a|\\e|b", "l:\\e", "d:", "d:00Ff"] {
        assert!(Table::parse(&with_cell(9, good)).is_ok(), "{good}");
    }
    // The lenient reader (version 1 seventh cell) keeps the old forms.
    for old in ["c:a||b", "l:", "s:+2:0x1:3ff0000000000000:3ff0000000000000"] {
        assert!(Extra::parse(old).is_ok(), "{old}");
        assert!(
            Table::parse(&format!("table v1\nA\tint\t?\t?\t?\t?\t{old}\n")).is_ok(),
            "{old}"
        );
    }
}

#[test]
fn f4_extra_escapes_and_empty_forms_round_trip() {
    let mut a = def(Some(1), "A", ParamKind::Popup);
    a.extra = Extra::Choices(vec![
        "".into(),
        "x|y".into(),
        "\\e".into(),
        "t\tn\nr\r\\".into(),
    ]);
    let mut b = def(Some(2), "B", ParamKind::Button);
    b.extra = Extra::Label("".into());
    let mut c = def(Some(3), "C", ParamKind::Popup);
    c.extra = Extra::Choices(vec![]);
    let mut d = def(Some(4), "D", ParamKind::Popup);
    d.extra = Extra::Choices(vec!["".into()]);
    let mut e = def(Some(5), "E", ParamKind::Arbitrary);
    e.extra = Extra::Data(vec![]);
    let tab = Table {
        params: vec![a, b, c, d, e],
    };
    let text = tab.serialize();
    assert!(
        text.contains("c:\\e|x\\py|\\\\e|t\\tn\\nr\\r\\\\\n"),
        "{text}"
    );
    assert!(text.contains("\tl:\\e\n"));
    assert!(text.contains("\tc:\n") && text.contains("\tc:\\e\n"));
    let back = Table::parse(&text).unwrap();
    for (x, y) in tab.params.iter().zip(&back.params) {
        assert_eq!(x.extra, y.extra);
    }
    assert_eq!(back.serialize(), text);
}

// F5

#[test]
fn f5_validate_names_the_row_and_the_problem() {
    let ok = Table {
        params: vec![
            def(Some(1), "A", ParamKind::Int),
            def(None, "A", ParamKind::Int),
        ],
    };
    assert_eq!(ok.validate(), Ok(()));
    let e = Table {
        params: vec![
            def(Some(1), "A", ParamKind::Int),
            def(None, "", ParamKind::Int),
        ],
    }
    .validate()
    .unwrap_err();
    assert_eq!(
        (e.row, e.message.as_str()),
        (2, "empty name needs a known id")
    );
    assert_eq!(e.to_string(), "row 2: empty name needs a known id");
    let e = Table {
        params: vec![
            def(Some(1), "A", ParamKind::Int),
            def(Some(2), "B", ParamKind::Int),
            def(Some(1), "C", ParamKind::Int),
        ],
    }
    .validate()
    .unwrap_err();
    assert_eq!((e.row, e.message.as_str()), (3, "duplicate id 1"));
    let mut d = def(None, "X", ParamKind::Float);
    d.default = Cell::None;
    assert_eq!(Table { params: vec![d] }.validate().unwrap_err().row, 1);
    for which in 0..2 {
        let mut l = def(None, "L", ParamKind::Layer);
        if which == 0 {
            l.min = Cell::None;
        } else {
            l.max = Cell::None;
        }
        assert!(Table { params: vec![l] }.validate().is_err());
    }
    let mut l = def(None, "L", ParamKind::Layer);
    l.default = Cell::None;
    assert!(Table { params: vec![l] }.validate().is_ok());
}

#[test]
fn f5_try_serialize_refuses_what_the_parser_would_refuse() {
    let bad_tables = vec![
        Table {
            params: vec![def(None, "", ParamKind::Int)],
        },
        Table {
            params: vec![
                def(Some(3), "A", ParamKind::Int),
                def(Some(3), "B", ParamKind::Int),
            ],
        },
        {
            let mut d = def(Some(1), "A", ParamKind::Int);
            d.max = Cell::None;
            Table { params: vec![d] }
        },
    ];
    for t in &bad_tables {
        assert!(t.try_serialize().is_err());
    }
    // The text those tables would have had is refused by the parser: the two agree.
    let hand = "table v2\n?\t\tint\t?\t?\t?\t?\t?\t?\t?\n";
    assert!(Table::parse(hand).is_err());
}

#[test]
fn f5_try_serialize_equals_serialize_on_a_valid_table_and_reads_back() {
    let mut l = def(Some(9), "Lay", ParamKind::Layer);
    l.default = Cell::None;
    let t = Table {
        params: vec![
            def(None, "Plain", ParamKind::Int),
            def(Some(8), "", ParamKind::Point),
            l,
        ],
    };
    let s = t.try_serialize().unwrap();
    assert_eq!(s, t.serialize());
    let back = Table::parse(&s).unwrap();
    assert_eq!(back.serialize(), s);
    assert_eq!(back.validate(), Ok(()));
}

#[test]
#[should_panic(expected = "invalid table")]
fn f5_serialize_panics_rather_than_write_an_unreadable_table() {
    let _ = Table {
        params: vec![def(None, "", ParamKind::Int)],
    }
    .serialize();
}

// F6

#[test]
fn f6_unmatched_observed_rows_with_an_empty_name_are_listed_by_id() {
    let p = Table {
        params: vec![def(Some(1), "Kept", ParamKind::Int)],
    };
    let o = Table {
        params: vec![
            def(Some(1), "Kept", ParamKind::Int),
            def(Some(41), "", ParamKind::Int),
            def(Some(-42), "", ParamKind::Int),
            def(None, "Named", ParamKind::Int),
            def(None, "", ParamKind::Int),
        ],
    };
    let r = pair_tables(&p, &o);
    assert_eq!(
        r.unmatched_observed,
        vec!["<id i:41>", "<id i:-42>", "Named", "<no name, no id>"]
    );
}
