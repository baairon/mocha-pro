//! Configuration of the tracker and the enumeration of its discrete choices.
//!
//! Every choice a tracker could make differently is a field here. The defaults are plain
//! engineering choices stated on each field; they make no claim about any other product.

use crate::error::Error;
use crate::model::Dof;
use crate::pyramid::PyramidConfig;

/// The warp update rule, after Baker and Matthews, "Lucas-Kanade 20 Years On: A Unifying
/// Framework", IJCV 56(3), 2004, and Benhimane and Malis, "Real-time image-based tracking of
/// planes using Efficient Second-order Minimization", IROS 2004.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateRule {
    /// Forward additive (Baker and Matthews section 2.2): the parameters are updated by addition.
    ForwardAdditive,
    /// Forward compositional (section 3.1): the pose is composed with the incremental warp.
    ForwardCompositional,
    /// Inverse compositional (section 3.2): the roles of image and template are exchanged; the
    /// pose is composed with the inverse of the incremental warp.
    InverseCompositional,
    /// Efficient second order minimisation (Benhimane and Malis 2004, the mean of the
    /// image and template Jacobians), with the incremental warp composed as in the forward
    /// compositional rule. The group exponential of the original paper is replaced by the
    /// parameterised warp of [`crate::model`], which agrees to first order.
    Esm,
}

impl UpdateRule {
    /// All rules.
    pub const ALL: [UpdateRule; 4] = [
        UpdateRule::ForwardAdditive,
        UpdateRule::ForwardCompositional,
        UpdateRule::InverseCompositional,
        UpdateRule::Esm,
    ];

    /// Short name.
    pub fn name(self) -> &'static str {
        match self {
            UpdateRule::ForwardAdditive => "forward-additive",
            UpdateRule::ForwardCompositional => "forward-compositional",
            UpdateRule::InverseCompositional => "inverse-compositional",
            UpdateRule::Esm => "esm",
        }
    }
}

/// The two parameter illumination model: template side, `T' = (1 + gain) T + offset`, compared
/// with the warped frame (Hager and Belhumeur, "Efficient Region Tracking With Parametric Models
/// of Geometry and Illumination", IEEE PAMI 20(10), 1998, the linear illumination subspace; here
/// the subspace is spanned by the template and a constant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Illumination {
    /// No illumination model: gain 0 and offset 0 throughout.
    Off,
    /// Gain and offset are unknowns of the same linear system as the pose.
    Joint,
    /// At the start of every iteration the weighted least squares gain and offset for the
    /// current pose are set in closed form; the step then solves the pose only.
    Alternate,
}

impl Illumination {
    /// All settings.
    pub const ALL: [Illumination; 3] = [
        Illumination::Off,
        Illumination::Joint,
        Illumination::Alternate,
    ];

    /// Short name.
    pub fn name(self) -> &'static str {
        match self {
            Illumination::Off => "off",
            Illumination::Joint => "joint",
            Illumination::Alternate => "alternate",
        }
    }
}

/// How the damping term is added to the normal matrix `A` (Levenberg, 1944; Marquardt, "An
/// Algorithm for Least-Squares Estimation of Nonlinear Parameters", SIAM J. Appl. Math. 11(2),
/// 1963; Baker and Matthews 2004, section 4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Damping {
    /// `A + lambda * I` (Levenberg).
    Levenberg,
    /// `A + lambda * diag(A)` (Marquardt); a zero diagonal entry gets `lambda`.
    Marquardt,
}

/// The test that accepts or rejects a trial step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Acceptance {
    /// Accept when the weighted mean squared residual falls strictly.
    Decrease,
    /// Accept when the weighted mean squared residual does not rise.
    DecreaseOrEqual,
    /// Accept when the gain ratio (actual over predicted reduction, Nielsen 1999 style) is at
    /// least this value, which must be in `[0, 1)`.
    GainRatio(f64),
}

/// Damping and step control of the Levenberg-Marquardt iteration.
#[derive(Debug, Clone, PartialEq)]
pub struct LmConfig {
    /// Initial lambda at the start of every level, at least 0.
    pub lambda_init: f64,
    /// Factor applied to lambda after a rejected step, above 1.
    pub lambda_up: f64,
    /// Factor applied to lambda after an accepted step, in `(0, 1]`.
    pub lambda_down: f64,
    /// Lambda is never lowered below this value (at least 0).
    pub lambda_min: f64,
    /// Damping form.
    pub damping: Damping,
    /// Acceptance test.
    pub acceptance: Acceptance,
}

