//! `track-planar`: a generic stage whose result is data. It tracks a planar region through the
//! frames with the `planar-track` crate (direct intensity based registration, every choice of
//! the method given as a parameter), writes every frame as it was read and writes one pose per
//! frame to a key file. It computes a published method under explicit configuration and claims
//! nothing about any product.
//!
//! How a data stage fits the convention: it is a temporal stage that preserves geometry and
//! passes its frames through, with a sink (see [`super::sink`]). The poses go to the file named
//! by the parameter `data`, in the key file form `keys-1` of [`crate::keys`], with the nine
//! tracks `m00` to `m22` and one linear key per frame at time N for frame N. A stage that keys
//! nine parameters of those names reads the file with `--layer keys=PATH` and no converter.
//!
//! The pose of frame N is the 3 by 3 matrix, row major and scaled so that `m22` is 1, that maps a
//! point of the reference frame to the same point of the tracked surface in frame N. Coordinates
//! are pixels with the centre of the first pixel at `(0, 0)`, the convention of `planar-track`.
//!
//! The grey level the tracker reads is `luma_r * R + luma_g * G + luma_b * B` on the stored
//! samples scaled to full scale 1 (depth 8 divided by 255, depth 16 by 65535, depth 32 as
//! stored). Alpha is not read and no transfer function is applied.
//!
//! Order and memory. Frame N is written once it has been tracked. Frames after the reference are
//! tracked one by one as they arrive, and the tracker keeps at most [`WINDOW`] frames, so memory
//! does not grow with the number of frames. Frames before the reference wait as grey images
//! until the reference has been read and are then tracked backwards from it, so memory grows
//! with the parameter `reference` only. The lines of the data file are written as the poses are
//! found and the file appears under its name when the input has ended.
//!
//! A frame the tracker refuses (a region without pixels, too few usable pixels, a sample that is
//! not a finite number, a pose that cannot be scaled) keeps the pose of its neighbour towards the
//! reference when `refusal` is `hold`, and ends the run as `unsupported` when it is `fail`.
//!
//! With neither `data` nor `report` the stage tracks nothing and only passes the frames.

use std::io::{Read, Write};

use planar_track::{
    Acceptance, BoundaryWeight, Config, Damping, Dof, FrameResult, Illumination, Image,
    LevelParams, LmConfig, NccConfig, Pose, PyramidConfig, Region, SelectionConfig, StopReason,
    Tracker, UpdateRule, Velocity,
};

use super::sink::Sink;
use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::keys;
use crate::params::Values;
use crate::registry::{
    FrameSpec, Geometry, Kind, Label, ParamDef, ParamKind, Plan, Stage, StageDef,
};
use crate::run::{copy_frames, map_frames};

/// The planar tracking stage.
pub struct TrackPlanar;

/// The most frames the tracker holds at once. When the count is reached the tracker is built
/// again from the reference frame and the last two frames with their poses, which gives the same
/// first guess for the next frame as a tracker that held every frame.
pub const WINDOW: usize = 16;

/// The first line of the report file.
pub const REPORT_FORM: &str = "track-report-1";

/// The largest magnitude of a pose entry that is written; a pose beyond it is refused.
pub const POSE_LIMIT: f64 = 1e9;

/// The ids of the nine tracks of the data file, row major.
pub const TRACKS: [&str; 9] = [
    "m00", "m01", "m02", "m10", "m11", "m12", "m20", "m21", "m22",
];

const fn text(id: &'static str, name: &'static str, about: &'static str) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Text { default: "" },
        keyed: false,
        about,
    }
}

const fn choice(
    id: &'static str,
    name: &'static str,
    default: &'static str,
    choices: &'static [&'static str],
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Choice { default, choices },
        keyed: false,
        about,
    }
}

const fn float(
    id: &'static str,
    name: &'static str,
    range: (f64, f64, f64),
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Float {
            default: range.0,
            min: range.1,
            max: range.2,
        },
        keyed: false,
        about,
    }
}

const fn int(
    id: &'static str,
    name: &'static str,
    range: (i64, i64, i64),
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Int {
            default: range.0,
            min: range.1,
            max: range.2,
        },
        keyed: false,
        about,
    }
}

const fn flag(
    id: &'static str,
    name: &'static str,
    default: bool,
    about: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        name,
        kind: ParamKind::Checkbox { default },
        keyed: false,
        about,
    }
}

const CORNER: (f64, f64, f64) = (0.0, -1e9, 1e9);
const CORNER_ABOUT: &str = "One coordinate of a corner of the tracked region on the reference frame, in pixels with the centre of the first pixel at 0; the four corners go around the boundary, and when all eight values are 0 the region is the whole frame.";

