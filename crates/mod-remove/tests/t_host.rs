#![allow(clippy::field_reassign_with_default)]
mod support;
use mod_remove::config::*;
use mod_remove::fill::SynthResult;
use mod_remove::host::*;
use mod_remove::*;
use support::*;

fn refused(p: Preconditions, want: Precondition) {
    assert_eq!(
        check_preconditions(&p),
        Err(RemoveError::Precondition(want))
    );
}
#[test]
fn pre_all_good_passes() {
    assert_eq!(check_preconditions(&Preconditions::all_good()), Ok(()));
}
#[test]
fn pre_single_tracked_frame() {
    let mut p = Preconditions::all_good();
    p.tracked_frames = 1;
    refused(p, Precondition::TooFewTrackedFrames);
}
#[test]
fn pre_coverage_missing() {
    let mut p = Preconditions::all_good();
    p.tracked_in_range = false;
    refused(p, Precondition::CoverageMissing);
}
#[test]
fn pre_layer_inactive() {
    let mut p = Preconditions::all_good();
    p.target_enabled = false;
    refused(p, Precondition::LayerInactive);
}
#[test]
fn pre_lower_layer_missing() {
    let mut p = Preconditions::all_good();
    p.lower_layer_ready = false;
    refused(p, Precondition::LowerLayerMissing);
}
#[test]
fn pre_in_stream_missing() {
    let mut p = Preconditions::all_good();
    p.in_stream_ok = false;
    refused(p, Precondition::InStreamMissing);
}
#[test]
fn pre_out_stream_cases() {
    let mut p = Preconditions::all_good();
    p.out_stream_ok = false;
    refused(p, Precondition::OutStreamUndeclared);
    let mut p = Preconditions::all_good();
    p.out_stream_found = false;
    refused(p, Precondition::OutStreamAbsent);
    let mut p = Preconditions::all_good();
    p.out_stream_locked = true;
    refused(p, Precondition::OutStreamLocked);
    let mut p = Preconditions::all_good();
    p.output_equals_input = true;
    refused(p, Precondition::OutStreamAliasesIn);
}
#[test]
fn pre_plates_exclusive_without_clip() {
    let mut p = Preconditions::all_good();
    p.plates_exclusive = true;
    refused(p, Precondition::PlateClipMissing);
    p.has_plate_clip = true;
    assert!(check_preconditions(&p).is_ok());
}
#[test]
fn pre_range_and_empty_region() {
    let mut p = Preconditions::all_good();
    p.range_ok = false;
    refused(p, Precondition::RangeNotMet);
    let mut p = Preconditions::all_good();
    p.something_to_render = false;
    refused(p, Precondition::EmptyRegion);
}
#[test]
fn pre_first_failure_in_listed_order() {
    let mut p = Preconditions::all_good();
    p.tracked_frames = 0;
    p.target_enabled = false;
    refused(p, Precondition::TooFewTrackedFrames);
}

#[test]
fn expand_single_pixel_square_and_cross() {
    let w = Win::new(0, 0, 8, 8);
    let m = rect_matte(w, 4, 4, 4, 4);
    let sq = widen_mask(&m, 1, ExpandShape::Square).unwrap();
    assert_eq!(sq.win, Win::new(3, 3, 3, 3));
    assert!(sq.data.iter().all(|&v| v == 255));
    let cr = widen_mask(&m, 1, ExpandShape::Cross).unwrap();
    assert_eq!(cr.win, Win::new(3, 3, 3, 3));
    assert_eq!(cr.data.iter().filter(|&&v| v == 255).count(), 5);
    assert_eq!(cr.get(3, 3), 0);
}
#[test]
fn expand_grows_square_by_one_each_side() {
    let w = Win::new(0, 0, 16, 16);
    let m = rect_matte(w, 5, 6, 9, 9);
    let e = widen_mask(&m, 1, ExpandShape::Square).unwrap();
    assert_eq!(e.win, Win::new(4, 5, 7, 6));
    assert!(e.data.iter().all(|&v| v == 255));
}
#[test]
fn expand_touching_border_extends_window() {
    let w = Win::new(0, 0, 4, 4);
    let m = rect_matte(w, 0, 0, 0, 0);
    let e = widen_mask(&m, 1, ExpandShape::Square).unwrap();
    assert_eq!(e.win, Win::new(-1, -1, 3, 3));
}
#[test]
fn widen_empty_mask_is_empty_region() {
    let m = Matte::new(Win::new(0, 0, 4, 4), 0);
    assert!(matches!(
        widen_mask(&m, 1, ExpandShape::Square),
        Err(RemoveError::Precondition(Precondition::EmptyRegion))
    ));
}
#[test]
fn expand_treats_any_nonzero_as_set() {
    let w = Win::new(0, 0, 4, 4);
    let mut m = Matte::new(w, 0);
    m.set(2, 2, 7);
    let e = widen_mask(&m, 1, ExpandShape::Square).unwrap();
    assert_eq!(e.get(2, 2), 255);
}

