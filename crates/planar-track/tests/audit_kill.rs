//! Tests added by an independent audit to kill mutants the first suite let live: the order of
//! the pose update of each rule, the gain ratio prediction, the relative cost fall, the rms
//! divisor with pixels outside the frame, and the correlation tie rule.

#![allow(clippy::needless_range_loop)]

use planar_track::align::{stats, FrameLevel, LevelTemplate, Solver};
use planar_track::config::{Acceptance, LevelParams, NccConfig};
use planar_track::image::Image;
use planar_track::ncc;
use planar_track::{Config, Dof, Pose, Pyramid, Region, Tracker, UpdateRule};

fn hmat(s: &[f64; 6]) -> [f64; 9] {
    [
        1.0 + s[2],
        s[3],
        s[0],
        s[4],
        1.0 + s[5],
        s[1],
        0.0,
        0.0,
        1.0,
    ]
}

fn mm(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut r = [0.0; 9];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                r[i * 3 + j] += a[i * 3 + k] * b[k * 3 + j];
            }
        }
    }
    r
}

fn smooth(w: usize, h: usize, dx: f64) -> Image {
    Image::from_fn(w, h, |x, y| {
        let (u, v) = (x as f64 + dx, y as f64);
        0.5 + 0.3 * (0.21 * u + 0.13 * v).sin() + 0.2 * (0.17 * v - 0.11 * u).cos()
    })
}

fn level_pair(t: &Image, f: &Image, region: &Region, cfg: &Config) -> (LevelTemplate, FrameLevel) {
    let tp = Pyramid::build(t, &cfg.pyramid).unwrap();
    let fp = Pyramid::build(f, &cfg.pyramid).unwrap();
    (
        LevelTemplate::build(tp.level(0), region, cfg).unwrap(),
        FrameLevel::new(fp.level(0)),
    )
}

fn cfg_for(rule: UpdateRule) -> Config {
    let mut c = Config {
        dof: Dof::Affine,
        rule,
        ..Config::default()
    };
    c.pyramid.levels = 1;
    c.selection.min_fraction = 1.0;
    c
}

#[test]
fn every_rule_composes_the_step_in_its_published_order() {
    let region = Region::from_rect(30, 30, 8.0, 8.0, 20.0, 20.0).unwrap();
    let t = smooth(30, 30, 0.0);
    let s = [0.1, -0.2, 0.05, -0.03, 0.02, 0.04];
    let pose = Pose {
        h: hmat(&[1.5, 0.5, 0.03, -0.02, 0.04, 0.01]),
    };
    for &rule in &UpdateRule::ALL {
        let cfg = cfg_for(rule);
        let (tpl, fr) = level_pair(&t, &t, &region, &cfg);
        let solver = Solver::new(&cfg, &tpl, &fr);
        let (new, _, _) = solver.apply_step(&pose, 0.0, 0.0, &s).unwrap();
        let want: [f64; 9] = match rule {
            UpdateRule::ForwardAdditive => {
                let mut q = [0.0; 6];
                // pose parameters (tx, ty, a11, a12, a21, a22) plus the step
                let p = [1.5, 0.5, 0.03, -0.02, 0.04, 0.01];
                for k in 0..6 {
                    q[k] = p[k] + s[k];
                }
                hmat(&q)
            }
            UpdateRule::ForwardCompositional | UpdateRule::Esm => mm(&pose.h, &hmat(&s)),
            UpdateRule::InverseCompositional => {
                // pose * H(s)^-1: check through pose = result * H(s)
                let back = mm(&new.h, &hmat(&s));
                for k in 0..9 {
                    assert!((back[k] - pose.h[k]).abs() < 1e-12, "{rule:?} {k}");
                }
                new.h
            }
        };
        for k in 0..9 {
            assert!(
                (new.h[k] - want[k]).abs() < 1e-12,
                "{rule:?} entry {k}: {} vs {}",
                new.h[k],
                want[k]
            );
        }
    }
}

#[test]
fn gain_ratio_uses_the_full_quadratic_model() {
    // A Levenberg step with lambda 1 on a smooth pair: the true predicted decrease is
    // 0.5 s^T A s + lambda |s|^2 and the actual ratio is below 0.999, so the step is rejected.
    // A prediction of lambda |s|^2 alone would make the ratio huge and accept it.
    let region = Region::from_rect(30, 30, 8.0, 8.0, 20.0, 20.0).unwrap();
    let t = smooth(30, 30, 0.0);
    let f = smooth(30, 30, 0.8);
    let mut cfg = cfg_for(UpdateRule::ForwardCompositional);
    cfg.dof = Dof::Translation;
    cfg.lm.damping = planar_track::Damping::Levenberg;
    cfg.lm.lambda_init = 1.0;
    cfg.lm.acceptance = Acceptance::GainRatio(0.999);
    let (tpl, fr) = level_pair(&t, &f, &region, &cfg);
    let solver = Solver::new(&cfg, &tpl, &fr);
    let start = Pose::identity();
    let lin = solver.linearize(&start, 0.0, 0.0, true);
    let mut a = lin.a.clone();
    for i in 0..2 {
        a[i * 2 + i] += 1.0;
    }
    let rhs = [-lin.b[0], -lin.b[1]];
    let s = planar_track::linalg::solve_spd(&a, &rhs, 2).unwrap();
    let (np, _, _) = solver.apply_step(&start, 0.0, 0.0, &s).unwrap();
    let new = solver.linearize(&np, 0.0, 0.0, false);
    let quad: f64 = (0..2)
        .map(|p| (0..2).map(|q| s[p] * lin.a[p * 2 + q] * s[q]).sum::<f64>())
        .sum();
    let lin_term = lin.b[0] * s[0] + lin.b[1] * s[1];
    let pred = -(lin_term + 0.5 * quad);
    let actual = 0.5 * (lin.mean() - new.mean()) * lin.wsum;
    let wrong_pred = -(lin_term + quad);
    assert!(
        actual > 0.0 && actual < 0.999 * pred,
        "precondition: ratio {}",
        actual / pred
    );
    assert!(actual > 0.999 * wrong_pred, "precondition for the mutant");
    let lp = LevelParams {
        max_iterations: 1,
        ..LevelParams::default()
    };
    let out = solver.solve(&start, 0.0, 0.0, &lp);
    assert_eq!(out.pose, start, "the step must be rejected");
}

