//! Bit level tests of single deterministic steps and of the per level building blocks. The
//! expected values are worked out by hand in the comments and checked with exact equality.

#![allow(clippy::needless_range_loop, clippy::manual_is_multiple_of)]

mod common;

use planar_track::align::{dwdq_identity, stats, FrameLevel, LevelTemplate, Solver, StopReason};
use planar_track::config::{Damping, LevelParams};
use planar_track::image::Image;
use planar_track::ncc;
use planar_track::{
    BoundaryWeight, Config, Dof, Error, Illumination, Pose, Pyramid, Region, Tracker, UpdateRule,
};

/// A 12 by 12 template `T = x` and the matching frame `I = x + 0.5`.
fn ramp_pair(offset: f64) -> (Pyramid, Pyramid) {
    let t = Image::from_fn(12, 12, |x, _| x as f64);
    let f = Image::from_fn(12, 12, |x, _| x as f64 + offset);
    let cfg = Config::default().pyramid;
    (
        Pyramid::build(&t, &cfg).unwrap(),
        Pyramid::build(&f, &cfg).unwrap(),
    )
}

fn step_config(rule: UpdateRule) -> Config {
    let mut c = Config {
        dof: Dof::Translation,
        rule,
        ..Config::default()
    };
    c.pyramid.levels = 1;
    c.selection.min_fraction = 1.0;
    c.lm.damping = Damping::Levenberg;
    c.lm.lambda_init = 1.0;
    c
}

fn one_iteration(cfg: &Config, region: &Region, start: &Pose) -> planar_track::align::LevelOutcome {
    let (tp, fp) = ramp_pair(0.5);
    let tpl = LevelTemplate::build(tp.level(0), region, cfg).unwrap();
    let fr = FrameLevel::new(fp.level(0));
    let solver = Solver::new(cfg, &tpl, &fr);
    let lp = LevelParams {
        max_iterations: 1,
        ..LevelParams::default()
    };
    solver.solve(start, 0.0, 0.0, &lp)
}

fn region_5x3() -> Region {
    Region::from_rect(12, 12, 3.0, 4.0, 7.0, 6.0).unwrap()
}

#[test]
fn linearisation_of_the_ramp_pair_is_exact() {
    // 15 pixels, gradient (1, 0), residual 0.5 everywhere: A = diag(15, 0), b = (7.5, 0).
    let cfg = step_config(UpdateRule::ForwardCompositional);
    let (tp, fp) = ramp_pair(0.5);
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    let fr = FrameLevel::new(fp.level(0));
    let lin = Solver::new(&cfg, &tpl, &fr).linearize(&Pose::identity(), 0.0, 0.0, true);
    assert_eq!(lin.a, vec![15.0, 0.0, 0.0, 0.0]);
    assert_eq!(lin.b, vec![7.5, 0.0]);
    assert_eq!(lin.cost, 3.75);
    assert_eq!(lin.wsum, 15.0);
    assert_eq!(lin.valid, 15);
    assert_eq!(lin.mean(), 0.25);
}

