use eye_layout::*;

fn rd() -> Readings {
    Readings::default()
}
fn nar(l: Layout, w: u32, h: u32, c: u32, s: bool) -> Rect {
    narrow(l, w, h, c, s, false, &rd()).unwrap()
}
/// (col first, col last, row first, row last), inclusive.
fn inc(r: Rect) -> (u32, u32, u32, u32) {
    (r.x0, r.x0 + r.w - 1, r.y0, r.y0 + r.h - 1)
}

macro_rules! eye {
    ($name:ident, $l:expr, $w:expr, $h:expr, $c:expr, $s:expr, $exp:expr) => {
        #[test]
        fn $name() {
            assert_eq!(inc(nar($l, $w, $h, $c, $s)), $exp);
        }
    };
}
use Layout::{SideBySide, Stacked};

// 8 by 4, every eye rectangle for both packed layouts and both exchange states
eye!(side_8x4_off_v0, SideBySide, 8, 4, 0, false, (0, 3, 0, 3));
eye!(side_8x4_off_v1, SideBySide, 8, 4, 1, false, (4, 7, 0, 3));
eye!(side_8x4_on_v0, SideBySide, 8, 4, 0, true, (4, 7, 0, 3));
eye!(side_8x4_on_v1, SideBySide, 8, 4, 1, true, (0, 3, 0, 3));
eye!(stack_8x4_off_v0, Stacked, 8, 4, 0, false, (0, 7, 0, 1));
eye!(stack_8x4_off_v1, Stacked, 8, 4, 1, false, (0, 7, 2, 3));
eye!(stack_8x4_on_v0, Stacked, 8, 4, 0, true, (0, 7, 2, 3));
eye!(stack_8x4_on_v1, Stacked, 8, 4, 1, true, (0, 7, 0, 1));
// odd vectors
eye!(side_5x4_off_v0, SideBySide, 5, 4, 0, false, (0, 1, 0, 3));
eye!(side_5x4_off_v1, SideBySide, 5, 4, 1, false, (3, 4, 0, 3));
eye!(side_5x4_on_v0, SideBySide, 5, 4, 0, true, (2, 3, 0, 3));
eye!(side_5x4_on_v1, SideBySide, 5, 4, 1, true, (1, 2, 0, 3));
eye!(stack_8x5_off_v0, Stacked, 8, 5, 0, false, (0, 7, 1, 2));
eye!(stack_8x5_off_v1, Stacked, 8, 5, 1, false, (0, 7, 2, 3));
eye!(stack_8x5_on_v0, Stacked, 8, 5, 0, true, (0, 7, 3, 4));
eye!(stack_8x5_on_v1, Stacked, 8, 5, 1, true, (0, 7, 0, 1));
// more odd sizes by hand
eye!(side_3x1_v0, SideBySide, 3, 1, 0, false, (0, 0, 0, 0));
eye!(side_3x2_on_v1, SideBySide, 3, 2, 1, true, (1, 1, 0, 1));
eye!(stack_2x3_off_v0, Stacked, 2, 3, 0, false, (0, 1, 1, 1));
eye!(stack_2x3_on_v1, Stacked, 2, 3, 1, true, (0, 1, 0, 0));

#[test]
fn eye_sizes() {
    assert_eq!(nar(SideBySide, 5, 4, 0, false).w, 2);
    assert_eq!(nar(SideBySide, 5, 4, 0, false).h, 4);
    assert_eq!(nar(Stacked, 8, 5, 0, false).h, 2);
    assert_eq!(nar(Stacked, 8, 5, 0, false).w, 8);
}

#[test]
fn width_one_side_by_side_empty_eye() {
    let r = nar(SideBySide, 1, 4, 1, false);
    assert_eq!((r.x0, r.w), (1, 0));
}

#[test]
fn whole_image_for_whole_and_two_layers() {
    for l in [Layout::Whole, Layout::TwoLayers] {
        for c in 0..2 {
            for s in [false, true] {
                assert_eq!(
                    nar(l, 7, 5, c, s),
                    Rect {
                        x0: 0,
                        y0: 0,
                        w: 7,
                        h: 5
                    }
                );
            }
        }
    }
}