#[test]
fn candidates_nearest_first() {
    let c = candidate_times(10, 2, 2, 1, (0, 100), &FillConfig::default()).unwrap();
    assert_eq!(c, vec![9, 11, 8, 12]);
    let mut r = FillConfig::default();
    r.tie_break = TieBreak::LaterFirst;
    assert_eq!(
        candidate_times(10, 2, 2, 1, (0, 100), &r).unwrap(),
        vec![11, 9, 12, 8]
    );
}
#[test]
fn candidates_step_and_range() {
    let c = candidate_times(10, 3, 1, 5, (0, 12), &FillConfig::default()).unwrap();
    assert_eq!(c, vec![5, 0]); // the later and earlier offsets fall outside the range
    assert_eq!(
        candidate_times(0, 0, 0, 1, (0, 9), &FillConfig::default()).unwrap(),
        Vec::<i32>::new()
    );
}
#[test]
fn candidates_automatic_step_unspecified() {
    let e = candidate_times(10, 2, 2, 0, (0, 100), &FillConfig::default());
    assert!(matches!(e, Err(RemoveError::Unspecified(_))));
}

fn synth_of(frame_win: Win) -> (Image, SynthResult, Matte) {
    let matte = rect_matte(frame_win, 1, 1, 2, 2);
    let mut img = Image::new(frame_win, [0.0; 4]);
    img.set(1, 1, [1.0, 1.0, 1.0, 1.0]);
    img.set(2, 1, [1.0, 1.0, 1.0, 1.0]);
    let mut filled = vec![true; frame_win.area()];
    filled[frame_win.idx(2, 2)] = false;
    (
        img.clone(),
        SynthResult {
            image: img,
            weights: vec![1.0; frame_win.area()],
            filled,
        },
        matte,
    )
}
#[test]
fn composite_matte_lerp_values() {
    let w = Win::new(0, 0, 4, 4);
    let frame = Image::new(w, [0.5; 4]);
    let (_, fr, mut matte) = synth_of(w);
    matte.set(1, 1, 255);
    matte.set(2, 1, 128);
    matte.set(1, 2, 0);
    let mut r = FillConfig::default();
    r.zero_weight = ZeroWeightPolicy::KeepFrame;
    let out = composite(&frame, &fr, &matte, &r).unwrap();
    assert_eq!(out.at(1, 1), [1.0; 4]);
    let m = 128.0f32 / 255.0f32;
    let e = 1.0f32 * m + 0.5f32 * (1.0f32 - m);
    assert_eq!(out.at(2, 1)[0].to_bits(), e.to_bits());
    assert_eq!(out.at(1, 2), [0.5; 4]); // matte 0: frame kept
    assert_eq!(out.at(3, 3), [0.5; 4]);
    assert_eq!(out.at(2, 2), [0.5; 4]); // zero weight, KeepFrame
}
#[test]
fn composite_unspecified_mode_and_zero_weight() {
    let w = Win::new(0, 0, 4, 4);
    let frame = Image::new(w, [0.5; 4]);
    let (_, fr, matte) = synth_of(w);
    let mut r = FillConfig::default();
    r.composite = CompositeMode::Unspecified;
    assert!(matches!(
        composite(&frame, &fr, &matte, &r),
        Err(RemoveError::Unspecified(_))
    ));
    let r2 = FillConfig::default();
    assert!(matches!(
        composite(&frame, &fr, &matte, &r2),
        Err(RemoveError::Unspecified(_))
    ));
}
#[test]
fn composite_fill_only_policy_writes_fill() {
    let w = Win::new(0, 0, 4, 4);
    let frame = Image::new(w, [0.5; 4]);
    let (_, fr, matte) = synth_of(w);
    let mut r = FillConfig::default();
    r.zero_weight = ZeroWeightPolicy::FillOnly;
    let out = composite(&frame, &fr, &matte, &r).unwrap();
    assert_eq!(out.at(2, 2), [0.0; 4]);
}