/// What the stage declares. Every default of a field of `planar_track::Config` is the default of
/// that crate; `gain_ratio` and `boundary_ramp` have no default there and get one here.
pub static DEF: StageDef = StageDef {
    id: "track-planar",
    name: "Track planar",
    about: "Tracks a planar region through the frames by direct intensity based registration (the Lucas and Kanade family with Levenberg and Marquardt damping on a Gaussian pyramid), writes every frame as it was read and writes one pose per frame, a 3 by 3 matrix from the reference frame to that frame, to the key file named by data.",
    label: Label::Generic,
    kind: Kind::Temporal,
    spool: false,
    geometry: Geometry::Preserve,
    depths: &DEPTHS,
    params: &[
        text("data", "Data file", "The path of the key file the stage writes (form keys-1, tracks m00 to m22, one linear key per frame); empty writes none."),
        text("report", "Report file", "The path of a table the stage writes with one row per frame (source, stop reason, iterations, residual, valid and inlier fractions, gain, offset); empty writes none."),
        float("x0", "Corner 0 X", CORNER, CORNER_ABOUT),
        float("y0", "Corner 0 Y", CORNER, CORNER_ABOUT),
        float("x1", "Corner 1 X", CORNER, CORNER_ABOUT),
        float("y1", "Corner 1 Y", CORNER, CORNER_ABOUT),
        float("x2", "Corner 2 X", CORNER, CORNER_ABOUT),
        float("y2", "Corner 2 Y", CORNER, CORNER_ABOUT),
        float("x3", "Corner 3 X", CORNER, CORNER_ABOUT),
        float("y3", "Corner 3 Y", CORNER, CORNER_ABOUT),
        int("reference", "Reference frame", (0, 0, 1_000_000_000), "The index of the frame that holds the region; its pose is the identity. Frames before it wait in memory until it has been read."),
        float("luma_r", "Grey weight R", (0.2126, 0.0, 1.0), "The weight of R in the grey level the tracker reads; the default weights are those of ITU-R BT.709."),
        float("luma_g", "Grey weight G", (0.7152, 0.0, 1.0), "The weight of G in the grey level the tracker reads."),
        float("luma_b", "Grey weight B", (0.0722, 0.0, 1.0), "The weight of B in the grey level the tracker reads."),
        choice("dof", "Motion model", "affine", &["translation", "trans-scale", "similarity", "affine", "homography"], "The degrees of freedom of the motion model (Config::dof)."),
        choice("rule", "Update rule", "inverse-compositional", &["forward-additive", "forward-compositional", "inverse-compositional", "esm"], "The warp update rule (Config::rule)."),
        choice("illumination", "Illumination", "off", &["off", "joint", "alternate"], "The gain and offset model of the grey level (Config::illumination)."),
        choice("damping", "Damping", "marquardt", &["levenberg", "marquardt"], "How the damping term is added to the normal matrix (LmConfig::damping)."),
        choice("acceptance", "Step acceptance", "decrease", &["decrease", "decrease-or-equal", "gain-ratio"], "The test that accepts a trial step (LmConfig::acceptance)."),
        float("gain_ratio", "Gain ratio", (0.0, 0.0, 0.999), "The least ratio of actual to predicted reduction; read when acceptance is gain-ratio. The crate has no default; 0 is this stage's."),
        choice("boundary", "Boundary weight", "off", &["off", "linear", "smoothstep", "quadratic"], "The weight of a region pixel from its distance to the outside of the region (SelectionConfig::boundary)."),
        float("boundary_ramp", "Boundary ramp", (4.0, 0.000001, 65536.0), "The distance in pixels at which the boundary weight reaches 1; read when boundary is not off. The crate has no default; 4 is this stage's."),
        choice("velocity", "Motion prediction", "off", &["off", "constant-image-space"], "The first guess of a frame: the pose of its neighbour, or that pose moved once more by the last motion (Config::velocity)."),
        int("pyramid_levels", "Pyramid levels", (4, 1, 16), "The most levels of the pyramid (PyramidConfig::levels)."),
        float("pyramid_scale", "Pyramid scale", (2.0, 1.0, 8.0), "The size ratio between neighbouring levels, above 1 (PyramidConfig::scale)."),
        int("pyramid_taps", "Pyramid kernel taps", (5, 1, 15), "The taps of the binomial blur kernel, odd (PyramidConfig::kernel_taps)."),
        int("pyramid_min_size", "Pyramid least size", (12, 1, 65536), "A level narrower or lower than this is not built (PyramidConfig::min_size)."),
        float("lambda_init", "Lambda start", (1e-4, 0.0, 1e12), "The damping at the start of every level (LmConfig::lambda_init)."),
        float("lambda_up", "Lambda up", (10.0, 1.0, 1e6), "The factor on the damping after a rejected step, above 1 (LmConfig::lambda_up)."),
        float("lambda_down", "Lambda down", (0.1, 0.0, 1.0), "The factor on the damping after an accepted step, above 0 (LmConfig::lambda_down)."),
        float("lambda_min", "Lambda floor", (1e-9, 0.0, 1e12), "The damping is never lowered below this (LmConfig::lambda_min)."),
        int("max_iterations", "Iteration limit", (30, 1, 100_000), "The most steps of one level (LevelParams::max_iterations)."),
        int("max_rejections", "Rejection limit", (10, 1, 100_000), "The rejected steps in a row after which a level stops (LevelParams::max_rejections)."),
        float("step_tol", "Step tolerance", (1e-3, 0.0, 1e6), "A level stops when the largest corner motion of an accepted step is below this, in level pixels (LevelParams::step_tol)."),
        float("cost_tol", "Cost tolerance", (1e-9, 0.0, 1.0), "A level stops when the relative fall of the cost in an accepted step is below this (LevelParams::cost_tol)."),
        float("lambda_max", "Lambda limit", (1e8, 0.000000001, 1e18), "A level stops when the damping exceeds this (LevelParams::lambda_max)."),
        float("selection_fraction", "Selection fraction", (0.5, 0.0, 1.0), "The share of the region pixels kept, those of largest gradient, above 0 (SelectionConfig::min_fraction)."),
        float("min_weight", "Weight floor", (0.05, 0.0, 1.0), "The floor of every pixel weight, above 0 (SelectionConfig::min_weight)."),
        flag("border_is_outside", "Border is outside", true, "Whether the pixels beyond the frame edge count as outside the region in the distance transform (SelectionConfig::border_is_outside)."),
        flag("ncc", "Coarse search", false, "Whether a search by normalised cross correlation over whole pixel shifts runs before the registration (NccConfig::enabled)."),
        int("ncc_level", "Coarse search level", (-1, -1, 15), "The pyramid level of the coarse search; -1 is the coarsest level in use (NccConfig::level)."),
        int("ncc_radius", "Coarse search radius", (6, 0, 4096), "The radius of the search window in pixels of that level (NccConfig::radius)."),
        int("ncc_step", "Coarse search step", (1, 1, 4096), "The step of the window scan in pixels of that level (NccConfig::step)."),
        float("ncc_min_valid_fraction", "Coarse search valid share", (0.6, 0.0, 1.0), "A shift is scored only when this share of the selected pixels falls inside the frame, above 0 (NccConfig::min_valid_fraction)."),
        float("ncc_min_score", "Coarse search least score", (0.0, -1.0, 1.0), "The best score must reach this for the shift to be taken (NccConfig::min_score)."),
        float("inlier_threshold", "Inlier threshold", (0.05, 0.0, 1e6), "A pixel is an inlier when its absolute residual is at most this, in grey units (Config::inlier_threshold)."),
        int("min_valid_pixels", "Least valid pixels", (12, 1, 1_000_000_000), "A level is abandoned when fewer pixels than this fall inside the frame (Config::min_valid_pixels)."),
        choice("refusal", "Refused frame", "hold", &["hold", "fail"], "What a frame the tracker refuses does: hold keeps the pose of its neighbour towards the reference and says so in the report, fail ends the run as unsupported."),
    ],
    layers: &[],
};

