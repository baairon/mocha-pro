//! Inverse mapping for a resampler, and the bounds of a mapped image rectangle.

use crate::config::Config;
use crate::model::LensError;

/// Which operation a resampler performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Make a distorted image from an undistorted one.
    ToDistorted,
    /// Make an undistorted image from a distorted one.
    ToUndistorted,
}

/// An axis aligned box in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f64; 2],
    pub max: [f64; 2],
}

impl Config {
    /// The map a resampler needs: for an output pixel position, the source pixel position to read.
    /// Applying distortion reads the undistorted source at `to_undistorted(out)`; removing it reads
    /// the distorted source at `to_distorted(out)`. `None` where the map is not defined.
    pub fn source_map(&self, dir: Direction) -> impl Fn([f64; 2]) -> Option<[f64; 2]> + '_ {
        move |out| {
            match dir {
                Direction::ToDistorted => self.to_undistorted(out),
                Direction::ToUndistorted => self.to_distorted(out),
            }
            .ok()
        }
    }

    /// The forward map of a direction, source to output: distorted positions for
    /// `ToDistorted`, undistorted positions for `ToUndistorted`.
    pub fn target_map(&self, dir: Direction, src: [f64; 2]) -> Result<[f64; 2], LensError> {
        match dir {
            Direction::ToDistorted => self.to_distorted(src),
            Direction::ToUndistorted => self.to_undistorted(src),
        }
    }

    /// Bounding box of the source rectangle `[0, width] x [0, height]` after the forward map of
    /// `dir`, sampled at `steps + 1` points on each of the four edges (corners included, positions
    /// `edge_length * i / steps`). `None` if `steps` is zero or any sample fails.
    pub fn mapped_bounds(&self, dir: Direction, steps: u32) -> Option<Bounds> {
        if steps == 0 {
            return None;
        }
        let (w, h) = (self.size[0], self.size[1]);
        let n = f64::from(steps);
        let mut b = Bounds {
            min: [f64::INFINITY; 2],
            max: [f64::NEG_INFINITY; 2],
        };
        for i in 0..=steps {
            let t = f64::from(i) / n;
            let pts = [[w * t, 0.0], [w * t, h], [0.0, h * t], [w, h * t]];
            for p in pts {
                let m = self.target_map(dir, p).ok()?;
                for (a, &v) in m.iter().enumerate() {
                    if v < b.min[a] {
                        b.min[a] = v;
                    }
                    if v > b.max[a] {
                        b.max[a] = v;
                    }
                }
            }
        }
        Some(b)
    }
}
