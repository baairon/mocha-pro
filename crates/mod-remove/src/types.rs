//! Windows, mattes, images, errors.

/// Integer window, absolute coordinates, row major buffers relative to the left top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Win {
    pub l: i32,
    pub t: i32,
    pub w: i32,
    pub h: i32,
}

impl Win {
    pub fn new(l: i32, t: i32, w: i32, h: i32) -> Win {
        Win { l, t, w, h }
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.l && y >= self.t && x < self.l + self.w && y < self.t + self.h
    }
    pub fn idx(&self, x: i32, y: i32) -> usize {
        ((y - self.t) * self.w + (x - self.l)) as usize
    }
    pub fn area(&self) -> usize {
        if self.w <= 0 || self.h <= 0 {
            0
        } else {
            (self.w * self.h) as usize
        }
    }
    pub fn grow(&self, n: i32) -> Win {
        Win::new(self.l - n, self.t - n, self.w + 2 * n, self.h + 2 * n)
    }
    /// Inclusive bounds to a window.
    pub fn from_bounds(x0: i32, y0: i32, x1: i32, y1: i32) -> Win {
        Win::new(x0, y0, x1 - x0 + 1, y1 - y0 + 1)
    }
    pub fn union(&self, o: &Win) -> Win {
        if self.area() == 0 {
            return *o;
        }
        if o.area() == 0 {
            return *self;
        }
        let x0 = self.l.min(o.l);
        let y0 = self.t.min(o.t);
        let x1 = (self.l + self.w).max(o.l + o.w) - 1;
        let y1 = (self.t + self.h).max(o.t + o.h) - 1;
        Win::from_bounds(x0, y0, x1, y1)
    }
}

/// 8 bit matte over a window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Matte {
    pub win: Win,
    pub data: Vec<u8>,
}

impl Matte {
    pub fn new(win: Win, v: u8) -> Matte {
        Matte {
            win,
            data: vec![v; win.area()],
        }
    }
    /// Zero outside the window.
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if self.win.contains(x, y) {
            self.data[self.win.idx(x, y)]
        } else {
            0
        }
    }
    /// Ignored outside the window.
    pub fn set(&mut self, x: i32, y: i32, v: u8) {
        if self.win.contains(x, y) {
            let i = self.win.idx(x, y);
            self.data[i] = v;
        }
    }
    pub fn nonzero_bounds(&self) -> Option<Win> {
        let mut b: Option<(i32, i32, i32, i32)> = None;
        for y in self.win.t..self.win.t + self.win.h {
            for x in self.win.l..self.win.l + self.win.w {
                if self.get(x, y) != 0 {
                    b = Some(match b {
                        None => (x, y, x, y),
                        Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x), e.max(y)),
                    });
                }
            }
        }
        b.map(|(a, c, d, e)| Win::from_bounds(a, c, d, e))
    }
}

/// Float32 RGBA image over a window.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub win: Win,
    pub data: Vec<[f32; 4]>,
}

impl Image {
    pub fn new(win: Win, v: [f32; 4]) -> Image {
        Image {
            win,
            data: vec![v; win.area()],
        }
    }
    pub fn at(&self, x: i32, y: i32) -> [f32; 4] {
        self.data[self.win.idx(x, y)]
    }
    pub fn set(&mut self, x: i32, y: i32, v: [f32; 4]) {
        let i = self.win.idx(x, y);
        self.data[i] = v;
    }
}

/// A precondition of the entry point that can fail. Each variant has its own
/// `Display` text; a compared output that needs another text takes it from the caller's
/// [`PreconditionTexts`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Precondition {
    TooFewTrackedFrames,
    CoverageMissing,
    LayerInactive,
    LowerLayerMissing,
    InStreamMissing,
    OutStreamUndeclared,
    OutStreamAbsent,
    OutStreamLocked,
    OutStreamAliasesIn,
    PlateClipMissing,
    RangeNotMet,
    EmptyRegion,
}

