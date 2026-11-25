//! Tracking of synthetic sequences: a textured plane warped by known homographies frame by frame.

#![allow(clippy::needless_range_loop, clippy::manual_is_multiple_of)]

mod common;

use common::*;
use planar_track::{
    BoundaryWeight, Config, Dof, Illumination, Pose, PoseSource, Tracker, UpdateRule, Velocity,
};

const FRAMES: usize = 5;

/// Track a sequence of `FRAMES` frames of the truth model `truth_dof` with the configuration
/// `cfg`, from the known first pose, and return the tracker.
fn run(cfg: Config, truth_dof: Dof, gain: f64, offset: f64) -> Tracker {
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    for k in 0..FRAMES {
        let img = render(&truth(truth_dof, k), gain, offset, 7);
        t.add_frame(&img).unwrap();
    }
    t.set_pose(0, truth(truth_dof, 0)).unwrap();
    t.track_forward(0).unwrap();
    t
}

fn assert_tracks(t: &Tracker, truth_dof: Dof, tol: f64) {
    for k in 0..FRAMES {
        let p = t.pose(k).expect("pose");
        let e = corner_err(&p, &truth(truth_dof, k));
        assert!(e <= tol, "frame {k}: corner error {e} above {tol}");
    }
}

macro_rules! dof_rule_test {
    ($name:ident, $dof:expr, $rule:expr) => {
        #[test]
        fn $name() {
            let t = run(config($dof, $rule), $dof, 1.0, 0.0);
            assert_tracks(&t, $dof, TOL);
            for k in 1..FRAMES {
                let r = t.result(k).unwrap();
                assert!(r.converged, "frame {k} did not converge: {:?}", r.stop);
                assert!(r.rms < 0.02, "frame {k} rms {}", r.rms);
                assert!(r.inlier_fraction > 0.95);
                assert_eq!(r.source, PoseSource::Tracked);
            }
        }
    };
}

dof_rule_test!(
    translation_forward_additive,
    Dof::Translation,
    UpdateRule::ForwardAdditive
);
dof_rule_test!(
    translation_forward_compositional,
    Dof::Translation,
    UpdateRule::ForwardCompositional
);
dof_rule_test!(
    translation_inverse_compositional,
    Dof::Translation,
    UpdateRule::InverseCompositional
);
dof_rule_test!(translation_esm, Dof::Translation, UpdateRule::Esm);
dof_rule_test!(
    transscale_forward_additive,
    Dof::TransScale,
    UpdateRule::ForwardAdditive
);
dof_rule_test!(
    transscale_forward_compositional,
    Dof::TransScale,
    UpdateRule::ForwardCompositional
);
dof_rule_test!(
    transscale_inverse_compositional,
    Dof::TransScale,
    UpdateRule::InverseCompositional
);
dof_rule_test!(transscale_esm, Dof::TransScale, UpdateRule::Esm);
dof_rule_test!(
    similarity_forward_additive,
    Dof::Similarity,
    UpdateRule::ForwardAdditive
);
dof_rule_test!(
    similarity_forward_compositional,
    Dof::Similarity,
    UpdateRule::ForwardCompositional
);
dof_rule_test!(
    similarity_inverse_compositional,
    Dof::Similarity,
    UpdateRule::InverseCompositional
);
dof_rule_test!(similarity_esm, Dof::Similarity, UpdateRule::Esm);
dof_rule_test!(
    affine_forward_additive,
    Dof::Affine,
    UpdateRule::ForwardAdditive
);
dof_rule_test!(
    affine_forward_compositional,
    Dof::Affine,
    UpdateRule::ForwardCompositional
);
dof_rule_test!(
    affine_inverse_compositional,
    Dof::Affine,
    UpdateRule::InverseCompositional
);
dof_rule_test!(affine_esm, Dof::Affine, UpdateRule::Esm);
dof_rule_test!(
    homography_forward_additive,
    Dof::Homography,
    UpdateRule::ForwardAdditive
);
dof_rule_test!(
    homography_forward_compositional,
    Dof::Homography,
    UpdateRule::ForwardCompositional
);
dof_rule_test!(
    homography_inverse_compositional,
    Dof::Homography,
    UpdateRule::InverseCompositional
);
dof_rule_test!(homography_esm, Dof::Homography, UpdateRule::Esm);

