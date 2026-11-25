//! Configuration: every convention a calibration may need to vary is a field here.

use crate::model::{InverseMethod, LensError, Model};

/// What pixel offsets are divided by to give normalised coordinates. Pixel units are vertical
/// pixels; a horizontal offset is first multiplied by the pixel aspect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Normalisation {
    /// `size_x / 2 * pixel_aspect`.
    HalfWidth,
    /// `size_y / 2`.
    HalfHeight,
    /// `sqrt((size_x / 2 * pixel_aspect)^2 + (size_y / 2)^2)`.
    HalfDiagonal,
    /// A focal length in pixels, used as given.
    Focal(f64),
}

/// Which way the model formula maps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelMaps {
    /// The formula takes an undistorted point to its distorted position.
    UndistortedToDistorted,
    /// The formula takes a distorted point to its undistorted position.
    DistortedToUndistorted,
}

/// A full lens configuration. [`Config::default`] is an arbitrary starting point that claims
/// nothing: an identity model on a 2 by 2 image with the centre at the image centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub model: Model,
    pub normalisation: Normalisation,
    /// Image width and height in pixels.
    pub size: [f64; 2],
    /// Distortion centre in pixels.
    pub centre: [f64; 2],
    /// Pixel width divided by pixel height.
    pub pixel_aspect: f64,
    pub maps: ModelMaps,
    /// Method used where the map being asked for is the inverse of the model formula.
    pub inverse_method: InverseMethod,
    /// Fixed step count of the iterative inverse methods.
    pub inverse_iterations: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            model: Model::identity(),
            normalisation: Normalisation::HalfWidth,
            size: [2.0, 2.0],
            centre: [1.0, 1.0],
            pixel_aspect: 1.0,
            maps: ModelMaps::UndistortedToDistorted,
            inverse_method: InverseMethod::Newton,
            inverse_iterations: 8,
        }
    }
}

impl Config {
    /// A default configuration for an image of the given size with the centre at its middle.
    pub fn new(width: f64, height: f64, model: Model) -> Config {
        Config {
            model,
            size: [width, height],
            centre: [width * 0.5, height * 0.5],
            ..Config::default()
        }
    }

    /// The normalisation divisor.
    pub fn scale(&self) -> f64 {
        let hw = self.size[0] * 0.5 * self.pixel_aspect;
        let hh = self.size[1] * 0.5;
        match self.normalisation {
            Normalisation::HalfWidth => hw,
            Normalisation::HalfHeight => hh,
            Normalisation::HalfDiagonal => (hw * hw + hh * hh).sqrt(),
            Normalisation::Focal(f) => f,
        }
    }

    /// Checks that size, aspect, scale and centre are usable.
    pub fn validate(&self) -> Result<(), LensError> {
        let s = self.scale();
        let ok = self.size[0] > 0.0
            && self.size[1] > 0.0
            && self.size[0].is_finite()
            && self.size[1].is_finite()
            && self.pixel_aspect > 0.0
            && self.pixel_aspect.is_finite()
            && s > 0.0
            && s.is_finite()
            && self.centre[0].is_finite()
            && self.centre[1].is_finite();
        if ok {
            Ok(())
        } else {
            Err(LensError::BadConfig)
        }
    }

    /// Pixel to normalised: `((px - cx) * aspect / s, (py - cy) / s)`.
    pub fn to_normalised(&self, px: [f64; 2]) -> [f64; 2] {
        let s = self.scale();
        [
            (px[0] - self.centre[0]) * self.pixel_aspect / s,
            (px[1] - self.centre[1]) / s,
        ]
    }

    /// Normalised to pixel, the inverse of [`Config::to_normalised`].
    pub fn from_normalised(&self, n: [f64; 2]) -> [f64; 2] {
        let s = self.scale();
        [
            n[0] * s / self.pixel_aspect + self.centre[0],
            n[1] * s + self.centre[1],
        ]
    }

