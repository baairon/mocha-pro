//! Edge cases of the table format, the pairing, the flag set and the kind names: exact error
//! text and line numbers, exact serialized text, the boundaries of the cell checks, the
//! normalisation rules one by one, the claim rules of the pairing, and the fixed values of the
//! flag constants and the kind list. Each test pins one rule so that a change to it fails here
//! by name.

use effect_params::*;

fn bare(name: &str, kind: ParamKind) -> ParamDef {
    ParamDef::new(name, kind)
}

fn with(name: &str, d: Cell, lo: Cell, hi: Cell, f: Option<u32>) -> ParamDef {
    ParamDef {
        name: name.into(),
        kind: ParamKind::Float,
        default: d,
        min: lo,
        max: hi,
        flags: f.map(ParamFlags),
        extra: Extra::Unknown,
        id: None,
        host_flags: None,
        host_ui_flags: None,
    }
}

fn table(params: Vec<ParamDef>) -> Table {
    Table { params }
}

fn err(text: &str) -> (usize, String) {
    let e = Table::parse(text).unwrap_err();
    (e.line, e.message)
}

fn row(cells: [&str; 6]) -> String {
    format!("table v1\n{}\n", cells.join("\t"))
}

#[test]
fn error_text_and_line_numbers_are_exact() {
    assert_eq!(
        err(""),
        (1, "missing \"table v1\" or \"table v2\"".to_string())
    );
    assert_eq!(
        err("# only\n\n"),
        (3, "missing \"table v1\" or \"table v2\"".to_string())
    );
    assert_eq!(
        err("nope\n"),
        (1, "expected \"table v1\" or \"table v2\"".to_string())
    );
    assert_eq!(
        err("table v1\nA\tint\t?\t?\t?\n"),
        (2, "expected 6 cells, found 5".to_string())
    );
    assert_eq!(
        err("table v1\nA\tint\t?\t?\t?\t?\t?\t?\n"),
        (2, "expected 6 cells, found 8".to_string())
    );
    assert_eq!(
        err("table v1\n\tint\t?\t?\t?\t?\n"),
        (2, "empty name".to_string())
    );
    assert_eq!(
        err(&row(["A", "nope", "?", "?", "?", "?"])),
        (2, "unknown kind \"nope\"".to_string())
    );
    assert_eq!(
        err(&row(["A", "int", "?", "?", "?", "12"])),
        (2, "bad flags cell \"12\"".to_string())
    );
    // Comment and blank lines count toward the line number.
    let late = "# c\n\ntable v1\n# d\nA\tint\t?\t?\t?\tzz\n";
    assert_eq!(err(late), (5, "bad flags cell \"zz\"".to_string()));
}

#[test]
fn cell_error_messages_name_the_cell_and_the_cause() {
    let (l, m) = err(&row(["A", "int", "i:x", "?", "?", "?"]));
    assert_eq!(l, 2);
    assert!(m.starts_with("bad int \"i:x\": "), "{m}");
    assert!(m.len() > "bad int \"i:x\": ".len());
    let (_, m) = err(&row(["A", "float", "?", "f:zz", "?", "?"]));
    assert!(m.starts_with("bad float \"f:zz\": "), "{m}");
    assert!(m.len() > "bad float \"f:zz\": ".len());
    let (_, m) = err(&row(["A", "float", "?", "?", "b:zzzzzzzzzzzzzzzz", "?"]));
    assert!(m.starts_with("bad bits \"b:zzzzzzzzzzzzzzzz\": "), "{m}");
    assert_eq!(
        err(&row(["A", "float", "b:12", "?", "?", "?"])).1,
        "bit cell needs 16 hex digits: \"b:12\""
    );
    assert_eq!(
        err(&row(["A", "float", "?", "q", "?", "?"])).1,
        "unrecognised cell \"q\""
    );
    let (_, m) = err(&row(["A", "int", "?", "?", "?", "0xzz"]));
    assert!(m.starts_with("bad flags: "), "{m}");
    assert!(m.len() > "bad flags: ".len());
}

