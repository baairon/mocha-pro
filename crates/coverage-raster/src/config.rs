//! The configuration: every convention that a caller may need to vary is a field or an enum case.

use crate::error::Error;

/// Several closed rings. A ring is a list of points; the last point joins the first. All rings of one
/// polygon are filled together under one fill rule, so a hole is a ring inside another.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Polygon(pub Vec<Vec<(f64, f64)>>);

impl Polygon {
    /// A polygon of one ring.
    pub fn from_ring(ring: Vec<(f64, f64)>) -> Polygon {
        Polygon(vec![ring])
    }
}

/// The target rectangle: pixel columns `x .. x + width` and rows `y .. y + height` of an unbounded
/// pixel grid. The buffer holds those pixels, row by row, `width * height` samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub x: i64,
    pub y: i64,
    pub width: u32,
    pub height: u32,
}

impl Target {
    /// A target with its origin at 0, 0.
    pub fn new(width: u32, height: u32) -> Target {
        Target {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}

/// How the coverage of one pixel is found.
#[derive(Debug, Clone, PartialEq)]
pub enum Method {
    /// The area of the pixel square inside the polygon under the fill rule, found by cutting the
    /// pixel row into slabs between the places where edges meet or cross (see the crate docs).
    ExactArea,
    /// One sample at the pixel centre: 0 or 1. The same as `Samples(Pattern::Grid(1))`.
    Point,
    /// The fraction of a set of sample points inside the polygon.
    Samples(Pattern),
}

/// A set of sample points inside a pixel. Offsets are measured from the minimum corner of the pixel
/// square, in pixel units, so the pixel centre is (0.5, 0.5).
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// `n` by `n` samples at `((a + 0.5) / n, (b + 0.5) / n)`; `n` from 1 to 250.
    Grid(u32),
    /// `n` samples, one per row and one per column of an `n` by `n` grid, at `((k + 0.5) / n,
    /// ((k * s) mod n + 0.5) / n)` where `s` is the smallest integer not below `0.382 * n` that has no
    /// common factor with `n` (0 when `n` is 1); `n` from 1 to 4096.
    Rotated(u32),
    /// Explicit offsets, each in 0 to 1 (finite); at least one, at most 65536.
    Offsets(Vec<(f64, f64)>),
}

/// Which points count as inside, given the winding number of the rings around them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    /// Inside when the winding number is not 0.
    NonZero,
    /// Inside when the winding number is odd.
    EvenOdd,
}

/// How the coverages of several polygons become one plane. The plane starts at 0 and each polygon,
/// in order, is folded in: with `a` the plane and `b` the polygon's coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combine {
    /// `max(a, b)`.
    Max,
    /// `min(a + b, 1)`.
    AddClamp,
    /// `a + b * (1 - a)`, evaluated in this order.
    Over,
    /// `b` where `b > 0`, else `a`.
    Replace,
}

/// The arithmetic width of the coverage values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    /// Every coverage value is a double.
    Double,
    /// Every coverage value, and every result of a combine step, is rounded to single precision.
    Single,
}

/// The sample type of the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    Bits8,
    Bits16,
    Float32,
}

/// The factor from coverage 1.0 to a code (ignored for `Float32`). The code is then clamped to the
/// largest value of the depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    /// The largest value of the depth: 255 or 65535.
    Full,
    /// the largest value plus one, so that coverage 1.0 clamps to the largest value.
    PowerOfTwo,
    /// An explicit factor, for example 32768 for a 16 bit depth that stores 0 to 32768.
    Custom(f64),
}

/// How a scaled coverage becomes a whole code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    /// To nearest, halves up.
    NearestHalfUp,
    /// To nearest, halves to even.
    NearestEven,
    /// Toward zero (all values are not negative).
    Floor,
    /// Away from zero.
    Ceil,
}

/// The conversion from coverage to a sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantise {
    pub depth: Depth,
    pub scale: Scale,
    pub rounding: Rounding,
}