impl Default for LmConfig {
    fn default() -> Self {
        LmConfig {
            lambda_init: 1e-4,
            lambda_up: 10.0,
            lambda_down: 0.1,
            lambda_min: 1e-9,
            damping: Damping::Marquardt,
            acceptance: Acceptance::Decrease,
        }
    }
}

/// Iteration limits and convergence tests of one pyramid level.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelParams {
    /// Iteration limit (accepted or rejected steps), at least 1.
    pub max_iterations: usize,
    /// Consecutive rejections after which the level stops, at least 1.
    pub max_rejections: usize,
    /// Stop when the largest motion of a region corner in one accepted step, in level pixels,
    /// is below this.
    pub step_tol: f64,
    /// Stop when the relative fall of the mean cost in one accepted step is below this.
    pub cost_tol: f64,
    /// Stop (counted as converged) when lambda exceeds this.
    pub lambda_max: f64,
}

impl Default for LevelParams {
    fn default() -> Self {
        LevelParams {
            max_iterations: 30,
            max_rejections: 10,
            step_tol: 1e-3,
            cost_tol: 1e-9,
            lambda_max: 1e8,
        }
    }
}

/// Weight of a template pixel from its distance `d` (in level pixels) to the outside of the mask,
/// with `t = min(1, d / ramp)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoundaryWeight {
    /// All weights 1; no distance transform is computed.
    Off,
    /// `t`.
    Linear {
        /// Distance at which the weight reaches 1, above 0.
        ramp: f64,
    },
    /// `t * t * (3 - 2 t)`.
    Smoothstep {
        /// Distance at which the weight reaches 1, above 0.
        ramp: f64,
    },
    /// `t * t`.
    Quadratic {
        /// Distance at which the weight reaches 1, above 0.
        ramp: f64,
    },
}

impl BoundaryWeight {
    /// The weight at distance `d`, before the floor of [`SelectionConfig::min_weight`].
    pub fn weight(self, d: f64) -> f64 {
        let t = |ramp: f64| {
            let t = d / ramp;
            if t > 1.0 {
                1.0
            } else {
                t
            }
        };
        match self {
            BoundaryWeight::Off => 1.0,
            BoundaryWeight::Linear { ramp } => t(ramp),
            BoundaryWeight::Smoothstep { ramp } => {
                let t = t(ramp);
                t * t * (3.0 - 2.0 * t)
            }
            BoundaryWeight::Quadratic { ramp } => {
                let t = t(ramp);
                t * t
            }
        }
    }
}

/// Template pixel selection and weighting.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectionConfig {
    /// Fraction of the mask pixels kept, those of largest template gradient magnitude, in
    /// `(0, 1]`. At least one pixel is always kept.
    pub min_fraction: f64,
    /// Boundary weight.
    pub boundary: BoundaryWeight,
    /// Floor of every weight, in `(0, 1]`.
    pub min_weight: f64,
    /// Count the pixels beyond the template edge as outside the mask in the distance transform.
    pub border_is_outside: bool,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        SelectionConfig {
            min_fraction: 0.5,
            boundary: BoundaryWeight::Off,
            min_weight: 0.05,
            border_is_outside: true,
        }
    }
}

/// Optional coarse search by normalised cross correlation (Lewis, "Fast Normalized
/// Cross-Correlation", Vision Interface 1995; Brunelli, Template Matching Techniques in Computer
/// Vision, 2009, chapter 3), over integer translations of the predicted pose at one pyramid
/// level.
#[derive(Debug, Clone, PartialEq)]
pub struct NccConfig {
    /// Run the search.
    pub enabled: bool,
    /// Pyramid level of the search; `None` is the coarsest level in use.
    pub level: Option<usize>,
    /// Window radius in pixels of that level.
    pub radius: usize,
    /// Step of the window scan in pixels of that level, at least 1.
    pub step: usize,
    /// A shift is scored only when at least this fraction of the selected pixels fall inside the
    /// frame, in `(0, 1]`.
    pub min_valid_fraction: f64,
    /// The best score must reach this (in `[-1, 1]`) for the shift to be taken.
    pub min_score: f64,
}

impl Default for NccConfig {
    fn default() -> Self {
        NccConfig {
            enabled: false,
            level: None,
            radius: 6,
            step: 1,
            min_valid_fraction: 0.6,
            min_score: 0.0,
        }
    }
}