#[test]
fn bit_cells_need_exactly_sixteen_hex_digits() {
    assert!(Table::parse(&row(["A", "float", "b:000000000000000", "?", "?", "?"])).is_err());
    assert!(Table::parse(&row(["A", "float", "b:00000000000000000", "?", "?", "?"])).is_err());
    assert!(Table::parse(&row(["A", "float", "b:000000000000000g", "?", "?", "?"])).is_err());
    let t = Table::parse(&row(["A", "float", "b:000000000000000a", "?", "?", "?"])).unwrap();
    match t.params[0].default {
        Cell::Float(v) => assert_eq!(v.to_bits(), 10),
        _ => panic!("wrong cell"),
    }
    let t = Table::parse(&row(["A", "float", "b:3FF0000000000000", "?", "?", "?"])).unwrap();
    assert!(matches!(t.params[0].default, Cell::Float(v) if v == 1.0));
}

#[test]
fn unknown_marker_is_exactly_a_question_mark() {
    assert!(Table::parse(&row(["A", "int", "?x", "?", "?", "?"])).is_err());
    assert!(Table::parse(&row(["A", "int", "??", "?", "?", "?"])).is_err());
    assert!(Table::parse(&row(["A", "int", "i:", "?", "?", "?"])).is_err());
    assert!(Table::parse(&row(["A", "int", "", "?", "?", "?"])).is_err());
}

#[test]
fn flags_cell_boundaries() {
    assert!(Table::parse(&row(["A", "int", "?", "?", "?", "0X10"])).is_err());
    assert!(Table::parse(&row(["A", "int", "?", "?", "?", "0x"])).is_err());
    assert!(Table::parse(&row(["A", "int", "?", "?", "?", "0x1ffffffff"])).is_err());
    let t = Table::parse(&row(["A", "int", "?", "?", "?", "0xffffffff"])).unwrap();
    assert_eq!(t.params[0].flags, Some(ParamFlags(u32::MAX)));
    let t = Table::parse(&row(["A", "int", "?", "?", "?", "0xAb"])).unwrap();
    assert_eq!(t.params[0].flags, Some(ParamFlags(0xab)));
}

#[test]
fn line_classes_comment_blank_and_magic() {
    // A whitespace-only line is blank; a comment must start in column one.
    let t = Table::parse("   \n\t\ntable v1\n   \nA\tint\t?\t?\t?\t?\n").unwrap();
    assert_eq!(t.params.len(), 1);
    assert_eq!(err("  # indented\ntable v1\n").0, 1);
    // The magic line is trimmed; a trailing carriage return does not break it.
    assert!(Table::parse("  table v1  \n").is_ok());
    assert!(Table::parse("table v1\r\nA\tint\t?\t?\t?\t?\r\n").is_ok());
    // Only the first data line is the magic: a second one is a row with one cell.
    assert_eq!(
        err("table v1\ntable v1\n"),
        (2, "expected 6 cells, found 1".to_string())
    );
    // An empty table is valid and serializes to the magic line alone.
    let t = Table::parse("table v1\n").unwrap();
    assert!(t.params.is_empty());
    assert_eq!(t.serialize(), "table v2\n");
}

#[test]
fn serialized_text_is_exact() {
    let t = table(vec![
        with(
            "Param A",
            Cell::Float(0.0),
            Cell::Float(1.0),
            Cell::Float(f64::NEG_INFINITY),
            Some(0xabc),
        ),
        with(
            "Param B",
            Cell::Int(-3),
            Cell::Int(4),
            Cell::Unknown,
            Some(0),
        ),
        bare("Param C", ParamKind::Popup),
    ]);
    let want = "table v2\n\
        ?\tParam A\tfloat\tb:0000000000000000\tb:3ff0000000000000\tb:fff0000000000000\t0xabc\t?\t?\t?\n\
        ?\tParam B\tfloat\ti:-3\ti:4\t?\t0x0\t?\t?\t?\n\
        ?\tParam C\tpopup\t?\t?\t?\t?\t?\t?\t?\n";
    assert_eq!(t.serialize(), want);
    let back = Table::parse(want).unwrap();
    assert!(back.params[0].min.bits_eq(&Cell::Float(1.0)));
    assert!(back.params[0].max.bits_eq(&Cell::Float(f64::NEG_INFINITY)));
}