macro_rules! bigger_model_test {
    ($name:ident, $rule:expr) => {
        #[test]
        fn $name() {
            // A model with more freedom than the motion still finds the motion.
            let t = run(config(Dof::Affine, $rule), Dof::Translation, 1.0, 0.0);
            assert_tracks(&t, Dof::Translation, TOL);
        }
    };
}

bigger_model_test!(affine_model_on_translation_fa, UpdateRule::ForwardAdditive);
bigger_model_test!(
    affine_model_on_translation_fc,
    UpdateRule::ForwardCompositional
);
bigger_model_test!(
    affine_model_on_translation_ic,
    UpdateRule::InverseCompositional
);
bigger_model_test!(affine_model_on_translation_esm, UpdateRule::Esm);

#[test]
fn homography_model_on_affine_motion() {
    let t = run(
        config(Dof::Homography, UpdateRule::InverseCompositional),
        Dof::Affine,
        1.0,
        0.0,
    );
    assert_tracks(&t, Dof::Affine, TOL);
}

#[test]
fn smaller_model_cannot_follow_larger_motion() {
    // Translation only on an affine motion leaves a visible error at the later frames, so the
    // tolerance of the matched model is a real test.
    let t = run(
        config(Dof::Translation, UpdateRule::InverseCompositional),
        Dof::Affine,
        1.0,
        0.0,
    );
    let e = corner_err(&t.pose(4).unwrap(), &truth(Dof::Affine, 4));
    assert!(e > TOL, "error {e}");
}

#[test]
fn backward_tracking_matches_forward() {
    let cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    for k in 0..FRAMES {
        t.add_frame(&render(&truth(Dof::Affine, k), 1.0, 0.0, 7))
            .unwrap();
    }
    t.set_pose(FRAMES - 1, truth(Dof::Affine, FRAMES - 1))
        .unwrap();
    let out = t.track_backward(FRAMES - 1).unwrap();
    assert_eq!(out.len(), FRAMES - 1);
    assert_tracks(&t, Dof::Affine, TOL);
}

#[test]
fn track_range_partial() {
    let cfg = config(Dof::Affine, UpdateRule::ForwardCompositional);
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    for k in 0..FRAMES {
        t.add_frame(&render(&truth(Dof::Affine, k), 1.0, 0.0, 7))
            .unwrap();
    }
    t.set_pose(1, truth(Dof::Affine, 1)).unwrap();
    let out = t.track(1, 3).unwrap();
    assert_eq!(out.len(), 2);
    assert!(t.pose(0).is_none());
    assert!(t.pose(4).is_none());
    assert!(corner_err(&t.pose(3).unwrap(), &truth(Dof::Affine, 3)) <= TOL);
}

#[test]
fn constant_velocity_prediction_tracks() {
    let mut cfg = config(Dof::Translation, UpdateRule::InverseCompositional);
    cfg.velocity = Velocity::ConstantImageSpace;
    let t = run(cfg, Dof::Translation, 1.0, 0.0);
    assert_tracks(&t, Dof::Translation, TOL);
}

#[test]
fn constant_velocity_prediction_lowers_iterations_on_steady_motion() {
    // A steady motion of 2.5 pixels per frame: the prediction starts nearer to the answer.
    let pose = |k: usize| Pose::translation(2.5 * k as f64, 1.5 * k as f64);
    let count = |velocity: Velocity| {
        let mut cfg = config(Dof::Translation, UpdateRule::InverseCompositional);
        cfg.pyramid.levels = 1;
        cfg.velocity = velocity;
        let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
        for k in 0..6 {
            t.add_frame(&render(&pose(k), 1.0, 0.0, 7)).unwrap();
        }
        t.set_pose(0, pose(0)).unwrap();
        t.set_pose(1, pose(1)).unwrap();
        let out = t.track(1, 5).unwrap();
        for (i, r) in out.iter().enumerate() {
            assert!(corner_err(&r.pose, &pose(i + 2)) <= TOL);
        }
        out.iter().map(|r| r.iterations).sum::<usize>()
    };
    let off = count(Velocity::Off);
    let on = count(Velocity::ConstantImageSpace);
    assert!(on < off, "with prediction {on}, without {off}");
}

