//! Gaussian image pyramid.
//!
//! Burt and Adelson, "The Laplacian Pyramid as a Compact Image Code", IEEE Transactions on
//! Communications 31(4), 1983 (a small separable, symmetric, normalised generating
//! kernel applied before each reduction; a pyramid of low pass images of falling resolution).
//!
//! Kernel rule (stated): with `taps = 2m + 1` the kernel is the binomial row of order `taps - 1`,
//! `w[k] = C(taps - 1, k) / 2^(taps - 1)`. Binomial rows approximate a Gaussian of variance
//! `(taps - 1) / 4` (de Moivre and Laplace), and every coefficient is a dyadic rational that
//! `f64` holds exactly. Three taps give `[1, 2, 1] / 4`, five give `[1, 4, 6, 4, 1] / 16`. No
//! transcendental function is needed.
//!
//! Reduction rule (stated): level `l + 1` has size `max(1, floor(size_l / scale + 1/2))` in each
//! axis; with the real ratio `s = size_l / size_(l+1)` its pixel `i` samples the blurred level `l`
//! bilinearly at `s * (i + 1/2) - 1/2` (pixel areas map onto pixel areas under the centre
//! convention of [`crate::image`]). For `scale = 2` and even sizes this is the mean of two
//! neighbouring blurred samples. The affine map from level coordinates to level 0 coordinates is
//! kept in [`Level`].

use crate::error::Error;
use crate::image::Image;

/// Pyramid parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct PyramidConfig {
    /// Maximum number of levels including level 0 (at least 1).
    pub levels: usize,
    /// Size ratio between neighbouring levels, above 1.
    pub scale: f64,
    /// Number of kernel taps, odd, from 1 to 15 (1 means no blur).
    pub kernel_taps: usize,
    /// A level whose width or height would fall below this is not built.
    pub min_size: usize,
}

impl Default for PyramidConfig {
    fn default() -> Self {
        PyramidConfig {
            levels: 4,
            scale: 2.0,
            kernel_taps: 5,
            min_size: 12,
        }
    }
}

impl PyramidConfig {
    /// Check the ranges documented on the fields.
    pub fn validate(&self) -> Result<(), Error> {
        if self.levels == 0 {
            return Err(Error::Config("pyramid.levels must be at least 1".into()));
        }
        if !(self.scale > 1.0 && self.scale <= 8.0) {
            return Err(Error::Config("pyramid.scale must be in (1, 8]".into()));
        }
        if self.kernel_taps & 1 == 0 || self.kernel_taps > 15 {
            return Err(Error::Config(
                "pyramid.kernel_taps must be odd and at most 15".into(),
            ));
        }
        if self.min_size == 0 {
            return Err(Error::Config("pyramid.min_size must be at least 1".into()));
        }
        Ok(())
    }
}

/// The binomial kernel of `taps` taps (see the module text).
pub fn binomial_kernel(taps: usize) -> Vec<f64> {
    let n = taps.max(1) - 1;
    let mut row = vec![1.0f64];
    for _ in 0..n {
        let mut next = vec![1.0f64; row.len() + 1];
        for i in 1..row.len() {
            next[i] = row[i - 1] + row[i];
        }
        row = next;
    }
    let mut total = 1.0;
    for _ in 0..n {
        total *= 2.0;
    }
    row.iter().map(|c| c / total).collect()
}

/// Separable blur with edge replication.
pub fn blur(img: &Image, kernel: &[f64]) -> Image {
    let (w, h) = (img.width(), img.height());
    let r = (kernel.len() / 2) as i64;
    let mut tmp = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for (k, c) in kernel.iter().enumerate() {
                s += c * img.at_clamped(x as i64 + k as i64 - r, y as i64);
            }
            tmp[y * w + x] = s;
        }
    }
    let mid = Image::new(w, h, tmp).expect("same size, finite");
    let mut out = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for (k, c) in kernel.iter().enumerate() {
                s += c * mid.at_clamped(x as i64, y as i64 + k as i64 - r);
            }
            out[y * w + x] = s;
        }
    }
    Image::new(w, h, out).expect("same size, finite")
}

/// One pyramid level with its map to level 0 coordinates:
/// `x0 = sx * x + ox`, `y0 = sy * y + oy`.
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    /// The level image.
    pub image: Image,
    /// Horizontal scale to level 0.
    pub sx: f64,
    /// Vertical scale to level 0.
    pub sy: f64,
    /// Horizontal offset to level 0.
    pub ox: f64,
    /// Vertical offset to level 0.
    pub oy: f64,
}

impl Level {
    /// The 3 by 3 matrix taking level coordinates to level 0 coordinates.
    pub fn to_base(&self) -> [f64; 9] {
        [self.sx, 0.0, self.ox, 0.0, self.sy, self.oy, 0.0, 0.0, 1.0]
    }

    /// The 3 by 3 matrix taking level 0 coordinates to level coordinates.
    pub fn from_base(&self) -> [f64; 9] {
        [
            1.0 / self.sx,
            0.0,
            -self.ox / self.sx,
            0.0,
            1.0 / self.sy,
            -self.oy / self.sy,
            0.0,
            0.0,
            1.0,
        ]
    }
}

/// A Gaussian pyramid, level 0 first.
#[derive(Debug, Clone, PartialEq)]
pub struct Pyramid {
    levels: Vec<Level>,
}

impl Pyramid {
    /// Build the pyramid of an image.
    pub fn build(img: &Image, cfg: &PyramidConfig) -> Result<Pyramid, Error> {
        cfg.validate()?;
        let kernel = binomial_kernel(cfg.kernel_taps);
        let mut levels = vec![Level {
            image: img.clone(),
            sx: 1.0,
            sy: 1.0,
            ox: 0.0,
            oy: 0.0,
        }];
        while levels.len() < cfg.levels {
            let last = levels.last().expect("at least one level");
            let (w, h) = (last.image.width(), last.image.height());
            let nw = ((w as f64 / cfg.scale + 0.5).floor() as usize).max(1);
            let nh = ((h as f64 / cfg.scale + 0.5).floor() as usize).max(1);
            if nw < cfg.min_size || nh < cfg.min_size || (nw == w && nh == h) {
                break;
            }
            let blurred = blur(&last.image, &kernel);
            let rx = w as f64 / nw as f64;
            let ry = h as f64 / nh as f64;
            let img = Image::from_fn(nw, nh, |x, y| {
                blurred.sample_clamped(rx * (x as f64 + 0.5) - 0.5, ry * (y as f64 + 0.5) - 0.5)
            });
            let next = Level {
                image: img,
                sx: last.sx * rx,
                sy: last.sy * ry,
                ox: last.sx * (rx - 1.0) * 0.5 + last.ox,
                oy: last.sy * (ry - 1.0) * 0.5 + last.oy,
            };
            levels.push(next);
        }
        Ok(Pyramid { levels })
    }

    /// Number of levels built.
    pub fn len(&self) -> usize {
        self.levels.len()
    }

    /// Always false: a pyramid has level 0.
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    /// Level `i`, 0 the full resolution image.
    pub fn level(&self, i: usize) -> &Level {
        &self.levels[i]
    }
}