#[test]
fn text_float_cells_parse_to_the_decimal_value() {
    let t = Table::parse(&row(["A", "float", "f:-1.5", "f:1e3", "f:0.25", "?"])).unwrap();
    assert!(Table::parse(&row(["A", "int", "i:9223372036854775807", "?", "?", "?"])).is_ok());
    assert!(Table::parse(&row(["A", "int", "i:9223372036854775808", "?", "?", "?"])).is_err());
    assert!(t.params[0].default.bits_eq(&Cell::Float(-1.5)));
    assert!(t.params[0].min.bits_eq(&Cell::Float(1000.0)));
    assert!(t.params[0].max.bits_eq(&Cell::Float(0.25)));
}

#[test]
fn bits_eq_is_bit_identity() {
    let nan = Cell::Float(f64::NAN);
    assert!(nan.bits_eq(&Cell::Float(f64::NAN)));
    assert!(!nan.bits_eq(&Cell::Float(f64::from_bits(f64::NAN.to_bits() ^ 1))));
    assert!(!Cell::Float(0.0).bits_eq(&Cell::Float(-0.0)));
    assert!(Cell::Unknown.bits_eq(&Cell::Unknown));
    assert!(!Cell::Unknown.bits_eq(&Cell::Int(0)));
    assert!(!Cell::Unknown.bits_eq(&Cell::Float(0.0)));
    assert!(!Cell::Int(0).bits_eq(&Cell::Unknown));
    assert!(!Cell::Int(0).bits_eq(&Cell::Float(0.0)));
    assert!(Cell::Int(i64::MIN).bits_eq(&Cell::Int(i64::MIN)));
    assert!(!Cell::Int(2).bits_eq(&Cell::Int(3)));
}

#[test]
fn only_the_unknown_cell_is_unknown() {
    assert!(Cell::Unknown.is_unknown());
    assert!(!Cell::Int(0).is_unknown());
    assert!(!Cell::Int(-1).is_unknown());
    assert!(!Cell::Float(0.0).is_unknown());
}

#[test]
fn normalize_each_rule_alone() {
    assert_eq!(normalize_name(""), "");
    assert_eq!(normalize_name("ABC"), "abc");
    assert_eq!(normalize_name("a\tb"), "a b");
    assert_eq!(normalize_name("\t a   b \t"), "a b");
    assert_eq!(normalize_name("a \u{a0} b"), "a b");
    assert_eq!(normalize_name("Name :"), "name");
    assert_eq!(normalize_name("Name ..."), "name");
    assert_eq!(normalize_name("Name ... :"), "name");
    assert_eq!(normalize_name("Name: ..."), "name");
    assert_eq!(normalize_name("Name:..."), "name");
    assert_eq!(normalize_name("Name::"), "name");
    assert_eq!(normalize_name("Name......"), "name");
    assert_eq!(normalize_name("Name.."), "name..");
    assert_eq!(normalize_name("Name.:"), "name.");
    assert_eq!(normalize_name("...Name"), "...name");
    assert_eq!(normalize_name("a: b"), "a: b");
    assert_eq!(normalize_name("..."), "");
    assert_eq!(normalize_name(":"), "");
}

fn plain(names: &[&str]) -> Table {
    table(names.iter().map(|n| bare(n, ParamKind::Int)).collect())
}

#[test]
fn identical_names_claim_one_observed_row_each() {
    let p = pair_tables(&plain(&["Param A", "Param A"]), &plain(&["Param A"]));
    assert_eq!(p.count(MatchClass::Exact), 1);
    assert_eq!(p.count(MatchClass::Unmatched), 1);
    assert_eq!(p.verdicts[0].class, MatchClass::Exact);
    assert_eq!(p.verdicts[1].class, MatchClass::Unmatched);
    assert!(p.unmatched_observed.is_empty());
}