    fn forward_n(&self, n: [f64; 2]) -> Result<[f64; 2], LensError> {
        self.model.apply(n)
    }

    fn inverse_n(&self, n: [f64; 2]) -> Result<[f64; 2], LensError> {
        self.model
            .invert(n, self.inverse_method, self.inverse_iterations)
    }

    /// Undistorted pixel position to distorted pixel position.
    pub fn to_distorted(&self, px: [f64; 2]) -> Result<[f64; 2], LensError> {
        self.validate()?;
        let n = self.to_normalised(px);
        let m = match self.maps {
            ModelMaps::UndistortedToDistorted => self.forward_n(n)?,
            ModelMaps::DistortedToUndistorted => self.inverse_n(n)?,
        };
        Ok(self.from_normalised(m))
    }

    /// Distorted pixel position to undistorted pixel position.
    pub fn to_undistorted(&self, px: [f64; 2]) -> Result<[f64; 2], LensError> {
        self.validate()?;
        let n = self.to_normalised(px);
        let m = match self.maps {
            ModelMaps::UndistortedToDistorted => self.inverse_n(n)?,
            ModelMaps::DistortedToUndistorted => self.forward_n(n)?,
        };
        Ok(self.from_normalised(m))
    }
}

/// The axes of the configuration space. [`ConfigSpace::enumerate`] returns every combination in a
/// fixed order: models outermost, then normalisation, maps, aspect, centre offset, method,
/// iteration count innermost.
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigSpace {
    pub models: Vec<Model>,
    /// Focal lengths in pixels offered besides the three size based normalisations.
    pub focal_lengths: Vec<f64>,
    pub maps: Vec<ModelMaps>,
    pub pixel_aspects: Vec<f64>,
    /// Centre offsets in pixels from the image middle.
    pub centre_offsets: Vec<[f64; 2]>,
    pub methods: Vec<InverseMethod>,
    pub iterations: Vec<u32>,
}

impl ConfigSpace {
    /// A space over the given models with default axes for everything else.
    pub fn new(models: Vec<Model>) -> ConfigSpace {
        ConfigSpace {
            models,
            focal_lengths: vec![1.0],
            maps: vec![
                ModelMaps::UndistortedToDistorted,
                ModelMaps::DistortedToUndistorted,
            ],
            pixel_aspects: vec![1.0],
            centre_offsets: vec![[0.0, 0.0]],
            methods: vec![InverseMethod::FixedPoint, InverseMethod::Newton],
            iterations: vec![8],
        }
    }

    /// Number of configurations [`ConfigSpace::enumerate`] returns.
    pub fn len(&self) -> usize {
        self.models.len()
            * (3 + self.focal_lengths.len())
            * self.maps.len()
            * self.pixel_aspects.len()
            * self.centre_offsets.len()
            * self.methods.len()
            * self.iterations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every configuration for an image of the given size.
    pub fn enumerate(&self, width: f64, height: f64) -> Vec<Config> {
        let mut norms = vec![
            Normalisation::HalfWidth,
            Normalisation::HalfHeight,
            Normalisation::HalfDiagonal,
        ];
        norms.extend(self.focal_lengths.iter().map(|&f| Normalisation::Focal(f)));
        let mut out = Vec::with_capacity(self.len());
        for &model in &self.models {
            for &normalisation in &norms {
                for &maps in &self.maps {
                    for &pixel_aspect in &self.pixel_aspects {
                        for off in &self.centre_offsets {
                            for &inverse_method in &self.methods {
                                for &inverse_iterations in &self.iterations {
                                    out.push(Config {
                                        model,
                                        normalisation,
                                        size: [width, height],
                                        centre: [width * 0.5 + off[0], height * 0.5 + off[1]],
                                        pixel_aspect,
                                        maps,
                                        inverse_method,
                                        inverse_iterations,
                                    });
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