#[test]
fn render_frame_refuses_preconditions_and_carry_over() {
    let w = Win::new(0, 0, 8, 8);
    let frame = Image::new(w, [0.5; 4]);
    let layer = rect_matte(w, 3, 3, 4, 4);
    let src = src_of(Image::new(w, [0.5; 4]));
    let mk = |pre: Preconditions, carry: bool| {
        render_frame(
            &FrameInputs {
                pre,
                carry_over_mode: carry,
                ..FrameInputs::sample(
                    &frame,
                    &layer,
                    std::slice::from_ref(&src),
                    &illum::IllumSet::None,
                )
            },
            &FillConfig::default(),
        )
    };
    let mut pre = Preconditions::all_good();
    pre.target_enabled = false;
    assert!(matches!(
        mk(pre, false),
        Err(RemoveError::Precondition(Precondition::LayerInactive))
    ));
    assert!(matches!(
        mk(Preconditions::all_good(), true),
        Err(RemoveError::Unspecified(_))
    ));
}

// a failing precondition is refused as a precondition even when carry-over is on
#[test]
fn failing_precondition_wins_over_carry_over() {
    let w = Win::new(0, 0, 8, 8);
    let frame = Image::new(w, [0.5; 4]);
    let layer = rect_matte(w, 3, 3, 4, 4);
    let src = src_of(Image::new(w, [0.5; 4]));
    let mut pre = Preconditions::all_good();
    pre.target_enabled = false;
    let inp = FrameInputs {
        pre,
        carry_over_mode: true,
        ..FrameInputs::sample(
            &frame,
            &layer,
            std::slice::from_ref(&src),
            &illum::IllumSet::None,
        )
    };
    assert_eq!(
        render_frame(&inp, &FillConfig::default()),
        Err(RemoveError::Precondition(Precondition::LayerInactive))
    );
}

// the carry-over value comes from the caller's inputs: a caller that sets it off renders
#[test]
fn carry_over_value_is_the_callers_not_the_default() {
    let w = Win::new(0, 0, 8, 8);
    let frame = Image::new(w, [0.5; 4]);
    let layer = rect_matte(w, 3, 3, 4, 4);
    let src = src_of(Image::new(w, [0.5; 4]));
    let mk = |carry: bool, spread: bool| {
        render_frame(
            &FrameInputs {
                spread,
                carry_over_mode: carry,
                ..FrameInputs::sample(
                    &frame,
                    &layer,
                    std::slice::from_ref(&src),
                    &illum::IllumSet::None,
                )
            },
            &FillConfig::default(),
        )
    };
    assert!(!matches!(
        mk(false, false),
        Err(RemoveError::Unspecified(_))
    ));
    assert!(matches!(mk(true, false), Err(RemoveError::Unspecified(_))));
    assert!(!matches!(mk(false, true), Err(RemoveError::Unspecified(_))));
}

// the sample inputs carry a made up value and render; the value is the sample's own
#[test]
fn sample_inputs_carry_the_sample_value_and_render() {
    let w = Win::new(0, 0, 8, 8);
    let frame = Image::new(w, [0.5; 4]);
    let layer = rect_matte(w, 3, 3, 4, 4);
    let src = src_of(Image::new(w, [0.5; 4]));
    let mut inp = FrameInputs::sample(
        &frame,
        &layer,
        std::slice::from_ref(&src),
        &illum::IllumSet::None,
    );
    assert_eq!(inp.carry_over_mode, host::SAMPLE_CARRY_OVER);
    assert!(render_frame(&inp, &FillConfig::default()).is_ok());
    inp.carry_over_mode = !host::SAMPLE_CARRY_OVER;
    assert!(matches!(
        render_frame(&inp, &FillConfig::default()),
        Err(RemoveError::Unspecified(_))
    ));
}

// one test per stage the specification does not give
#[test]
fn not_given_fill_kind_selector() {
    for k in 0..4 {
        assert!(matches!(
            fill_kind_selector(k),
            Err(RemoveError::Unspecified(_))
        ));
    }
}
#[test]
fn not_given_tracks_to_homographies() {
    assert!(matches!(
        tracks_to_homographies(),
        Err(RemoveError::Unspecified(_))
    ));
}
#[test]
fn not_given_plate_join() {
    assert!(matches!(plate_join(), Err(RemoveError::Unspecified(_))));
}
#[test]
fn not_given_source_region() {
    assert!(matches!(
        derive_source_region(),
        Err(RemoveError::Unspecified(_))
    ));
}
#[test]
fn not_given_colour_range() {
    assert!(matches!(
        colour_range_and_premultiplication(),
        Err(RemoveError::Unspecified(_))
    ));
}
#[test]
fn not_given_soften_interior_stage() {
    assert!(matches!(
        soften_interior_stage(),
        Err(RemoveError::Unspecified(_))
    ));
}