#[test]
fn normalized_matches_also_claim_once() {
    let p = pair_tables(&plain(&["a", "A "]), &plain(&["A"]));
    assert_eq!(p.verdicts[0].class, MatchClass::Normalized);
    assert_eq!(p.verdicts[1].class, MatchClass::Unmatched);
    assert_eq!(p.count(MatchClass::Normalized), 1);
    assert!(p.unmatched_observed.is_empty());
}

#[test]
fn an_exact_match_is_not_replaced_by_a_normalized_one() {
    let p = pair_tables(&plain(&["Param A"]), &plain(&["Param A", "param a"]));
    let v = &p.verdicts[0];
    assert_eq!(v.class, MatchClass::Exact);
    assert_eq!(v.observed.as_deref(), Some("Param A"));
    assert_eq!(p.unmatched_observed, vec!["param a".to_string()]);
    assert_eq!(p.count(MatchClass::Normalized), 0);
}

#[test]
fn case_only_difference_is_normalized_not_exact() {
    let p = pair_tables(&plain(&["param a"]), &plain(&["Param A"]));
    assert_eq!(p.verdicts[0].class, MatchClass::Normalized);
    assert_eq!(p.verdicts[0].observed.as_deref(), Some("Param A"));
    assert_eq!(p.verdicts[0].predicted, "param a");
}

#[test]
fn unmatched_observed_lists_only_unclaimed_rows_in_order() {
    let p = pair_tables(
        &plain(&["Param B"]),
        &plain(&["Param A", "Param B", "Param C"]),
    );
    assert_eq!(
        p.unmatched_observed,
        vec!["Param A".to_string(), "Param C".to_string()]
    );
    assert_eq!(p.count(MatchClass::Exact), 1);
    assert_eq!(p.count(MatchClass::Unmatched), 0);
}

#[test]
fn empty_tables_pair_to_nothing() {
    let p = pair_tables(&Table::default(), &Table::default());
    assert!(p.verdicts.is_empty() && p.unmatched_observed.is_empty());
    let p = pair_tables(&Table::default(), &plain(&["X"]));
    assert_eq!(p.unmatched_observed, vec!["X".to_string()]);
}

#[test]
fn each_field_is_judged_against_its_own_field() {
    let mk = |d: f64, lo: f64, hi: f64| {
        table(vec![with(
            "Param A",
            Cell::Float(d),
            Cell::Float(lo),
            Cell::Float(hi),
            Some(1),
        )])
    };
    let pred = mk(1.0, 2.0, 3.0);
    for (obs, d, lo, hi) in [
        (
            mk(9.0, 2.0, 3.0),
            FieldVerdict::Differ,
            FieldVerdict::Same,
            FieldVerdict::Same,
        ),
        (
            mk(1.0, 9.0, 3.0),
            FieldVerdict::Same,
            FieldVerdict::Differ,
            FieldVerdict::Same,
        ),
        (
            mk(1.0, 2.0, 9.0),
            FieldVerdict::Same,
            FieldVerdict::Same,
            FieldVerdict::Differ,
        ),
    ] {
        let v = &pair_tables(&pred, &obs).verdicts[0];
        assert_eq!((v.default, v.min, v.max), (Some(d), Some(lo), Some(hi)));
    }
    // Swapped min and max are a difference on both fields.
    let v = &pair_tables(&pred, &mk(1.0, 3.0, 2.0)).verdicts[0];
    assert_eq!(v.min, Some(FieldVerdict::Differ));
    assert_eq!(v.max, Some(FieldVerdict::Differ));
}

#[test]
fn unknown_cells_are_unscored_or_observed_unknown() {
    let mk = |d: Cell, lo: Cell, hi: Cell| table(vec![with("Param A", d, lo, hi, None)]);
    let u = Cell::Unknown;
    let k = Cell::Int(0);
    let v = &pair_tables(&mk(u, u, k), &mk(u, k, u)).verdicts[0];
    assert_eq!(v.default, Some(FieldVerdict::Unscored));
    assert_eq!(v.min, Some(FieldVerdict::Unscored));
    assert_eq!(v.max, Some(FieldVerdict::ObservedUnknown));
    let v = &pair_tables(&mk(k, k, k), &mk(u, k, u)).verdicts[0];
    assert_eq!(v.default, Some(FieldVerdict::ObservedUnknown));
    assert_eq!(v.min, Some(FieldVerdict::Same));
    assert_eq!(v.max, Some(FieldVerdict::ObservedUnknown));
    assert_eq!(v.flags, Some(FieldVerdict::Unscored));
    assert_eq!(v.flag_diff, None);
}

