use effect_params::*;

fn row(name: &str, kind: ParamKind, extra: Extra) -> ParamDef {
    let mut d = ParamDef::new(name, kind);
    d.extra = extra;
    d
}

fn one(extra: Extra) -> Table {
    Table {
        params: vec![row("Item", ParamKind::Popup, extra)],
    }
}

fn verdict(p: Extra, o: Extra) -> FieldVerdict {
    pair_tables(&one(p), &one(o)).verdicts[0].extra.unwrap()
}

#[test]
fn every_extra_round_trips() {
    let all = vec![
        Extra::Unknown,
        Extra::Absent,
        Extra::Choices(vec![]),
        Extra::Choices(vec!["one".into(), "two words".into(), "".into()]),
        Extra::Choices(vec!["".into()]),
        Extra::Choices(vec![
            "a|b".into(),
            "t\tab\nnl\r".into(),
            "back\\slash".into(),
        ]),
        Extra::slider(3, 0x6, -1.5, f64::INFINITY),
        Extra::slider(0, 0, -0.0, 0.0),
        Extra::point(0.25, -7.0),
        Extra::Data(vec![]),
        Extra::Data(vec![0, 1, 0xab, 0xff]),
        Extra::Label("Go now".into()),
        Extra::Label("".into()),
        Extra::Label("a|b\\".into()),
    ];
    for e in all {
        let t = one(e.clone());
        let text = t.serialize();
        let back = Table::parse(&text).unwrap();
        assert_eq!(back.params[0].extra, e, "{text:?}");
        assert_eq!(back.serialize(), text);
    }
}

#[test]
fn six_cell_rows_stay_unknown_and_unchanged() {
    let text = "table v1\nA\tfloat\t?\t?\t?\t?\n";
    let t = Table::parse(text).unwrap();
    assert!(t.params[0].extra.is_unknown());
    assert_eq!(
        t.serialize(),
        "table v2\n?\tA\tfloat\t?\t?\t?\t?\t?\t?\t?\n"
    );
    let t = one(Extra::Absent);
    assert_eq!(
        t.serialize().lines().nth(1).unwrap().split('\t').count(),
        10
    );
}

#[test]
fn exact_text_forms() {
    assert_eq!(
        Extra::Choices(vec!["x".into(), "y|".into()]).text(),
        "c:x|y\\p"
    );
    assert_eq!(Extra::Choices(vec!["".into()]).text(), "c:\\e");
    assert_eq!(Extra::Choices(vec![]).text(), "c:");
    assert_eq!(Extra::Data(vec![1, 0xa0]).text(), "d:01a0");
    assert_eq!(Extra::Label("q".into()).text(), "l:q");
    assert_eq!(
        Extra::slider(2, 0x10, 1.0, 2.0).text(),
        "s:2:0x10:3ff0000000000000:4000000000000000"
    );
    assert_eq!(
        Extra::point(1.0, 2.0).text(),
        "p:3ff0000000000000:4000000000000000"
    );
    assert_eq!(Extra::Absent.text(), "-");
    assert_eq!(Extra::Unknown.text(), "?");
}

#[test]
fn bad_extra_cells_are_rejected() {
    for bad in [
        "z:1",
        "c:a\\x",
        "l:\\",
        "d:abc",
        "d:zz",
        "d:+a",
        "d:-1",
        "s:1:0x+1:3ff0000000000000:3ff0000000000000",
        "s:1:0x:3ff0000000000000:3ff0000000000000",
        "p:+ff0000000000000:3ff0000000000000",
        "d:\u{e9}\u{e9}",
        "p:1:2",
        "p:3ff0000000000000",
        "p:3ff0000000000000:4000000000000000:0",
        "s:1:0x1:3ff0000000000000",
        "s:256:0x0:3ff0000000000000:3ff0000000000000",
        "s:1:5:3ff0000000000000:3ff0000000000000",
        "s:1:0xzz:3ff0000000000000:3ff0000000000000",
        "s:1:0x1:12:3ff0000000000000",
        "s:1:0x1:3ff0000000000000:12",
        "",
    ] {
        assert!(Extra::parse(bad).is_err(), "{bad:?}");
    }
    let e = Table::parse("table v1\nA\tpopup\t?\t?\t?\t?\tz:1\n").unwrap_err();
    assert_eq!(e.line, 2);
    assert!(Table::parse("table v1\nA\tpopup\t?\t?\t?\t?\t-\textra\n").is_err());
}

#[test]
fn new_kinds_parse_and_name() {
    for (n, k) in [
        ("group_end", ParamKind::GroupEnd),
        ("arbitrary", ParamKind::Arbitrary),
        ("nodata", ParamKind::NoData),
        ("group", ParamKind::Group),
    ] {
        assert_eq!(ParamKind::parse(n), Some(k));
        assert_eq!(k.as_str(), n);
    }
    assert_eq!(ParamKind::ALL.len(), 13);
    let mut names: Vec<_> = ParamKind::ALL.iter().map(|k| k.as_str()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 13);
}

