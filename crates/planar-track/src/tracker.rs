//! Frame to frame tracking and the correction API.
//!
//! The template is fixed (an image and a region of it). The tracker holds a list of frames and,
//! for each, an optional pose with its source ([`PoseSource::Known`] when set by the caller,
//! [`PoseSource::Tracked`] when solved) and its [`FrameResult`]. Frames are solved coarse to
//! fine: the pose is carried between levels through the affine level maps of [`crate::pyramid`]
//! (coarse to fine tracking after Bouguet, "Pyramidal Implementation of the Affine Lucas Kanade
//! Feature Tracker", Intel Corporation, 2001). The first guess of a frame is the pose of the neighbouring frame in
//! the direction of travel, optionally moved by the constant image space velocity of the two
//! frames before it.

use crate::align::{stats, FrameLevel, LevelTemplate, Solver, StopReason};
use crate::config::{Config, Illumination, Velocity};
use crate::error::Error;
use crate::image::Image;
use crate::linalg::{inv3, mul3};
use crate::model::Pose;
use crate::ncc;
use crate::pyramid::Pyramid;
use crate::region::Region;

/// Where a stored pose came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoseSource {
    /// Set by the caller with [`Tracker::set_pose`].
    Known,
    /// Solved by the tracker.
    Tracked,
}

/// What is reported for every frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameResult {
    /// The pose, template plane to frame.
    pub pose: Pose,
    /// Where the pose came from.
    pub source: PoseSource,
    /// Root mean square residual at the finest level over the selected pixels inside the frame.
    pub rms: f64,
    /// Fraction of the selected finest level pixels that are inside the frame with a residual
    /// within `Config::inlier_threshold`.
    pub inlier_fraction: f64,
    /// Fraction of the selected finest level pixels inside the frame.
    pub valid_fraction: f64,
    /// Iterations summed over the levels (0 for a known pose).
    pub iterations: usize,
    /// Whether the finest level stopped for a converged reason.
    pub converged: bool,
    /// Why the finest level stopped (`None` for a known pose).
    pub stop: Option<StopReason>,
    /// Estimated gain (0 when the illumination model is off).
    pub gain: f64,
    /// Estimated offset (0 when the illumination model is off).
    pub offset: f64,
    /// Best correlation score of the coarse search, when it ran and took a shift.
    pub ncc_score: Option<f64>,
}

#[derive(Debug, Clone)]
struct Entry {
    result: FrameResult,
}

/// The tracker.
pub struct Tracker {
    cfg: Config,
    template: Vec<LevelTemplate>,
    template_levels: usize,
    frames: Vec<Vec<FrameLevel>>,
    entries: Vec<Option<Entry>>,
}

impl Tracker {
    /// A tracker for a template image and a region of it (same size as the image).
    pub fn new(cfg: Config, template: &Image, region: &Region) -> Result<Tracker, Error> {
        cfg.validate()?;
        if region.width() != template.width() || region.height() != template.height() {
            return Err(Error::Image("region and template sizes differ".into()));
        }
        let pyr = Pyramid::build(template, &cfg.pyramid)?;
        let mut levels = Vec::new();
        for i in 0..pyr.len() {
            levels.push(LevelTemplate::build(pyr.level(i), region, &cfg)?);
        }
        let n = levels.len();
        Ok(Tracker {
            cfg,
            template: levels,
            template_levels: n,
            frames: Vec::new(),
            entries: Vec::new(),
        })
    }

    /// The configuration.
    pub fn config(&self) -> &Config {
        &self.cfg
    }

    /// Number of frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Add a frame; returns its index.
    pub fn add_frame(&mut self, img: &Image) -> Result<usize, Error> {
        let pyr = Pyramid::build(img, &self.cfg.pyramid)?;
        let levels = (0..pyr.len())
            .map(|i| FrameLevel::new(pyr.level(i)))
            .collect();
        self.frames.push(levels);
        self.entries.push(None);
        Ok(self.frames.len() - 1)
    }

    fn check(&self, frame: usize) -> Result<(), Error> {
        if frame < self.frames.len() {
            Ok(())
        } else {
            Err(Error::NoSuchFrame(frame))
        }
    }

    /// The pose of a frame, when it has one.
    pub fn pose(&self, frame: usize) -> Option<Pose> {
        self.entries.get(frame)?.as_ref().map(|e| e.result.pose)
    }

    /// The report of a frame, when it has one.
    pub fn result(&self, frame: usize) -> Option<&FrameResult> {
        self.entries.get(frame)?.as_ref().map(|e| &e.result)
    }

    /// The source of the pose of a frame.
    pub fn source(&self, frame: usize) -> Option<PoseSource> {
        self.result(frame).map(|r| r.source)
    }

    /// Forget the pose of a frame.
    pub fn clear_pose(&mut self, frame: usize) -> Result<(), Error> {
        self.check(frame)?;
        self.entries[frame] = None;
        Ok(())
    }

    /// Correction: set a known pose at a frame. The residual figures are measured for it; later
    /// calls to [`Tracker::track`] keep it and start from it. Poses of other frames are not
    /// touched.
    pub fn set_pose(&mut self, frame: usize, pose: Pose) -> Result<(), Error> {
        self.check(frame)?;
        let pose = pose.normalised()?;
        let result = self.measure(frame, &pose, 0.0, 0.0, PoseSource::Known);
        self.entries[frame] = Some(Entry { result });
        Ok(())
    }