#[test]
fn flag_verdicts_and_their_difference() {
    let mk = |f: Option<u32>| {
        table(vec![with(
            "Param A",
            Cell::Unknown,
            Cell::Unknown,
            Cell::Unknown,
            f,
        )])
    };
    let same = &pair_tables(&mk(Some(0x1f)), &mk(Some(0x1f))).verdicts[0];
    assert_eq!(same.flags, Some(FieldVerdict::Same));
    assert_eq!(same.flag_diff, Some(ParamFlags::NONE));
    let d = &pair_tables(&mk(Some(0b0110)), &mk(Some(0b0011))).verdicts[0];
    assert_eq!(d.flags, Some(FieldVerdict::Differ));
    assert_eq!(d.flag_diff, Some(ParamFlags(0b0101)));
    let o = &pair_tables(&mk(Some(5)), &mk(None)).verdicts[0];
    assert_eq!(o.flags, Some(FieldVerdict::ObservedUnknown));
    assert_eq!(o.flag_diff, None);
    let u = &pair_tables(&mk(None), &mk(None)).verdicts[0];
    assert_eq!(u.flags, Some(FieldVerdict::Unscored));
    assert_eq!(u.flag_diff, None);
}

#[test]
fn kind_verdict_is_same_for_equal_kinds() {
    let one = |k: ParamKind| table(vec![bare("Param A", k)]);
    let v = &pair_tables(&one(ParamKind::Point), &one(ParamKind::Point)).verdicts[0];
    assert_eq!(v.kind, Some(FieldVerdict::Same));
    let v = &pair_tables(&one(ParamKind::Point), &one(ParamKind::Angle)).verdicts[0];
    assert_eq!(v.kind, Some(FieldVerdict::Differ));
}

#[test]
fn flag_constants_have_fixed_values() {
    assert_eq!(ParamFlags::NONE.bits(), 0);
    assert_eq!(ParamFlags::SUPERVISED.bits(), 1);
    assert_eq!(ParamFlags::COLLAPSED.bits(), 2);
    assert_eq!(ParamFlags::HIDDEN.bits(), 4);
    assert_eq!(ParamFlags::NOT_ANIMATABLE.bits(), 8);
    assert_eq!(ParamFlags::REDRAW_ON_CHANGE.bits(), 16);
    assert_eq!(ParamFlags::default(), ParamFlags::NONE);
}

#[test]
fn flag_set_algebra() {
    let a = ParamFlags(0b0110);
    let b = ParamFlags(0b0011);
    assert_eq!(a.union(a), a);
    assert_eq!(a.union(b), ParamFlags(0b0111));
    assert_eq!(a.diff(b), ParamFlags(0b0101));
    assert_eq!(a.diff(b), b.diff(a));
    assert_eq!(a.diff(a), ParamFlags::NONE);
    assert_eq!(a.diff(ParamFlags::NONE), a);
    assert!(a.contains(a));
    assert!(a.contains(ParamFlags(0b0100)));
    assert!(!a.contains(b));
    assert!(!a.contains(ParamFlags(0b1100)));
    assert!(ParamFlags(u32::MAX).contains(a));
}

#[test]
fn kind_list_and_names_are_fixed() {
    let names: Vec<&str> = ParamKind::ALL.iter().map(|k| k.as_str()).collect();
    assert_eq!(
        names,
        [
            "float",
            "int",
            "checkbox",
            "popup",
            "button",
            "layer",
            "point",
            "angle",
            "color",
            "group",
            "group_end",
            "arbitrary",
            "nodata"
        ]
    );
    assert_eq!(ParamKind::parse("checkbox"), Some(ParamKind::Checkbox));
    assert_eq!(ParamKind::parse("popup"), Some(ParamKind::Popup));
    assert_eq!(ParamKind::parse("ANGLE"), None);
    assert_eq!(ParamKind::parse("Angle"), None);
    assert_eq!(ParamKind::parse(" int"), None);
    assert_eq!(ParamKind::parse(""), None);
}