/// The configuration of the tracker for a set of values.
pub fn config(values: &Values) -> Config {
    let ramp = values.float("boundary_ramp");
    let level = values.int("ncc_level");
    Config {
        pyramid: PyramidConfig {
            levels: values.int("pyramid_levels") as usize,
            scale: values.float("pyramid_scale"),
            kernel_taps: values.int("pyramid_taps") as usize,
            min_size: values.int("pyramid_min_size") as usize,
        },
        dof: match values.choice("dof") {
            "translation" => Dof::Translation,
            "trans-scale" => Dof::TransScale,
            "similarity" => Dof::Similarity,
            "homography" => Dof::Homography,
            _ => Dof::Affine,
        },
        rule: match values.choice("rule") {
            "forward-additive" => UpdateRule::ForwardAdditive,
            "forward-compositional" => UpdateRule::ForwardCompositional,
            "esm" => UpdateRule::Esm,
            _ => UpdateRule::InverseCompositional,
        },
        illumination: match values.choice("illumination") {
            "joint" => Illumination::Joint,
            "alternate" => Illumination::Alternate,
            _ => Illumination::Off,
        },
        lm: LmConfig {
            lambda_init: values.float("lambda_init"),
            lambda_up: values.float("lambda_up"),
            lambda_down: values.float("lambda_down"),
            lambda_min: values.float("lambda_min"),
            damping: match values.choice("damping") {
                "levenberg" => Damping::Levenberg,
                _ => Damping::Marquardt,
            },
            acceptance: match values.choice("acceptance") {
                "decrease-or-equal" => Acceptance::DecreaseOrEqual,
                "gain-ratio" => Acceptance::GainRatio(values.float("gain_ratio")),
                _ => Acceptance::Decrease,
            },
        },
        level_params: vec![LevelParams {
            max_iterations: values.int("max_iterations") as usize,
            max_rejections: values.int("max_rejections") as usize,
            step_tol: values.float("step_tol"),
            cost_tol: values.float("cost_tol"),
            lambda_max: values.float("lambda_max"),
        }],
        selection: SelectionConfig {
            min_fraction: values.float("selection_fraction"),
            boundary: match values.choice("boundary") {
                "linear" => BoundaryWeight::Linear { ramp },
                "smoothstep" => BoundaryWeight::Smoothstep { ramp },
                "quadratic" => BoundaryWeight::Quadratic { ramp },
                _ => BoundaryWeight::Off,
            },
            min_weight: values.float("min_weight"),
            border_is_outside: values.flag("border_is_outside"),
        },
        ncc: NccConfig {
            enabled: values.flag("ncc"),
            level: usize::try_from(level).ok(),
            radius: values.int("ncc_radius") as usize,
            step: values.int("ncc_step") as usize,
            min_valid_fraction: values.float("ncc_min_valid_fraction"),
            min_score: values.float("ncc_min_score"),
        },
        velocity: match values.choice("velocity") {
            "constant-image-space" => Velocity::ConstantImageSpace,
            _ => Velocity::Off,
        },
        inlier_threshold: values.float("inlier_threshold"),
        min_valid_pixels: values.int("min_valid_pixels") as usize,
    }
}

/// The eight corner values as four points, or `None` when all eight are 0.
pub fn given_corners(values: &Values) -> Option<[(f64, f64); 4]> {
    let given = [
        (values.float("x0"), values.float("y0")),
        (values.float("x1"), values.float("y1")),
        (values.float("x2"), values.float("y2")),
        (values.float("x3"), values.float("y3")),
    ];
    let all_zero = given.iter().all(|p| p.0 == 0.0 && p.1 == 0.0);
    (!all_zero).then_some(given)
}

