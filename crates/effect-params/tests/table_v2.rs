//! Table format version 2: ids, raw host words, empty names, layer default none, and the
//! id keyed pairing. Made up data only.

use effect_params::*;

fn def(id: Option<i64>, name: &str, kind: ParamKind) -> ParamDef {
    let mut d = ParamDef::new(name, kind);
    d.id = id;
    d
}

fn t(rows: Vec<ParamDef>) -> Table {
    Table { params: rows }
}

fn err(text: &str) -> String {
    Table::parse(text).unwrap_err().message
}

fn v2row(cells: [&str; 10]) -> String {
    format!("table v2\n{}\n", cells.join("\t"))
}

const OK: [&str; 10] = ["i:7", "Gain", "float", "?", "?", "?", "?", "?", "?", "?"];

fn with(i: usize, v: &str) -> String {
    let mut c = OK;
    c[i] = v;
    v2row(c)
}

#[test]
fn serializer_writes_v2_magic_and_ten_cells() {
    let text = t(vec![def(Some(5), "Gain", ParamKind::Float)]).serialize();
    assert_eq!(text.lines().next(), Some("table v2"));
    assert_eq!(text.lines().nth(1).unwrap().split('\t').count(), 10);
    assert_eq!(text, "table v2\ni:5\tGain\tfloat\t?\t?\t?\t?\t?\t?\t?\n");
}

#[test]
fn v2_round_trip_keeps_every_new_cell() {
    let mut a = def(Some(-12), "Mix", ParamKind::Popup);
    a.default = Cell::Int(2);
    a.flags = Some(ParamFlags(0x5));
    a.host_flags = Some(0x2300_0011);
    a.host_ui_flags = Some(0x7a1);
    a.extra = Extra::Choices(vec!["x".into(), "y".into()]);
    let mut b = def(Some(9), "", ParamKind::Button);
    b.extra = Extra::Label("Go".into());
    let mut c = def(None, "Free", ParamKind::Layer);
    c.default = Cell::None;
    let tab = t(vec![a, b, c]);
    let text = tab.serialize();
    let back = Table::parse(&text).unwrap();
    assert_eq!(back.serialize(), text);
    assert_eq!(back.params[0].id, Some(-12));
    assert_eq!(back.params[0].host_flags, Some(0x2300_0011));
    assert_eq!(back.params[0].host_ui_flags, Some(0x7a1));
    assert_eq!(back.params[1].name, "");
    assert_eq!(back.params[1].id, Some(9));
    assert!(matches!(back.params[2].default, Cell::None));
    assert_eq!(back.params[2].id, None);
}

#[test]
fn v1_files_still_load_with_new_cells_unknown() {
    let tab =
        Table::parse("table v1\nA\tint\ti:1\t?\t?\t0x3\nB\tpopup\t?\t?\t?\t?\tc:a|b\n").unwrap();
    assert_eq!(tab.params.len(), 2);
    for p in &tab.params {
        assert_eq!(p.id, None);
        assert_eq!(p.host_flags, None);
        assert_eq!(p.host_ui_flags, None);
    }
    assert_eq!(tab.params[0].flags, Some(ParamFlags(3)));
    assert_eq!(
        tab.params[1].extra,
        Extra::Choices(vec!["a".into(), "b".into()])
    );
}

#[test]
fn v1_still_refuses_empty_name_and_none_cell() {
    assert_eq!(err("table v1\n\tint\t?\t?\t?\t?\n"), "empty name");
    assert!(Table::parse("table v1\nA\tlayer\tnone\t?\t?\t?\n").is_err());
}

#[test]
fn v1_to_v2_conversion_is_stable() {
    let v1 = "table v1\nA\tfloat\tb:3ff0000000000000\t?\t?\t0x1\n";
    let v2 = Table::parse(v1).unwrap().serialize();
    assert!(v2.starts_with("table v2\n"));
    let again = Table::parse(&v2).unwrap();
    assert_eq!(again.serialize(), v2);
    assert_eq!(again.params[0].flags, Some(ParamFlags(1)));
}