#[test]
fn extra_verdicts() {
    let c = Extra::Choices(vec!["a".into(), "b".into()]);
    assert_eq!(verdict(c.clone(), c.clone()), FieldVerdict::Same);
    assert_eq!(
        verdict(c.clone(), Extra::Choices(vec!["a".into()])),
        FieldVerdict::Differ
    );
    assert_eq!(
        verdict(c.clone(), Extra::Choices(vec!["b".into(), "a".into()])),
        FieldVerdict::Differ
    );
    assert_eq!(verdict(Extra::Unknown, c.clone()), FieldVerdict::Unscored);
    assert_eq!(
        verdict(Extra::Unknown, Extra::Unknown),
        FieldVerdict::Unscored
    );
    assert_eq!(
        verdict(c.clone(), Extra::Unknown),
        FieldVerdict::ObservedUnknown
    );
    assert_eq!(verdict(Extra::Absent, Extra::Absent), FieldVerdict::Same);
    assert_eq!(verdict(Extra::Absent, c), FieldVerdict::Differ);
    assert_eq!(
        verdict(Extra::slider(2, 1, 0.0, 1.0), Extra::slider(3, 1, 0.0, 1.0)),
        FieldVerdict::Differ
    );
    assert_eq!(
        verdict(Extra::slider(2, 1, 0.0, 1.0), Extra::slider(2, 3, 0.0, 1.0)),
        FieldVerdict::Differ
    );
    assert_eq!(
        verdict(
            Extra::slider(2, 1, 0.0, 1.0),
            Extra::slider(2, 1, -0.0, 1.0)
        ),
        FieldVerdict::Differ
    );
    assert_eq!(
        verdict(Extra::slider(2, 1, 0.0, 1.0), Extra::slider(2, 1, 0.0, 2.0)),
        FieldVerdict::Differ
    );
    assert_eq!(
        verdict(Extra::point(1.0, 2.0), Extra::point(2.0, 1.0)),
        FieldVerdict::Differ
    );
    assert_eq!(
        verdict(Extra::point(1.0, 2.0), Extra::point(1.0, 3.0)),
        FieldVerdict::Differ
    );
    assert_eq!(
        verdict(Extra::Data(vec![1]), Extra::Data(vec![1, 0])),
        FieldVerdict::Differ
    );
    assert_eq!(
        verdict(Extra::Label("x".into()), Extra::Label("X".into())),
        FieldVerdict::Differ
    );
    assert_eq!(
        verdict(Extra::Label("x".into()), Extra::Data(vec![])),
        FieldVerdict::Differ
    );
}

#[test]
fn unmatched_row_has_no_extra_verdict() {
    let p = one(Extra::Absent);
    let o = Table {
        params: vec![row("Other", ParamKind::Popup, Extra::Absent)],
    };
    let r = pair_tables(&p, &o);
    assert_eq!(r.verdicts[0].extra, None);
}

#[test]
fn field_kinds_in_a_mixed_table_round_trip() {
    let t = Table {
        params: vec![
            row("G", ParamKind::Group, Extra::Absent),
            row("P", ParamKind::Point, Extra::point(0.5, 0.5)),
            row("A", ParamKind::Angle, Extra::Absent),
            row("D", ParamKind::Arbitrary, Extra::Data(vec![9, 8])),
            row("B", ParamKind::Button, Extra::Label("Run".into())),
            row("L", ParamKind::Layer, Extra::Absent),
            row("E", ParamKind::GroupEnd, Extra::Absent),
            row("N", ParamKind::NoData, Extra::Unknown),
        ],
    };
    let text = t.serialize();
    assert_eq!(Table::parse(&text).unwrap().serialize(), text);
    assert_eq!(Table::parse(&text).unwrap().params.len(), 8);
}

#[test]
fn new_rows_start_unknown_and_long_forms_fail() {
    assert!(ParamDef::new("N", ParamKind::Float).extra.is_unknown());
    assert!(Extra::parse("s:1:0x1:3ff0000000000000:3ff0000000000000:3ff0000000000000").is_err());
}

#[test]
fn explicit_question_mark_extra_parses_as_unknown() {
    assert_eq!(Extra::parse("?").unwrap(), Extra::Unknown);
    let t = Table::parse("table v1\nA\tpopup\t?\t?\t?\t?\t?\n").unwrap();
    assert_eq!(t.params[0].extra, Extra::Unknown);
    assert!(Extra::parse("??").is_err());
}

#[test]
fn slider_precision_with_spaces_is_refused() {
    for bad in [
        "s: 3:0x1:3ff0000000000000:3ff0000000000000",
        "s:3 :0x1:3ff0000000000000:3ff0000000000000",
    ] {
        assert!(Extra::parse(bad).is_err(), "{bad:?}");
    }
    assert!(Extra::parse("s:3:0x1:3ff0000000000000:3ff0000000000000").is_ok());
}
