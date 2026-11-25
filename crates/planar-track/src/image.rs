//! Grey level images, the pixel centre convention, bilinear sampling and gradients.
//!
//! Convention (stated, and the same everywhere in the crate): the sample `data[y * width + x]`
//! lies at the point `(x, y)` with integer coordinates, so the centre of the first pixel is
//! `(0, 0)` and the covered domain is `0 <= x <= width - 1`, `0 <= y <= height - 1`. Grey values
//! are `f64`; the nominal range is `0.0` to `1.0` but nothing here depends on it.
//!
//! Bilinear sampling (the textbook weighted mean of the four surrounding samples, for example
//! Gonzalez and Woods, Digital Image Processing, section on interpolation): with
//! `x0 = floor(x)`, `fx = x - x0`, the value is
//! `(1-fy) * ((1-fx) * a + fx * b) + fy * ((1-fx) * c + fx * d)`. The border rule of
//! [`Image::sample_clamped`] replicates the edge samples (coordinates are clamped into the
//! domain); [`Image::sample_inside`] instead refuses any point outside the domain.

use crate::error::Error;

/// A grey level image.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    width: usize,
    height: usize,
    data: Vec<f64>,
}

impl Image {
    /// Build an image from row major samples; refuses a zero size, a length mismatch or a non
    /// finite sample.
    pub fn new(width: usize, height: usize, data: Vec<f64>) -> Result<Image, Error> {
        if width == 0 || height == 0 || data.len() != width * height {
            return Err(Error::Image(format!(
                "size {width}x{height} does not match {} samples",
                data.len()
            )));
        }
        if data.iter().any(|v| !v.is_finite()) {
            return Err(Error::Image("non finite sample".into()));
        }
        Ok(Image {
            width,
            height,
            data,
        })
    }

    /// An image filled by a function of the integer pixel position.
    pub fn from_fn(width: usize, height: usize, f: impl Fn(usize, usize) -> f64) -> Image {
        let mut data = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                data.push(f(x, y));
            }
        }
        Image {
            width: width.max(1),
            height: height.max(1),
            data,
        }
    }

    /// Width in pixels.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> usize {
        self.height
    }

    /// The row major samples.
    pub fn data(&self) -> &[f64] {
        &self.data
    }

    /// The sample at an integer position (panics outside the image).
    pub fn at(&self, x: usize, y: usize) -> f64 {
        self.data[y * self.width + x]
    }

    /// The sample at a signed integer position, edge samples replicated outside.
    pub fn at_clamped(&self, x: i64, y: i64) -> f64 {
        let xi = x.clamp(0, self.width as i64 - 1) as usize;
        let yi = y.clamp(0, self.height as i64 - 1) as usize;
        self.data[yi * self.width + xi]
    }

    /// Bilinear sample with the clamp (edge replication) border rule.
    pub fn sample_clamped(&self, x: f64, y: f64) -> f64 {
        let xm = self.width as f64 - 1.0;
        let ym = self.height as f64 - 1.0;
        let x = if x < 0.0 {
            0.0
        } else if x > xm {
            xm
        } else {
            x
        };
        let y = if y < 0.0 {
            0.0
        } else if y > ym {
            ym
        } else {
            y
        };
        self.bilinear(x, y)
    }

    /// Bilinear sample; `None` when the point is outside `[0, width-1] x [0, height-1]` or not a
    /// number.
    pub fn sample_inside(&self, x: f64, y: f64) -> Option<f64> {
        if x >= 0.0 && x <= self.width as f64 - 1.0 && y >= 0.0 && y <= self.height as f64 - 1.0 {
            Some(self.bilinear(x, y))
        } else {
            None
        }
    }

    /// Bilinear interpolation for a point already inside the domain.
    fn bilinear(&self, x: f64, y: f64) -> f64 {
        let x0f = x.floor();
        let y0f = y.floor();
        let fx = x - x0f;
        let fy = y - y0f;
        let x0 = x0f as usize;
        let y0 = y0f as usize;
        let x1 = if x0 + 1 < self.width { x0 + 1 } else { x0 };
        let y1 = if y0 + 1 < self.height { y0 + 1 } else { y0 };
        let a = self.data[y0 * self.width + x0];
        let b = self.data[y0 * self.width + x1];
        let c = self.data[y1 * self.width + x0];
        let d = self.data[y1 * self.width + x1];
        let top = (1.0 - fx) * a + fx * b;
        let bot = (1.0 - fx) * c + fx * d;
        (1.0 - fy) * top + fy * bot
    }

    /// Central difference gradient images `(gx, gy)`: `gx(x, y) = (I(x+1, y) - I(x-1, y)) / 2`
    /// with edge replication, and likewise for `gy`. The same convention is used for the
    /// template and for the frames (Baker and Matthews 2004, section 2.2, only require a gradient
    /// image; the central difference is this crate's choice).
    pub fn gradients(&self) -> (Image, Image) {
        let mut gx = Vec::with_capacity(self.data.len());
        let mut gy = Vec::with_capacity(self.data.len());
        for y in 0..self.height as i64 {
            for x in 0..self.width as i64 {
                gx.push((self.at_clamped(x + 1, y) - self.at_clamped(x - 1, y)) * 0.5);
                gy.push((self.at_clamped(x, y + 1) - self.at_clamped(x, y - 1)) * 0.5);
            }
        }
        (
            Image {
                width: self.width,
                height: self.height,
                data: gx,
            },
            Image {
                width: self.width,
                height: self.height,
                data: gy,
            },
        )
    }
}
