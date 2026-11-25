//! Generic planar contour evaluation: cubic Bezier and X-spline contours, evaluated at a
//! parameter of one span and flattened to polylines.
//!
//! The crate holds generic code only. Every choice a caller may need to vary is a field of
//! [`Config`]: the evaluation form, the number type, the tangent convention, the map from a
//! stored weight to the X-spline shape parameter, the X-spline variant, the treatment of the
//! ends of an open contour, whether a closed contour has its closing span, and whether a
//! closed polyline repeats its first point. [`Config::canonical`] gives a documented default
//! that claims nothing about any particular program; [`Config::enumerate`] lists the discrete
//! combinations.
//!
//! Published definitions (see the crate README for full citations): G. Farin, "Curves and
//! Surfaces for CAGD", 5th ed., 2002 (Bezier, de Casteljau); C. Blanc and C. Schlick,
//! "X-Splines: A Spline Model Designed for the End-User", SIGGRAPH 1995; J. Lane and
//! R. Riesenfeld, "A Theoretical Development for the Computer Generation and Display of
//! Piecewise Polynomial Surfaces", IEEE PAMI 2(1), 1980 (flatness test).
//!
//! Floating point: only add, subtract, multiply, divide and negate are used on the evaluation
//! path (the operations IEEE 754 fixes), plus square root in the length estimate; no fused
//! multiply add and no libm function. Results are the same on every platform.

mod bezier;
mod flatten;
mod real;
mod xspline;

use std::fmt;

use real::Real;

pub use flatten::{flatten, Flatten};

/// The X-spline variant (Blanc and Schlick 1995).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XVariant {
    /// Shape parameters in [0, 1] only: approximating points, no negative influence.
    Basic,
    /// Shape parameters in [-1, 1]: negative values interpolate.
    General,
}

/// The curve family of a contour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bezier,
    XSpline { variant: XVariant },
}

/// One control point. Fields a kind does not use are ignored for that kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    /// Bezier: the incoming tangent handle (see [`TangentMode`]).
    pub tan_in: (f64, f64),
    /// Bezier: the outgoing tangent handle.
    pub tan_out: (f64, f64),
    /// X-spline: the stored weight, mapped to the shape parameter by [`WeightMap`].
    pub s: f64,
}

impl Point {
    /// A point with zero handles and zero weight.
    pub fn new(x: f64, y: f64) -> Point {
        Point {
            x,
            y,
            tan_in: (0.0, 0.0),
            tan_out: (0.0, 0.0),
            s: 0.0,
        }
    }
}

/// A contour: a kind, a closed flag and control points in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Contour {
    pub kind: Kind,
    pub closed: bool,
    pub points: Vec<Point>,
}

/// Evaluation form of a cubic Bezier span (equal in exact arithmetic).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvalForm {
    Bernstein,
    DeCasteljau,
    Horner,
}

/// The number type used inside an evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    F32,
    F64,
}

/// Whether Bezier tangent handles are offsets from their anchor or absolute positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TangentMode {
    Relative,
    Absolute,
}

/// How the ends of an open X-spline contour are extended: the virtual neighbour before the
/// first point and after the last one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenEnd {
    /// The virtual neighbours are copies of the end points; the end points keep their own
    /// shape parameters.
    RepeatEnd,
    /// As `RepeatEnd`, and the first and last points of an open contour take shape
    /// parameter 0, so the open curve passes through them.
    RepeatEndSharp,
}

/// The affine map from a stored weight w to the shape parameter s = a w + b.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeightMap {
    pub a: f64,
    pub b: f64,
}

impl WeightMap {
    pub const IDENTITY: WeightMap = WeightMap { a: 1.0, b: 0.0 };
    fn apply(self, w: f64) -> f64 {
        self.a * w + self.b
    }
}

/// Which flatness test a [`Flatten::Subdivide`] uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlatTest {
    /// Distance of the Bezier inner control points from the chord (Bezier only; refused for
    /// X-splines, which have no control polygon per span).
    ControlPolygon,
    /// Distance of the curve points at the parameters 1/4, 1/2 and 3/4 of the interval from
    /// the chord of the interval ends (any kind; a sample test, not a bound).
    Sampled,
}

