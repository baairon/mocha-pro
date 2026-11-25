//! Configuration of the resampler. Every field is a choice that a later calibration may vary.
//! The defaults are arbitrary starting points and claim nothing about any other software.

use crate::border::Border;

/// Reconstruction filter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Filter {
    /// One tap; ties are resolved by [`Config::tie`].
    Nearest,
    /// Triangle kernel, two taps per axis.
    Bilinear,
    /// Keys 1981 cubic convolution with parameter `a`, four taps per axis.
    Keys { a: f64 },
    /// Mitchell and Netravali 1988 family with parameters `b` and `c`, four taps per axis.
    Mitchell { b: f64, c: f64 },
    /// Lanczos (Duchon 1979, Turkowski 1990) with `lobes` lobes per side (at least 1).
    Lanczos { lobes: u32 },
    /// Exact area average over an axis aligned rectangle, sized by [`Config::footprint`].
    Box,
}

/// Tie rule for nearest sampling when the source coordinate is exactly halfway between two
/// pixel centres.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TieRule {
    /// Round to the higher index.
    HalfUp,
    /// Round to the lower index.
    HalfDown,
    /// Round to the even index.
    HalfEven,
}

/// Pixel centre convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PixelCenter {
    /// Pixel `i` covers `[i, i + 1)` and its centre is at `i + 0.5`; the mapping is called
    /// with output centres and returns source centres in the same convention.
    Half,
    /// Pixel `i` is at coordinate `i` exactly.
    Corner,
}

impl PixelCenter {
    /// Offset added to an integer pixel index to form the coordinate given to the mapping.
    pub fn offset(self) -> f64 {
        match self {
            PixelCenter::Half => 0.5,
            PixelCenter::Corner => 0.0,
        }
    }
}

/// How colour is interpolated with respect to alpha. Needs [`Config::alpha_channel`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlphaMode {
    /// Every channel is interpolated independently.
    Straight,
    /// Colour is multiplied by alpha before interpolation and divided by the interpolated
    /// alpha afterwards (zero where that alpha is zero).
    Premultiplied,
}

/// Weight normalisation rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Normalize {
    /// Weights are used as the kernel gives them.
    None,
    /// The result is divided by the sum of all tap weights (zero when that sum is zero).
    Sum,
    /// The result is divided by the sum of the weights of taps whose index lies inside the
    /// image, before any border wrapping (zero when that sum is zero).
    SumInBounds,
}

/// Order of the two dimensional accumulation loop (matters only for rounding).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccumOrder {
    /// Outer loop over rows, inner loop over columns.
    RowMajor,
    /// Outer loop over columns, inner loop over rows.
    ColumnMajor,
}

/// Footprint rule of the box filter, in source pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Footprint {
    /// One source pixel by one source pixel.
    Unit,
    /// Bounding box of the mapped output pixel, from forward differences of the mapping one
    /// output pixel to the right and below, never smaller than [`Config::min_extent`].
    Jacobian,
    /// A fixed width and height in source pixels (both positive and finite).
    Fixed { w: f64, h: f64 },
}

/// Full configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub filter: Filter,
    pub tie: TieRule,
    pub center: PixelCenter,
    pub border: Border,
    pub alpha: AlphaMode,
    /// Index of the alpha channel, if the data has one.
    pub alpha_channel: Option<usize>,
    pub normalize: Normalize,
    pub order: AccumOrder,
    pub footprint: Footprint,
    /// Lower bound of the [`Footprint::Jacobian`] extent (positive and finite).
    pub min_extent: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            filter: Filter::Bilinear,
            tie: TieRule::HalfUp,
            center: PixelCenter::Half,
            border: Border::Clamp,
            alpha: AlphaMode::Straight,
            alpha_channel: None,
            normalize: Normalize::None,
            order: AccumOrder::RowMajor,
            footprint: Footprint::Unit,
            min_extent: 1.0,
        }
    }
}

/// Errors reported by the resampler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// A width, height or channel count is zero, or their product overflows.
    BadDimensions,
    /// The buffer length differs from `width * height * channels`.
    BadLength,
    /// `Lanczos` with zero lobes.
    BadLobes,
    /// Alpha channel index out of range, or premultiplied mode without an alpha channel.
    BadAlpha,
    /// A box footprint size or `min_extent` that is not positive and finite.
    BadFootprint,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Error::BadDimensions => "bad dimensions",
            Error::BadLength => "buffer length does not match dimensions",
            Error::BadLobes => "lanczos needs at least one lobe",
            Error::BadAlpha => "bad alpha channel configuration",
            Error::BadFootprint => "bad box footprint",
        };
        f.write_str(s)
    }
}