#[test]
fn every_kind_survives_a_table_round_trip() {
    let t = table(
        ParamKind::ALL
            .iter()
            .map(|k| bare(k.as_str(), *k))
            .collect(),
    );
    let back = Table::parse(&t.serialize()).unwrap();
    for (a, b) in t.params.iter().zip(&back.params) {
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.name, b.name);
    }
    assert_eq!(back.params.len(), 13);
}

#[test]
fn a_nan_cell_is_known_and_scored() {
    let nan = Cell::Float(f64::NAN);
    assert!(!nan.is_unknown());
    let one = |c: Cell| table(vec![with("Param A", c, Cell::Unknown, Cell::Unknown, None)]);
    let v = &pair_tables(&one(nan), &one(Cell::Float(f64::NAN))).verdicts[0];
    assert_eq!(v.default, Some(FieldVerdict::Same));
    let v = &pair_tables(&one(nan), &one(Cell::Unknown)).verdicts[0];
    assert_eq!(v.default, Some(FieldVerdict::ObservedUnknown));
    let v = &pair_tables(&one(Cell::Unknown), &one(nan)).verdicts[0];
    assert_eq!(v.default, Some(FieldVerdict::Unscored));
}

#[test]
fn cells_are_not_trimmed_or_padded() {
    for bad in [" ?", "? ", "i: 5", "i:5 ", " i:5", "f: 1", "f:1 "] {
        let e = Table::parse(&row(["A", "int", bad, "?", "?", "?"])).unwrap_err();
        assert_eq!(e.line, 2, "{bad:?}");
    }
    assert_eq!(
        err(&row(["A", "int", " ?", "?", "?", "?"])).1,
        "unrecognised cell \" ?\""
    );
    assert!(err(&row(["A", "int", "i: 5", "?", "?", "?"]))
        .1
        .starts_with("bad int \"i: 5\": "));
    assert_eq!(
        err(&row(["A", "int", "?", "?", "?", "?x"])).1,
        "bad flags cell \"?x\""
    );
    assert_eq!(
        err(&row(["A", "int", "?", "?", "?", " ?"])).1,
        "bad flags cell \" ?\""
    );
}

#[test]
fn decimal_float_cells_keep_double_precision() {
    let t = Table::parse(&row(["A", "float", "f:0.1", "f:16777217", "?", "?"])).unwrap();
    assert!(t.params[0].default.bits_eq(&Cell::Float(0.1f64)));
    assert!(t.params[0].min.bits_eq(&Cell::Float(16777217.0f64)));
    assert!(!t.params[0].default.bits_eq(&Cell::Float(0.1f32 as f64)));
}

#[test]
fn bit_cell_length_is_counted_in_bytes() {
    // Sixteen two-byte characters are 32 bytes, so the length check refuses them.
    let wide = format!("b:{}", "\u{e9}".repeat(16));
    assert_eq!(
        err(&row(["A", "float", &wide, "?", "?", "?"])).1,
        format!("bit cell needs 16 hex digits: {wide:?}")
    );
    assert!(Table::parse(&row(["A", "float", "b:++00000000000000", "?", "?", "?"])).is_err());
}

#[test]
fn flags_cell_prefix_and_sign() {
    assert!(err(&row(["A", "int", "?", "?", "?", "0x"]))
        .1
        .starts_with("bad flags: "));
    assert!(Table::parse(&row(["A", "int", "?", "?", "?", "0x++1"])).is_err());
    assert!(Table::parse(&row(["A", "int", "?", "?", "?", "0x-1"])).is_err());
}