#[test]
fn every_frame_reports_residual_and_inliers() {
    let t = run(
        config(Dof::Affine, UpdateRule::InverseCompositional),
        Dof::Affine,
        1.0,
        0.0,
    );
    for k in 0..FRAMES {
        let r = t.result(k).unwrap();
        assert!(r.rms.is_finite() && r.rms < 0.02);
        assert!(r.valid_fraction > 0.99);
        assert!(r.inlier_fraction > 0.95 && r.inlier_fraction <= 1.0);
    }
    assert_eq!(t.result(0).unwrap().source, PoseSource::Known);
    assert_eq!(t.result(0).unwrap().iterations, 0);
    assert!(t.result(2).unwrap().iterations > 0);
}

#[test]
fn wrong_plane_pattern_gives_poor_residual() {
    let cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    t.add_frame(&render(&Pose::identity(), 1.0, 0.0, 99))
        .unwrap();
    t.set_pose(0, Pose::identity()).unwrap();
    let r = t.result(0).unwrap();
    assert!(r.rms > 0.05, "rms {}", r.rms);
    assert!(r.inlier_fraction < 0.8, "inliers {}", r.inlier_fraction);
}

#[test]
fn pose_leaving_the_frame_lowers_valid_fraction() {
    let cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    t.add_frame(&render(&Pose::identity(), 1.0, 0.0, 7))
        .unwrap();
    t.set_pose(0, Pose::translation(60.0, 0.0)).unwrap();
    let r = t.result(0).unwrap();
    assert!(r.valid_fraction < 0.5, "valid {}", r.valid_fraction);
}

// ---- illumination -------------------------------------------------------------------------

const GAIN: f64 = 1.25;
const OFFSET: f64 = 0.06;

macro_rules! illum_test {
    ($name:ident, $mode:expr, $rule:expr) => {
        #[test]
        fn $name() {
            let mut cfg = config(Dof::Affine, $rule);
            cfg.illumination = $mode;
            let t = run(cfg, Dof::Affine, GAIN, OFFSET);
            assert_tracks(&t, Dof::Affine, TOL);
            let r = t.result(FRAMES - 1).unwrap();
            // The frame is 1.25 T + 0.06, so the template side model has gain 0.25, offset 0.06.
            // Bilinear sampling of the frame lowers its contrast a little, so the estimates sit
            // slightly below the exact values.
            assert!((r.gain - 0.25).abs() < 0.03, "gain {}", r.gain);
            assert!((r.offset - 0.06).abs() < 0.02, "offset {}", r.offset);
            assert!(r.rms < 0.01, "rms {}", r.rms);
        }
    };
}

illum_test!(
    illumination_joint_fa,
    Illumination::Joint,
    UpdateRule::ForwardAdditive
);
illum_test!(
    illumination_joint_fc,
    Illumination::Joint,
    UpdateRule::ForwardCompositional
);
illum_test!(
    illumination_joint_ic,
    Illumination::Joint,
    UpdateRule::InverseCompositional
);
illum_test!(illumination_joint_esm, Illumination::Joint, UpdateRule::Esm);
illum_test!(
    illumination_alternate_fa,
    Illumination::Alternate,
    UpdateRule::ForwardAdditive
);
illum_test!(
    illumination_alternate_fc,
    Illumination::Alternate,
    UpdateRule::ForwardCompositional
);
illum_test!(
    illumination_alternate_ic,
    Illumination::Alternate,
    UpdateRule::InverseCompositional
);
illum_test!(
    illumination_alternate_esm,
    Illumination::Alternate,
    UpdateRule::Esm
);