impl std::error::Error for Error {}

impl Config {
    /// Check the configuration against a channel count.
    pub fn validate(&self, channels: usize) -> Result<(), Error> {
        if let Filter::Lanczos { lobes: 0 } = self.filter {
            return Err(Error::BadLobes);
        }
        if let Some(a) = self.alpha_channel {
            if a >= channels {
                return Err(Error::BadAlpha);
            }
        } else if self.alpha == AlphaMode::Premultiplied {
            return Err(Error::BadAlpha);
        }
        let ok = |v: f64| v.is_finite() && v > 0.0;
        if !ok(self.min_extent) {
            return Err(Error::BadFootprint);
        }
        if let Footprint::Fixed { w, h } = self.footprint {
            if !ok(w) || !ok(h) {
                return Err(Error::BadFootprint);
            }
        }
        Ok(())
    }
}

/// A product space of configuration axes. [`ConfigSpace::all`] lists every combination in a
/// fixed order (the last axis, `footprint`, varies fastest) with the fields not listed here
/// taken from `base`.
#[derive(Clone, Debug)]
pub struct ConfigSpace {
    pub base: Config,
    pub filters: Vec<Filter>,
    pub ties: Vec<TieRule>,
    pub centers: Vec<PixelCenter>,
    pub borders: Vec<Border>,
    pub alphas: Vec<AlphaMode>,
    pub normalizes: Vec<Normalize>,
    pub orders: Vec<AccumOrder>,
    pub footprints: Vec<Footprint>,
}

impl ConfigSpace {
    /// A representative space: every filter family at a few parameter values and every
    /// value of every other axis. Alpha modes are limited to `Straight` because premultiplied
    /// mode needs an alpha channel.
    pub fn standard() -> Self {
        ConfigSpace {
            base: Config::default(),
            filters: vec![
                Filter::Nearest,
                Filter::Bilinear,
                Filter::Keys { a: -0.5 },
                Filter::Keys { a: -0.75 },
                Filter::Keys { a: -1.0 },
                Filter::Mitchell { b: 0.0, c: 0.5 },
                Filter::Mitchell {
                    b: 1.0 / 3.0,
                    c: 1.0 / 3.0,
                },
                Filter::Mitchell { b: 1.0, c: 0.0 },
                Filter::Lanczos { lobes: 2 },
                Filter::Lanczos { lobes: 3 },
                Filter::Box,
            ],
            ties: vec![TieRule::HalfUp, TieRule::HalfDown, TieRule::HalfEven],
            centers: vec![PixelCenter::Half, PixelCenter::Corner],
            borders: vec![
                Border::Zero,
                Border::Clamp,
                Border::Repeat,
                Border::Mirror,
                Border::MirrorNoEdge,
            ],
            alphas: vec![AlphaMode::Straight],
            normalizes: vec![Normalize::None, Normalize::Sum, Normalize::SumInBounds],
            orders: vec![AccumOrder::RowMajor, AccumOrder::ColumnMajor],
            footprints: vec![Footprint::Unit, Footprint::Jacobian],
        }
    }

    /// Number of combinations.
    pub fn len(&self) -> usize {
        self.filters.len()
            * self.ties.len()
            * self.centers.len()
            * self.borders.len()
            * self.alphas.len()
            * self.normalizes.len()
            * self.orders.len()
            * self.footprints.len()
    }

    /// True when some axis is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every combination, in a fixed order.
    pub fn all(&self) -> Vec<Config> {
        let mut out = Vec::with_capacity(self.len());
        for &filter in &self.filters {
            for &tie in &self.ties {
                for &center in &self.centers {
                    for &border in &self.borders {
                        for &alpha in &self.alphas {
                            for &normalize in &self.normalizes {
                                for &order in &self.orders {
                                    for &footprint in &self.footprints {
                                        out.push(Config {
                                            filter,
                                            tie,
                                            center,
                                            border,
                                            alpha,
                                            normalize,
                                            order,
                                            footprint,
                                            ..self.base
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
