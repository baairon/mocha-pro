use plugin_commands::*;

/// A nonzero engine value used where the engine did not answer success.
const ENGINE_ABSENT: i32 = 83;

fn rd() -> Readings {
    Readings::default()
}
fn st() -> RoleTable {
    RoleTable::sample()
}
fn idx(t: &RoleTable, r: ParamRole) -> i32 {
    t.index_of(r).unwrap()
}

// ---- dispatch
#[test]
fn dispatch_own_rows() {
    let own = [
        (CMD_ABOUT, Command::About),
        (CMD_GLOBAL_SETUP, Command::GlobalSetup),
        (CMD_GLOBAL_SETDOWN, Command::GlobalSetdown),
        (CMD_PARAMS_SETUP, Command::ParamsSetup),
        (CMD_SEQUENCE_SETUP, Command::SequenceSetup),
        (CMD_SEQUENCE_RESETUP, Command::SequenceResetup),
        (CMD_SEQUENCE_SETDOWN, Command::SequenceSetdown),
        (CMD_DO_DIALOG, Command::DoDialog),
        (CMD_RENDER, Command::Render),
        (CMD_USER_CHANGED_PARAM, Command::UserChangedParam),
        (CMD_UPDATE_PARAMS_UI, Command::UpdateParamsUi),
        (CMD_EVENT, Command::Event),
        (CMD_ARBITRARY_CALLBACK, Command::ArbitraryCallback),
        (CMD_SMART_PRE_RENDER, Command::SmartPreRender),
        (CMD_SMART_RENDER, Command::SmartRender),
    ];
    for (c, k) in own {
        assert_eq!(command_from_code(c), Slot::Held(k), "code {c}");
    }
    assert_eq!(own.len(), 15);
}
#[test]
fn dispatch_default_rows() {
    let own: Vec<u32> = [
        CMD_ABOUT,
        CMD_GLOBAL_SETUP,
        CMD_GLOBAL_SETDOWN,
        CMD_PARAMS_SETUP,
        CMD_SEQUENCE_SETUP,
        CMD_SEQUENCE_RESETUP,
        CMD_SEQUENCE_SETDOWN,
        CMD_DO_DIALOG,
        CMD_RENDER,
        CMD_USER_CHANGED_PARAM,
        CMD_UPDATE_PARAMS_UI,
        CMD_EVENT,
        CMD_ARBITRARY_CALLBACK,
        CMD_SMART_PRE_RENDER,
        CMD_SMART_RENDER,
    ]
    .to_vec();
    for c in 0u32..=CMD_SMART_RENDER {
        assert_eq!(
            command_from_code(c).is_empty(),
            !own.contains(&c),
            "code {c}"
        );
    }
}
#[test]
fn dispatch_above_table() {
    for c in [
        CMD_SMART_RENDER + 1,
        CMD_SMART_RENDER + 2,
        i32::MAX as u32,
        u32::MAX,
    ] {
        assert_eq!(command_from_code(c), Slot::Empty);
    }
    assert_eq!(DEFAULT_RETURN, OK);
}
#[test]
fn never_sent_set() {
    for c in 0u32..=CMD_SMART_RENDER + 4 {
        assert_eq!(never_sent(c), CMD_NEVER_SENT.contains(&c), "{c}");
        if never_sent(c) {
            assert!(command_from_code(c).is_empty());
        }
    }
    assert_eq!(CMD_NEVER_SENT.len(), 3);
}
#[test]
fn thrown_code_reading() {
    assert_eq!(thrown_return(ERR_SUITE, &rd()), ERR_SUITE);
    let r = Readings {
        thrown_code_is_returned: false,
        ..rd()
    };
    assert_eq!(thrown_return(ERR_SUITE, &r), OK);
}
#[test]
fn wrapper_returns() {
    assert_eq!(engine_wrapper_return(0), OK);
    for e in [1, ENGINE_ABSENT, ERR_NULL_ARGS, -1, 99] {
        assert_eq!(engine_wrapper_return(e), ERR_ENGINE);
    }
}
#[test]
fn code_constants_are_distinct() {
    let c = [
        ERR_NULL_ARGS,
        ERR_DRAWBOT_NULL,
        ERR_LAUNCH,
        ERR_SUITE,
        ERR_ENGINE,
    ];
    for i in 0..c.len() {
        for j in 0..i {
            assert_ne!(c[i], c[j]);
        }
    }
}