#[test]
fn bottom_up_does_not_invert_stacked() {
    let r = narrow(Stacked, 8, 4, 0, false, true, &rd()).unwrap();
    assert_eq!(inc(r), (0, 7, 2, 3));
    let r = narrow(Stacked, 8, 4, 0, true, true, &rd()).unwrap();
    assert_eq!(inc(r), (0, 7, 0, 1));
}

#[test]
fn bottom_up_irrelevant_for_side_by_side() {
    for c in 0..2 {
        for s in [false, true] {
            assert_eq!(
                narrow(SideBySide, 9, 4, c, s, true, &rd()),
                narrow(SideBySide, 9, 4, c, s, false, &rd())
            );
        }
    }
}

#[test]
fn bad_eye_index_errors_by_default() {
    assert_eq!(
        narrow(SideBySide, 8, 4, 2, false, false, &rd()),
        Err(LayoutError::BadEyeIndex(2))
    );
}

#[test]
fn clamp_reading_clamps() {
    let r = Readings {
        clamp_eye_index: true,
        ..rd()
    };
    assert_eq!(
        narrow(SideBySide, 8, 4, 5, false, false, &r).unwrap(),
        nar(SideBySide, 8, 4, 1, false)
    );
}

// alternative reading
#[test]
fn eye_position_reading_side_by_side_5_on() {
    let r = Readings {
        odd_offset: OddOffset::ByEyePosition,
        ..rd()
    };
    let a = narrow(SideBySide, 5, 4, 0, true, false, &r).unwrap();
    let b = narrow(SideBySide, 5, 4, 1, true, false, &r).unwrap();
    assert_eq!(inc(a), (3, 4, 0, 3));
    assert_eq!(inc(b), (0, 1, 0, 3));
}
#[test]
fn eye_position_reading_stacked_5_off() {
    let r = Readings {
        odd_offset: OddOffset::ByEyePosition,
        ..rd()
    };
    let a = narrow(Stacked, 8, 5, 0, false, false, &r).unwrap();
    let b = narrow(Stacked, 8, 5, 1, false, false, &r).unwrap();
    assert_eq!(inc(a), (0, 7, 0, 1));
    assert_eq!(inc(b), (0, 7, 3, 4));
}
#[test]
fn readings_agree_on_even_sizes() {
    let alt = Readings {
        odd_offset: OddOffset::ByEyePosition,
        ..rd()
    };
    for l in [SideBySide, Stacked] {
        for c in 0..2 {
            for s in [false, true] {
                assert_eq!(
                    narrow(l, 8, 6, c, s, false, &rd()),
                    narrow(l, 8, 6, c, s, false, &alt)
                );
            }
        }
    }
}

// independent slow reference: integer arithmetic, written again from the description
fn reference(l: Layout, w: u32, h: u32, c: u32, s: bool) -> (u32, u32, u32, u32) {
    let sp = if l == Stacked { !s } else { s };
    let lead = if sp { 1 - c } else { c };
    if l == SideBySide {
        let mut x = (lead * w) / 2;
        if c == 1 && w % 2 == 1 {
            x += 1;
        }
        (x, w / 2, 0, h)
    } else {
        let mut y = ((1 - lead) * h) / 2;
        if c == 0 && h % 2 == 1 {
            y += 1;
        }
        (0, w, y, h / 2)
    }
}

#[test]
fn reference_agrees_for_all_small_sizes() {
    for l in [SideBySide, Stacked] {
        for w in 1..40 {
            for h in 1..40 {
                for c in 0..2 {
                    for s in [false, true] {
                        let r = nar(l, w, h, c, s);
                        assert_eq!((r.x0, r.w, r.y0, r.h), reference(l, w, h, c, s));
                    }
                }
            }
        }
    }
}

#[test]
fn eyes_never_exceed_frame_for_even_sizes() {
    for w in (2..30).step_by(2) {
        for h in (2..30).step_by(2) {
            for l in [SideBySide, Stacked] {
                for c in 0..2 {
                    for s in [false, true] {
                        let r = nar(l, w, h, c, s);
                        assert!(r.x0 + r.w <= w && r.y0 + r.h <= h);
                    }
                }
            }
        }
    }
}

#[test]
fn even_size_eyes_tile_frame() {
    for s in [false, true] {
        let a = nar(SideBySide, 10, 4, 0, s);
        let b = nar(SideBySide, 10, 4, 1, s);
        assert_eq!(a.w + b.w, 10);
        assert_ne!(a.x0, b.x0);
    }
}