/// The four corners of the region for a set of values and a frame size: the eight values, or
/// the centres of the four corner pixels of the frame when all eight are 0.
pub fn corners(values: &Values, width: u32, height: u32) -> [(f64, f64); 4] {
    given_corners(values).unwrap_or_else(|| {
        let (right, bottom) = (f64::from(width) - 1.0, f64::from(height) - 1.0);
        [(0.0, 0.0), (right, 0.0), (right, bottom), (0.0, bottom)]
    })
}

/// Whether four corners turn the same way at every corner, strictly.
pub fn convex(p: &[(f64, f64); 4]) -> bool {
    let turns = [(0, 1, 2), (1, 2, 3), (2, 3, 0), (3, 0, 1)].map(|(i, j, k)| {
        let (a, b, c) = (p[i], p[j], p[k]);
        (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0)
    });
    turns.iter().all(|t| *t > 0.0) || turns.iter().all(|t| *t < 0.0)
}

/// The grey image of one raw frame under the rule of the module text. The error is the refusal
/// of the crate, for a sample that is not a finite number.
pub fn grey(
    frame: &[u8],
    width: usize,
    height: usize,
    depth: u32,
    weights: [f64; 3],
) -> Result<Image, String> {
    let count = width * height;
    let luma = |r: f64, g: f64, b: f64| weights[0] * r + weights[1] * g + weights[2] * b;
    let mut data = Vec::with_capacity(count);
    match depth {
        8 => {
            for px in frame.as_chunks::<4>().0 {
                let c = |i: usize| f64::from(px[i]) / 255.0;
                data.push(luma(c(0), c(1), c(2)));
            }
        }
        16 => {
            for px in frame.as_chunks::<8>().0 {
                let c =
                    |i: usize| f64::from(u16::from_le_bytes([px[2 * i], px[2 * i + 1]])) / 65535.0;
                data.push(luma(c(0), c(1), c(2)));
            }
        }
        _ => {
            let plane = |p: usize, i: usize| {
                let o = (p * count + i) * 4;
                f64::from(f32::from_le_bytes([
                    frame[o],
                    frame[o + 1],
                    frame[o + 2],
                    frame[o + 3],
                ]))
            };
            // the planes are G, B, R, A
            for i in 0..count {
                data.push(luma(plane(2, i), plane(0, i), plane(1, i)));
            }
        }
    }
    Image::new(width, height, data).map_err(|e| e.to_string())
}

/// The report word of a stop reason.
pub fn stop_word(stop: Option<StopReason>) -> &'static str {
    match stop {
        None => "none",
        Some(StopReason::StepTol) => "step-tol",
        Some(StopReason::CostTol) => "cost-tol",
        Some(StopReason::LambdaMax) => "lambda-max",
        Some(StopReason::MaxIterations) => "max-iterations",
        Some(StopReason::MaxRejections) => "max-rejections",
        Some(StopReason::NoPixels) => "no-pixels",
    }
}

fn pose_ok(pose: &Pose) -> bool {
    pose.h
        .iter()
        .all(|v| v.is_finite() && v.abs() <= POSE_LIMIT)
}

/// What one step of a [`Walker`] gives.
enum Outcome {
    /// The tracker solved the frame.
    Tracked(Box<FrameResult>),
    /// The tracker refused the frame, with the reason.
    Refused(String),
}

/// A tracker that walks away from the reference frame one frame at a time and holds at most
/// [`WINDOW`] frames. The same walker serves both directions: the frames before the reference
/// are handed to it in falling order.
struct Walker {
    cfg: Config,
    template: Image,
    region: Region,
    tracker: Tracker,
    /// The last one or two frames that have a pose, oldest first.
    recent: Vec<(Image, Pose)>,
}

impl Walker {
    /// A walker on a reference frame and a region, with the measured figures of the identity
    /// pose on the reference frame.
    fn start(
        cfg: &Config,
        template: &Image,
        region: &Region,
    ) -> Result<(Walker, FrameResult), String> {
        let recent = vec![(template.clone(), Pose::identity())];
        let tracker = Walker::build(cfg, template, region, &recent)?;
        let measured = tracker
            .result(0)
            .cloned()
            .ok_or("the reference frame has no pose")?;
        let walker = Walker {
            cfg: cfg.clone(),
            template: template.clone(),
            region: region.clone(),
            tracker,
            recent,
        };
        Ok((walker, measured))
    }

    fn build(
        cfg: &Config,
        template: &Image,
        region: &Region,
        recent: &[(Image, Pose)],
    ) -> Result<Tracker, String> {
        let mut tracker = Tracker::new(cfg.clone(), template, region).map_err(|e| e.to_string())?;
        for (image, pose) in recent {
            let index = tracker.add_frame(image).map_err(|e| e.to_string())?;
            tracker.set_pose(index, *pose).map_err(|e| e.to_string())?;
        }
        Ok(tracker)
    }

    fn rebuild(&mut self) -> Result<(), String> {
        self.tracker = Walker::build(&self.cfg, &self.template, &self.region, &self.recent)?;
        Ok(())
    }

    /// The pose of the last frame that has one.
    fn last(&self) -> Pose {
        self.recent[self.recent.len() - 1].1
    }

