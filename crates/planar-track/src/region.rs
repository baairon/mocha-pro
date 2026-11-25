//! The tracked region: a mask on the template grid and four corner points.

use crate::error::Error;

/// A region of the template image.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    width: usize,
    height: usize,
    mask: Vec<bool>,
    corners: [(f64, f64); 4],
}

impl Region {
    /// A mask given directly (`width * height`, row major); the corners are the corners of the
    /// bounding box of the set pixels, taken as pixel centres: `(x0, y0), (x1, y0), (x1, y1),
    /// (x0, y1)`. Refuses an empty mask.
    pub fn from_mask(width: usize, height: usize, mask: Vec<bool>) -> Result<Region, Error> {
        if width == 0 || height == 0 || mask.len() != width * height {
            return Err(Error::Image("mask size mismatch".into()));
        }
        let mut x0 = usize::MAX;
        let mut y0 = usize::MAX;
        let mut x1 = 0usize;
        let mut y1 = 0usize;
        let mut any = false;
        for y in 0..height {
            for x in 0..width {
                if mask[y * width + x] {
                    any = true;
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        if !any {
            return Err(Error::Image("empty mask".into()));
        }
        let (fx0, fy0, fx1, fy1) = (x0 as f64, y0 as f64, x1 as f64, y1 as f64);
        Ok(Region {
            width,
            height,
            mask,
            corners: [(fx0, fy0), (fx1, fy0), (fx1, fy1), (fx0, fy1)],
        })
    }

    /// The pixels whose centres lie in the closed rectangle `[x0, x1] x [y0, y1]`.
    pub fn from_rect(
        width: usize,
        height: usize,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    ) -> Result<Region, Error> {
        Region::from_quad(width, height, [(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    /// The pixels whose centres lie in the closed convex quadrilateral with the given corners
    /// (in order around the boundary, either orientation). The corners are kept as given.
    pub fn from_quad(
        width: usize,
        height: usize,
        corners: [(f64, f64); 4],
    ) -> Result<Region, Error> {
        if width == 0 || height == 0 {
            return Err(Error::Image("zero size".into()));
        }
        let cross = |a: (f64, f64), b: (f64, f64), p: (f64, f64)| {
            (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
        };
        let mut mask = vec![false; width * height];
        for y in 0..height {
            for x in 0..width {
                let p = (x as f64, y as f64);
                let mut pos = false;
                let mut neg = false;
                for i in 0..4 {
                    let c = cross(corners[i], corners[(i + 1) % 4], p);
                    if c > 0.0 {
                        pos = true;
                    } else if c < 0.0 {
                        neg = true;
                    }
                }
                mask[y * width + x] = !(pos && neg);
            }
        }
        if !mask.iter().any(|&m| m) {
            return Err(Error::Image("region contains no pixel".into()));
        }
        Ok(Region {
            width,
            height,
            mask,
            corners,
        })
    }

    /// Width of the template grid.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Height of the template grid.
    pub fn height(&self) -> usize {
        self.height
    }

    /// The row major mask.
    pub fn mask(&self) -> &[bool] {
        &self.mask
    }

    /// The four corner points, template coordinates.
    pub fn corners(&self) -> [(f64, f64); 4] {
        self.corners
    }

    /// Number of set pixels.
    pub fn count(&self) -> usize {
        self.mask.iter().filter(|&&m| m).count()
    }
}