#[test]
fn odd_dropped_line_cases() {
    // side by side, exchange off: centre column 2 dropped; exchange on: shared column 2
    let (a, b) = (
        nar(SideBySide, 5, 1, 0, false),
        nar(SideBySide, 5, 1, 1, false),
    );
    assert_eq!((a.x0 + a.w, b.x0), (2, 3));
    let (a, b) = (
        nar(SideBySide, 5, 1, 0, true),
        nar(SideBySide, 5, 1, 1, true),
    );
    assert_eq!((a.x0, b.x0 + b.w - 1), (2, 2));
    // stacked, exchange on: dropped centre row
    let (a, b) = (nar(Stacked, 1, 5, 0, true), nar(Stacked, 1, 5, 1, true));
    assert_eq!((b.y0 + b.h, a.y0), (2, 3));
}

// selector table, on the made up sample
fn sel(position: i32) -> i32 {
    position + 1
}

#[test]
fn table_rows_parse_to_their_modes() {
    let t = RoleTable::sample();
    assert!(!t.rows().is_empty());
    for row in t.rows() {
        assert_eq!(
            t.parse_selector(sel(row.position)),
            Ok(LayoutMode {
                layout: row.layout,
                spherical: row.spherical
            })
        );
    }
}

#[test]
fn table_sample_rows_are_pinned() {
    let t = RoleTable::sample();
    let m = |v| t.parse_selector(sel(v)).unwrap();
    assert_eq!((m(201).layout, m(201).spherical), (Layout::Whole, false));
    assert_eq!((m(202).layout, m(202).spherical), (Stacked, false));
    assert_eq!((m(203).layout, m(203).spherical), (SideBySide, false));
    assert_eq!(
        (m(205).layout, m(205).spherical),
        (Layout::TwoLayers, false)
    );
    assert_eq!((m(207).layout, m(207).spherical), (Stacked, true));
    assert_eq!((m(208).layout, m(208).spherical), (SideBySide, true));
}

#[test]
fn table_separators_are_refused() {
    let t = RoleTable::sample();
    assert!(!t.separators().is_empty());
    for p in t.separators() {
        assert_eq!(
            t.parse_selector(sel(*p)),
            Err(LayoutError::SeparatorPosition)
        );
    }
}

#[test]
fn table_unlisted_positions_have_no_row() {
    let t = RoleTable::sample();
    assert_eq!(t.parse_selector(0), Err(LayoutError::NoRow(-1)));
    assert_eq!(t.parse_selector(sel(209)), Err(LayoutError::NoRow(209)));
    assert_eq!(t.parse_selector(-3), Err(LayoutError::NoRow(-4)));
    assert_eq!(t.parse_selector(sel(200)), Err(LayoutError::NoRow(200)));
}

#[test]
fn table_selector_value_is_one_based() {
    let t = RoleTable::sample();
    // the raw position is not a selector value: it reads as the row before it
    assert_eq!(t.parse_selector(202).unwrap().layout, Layout::Whole);
    assert_eq!(t.parse_selector(201), Err(LayoutError::NoRow(200)));
}

#[test]
fn table_new_refuses_duplicate_positions() {
    let row = |position| LayoutRow {
        position,
        layout: Layout::Whole,
        spherical: false,
    };
    assert_eq!(
        RoleTable::new(vec![row(70), row(70)], vec![]),
        Err(TableError::DuplicatePosition(70))
    );
    assert_eq!(
        RoleTable::new(vec![row(70)], vec![70]),
        Err(TableError::DuplicatePosition(70))
    );
    assert_eq!(
        RoleTable::new(vec![row(70)], vec![71, 71]),
        Err(TableError::DuplicatePosition(71))
    );
    assert!(RoleTable::new(vec![row(70)], vec![71]).is_ok());
}

#[test]
fn table_any_shape_is_accepted() {
    let t = RoleTable::new(vec![], vec![]).unwrap();
    assert_eq!(t.parse_selector(1), Err(LayoutError::NoRow(0)));
    let t = RoleTable::new(
        vec![LayoutRow {
            position: 0,
            layout: Stacked,
            spherical: true,
        }],
        vec![1],
    )
    .unwrap();
    assert_eq!(
        t.parse_selector(1),
        Ok(LayoutMode {
            layout: Stacked,
            spherical: true
        })
    );
    assert_eq!(t.parse_selector(2), Err(LayoutError::SeparatorPosition));
}