#[test]
fn illumination_off_leaves_residual() {
    let cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    let t = run(cfg, Dof::Affine, GAIN, OFFSET);
    let r = t.result(FRAMES - 1).unwrap();
    assert_eq!(r.gain, 0.0);
    assert_eq!(r.offset, 0.0);
    assert!(r.rms > 0.05, "rms {}", r.rms);
}

#[test]
fn illumination_joint_and_alternate_agree() {
    let mut a = config(Dof::Affine, UpdateRule::InverseCompositional);
    a.illumination = Illumination::Joint;
    let mut b = a.clone();
    b.illumination = Illumination::Alternate;
    let ta = run(a, Dof::Affine, GAIN, OFFSET);
    let tb = run(b, Dof::Affine, GAIN, OFFSET);
    for k in 0..FRAMES {
        let e = corner_err(&ta.pose(k).unwrap(), &tb.pose(k).unwrap());
        assert!(e < 0.05, "frame {k}: {e}");
    }
}

#[test]
fn darkening_sequence_is_absorbed() {
    // Each frame is darker than the one before: gain falls from 1 to 0.6.
    let mut cfg = config(Dof::Similarity, UpdateRule::InverseCompositional);
    cfg.illumination = Illumination::Joint;
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    for k in 0..FRAMES {
        t.add_frame(&render(
            &truth(Dof::Similarity, k),
            1.0 - 0.1 * k as f64,
            0.0,
            7,
        ))
        .unwrap();
    }
    t.set_pose(0, truth(Dof::Similarity, 0)).unwrap();
    t.track_forward(0).unwrap();
    assert_tracks(&t, Dof::Similarity, TOL);
    let g = t.result(4).unwrap().gain;
    assert!((g - (-0.4)).abs() < 0.02, "gain {g}");
}

// ---- pixel selection and boundary weights ---------------------------------------------------

#[test]
fn gradient_selection_half_of_the_pixels_tracks() {
    let mut cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    cfg.selection.min_fraction = 0.5;
    let t = run(cfg, Dof::Affine, 1.0, 0.0);
    assert_tracks(&t, Dof::Affine, TOL);
}

#[test]
fn gradient_selection_quarter_of_the_pixels_tracks() {
    let mut cfg = config(Dof::Similarity, UpdateRule::ForwardCompositional);
    cfg.selection.min_fraction = 0.25;
    let t = run(cfg, Dof::Similarity, 1.0, 0.0);
    assert_tracks(&t, Dof::Similarity, TOL);
}

macro_rules! boundary_test {
    ($name:ident, $b:expr) => {
        #[test]
        fn $name() {
            let mut cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
            cfg.selection.boundary = $b;
            let t = run(cfg, Dof::Affine, 1.0, 0.0);
            assert_tracks(&t, Dof::Affine, TOL);
        }
    };
}

boundary_test!(boundary_linear_tracks, BoundaryWeight::Linear { ramp: 5.0 });
boundary_test!(
    boundary_smoothstep_tracks,
    BoundaryWeight::Smoothstep { ramp: 5.0 }
);
boundary_test!(
    boundary_quadratic_tracks,
    BoundaryWeight::Quadratic { ramp: 5.0 }
);

#[test]
fn boundary_weight_downweights_a_corrupted_border() {
    // The outer ring of the region is corrupted in the frames; weights that vanish at the
    // border keep the pose close to the truth, where uniform weights are pulled away.
    let corrupt = |img: &planar_track::Image| {
        let (w, h) = (img.width(), img.height());
        planar_track::Image::from_fn(w, h, |x, y| {
            let edge = (24..=71).contains(&x)
                && (24..=71).contains(&y)
                && !((29..=66).contains(&x) && (29..=66).contains(&y));
            let v = img.at(x, y);
            if edge {
                1.0 - v
            } else {
                v
            }
        })
    };
    let go = |boundary: BoundaryWeight| {
        let mut cfg = config(Dof::Translation, UpdateRule::InverseCompositional);
        cfg.pyramid.levels = 1;
        cfg.selection.boundary = boundary;
        cfg.selection.min_weight = 0.001;
        let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
        let truth = Pose::translation(1.0, 0.5);
        t.add_frame(&corrupt(&render(&truth, 1.0, 0.0, 7))).unwrap();
        t.set_pose(0, Pose::identity()).unwrap();
        let r = t.solve_frame(0, &Pose::identity()).unwrap();
        corner_err(&r.pose, &truth)
    };
    let uniform = go(BoundaryWeight::Off);
    let weighted = go(BoundaryWeight::Quadratic { ramp: 8.0 });
    assert!(weighted < uniform, "weighted {weighted}, uniform {uniform}");
}