#[test]
fn the_cost_fall_is_relative() {
    // The same sequence at amplitude 1e-5: every absolute cost is tiny, the relative fall is not.
    let region = Region::from_rect(60, 48, 12.0, 10.0, 47.0, 37.0).unwrap();
    let amp = 1e-5;
    let tex = |u: f64, v: f64| {
        amp * (0.5 + 0.3 * (0.35 * u + 0.21 * v).sin() + 0.2 * (0.29 * v - 0.23 * u + 1.0).cos())
    };
    let t = Image::from_fn(60, 48, |x, y| tex(x as f64, y as f64));
    let f = Image::from_fn(60, 48, |x, y| tex(x as f64 - 1.3, y as f64 + 0.9));
    let mut cfg = Config {
        dof: Dof::Translation,
        ..Config::default()
    };
    cfg.pyramid.levels = 1;
    let mut tr = Tracker::new(cfg, &t, &region).unwrap();
    tr.add_frame(&f).unwrap();
    let r = tr.solve_frame(0, &Pose::identity()).unwrap();
    let e = r
        .pose
        .max_point_distance(&Pose::translation(1.3, -0.9), &region.corners())
        .unwrap();
    assert!(e < 0.05, "error {e}, stop {:?}", r.stop);
    assert!(
        r.iterations > 2,
        "stopped after {} iterations",
        r.iterations
    );
}

#[test]
fn rms_is_taken_over_the_pixels_inside_the_frame() {
    let t = Image::from_fn(12, 12, |x, _| x as f64);
    let region = Region::from_rect(12, 12, 5.0, 4.0, 9.0, 6.0).unwrap();
    let cfg = cfg_for(UpdateRule::InverseCompositional);
    let (tpl, fr) = level_pair(&t, &t, &region, &cfg);
    // shift 3: columns 5 to 8 map inside (u <= 11), column 9 maps outside. Residual 3 everywhere.
    let st = stats(&tpl, &fr, &Pose::translation(3.0, 0.0), 0.0, 0.0, 1.0);
    assert!((st.rms - 3.0).abs() < 1e-12, "rms {}", st.rms);
    assert!((st.valid_fraction - 0.8).abs() < 1e-12);
    assert_eq!(st.inlier_fraction, 0.0);
}

#[test]
fn the_correlation_search_keeps_the_first_of_equal_scores() {
    // Columns repeat with period 4, so shifts of -4, 0 and 4 give bit identical scores.
    let img = Image::from_fn(32, 24, |x, y| {
        (((x % 4) * 7 + y * 3) % 11 + (y * y) % 5) as f64
    });
    let region = Region::from_rect(32, 24, 12.0, 6.0, 19.0, 17.0).unwrap();
    let mut cfg = cfg_for(UpdateRule::InverseCompositional);
    cfg.pyramid.levels = 1;
    let (tpl, fr) = level_pair(&img, &img, &region, &cfg);
    let nc = NccConfig {
        enabled: true,
        radius: 4,
        step: 1,
        ..NccConfig::default()
    };
    let found = ncc::search(&tpl, &fr, &Pose::identity(), &nc).unwrap();
    assert_eq!(found.shift, (-4.0, 0.0));
}

#[test]
fn the_correlation_shift_is_applied_in_frame_space() {
    // A start pose that does not commute with translations: the shift moves the frame side,
    // pose' = T(shift) * start.
    let img = smooth(40, 40, 0.0);
    let region = Region::from_rect(40, 40, 10.0, 10.0, 25.0, 25.0).unwrap();
    let mut cfg = cfg_for(UpdateRule::InverseCompositional);
    cfg.pyramid.levels = 1;
    let (tpl, fr) = level_pair(&img, &img, &region, &cfg);
    let start = Pose {
        h: [1.2, 0.05, 0.5, -0.04, 0.9, 0.3, 0.0, 0.0, 1.0],
    };
    let nc = NccConfig {
        enabled: true,
        radius: 3,
        step: 1,
        min_score: -1.0,
        ..NccConfig::default()
    };
    let found = ncc::search(&tpl, &fr, &start, &nc).unwrap();
    let want = mm(&Pose::translation(found.shift.0, found.shift.1).h, &start.h);
    assert_ne!(found.shift, (0.0, 0.0));
    for k in 0..9 {
        assert!((found.pose.h[k] - want[k]).abs() < 1e-12, "entry {k}");
    }
}