#[test]
fn spherical_flag_does_not_change_narrowing_inputs() {
    let t = RoleTable::sample();
    for (plain, sph) in [(202, 207), (203, 208)] {
        let a = t.parse_selector(sel(plain)).unwrap();
        let b = t.parse_selector(sel(sph)).unwrap();
        assert_eq!(a.layout, b.layout);
        assert!(!a.spherical && b.spherical);
    }
}

// map and split
#[test]
fn layout_map_entries() {
    for l in [Layout::Whole, Layout::TwoLayers, Stacked, SideBySide] {
        assert_eq!(map_entry(Source::Input, l), l);
        assert_eq!(map_entry(Source::SecondLayer, l), l);
        assert_eq!(map_entry(Source::Extra, l), Layout::Whole);
    }
}

#[test]
fn split_codes() {
    assert_eq!(split_code(Layout::Whole), 0);
    assert_eq!(split_code(Layout::TwoLayers), 0);
    assert_eq!(split_code(Stacked), 1);
    assert_eq!(split_code(SideBySide), 2);
}
#[test]
fn clip_scale_rules() {
    assert_eq!(clip_scale(Stacked, 1, false, &rd()), Some(0.5));
    assert_eq!(clip_scale(SideBySide, 1, false, &rd()), Some(0.5));
    assert_eq!(clip_scale(Layout::Whole, 1, false, &rd()), None);
    assert_eq!(clip_scale(Stacked, 0, false, &rd()), None);
    assert_eq!(clip_scale(Stacked, 1, true, &rd()), None);
    let off = Readings {
        clip_scale_enabled: false,
        ..rd()
    };
    assert_eq!(clip_scale(Stacked, 1, false, &off), None);
}
#[test]
fn variant_codes() {
    let m = |f| LayoutMode {
        layout: Layout::Whole,
        spherical: f,
    };
    assert_eq!(variant_code(m(true), true, 99, &rd()), 6);
    assert_eq!(variant_code(m(false), true, 99, &rd()), 7);
    assert_eq!(variant_code(m(true), false, 99, &rd()), 99);
}

// layer vectors
#[test]
fn two_layer_first_call_layers() {
    let c = first_call_eye(Layout::TwoLayers, 0).unwrap();
    assert_eq!(
        input_read(Layout::TwoLayers, 8, 4, c, false, &rd())
            .unwrap()
            .layer,
        Layer::Input
    );
    let c = first_call_eye(Layout::TwoLayers, 1).unwrap();
    assert_eq!(
        input_read(Layout::TwoLayers, 8, 4, c, false, &rd())
            .unwrap()
            .layer,
        Layer::SecondLayer
    );
}
#[test]
fn two_layer_exchange_swaps_layers() {
    let c = first_call_eye(Layout::TwoLayers, 0).unwrap();
    assert_eq!(
        input_read(Layout::TwoLayers, 8, 4, c, true, &rd())
            .unwrap()
            .layer,
        Layer::SecondLayer
    );
    let c = first_call_eye(Layout::TwoLayers, 1).unwrap();
    assert_eq!(
        input_read(Layout::TwoLayers, 8, 4, c, true, &rd())
            .unwrap()
            .layer,
        Layer::Input
    );
}
#[test]
fn two_layer_read_is_whole_frame() {
    let r = input_read(Layout::TwoLayers, 9, 5, 1, false, &rd())
        .unwrap()
        .rect;
    assert_eq!(
        r,
        Rect {
            x0: 0,
            y0: 0,
            w: 9,
            h: 5
        }
    );
}
#[test]
fn packed_read_uses_input_layer_and_narrows() {
    let r = input_read(SideBySide, 8, 4, 1, false, &rd()).unwrap();
    assert_eq!((r.layer, inc(r.rect)), (Layer::Input, (4, 7, 0, 3)));
    let r = input_read(Stacked, 8, 4, 1, true, &rd()).unwrap();
    assert_eq!((r.layer, inc(r.rect)), (Layer::Input, (0, 7, 0, 1)));
}
#[test]
fn output_rect_matches_input_rect() {
    for l in [SideBySide, Stacked] {
        for c in 0..2 {
            for s in [false, true] {
                assert_eq!(
                    output_rect(l, 7, 9, c, s, &rd()).unwrap(),
                    input_read(l, 7, 9, c, s, &rd()).unwrap().rect
                );
            }
        }
    }
}
#[test]
fn request_eye_rules() {
    let r = rd();
    assert_eq!(request_eye(Layout::TwoLayers, false, 0, &r), Ok(0));
    assert_eq!(request_eye(Layout::TwoLayers, false, 1, &r), Ok(1));
    assert_eq!(request_eye(Layout::TwoLayers, true, 0, &r), Ok(1));
    assert_eq!(request_eye(Layout::TwoLayers, true, 1, &r), Ok(0));
    for l in [Layout::Whole, Stacked, SideBySide] {
        assert_eq!(request_eye(l, true, 1, &r), Ok(0));
    }
    let alt = Readings {
        plain_request_eye: 1,
        ..r
    };
    assert_eq!(request_eye(Stacked, false, 0, &alt), Ok(1));
}
#[test]
fn resolve_source_rules() {
    use Source::*;
    assert_eq!(resolve_source(Input, 1, Layout::TwoLayers), SecondLayer);
    assert_eq!(resolve_source(Input, 0, Layout::TwoLayers), Input);
    assert_eq!(resolve_source(Input, 1, Stacked), Input);
    assert_eq!(resolve_source(Extra, 1, Layout::TwoLayers), Extra);
    assert_eq!(
        resolve_source(SecondLayer, 1, Layout::TwoLayers),
        SecondLayer
    );
}
#[test]
fn first_call_eye_values() {
    assert_eq!(first_call_eye(Layout::TwoLayers, 0), Ok(0));
    assert_eq!(first_call_eye(Layout::TwoLayers, 1), Ok(1));
    assert_eq!(
        first_call_eye(Layout::TwoLayers, 2),
        Err(LayoutError::BadEyeIndex(2))
    );
    for l in [Layout::Whole, Stacked, SideBySide] {
        assert_eq!(first_call_eye(l, 1), Ok(0));
    }
}