/// Motion prediction for the first guess of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Velocity {
    /// The first guess is the pose of the neighbouring frame already solved.
    Off,
    /// The image space motion between the two neighbouring solved frames, `P1 * inverse(P0)`, is
    /// applied once more.
    ConstantImageSpace,
}

/// The whole configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Pyramid. Default 4 levels, scale 2, 5 taps, minimum level size 12.
    pub pyramid: PyramidConfig,
    /// Degrees of freedom. Default affine.
    pub dof: Dof,
    /// Update rule. Default inverse compositional.
    pub rule: UpdateRule,
    /// Illumination model. Default off.
    pub illumination: Illumination,
    /// Damping and step control.
    pub lm: LmConfig,
    /// Limits per level, coarsest level first; levels beyond the list use the last entry.
    pub level_params: Vec<LevelParams>,
    /// Pixel selection and weighting.
    pub selection: SelectionConfig,
    /// Coarse search.
    pub ncc: NccConfig,
    /// Motion prediction.
    pub velocity: Velocity,
    /// A pixel is an inlier when its absolute residual is at most this (grey units; the nominal
    /// range is 0 to 1). Default 0.05.
    pub inlier_threshold: f64,
    /// A level is abandoned when fewer pixels than this fall inside the frame. Default 12.
    pub min_valid_pixels: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            pyramid: PyramidConfig::default(),
            dof: Dof::Affine,
            rule: UpdateRule::InverseCompositional,
            illumination: Illumination::Off,
            lm: LmConfig::default(),
            level_params: vec![LevelParams::default()],
            selection: SelectionConfig::default(),
            ncc: NccConfig::default(),
            velocity: Velocity::Off,
            inlier_threshold: 0.05,
            min_valid_pixels: 12,
        }
    }
}

fn bad(s: &str) -> Error {
    Error::Config(s.into())
}

impl Config {
    /// Check every documented range.
    pub fn validate(&self) -> Result<(), Error> {
        self.pyramid.validate()?;
        let lm = &self.lm;
        if !(lm.lambda_init >= 0.0 && lm.lambda_init.is_finite()) {
            return Err(bad("lm.lambda_init must be finite and at least 0"));
        }
        if !(lm.lambda_up > 1.0 && lm.lambda_up.is_finite()) {
            return Err(bad("lm.lambda_up must be above 1"));
        }
        if !(lm.lambda_down > 0.0 && lm.lambda_down <= 1.0) {
            return Err(bad("lm.lambda_down must be in (0, 1]"));
        }
        if !(lm.lambda_min >= 0.0 && lm.lambda_min.is_finite()) {
            return Err(bad("lm.lambda_min must be finite and at least 0"));
        }
        if let Acceptance::GainRatio(r) = lm.acceptance {
            if !(0.0..1.0).contains(&r) {
                return Err(bad("lm.acceptance gain ratio must be in [0, 1)"));
            }
        }
        if self.level_params.is_empty() {
            return Err(bad("level_params must not be empty"));
        }
        for lp in &self.level_params {
            if lp.max_iterations == 0 || lp.max_rejections == 0 {
                return Err(bad("level limits must be at least 1"));
            }
            if !(lp.step_tol >= 0.0 && lp.cost_tol >= 0.0 && lp.lambda_max > 0.0) {
                return Err(bad("level tolerances must be non negative"));
            }
        }
        let s = &self.selection;
        if !(s.min_fraction > 0.0 && s.min_fraction <= 1.0) {
            return Err(bad("selection.min_fraction must be in (0, 1]"));
        }
        if !(s.min_weight > 0.0 && s.min_weight <= 1.0) {
            return Err(bad("selection.min_weight must be in (0, 1]"));
        }
        match s.boundary {
            BoundaryWeight::Off => {}
            BoundaryWeight::Linear { ramp }
            | BoundaryWeight::Smoothstep { ramp }
            | BoundaryWeight::Quadratic { ramp } => {
                if !(ramp > 0.0 && ramp.is_finite()) {
                    return Err(bad("boundary ramp must be above 0"));
                }
            }
        }
        let n = &self.ncc;
        if n.step == 0 {
            return Err(bad("ncc.step must be at least 1"));
        }
        if !(n.min_valid_fraction > 0.0 && n.min_valid_fraction <= 1.0) {
            return Err(bad("ncc.min_valid_fraction must be in (0, 1]"));
        }
        if !(-1.0..=1.0).contains(&n.min_score) {
            return Err(bad("ncc.min_score must be in [-1, 1]"));
        }
        if !(self.inlier_threshold >= 0.0 && self.inlier_threshold.is_finite()) {
            return Err(bad("inlier_threshold must be finite and non negative"));
        }
        if self.min_valid_pixels == 0 {
            return Err(bad("min_valid_pixels must be at least 1"));
        }
        Ok(())
    }