#[test]
fn names_and_kinds_are_taken_as_written() {
    let t = Table::parse(&row([" Param A ", "int", "?", "?", "?", "?"])).unwrap();
    assert_eq!(t.params[0].name, " Param A ");
    let t = Table::parse(&row(["  ", "int", "?", "?", "?", "?"])).unwrap();
    assert_eq!(t.params[0].name, "  ");
    assert_eq!(t.serialize(), "table v2\n?\t  \tint\t?\t?\t?\t?\t?\t?\t?\n");
    assert_eq!(
        err(&row(["A", " int", "?", "?", "?", "?"])).1,
        "unknown kind \" int\""
    );
    assert_eq!(
        err(&row(["A", "int ", "?", "?", "?", "?"])).1,
        "unknown kind \"int \""
    );
}

#[test]
fn parse_error_display_is_line_and_message() {
    let e = Table::parse("table v1\n\tint\t?\t?\t?\t?\n").unwrap_err();
    assert_eq!(e.to_string(), "line 2: empty name");
    let e = Table::parse("").unwrap_err();
    assert_eq!(
        e.to_string(),
        "line 1: missing \"table v1\" or \"table v2\""
    );
}

#[test]
fn normalize_folds_full_unicode_case_and_strips_only_colon_and_dots() {
    assert_eq!(normalize_name("\u{c9}COLE"), "\u{e9}cole");
    assert_eq!(normalize_name("Name;"), "name;");
    assert_eq!(normalize_name("Name,"), "name,");
    assert_eq!(normalize_name("Name!"), "name!");
}

#[test]
fn the_first_of_two_same_name_rows_is_claimed() {
    let pred = table(vec![with(
        "Param A",
        Cell::Int(1),
        Cell::Unknown,
        Cell::Unknown,
        None,
    )]);
    let obs = table(vec![
        with("Param A", Cell::Int(1), Cell::Unknown, Cell::Unknown, None),
        with("Param A", Cell::Int(2), Cell::Unknown, Cell::Unknown, None),
    ]);
    let p = pair_tables(&pred, &obs);
    assert_eq!(p.verdicts[0].class, MatchClass::Exact);
    assert_eq!(p.verdicts[0].default, Some(FieldVerdict::Same));
    assert_eq!(p.unmatched_observed, vec!["Param A".to_string()]);

    // The same holds for the normalized pass: the first equal row wins.
    let pred = table(vec![with(
        "param a",
        Cell::Int(1),
        Cell::Unknown,
        Cell::Unknown,
        None,
    )]);
    let obs = table(vec![
        with("Param A", Cell::Int(1), Cell::Unknown, Cell::Unknown, None),
        with("PARAM A:", Cell::Int(2), Cell::Unknown, Cell::Unknown, None),
    ]);
    let p = pair_tables(&pred, &obs);
    assert_eq!(p.verdicts[0].class, MatchClass::Normalized);
    assert_eq!(p.verdicts[0].observed.as_deref(), Some("Param A"));
    assert_eq!(p.verdicts[0].default, Some(FieldVerdict::Same));
    assert_eq!(p.unmatched_observed, vec!["PARAM A:".to_string()]);
}

#[test]
fn a_superset_of_flags_differs() {
    let mk = |f: u32| {
        table(vec![with(
            "Param A",
            Cell::Unknown,
            Cell::Unknown,
            Cell::Unknown,
            Some(f),
        )])
    };
    let v = &pair_tables(&mk(0x1f), &mk(0x01)).verdicts[0];
    assert_eq!(v.flags, Some(FieldVerdict::Differ));
    assert_eq!(v.flag_diff, Some(ParamFlags(0x1e)));
    let v = &pair_tables(&mk(0x01), &mk(0x1f)).verdicts[0];
    assert_eq!(v.flags, Some(FieldVerdict::Differ));
    assert_eq!(v.flag_diff, Some(ParamFlags(0x1e)));
    let v = &pair_tables(&mk(0), &mk(0x01)).verdicts[0];
    assert_eq!(v.flags, Some(FieldVerdict::Differ));
}