// second call
#[test]
fn second_call_args() {
    assert_eq!(FIRST_CALL_ARGS, (0, 0));
    assert_eq!(second_call(Layout::Whole, 8, 4), SecondCall::None);
    assert_eq!(second_call(Stacked, 8, 5), SecondCall::Pair(0, 2));
    assert_eq!(second_call(SideBySide, 9, 4), SecondCall::Pair(4, 0));
    assert_eq!(
        second_call(Layout::TwoLayers, 8, 4),
        SecondCall::Helper {
            constant: 8,
            pair: (0, 0)
        }
    );
    assert_eq!(SECOND_CALL_EYE, 1);
    assert_eq!(FIRST_PREP_ERROR, 517);
}
#[test]
fn second_call_even_sizes() {
    assert_eq!(second_call(Stacked, 8, 4), SecondCall::Pair(0, 2));
    assert_eq!(second_call(SideBySide, 8, 4), SecondCall::Pair(4, 0));
}
#[test]
fn second_call_ignores_other_dimension() {
    assert_eq!(second_call(Stacked, 1000, 6), SecondCall::Pair(0, 3));
    assert_eq!(second_call(SideBySide, 6, 1000), SecondCall::Pair(3, 0));
}

// pass through
#[test]
fn copy_path_layers() {
    assert_eq!(
        copy_path_layer(Layout::TwoLayers, 1, true),
        Layer::SecondLayer
    );
    assert_eq!(copy_path_layer(Layout::TwoLayers, 1, false), Layer::Input);
    assert_eq!(copy_path_layer(Layout::TwoLayers, 0, true), Layer::Input);
    for l in [Layout::Whole, Stacked, SideBySide] {
        assert_eq!(copy_path_layer(l, 1, true), Layer::Input);
    }
}
#[test]
fn copy_path_rect_whole_or_unspecified() {
    assert_eq!(
        copy_path_rect(SideBySide, 8, 4, &rd()),
        Ok(Rect {
            x0: 0,
            y0: 0,
            w: 8,
            h: 4
        })
    );
    let r = Readings {
        copy_path_narrows: true,
        ..rd()
    };
    assert!(matches!(
        copy_path_rect(SideBySide, 8, 4, &r),
        Err(LayoutError::Unspecified(_))
    ));
    assert!(copy_path_rect(Layout::Whole, 8, 4, &r).is_ok());
}