impl Precondition {
    /// Every variant, in declaration order.
    pub const ALL: [Precondition; 12] = [
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
}

impl std::fmt::Display for Precondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = match self {
            Precondition::TooFewTrackedFrames => "precondition: fewer than two tracked frames",
            Precondition::CoverageMissing => "precondition: tracking does not cover the range",
            Precondition::LayerInactive => "precondition: target layer inactive",
            Precondition::LowerLayerMissing => "precondition: lower layer not ready",
            Precondition::InStreamMissing => "precondition: input stream missing",
            Precondition::OutStreamUndeclared => "precondition: output stream undeclared",
            Precondition::OutStreamAbsent => "precondition: output stream absent",
            Precondition::OutStreamLocked => "precondition: output stream locked",
            Precondition::OutStreamAliasesIn => "precondition: output stream aliases the input",
            Precondition::PlateClipMissing => {
                "precondition: exclusive plate mode without a plate clip"
            }
            Precondition::RangeNotMet => "precondition: range settings not satisfied",
            Precondition::EmptyRegion => "precondition: empty region",
        };
        f.write_str(t)
    }
}

/// Caller supplied text by precondition, for outputs that must carry a specific wording.
/// The crate ships no texts of its own besides the `Display` text of [`Precondition`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PreconditionTexts {
    texts: std::collections::BTreeMap<Precondition, String>,
}

impl PreconditionTexts {
    pub fn new() -> PreconditionTexts {
        PreconditionTexts::default()
    }
    /// A made up sample table for tests, examples and docs: one text per variant, each
    /// with its own wording, distinct from the `Display` text.
    pub fn sample() -> PreconditionTexts {
        let mut t = PreconditionTexts::new();
        for (i, p) in Precondition::ALL.iter().enumerate() {
            t.set(*p, &format!("sample text number {}", i + 1));
        }
        t
    }
    pub fn set(&mut self, p: Precondition, text: &str) {
        self.texts.insert(p, text.to_string());
    }
    /// The caller's text for `p`, if the caller supplied one.
    pub fn get(&self, p: Precondition) -> Option<&str> {
        self.texts.get(&p).map(String::as_str)
    }
}

impl RemoveError {
    /// Text of a precondition failure: the caller's text when present, else the `Display` text.
    /// `None` for the other variants.
    pub fn precondition_text(&self, table: &PreconditionTexts) -> Option<String> {
        match self {
            RemoveError::Precondition(p) => Some(
                table
                    .get(*p)
                    .map(str::to_string)
                    .unwrap_or_else(|| p.to_string()),
            ),
            _ => None,
        }
    }
}

/// Error value. `Unspecified` names a point the specification does not give.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoveError {
    Unspecified(&'static str),
    Refused(&'static str),
    Invalid(&'static str),
    Precondition(Precondition),
}

impl std::fmt::Display for RemoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoveError::Unspecified(s) => write!(f, "unspecified: {s}"),
            RemoveError::Refused(s) => write!(f, "refused: {s}"),
            RemoveError::Invalid(s) => write!(f, "invalid: {s}"),
            RemoveError::Precondition(p) => p.fmt(f),
        }
    }
}

pub type Res<T> = Result<T, RemoveError>;

/// Camera record; only the linear model is specified here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub c0: [f32; 2],
    pub f: f32,
    pub zh: f32,
    pub kind: CamKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CamKind {
    Linear,
    Radial1,
    Radial2,
    FourCoefficient,
    DistortionMap,
    Equirectangular,
}

impl Camera {
    pub fn identity() -> Camera {
        Camera {
            c0: [0.0, 0.0],
            f: 1.0,
            zh: 1.0,
            kind: CamKind::Linear,
        }
    }
}

/// One candidate source: its pixels, cover matte, homography, camera, layer and mix weight.
#[derive(Clone, Debug)]
pub struct Source {
    pub pixels: Image,
    pub cover: Matte,
    /// Row major 3x3 homography from the target to this source.
    pub h: [f32; 9],
    pub cam: Camera,
    pub layer_id: u32,
    /// Mix weight of the source: it scales every contribution of this source to the fill.
    pub weight: f32,
}