// ---- the refusals
#[test]
fn dialog_is_refused_without_a_code() {
    assert_eq!(dialog_answer(), Err(Error::NotRebuilt));
    assert_eq!(Error::NotRebuilt.to_string(), "not rebuilt");
}
#[test]
fn only_the_dialog_command_carries_a_refusal() {
    let all = [
        Command::About,
        Command::GlobalSetup,
        Command::GlobalSetdown,
        Command::ParamsSetup,
        Command::SequenceSetup,
        Command::SequenceResetup,
        Command::SequenceSetdown,
        Command::DoDialog,
        Command::Render,
        Command::UserChangedParam,
        Command::UpdateParamsUi,
        Command::Event,
        Command::ArbitraryCallback,
        Command::SmartPreRender,
        Command::SmartRender,
    ];
    for c in all {
        let want = if c == Command::DoDialog {
            Slot::Held(Error::NotRebuilt)
        } else {
            Slot::Empty
        };
        assert_eq!(command_refusal(c), want, "{c:?}");
    }
    let code = command_from_code(CMD_DO_DIALOG).unwrap();
    assert_eq!(command_refusal(code), Slot::Held(Error::NotRebuilt));
}
#[test]
fn index_without_role_is_refused() {
    let t = st();
    let lo = t.params.iter().map(|p| p.0).min().unwrap();
    let hi = t.params.iter().map(|p| p.0).max().unwrap();
    for i in [i32::MIN, lo - 1, lo - 2, hi + 1, hi + 2, i32::MAX] {
        assert_eq!(param_action(&t, i), Err(Error::NotRebuilt), "index {i}");
    }
}
#[test]
fn removed_role_stays_removed() {
    // A table with no entry for an index refuses it, whatever its neighbours do.
    let mut t = st();
    t.params.retain(|(i, _)| *i != 2703);
    assert_eq!(param_action(&t, 2703), Err(Error::NotRebuilt));
    assert_eq!(param_action(&t, 2704), Ok(ParamAction::ActionB));
}
#[test]
fn refusal_text_names_nothing() {
    let s = Error::NotRebuilt.to_string();
    assert!(!s.contains(|c: char| c.is_ascii_digit()));
    assert_eq!(s, "not rebuilt");
}

// ---- parameter changes
#[test]
fn param_table_rows() {
    use ParamAction::*;
    let t = st();
    let rows = [
        (2701, SentinelTracker),
        (2702, SentinelTracker),
        (2703, ActionA),
        (2704, ActionB),
        (2705, PresetChange),
        (2706, TargetMoved),
        (2707, TargetMoved),
        (2708, TargetMoved),
        (2709, ActionC),
        (2710, ActionD),
        (2711, ActionE),
        (2712, ActionF),
        (2713, None),
        (2714, None),
    ];
    assert_eq!(rows.len(), t.params.len());
    for (i, a) in rows {
        assert_eq!(param_action(&t, i), Ok(a), "index {i}");
    }
}
#[test]
fn role_lookup_both_ways() {
    let t = st();
    for (i, r) in t.params.clone() {
        assert_eq!(t.role_of(i), Slot::Held(r));
        assert_eq!(t.role_of(t.index_of(r).unwrap()), Slot::Held(r));
    }
    assert_eq!(t.role_of(-1), Slot::Empty);
    let empty = RoleTable {
        params: vec![],
        ..st()
    };
    assert_eq!(empty.index_of(ParamRole::PresetSelector), Slot::Empty);
}
#[test]
fn sentinel_machine() {
    let r = rd();
    let t = st();
    let a = idx(&t, ParamRole::SentinelA) as u32;
    let b = idx(&t, ParamRole::SentinelB) as u32;
    assert_eq!(SENTINEL_VACANT, 1u32 << 31);
    assert_eq!(sentinel_step(SENTINEL_VACANT, a, &r), a);
    assert_eq!(sentinel_step(SENTINEL_VACANT, b, &r), b);
    assert_eq!(sentinel_step(a, a, &r), a);
    assert_eq!(sentinel_step(a, b, &r), SENTINEL_VACANT);
    assert_eq!(sentinel_step(b, a, &r), SENTINEL_VACANT);
    assert_eq!(sentinel_step(b, b, &r), b);
}
#[test]
fn sentinel_recorded_flag() {
    assert!(!sentinel_recorded(SENTINEL_VACANT));
    assert!(sentinel_recorded(0));
    assert!(sentinel_recorded(7));
}
#[test]
fn sentinel_switch_off() {
    let r = Readings {
        sentinel_tracker_enabled: false,
        ..rd()
    };
    let t = st();
    let a = idx(&t, ParamRole::SentinelA) as u32;
    let b = idx(&t, ParamRole::SentinelB) as u32;
    assert_eq!(sentinel_step(SENTINEL_VACANT, a, &r), SENTINEL_VACANT);
    assert_eq!(sentinel_step(a, b, &r), a);
}