/// All open choices in one value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub form: EvalForm,
    pub precision: Precision,
    pub tangents: TangentMode,
    pub weight_map: WeightMap,
    pub open_end: OpenEnd,
    /// Whether a closed contour has the span from its last point back to its first. When false
    /// a closed contour evaluates like an open one.
    pub closing_span: bool,
    /// Whether a flattened closed contour ends with a copy of its first point.
    pub repeat_first: bool,
    pub flat_test: FlatTest,
}

impl Config {
    /// The documented default: Bernstein form, f64, relative tangents, identity weight map,
    /// repeated end points, a closing span, no repeated first point, sampled flatness test.
    /// It is a neutral starting point and claims nothing about any program.
    pub fn canonical() -> Config {
        Config {
            form: EvalForm::Bernstein,
            precision: Precision::F64,
            tangents: TangentMode::Relative,
            weight_map: WeightMap::IDENTITY,
            open_end: OpenEnd::RepeatEnd,
            closing_span: true,
            repeat_first: false,
            flat_test: FlatTest::Sampled,
        }
    }

    /// The weight maps that [`Config::enumerate`] lists: identity, [0, 1] onto [-1, 1],
    /// [0, 100] onto [0, 1], [0, 100] onto [-1, 1].
    pub const WEIGHT_MAPS: [WeightMap; 4] = [
        WeightMap { a: 1.0, b: 0.0 },
        WeightMap { a: 2.0, b: -1.0 },
        WeightMap { a: 0.01, b: 0.0 },
        WeightMap { a: 0.02, b: -1.0 },
    ];