// placement
#[test]
fn place_eye_writes_only_rectangle() {
    let mut f = vec![0u32; 8 * 4];
    let rect = nar(SideBySide, 8, 4, 1, false);
    let eye: Vec<u32> = (1..=16).collect();
    place_eye(&mut f, 8, rect, &eye).unwrap();
    for y in 0..4usize {
        for x in 0..8usize {
            let want = if x >= 4 {
                (y * 4 + (x - 4) + 1) as u32
            } else {
                0
            };
            assert_eq!(f[y * 8 + x], want);
        }
    }
}
#[test]
fn place_eye_odd_leaves_dropped_column() {
    let mut f = vec![0u8; 5 * 2];
    let rect = nar(SideBySide, 5, 2, 1, false);
    place_eye(&mut f, 5, rect, &[1, 2, 3, 4]).unwrap();
    assert_eq!(f, vec![0, 0, 0, 1, 2, 0, 0, 0, 3, 4]);
}
#[test]
fn place_eye_stacked() {
    let mut f = vec![0u8; 3 * 4];
    let rect = nar(Stacked, 3, 4, 1, false);
    place_eye(&mut f, 3, rect, &[1; 6]).unwrap();
    assert_eq!(f, vec![0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1]);
}
#[test]
fn place_eye_size_mismatch() {
    let mut f = vec![0u8; 8];
    let rect = nar(SideBySide, 4, 2, 0, false);
    assert!(matches!(
        place_eye(&mut f, 4, rect, &[1, 2]),
        Err(LayoutError::Unspecified(_))
    ));
}
#[test]
fn place_eye_outside_frame() {
    let mut f = vec![0u8; 4];
    let rect = Rect {
        x0: 3,
        y0: 0,
        w: 2,
        h: 1,
    };
    assert!(place_eye(&mut f, 4, rect, &[1, 2]).is_err());
}
#[test]
fn cache_validity() {
    let r = nar(SideBySide, 8, 4, 0, false);
    assert!(cache_still_valid(r, 4, 4));
    assert!(!cache_still_valid(r, 8, 4));
    assert!(!cache_still_valid(r, 4, 3));
}

#[test]
fn readings_defaults_are_preferred() {
    let r = rd();
    assert_eq!(r.odd_offset, OddOffset::ByEyeIndex);
    assert!(!r.clamp_eye_index && !r.copy_path_narrows && r.clip_scale_enabled);
    assert_eq!(
        (r.plain_request_eye, r.code_spherical, r.code_plain),
        (0, 6, 7)
    );
}

#[test]
fn place_eye_longer_eye_is_refused() {
    let mut f = vec![0u8; 8];
    let rect = nar(SideBySide, 4, 2, 0, false);
    assert_eq!(rect.w * rect.h, 4);
    assert!(matches!(
        place_eye(&mut f, 4, rect, &[1, 2, 3, 4, 5]),
        Err(LayoutError::Unspecified(_))
    ));
    assert_eq!(f, vec![0u8; 8]);
}

#[test]
fn request_eye_index_above_one() {
    let r = rd();
    assert_eq!(
        request_eye(Layout::TwoLayers, false, 2, &r),
        Err(LayoutError::BadEyeIndex(2))
    );
    assert_eq!(
        request_eye(Stacked, false, 2, &r),
        Err(LayoutError::BadEyeIndex(2))
    );
    let clamp = Readings {
        clamp_eye_index: true,
        ..r
    };
    assert_eq!(request_eye(Layout::TwoLayers, false, 2, &clamp), Ok(1));
    assert_eq!(request_eye(Layout::TwoLayers, true, 7, &clamp), Ok(0));
}