// ---- pyramid variants -----------------------------------------------------------------------

#[test]
fn scale_factor_one_and_a_half_tracks() {
    let mut cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    cfg.pyramid.scale = 1.5;
    cfg.pyramid.levels = 3;
    let t = run(cfg, Dof::Affine, 1.0, 0.0);
    assert_tracks(&t, Dof::Affine, TOL);
}

#[test]
fn three_tap_kernel_tracks() {
    let mut cfg = config(Dof::Affine, UpdateRule::Esm);
    cfg.pyramid.kernel_taps = 3;
    let t = run(cfg, Dof::Affine, 1.0, 0.0);
    assert_tracks(&t, Dof::Affine, TOL);
}

#[test]
fn single_level_tracks_small_motion() {
    let mut cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    cfg.pyramid.levels = 1;
    let t = run(cfg, Dof::Affine, 1.0, 0.0);
    assert_tracks(&t, Dof::Affine, TOL);
}

// ---- LM variants ----------------------------------------------------------------------------

#[test]
fn levenberg_damping_tracks() {
    let mut cfg = config(Dof::Affine, UpdateRule::ForwardCompositional);
    cfg.lm.damping = planar_track::Damping::Levenberg;
    let t = run(cfg, Dof::Affine, 1.0, 0.0);
    assert_tracks(&t, Dof::Affine, TOL);
}

#[test]
fn gain_ratio_acceptance_tracks() {
    let mut cfg = config(Dof::Homography, UpdateRule::Esm);
    cfg.lm.acceptance = planar_track::Acceptance::GainRatio(0.1);
    let t = run(cfg, Dof::Homography, 1.0, 0.0);
    assert_tracks(&t, Dof::Homography, TOL);
}

#[test]
fn decrease_or_equal_acceptance_tracks() {
    let mut cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    cfg.lm.acceptance = planar_track::Acceptance::DecreaseOrEqual;
    let t = run(cfg, Dof::Affine, 1.0, 0.0);
    assert_tracks(&t, Dof::Affine, TOL);
}

#[test]
fn large_initial_lambda_still_converges_with_more_iterations() {
    let mut cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    cfg.lm.lambda_init = 10.0;
    cfg.level_params[0].max_iterations = 80;
    let t = run(cfg, Dof::Affine, 1.0, 0.0);
    assert_tracks(&t, Dof::Affine, TOL);
}

#[test]
fn iteration_limit_is_honoured_and_reported() {
    let mut cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    cfg.level_params[0].max_iterations = 1;
    cfg.pyramid.levels = 1;
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    t.add_frame(&render(&truth(Dof::Affine, 4), 1.0, 0.0, 7))
        .unwrap();
    let r = t.solve_frame(0, &Pose::identity()).unwrap();
    assert_eq!(r.iterations, 1);
    assert_eq!(r.stop, Some(planar_track::StopReason::MaxIterations));
    assert!(!r.converged);
}

#[test]
fn per_level_limits_are_used_coarsest_first() {
    let mut cfg = config(Dof::Translation, UpdateRule::InverseCompositional);
    let mut coarse = cfg.level_params[0].clone();
    coarse.max_iterations = 1;
    let fine = cfg.level_params[0].clone();
    cfg.level_params = vec![coarse, fine.clone(), fine];
    let lp0 = cfg.params_for_level(2, 3).max_iterations;
    let lp1 = cfg.params_for_level(0, 3).max_iterations;
    assert_eq!((lp0, lp1), (1, 30));
}