// ---- preset loading
fn load(t: &RoleTable, v: i32) -> [ParamWrite; 3] {
    match preset_load(t, v, &rd()).unwrap() {
        PresetLoad::Load(w) => w,
        other => panic!("expected a loading result, got {other:?}"),
    }
}
#[test]
fn every_preset_loads_in_order() {
    let t = st();
    let targets = [
        idx(&t, ParamRole::PresetTargetA),
        idx(&t, ParamRole::PresetTargetB),
        idx(&t, ParamRole::PresetTargetC),
    ];
    assert!(!t.presets.is_empty());
    for (n, p) in t.presets.iter().enumerate() {
        let w = load(&t, n as i32 + 1);
        assert_eq!([w[0].index, w[1].index, w[2].index], targets);
        assert_eq!(
            [
                w[0].value.to_bits(),
                w[1].value.to_bits(),
                w[2].value.to_bits()
            ],
            [
                f64::from(p.a).to_bits(),
                f64::from(p.b).to_bits(),
                f64::from(p.c).to_bits()
            ],
            "preset {n}"
        );
    }
}
#[test]
fn preset_positions_are_not_shifted() {
    let t = st();
    let first = load(&t, 1);
    let last = load(&t, t.presets.len() as i32);
    assert_eq!(first[2].value, f64::from(t.presets[0].c));
    assert_eq!(last[2].value, f64::from(t.presets[t.presets.len() - 1].c));
    assert_ne!(first[2].value, last[2].value);
}
#[test]
fn preset_zero_values_are_positive_zero() {
    let t = st();
    let n = t.presets.iter().position(|p| p.a == 0.0).unwrap();
    assert_eq!(load(&t, n as i32 + 1)[0].value.to_bits(), 0);
}
#[test]
fn preset_free_noop() {
    let t = st();
    assert_eq!(
        preset_load(&t, t.free_position, &rd()),
        Ok(PresetLoad::NoOp)
    );
}
#[test]
fn preset_out_of_range_unspecified() {
    let t = st();
    let past = t.presets.len() as i32 + 1;
    for v in [0, past, -1, i32::MIN, i32::MAX] {
        assert!(
            matches!(preset_load(&t, v, &rd()), Err(Error::Unspecified(_))),
            "{v}"
        );
    }
}
#[test]
fn preset_out_of_range_reading() {
    let t = st();
    let r = Readings {
        preset_out_of_range_noop: true,
        ..rd()
    };
    assert_eq!(preset_load(&t, 0, &r), Ok(PresetLoad::NoOp));
    assert_eq!(
        preset_load(&t, t.presets.len() as i32 + 1, &r),
        Ok(PresetLoad::NoOp)
    );
    assert!(matches!(preset_load(&t, 1, &r), Ok(PresetLoad::Load(_))));
}
#[test]
fn preset_free_checked_before_range() {
    // A free position inside the preset range still loads nothing.
    let mut t = st();
    t.free_position = 2;
    assert_eq!(preset_load(&t, 2, &rd()), Ok(PresetLoad::NoOp));
    assert!(matches!(preset_load(&t, 3, &rd()), Ok(PresetLoad::Load(_))));
}
#[test]
fn preset_missing_target_role_is_refused() {
    let mut t = st();
    t.params.retain(|(_, r)| *r != ParamRole::PresetTargetB);
    assert_eq!(preset_load(&t, 1, &rd()), Err(Error::NotRebuilt));
}
#[test]
fn preset_load_error_flow() {
    assert_eq!(preset_load_outcome([0, 0, 0]), (0, [true, true, true]));
    assert_eq!(preset_load_outcome([5, 0, 0]), (5, [true, false, false]));
    assert_eq!(preset_load_outcome([0, 7, 9]), (7, [true, true, false]));
    assert_eq!(preset_load_outcome([0, 0, 3]), (3, [true, true, true]));
}
#[test]
fn target_moved_flip() {
    let t = st();
    for v in [1, 2, 0, -1, t.free_position + 1, t.free_position - 1] {
        assert!(target_moved_sets_free(&t, v));
    }
    assert!(!target_moved_sets_free(&t, t.free_position));
}

