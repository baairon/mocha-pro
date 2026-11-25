//! Regression tests for two defects found by an independent audit: forward additive steps on a
//! pose that is off the model, and a velocity guess poisoned by a stale tracked pose.

use planar_track::{Config, Dof, Image, Pose, Region, Tracker, UpdateRule, Velocity};

const W: usize = 120;
const H: usize = 72;

fn tex(u: f64, v: f64) -> f64 {
    0.5 + 0.18 * (0.37 * u + 0.11 * v).sin()
        + 0.15 * (0.23 * v - 0.19 * u + 1.0).cos()
        + 0.10 * (0.61 * u * 0.5 + 0.53 * v).sin()
        + 0.07 * (0.9 * u + 0.8 * v + 2.0).sin()
}

fn render(w: usize, h: usize, pose: &Pose, gain: f64, offset: f64) -> Image {
    let inv = pose.inverse().unwrap();
    Image::from_fn(w, h, |x, y| {
        let (u, v) = inv.apply(x as f64, y as f64).unwrap();
        gain * tex(u, v) + offset
    })
}

fn region() -> Region {
    Region::from_rect(W, H, 30.0, 16.0, 89.0, 55.0).unwrap()
}

/// A frame of 170 by 110 pixels with three levels of scale 2 has level maps with different
/// horizontal and vertical scales at the coarsest level, so a similarity pose is off the
/// similarity model there. Forward additive must still converge (it used to project the pose
/// onto the model at every step and stall with MaxRejections, about 0.3 pixel off).
#[test]
fn forward_additive_converges_on_an_anisotropic_level_map() {
    let region = region();
    let t = render(W, H, &Pose::identity(), 1.0, 0.0);
    let c = Pose::translation(60.0, 36.0);
    let ci = Pose::translation(-60.0, -36.0);
    for &rule in &UpdateRule::ALL {
        let mut cfg = Config {
            dof: Dof::Similarity,
            rule,
            ..Config::default()
        };
        cfg.pyramid.levels = 3;
        let mut tr = Tracker::new(cfg, &t, &region).unwrap();
        let mut truths = Vec::new();
        for k in 0..4 {
            let kf = k as f64;
            let p = [0.8 * kf, 0.5 * kf, 0.005 * kf, 0.011 * kf];
            let tp = Pose::translation(20.0, 10.0).compose(
                &c.compose(&Pose::from_params(Dof::Similarity, &p))
                    .compose(&ci),
            );
            tr.add_frame(&render(170, 110, &tp, 1.0, 0.0)).unwrap();
            truths.push(tp);
        }
        tr.set_pose(0, truths[0]).unwrap();
        for (i, r) in tr.track_forward(0).unwrap().iter().enumerate() {
            let e = r
                .pose
                .max_point_distance(&truths[i + 1], &region.corners())
                .unwrap();
            assert!(e < 0.05, "{rule:?} frame {} error {e}", i + 1);
            assert!(r.converged, "{rule:?} frame {} stop {:?}", i + 1, r.stop);
        }
    }
}

/// After `set_pose` at frame 7, tracking backward must not take its velocity from the stale
/// pose of frame 8 that an earlier run lost (it used to guess 20 pixels off at frame 6).
#[test]
fn a_correction_is_not_extrapolated_from_a_stale_tracked_pose() {
    let region = region();
    let t = render(W, H, &Pose::identity(), 1.0, 0.0);
    let mut cfg = Config {
        dof: Dof::Translation,
        rule: UpdateRule::InverseCompositional,
        velocity: Velocity::ConstantImageSpace,
        ..Config::default()
    };
    cfg.pyramid.levels = 2;
    let mut tr = Tracker::new(cfg, &t, &region).unwrap();
    let mut truths = Vec::new();
    for k in 0..10 {
        // a jump of 12 pixels at frame 5 and a blank frame 4: the first run loses the plane
        let jump = if k >= 5 { 12.0 } else { 0.0 };
        let tp = Pose::translation(1.5 * k as f64 + jump, -(k as f64));
        let img = if k == 4 {
            render(W, H, &tp, 0.0, 0.5)
        } else {
            render(W, H, &tp, 1.0, 0.0)
        };
        tr.add_frame(&img).unwrap();
        truths.push(tp);
    }
    tr.set_pose(0, truths[0]).unwrap();
    tr.track_forward(0).unwrap();
    tr.set_pose(7, truths[7]).unwrap();
    tr.track(7, 5).unwrap();
    for f in [6usize, 5] {
        let e = tr
            .pose(f)
            .unwrap()
            .max_point_distance(&truths[f], &region.corners())
            .unwrap();
        assert!(e < 0.2, "frame {f} error {e}");
    }
}

/// The prediction itself: a known pose after a tracked one is a correction, not a velocity.
#[test]
fn predict_skips_the_velocity_across_a_correction() {
    let region = region();
    let t = render(W, H, &Pose::identity(), 1.0, 0.0);
    let cfg = Config {
        dof: Dof::Translation,
        velocity: Velocity::ConstantImageSpace,
        ..Config::default()
    };
    let mut tr = Tracker::new(cfg, &t, &region).unwrap();
    for _ in 0..3 {
        tr.add_frame(&t).unwrap();
    }
    tr.set_pose(0, Pose::translation(0.0, 0.0)).unwrap();
    tr.solve_frame(1, &Pose::translation(5.0, 0.0)).unwrap();
    tr.set_pose(2, Pose::translation(1.0, 0.0)).unwrap();
    // frame 1 is tracked, frame 2 known: stepping forward from 2 to 3 would use (1 -> 2)
    let g = tr.predict(2, 1).unwrap();
    assert_eq!(g.h, Pose::translation(1.0, 0.0).h);
    // two known poses: the velocity applies
    tr.set_pose(1, Pose::translation(0.5, 0.0)).unwrap();
    let g = tr.predict(2, 1).unwrap();
    assert_eq!(g.h, Pose::translation(1.5, 0.0).h);
}