#[test]
fn magic_line_names_the_version() {
    assert!(err("table v3\n").contains("table v2"));
    assert!(Table::parse("# note\n\ntable v2\n")
        .unwrap()
        .params
        .is_empty());
    assert!(Table::parse("table v2\nbad\n").is_err());
    assert_eq!(err("table v2\nbad\n"), "expected 10 cells, found 1");
}

#[test]
fn v2_needs_exactly_ten_cells() {
    assert_eq!(
        err("table v2\ni:1\tA\tint\t?\t?\t?\t?\t?\t?\n"),
        "expected 10 cells, found 9"
    );
    assert_eq!(
        err("table v2\ni:1\tA\tint\t?\t?\t?\t?\t?\t?\t?\t?\n"),
        "expected 10 cells, found 11"
    );
}

#[test]
fn empty_name_is_legal_with_an_id() {
    let tab = Table::parse(&with(1, "")).unwrap();
    assert_eq!(tab.params[0].name, "");
    assert_eq!(tab.params[0].id, Some(7));
}

#[test]
fn empty_name_without_id_is_refused() {
    let mut c = OK;
    c[0] = "?";
    c[1] = "";
    assert_eq!(err(&v2row(c)), "empty name needs a known id");
}

#[test]
fn name_escapes_round_trip() {
    let tab = t(vec![def(None, "a\tb\nc\\d\re", ParamKind::Int)]);
    let text = tab.serialize();
    assert_eq!(text.lines().count(), 2);
    let back = Table::parse(&text).unwrap();
    assert_eq!(back.params[0].name, "a\tb\nc\\d\re");
    assert!(err(&with(1, "a\\q")).contains("bad escape"));
    assert!(Table::parse(&with(1, "a\\")).is_err());
}

#[test]
fn id_cell_forms() {
    assert_eq!(Table::parse(&with(0, "?")).unwrap().params[0].id, None);
    assert_eq!(Table::parse(&with(0, "i:0")).unwrap().params[0].id, Some(0));
    assert_eq!(
        Table::parse(&with(0, "i:-5")).unwrap().params[0].id,
        Some(-5)
    );
    for bad in [
        "7",
        "0x7",
        "i:",
        "i:x",
        "i:1.5",
        "",
        "I:7",
        "i:99999999999999999999",
    ] {
        assert!(Table::parse(&with(0, bad)).is_err(), "{bad:?}");
    }
}

#[test]
fn duplicate_known_ids_are_refused_and_unknown_may_repeat() {
    let a = "i:4\tA\tint\t?\t?\t?\t?\t?\t?\t?";
    let b = "i:4\tB\tint\t?\t?\t?\t?\t?\t?\t?";
    let e = Table::parse(&format!("table v2\n{a}\n{b}\n")).unwrap_err();
    assert_eq!((e.line, e.message.as_str()), (3, "duplicate id 4"));
    let u = "?\tA\tint\t?\t?\t?\t?\t?\t?\t?";
    assert_eq!(
        Table::parse(&format!("table v2\n{u}\n{u}\n"))
            .unwrap()
            .params
            .len(),
        2
    );
}

#[test]
fn host_word_cells() {
    let tab = Table::parse(&{
        let mut c = OK;
        c[7] = "0xffffffff";
        c[8] = "0x0";
        v2row(c)
    })
    .unwrap();
    assert_eq!(tab.params[0].host_flags, Some(u32::MAX));
    assert_eq!(tab.params[0].host_ui_flags, Some(0));
    assert!(tab.serialize().contains("\t0xffffffff\t0x0\t"));
    for i in [7usize, 8] {
        for bad in [
            "0x",
            "0x100000000",
            "0x000000001",
            "0x-1",
            "0x+1",
            "12",
            "0xzz",
            "0X1",
            " 0x1",
            "",
        ] {
            assert!(Table::parse(&with(i, bad)).is_err(), "{i} {bad:?}");
        }
        assert!(Table::parse(&with(i, "0xAbC")).is_ok());
    }
    assert!(err(&with(7, "9")).contains("host flags"));
    assert!(err(&with(8, "9")).contains("host ui flags"));
}

#[test]
fn own_flags_and_host_words_are_independent_cells() {
    let tab = Table::parse(&{
        let mut c = OK;
        c[6] = "0x1";
        c[7] = "0x2";
        c[8] = "0x4";
        v2row(c)
    })
    .unwrap();
    let p = &tab.params[0];
    assert_eq!(p.flags, Some(ParamFlags(1)));
    assert_eq!(p.host_flags, Some(2));
    assert_eq!(p.host_ui_flags, Some(4));
}