// ---- interface rules, with an independent slow reference
fn reference(t: &RoleTable, i: &UiInputs) -> [Slot<bool>; 8] {
    let mut out = [Slot::Empty; 8];
    out[0] = Slot::Held(i.gate_a != 0);
    out[1] = Slot::Held(i.gate_a != 0);
    let mp = &t.selector_b_positions;
    let mut pair_on = false;
    for q in mp.pair {
        if q == i.selector_b {
            pair_on = true;
        }
    }
    out[2] = Slot::Held(!pair_on);
    out[3] = Slot::Held(i.selector_b != mp.single);
    out[4] = Slot::Held(i.selector_b != mp.single);
    if i.flags_w & t.paired_capability_mask != 0 {
        let mut mode = 0;
        for (k, m) in &t.selector_c_modes {
            if *k == i.selector_c {
                mode = *m;
            }
        }
        out[5] = Slot::Held(mode == 1);
        out[6] = Slot::Held(mode == 1);
        out[7] = Slot::Held(mode != 0);
    }
    out
}
fn flat(s: &UiState) -> [Slot<bool>; 8] {
    let (a, b, c) = match s.paired {
        Slot::Held(p) => (
            Slot::Held(p.dep_a_shown),
            Slot::Held(p.dep_b_shown),
            Slot::Held(p.dep_c_shown),
        ),
        Slot::Empty => (Slot::Empty, Slot::Empty, Slot::Empty),
    };
    [
        Slot::Held(s.gate_dep_a_disabled),
        Slot::Held(s.gate_dep_b_disabled),
        Slot::Held(s.selector_b_dep_c_disabled),
        Slot::Held(s.selector_b_dep_a_disabled),
        Slot::Held(s.selector_b_dep_b_disabled),
        a,
        b,
        c,
    ]
}
#[test]
fn ui_matches_reference_exhaustive() {
    let t = st();
    let mut sb_values: Vec<i32> = vec![-1, 0];
    sb_values.extend(t.selector_b_positions.pair);
    sb_values.push(t.selector_b_positions.single);
    sb_values.push(t.selector_b_positions.single + 1);
    let mut sc_values: Vec<i32> = vec![-1, 0, i32::MAX];
    sc_values.extend(t.selector_c_modes.iter().map(|(k, _)| *k));
    sc_values.push(t.selector_c_modes[0].0 + 50);
    let bit_sets = [
        0u64,
        t.paired_capability_mask,
        !t.paired_capability_mask,
        u64::MAX,
    ];
    for ga in [0, 1, 2, -1] {
        for &sb in &sb_values {
            for &v in &sc_values {
                for &w in &bit_sets {
                    let i = UiInputs {
                        gate_a: ga,
                        selector_b: sb,
                        selector_c: v,
                        flags_w: w,
                    };
                    assert_eq!(flat(&update_ui(&t, &i)), reference(&t, &i), "{i:?}");
                }
            }
        }
    }
}
fn base(t: &RoleTable) -> UiInputs {
    UiInputs {
        gate_a: 0,
        selector_b: t.selector_b_positions.single,
        selector_c: t.selector_c_modes[0].0,
        flags_w: 0,
    }
}
#[test]
fn ui_gate_rows() {
    let t = st();
    let off = update_ui(&t, &base(&t));
    assert!(!off.gate_dep_a_disabled && !off.gate_dep_b_disabled);
    let on = update_ui(
        &t,
        &UiInputs {
            gate_a: 1,
            ..base(&t)
        },
    );
    assert!(on.gate_dep_a_disabled && on.gate_dep_b_disabled);
}
#[test]
fn ui_selector_b_rows() {
    let t = st();
    let m = t.selector_b_positions;
    let s = update_ui(&t, &base(&t));
    assert!(
        !s.selector_b_dep_a_disabled && !s.selector_b_dep_b_disabled && s.selector_b_dep_c_disabled
    );
    for q in m.pair {
        let s = update_ui(
            &t,
            &UiInputs {
                selector_b: q,
                ..base(&t)
            },
        );
        assert!(!s.selector_b_dep_c_disabled, "selector b {q}");
        assert!(s.selector_b_dep_a_disabled && s.selector_b_dep_b_disabled);
    }
    for other in [m.single + 10, m.single - 1, 0] {
        assert!(
            update_ui(
                &t,
                &UiInputs {
                    selector_b: other,
                    ..base(&t)
                }
            )
            .selector_b_dep_c_disabled
        );
    }
}
#[test]
fn ui_paired_gate() {
    let t = st();
    let off = UiInputs {
        flags_w: !t.paired_capability_mask,
        ..base(&t)
    };
    assert_eq!(update_ui(&t, &off).paired, Slot::Empty);
    let on = UiInputs {
        flags_w: t.paired_capability_mask,
        ..base(&t)
    };
    assert!(update_ui(&t, &on).paired.is_held());
}
#[test]
fn ui_selector_c_modes() {
    let t = st();
    for (k, m) in t.selector_c_modes.clone() {
        assert_eq!(selector_c_mode(&t, k), m);
    }
    assert_eq!(selector_c_mode(&t, 0), MODE_PLAIN);
    assert_eq!(selector_c_mode(&t, i32::MIN), MODE_PLAIN);
    assert_eq!(selector_c_mode(&t, i32::MAX), MODE_PLAIN);
}
#[test]
fn ui_paired_modes() {
    let t = st();
    let at = |mode: i32| {
        let key = t
            .selector_c_modes
            .iter()
            .find(|(_, m)| *m == mode)
            .unwrap()
            .0;
        update_ui(
            &t,
            &UiInputs {
                selector_c: key,
                flags_w: t.paired_capability_mask,
                ..base(&t)
            },
        )
        .paired
        .unwrap()
    };
    let plain = at(MODE_PLAIN);
    assert!(!plain.dep_a_shown && !plain.dep_b_shown && !plain.dep_c_shown);
    let paired = at(MODE_PAIRED);
    assert!(paired.dep_a_shown && paired.dep_b_shown && paired.dep_c_shown);
    for other in [2, 3] {
        let o = at(other);
        assert!(
            !o.dep_a_shown && !o.dep_b_shown && o.dep_c_shown,
            "mode {other}"
        );
    }
}
#[test]
fn ui_flag_helpers() {
    assert_eq!(enable_rule_flags(0, true), PUI_DISABLED);
    assert_eq!(
        enable_rule_flags(PUI_DISABLED | PUI_INVISIBLE, false),
        PUI_INVISIBLE
    );
    assert_eq!(enable_rule_flags(PUI_DISABLED, true), PUI_DISABLED);
    assert_eq!(shown_rule_flags(0, false), PUI_INVISIBLE);
    assert_eq!(
        shown_rule_flags(PUI_DISABLED | PUI_INVISIBLE, true),
        PUI_DISABLED
    );
    assert_eq!(shown_rule_flags(0, true), 0);
    assert_eq!(PUI_DISABLED & PUI_INVISIBLE, 0);
}
#[test]
fn ui_update_call_only_on_difference() {
    assert!(!update_needs_host_call(PUI_DISABLED, PUI_DISABLED));
    assert!(update_needs_host_call(PUI_DISABLED, 0));
}

