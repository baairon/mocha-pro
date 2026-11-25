use crate::{Config, Convention, Homography, PixelCenter, Point, Rect, YAxis};

/// A pixel grid of `width` by `height` pixels with the choices of a [`Config`]
/// (`center`, `y_axis`, `pixel_aspect`; the convention selects how matrices are stored).
///
/// The normalised unit square `[0,1] x [0,1]` covers the whole image extent.
/// Horizontally `u = (px + c) / width` with `c = 0` for half integer centres (pixel `i` has
/// its centre at `i + 0.5`) and `c = 0.5` for integer centres (centre at `i`). Vertically,
/// with the same `c`, `t = (py + c) / height` and `v = t` for Y down or `v = 1 - t` for Y up.
/// The physical space multiplies the horizontal pixel coordinate by `pixel_aspect`
/// (width of a pixel over its height), so the physical image is `width * aspect` by `height`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelSpace {
    pub width: f64,
    pub height: f64,
    pub cfg: Config,
}

impl PixelSpace {
    pub fn new(width: f64, height: f64, cfg: Config) -> PixelSpace {
        PixelSpace { width, height, cfg }
    }

    fn offset(&self) -> f64 {
        match self.cfg.center {
            PixelCenter::Integer => 0.5,
            PixelCenter::Half => 0.0,
        }
    }

    /// Normalised unit square to pixel coordinates.
    pub fn unit_to_pixel(&self) -> Homography {
        let c = self.offset();
        let (sy, ty) = match self.cfg.y_axis {
            YAxis::Down => (self.height, -c),
            YAxis::Up => (-self.height, self.height - c),
        };
        let m = [[self.width, 0.0, -c], [0.0, sy, ty], [0.0, 0.0, 1.0]];
        Homography::from_matrix(m, Convention::Column).with_convention(self.cfg.convention)
    }

    /// Pixel coordinates to the normalised unit square (the exact inverse formulas,
    /// not a numerical inverse).
    pub fn pixel_to_unit(&self) -> Homography {
        let c = self.offset();
        let (sy, ty) = match self.cfg.y_axis {
            YAxis::Down => (1.0 / self.height, c / self.height),
            YAxis::Up => (-1.0 / self.height, 1.0 - c / self.height),
        };
        let m = [
            [1.0 / self.width, 0.0, c / self.width],
            [0.0, sy, ty],
            [0.0, 0.0, 1.0],
        ];
        Homography::from_matrix(m, Convention::Column).with_convention(self.cfg.convention)
    }

    /// Pixel coordinates to physical coordinates: `x * pixel_aspect`, `y` unchanged.
    pub fn pixel_to_physical(&self) -> Homography {
        Homography::scale(self.cfg.convention, self.cfg.pixel_aspect, 1.0)
    }

    /// Physical coordinates to pixel coordinates: `x / pixel_aspect`.
    pub fn physical_to_pixel(&self) -> Homography {
        Homography::scale(self.cfg.convention, 1.0 / self.cfg.pixel_aspect, 1.0)
    }

    /// Normalised unit square to physical coordinates.
    pub fn unit_to_physical(&self) -> Homography {
        self.unit_to_pixel().then(&self.pixel_to_physical())
    }

    /// Physical coordinates to the normalised unit square.
    pub fn physical_to_unit(&self) -> Homography {
        self.physical_to_pixel().then(&self.pixel_to_unit())
    }

    /// A pixel rectangle as a normalised rectangle (corners mapped, then re-sorted so that
    /// `x0 <= x1` and `y0 <= y1`, which matters for Y up).
    pub fn pixel_rect_to_unit(&self, r: &Rect) -> Rect {
        let h = self.pixel_to_unit();
        let pts = r.corners().map(|p| affine(&h, p));
        Rect::bounding(&pts).unwrap_or(*r)
    }

    /// A normalised rectangle as a pixel rectangle, sorted as above.
    pub fn unit_rect_to_pixel(&self, r: &Rect) -> Rect {
        let h = self.unit_to_pixel();
        let pts = r.corners().map(|p| affine(&h, p));
        Rect::bounding(&pts).unwrap_or(*r)
    }
}

fn affine(h: &Homography, p: Point) -> Point {
    let [x, y, _] = h.apply_homogeneous(p);
    Point::new(x, y)
}