#[test]
fn layer_default_none_is_distinct_from_unknown_and_zero() {
    let mut c = OK;
    c[2] = "layer";
    c[3] = "none";
    let tab = Table::parse(&v2row(c)).unwrap();
    assert!(matches!(tab.params[0].default, Cell::None));
    assert!(!tab.params[0].default.is_unknown());
    assert!(!Cell::None.bits_eq(&Cell::Unknown));
    assert!(!Cell::None.bits_eq(&Cell::Int(0)));
    assert!(Cell::None.bits_eq(&Cell::None));
    assert!(tab.serialize().contains("\tlayer\tnone\t"));
}

#[test]
fn none_is_refused_outside_a_layer_default() {
    // Every kind except layer refuses `none` as a default (audit mutant M11).
    for k in ParamKind::ALL.iter().filter(|k| **k != ParamKind::Layer) {
        let mut c = OK;
        c[2] = k.as_str();
        c[3] = "none";
        assert!(
            Table::parse(&v2row(c))
                .unwrap_err()
                .message
                .contains("layer"),
            "kind {}",
            k.as_str()
        );
    }
    assert!(err(&with(3, "none")).contains("layer"));
    let mut c = OK;
    c[2] = "layer";
    for i in [4usize, 5] {
        c[i] = "none";
        assert!(Table::parse(&v2row(c)).is_err(), "cell {i}");
        c[i] = "?";
    }
}

#[test]
fn every_kind_round_trips_in_v2() {
    for (n, k) in ParamKind::ALL.iter().enumerate() {
        let mut d = def(
            Some(n as i64),
            if *k == ParamKind::GroupEnd { "" } else { "Row" },
            *k,
        );
        d.host_flags = Some(n as u32);
        let text = t(vec![d]).serialize();
        let back = Table::parse(&text).unwrap();
        assert_eq!(back.params[0].kind, *k);
        assert_eq!(back.serialize(), text);
    }
}

#[test]
fn arbitrary_nodata_and_group_end_rows() {
    let text = "table v2\n\
        i:1\tBlob\tarbitrary\t?\t?\t?\t?\t0x0\t0x0\td:\n\
        i:2\t\tnodata\t?\t?\t?\t?\t0x2\t0x4\t-\n\
        i:3\t\tgroup_end\t?\t?\t?\t?\t?\t?\t?\n";
    let tab = Table::parse(text).unwrap();
    assert_eq!(tab.params[0].kind, ParamKind::Arbitrary);
    assert_eq!(tab.params[0].extra, Extra::Data(vec![]));
    assert_eq!(tab.params[1].kind, ParamKind::NoData);
    assert_eq!(tab.params[2].kind, ParamKind::GroupEnd);
    assert_eq!(tab.serialize(), text);
}

#[test]
fn per_kind_extras_survive_in_v2() {
    let mut p = def(Some(1), "Pop", ParamKind::Popup);
    p.extra = Extra::Choices(vec!["a b".into(), "".into()]);
    let mut s = def(Some(2), "Sl", ParamKind::Float);
    s.extra = Extra::slider(2, 0x8, 0.0, 100.0);
    let mut q = def(Some(3), "Pt", ParamKind::Point);
    q.extra = Extra::point(10.0, 20.0);
    let mut b = def(Some(4), "", ParamKind::Button);
    b.extra = Extra::Label("Run".into());
    let tab = t(vec![p, s, q, b]);
    let back = Table::parse(&tab.serialize()).unwrap();
    for (x, y) in tab.params.iter().zip(&back.params) {
        assert_eq!(x.extra, y.extra);
    }
}

// pairing

fn pair1(p: ParamDef, o: ParamDef) -> ParamVerdict {
    pair_tables(&t(vec![p]), &t(vec![o])).verdicts.remove(0)
}