// refusals of place_eye and the selector minimum (audit fixes)
#[test]
fn place_eye_row_end_past_frame_row_is_refused() {
    // the buffer is long enough for the flat index, but the rectangle runs past its frame row
    let mut f = vec![0u8; 4 * 2];
    let rect = Rect {
        x0: 3,
        y0: 0,
        w: 2,
        h: 1,
    };
    assert!(matches!(
        place_eye(&mut f, 4, rect, &[1, 2]),
        Err(LayoutError::Unspecified(_))
    ));
    assert_eq!(f, vec![0u8; 8]);
}
#[test]
fn place_eye_refusal_leaves_frame_untouched() {
    // row 0 fits, row 1 is past the end of the buffer
    let mut f = vec![0u8; 4 * 2];
    let rect = Rect {
        x0: 0,
        y0: 1,
        w: 2,
        h: 2,
    };
    assert!(place_eye(&mut f, 4, rect, &[1, 2, 3, 4]).is_err());
    assert_eq!(f, vec![0u8; 8]);
    let rect = Rect {
        x0: 0,
        y0: 0,
        w: 2,
        h: 3,
    };
    assert!(place_eye(&mut f, 4, rect, &[1; 6]).is_err());
    assert_eq!(f, vec![0u8; 8]);
}
#[test]
fn place_eye_exact_fit_and_empty_rectangle() {
    let mut f = vec![0u8; 4 * 2];
    let rect = Rect {
        x0: 2,
        y0: 1,
        w: 2,
        h: 1,
    };
    place_eye(&mut f, 4, rect, &[5, 6]).unwrap();
    assert_eq!(f, vec![0, 0, 0, 0, 0, 0, 5, 6]);
    let empty = Rect {
        x0: 0,
        y0: 9,
        w: 0,
        h: 0,
    };
    assert!(place_eye(&mut f, 4, empty, &[]).is_ok());
}
#[test]
fn place_eye_index_overflow_is_refused() {
    // the row width is 2^31 + 1, so last row times width wraps; with x0 = 4 a wrapping end is 0
    let mut f = vec![0u8; 4];
    let rect = Rect {
        x0: 4,
        y0: u32::MAX,
        w: 0,
        h: u32::MAX - 1,
    };
    let r = std::panic::catch_unwind(move || place_eye(&mut f, (1u32 << 31) + 1, rect, &[]));
    assert!(matches!(r, Ok(Err(LayoutError::Unspecified(_)))));
}
#[test]
fn place_eye_wrapping_end_is_refused() {
    // last row 4295098370 times 4294836226 pixels per row is 2^64 + 4: a wrapping product gives 4
    let mut f = vec![0u8; 4];
    let rect = Rect {
        x0: 0,
        y0: u32::MAX,
        w: 0,
        h: 131_076,
    };
    assert!(place_eye(&mut f, 4_294_836_226, rect, &[]).is_err());
    assert_eq!(f, vec![0u8; 4]);
}
#[test]
fn place_eye_zero_width_below_frame_is_refused() {
    let mut f = vec![0u8; 8];
    let rect = Rect {
        x0: 0,
        y0: 5,
        w: 0,
        h: 1,
    };
    assert!(place_eye(&mut f, 4, rect, &[]).is_err());
}
#[test]
fn place_eye_short_last_row_is_refused() {
    // 7 cells, row width 4: row 1 holds only 3 cells, so x0 2 and w 2 on row 1 ends at cell 8
    let mut f = vec![0u8; 7];
    let rect = Rect {
        x0: 2,
        y0: 1,
        w: 2,
        h: 1,
    };
    assert!(place_eye(&mut f, 4, rect, &[1, 2]).is_err());
    assert_eq!(f, vec![0u8; 7]);
}
#[test]
fn table_refuses_duplicate_positions_that_are_not_adjacent() {
    let row = |p| LayoutRow {
        position: p,
        layout: Layout::Whole,
        spherical: false,
    };
    assert_eq!(
        RoleTable::new(vec![row(310), row(311)], vec![310]),
        Err(TableError::DuplicatePosition(310))
    );
}
#[test]
fn selector_minimum_is_refused_without_panic() {
    let t = RoleTable::sample();
    assert_eq!(
        t.parse_selector(i32::MIN),
        Err(LayoutError::NoRow(i32::MIN))
    );
    let t = RoleTable::new(
        vec![LayoutRow {
            position: i32::MIN,
            layout: Layout::Whole,
            spherical: false,
        }],
        vec![],
    )
    .unwrap();
    assert!(t.parse_selector(i32::MIN).is_err());
}
#[test]
fn place_eye_one_cell_past_the_buffer_is_refused() {
    let mut f = vec![0u8; 4 * 2];
    let rect = Rect {
        x0: 0,
        y0: 2,
        w: 1,
        h: 1,
    };
    assert!(place_eye(&mut f, 4, rect, &[1]).is_err());
    assert_eq!(f, vec![0u8; 8]);
    let rect = Rect {
        x0: 3,
        y0: 1,
        w: 1,
        h: 1,
    };
    place_eye(&mut f, 4, rect, &[1]).unwrap();
    assert_eq!(f[7], 1);
}