#[test]
fn flag_bits_keep_all_thirty_two_bits() {
    assert_eq!(ParamFlags(0xffff_ffff).bits(), u32::MAX);
    assert_eq!(ParamFlags(0x1234_5678).bits(), 0x1234_5678);
    let t = table(vec![with(
        "Param A",
        Cell::Unknown,
        Cell::Unknown,
        Cell::Unknown,
        Some(0x1234_5678),
    )]);
    assert_eq!(
        t.serialize(),
        "table v2\n?\tParam A\tfloat\t?\t?\t?\t0x12345678\t?\t?\t?\n"
    );
    let back = Table::parse(&t.serialize()).unwrap();
    assert_eq!(back.params[0].flags, Some(ParamFlags(0x1234_5678)));
}

#[test]
fn kind_names_are_whole_words() {
    for s in ["fl", "floatx", "flo at", "in", "intt", "groups", "colo"] {
        assert_eq!(ParamKind::parse(s), None, "{s:?}");
    }
}

#[test]
fn unknown_is_only_the_unknown_variant() {
    assert!(!Cell::Float(f64::INFINITY).is_unknown());
    assert!(!Cell::Float(f64::NEG_INFINITY).is_unknown());
    assert!(!Cell::Float(-f64::NAN).is_unknown());
    assert!(!Cell::Int(i64::MIN).is_unknown());
    assert!(!Cell::Int(i64::MAX).is_unknown());
}

#[test]
fn nan_payloads_differ_in_the_verdict() {
    let one = |c: Cell| table(vec![with("Param A", c, Cell::Unknown, Cell::Unknown, None)]);
    let other = Cell::Float(f64::from_bits(f64::NAN.to_bits() ^ 1));
    let v = &pair_tables(&one(Cell::Float(f64::NAN)), &one(other)).verdicts[0];
    assert_eq!(v.default, Some(FieldVerdict::Differ));
}

#[test]
fn kind_cell_is_case_sensitive_in_a_table() {
    assert_eq!(
        err(&row(["A", "Int", "?", "?", "?", "?"])).1,
        "unknown kind \"Int\""
    );
}

#[test]
fn flags_cell_is_not_padded() {
    assert!(Table::parse(&row(["A", "int", "?", "?", "?", " 0x1"])).is_err());
    assert!(Table::parse(&row(["A", "int", "?", "?", "?", "0x1 "])).is_err());
    assert!(Table::parse(&row(["A", "int", "?", "?", "?", "0x 1"])).is_err());
}

#[test]
fn text_float_special_values() {
    let t = Table::parse(&row(["A", "float", "f:-0.0", "f:inf", "f:-inf", "?"])).unwrap();
    assert!(t.params[0].default.bits_eq(&Cell::Float(-0.0)));
    assert!(t.params[0].min.bits_eq(&Cell::Float(f64::INFINITY)));
    assert!(t.params[0].max.bits_eq(&Cell::Float(f64::NEG_INFINITY)));
}

#[test]
fn nan_payload_round_trip() {
    let odd = f64::from_bits(0x7ff8_0000_0000_0123);
    let t = table(vec![with(
        "Param A",
        Cell::Float(odd),
        Cell::Float(f64::from_bits(0x7ff0_0000_0000_0001)),
        Cell::Float(f64::from_bits(0xfff8_0000_0000_0001)),
        None,
    )]);
    assert_eq!(
        t.serialize(),
        "table v2\n?\tParam A\tfloat\tb:7ff8000000000123\tb:7ff0000000000001\tb:fff8000000000001\t?\t?\t?\t?\n"
    );
    let back = Table::parse(&t.serialize()).unwrap();
    assert!(back.params[0].default.bits_eq(&Cell::Float(odd)));
}

#[test]
fn predicted_name_is_kept_as_written() {
    let p = pair_tables(&plain(&["Param  A:"]), &plain(&["param a"]));
    assert_eq!(p.verdicts[0].class, MatchClass::Normalized);
    assert_eq!(p.verdicts[0].predicted, "Param  A:");
}

#[test]
fn leading_colon_is_kept() {
    assert_eq!(normalize_name(":Name"), ":name");
    assert_eq!(normalize_name("::Name:"), "::name");
}

#[test]
fn double_cr_is_not_stripped() {
    assert!(Table::parse("table v1\nA\tint\t?\t?\t?\t?\r\r\n").is_err());
}