    /// Track the next frame. A refused frame leaves the walker at the last frame that has a
    /// pose, so the frame after it starts from that pose. `Err` means the walker itself can no
    /// longer be built.
    fn step(&mut self, image: Result<Image, String>) -> Result<Outcome, String> {
        if self.tracker.frame_count() >= WINDOW {
            self.rebuild()?;
        }
        let solved = image.and_then(|image| {
            let result = self
                .tracker
                .add_frame(&image)
                .and_then(|index| self.tracker.track(index - 1, index))
                .map_err(|e| e.to_string())?
                .pop()
                .ok_or("the tracker gave no result")?;
            if pose_ok(&result.pose) {
                Ok((image, result))
            } else {
                Err("the pose is not finite or beyond the limit of a key value".to_string())
            }
        });
        match solved {
            Ok((image, result)) => {
                if self.recent.len() == 2 {
                    self.recent.remove(0);
                }
                self.recent.push((image, result.pose));
                Ok(Outcome::Tracked(Box::new(result)))
            }
            Err(why) => {
                let keep = self.recent.len() - 1;
                self.recent.drain(..keep);
                self.rebuild()?;
                Ok(Outcome::Refused(why))
            }
        }
    }
}

/// One row of the result: the pose of a frame and how it was found.
struct Row {
    frame: u64,
    pose: Pose,
    source: &'static str,
    stop: &'static str,
    converged: bool,
    iterations: usize,
    rms: f64,
    valid: f64,
    inliers: f64,
    gain: f64,
    offset: f64,
    why: Option<String>,
}

impl Row {
    fn solved(frame: u64, source: &'static str, r: &FrameResult) -> Row {
        Row {
            frame,
            pose: r.pose,
            source,
            stop: stop_word(r.stop),
            converged: r.converged,
            iterations: r.iterations,
            rms: r.rms,
            valid: r.valid_fraction,
            inliers: r.inlier_fraction,
            gain: r.gain,
            offset: r.offset,
            why: None,
        }
    }

    fn held(frame: u64, pose: Pose, why: &str) -> Row {
        Row {
            frame,
            pose,
            source: "held",
            stop: "refused",
            converged: false,
            iterations: 0,
            rms: 0.0,
            valid: 0.0,
            inliers: 0.0,
            gain: 0.0,
            offset: 0.0,
            why: Some(why.to_string()),
        }
    }
}

/// A finite number as text; any other value as 0, so a table cell is always a number.
fn cell(v: f64) -> String {
    if v.is_finite() {
        format!("{v}")
    } else {
        "0".to_string()
    }
}

/// The tracking of one run: the configuration, the two files and the state between frames.
struct Job {
    cfg: Config,
    weights: [f64; 3],
    corners: [(f64, f64); 4],
    width: usize,
    height: usize,
    depth: u32,
    reference: u64,
    strict: bool,
    data: Option<Sink>,
    report: Option<Sink>,
    /// The grey images of the frames before the reference, in stream order.
    waiting: Vec<Result<Image, String>>,
    /// The walker of the frames after the reference, once the reference has been read.
    walker: Option<Walker>,
    /// Why there is no walker after the reference has been read.
    lost: String,
    /// The pose a refused frame keeps.
    last: Pose,
}

impl Job {
    /// Open the files a run asks for, or `None` when it asks for neither.
    fn open(plan: &Plan) -> Result<Option<Job>, StageError> {
        let values = &plan.values;
        let (data, report) = (values.text("data"), values.text("report"));
        if data.is_empty() && report.is_empty() {
            return Ok(None);
        }
        let mut job = Job {
            cfg: config(values),
            weights: [
                values.float("luma_r"),
                values.float("luma_g"),
                values.float("luma_b"),
            ],
            corners: corners(values, plan.input.width, plan.input.height),
            width: plan.input.width as usize,
            height: plan.input.height as usize,
            depth: plan.input.depth,
            reference: values.int("reference") as u64,
            strict: values.choice("refusal") == "fail",
            data: None,
            report: None,
            waiting: Vec::new(),
            walker: None,
            lost: String::new(),
            last: Pose::identity(),
        };
        if !data.is_empty() {
            let mut sink = Sink::create(data)?;
            sink.line(keys::FORM)?;
            sink.line(
                "# track-planar: m00 to m22, row major, from the reference frame to frame N",
            )?;
            sink.line("start 0")?;
            job.data = Some(sink);
        }
        if !report.is_empty() {
            let mut sink = Sink::create(report)?;
            sink.line(REPORT_FORM)?;
            sink.line("# frame source stop converged iterations rms valid_fraction inlier_fraction gain offset")?;
            job.report = Some(sink);
        }
        Ok(Some(job))
    }

    /// Write the lines of one row, or end the run when the row is a refusal and `refusal` is
    /// `fail`.
    fn emit(&mut self, row: &Row) -> Result<(), StageError> {
        if let (true, Some(why)) = (self.strict, &row.why) {
            return Err(StageError::unsupported(format!(
                "frame {}: the tracker refuses the frame: {why}",
                row.frame
            )));
        }
        if let Some(sink) = self.data.as_mut() {
            for (track, value) in TRACKS.iter().zip(row.pose.h.iter()) {
                sink.line(&format!(
                    "key {track} {} {} linear",
                    row.frame,
                    cell(*value)
                ))?;
            }
        }
        if let Some(sink) = self.report.as_mut() {
            if let Some(why) = &row.why {
                let why: String = why
                    .chars()
                    .map(|c| if c.is_control() { ' ' } else { c })
                    .collect();
                sink.line(&format!("# frame {}: {why}", row.frame))?;
            }
            sink.line(&format!(
                "{} {} {} {} {} {} {} {} {} {}",
                row.frame,
                row.source,
                row.stop,
                u8::from(row.converged),
                row.iterations,
                cell(row.rms),
                cell(row.valid),
                cell(row.inliers),
                cell(row.gain),
                cell(row.offset)
            ))?;
        }
        Ok(())
    }