#[test]
fn config_defaults_are_the_preferred_values() {
    let r = FillConfig::default();
    assert_eq!(r.index_channels, 1);
    assert_eq!(r.ramp_len, 1);
    assert_eq!(r.ramp_phases, RampPhases::DownOnly);
    assert_eq!(r.fit_passes, 2);
    assert_eq!(r.expand_shape, ExpandShape::Square);
    assert!(!r.scatter);
    assert!(r.preserve_weight);
}
#[test]
fn win_helpers() {
    let a = Win::new(0, 0, 2, 2);
    let b = Win::new(3, 1, 2, 2);
    assert_eq!(a.union(&b), Win::new(0, 0, 5, 3));
    assert_eq!(a.union(&Win::new(0, 0, 0, 0)), a);
    assert_eq!(Win::new(0, 0, 0, 0).union(&a), a);
    assert_eq!(a.grow(2), Win::new(-2, -2, 6, 6));
    assert!(!a.contains(2, 0));
    assert_eq!(Matte::new(a, 0).nonzero_bounds(), None);
}

#[test]
fn composite_unspecified_mode_refused_even_with_fill_only_policy() {
    let w = Win::new(0, 0, 4, 4);
    let frame = Image::new(w, [0.5; 4]);
    let (_, fr, matte) = synth_of(w);
    let mut r = FillConfig::default();
    r.composite = CompositeMode::Unspecified;
    r.zero_weight = ZeroWeightPolicy::FillOnly;
    assert!(matches!(
        composite(&frame, &fr, &matte, &r),
        Err(RemoveError::Unspecified(_))
    ));
}
#[test]
fn precondition_display_text_is_distinct_and_prefixed() {
    let all = [
        Precondition::TooFewTrackedFrames,
        Precondition::CoverageMissing,
        Precondition::LayerInactive,
        Precondition::LowerLayerMissing,
        Precondition::InStreamMissing,
        Precondition::OutStreamUndeclared,
        Precondition::OutStreamAbsent,
        Precondition::OutStreamLocked,
        Precondition::OutStreamAliasesIn,
        Precondition::PlateClipMissing,
        Precondition::RangeNotMet,
        Precondition::EmptyRegion,
    ];
    let mut seen = std::collections::BTreeSet::new();
    for p in all {
        let t = p.to_string();
        assert!(t.starts_with("precondition: "), "{t}");
        assert!(seen.insert(t));
    }
}
#[test]
fn precondition_text_comes_from_the_caller_table_when_present() {
    let e = RemoveError::Precondition(Precondition::LayerInactive);
    let mut table = PreconditionTexts::new();
    assert_eq!(
        e.precondition_text(&table),
        Some(Precondition::LayerInactive.to_string())
    );
    table.set(Precondition::LayerInactive, "sample text 7");
    assert_eq!(
        e.precondition_text(&table),
        Some("sample text 7".to_string())
    );
    table.set(Precondition::EmptyRegion, "other sample");
    assert_eq!(
        e.precondition_text(&table),
        Some("sample text 7".to_string())
    );
    assert_eq!(RemoveError::Refused("x").precondition_text(&table), None);
}
#[test]
fn render_precondition_variant_flows_through_display() {
    let e = RemoveError::Precondition(Precondition::OutStreamAliasesIn);
    assert_eq!(e.to_string(), Precondition::OutStreamAliasesIn.to_string());
    assert!(RemoveError::Refused("abc")
        .to_string()
        .starts_with("refused: "));
}

#[test]
fn sample_text_table_covers_every_precondition_and_differs_from_display() {
    let table = PreconditionTexts::sample();
    let mut seen = std::collections::BTreeSet::new();
    for p in Precondition::ALL {
        let t = table.get(p).expect("sample text for every variant");
        assert_ne!(t, p.to_string());
        assert!(seen.insert(t.to_string()));
        assert_eq!(
            RemoveError::Precondition(p).precondition_text(&table),
            Some(t.to_string())
        );
    }
    assert_eq!(seen.len(), Precondition::ALL.len());
}

#[test]
fn precondition_all_lists_each_variant_once() {
    let set: std::collections::BTreeSet<_> = Precondition::ALL.iter().collect();
    assert_eq!(set.len(), 12);
}