// ---- events
#[test]
fn event_types() {
    assert_eq!(event_kind(EVENT_DO_CLICK), EventKind::DoClick);
    assert_eq!(event_kind(EVENT_DRAG), EventKind::Drag);
    assert_eq!(event_kind(EVENT_DRAW), EventKind::Draw);
    assert_eq!(event_kind(EVENT_ADJUST_CURSOR), EventKind::AdjustCursor);
    for t in [0, 1, 5, 6, 7, 8, 10, 11, 12, -1] {
        assert_eq!(event_kind(t), EventKind::Ignored, "{t}");
    }
}
#[test]
fn hover_edges() {
    let t = st();
    let (w, h) = t.control_size;
    let (l, tp) = (10, 20);
    let a = AREA_CONTROL;
    assert!(!hover_hit(&t, a, l, tp, l + 1, tp + 2));
    assert!(hover_hit(&t, a, l, tp, l + 2, tp + 2));
    assert!(hover_hit(&t, a, l, tp, l + w - 1, tp + h - 1));
    assert!(!hover_hit(&t, a, l, tp, l + w, tp + h - 1));
    assert!(!hover_hit(&t, a, l, tp, l + w - 1, tp + h));
    assert!(!hover_hit(&t, a, l, tp, l + w - 1, tp + 1));
    assert!(hover_hit(&t, a, l, tp, l + w - 1, tp + 2));
}
#[test]
fn click_edges() {
    let t = st();
    let (w, h) = t.control_size;
    let (l, tp) = (10, 20);
    let a = AREA_CONTROL;
    assert!(!click_hit(&t, a, l, tp, l, tp + 1));
    assert!(click_hit(&t, a, l, tp, l + 1, tp + 1));
    assert!(click_hit(&t, a, l, tp, l + w, tp + h));
    assert!(!click_hit(&t, a, l, tp, l + w + 1, tp + h));
    assert!(!click_hit(&t, a, l, tp, l + w, tp + h + 1));
    assert!(!click_hit(&t, a, l, tp, l + 1, tp));
}
#[test]
fn hit_needs_control_area() {
    let t = st();
    for a in [
        AREA_CONTROL - 2,
        AREA_CONTROL - 1,
        AREA_CONTROL + 1,
        -AREA_CONTROL,
    ] {
        assert!(!hover_hit(&t, a, 0, 0, 50, 20));
        assert!(!click_hit(&t, a, 0, 0, 50, 20));
    }
}
#[test]
fn hit_negative_and_extreme_origin() {
    let t = st();
    let (w, _) = t.control_size;
    assert!(click_hit(&t, AREA_CONTROL, -100, -50, -90, -40));
    assert!(click_hit(
        &t,
        AREA_CONTROL,
        i32::MAX - w - 5,
        0,
        i32::MAX - 5,
        10
    ));
    assert!(!click_hit(&t, AREA_CONTROL, i32::MAX - 1, 0, i32::MAX, 0));
}
#[test]
fn adjust_cursor_flag() {
    let t = st();
    assert!(adjust_cursor_hover(&t, AREA_CONTROL, 0, 0, 50, 20));
    assert!(!adjust_cursor_hover(
        &t,
        AREA_CONTROL,
        0,
        0,
        t.control_size.0 * 4,
        20
    ));
}
#[test]
fn click_effects() {
    let t = st();
    let a = AREA_CONTROL;
    let e = do_click_effect(&t, a, 0, 0, 5, 5, false);
    assert!(e.send_drag && e.invalidate_rect && e.hover == Slot::Held(true));
    assert_eq!(e.evt_out_or, EVT_HANDLED_UPDATE);
    let p = do_click_effect(&t, a, 0, 0, 5, 5, true);
    assert!(!p.invalidate_rect && p.evt_out_or == EVT_HANDLED_UPDATE);
    let miss = do_click_effect(&t, a, 0, 0, t.control_size.0 * 4, 5, false);
    assert!(
        !miss.send_drag && miss.hover.is_empty() && miss.evt_out_or == 0 && !miss.invalidate_rect
    );
}
#[test]
fn drag_effects() {
    let t = st();
    let a = AREA_CONTROL;
    let e = drag_effect(&t, a, 0, 0, 5, 5, 1, false);
    assert!(e.launch_mode0 && e.invalidate_rect && e.assign_changed_flags);
    assert_eq!(
        (e.hover, e.evt_out_or),
        (Slot::Held(false), EVT_HANDLED_UPDATE)
    );
    assert!(!drag_effect(&t, a, 0, 0, 5, 5, 1, true).invalidate_rect);
    let none = drag_effect(&t, a, 0, 0, 5, 5, 0, false);
    assert!(!none.launch_mode0 && none.hover.is_empty() && none.evt_out_or == 0);
    let out = drag_effect(&t, a, 0, 0, t.control_size.0 * 4, 5, 9, false);
    assert!(!out.launch_mode0 && !out.assign_changed_flags);
    assert!(!drag_effect(&t, a + 1, 0, 0, 5, 5, 9, false).launch_mode0);
}
#[test]
fn draw_plan_cases() {
    let t = st();
    let r = rd();
    let a = AREA_CONTROL;
    let p = draw_plan(&t, a, false, true, true, 7, 9, false, &r).unwrap();
    assert_eq!(p.bitmap, t.bitmaps.normal);
    assert_eq!(
        p.origin.map(f32::to_bits),
        [14.0f32.to_bits(), 18.0f32.to_bits()]
    );
    assert_eq!(p.scale.to_bits(), 0.5f32.to_bits());
    assert_eq!(
        (p.restore_scale, p.evt_out_flags_assign),
        (Slot::Empty, EVT_HANDLED)
    );
    let h = draw_plan(&t, a, true, true, true, 0, 0, true, &r).unwrap();
    assert_eq!(
        (h.bitmap, h.restore_scale),
        (t.bitmaps.hover, Slot::Held(2.0))
    );
    assert!(draw_plan(&t, a + 1, false, true, true, 0, 0, false, &r).is_empty());
    assert!(draw_plan(&t, a, true, true, false, 0, 0, false, &r).is_empty());
    assert!(draw_plan(&t, a, false, false, true, 0, 0, false, &r).is_empty());
}
#[test]
fn draw_scale_reading() {
    let t = st();
    let r = Readings {
        draw_scale: 1.0,
        ..rd()
    };
    let p = draw_plan(&t, AREA_CONTROL, false, true, true, 0, 0, false, &r).unwrap();
    assert_eq!(p.scale, 1.0);
}
#[test]
fn draw_origin_positive_zero() {
    let t = st();
    let p = draw_plan(&t, AREA_CONTROL, false, true, true, 0, 0, false, &rd()).unwrap();
    assert_eq!(p.origin[0].to_bits(), 0);
}