#[test]
fn rows_with_equal_ids_pair_by_id_even_with_different_names() {
    let v = pair1(
        def(Some(3), "Old Name", ParamKind::Int),
        def(Some(3), "New Name", ParamKind::Int),
    );
    assert_eq!(v.class, MatchClass::ById);
    assert_eq!(v.key, Some(PairKey::Id));
    assert_eq!(v.id, Some(FieldVerdict::Same));
    assert_eq!(v.name, Some(FieldVerdict::Differ));
    assert_eq!(v.id_predicted, Some(3));
}

#[test]
fn empty_named_rows_pair_by_id_without_skipping() {
    let p = t(vec![
        def(Some(1), "", ParamKind::Button),
        def(Some(2), "", ParamKind::Button),
        def(Some(3), "", ParamKind::GroupEnd),
    ]);
    let o = t(vec![
        def(Some(3), "", ParamKind::GroupEnd),
        def(Some(2), "", ParamKind::Button),
        def(Some(1), "", ParamKind::Button),
    ]);
    let r = pair_tables(&p, &o);
    assert_eq!(r.count(MatchClass::ById), 3);
    assert!(r.unmatched_observed.is_empty());
    assert!(r
        .verdicts
        .iter()
        .all(|v| v.kind == Some(FieldVerdict::Same) && v.name == Some(FieldVerdict::Same)));
}

#[test]
fn empty_names_without_ids_never_pair() {
    let r = pair_tables(
        &t(vec![def(None, "", ParamKind::Button)]),
        &t(vec![def(None, "", ParamKind::Button)]),
    );
    assert_eq!(r.verdicts[0].class, MatchClass::Unmatched);
    assert_eq!(r.verdicts[0].key, None);
    assert_eq!(r.unmatched_observed.len(), 1);
}

#[test]
fn name_pairing_is_used_when_an_id_is_missing_on_one_side() {
    let v = pair1(
        def(Some(3), "Gain", ParamKind::Float),
        def(None, "gain:", ParamKind::Float),
    );
    assert_eq!(v.class, MatchClass::Normalized);
    assert_eq!(v.key, Some(PairKey::Name));
    assert_eq!(v.id, Some(FieldVerdict::ObservedUnknown));
    let v = pair1(
        def(None, "Gain", ParamKind::Float),
        def(Some(3), "Gain", ParamKind::Float),
    );
    assert_eq!(v.class, MatchClass::Exact);
    assert_eq!(v.id, Some(FieldVerdict::Unscored));
}

#[test]
fn different_known_ids_block_a_name_match() {
    let r = pair_tables(
        &t(vec![def(Some(1), "Same", ParamKind::Int)]),
        &t(vec![def(Some(2), "Same", ParamKind::Int)]),
    );
    assert_eq!(r.verdicts[0].class, MatchClass::Unmatched);
    assert_eq!(r.unmatched_observed, vec!["Same".to_string()]);
}

#[test]
fn id_pairing_beats_name_pairing_for_near_duplicates() {
    let p = t(vec![
        def(Some(10), "Flip", ParamKind::Checkbox),
        def(Some(11), "Flip", ParamKind::Checkbox),
    ]);
    let o = t(vec![
        def(Some(11), "Flip", ParamKind::Checkbox),
        def(Some(10), "Flip", ParamKind::Checkbox),
    ]);
    let r = pair_tables(&p, &o);
    assert_eq!(r.count(MatchClass::ById), 2);
    assert_eq!(r.verdicts[0].observed.as_deref(), Some("Flip"));
}

#[test]
fn mixed_keys_report_each_key_used() {
    let p = t(vec![
        def(Some(1), "A", ParamKind::Int),
        def(None, "B", ParamKind::Int),
        def(Some(3), "", ParamKind::Int),
    ]);
    let o = t(vec![
        def(Some(1), "A2", ParamKind::Int),
        def(None, "B", ParamKind::Int),
        def(Some(4), "", ParamKind::Int),
    ]);
    let r = pair_tables(&p, &o);
    assert_eq!(r.verdicts[0].key, Some(PairKey::Id));
    assert_eq!(r.verdicts[1].key, Some(PairKey::Name));
    assert_eq!(r.verdicts[2].key, None);
    assert_eq!(r.unmatched_observed.len(), 1);
}