macro_rules! step_test {
    ($name:ident, $rule:expr) => {
        #[test]
        fn $name() {
            // (A + I) s = -b with A + I = diag(16, 1): s = (-7.5 / 16, 0) = (-0.46875, 0).
            let out = one_iteration(&step_config($rule), &region_5x3(), &Pose::identity());
            assert_eq!(
                out.pose.h,
                [1.0, 0.0, -0.46875, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
            );
            assert_eq!(out.iterations, 1);
            assert_eq!(out.stop, StopReason::MaxIterations);
            // The new residual is 0.03125 everywhere.
            assert_eq!(out.mean_cost, 0.03125 * 0.03125);
        }
    };
}

step_test!(
    one_levenberg_step_forward_additive,
    UpdateRule::ForwardAdditive
);
step_test!(
    one_levenberg_step_forward_compositional,
    UpdateRule::ForwardCompositional
);
step_test!(
    one_levenberg_step_inverse_compositional,
    UpdateRule::InverseCompositional
);
step_test!(one_levenberg_step_esm, UpdateRule::Esm);

#[test]
fn one_marquardt_step_scales_the_diagonal() {
    // 4 by 2 pixels: A = diag(8, 0). Marquardt with lambda 1 gives diag(16, 1) (a zero diagonal
    // entry receives lambda itself); b = (4, 0); s = (-0.25, 0).
    let mut cfg = step_config(UpdateRule::ForwardCompositional);
    cfg.lm.damping = Damping::Marquardt;
    cfg.min_valid_pixels = 4;
    let region = Region::from_rect(12, 12, 3.0, 4.0, 6.0, 5.0).unwrap();
    let out = one_iteration(&cfg, &region, &Pose::identity());
    assert_eq!(out.pose.h, [1.0, 0.0, -0.25, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn a_step_from_the_exact_pose_stops_at_zero_cost() {
    let cfg = step_config(UpdateRule::InverseCompositional);
    let (tp, fp) = ramp_pair(0.5);
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    let fr = FrameLevel::new(fp.level(0));
    let solver = Solver::new(&cfg, &tpl, &fr);
    let exact = Pose::translation(-0.5, 0.0);
    let out = solver.solve(&exact, 0.0, 0.0, &LevelParams::default());
    assert_eq!(out.stop, StopReason::CostTol);
    assert_eq!(out.iterations, 0);
    assert_eq!(out.pose, exact);
    assert!(out.stop.converged());
}

#[test]
fn a_pose_outside_the_frame_stops_with_no_pixels() {
    let cfg = step_config(UpdateRule::InverseCompositional);
    let (tp, fp) = ramp_pair(0.5);
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    let fr = FrameLevel::new(fp.level(0));
    let out = Solver::new(&cfg, &tpl, &fr).solve(
        &Pose::translation(150.0, 0.0),
        0.0,
        0.0,
        &LevelParams::default(),
    );
    assert_eq!(out.stop, StopReason::NoPixels);
    assert!(!out.stop.converged());
}

#[test]
fn iteration_converges_to_the_exact_shift_on_the_ramp() {
    for rule in UpdateRule::ALL {
        let cfg = step_config(rule);
        let (tp, fp) = ramp_pair(0.5);
        let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
        let fr = FrameLevel::new(fp.level(0));
        let out = Solver::new(&cfg, &tpl, &fr).solve(
            &Pose::identity(),
            0.0,
            0.0,
            &LevelParams::default(),
        );
        assert!(
            (out.pose.h[2] + 0.5).abs() < 1e-3,
            "{}: {}",
            rule.name(),
            out.pose.h[2]
        );
        assert!(out.stop.converged(), "{}: {:?}", rule.name(), out.stop);
    }
}

#[test]
fn closed_form_gain_and_offset_are_exact() {
    // Frame = 2 T + 1 with T = x: the template side model has gain 1 and offset 1.
    let t = Image::from_fn(12, 12, |x, _| x as f64);
    let f = Image::from_fn(12, 12, |x, _| 2.0 * x as f64 + 1.0);
    let cfg = Config {
        pyramid: planar_track::PyramidConfig {
            levels: 1,
            ..Default::default()
        },
        ..step_config(UpdateRule::InverseCompositional)
    };
    let tp = Pyramid::build(&t, &cfg.pyramid).unwrap();
    let fp = Pyramid::build(&f, &cfg.pyramid).unwrap();
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    let fr = FrameLevel::new(fp.level(0));
    let (g, o) = Solver::new(&cfg, &tpl, &fr).regress(&Pose::identity(), 0.0, 0.0);
    assert_eq!((g, o), (1.0, 1.0));
}

#[test]
fn degenerate_regression_keeps_the_given_values() {
    // A constant template has no contrast: the 2 by 2 system is singular.
    let t = Image::from_fn(12, 12, |_, _| 0.5);
    let cfg = step_config(UpdateRule::InverseCompositional);
    let tp = Pyramid::build(&t, &cfg.pyramid).unwrap();
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    let fr = FrameLevel::new(tp.level(0));
    assert_eq!(
        Solver::new(&cfg, &tpl, &fr).regress(&Pose::identity(), 0.25, 0.125),
        (0.25, 0.125)
    );
}

#[test]
fn system_size_counts_the_illumination_unknowns() {
    let (tp, fp) = ramp_pair(0.5);
    let fr = FrameLevel::new(fp.level(0));
    for (mode, extra) in [
        (Illumination::Off, 0),
        (Illumination::Alternate, 0),
        (Illumination::Joint, 2),
    ] {
        let mut cfg = step_config(UpdateRule::ForwardCompositional);
        cfg.illumination = mode;
        let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
        assert_eq!(Solver::new(&cfg, &tpl, &fr).system_size(), 2 + extra);
    }
}

#[test]
fn template_selection_by_gradient_keeps_the_strongest() {
    // T = x^2 has horizontal gradient 2 x inside: the largest are the columns 7 and 6.
    let t = Image::from_fn(12, 12, |x, _| (x * x) as f64);
    let mut cfg = step_config(UpdateRule::InverseCompositional);
    cfg.selection.min_fraction = 0.4; // ceil(0.4 * 15) = 6 pixels
    let tp = Pyramid::build(&t, &cfg.pyramid).unwrap();
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    assert_eq!(tpl.candidates, 15);
    assert_eq!(tpl.len(), 6);
    assert!(tpl.x.iter().all(|&x| x == 6.0 || x == 7.0));
    // Kept in scan order.
    assert_eq!(tpl.y, vec![4.0, 4.0, 5.0, 5.0, 6.0, 6.0]);
}

#[test]
fn equal_gradients_are_selected_in_scan_order() {
    let (tp, _) = ramp_pair(0.0);
    let mut cfg = step_config(UpdateRule::InverseCompositional);
    cfg.selection.min_fraction = 0.5; // ceil(7.5) = 8
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    assert_eq!(tpl.len(), 8);
    assert_eq!(&tpl.x[..5], &[3.0, 4.0, 5.0, 6.0, 7.0]);
    assert_eq!(&tpl.y[..5], &[4.0; 5]);
    assert_eq!(tpl.y[5], 5.0);
}

#[test]
fn at_least_one_pixel_is_kept() {
    let (tp, _) = ramp_pair(0.0);
    let mut cfg = step_config(UpdateRule::InverseCompositional);
    cfg.selection.min_fraction = 1e-9;
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    assert_eq!(tpl.len(), 1);
}

#[test]
fn template_rows_hold_the_inverse_compositional_jacobian() {
    // grad T = (1, 0): the translation rows are (1, 0) for every pixel.
    let (tp, _) = ramp_pair(0.0);
    let cfg = step_config(UpdateRule::InverseCompositional);
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    for i in 0..tpl.len() {
        assert_eq!(&tpl.jt[2 * i..2 * i + 2], &[1.0, 0.0]);
        assert_eq!(tpl.t[i], tpl.x[i]);
        assert_eq!(tpl.w[i], 1.0);
    }
}

#[test]
fn boundary_weights_follow_the_distance_to_the_outside() {
    let (tp, _) = ramp_pair(0.0);
    let mut cfg = step_config(UpdateRule::InverseCompositional);
    cfg.selection.boundary = BoundaryWeight::Linear { ramp: 2.0 };
    cfg.selection.min_weight = 0.1;
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    let weight = |x: f64, y: f64| {
        let i = (0..tpl.len())
            .find(|&i| tpl.x[i] == x && tpl.y[i] == y)
            .unwrap();
        tpl.w[i]
    };
    assert_eq!(weight(3.0, 4.0), 0.5); // distance 1
    assert_eq!(weight(4.0, 4.0), 0.5); // the row above is outside
    assert_eq!(weight(5.0, 5.0), 1.0); // distance 2
    assert_eq!(weight(7.0, 5.0), 0.5);
}

#[test]
fn the_weight_floor_applies() {
    let (tp, _) = ramp_pair(0.0);
    let mut cfg = step_config(UpdateRule::InverseCompositional);
    cfg.selection.boundary = BoundaryWeight::Quadratic { ramp: 10.0 };
    cfg.selection.min_weight = 0.25;
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    assert!(tpl.w.iter().all(|&w| w >= 0.25));
    assert!(tpl.w.contains(&0.25));
}

#[test]
fn level_one_corners_follow_the_level_map() {
    let t = Image::from_fn(32, 32, |x, y| (x + y) as f64);
    let mut cfg = Config::default();
    cfg.pyramid.levels = 2;
    cfg.pyramid.min_size = 4;
    let p = Pyramid::build(&t, &cfg.pyramid).unwrap();
    let region = Region::from_rect(32, 32, 8.0, 8.0, 23.0, 23.0).unwrap();
    let tpl = LevelTemplate::build(p.level(1), &region, &cfg).unwrap();
    // x0 = 2 x + 0.5, so x = (x0 - 0.5) / 2.
    assert_eq!(tpl.corners[0], (3.75, 3.75));
    assert_eq!(tpl.corners[2], (11.25, 11.25));
}

#[test]
fn identity_jacobian_matches_finite_differences() {
    let q0 = [0.0; 8];
    for &(x, y) in &[(3.0, 5.0), (-2.0, 7.5), (10.0, -4.0)] {
        let j = dwdq_identity(x, y);
        for k in 0..8 {
            let mut q = q0;
            q[k] = 1e-6;
            let (u, v) = Pose::from_q(&q).apply(x, y).unwrap();
            assert!(
                ((u - x) / 1e-6 - j[0][k]).abs() < 1e-3 * (1.0 + x * x + y * y),
                "u, q{k}"
            );
            assert!(
                ((v - y) / 1e-6 - j[1][k]).abs() < 1e-3 * (1.0 + x * x + y * y),
                "v, q{k}"
            );
        }
    }
}

// ---- residual statistics ----------------------------------------------------------------------

#[test]
fn stats_of_an_exact_pose_are_zero() {
    let cfg = step_config(UpdateRule::InverseCompositional);
    let (tp, fp) = ramp_pair(0.5);
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    let fr = FrameLevel::new(fp.level(0));
    let s = stats(&tpl, &fr, &Pose::translation(-0.5, 0.0), 0.0, 0.0, 0.05);
    assert_eq!(
        (s.rms, s.inlier_fraction, s.valid_fraction),
        (0.0, 1.0, 1.0)
    );
}

#[test]
fn stats_of_a_biased_pose() {
    let cfg = step_config(UpdateRule::InverseCompositional);
    let (tp, fp) = ramp_pair(0.5);
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    let fr = FrameLevel::new(fp.level(0));
    let s = stats(&tpl, &fr, &Pose::identity(), 0.0, 0.0, 0.25);
    assert_eq!(s.rms, 0.5);
    assert_eq!(s.inlier_fraction, 0.0);
    let s = stats(&tpl, &fr, &Pose::identity(), 0.0, 0.0, 0.5);
    assert_eq!(s.inlier_fraction, 1.0);
}

#[test]
fn stats_count_pixels_outside_the_frame() {
    let cfg = step_config(UpdateRule::InverseCompositional);
    let (tp, fp) = ramp_pair(0.5);
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), &cfg).unwrap();
    let fr = FrameLevel::new(fp.level(0));
    // Shift by 6: x = 3..=7 maps to 9..=13, and only 9, 10, 11 are inside (width 12).
    let s = stats(&tpl, &fr, &Pose::translation(6.0, 0.0), 0.0, 0.0, 1e9);
    assert_eq!(s.valid_fraction, 3.0 / 5.0);
    assert_eq!(s.inlier_fraction, 3.0 / 5.0);
}

// ---- correlation search -----------------------------------------------------------------------

fn texture_levels(shift: (f64, f64), gain: f64, offset: f64) -> (LevelTemplate, FrameLevel) {
    let cfg = Config {
        selection: planar_track::SelectionConfig {
            min_fraction: 1.0,
            ..Default::default()
        },
        ..Config::default()
    };
    let tp = Pyramid::build(&common::template(), &Config::default().pyramid).unwrap();
    let tpl = LevelTemplate::build(tp.level(0), &common::region(), &cfg).unwrap();
    let frame = common::render(&Pose::translation(shift.0, shift.1), gain, offset, 7);
    let fp = Pyramid::build(&frame, &Config::default().pyramid).unwrap();
    (tpl, FrameLevel::new(fp.level(0)))
}

#[test]
fn ncc_of_identical_data_is_one() {
    let (tpl, fr) = texture_levels((0.0, 0.0), 1.0, 0.0);
    let s = ncc::score(&tpl, &fr, &Pose::identity(), 0.5).unwrap();
    assert!((s - 1.0).abs() < 1e-12, "{s}");
}

#[test]
fn ncc_ignores_gain_and_offset() {
    let (tpl, fr) = texture_levels((0.0, 0.0), 0.5, 0.25);
    let s = ncc::score(&tpl, &fr, &Pose::identity(), 0.5).unwrap();
    assert!((s - 1.0).abs() < 1e-12, "{s}");
}

#[test]
fn ncc_of_inverted_contrast_is_minus_one() {
    let (tpl, fr) = texture_levels((0.0, 0.0), -1.0, 1.0);
    let s = ncc::score(&tpl, &fr, &Pose::identity(), 0.5).unwrap();
    assert!((s + 1.0).abs() < 1e-12, "{s}");
}

#[test]
fn ncc_falls_with_misalignment() {
    let (tpl, fr) = texture_levels((0.0, 0.0), 1.0, 0.0);
    let good = ncc::score(&tpl, &fr, &Pose::identity(), 0.5).unwrap();
    let bad = ncc::score(&tpl, &fr, &Pose::translation(4.0, 3.0), 0.5).unwrap();
    assert!(bad < good - 0.3, "{bad} vs {good}");
}

#[test]
fn ncc_needs_enough_valid_pixels() {
    let (tpl, fr) = texture_levels((0.0, 0.0), 1.0, 0.0);
    assert!(ncc::score(&tpl, &fr, &Pose::translation(70.0, 0.0), 0.6).is_none());
}

#[test]
fn ncc_of_a_flat_frame_has_no_score() {
    let (tpl, _) = texture_levels((0.0, 0.0), 1.0, 0.0);
    let flat = Pyramid::build(
        &Image::from_fn(96, 96, |_, _| 0.3),
        &Config::default().pyramid,
    )
    .unwrap();
    let fr = FrameLevel::new(flat.level(0));
    assert!(ncc::score(&tpl, &fr, &Pose::identity(), 0.5).is_none());
}

#[test]
fn ncc_search_finds_an_integer_shift() {
    let (tpl, fr) = texture_levels((3.0, -2.0), 1.0, 0.0);
    let cfg = planar_track::NccConfig {
        enabled: true,
        radius: 5,
        ..Default::default()
    };
    let r = ncc::search(&tpl, &fr, &Pose::identity(), &cfg).unwrap();
    assert_eq!(r.shift, (3.0, -2.0));
    assert!((r.score - 1.0).abs() < 1e-12);
    assert_eq!(r.pose, Pose::translation(3.0, -2.0));
}

#[test]
fn ncc_search_respects_the_minimum_score() {
    let (tpl, fr) = texture_levels((3.0, -2.0), 1.0, 0.0);
    let cfg = planar_track::NccConfig {
        enabled: true,
        radius: 5,
        min_score: 1.0,
        ..Default::default()
    };
    // The best score is 1 up to rounding; ask for more than any window can give with a tiny radius.
    let cfg2 = planar_track::NccConfig {
        radius: 1,
        min_score: 0.999,
        ..cfg.clone()
    };
    assert!(ncc::search(&tpl, &fr, &Pose::identity(), &cfg2).is_none());
}

#[test]
fn ncc_search_scans_in_order_and_takes_the_first_best() {
    // With a flat tie impossible on texture, check the step: radius 4 step 4 visits -4, 0, 4.
    let (tpl, fr) = texture_levels((4.0, 0.0), 1.0, 0.0);
    let cfg = planar_track::NccConfig {
        enabled: true,
        radius: 4,
        step: 4,
        ..Default::default()
    };
    let r = ncc::search(&tpl, &fr, &Pose::identity(), &cfg).unwrap();
    assert_eq!(r.shift, (4.0, 0.0));
    let cfg = planar_track::NccConfig { step: 3, ..cfg };
    let r = ncc::search(&tpl, &fr, &Pose::identity(), &cfg).unwrap();
    assert!(r.shift.0 % 3.0 == 0.0 || r.shift.0 == -4.0 + 3.0 * 3.0 || r.shift.0.abs() <= 4.0);
}

// ---- tracker API errors -------------------------------------------------------------------------

fn small_tracker() -> Tracker {
    Tracker::new(
        common::config(Dof::Translation, UpdateRule::InverseCompositional),
        &common::template(),
        &common::region(),
    )
    .unwrap()
}

#[test]
fn tracker_refuses_a_region_of_another_size() {
    let r = Region::from_rect(50, 50, 5.0, 5.0, 20.0, 20.0).unwrap();
    let e = Tracker::new(Config::default(), &common::template(), &r).err();
    assert!(matches!(e, Some(Error::Image(_))));
}

#[test]
fn tracker_refuses_an_invalid_configuration() {
    let mut c = Config::default();
    c.lm.lambda_up = 0.5;
    assert!(matches!(
        Tracker::new(c, &common::template(), &common::region()).err(),
        Some(Error::Config(_))
    ));
}

#[test]
fn unknown_frames_are_refused() {
    let mut t = small_tracker();
    assert_eq!(t.set_pose(0, Pose::identity()), Err(Error::NoSuchFrame(0)));
    assert_eq!(t.clear_pose(3), Err(Error::NoSuchFrame(3)));
    assert_eq!(
        t.solve_frame(1, &Pose::identity()).err(),
        Some(Error::NoSuchFrame(1))
    );
    assert_eq!(t.track(0, 1).err(), Some(Error::NoSuchFrame(0)));
    assert_eq!(t.track_forward(0).err(), Some(Error::NoSuchFrame(0)));
    assert!(t.pose(0).is_none() && t.result(0).is_none() && t.source(0).is_none());
}

#[test]
fn tracking_needs_a_pose_at_the_start() {
    let mut t = small_tracker();
    t.add_frame(&common::template()).unwrap();
    t.add_frame(&common::template()).unwrap();
    assert_eq!(t.frame_count(), 2);
    assert_eq!(t.track(0, 1).err(), Some(Error::NoPose(0)));
}

#[test]
fn a_singular_pose_cannot_be_set() {
    let mut t = small_tracker();
    t.add_frame(&common::template()).unwrap();
    let bad = Pose {
        h: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    };
    assert_eq!(t.set_pose(0, bad), Err(Error::Singular));
}

#[test]
fn tracking_to_the_same_frame_returns_nothing() {
    let mut t = small_tracker();
    t.add_frame(&common::template()).unwrap();
    t.set_pose(0, Pose::identity()).unwrap();
    assert!(t.track(0, 0).unwrap().is_empty());
}

#[test]
fn clear_pose_forgets_the_entry() {
    let mut t = small_tracker();
    t.add_frame(&common::template()).unwrap();
    t.set_pose(0, Pose::identity()).unwrap();
    assert!(t.pose(0).is_some());
    t.clear_pose(0).unwrap();
    assert!(t.pose(0).is_none());
}

#[test]
fn set_pose_normalises_the_pose() {
    let mut t = small_tracker();
    t.add_frame(&common::template()).unwrap();
    t.set_pose(
        0,
        Pose {
            h: [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0],
        },
    )
    .unwrap();
    assert_eq!(t.pose(0).unwrap(), Pose::identity());
}

#[test]
fn config_accessor_returns_the_configuration() {
    let t = small_tracker();
    assert_eq!(t.config().dof, Dof::Translation);
}

#[test]
fn display_of_errors_is_plain_text() {
    assert_eq!(Error::NoPose(3).to_string(), "frame 3 has no pose");
    assert_eq!(Error::Singular.to_string(), "singular pose");
    assert!(Error::Config("x".into()).to_string().contains("x"));
}

// ---- gradients, tie cases, level masks, prediction ------------------------------------------------

/// The cost gradient `b` of the linearisation against central differences of half the cost, for
/// the rules whose columns are exact first derivatives of the residual (forward additive and
/// forward compositional).
fn gradient_ratio(rule: UpdateRule) -> f64 {
    let mut cfg = common::config(Dof::Homography, rule);
    cfg.pyramid.levels = 1;
    let tp = Pyramid::build(&common::template(), &cfg.pyramid).unwrap();
    let tpl = LevelTemplate::build(tp.level(0), &common::region(), &cfg).unwrap();
    let truth = Pose::from_params(
        Dof::Homography,
        &[1.5, -1.0, 0.01, 0.005, -0.004, 0.008, 0.002, -0.0015],
    );
    let fp = Pyramid::build(&common::render(&truth, 1.0, 0.0, 7), &cfg.pyramid).unwrap();
    let fr = FrameLevel::new(fp.level(0));
    let solver = Solver::new(&cfg, &tpl, &fr);
    let pose = Pose::from_params(
        Dof::Homography,
        &[1.0, -0.8, 0.012, 0.004, -0.003, 0.009, 0.006, -0.005],
    );
    let lin = solver.linearize(&pose, 0.0, 0.0, true);
    let half_cost = |p: &Pose| 0.5 * solver.linearize(p, 0.0, 0.0, false).cost;
    let eps = 1e-6;
    let base = pose.params(Dof::Homography).unwrap();
    let scale = lin.b.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut worst = 0.0f64;
    for k in 0..8 {
        let mut e = [0.0; 8];
        e[k] = eps;
        let (plus, minus) = if rule == UpdateRule::ForwardAdditive {
            let mut a = base.clone();
            let mut b = base.clone();
            a[k] += eps;
            b[k] -= eps;
            (
                Pose::from_params(Dof::Homography, &a),
                Pose::from_params(Dof::Homography, &b),
            )
        } else {
            (
                pose.compose(&Pose::from_params(Dof::Homography, &e)),
                pose.compose(&Pose::from_params(Dof::Homography, &e.map(|v| -v))),
            )
        };
        let fd = (half_cost(&plus) - half_cost(&minus)) / (2.0 * eps);
        worst = worst.max((fd - lin.b[k]).abs() / scale);
    }
    worst
}

#[test]
fn forward_additive_gradient_matches_finite_differences() {
    let w = gradient_ratio(UpdateRule::ForwardAdditive);
    assert!(w < 0.08, "worst relative gap {w}");
}

#[test]
fn forward_compositional_gradient_matches_finite_differences() {
    let w = gradient_ratio(UpdateRule::ForwardCompositional);
    assert!(w < 0.08, "worst relative gap {w}");
}

/// Constant template 0.5, constant frame 0.7: no gradient, so the damped step is exactly zero and
/// the cost does not change.
fn tie_outcome(cfg: &Config) -> planar_track::align::LevelOutcome {
    let t = Image::from_fn(12, 12, |_, _| 0.5);
    let f = Image::from_fn(12, 12, |_, _| 0.7);
    let tp = Pyramid::build(&t, &cfg.pyramid).unwrap();
    let fp = Pyramid::build(&f, &cfg.pyramid).unwrap();
    let tpl = LevelTemplate::build(tp.level(0), &region_5x3(), cfg).unwrap();
    let fr = FrameLevel::new(fp.level(0));
    let lp = LevelParams {
        max_rejections: 3,
        step_tol: 0.0,
        ..LevelParams::default()
    };
    Solver::new(cfg, &tpl, &fr).solve(&Pose::identity(), 0.0, 0.0, &lp)
}

#[test]
fn a_tie_is_rejected_by_the_strict_decrease_test() {
    let mut cfg = step_config(UpdateRule::ForwardCompositional);
    cfg.lm.acceptance = planar_track::Acceptance::Decrease;
    let out = tie_outcome(&cfg);
    assert_eq!((out.stop, out.iterations), (StopReason::MaxRejections, 3));
}

#[test]
fn a_tie_is_accepted_by_the_or_equal_test() {
    let mut cfg = step_config(UpdateRule::ForwardCompositional);
    cfg.lm.acceptance = planar_track::Acceptance::DecreaseOrEqual;
    let out = tie_outcome(&cfg);
    assert_eq!((out.stop, out.iterations), (StopReason::CostTol, 1));
}

#[test]
fn a_zero_prediction_is_rejected_by_the_gain_ratio_test() {
    let mut cfg = step_config(UpdateRule::ForwardCompositional);
    cfg.lm.acceptance = planar_track::Acceptance::GainRatio(0.0);
    let out = tie_outcome(&cfg);
    assert_eq!((out.stop, out.iterations), (StopReason::MaxRejections, 3));
}

#[test]
fn level_one_mask_uses_nearest_base_pixel() {
    // Level 1 pixel x has its centre at base x' = 2 x + 0.5, which rounds to the base pixel
    // 2 x + 1: the region columns 7 to 20 hold the level 1 columns 3 to 9.
    let t = Image::from_fn(32, 32, |x, y| ((x * 5 + y * 3) % 11) as f64);
    let mut cfg = Config::default();
    cfg.pyramid.levels = 2;
    cfg.pyramid.min_size = 4;
    cfg.selection.min_fraction = 1.0;
    let p = Pyramid::build(&t, &cfg.pyramid).unwrap();
    let region = Region::from_rect(32, 32, 7.0, 8.0, 20.0, 23.0).unwrap();
    let tpl = LevelTemplate::build(p.level(1), &region, &cfg).unwrap();
    let min = tpl.x.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = tpl.x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    assert_eq!((min, max), (3.0, 9.0));
}

#[test]
fn ncc_ties_take_the_first_shift_in_scan_order() {
    // A pattern with period 4 along x: shifts that differ by 4 give bit identical scores, and the
    // first one scanned (the smallest dy, then the smallest dx) wins.
    let t = Image::from_fn(96, 96, |x, y| {
        (x % 4) as f64 * 0.25 + (y * y) as f64 * 0.001
    });
    let cfg = Config {
        selection: planar_track::SelectionConfig {
            min_fraction: 1.0,
            ..Default::default()
        },
        ..Config::default()
    };
    let tp = Pyramid::build(&t, &cfg.pyramid).unwrap();
    let tpl = LevelTemplate::build(tp.level(0), &common::region(), &cfg).unwrap();
    let fr = FrameLevel::new(tp.level(0));
    let n = planar_track::NccConfig {
        enabled: true,
        radius: 8,
        ..Default::default()
    };
    let r = ncc::search(&tpl, &fr, &Pose::identity(), &n).unwrap();
    assert_eq!(r.shift, (-8.0, 0.0));
}

#[test]
fn prediction_applies_the_image_space_velocity() {
    let mut cfg = common::config(Dof::Similarity, UpdateRule::InverseCompositional);
    cfg.velocity = planar_track::Velocity::ConstantImageSpace;
    let mut t = Tracker::new(cfg, &common::template(), &common::region()).unwrap();
    for _ in 0..3 {
        t.add_frame(&common::template()).unwrap();
    }
    let q = common::about_centre(Dof::Similarity, &[1.0, 0.5, 0.01, 0.02]);
    let p0 = common::about_centre(Dof::Similarity, &[10.0, -8.0, 0.1, 0.05]);
    let p1 = q.compose(&p0);
    let p2 = q.compose(&p1);
    t.set_pose(0, p0).unwrap();
    t.set_pose(1, p1).unwrap();
    let g = t.predict(1, 1).unwrap();
    let want = p2.normalised().unwrap();
    for (a, b) in g.h.iter().zip(want.h) {
        assert!((a - b).abs() < 1e-9, "{a} {b}");
    }
    // Backward: from frame 1 towards 0 the two frames before it are 2 and 3, so only the plain pose.
    assert_eq!(t.predict(1, -1).unwrap(), p1.normalised().unwrap());
    assert!(t.predict(2, 1).is_none());
}

#[test]
fn prediction_is_the_plain_pose_when_velocity_is_off() {
    let mut t = small_tracker();
    for _ in 0..3 {
        t.add_frame(&common::template()).unwrap();
    }
    t.set_pose(0, Pose::translation(1.0, 0.0)).unwrap();
    t.set_pose(1, Pose::translation(3.0, 0.0)).unwrap();
    assert_eq!(t.predict(1, 1).unwrap(), Pose::translation(3.0, 0.0));
}