// ---- launch, about
#[test]
fn guard_states() {
    let t = st();
    let d = t.dialog_results;
    assert_eq!(guard_state(&t, d.go_quiet), (1, true));
    assert_eq!(guard_state(&t, d.go), (1, false));
    assert_eq!(guard_state(&t, d.go_full), (2, false));
    for r in [2, 8, 0, -1, d.go_full + 1, d.go_quiet - 1] {
        assert_eq!(guard_state(&t, r), (0, false));
    }
}
#[test]
fn guard_dialog_condition() {
    assert!(guard_shows_dialog(false, 0.98, 1.0));
    assert!(guard_shows_dialog(false, 1.0, 0.5));
    assert!(!guard_shows_dialog(false, 0.99, 1.0));
    assert!(!guard_shows_dialog(false, 1.0, 0.99));
    assert!(!guard_shows_dialog(true, 0.1, 0.1));
}
#[test]
fn ratios() {
    assert_eq!(downsample_ratio(1, 0), 1.0);
    assert_eq!(downsample_ratio(1, 2).to_bits(), 0.5f64.to_bits());
    assert_eq!(downsample_ratio(3, 3), 1.0);
}
#[test]
fn launch_codes() {
    assert_eq!(launch_return(0), OK);
    assert_eq!(launch_return(1), OK);
    for e in [2, ENGINE_ABSENT, -1] {
        assert_eq!(launch_return(e), ERR_LAUNCH);
        assert!(launch_writes_message(e));
    }
    assert!(!launch_writes_message(0) && !launch_writes_message(1));
}
#[test]
fn launch_text_comes_from_the_table() {
    let mut t = st();
    assert_eq!(
        error_stream_text(&t),
        "sample: the interface did not start."
    );
    t.texts.error_stream = "other".to_string();
    assert_eq!(error_stream_text(&t), "other");
}
#[test]
fn about_cases() {
    assert_eq!(about_outcome(Slot::Held(0)), AboutOutcome::EngineHandled);
    assert_eq!(
        about_outcome(Slot::Held(ENGINE_ABSENT)),
        AboutOutcome::Fallback
    );
    assert_eq!(about_outcome(Slot::Empty), AboutOutcome::Fallback);
    let t = st();
    assert_eq!(about_text(&t, "Name", 7, 0), "sample Name / 7 / 0");
    let mut u = st();
    u.texts.about_template = "{num_b}|{num_a}|{name}|{name}".to_string();
    assert_eq!(about_text(&u, "N", 1, 2), "2|1|N|N");
    assert_eq!(about_refused_return(&rd()), ERR_SUITE);
    let r = Readings {
        about_throw_returns_code: false,
        ..rd()
    };
    assert_eq!(about_refused_return(&r), OK);
}
#[test]
fn sequence_helpers() {
    assert!(setdown_skips_registry("a_b"));
    assert!(!setdown_skips_registry("a-b"));
    assert_eq!(render_only_id("a-b-c"), "a_b_c");
    assert!(!setdown_skips_registry(""));
    assert!(setdown_skips_registry("_"));
    assert_eq!(render_only_id("abc"), "abc");
    assert_eq!(render_only_id("-"), "_");
    assert_eq!(about_text(&st(), "", -1, 0), "sample  / -1 / 0");
}
#[test]
fn sample_values_are_self_consistent() {
    let t = st();
    assert_eq!(t, RoleTable::sample());
    let mut ids: Vec<i32> = t.params.iter().map(|(i, _)| *i).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), t.params.len());
    assert!(!(1..=t.presets.len() as i32).contains(&t.free_position));
    assert_ne!(t.bitmaps.normal, t.bitmaps.hover);
}