    /// The limits of pyramid level `level` when `levels` levels are in use (level 0 finest).
    pub fn params_for_level(&self, level: usize, levels: usize) -> &LevelParams {
        let idx = levels - 1 - level;
        &self.level_params[idx.min(self.level_params.len() - 1)]
    }
}

/// One discrete dimension of the configuration space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dimension {
    /// Field name.
    pub name: &'static str,
    /// Names of the choices, in index order.
    pub choices: Vec<&'static str>,
}

/// The product of the discrete choices of [`Config`]: degrees of freedom, update rule,
/// illumination mode, damping form, acceptance test, boundary weight, motion prediction and the
/// coarse search switch. Continuous fields keep their defaults. A configuration is addressed by
/// one index in mixed radix, the first dimension varying slowest.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConfigSpace;

const ACCEPTANCE_NAMES: [&str; 3] = ["decrease", "decrease-or-equal", "gain-ratio"];
const DAMPING_NAMES: [&str; 2] = ["levenberg", "marquardt"];
const BOUNDARY_NAMES: [&str; 4] = ["off", "linear", "smoothstep", "quadratic"];
const VELOCITY_NAMES: [&str; 2] = ["off", "constant-image-space"];
const SEARCH_NAMES: [&str; 2] = ["off", "on"];

impl ConfigSpace {
    /// The dimensions with their choice names.
    pub fn dimensions(&self) -> Vec<Dimension> {
        vec![
            Dimension {
                name: "dof",
                choices: Dof::ALL.iter().map(|d| d.name()).collect(),
            },
            Dimension {
                name: "rule",
                choices: UpdateRule::ALL.iter().map(|d| d.name()).collect(),
            },
            Dimension {
                name: "illumination",
                choices: Illumination::ALL.iter().map(|d| d.name()).collect(),
            },
            Dimension {
                name: "damping",
                choices: DAMPING_NAMES.to_vec(),
            },
            Dimension {
                name: "acceptance",
                choices: ACCEPTANCE_NAMES.to_vec(),
            },
            Dimension {
                name: "boundary",
                choices: BOUNDARY_NAMES.to_vec(),
            },
            Dimension {
                name: "velocity",
                choices: VELOCITY_NAMES.to_vec(),
            },
            Dimension {
                name: "ncc",
                choices: SEARCH_NAMES.to_vec(),
            },
        ]
    }

    /// Number of configurations.
    pub fn size(&self) -> usize {
        self.dimensions().iter().map(|d| d.choices.len()).product()
    }

    /// The choice indices of configuration `index`, one per dimension (`None` past the end).
    pub fn digits(&self, index: usize) -> Option<Vec<usize>> {
        if index >= self.size() {
            return None;
        }
        let dims = self.dimensions();
        let mut rest = index;
        let mut digits = vec![0; dims.len()];
        for (i, d) in dims.iter().enumerate().rev() {
            digits[i] = rest % d.choices.len();
            rest /= d.choices.len();
        }
        Some(digits)
    }

    /// The configuration at `index` (`None` past the end).
    pub fn get(&self, index: usize) -> Option<Config> {
        let g = self.digits(index)?;
        let mut c = Config {
            dof: Dof::ALL[g[0]],
            rule: UpdateRule::ALL[g[1]],
            illumination: Illumination::ALL[g[2]],
            ..Config::default()
        };
        c.lm.damping = [Damping::Levenberg, Damping::Marquardt][g[3]];
        c.lm.acceptance = [
            Acceptance::Decrease,
            Acceptance::DecreaseOrEqual,
            Acceptance::GainRatio(0.25),
        ][g[4]];
        c.selection.boundary = [
            BoundaryWeight::Off,
            BoundaryWeight::Linear { ramp: 4.0 },
            BoundaryWeight::Smoothstep { ramp: 4.0 },
            BoundaryWeight::Quadratic { ramp: 4.0 },
        ][g[5]];
        c.velocity = [Velocity::Off, Velocity::ConstantImageSpace][g[6]];
        c.ncc.enabled = g[7] == 1;
        Some(c)
    }

    /// Every configuration, in index order.
    pub fn iter(&self) -> impl Iterator<Item = Config> {
        let space = *self;
        (0..space.size()).filter_map(move |i| space.get(i))
    }
}