    /// Take one frame of the stream.
    fn frame(&mut self, index: u64, bytes: &[u8]) -> Result<(), StageError> {
        let image = grey(bytes, self.width, self.height, self.depth, self.weights);
        if index < self.reference {
            self.waiting.push(image);
            return Ok(());
        }
        if index == self.reference {
            return self.begin(image);
        }
        let row = match self.walker.as_mut() {
            None => Row::held(index, self.last, &self.lost),
            Some(walker) => match walker.step(image) {
                Ok(Outcome::Tracked(result)) => {
                    self.last = result.pose;
                    Row::solved(index, "tracked", &result)
                }
                Ok(Outcome::Refused(why)) => Row::held(index, self.last, &why),
                Err(why) => {
                    self.walker = None;
                    self.lost = why;
                    Row::held(index, self.last, &self.lost)
                }
            },
        };
        self.emit(&row)
    }

    /// The reference frame has been read: track the frames that waited for it, backwards, and
    /// start the walker of the frames after it.
    fn begin(&mut self, image: Result<Image, String>) -> Result<(), StageError> {
        let before = std::mem::take(&mut self.waiting);
        let setup = image.and_then(|template| {
            let region = Region::from_quad(self.width, self.height, self.corners)
                .map_err(|e| e.to_string())?;
            let (walker, measured) = Walker::start(&self.cfg, &template, &region)?;
            Ok((template, region, walker, measured))
        });
        let (template, region, walker, measured) = match setup {
            Ok(parts) => parts,
            Err(why) => {
                for frame in 0..=self.reference {
                    self.emit(&Row::held(frame, Pose::identity(), &why))?;
                }
                self.lost = why;
                return Ok(());
            }
        };
        let mut rows: Vec<Row> = Vec::with_capacity(before.len());
        let mut back = Walker::start(&self.cfg, &template, &region)
            .map(|(walker, _)| walker)
            .map_err(|why| (Pose::identity(), why));
        for (frame, image) in before.into_iter().enumerate().rev() {
            let frame = frame as u64;
            let (stepped, last) = match back.as_mut() {
                Ok(walker) => (walker.step(image), walker.last()),
                Err((pose, why)) => (Ok(Outcome::Refused(why.clone())), *pose),
            };
            rows.push(match stepped {
                Ok(Outcome::Tracked(result)) => Row::solved(frame, "tracked", &result),
                Ok(Outcome::Refused(why)) => Row::held(frame, last, &why),
                Err(why) => {
                    let row = Row::held(frame, last, &why);
                    back = Err((last, why));
                    row
                }
            });
        }
        for row in rows.iter().rev() {
            self.emit(row)?;
        }
        self.emit(&Row::solved(self.reference, "reference", &measured))?;
        self.walker = Some(walker);
        Ok(())
    }

    /// The input has ended after `count` frames: move the files into place.
    fn end(mut self, count: u64) -> Result<(), StageError> {
        if count > 0 && count <= self.reference {
            return Err(StageError::bad_input(format!(
                "the reference frame {} is not among the {count} frames read",
                self.reference
            )));
        }
        if let Some(sink) = self.data.take() {
            sink.commit()?;
        }
        if let Some(sink) = self.report.take() {
            sink.commit()?;
        }
        Ok(())
    }
}