/// One full choice of every open convention.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub method: Method,
    pub fill: FillRule,
    /// Where pixel `i` has its centre: at coordinate `i + centre`. The pixel square is the unit
    /// square around that centre. 0.5 puts the square corners on integers; 0.0 puts the centres on
    /// integers. Any finite value from -1 to 1 is accepted.
    pub centre: f64,
    /// When true, row 0 of the buffer is at the smallest `y` and rows grow with `y`. When false, row 0
    /// is at the largest `y` of the target and rows grow against `y`.
    pub y_down: bool,
    /// The sample type of [`crate::rasterise_buffer`]; `None` gives `Float32` rounded from the double.
    pub quantise: Option<Quantise>,
    pub combine: Combine,
    pub precision: Precision,
    /// When set, every coordinate is first rounded to a multiple of `2^-bits` (halves away from zero),
    /// `bits` from 0 to 30.
    pub snap_bits: Option<u32>,
}

impl Config {
    /// The documented default. It is a stated starting point and claims no match with any program:
    /// exact area, non-zero winding, centre 0.5, rows growing with `y`, 8 bit output scaled by 255
    /// and rounded to nearest with halves up, `Max` combine, double precision, no snapping.
    pub fn canonical() -> Config {
        Config {
            method: Method::ExactArea,
            fill: FillRule::NonZero,
            centre: 0.5,
            y_down: true,
            quantise: Some(Quantise {
                depth: Depth::Bits8,
                scale: Scale::Full,
                rounding: Rounding::NearestHalfUp,
            }),
            combine: Combine::Max,
            precision: Precision::Double,
            snap_bits: None,
        }
    }

    /// A stable one line name of the configuration, for a search log. Equal configurations give equal
    /// labels.
    pub fn label(&self) -> String {
        let method = match &self.method {
            Method::ExactArea => "exact".to_string(),
            Method::Point => "point".to_string(),
            Method::Samples(Pattern::Grid(n)) => format!("grid{n}"),
            Method::Samples(Pattern::Rotated(n)) => format!("rot{n}"),
            Method::Samples(Pattern::Offsets(o)) => format!("offsets{}", o.len()),
        };
        let fill = match self.fill {
            FillRule::NonZero => "nz",
            FillRule::EvenOdd => "eo",
        };
        let q = match &self.quantise {
            None => "f32".to_string(),
            Some(q) => {
                let d = match q.depth {
                    Depth::Bits8 => "u8",
                    Depth::Bits16 => "u16",
                    Depth::Float32 => "f32",
                };
                let s = match q.scale {
                    Scale::Full => "full".to_string(),
                    Scale::PowerOfTwo => "pow2".to_string(),
                    Scale::Custom(v) => format!("x{v}"),
                };
                let r = match q.rounding {
                    Rounding::NearestHalfUp => "up",
                    Rounding::NearestEven => "even",
                    Rounding::Floor => "floor",
                    Rounding::Ceil => "ceil",
                };
                format!("{d}-{s}-{r}")
            }
        };
        let c = match self.combine {
            Combine::Max => "max",
            Combine::AddClamp => "add",
            Combine::Over => "over",
            Combine::Replace => "replace",
        };
        let p = match self.precision {
            Precision::Double => "f64",
            Precision::Single => "f32acc",
        };
        let snap = match self.snap_bits {
            None => "nosnap".to_string(),
            Some(b) => format!("snap{b}"),
        };
        format!(
            "{method}/{fill}/c{}/{}/{q}/{c}/{p}/{snap}",
            self.centre,
            if self.y_down { "ydown" } else { "yup" }
        )
    }