// ---- large motion and the correlation search -------------------------------------------------

fn large_motion(ncc: bool) -> f64 {
    let mut cfg = config(Dof::Translation, UpdateRule::InverseCompositional);
    cfg.pyramid.levels = 2;
    cfg.ncc.enabled = ncc;
    cfg.ncc.radius = 10;
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    let truth = Pose::translation(17.0, -13.0);
    t.add_frame(&render(&truth, 1.0, 0.0, 7)).unwrap();
    let r = t.solve_frame(0, &Pose::identity()).unwrap();
    corner_err(&r.pose, &truth)
}

#[test]
fn large_motion_without_search_is_lost() {
    let e = large_motion(false);
    assert!(e > 2.0, "error {e}");
}

#[test]
fn large_motion_with_search_is_found() {
    let e = large_motion(true);
    assert!(e <= TOL, "error {e}");
}

#[test]
fn search_reports_its_score() {
    let mut cfg = config(Dof::Translation, UpdateRule::InverseCompositional);
    cfg.pyramid.levels = 2;
    cfg.ncc.enabled = true;
    cfg.ncc.radius = 10;
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    t.add_frame(&render(&Pose::translation(17.0, -13.0), 1.0, 0.0, 7))
        .unwrap();
    let r = t.solve_frame(0, &Pose::identity()).unwrap();
    assert!(r.ncc_score.unwrap() > 0.9);
}

#[test]
fn search_with_a_larger_model_and_illumination_change() {
    let mut cfg = config(Dof::Affine, UpdateRule::InverseCompositional);
    cfg.pyramid.levels = 2;
    cfg.ncc.enabled = true;
    cfg.ncc.radius = 10;
    cfg.illumination = Illumination::Joint;
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    let truth = Pose::translation(-15.0, 14.0);
    t.add_frame(&render(&truth, 1.2, 0.05, 7)).unwrap();
    let r = t.solve_frame(0, &Pose::identity()).unwrap();
    assert!(corner_err(&r.pose, &truth) <= TOL);
}

#[test]
fn search_with_step_two_still_lands_in_the_basin() {
    let mut cfg = config(Dof::Translation, UpdateRule::ForwardCompositional);
    cfg.pyramid.levels = 2;
    cfg.ncc.enabled = true;
    cfg.ncc.radius = 10;
    cfg.ncc.step = 2;
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    let truth = Pose::translation(17.0, -13.0);
    t.add_frame(&render(&truth, 1.0, 0.0, 7)).unwrap();
    let r = t.solve_frame(0, &Pose::identity()).unwrap();
    assert!(corner_err(&r.pose, &truth) <= TOL);
}

#[test]
fn search_with_zero_radius_is_no_search() {
    let mut cfg = config(Dof::Translation, UpdateRule::InverseCompositional);
    cfg.pyramid.levels = 2;
    cfg.ncc.enabled = true;
    cfg.ncc.radius = 0;
    cfg.ncc.min_score = -1.0;
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    let truth = Pose::translation(17.0, -13.0);
    t.add_frame(&render(&truth, 1.0, 0.0, 7)).unwrap();
    let r = t.solve_frame(0, &Pose::identity()).unwrap();
    let e = corner_err(&r.pose, &truth);
    assert!(e > 2.0, "error {e}");
    assert!(r.ncc_score.is_some());
}

// ---- drift and the correction API -------------------------------------------------------------

/// Frames of a translation that jumps by 9 pixels at frame 5, with frame 5 itself ruined.
fn drift_scene() -> (Tracker, Vec<Pose>) {
    let truths: Vec<Pose> = (0..10)
        .map(|k| {
            let x = if k < 5 {
                2.0 * k as f64
            } else {
                2.0 * k as f64 + 9.0
            };
            Pose::translation(x, 0.5 * k as f64)
        })
        .collect();
    let mut cfg = config(Dof::Translation, UpdateRule::InverseCompositional);
    cfg.pyramid.levels = 1;
    let mut t = Tracker::new(cfg, &template(), &region()).unwrap();
    for (k, p) in truths.iter().enumerate() {
        let img = if k == 5 {
            render(p, 1.0, 0.0, 99)
        } else {
            render(p, 1.0, 0.0, 7)
        };
        t.add_frame(&img).unwrap();
    }
    t.set_pose(0, truths[0]).unwrap();
    t.track_forward(0).unwrap();
    (t, truths)
}