// ---- about text templates and table checks
#[test]
fn name_with_placeholder_is_kept() {
    let t = st();
    assert_eq!(about_text(&t, "{num_a}", 1, 2), "sample {num_a} / 1 / 2");
    assert_eq!(about_text(&t, "{name}", 1, 2), "sample {name} / 1 / 2");
    assert_eq!(about_text(&t, "x", 1, 2), "sample x / 1 / 2");
}
#[test]
fn template_text_around_placeholders_is_kept() {
    let mut t = st();
    t.texts.about_template = "{ {name}{num_a}}{x} {num_b} {".to_string();
    assert_eq!(about_text(&t, "n", 4, 5), "{ n4}{x} 5 {");
}
#[test]
fn repeated_index_is_refused_by_check() {
    let t = st();
    assert_eq!(t.check(), Ok(()));
    let mut d = st();
    let i = d.params[0].0;
    d.params.push((i, ParamRole::NoAction));
    assert!(d.check().is_err());
}
#[test]
fn repeated_role_is_refused_by_check() {
    let mut t = st();
    let first = t
        .params
        .iter()
        .find(|p| p.1 != ParamRole::NoAction)
        .unwrap()
        .1;
    let hi = t.params.iter().map(|p| p.0).max().unwrap();
    t.params.push((hi + 1, first));
    assert!(t.check().is_err());
}
#[test]
fn repeated_no_action_rows_are_accepted_by_check() {
    let mut t = st();
    let hi = t.params.iter().map(|p| p.0).max().unwrap();
    t.params.push((hi + 1, ParamRole::NoAction));
    t.params.push((hi + 2, ParamRole::NoAction));
    assert_eq!(t.check(), Ok(()));
}
#[test]
fn repeated_index_with_the_same_role_is_refused_by_check() {
    let mut d = st();
    let hi = d.params.iter().map(|p| p.0).max().unwrap();
    d.params.push((hi + 1, ParamRole::NoAction));
    d.params.push((hi + 1, ParamRole::NoAction));
    assert!(d.check().is_err());
}
#[test]
fn repeated_row_next_to_its_twin_is_refused_by_check() {
    let mut d = st();
    let twin = d.params[0];
    d.params.push(twin);
    d.params[1..].rotate_right(1);
    assert!(d.check().is_err());
    let mut e = st();
    let at = e
        .params
        .iter()
        .position(|p| p.1 != ParamRole::NoAction)
        .unwrap();
    let hi = e.params.iter().map(|p| p.0).max().unwrap();
    let copy = (hi + 1, e.params[at].1);
    e.params.push(copy);
    e.params[at + 1..].rotate_right(1);
    assert!(e.check().is_err());
}
#[test]
fn repeated_index_refusal_is_the_unspecified_kind() {
    let mut d = st();
    let i = d.params[0].0;
    d.params.push((i, ParamRole::NoAction));
    assert!(matches!(d.check(), Err(Error::Unspecified(_))));
}
#[test]
fn about_text_writes_a_negative_minor_with_its_sign() {
    let t = st();
    assert_eq!(about_text(&t, "x", 1, -2), "sample x / 1 / -2");
    assert_eq!(about_text(&t, "x", -3, 2), "sample x / -3 / 2");
}
#[test]
fn about_text_keeps_a_word_that_starts_like_a_placeholder() {
    let mut t = st();
    t.texts.about_template = "{namex} {num_ab} {name} {num_a}{num_b}".to_string();
    assert_eq!(about_text(&t, "n", 4, 5), "{namex} {num_ab} n 45");
}
#[test]
fn negative_mode_code_shows_dependent_row_c() {
    let mut t = st();
    let pos = t.selector_c_modes[0].0 + 50;
    t.selector_c_modes.push((pos, -1));
    let u = update_ui(
        &t,
        &UiInputs {
            gate_a: 0,
            selector_b: 0,
            selector_c: pos,
            flags_w: t.paired_capability_mask,
        },
    );
    let p = u.paired.unwrap();
    assert!(p.dep_c_shown);
    assert!(!p.dep_a_shown);
    assert!(!p.dep_b_shown);
}