    /// Check every range the crate documents.
    pub fn validate(&self) -> Result<(), Error> {
        if !self.centre.is_finite() || self.centre.abs() > 1.0 {
            return Err(Error::BadConfig("centre must be finite and within -1 to 1"));
        }
        if let Some(b) = self.snap_bits {
            if b > 30 {
                return Err(Error::BadConfig("snap_bits above 30"));
            }
        }
        if let Some(q) = &self.quantise {
            if let Scale::Custom(v) = q.scale {
                if !v.is_finite() || v <= 0.0 {
                    return Err(Error::BadConfig("custom scale must be finite and positive"));
                }
            }
        }
        match &self.method {
            Method::Samples(Pattern::Grid(n)) if *n == 0 || *n > 250 => {
                Err(Error::BadConfig("grid size must be 1 to 250"))
            }
            Method::Samples(Pattern::Rotated(n)) if *n == 0 || *n > 4096 => {
                Err(Error::BadConfig("rotated count must be 1 to 4096"))
            }
            Method::Samples(Pattern::Offsets(o)) => {
                if o.is_empty() || o.len() > 65536 {
                    return Err(Error::BadConfig("offset count must be 1 to 65536"));
                }
                if o.iter().any(|&(x, y)| {
                    !(x.is_finite()
                        && y.is_finite()
                        && (0.0..=1.0).contains(&x)
                        && (0.0..=1.0).contains(&y))
                }) {
                    return Err(Error::BadConfig("offsets must be finite and within 0 to 1"));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// The lists a search runs over. [`Space::enumerate`] gives every combination in a fixed order.
#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    pub methods: Vec<Method>,
    pub fills: Vec<FillRule>,
    pub centres: Vec<f64>,
    pub y_downs: Vec<bool>,
    pub quantises: Vec<Option<Quantise>>,
    pub combines: Vec<Combine>,
    pub precisions: Vec<Precision>,
    pub snaps: Vec<Option<u32>>,
}

impl Default for Space {
    /// A stated starting list: exact area, point, grids of 2, 4 and 8, rotated 4 and 8; both fill
    /// rules; centres 0.0 and 0.5; both y directions; 8 bit and 16 bit with every scale and
    /// rounding listed here; all four combines; double precision; no snapping and snapping to
    /// 2^-8.
    fn default() -> Space {
        let mut quantises = Vec::new();
        for depth in [Depth::Bits8, Depth::Bits16] {
            for scale in [Scale::Full, Scale::PowerOfTwo] {
                for rounding in [
                    Rounding::NearestHalfUp,
                    Rounding::NearestEven,
                    Rounding::Floor,
                    Rounding::Ceil,
                ] {
                    quantises.push(Some(Quantise {
                        depth,
                        scale,
                        rounding,
                    }));
                }
            }
        }
        quantises.push(None);
        Space {
            methods: vec![
                Method::ExactArea,
                Method::Point,
                Method::Samples(Pattern::Grid(2)),
                Method::Samples(Pattern::Grid(4)),
                Method::Samples(Pattern::Grid(8)),
                Method::Samples(Pattern::Rotated(4)),
                Method::Samples(Pattern::Rotated(8)),
            ],
            fills: vec![FillRule::NonZero, FillRule::EvenOdd],
            centres: vec![0.5, 0.0],
            y_downs: vec![true, false],
            quantises,
            combines: vec![
                Combine::Max,
                Combine::AddClamp,
                Combine::Over,
                Combine::Replace,
            ],
            precisions: vec![Precision::Double],
            snaps: vec![None, Some(8)],
        }
    }
}

impl Space {
    /// How many configurations [`Space::enumerate`] returns.
    pub fn len(&self) -> usize {
        self.methods.len()
            * self.fills.len()
            * self.centres.len()
            * self.y_downs.len()
            * self.quantises.len()
            * self.combines.len()
            * self.precisions.len()
            * self.snaps.len()
    }

    /// True when some list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every combination, the first list varying slowest and the last fastest, in the field order of
    /// the struct. The order is fixed, so an index names a configuration.
    pub fn enumerate(&self) -> Vec<Config> {
        let mut out = Vec::with_capacity(self.len());
        for method in &self.methods {
            for &fill in &self.fills {
                for &centre in &self.centres {
                    for &y_down in &self.y_downs {
                        for quantise in &self.quantises {
                            for &combine in &self.combines {
                                for &precision in &self.precisions {
                                    for &snap_bits in &self.snaps {
                                        out.push(Config {
                                            method: method.clone(),
                                            fill,
                                            centre,
                                            y_down,
                                            quantise: *quantise,
                                            combine,
                                            precision,
                                            snap_bits,
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