#[test]
fn drift_scene_loses_the_later_frames() {
    let (t, truths) = drift_scene();
    for k in 6..10 {
        let e = corner_err(&t.pose(k).unwrap(), &truths[k]);
        assert!(e > 2.0, "frame {k} unexpectedly tracked: {e}");
    }
    // The frames before the break are fine.
    for k in 0..5 {
        assert!(corner_err(&t.pose(k).unwrap(), &truths[k]) <= TOL);
    }
}

#[test]
fn correction_at_the_break_recovers_later_frames() {
    let (mut t, truths) = drift_scene();
    t.set_pose(5, truths[5]).unwrap();
    assert_eq!(t.source(5), Some(PoseSource::Known));
    t.track(5, 9).unwrap();
    for k in 6..10 {
        let e = corner_err(&t.pose(k).unwrap(), &truths[k]);
        assert!(e <= TOL, "frame {k}: {e}");
    }
    assert_eq!(t.pose(5).unwrap(), truths[5].normalised().unwrap());
}

#[test]
fn correction_with_a_known_pose_later_in_the_range() {
    let (mut t, truths) = drift_scene();
    t.set_pose(7, truths[7]).unwrap();
    t.set_pose(5, truths[5]).unwrap();
    // A known pose inside the range is kept and tracking continues from it.
    let out = t.track(5, 9).unwrap();
    assert_eq!(out[1].source, PoseSource::Known);
    assert_eq!(t.pose(7).unwrap(), truths[7].normalised().unwrap());
    for k in 6..10 {
        assert!(
            corner_err(&t.pose(k).unwrap(), &truths[k]) <= TOL,
            "frame {k}"
        );
    }
}

#[test]
fn correction_resolves_backward_from_a_late_frame() {
    let (mut t, truths) = drift_scene();
    // Clear the earlier frames and re-solve them backward from a corrected frame 9.
    for k in 0..9 {
        t.clear_pose(k).unwrap();
    }
    t.set_pose(9, truths[9]).unwrap();
    t.track(9, 6).unwrap();
    for k in 6..9 {
        assert!(
            corner_err(&t.pose(k).unwrap(), &truths[k]) <= TOL,
            "frame {k}"
        );
    }
    assert!(t.pose(5).is_none());
}

#[test]
fn solve_frame_from_a_chosen_guess_replaces_the_entry() {
    let (mut t, truths) = drift_scene();
    let r = t.solve_frame(6, &truths[6]).unwrap();
    assert!(corner_err(&r.pose, &truths[6]) <= TOL);
    assert_eq!(t.source(6), Some(PoseSource::Tracked));
}

#[test]
fn resolving_is_repeatable() {
    let (mut t, truths) = drift_scene();
    t.set_pose(5, truths[5]).unwrap();
    t.track(5, 9).unwrap();
    let first: Vec<Pose> = (6..10).map(|k| t.pose(k).unwrap()).collect();
    t.track(5, 9).unwrap();
    let second: Vec<Pose> = (6..10).map(|k| t.pose(k).unwrap()).collect();
    assert_eq!(first, second);
}

#[test]
fn identical_runs_are_bit_identical() {
    let a = run(
        config(Dof::Homography, UpdateRule::Esm),
        Dof::Homography,
        1.0,
        0.0,
    );
    let b = run(
        config(Dof::Homography, UpdateRule::Esm),
        Dof::Homography,
        1.0,
        0.0,
    );
    for k in 0..FRAMES {
        assert_eq!(
            a.pose(k).unwrap().h.map(f64::to_bits),
            b.pose(k).unwrap().h.map(f64::to_bits)
        );
    }
}