    fn measure(
        &self,
        frame: usize,
        pose: &Pose,
        gain: f64,
        offset: f64,
        source: PoseSource,
    ) -> FrameResult {
        let st = stats(
            &self.template[0],
            &self.frames[frame][0],
            pose,
            gain,
            offset,
            self.cfg.inlier_threshold,
        );
        FrameResult {
            pose: *pose,
            source,
            rms: st.rms,
            inlier_fraction: st.inlier_fraction,
            valid_fraction: st.valid_fraction,
            iterations: 0,
            converged: source == PoseSource::Known,
            stop: None,
            gain,
            offset,
            ncc_score: None,
        }
    }

    /// Solve one frame from a first guess, store the result as tracked and return it.
    pub fn solve_frame(&mut self, frame: usize, guess: &Pose) -> Result<FrameResult, Error> {
        self.check(frame)?;
        let guess = guess.normalised()?;
        let n = self.template_levels.min(self.frames[frame].len());
        let mut gain = 0.0;
        let mut offset = 0.0;
        let mut iterations = 0;
        let mut stop = StopReason::NoPixels;
        let mut ncc_score = None;
        let mut pose = guess;
        let search_level = self.cfg.ncc.level.unwrap_or(n - 1).min(n - 1);
        // The pose in base coordinates is carried; each level converts it to its own.
        for l in (0..n).rev() {
            let tpl = &self.template[l];
            let fr = &self.frames[frame][l];
            let mut h = Pose {
                h: mul3(&mul3(&fr.base_to_level, &pose.h), &tpl.level_to_base),
            };
            if self.cfg.ncc.enabled && l == search_level {
                if let Some(found) = ncc::search(tpl, fr, &h, &self.cfg.ncc) {
                    h = found.pose;
                    ncc_score = Some(found.score);
                }
            }
            let solver = Solver::new(&self.cfg, tpl, fr);
            let lp = self.cfg.params_for_level(l, n);
            let out = solver.solve(&h, gain, offset, lp);
            iterations += out.iterations;
            stop = out.stop;
            let result_pose = if out.stop == StopReason::NoPixels {
                h
            } else {
                out.pose
            };
            if out.stop != StopReason::NoPixels && self.cfg.illumination != Illumination::Off {
                gain = out.gain;
                offset = out.offset;
            }
            let base = Pose {
                h: mul3(&mul3(&fr.level_to_base, &result_pose.h), &tpl.base_to_level),
            };
            pose = base.normalised()?;
        }
        let mut result = self.measure(frame, &pose, gain, offset, PoseSource::Tracked);
        result.iterations = iterations;
        result.converged = stop.converged();
        result.stop = Some(stop);
        result.ncc_score = ncc_score;
        self.entries[frame] = Some(Entry {
            result: result.clone(),
        });
        Ok(result)
    }

    /// The first guess for the frame next to `prev` in the direction `dir` (+1 or -1): the pose of
    /// `prev`, moved by the constant image space velocity of the two frames before it when the
    /// configuration asks for that and both have poses. A known pose after a tracked one is a
    /// correction, not a motion: then the velocity is not used (the tracked pose may be a lost
    /// one) and the guess is the pose of `prev`. `None` when `prev` has no pose.
    pub fn predict(&self, prev: usize, dir: i64) -> Option<Pose> {
        let p1 = self.pose(prev)?;
        if self.cfg.velocity == Velocity::ConstantImageSpace {
            let prev2 = prev as i64 - dir;
            if prev2 >= 0 {
                let corrected = self.source(prev) == Some(PoseSource::Known)
                    && self.source(prev2 as usize) == Some(PoseSource::Tracked);
                if let Some(p0) = self.pose(prev2 as usize).filter(|_| !corrected) {
                    if let Some(inv) = inv3(&p0.h) {
                        let m = Pose {
                            h: mul3(&p1.h, &inv),
                        };
                        if let Ok(g) = m.compose(&p1).normalised() {
                            return Some(g);
                        }
                    }
                }
            }
        }
        Some(p1)
    }

    /// Track from `from` (which must have a pose) to `to`, forward when `to > from` and backward
    /// when `to < from`, solving every frame in between and `to` itself. A frame with a known
    /// pose in the range is kept and tracking continues from it. Returns the report of every
    /// frame after `from`, in the order visited. Frames that were already solved are solved
    /// again.
    pub fn track(&mut self, from: usize, to: usize) -> Result<Vec<FrameResult>, Error> {
        self.check(from)?;
        self.check(to)?;
        if self.pose(from).is_none() {
            return Err(Error::NoPose(from));
        }
        let dir: i64 = if to >= from { 1 } else { -1 };
        let mut out = Vec::new();
        let mut cur = from as i64;
        while cur != to as i64 {
            let prev = cur as usize;
            cur += dir;
            let f = cur as usize;
            if self.source(f) == Some(PoseSource::Known) {
                out.push(self.result(f).expect("known pose has a result").clone());
                continue;
            }
            let guess = self.predict(prev, dir).ok_or(Error::NoPose(prev))?;
            out.push(self.solve_frame(f, &guess)?);
        }
        Ok(out)
    }

    /// Track from `from` to the last frame.
    pub fn track_forward(&mut self, from: usize) -> Result<Vec<FrameResult>, Error> {
        let last = self
            .frames
            .len()
            .checked_sub(1)
            .ok_or(Error::NoSuchFrame(0))?;
        self.track(from, last)
    }

    /// Track from `from` back to frame 0.
    pub fn track_backward(&mut self, from: usize) -> Result<Vec<FrameResult>, Error> {
        self.track(from, 0)
    }
}
