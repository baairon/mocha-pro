//! Stereo layouts as a rectangle split of the image.
//!
//! Over under puts the first eye on the top half and the second on the bottom half. Side by side puts
//! the first eye on the left half and the second on the right half. An odd extent gives the first half
//! `n / 2` (floor) and the second the rest. `swap_eyes` exchanges the two rectangles.

/// An integer rectangle: origin (x, y) and extent (w, h).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    /// Pixel coordinate of the whole image to the coordinate inside the rectangle.
    pub fn to_local(&self, x: f64, y: f64) -> (f64, f64) {
        (x - f64::from(self.x), y - f64::from(self.y))
    }

    /// Coordinate inside the rectangle to the pixel coordinate of the whole image.
    pub fn to_global(&self, x: f64, y: f64) -> (f64, f64) {
        (x + f64::from(self.x), y + f64::from(self.y))
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= f64::from(self.x)
            && y >= f64::from(self.y)
            && x < f64::from(self.x + self.w)
            && y < f64::from(self.y + self.h)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StereoLayout {
    /// One view: the whole image.
    Mono,
    OverUnder,
    SideBySide,
}

impl StereoLayout {
    pub const ALL: [StereoLayout; 3] = [
        StereoLayout::Mono,
        StereoLayout::OverUnder,
        StereoLayout::SideBySide,
    ];

    /// Number of views.
    pub fn views(self) -> usize {
        match self {
            StereoLayout::Mono => 1,
            _ => 2,
        }
    }

    /// Rectangle of view `eye` (0 or 1; any index of a mono layout gives the whole image), after the
    /// optional exchange of the two eyes.
    pub fn rect(self, w: u32, h: u32, swap_eyes: bool, eye: usize) -> Rect {
        let e = if swap_eyes { 1 - (eye & 1) } else { eye & 1 };
        match self {
            StereoLayout::Mono => Rect { x: 0, y: 0, w, h },
            StereoLayout::OverUnder => {
                let h0 = h / 2;
                if e == 0 {
                    Rect {
                        x: 0,
                        y: 0,
                        w,
                        h: h0,
                    }
                } else {
                    Rect {
                        x: 0,
                        y: h0,
                        w,
                        h: h - h0,
                    }
                }
            }
            StereoLayout::SideBySide => {
                let w0 = w / 2;
                if e == 0 {
                    Rect {
                        x: 0,
                        y: 0,
                        w: w0,
                        h,
                    }
                } else {
                    Rect {
                        x: w0,
                        y: 0,
                        w: w - w0,
                        h,
                    }
                }
            }
        }
    }
}