#[test]
fn host_word_verdicts_cover_all_four_outcomes() {
    let mk = |f: Option<u32>, u: Option<u32>| {
        let mut d = def(Some(1), "A", ParamKind::Int);
        d.host_flags = f;
        d.host_ui_flags = u;
        d
    };
    let v = pair1(mk(Some(1), Some(2)), mk(Some(1), Some(3)));
    assert_eq!(v.host_flags, Some(FieldVerdict::Same));
    assert_eq!(v.host_ui_flags, Some(FieldVerdict::Differ));
    let v = pair1(mk(Some(1), None), mk(None, Some(3)));
    assert_eq!(v.host_flags, Some(FieldVerdict::ObservedUnknown));
    assert_eq!(v.host_ui_flags, Some(FieldVerdict::Unscored));
    let v = pair1(mk(Some(0), Some(0)), mk(Some(0), Some(0)));
    assert_eq!(v.host_flags, Some(FieldVerdict::Same));
    assert_eq!(v.flags, Some(FieldVerdict::Unscored));
}

#[test]
fn layer_default_none_verdicts() {
    let mk = |c: Cell| {
        let mut d = def(Some(1), "L", ParamKind::Layer);
        d.default = c;
        d
    };
    assert_eq!(
        pair1(mk(Cell::None), mk(Cell::None)).default,
        Some(FieldVerdict::Same)
    );
    assert_eq!(
        pair1(mk(Cell::None), mk(Cell::Int(0))).default,
        Some(FieldVerdict::Differ)
    );
    assert_eq!(
        pair1(mk(Cell::None), mk(Cell::Unknown)).default,
        Some(FieldVerdict::ObservedUnknown)
    );
    assert_eq!(
        pair1(mk(Cell::Unknown), mk(Cell::None)).default,
        Some(FieldVerdict::Unscored)
    );
}

#[test]
fn unmatched_row_carries_its_id_and_no_verdicts() {
    let r = pair_tables(
        &t(vec![def(Some(77), "Lonely", ParamKind::Int)]),
        &Table::default(),
    );
    let v = &r.verdicts[0];
    assert_eq!(v.id_predicted, Some(77));
    assert!(v.id.is_none() && v.name.is_none() && v.host_flags.is_none());
    assert!(v.host_ui_flags.is_none());
}

#[test]
fn each_observed_row_is_claimed_once_across_keys() {
    let p = t(vec![
        def(Some(1), "X", ParamKind::Int),
        def(None, "X", ParamKind::Int),
    ]);
    let o = t(vec![def(Some(1), "X", ParamKind::Int)]);
    let r = pair_tables(&p, &o);
    assert_eq!(r.verdicts[0].class, MatchClass::ById);
    assert_eq!(r.verdicts[1].class, MatchClass::Unmatched);
}

#[test]
fn carriage_return_in_a_name_is_escaped_in_the_text() {
    let text = t(vec![def(Some(1), "a\rb", ParamKind::Int)]).serialize();
    assert!(text.contains("a\\rb"));
    assert!(!text.contains('\r'));
}

#[test]
fn an_observed_empty_name_is_never_a_name_match() {
    let r = pair_tables(
        &t(vec![def(None, "...", ParamKind::Int)]),
        &t(vec![def(Some(5), "", ParamKind::Int)]),
    );
    assert_eq!(r.verdicts[0].class, MatchClass::Unmatched);
}

#[test]
fn name_keyed_verdict_still_names_the_predicted_id() {
    let v = pair1(
        def(Some(3), "Gain", ParamKind::Float),
        def(None, "Gain", ParamKind::Float),
    );
    assert_eq!(v.id_predicted, Some(3));
    assert_eq!(v.key, Some(PairKey::Name));
}

#[test]
fn a_repeated_predicted_id_claims_the_observed_row_only_once() {
    // Hand built: parse refuses a repeated id, a caller can still build one (audit mutant M17).
    let p = t(vec![
        def(Some(4), "First", ParamKind::Int),
        def(Some(4), "Second", ParamKind::Int),
    ]);
    let o = t(vec![def(Some(4), "Other", ParamKind::Int)]);
    let r = pair_tables(&p, &o);
    assert_eq!(r.verdicts[0].class, MatchClass::ById);
    assert_eq!(r.verdicts[1].class, MatchClass::Unmatched);
    assert!(r.unmatched_observed.is_empty());
}
