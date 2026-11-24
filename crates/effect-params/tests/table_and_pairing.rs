use effect_params::*;

fn def(name: &str, kind: ParamKind, d: Cell, lo: Cell, hi: Cell, f: Option<u32>) -> ParamDef {
    ParamDef {
        name: name.into(),
        kind,
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

#[test]
fn round_trip_keeps_every_bit() {
    let t = Table {
        params: vec![
            def(
                "Param A",
                ParamKind::Float,
                Cell::Float(0.1),
                Cell::Float(-0.0),
                Cell::Float(f64::INFINITY),
                Some(5),
            ),
            def(
                "Param B",
                ParamKind::Int,
                Cell::Int(-7),
                Cell::Unknown,
                Cell::Int(9),
                None,
            ),
            def(
                "Param C",
                ParamKind::Checkbox,
                Cell::Int(1),
                Cell::Unknown,
                Cell::Unknown,
                Some(0),
            ),
        ],
    };
    let text = t.serialize();
    let back = Table::parse(&text).unwrap();
    assert_eq!(back.serialize(), text);
    match back.params[0].min {
        Cell::Float(v) => assert_eq!(v.to_bits(), (-0.0f64).to_bits()),
        _ => panic!("wrong cell"),
    }
    assert!(back.params[1].min.is_unknown());
    assert_eq!(back.params[2].flags, Some(ParamFlags::NONE));
}

#[test]
fn parse_accepts_decimal_and_comments() {
    let text = "# note\n\ntable v1\nParam A\tfloat\tf:0.5\t?\tf:2\t0x10\n";
    let t = Table::parse(text).unwrap();
    assert_eq!(t.params.len(), 1);
    assert!(matches!(t.params[0].default, Cell::Float(v) if v == 0.5));
    assert_eq!(t.params[0].flags, Some(ParamFlags(0x10)));
}

#[test]
fn parse_rejects_bad_input() {
    assert_eq!(Table::parse("").unwrap_err().line, 1);
    assert_eq!(Table::parse("wrong\n").unwrap_err().line, 1);
    assert!(Table::parse("table v1\nA\tfloat\t?\t?\n").is_err());
    assert!(Table::parse("table v1\nA\tnope\t?\t?\t?\t?\n").is_err());
    assert!(Table::parse("table v1\nA\tint\ti:x\t?\t?\t?\n").is_err());
    assert!(Table::parse("table v1\nA\tint\tb:12\t?\t?\t?\n").is_err());
    assert!(Table::parse("table v1\nA\tint\t?\t?\t?\t12\n").is_err());
    assert!(Table::parse("table v1\n\tint\t?\t?\t?\t?\n").is_err());
}

#[test]
fn kind_names_round_trip() {
    for k in ParamKind::ALL {
        assert_eq!(ParamKind::parse(k.as_str()), Some(k));
    }
    assert_eq!(ParamKind::parse("Float"), None);
}

#[test]
fn flags_helpers() {
    let a = ParamFlags::SUPERVISED.union(ParamFlags::HIDDEN);
    assert!(a.contains(ParamFlags::HIDDEN));
    assert!(!a.contains(ParamFlags::COLLAPSED));
    assert_eq!(a.diff(ParamFlags::HIDDEN), ParamFlags::SUPERVISED);
}

#[test]
fn normalize_rules() {
    assert_eq!(normalize_name("  Param   A: "), "param a");
    assert_eq!(normalize_name("Create Things..."), "create things");
    assert_eq!(normalize_name("X...:"), "x");
    assert_eq!(normalize_name("Same"), normalize_name("sAME"));
    assert_ne!(normalize_name("Param A"), normalize_name("Param B"));
}

#[test]
fn pairing_classes_and_fields() {
    let pred = Table {
        params: vec![
            def(
                "Param A",
                ParamKind::Float,
                Cell::Float(1.0),
                Cell::Float(0.0),
                Cell::Float(10.0),
                Some(1),
            ),
            def(
                "param b:",
                ParamKind::Int,
                Cell::Int(3),
                Cell::Unknown,
                Cell::Int(4),
                Some(2),
            ),
            def(
                "Param C",
                ParamKind::Button,
                Cell::Unknown,
                Cell::Unknown,
                Cell::Unknown,
                None,
            ),
        ],
    };
    let obs = Table {
        params: vec![
            def(
                "Param B",
                ParamKind::Int,
                Cell::Int(3),
                Cell::Int(0),
                Cell::Int(5),
                Some(2),
            ),
            def(
                "Param A",
                ParamKind::Float,
                Cell::Float(1.0),
                Cell::Float(-0.0),
                Cell::Float(10.0),
                Some(3),
            ),
            def(
                "Param Z",
                ParamKind::Layer,
                Cell::Unknown,
                Cell::Unknown,
                Cell::Unknown,
                None,
            ),
        ],
    };
    let p = pair_tables(&pred, &obs);
    assert_eq!(p.count(MatchClass::Exact), 1);
    assert_eq!(p.count(MatchClass::Normalized), 1);
    assert_eq!(p.count(MatchClass::Unmatched), 1);
    assert_eq!(p.unmatched_observed, vec!["Param Z".to_string()]);

    let a = &p.verdicts[0];
    assert_eq!(a.class, MatchClass::Exact);
    assert_eq!(a.kind, Some(FieldVerdict::Same));
    assert_eq!(a.default, Some(FieldVerdict::Same));
    // 0.0 against -0.0 differs as bits.
    assert_eq!(a.min, Some(FieldVerdict::Differ));
    assert_eq!(a.max, Some(FieldVerdict::Same));
    assert_eq!(a.flags, Some(FieldVerdict::Differ));
    assert_eq!(a.flag_diff, Some(ParamFlags(2)));

    let b = &p.verdicts[1];
    assert_eq!(b.class, MatchClass::Normalized);
    assert_eq!(b.observed.as_deref(), Some("Param B"));
    assert_eq!(b.min, Some(FieldVerdict::Unscored));
    assert_eq!(b.max, Some(FieldVerdict::Differ));
    assert_eq!(b.flags, Some(FieldVerdict::Same));
    assert_eq!(b.flag_diff, Some(ParamFlags::NONE));

    let c = &p.verdicts[2];
    assert_eq!(c.class, MatchClass::Unmatched);
    assert!(c.observed.is_none() && c.kind.is_none() && c.default.is_none());
}

#[test]
fn exact_claims_before_normalized_and_one_to_one() {
    let pred = Table {
        params: vec![
            def(
                "param a",
                ParamKind::Int,
                Cell::Unknown,
                Cell::Unknown,
                Cell::Unknown,
                None,
            ),
            def(
                "Param A",
                ParamKind::Int,
                Cell::Unknown,
                Cell::Unknown,
                Cell::Unknown,
                None,
            ),
        ],
    };
    let obs = Table {
        params: vec![def(
            "Param A",
            ParamKind::Int,
            Cell::Unknown,
            Cell::Unknown,
            Cell::Unknown,
            None,
        )],
    };
    let p = pair_tables(&pred, &obs);
    assert_eq!(p.verdicts[0].class, MatchClass::Unmatched);
    assert_eq!(p.verdicts[1].class, MatchClass::Exact);
    assert!(p.unmatched_observed.is_empty());
}

#[test]
fn observed_unknown_and_kind_mismatch() {
    let pred = Table {
        params: vec![def(
            "Param A",
            ParamKind::Float,
            Cell::Float(2.0),
            Cell::Unknown,
            Cell::Unknown,
            Some(1),
        )],
    };
    let obs = Table {
        params: vec![def(
            "Param A",
            ParamKind::Int,
            Cell::Unknown,
            Cell::Unknown,
            Cell::Unknown,
            None,
        )],
    };
    let v = &pair_tables(&pred, &obs).verdicts[0];
    assert_eq!(v.kind, Some(FieldVerdict::Differ));
    assert_eq!(v.default, Some(FieldVerdict::ObservedUnknown));
    assert_eq!(v.flags, Some(FieldVerdict::ObservedUnknown));
    assert_eq!(v.flag_diff, None);
}

#[test]
fn int_and_float_cells_never_equal() {
    let pred = Table {
        params: vec![def(
            "Param A",
            ParamKind::Int,
            Cell::Int(1),
            Cell::Unknown,
            Cell::Unknown,
            None,
        )],
    };
    let obs = Table {
        params: vec![def(
            "Param A",
            ParamKind::Int,
            Cell::Float(1.0),
            Cell::Unknown,
            Cell::Unknown,
            None,
        )],
    };
    assert_eq!(
        pair_tables(&pred, &obs).verdicts[0].default,
        Some(FieldVerdict::Differ)
    );
}

#[test]
fn contains_is_subset_not_intersection() {
    let two = ParamFlags::SUPERVISED.union(ParamFlags::COLLAPSED);
    let other = ParamFlags::COLLAPSED.union(ParamFlags::HIDDEN);
    assert!(!two.contains(other));
    assert!(other.contains(ParamFlags::COLLAPSED));
    assert!(!ParamFlags::NONE.contains(ParamFlags::SUPERVISED));
    assert!(ParamFlags::NONE.contains(ParamFlags::NONE));
}

#[test]
fn flag_constants_are_distinct_single_bits() {
    let all = [
        ParamFlags::SUPERVISED,
        ParamFlags::COLLAPSED,
        ParamFlags::HIDDEN,
        ParamFlags::NOT_ANIMATABLE,
        ParamFlags::REDRAW_ON_CHANGE,
    ];
    for (i, a) in all.iter().enumerate() {
        assert_eq!(a.bits().count_ones(), 1);
        for b in &all[i + 1..] {
            assert_ne!(a, b);
            assert!(!a.contains(*b));
        }
    }
    assert_eq!(ParamFlags::HIDDEN.bits(), 4);
    assert_eq!(ParamFlags::NOT_ANIMATABLE.bits(), 8);
    assert_eq!(
        ParamFlags::HIDDEN.diff(ParamFlags::NOT_ANIMATABLE),
        ParamFlags(12)
    );
}

#[test]
fn unknown_predicted_flags_are_unscored() {
    let pred = Table {
        params: vec![def(
            "Param A",
            ParamKind::Int,
            Cell::Unknown,
            Cell::Unknown,
            Cell::Unknown,
            None,
        )],
    };
    let obs = Table {
        params: vec![def(
            "Param A",
            ParamKind::Int,
            Cell::Unknown,
            Cell::Unknown,
            Cell::Unknown,
            Some(6),
        )],
    };
    let v = &pair_tables(&pred, &obs).verdicts[0];
    assert_eq!(v.flags, Some(FieldVerdict::Unscored));
    assert_eq!(v.flag_diff, None);
}

#[test]
fn integer_sign_matters() {
    let mk = |d: i64| Table {
        params: vec![def(
            "Param A",
            ParamKind::Int,
            Cell::Int(d),
            Cell::Int(-d),
            Cell::Unknown,
            None,
        )],
    };
    let v = &pair_tables(&mk(5), &mk(-5)).verdicts[0];
    assert_eq!(v.default, Some(FieldVerdict::Differ));
    assert_eq!(v.min, Some(FieldVerdict::Differ));
    let w = &pair_tables(&mk(-5), &mk(-5)).verdicts[0];
    assert_eq!(w.default, Some(FieldVerdict::Same));
    assert!(!Cell::Int(-1).bits_eq(&Cell::Int(1)));
    assert!(Cell::Int(0).bits_eq(&Cell::Int(0)));
}

#[test]
fn flag_high_bits_are_compared() {
    let mk = |f: u32| Table {
        params: vec![def(
            "Param A",
            ParamKind::Int,
            Cell::Unknown,
            Cell::Unknown,
            Cell::Unknown,
            Some(f),
        )],
    };
    for (a, b) in [(0x01u32, 0x11u32), (0x00, 0x100), (0x0f, 0x8000_000f)] {
        let v = &pair_tables(&mk(a), &mk(b)).verdicts[0];
        assert_eq!(v.flags, Some(FieldVerdict::Differ), "{a:x} vs {b:x}");
        assert_eq!(v.flag_diff, Some(ParamFlags(a ^ b)));
    }
    let same = &pair_tables(&mk(0x110), &mk(0x110)).verdicts[0];
    assert_eq!(same.flags, Some(FieldVerdict::Same));
}

#[test]
fn parse_rejects_too_many_cells() {
    assert!(Table::parse("table v1\nA\tint\t?\t?\t?\t?\n").is_ok());
    assert!(Table::parse("table v1\nA\tint\t?\t?\t?\t?\t?\t?\t?\n").is_err());
    assert!(Table::parse("table v1\nA\tint\t?\t?\t?\t?\t?\t?\n").is_err());
}

/// The first fenced block of the reference page that starts with the magic line (comment lines
/// and blank lines before it are allowed).
fn first_table_block(page: &str) -> String {
    let mut in_block = false;
    let mut block = String::new();
    for line in page.lines() {
        if line.starts_with("```") {
            if in_block {
                let first = block
                    .lines()
                    .find(|l| !l.trim().is_empty() && !l.starts_with('#'));
                if first.map(str::trim) == Some("table v1") {
                    return block;
                }
                block.clear();
            }
            in_block = !in_block;
        } else if in_block {
            block.push_str(line);
            block.push('\n');
        }
    }
    panic!("no fenced table block in the reference page");
}

#[test]
fn example_parses() {
    let page = include_str!("../../../docs/reference.md");
    let block = first_table_block(page);
    let t = Table::parse(&block).expect("the example table on the reference page must parse");
    // The page shows two rows: Param A and Param B.
    assert_eq!(t.params.len(), 2);
    assert_eq!(t.params[0].name, "Param A");
    assert_eq!(t.params[1].name, "Param B");
}

#[test]
fn serialized_table_round_trips_but_hand_written_text_does_not() {
    // Text written by the serializer comes back byte for byte.
    let t = Table {
        params: vec![
            def(
                "Param A",
                ParamKind::Float,
                Cell::Float(0.5),
                Cell::Unknown,
                Cell::Float(2.0),
                Some(0x10),
            ),
            def(
                "Param B",
                ParamKind::Int,
                Cell::Int(1),
                Cell::Unknown,
                Cell::Unknown,
                None,
            ),
        ],
    };
    let written = t.serialize();
    let again = Table::parse(&written).unwrap().serialize();
    assert_eq!(again, written);

    // A hand-written table with a comment, a blank line and an f: cell is read fine but is not
    // written back as the same text: the comment and the blank line are dropped and the float
    // becomes a b: cell.
    let hand = "# note\n\ntable v1\nParam A\tfloat\tf:0.5\t?\tf:2\t0x10\n";
    let rewritten = Table::parse(hand).unwrap().serialize();
    assert_ne!(rewritten, hand);
    assert!(!rewritten.contains('#'));
    assert!(!rewritten.contains("f:"));
}