impl Stage for TrackPlanar {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    /// The geometry is the input's. The checks that need the values together run here, so that
    /// `--describe` refuses what a run refuses: a configuration the crate refuses and a region
    /// that is not convex are `unsupported`, and one path for both files is `usage`.
    fn output_size(&self, input: &FrameSpec, values: &Values) -> Result<(u32, u32), StageError> {
        config(values)
            .validate()
            .map_err(|e| StageError::unsupported(format!("{}: {e}", DEF.id)))?;
        if given_corners(values).is_some_and(|given| !convex(&given)) {
            return Err(StageError::unsupported(format!(
                "{}: the four corners do not go around a convex region",
                DEF.id
            )));
        }
        let data = values.text("data");
        if !data.is_empty() && data == values.text("report") {
            return Err(StageError::usage(format!(
                "{}: data and report name the same file",
                DEF.id
            )));
        }
        Ok((input.width, input.height))
    }

    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError> {
        let bytes = plan.input.buffer_len()?;
        let Some(mut job) = Job::open(plan)? else {
            return copy_frames(input, output, bytes, plan.frames);
        };
        let count = map_frames(
            input,
            output,
            bytes,
            bytes,
            plan.frames,
            |index, frame, out| {
                out.clear();
                out.extend_from_slice(frame);
                job.frame(index, frame)
            },
        )?;
        job.end(count)?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(pairs: &[(&str, &str)]) -> Values {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Values::parse(&DEF, &pairs).unwrap()
    }

    #[test]
    fn the_defaults_are_the_defaults_of_the_tracker() {
        let cfg = config(&Values::defaults(&DEF));
        assert_eq!(cfg, Config::default());
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        let dof = |word| config(&set(&[("dof", word)])).dof;
        assert_eq!(dof("translation"), Dof::Translation);
        assert_eq!(dof("trans-scale"), Dof::TransScale);
        assert_eq!(dof("similarity"), Dof::Similarity);
        assert_eq!(dof("affine"), Dof::Affine);
        assert_eq!(dof("homography"), Dof::Homography);
        let rule = |word| config(&set(&[("rule", word)])).rule;
        assert_eq!(rule("forward-additive"), UpdateRule::ForwardAdditive);
        assert_eq!(
            rule("forward-compositional"),
            UpdateRule::ForwardCompositional
        );
        assert_eq!(
            rule("inverse-compositional"),
            UpdateRule::InverseCompositional
        );
        assert_eq!(rule("esm"), UpdateRule::Esm);
        let light = |word| config(&set(&[("illumination", word)])).illumination;
        assert_eq!(light("off"), Illumination::Off);
        assert_eq!(light("joint"), Illumination::Joint);
        assert_eq!(light("alternate"), Illumination::Alternate);
        assert_eq!(
            config(&set(&[("damping", "levenberg")])).lm.damping,
            Damping::Levenberg
        );
        assert_eq!(
            config(&set(&[("damping", "marquardt")])).lm.damping,
            Damping::Marquardt
        );
        let accept = |pairs: &[(&str, &str)]| config(&set(pairs)).lm.acceptance;
        assert_eq!(accept(&[("acceptance", "decrease")]), Acceptance::Decrease);
        assert_eq!(
            accept(&[("acceptance", "decrease-or-equal")]),
            Acceptance::DecreaseOrEqual
        );
        assert_eq!(
            accept(&[("acceptance", "gain-ratio"), ("gain_ratio", "0.25")]),
            Acceptance::GainRatio(0.25)
        );
        assert_eq!(
            accept(&[("gain_ratio", "0.25")]),
            Acceptance::Decrease,
            "gain_ratio is read only with acceptance gain-ratio"
        );
        let edge = |pairs: &[(&str, &str)]| config(&set(pairs)).selection.boundary;
        assert_eq!(edge(&[("boundary_ramp", "7")]), BoundaryWeight::Off);
        assert_eq!(
            edge(&[("boundary", "linear")]),
            BoundaryWeight::Linear { ramp: 4.0 }
        );
        assert_eq!(
            edge(&[("boundary", "smoothstep"), ("boundary_ramp", "2.5")]),
            BoundaryWeight::Smoothstep { ramp: 2.5 }
        );
        assert_eq!(
            edge(&[("boundary", "quadratic"), ("boundary_ramp", "9")]),
            BoundaryWeight::Quadratic { ramp: 9.0 }
        );
        assert_eq!(
            config(&set(&[("velocity", "constant-image-space")])).velocity,
            Velocity::ConstantImageSpace
        );
    }

    #[test]
    fn every_number_reaches_its_field() {
        let cfg = config(&set(&[
            ("pyramid_levels", "3"),
            ("pyramid_scale", "1.5"),
            ("pyramid_taps", "7"),
            ("pyramid_min_size", "9"),
            ("lambda_init", "0.5"),
            ("lambda_up", "4"),
            ("lambda_down", "0.25"),
            ("lambda_min", "0.125"),
            ("max_iterations", "11"),
            ("max_rejections", "6"),
            ("step_tol", "0.0625"),
            ("cost_tol", "0.03125"),
            ("lambda_max", "512"),
            ("selection_fraction", "0.75"),
            ("min_weight", "0.375"),
            ("border_is_outside", "off"),
            ("ncc", "on"),
            ("ncc_level", "2"),
            ("ncc_radius", "5"),
            ("ncc_step", "3"),
            ("ncc_min_valid_fraction", "0.875"),
            ("ncc_min_score", "-0.5"),
            ("inlier_threshold", "0.1875"),
            ("min_valid_pixels", "17"),
        ]));
        assert_eq!(
            cfg.pyramid,
            PyramidConfig {
                levels: 3,
                scale: 1.5,
                kernel_taps: 7,
                min_size: 9
            }
        );
        assert_eq!(
            (
                cfg.lm.lambda_init,
                cfg.lm.lambda_up,
                cfg.lm.lambda_down,
                cfg.lm.lambda_min
            ),
            (0.5, 4.0, 0.25, 0.125)
        );
        assert_eq!(
            cfg.level_params,
            [LevelParams {
                max_iterations: 11,
                max_rejections: 6,
                step_tol: 0.0625,
                cost_tol: 0.03125,
                lambda_max: 512.0
            }]
        );
        assert_eq!(
            (
                cfg.selection.min_fraction,
                cfg.selection.min_weight,
                cfg.selection.border_is_outside
            ),
            (0.75, 0.375, false)
        );
        assert_eq!(
            cfg.ncc,
            NccConfig {
                enabled: true,
                level: Some(2),
                radius: 5,
                step: 3,
                min_valid_fraction: 0.875,
                min_score: -0.5
            }
        );
        assert_eq!((cfg.inlier_threshold, cfg.min_valid_pixels), (0.1875, 17));
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn the_region_is_the_whole_frame_unless_corners_are_given() {
        assert_eq!(
            corners(&Values::defaults(&DEF), 5, 3),
            [(0.0, 0.0), (4.0, 0.0), (4.0, 2.0), (0.0, 2.0)]
        );
        let given = set(&[("x1", "6"), ("x2", "6"), ("y2", "4.5"), ("y3", "4.5")]);
        assert_eq!(
            corners(&given, 50, 30),
            [(0.0, 0.0), (6.0, 0.0), (6.0, 4.5), (0.0, 4.5)]
        );
    }

    #[test]
    fn a_region_is_convex_in_either_orientation() {
        let square = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        assert!(convex(&square));
        let mut turned = square;
        turned.reverse();
        assert!(convex(&turned));
        assert!(!convex(&[(0.0, 0.0), (4.0, 4.0), (4.0, 0.0), (0.0, 4.0)]));
        assert!(!convex(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]));
    }

    #[test]
    fn the_grey_rule_per_depth() {
        let weights = [0.25, 0.5, 0.125];
        let g8 = grey(&[255, 0, 0, 9, 0, 255, 51, 9], 2, 1, 8, weights).unwrap();
        assert_eq!(g8.data(), [0.25, 0.5 + 0.125 * (51.0 / 255.0)]);
        let mut b16 = Vec::new();
        for c in [0u16, 65535, 65535, 7] {
            b16.extend_from_slice(&c.to_le_bytes());
        }
        assert_eq!(grey(&b16, 1, 1, 16, weights).unwrap().data(), [0.625]);
        // one pixel in the planes G, B, R, A
        let mut b32 = Vec::new();
        for c in [1.0f32, 0.0, 2.0, 0.5] {
            b32.extend_from_slice(&c.to_le_bytes());
        }
        assert_eq!(grey(&b32, 1, 1, 32, weights).unwrap().data(), [1.0]);
        b32[0..4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(grey(&b32, 1, 1, 32, weights).is_err());
    }

    #[test]
    fn the_stop_words() {
        assert_eq!(stop_word(None), "none");
        assert_eq!(stop_word(Some(StopReason::StepTol)), "step-tol");
        assert_eq!(stop_word(Some(StopReason::CostTol)), "cost-tol");
        assert_eq!(stop_word(Some(StopReason::LambdaMax)), "lambda-max");
        assert_eq!(stop_word(Some(StopReason::MaxIterations)), "max-iterations");
        assert_eq!(stop_word(Some(StopReason::MaxRejections)), "max-rejections");
        assert_eq!(stop_word(Some(StopReason::NoPixels)), "no-pixels");
    }

    #[test]
    fn a_table_cell_is_always_a_number() {
        assert_eq!(cell(0.5), "0.5");
        assert_eq!(cell(-2.0), "-2");
        assert_eq!(cell(f64::NAN), "0");
        assert_eq!(cell(f64::INFINITY), "0");
        assert_eq!(cell(1e-9), "0.000000001");
    }

    /// A smooth made up pattern moved by whole pixels.
    fn pattern(width: usize, height: usize, dx: i64, dy: i64) -> Image {
        Image::from_fn(width, height, |x, y| {
            let (u, v) = ((x as i64 - dx) as f64, (y as i64 - dy) as f64);
            0.5 + ((u * 0.35).sin() + (v * 0.27).cos() + ((u + v) * 0.11).sin()) / 6.0
        })
    }

    #[test]
    fn a_walker_gives_the_poses_of_one_tracker_that_holds_every_frame() {
        let cfg = Config {
            velocity: Velocity::ConstantImageSpace,
            ..Config::default()
        };
        let frames: Vec<Image> = (0..(2 * WINDOW as i64 + 3))
            .map(|i| pattern(40, 32, i.rem_euclid(3), (i / 2).rem_euclid(2)))
            .collect();
        let region =
            Region::from_quad(40, 32, [(8.0, 6.0), (31.0, 6.0), (31.0, 25.0), (8.0, 25.0)])
                .unwrap();
        let mut whole = Tracker::new(cfg.clone(), &frames[0], &region).unwrap();
        for frame in &frames {
            whole.add_frame(frame).unwrap();
        }
        whole.set_pose(0, Pose::identity()).unwrap();
        let expected = whole.track(0, frames.len() - 1).unwrap();
        let (mut walker, measured) = Walker::start(&cfg, &frames[0], &region).unwrap();
        assert_eq!(measured.pose, Pose::identity());
        for (frame, want) in frames[1..].iter().zip(&expected) {
            let Ok(Outcome::Tracked(got)) = walker.step(Ok(frame.clone())) else {
                panic!("the walker refused a frame");
            };
            assert_eq!(got.pose, want.pose);
            assert_eq!(got.iterations, want.iterations);
            assert!(walker.tracker.frame_count() <= WINDOW);
        }
    }

    #[test]
    fn a_refused_frame_leaves_the_walker_at_the_last_pose() {
        let cfg = Config::default();
        let frames: Vec<Image> = (0..4).map(|i| pattern(40, 32, i, 0)).collect();
        let region =
            Region::from_quad(40, 32, [(8.0, 6.0), (31.0, 6.0), (31.0, 25.0), (8.0, 25.0)])
                .unwrap();
        let (mut walker, _) = Walker::start(&cfg, &frames[0], &region).unwrap();
        assert!(matches!(
            walker.step(Ok(frames[1].clone())),
            Ok(Outcome::Tracked(_))
        ));
        let before = walker.last();
        let refused = walker.step(Err("a made up refusal".to_string()));
        assert!(matches!(refused, Ok(Outcome::Refused(why)) if why == "a made up refusal"));
        assert_eq!(walker.last(), before);
        assert!(matches!(
            walker.step(Ok(frames[2].clone())),
            Ok(Outcome::Tracked(_))
        ));
    }
}