    /// Every combination of the discrete choices, in a fixed order (the first is
    /// [`Config::canonical`]). The count is 3 * 2 * 2 * 4 * 2 * 2 * 2 * 2 = 768.
    pub fn enumerate() -> Vec<Config> {
        let mut out = Vec::new();
        for &flat_test in &[FlatTest::Sampled, FlatTest::ControlPolygon] {
            for &repeat_first in &[false, true] {
                for &closing_span in &[true, false] {
                    for &open_end in &[OpenEnd::RepeatEnd, OpenEnd::RepeatEndSharp] {
                        for &weight_map in &Config::WEIGHT_MAPS {
                            for &tangents in &[TangentMode::Relative, TangentMode::Absolute] {
                                for &precision in &[Precision::F64, Precision::F32] {
                                    for &form in &[
                                        EvalForm::Bernstein,
                                        EvalForm::DeCasteljau,
                                        EvalForm::Horner,
                                    ] {
                                        out.push(Config {
                                            form,
                                            precision,
                                            tangents,
                                            weight_map,
                                            open_end,
                                            closing_span,
                                            repeat_first,
                                            flat_test,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }
}

/// A typed refusal. The crate never panics on bad input.
#[derive(Clone, Debug, PartialEq)]
pub enum ContourError {
    /// Fewer points than a contour needs.
    TooFewPoints { have: usize, need: usize },
    /// A used field of a point is NaN or infinite.
    NonFinite { index: usize, field: &'static str },
    /// The span index is not below the span count.
    SpanOutOfRange { span: usize, spans: usize },
    /// The span parameter is NaN, infinite or outside [0, 1].
    ParamOutOfRange,
    /// A mapped shape parameter is outside the range of the variant.
    ShapeOutOfRange { index: usize, s: f64 },
    /// The X-spline weights of a point sum to zero or not to a finite number.
    ZeroWeightSum,
    /// An evaluation result is NaN or infinite (overflow in the chosen precision).
    NonFiniteResult,
    /// A flattening rule has an invalid constant.
    BadFlatten(&'static str),
    /// The control polygon flatness test needs a Bezier contour.
    UnsupportedFlatTest,
    /// The call needs a contour of another kind.
    UnsupportedKind,
}

impl fmt::Display for ContourError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContourError::TooFewPoints { have, need } => {
                write!(f, "contour has {have} points, needs at least {need}")
            }
            ContourError::NonFinite { index, field } => {
                write!(f, "point {index} has a non-finite {field}")
            }
            ContourError::SpanOutOfRange { span, spans } => {
                write!(f, "span {span} out of range, contour has {spans}")
            }
            ContourError::ParamOutOfRange => write!(f, "span parameter outside [0, 1]"),
            ContourError::ShapeOutOfRange { index, s } => {
                write!(
                    f,
                    "point {index} has shape parameter {s} outside the variant's range"
                )
            }
            ContourError::ZeroWeightSum => write!(f, "X-spline weights sum to zero or overflow"),
            ContourError::NonFiniteResult => write!(f, "evaluation result is not finite"),
            ContourError::BadFlatten(why) => write!(f, "invalid flattening rule: {why}"),
            ContourError::UnsupportedKind => write!(f, "the contour is of another kind"),
            ContourError::UnsupportedFlatTest => {
                write!(
                    f,
                    "the control polygon flatness test needs a Bezier contour"
                )
            }
        }
    }
}

impl std::error::Error for ContourError {}

/// A span ready to evaluate, in the chosen number type.
pub(crate) enum Prepared<R: Real> {
    Bezier([(R, R); 4]),
    XSpline { pts: [(R, R); 4], s1: R, s2: R },
}

fn check_finite(v: f64, index: usize, field: &'static str) -> Result<(), ContourError> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(ContourError::NonFinite { index, field })
    }
}

impl Contour {
    /// Validates the used fields of every point and the point count.
    pub fn validate(&self, cfg: &Config) -> Result<(), ContourError> {
        if self.points.len() < 2 {
            return Err(ContourError::TooFewPoints {
                have: self.points.len(),
                need: 2,
            });
        }
        for (i, p) in self.points.iter().enumerate() {
            check_finite(p.x, i, "x")?;
            check_finite(p.y, i, "y")?;
            match self.kind {
                Kind::Bezier => {
                    check_finite(p.tan_in.0, i, "tan_in.x")?;
                    check_finite(p.tan_in.1, i, "tan_in.y")?;
                    check_finite(p.tan_out.0, i, "tan_out.x")?;
                    check_finite(p.tan_out.1, i, "tan_out.y")?;
                }
                Kind::XSpline { variant } => {
                    check_finite(p.s, i, "s")?;
                    let s = cfg.weight_map.apply(p.s);
                    let lo = match variant {
                        XVariant::Basic => 0.0,
                        XVariant::General => -1.0,
                    };
                    if !(s >= lo && s <= 1.0) {
                        return Err(ContourError::ShapeOutOfRange { index: i, s });
                    }
                }
            }
        }
        Ok(())
    }

    /// The number of spans: n for a closed contour with a closing span, else n - 1.
    pub fn span_count(&self, cfg: &Config) -> Result<usize, ContourError> {
        self.validate(cfg)?;
        Ok(self.spans_unchecked(cfg))
    }

    fn spans_unchecked(&self, cfg: &Config) -> usize {
        if self.closed && cfg.closing_span {
            self.points.len()
        } else {
            self.points.len() - 1
        }
    }

    /// The four Bezier control points of a span (anchor, out handle, next in handle, next
    /// anchor) under the tangent convention of `cfg`, in f64.
    pub fn bezier_span(&self, span: usize, cfg: &Config) -> Result<[(f64, f64); 4], ContourError> {
        if self.kind != Kind::Bezier {
            return Err(ContourError::UnsupportedKind);
        }
        let spans = self.span_count(cfg)?;
        if span >= spans {
            return Err(ContourError::SpanOutOfRange { span, spans });
        }
        Ok(self.bezier_pts(span, cfg))
    }

    fn bezier_pts(&self, span: usize, cfg: &Config) -> [(f64, f64); 4] {
        let n = self.points.len();
        let a = &self.points[span];
        let b = &self.points[(span + 1) % n];
        let (p1, p2) = match cfg.tangents {
            TangentMode::Relative => (
                (a.x + a.tan_out.0, a.y + a.tan_out.1),
                (b.x + b.tan_in.0, b.y + b.tan_in.1),
            ),
            TangentMode::Absolute => (a.tan_out, b.tan_in),
        };
        [(a.x, a.y), p1, p2, (b.x, b.y)]
    }

    /// The four points around a span and the mapped shape parameters of its two end points.
    fn xspline_pts(&self, span: usize, cfg: &Config) -> ([(f64, f64); 4], f64, f64) {
        let n = self.points.len() as isize;
        let wrap = self.closed && cfg.closing_span;
        let sharpen = cfg.open_end == OpenEnd::RepeatEndSharp;
        let at = |j: isize| -> (f64, f64, f64) {
            let (idx, end) = if wrap {
                (j.rem_euclid(n) as usize, false)
            } else {
                let c = j.clamp(0, n - 1);
                (c as usize, c == 0 || c == n - 1)
            };
            let p = &self.points[idx];
            let s = if end && sharpen {
                0.0
            } else {
                cfg.weight_map.apply(p.s)
            };
            (p.x, p.y, s)
        };
        let i = span as isize;
        let (x0, y0, _) = at(i - 1);
        let (x1, y1, s1) = at(i);
        let (x2, y2, s2) = at(i + 1);
        let (x3, y3, _) = at(i + 2);
        ([(x0, y0), (x1, y1), (x2, y2), (x3, y3)], s1, s2)
    }

    pub(crate) fn prepare<R: Real>(&self, span: usize, cfg: &Config) -> Prepared<R> {
        let c = |p: (f64, f64)| (R::from_f64(p.0), R::from_f64(p.1));
        match self.kind {
            Kind::Bezier => {
                let q = self.bezier_pts(span, cfg);
                Prepared::Bezier([c(q[0]), c(q[1]), c(q[2]), c(q[3])])
            }
            Kind::XSpline { .. } => {
                let (q, s1, s2) = self.xspline_pts(span, cfg);
                Prepared::XSpline {
                    pts: [c(q[0]), c(q[1]), c(q[2]), c(q[3])],
                    s1: R::from_f64(s1),
                    s2: R::from_f64(s2),
                }
            }
        }
    }
}

pub(crate) fn eval_prepared<R: Real>(
    p: &Prepared<R>,
    form: EvalForm,
    t: f64,
) -> Result<(f64, f64), ContourError> {
    let tr = R::from_f64(t);
    let (x, y) = match p {
        Prepared::Bezier(q) => bezier::eval(form, q, tr),
        Prepared::XSpline { pts, s1, s2 } => xspline::point(pts, *s1, *s2, tr)?,
    };
    let (x, y) = (x.to_f64(), y.to_f64());
    if x.is_finite() && y.is_finite() {
        Ok((x, y))
    } else {
        Err(ContourError::NonFiniteResult)
    }
}

/// Evaluates span `span` of a contour at parameter `t` in [0, 1].
///
/// A Bezier span returns its first anchor at t = 0 and the next anchor at t = 1 exactly, in
/// every form. An X-spline span returns the first point at t = 0 when its shape parameter is
/// not positive, and the next point at t = 1 under the same condition.
pub fn eval(
    contour: &Contour,
    span: usize,
    t: f64,
    cfg: &Config,
) -> Result<(f64, f64), ContourError> {
    let spans = contour.span_count(cfg)?;
    if span >= spans {
        return Err(ContourError::SpanOutOfRange { span, spans });
    }
    if !(0.0..=1.0).contains(&t) {
        return Err(ContourError::ParamOutOfRange);
    }
    match cfg.precision {
        Precision::F64 => eval_prepared(&contour.prepare::<f64>(span, cfg), cfg.form, t),
        Precision::F32 => eval_prepared(&contour.prepare::<f32>(span, cfg), cfg.form, t),
    }
}

/// The four control points of a cubic Bezier span.
pub type Quad = [(f64, f64); 4];

/// Splits a cubic Bezier span at `t` by de Casteljau subdivision (Farin 2002, section 3.1)
/// into two spans covering [0, t] and [t, 1] of the original, in f64.
pub fn bezier_split(p: [(f64, f64); 4], t: f64) -> (Quad, Quad) {
    bezier::split(&p, t)
}

/// The three X-spline weight functions of the paper as plain f64 functions: F with support
/// ratio `num / den`, G and H with parameter `q = -s`.
pub fn f_blend(num: f64, den: f64) -> f64 {
    xspline::f_blend(num, den)
}
pub fn g_blend(u: f64, q: f64) -> f64 {
    xspline::g_blend(u, q)
}
pub fn h_blend(u: f64, q: f64) -> f64 {
    xspline::h_blend(u, q)
}

/// The unnormalised weights of the four points around an X-spline span at `t`, for the shape
/// parameters `s1` and `s2` of its two end points, in f64.
pub fn xspline_weights(s1: f64, s2: f64, t: f64) -> [f64; 4] {
    xspline::weights(s1, s2, t)
}
