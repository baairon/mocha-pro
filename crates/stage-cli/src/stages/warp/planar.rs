//! What the two planar stages share: the nine convention parameters that map onto
//! `planar_xform::Config`, and the map from an output position to a source position for a 3 by 3
//! matrix.
//!
//! The chain of one frame, for a matrix `M` given in the coordinates that `coords` names:
//!
//! 1. `maps` says which way `M` goes. `output-to-source` uses `M` as it is. `source-to-output`
//!    uses `Homography::inverse` of `M`; a matrix without an inverse is refused.
//! 2. `coords` says where `M` acts. `pixel`: on pixel positions with the x coordinate multiplied
//!    by `pixel_aspect` (`PixelSpace::pixel_to_physical` of the output, `M`,
//!    `PixelSpace::physical_to_pixel` of the source). `unit`: on the unit square of each frame
//!    (`PixelSpace::pixel_to_unit` of the output, `M`, `PixelSpace::unit_to_pixel` of the
//!    source), where `y_axis` says whether v grows down or up.
//! 3. The three are composed once per frame with `Homography::then`, and each output position
//!    goes through `Homography::apply`, where `guard` and `divisor_min` judge the divisor. A
//!    position the guard refuses has no source position.
//!
//! `xform_center` says where the pixel centres lie in the pixel coordinates of the matrix
//! (`half`: pixel i has its centre at i + 0.5; `integer`: at i). The resampler has its own rule
//! (`center`). When the two differ, a position is moved by the difference before the chain and
//! back after it, so each parameter keeps its meaning whatever the other says.

use planar_xform::{
    Config, Convention, DivisorGuard, Error, Homography, PixelCenter, PixelSpace, Point, YAxis,
};

use crate::error::StageError;
use crate::params::Values;
use crate::registry::ParamDef;

use super::sampler::{choice, float};

/// The number of convention parameters.
pub const CONVENTIONS: usize = 9;

/// The convention parameters of a planar stage. `maps` and `coords` are the defaults of the two
/// choices that differ between the stages; every other default is that of
/// `planar_xform::Config`.
pub const fn conventions(
    maps: &'static str,
    maps_about: &'static str,
    coords: &'static str,
    coords_about: &'static str,
) -> [ParamDef; CONVENTIONS] {
    [
        choice("maps", "Maps", maps, &["output-to-source", "source-to-output"], maps_about),
        choice("convention", "Vector convention", "column", &["column", "row"], "Whether a point is a column vector (p' = M p) or a row vector (p' = p M) (Config::convention)."),
        choice("guard", "Divisor guard", "return-none", &["return-none", "clamp", "pass-through"], "What happens where the homogeneous divisor is not above divisor_min: the pixel is zero, the divisor is replaced by divisor_min, or the numerators are used undivided (Config::guard)."),
        float("divisor_min", "Divisor minimum", (0.0, -1e9, 1e9), false, "The divisor is valid when it is above this value (Config::divisor_min); with guard clamp it is also the value divided by."),
        choice("coords", "Coordinates", coords, &["pixel", "unit"], coords_about),
        choice("xform_center", "Matrix pixel centre", "half", &["integer", "half"], "Where pixel i has its centre in the pixel coordinates of the matrix: at i, or at i + 0.5 (Config::center)."),
        choice("y_axis", "Y axis", "down", &["down", "up"], "Whether v of the unit square grows down or up the rows; read when coords is unit (Config::y_axis)."),
        float("pixel_aspect", "Pixel aspect", (1.0, 0.01, 100.0), false, "The width of a pixel over its height; read when coords is pixel (Config::pixel_aspect)."),
        float("singular_tolerance", "Singular tolerance", (1e-12, 0.0, 1.0), false, "The relative pivot tolerance of the four point solve (Config::singular_tolerance)."),
    ]
}

/// The configuration of `planar_xform` for a set of values.
pub fn config(values: &Values) -> Config {
    let divisor_min = values.float("divisor_min");
    Config {
        convention: match values.choice("convention") {
            "row" => Convention::Row,
            _ => Convention::Column,
        },
        guard: match values.choice("guard") {
            "clamp" => DivisorGuard::Clamp(divisor_min),
            "pass-through" => DivisorGuard::PassThrough,
            _ => DivisorGuard::ReturnNone,
        },
        divisor_min,
        center: match values.choice("xform_center") {
            "integer" => PixelCenter::Integer,
            _ => PixelCenter::Half,
        },
        y_axis: match values.choice("y_axis") {
            "up" => YAxis::Up,
            _ => YAxis::Down,
        },
        pixel_aspect: values.float("pixel_aspect"),
        singular_tolerance: values.float("singular_tolerance"),
    }
}

/// What `xform_center` adds to a pixel index to give the centre of that pixel.
pub fn matrix_offset(cfg: &Config) -> f64 {
    match cfg.center {
        PixelCenter::Integer => 0.0,
        PixelCenter::Half => 0.5,
    }
}

/// The refusal for a matrix or a set of corners that `planar_xform` does not take.
pub fn refuse(index: u64, what: &str, error: Error) -> StageError {
    let why = match error {
        Error::Singular => "is singular",
        Error::NonFinite => "gives a value that is not finite",
    };
    StageError::unsupported(format!("frame {index}: {what} {why}"))
}

/// The transform from output pixel positions to source pixel positions, in the pixel
/// coordinates of the matrix: steps 1 to 3 of the module text.
pub fn chain(
    matrix: Homography,
    values: &Values,
    src: (u32, u32),
    dst: (u32, u32),
) -> Result<Homography, Error> {
    let cfg = config(values);
    let to_source = match values.choice("maps") {
        "source-to-output" => matrix.inverse()?,
        _ => matrix,
    };
    let from = PixelSpace::new(f64::from(dst.0), f64::from(dst.1), cfg);
    let to = PixelSpace::new(f64::from(src.0), f64::from(src.1), cfg);
    Ok(match values.choice("coords") {
        "unit" => from
            .pixel_to_unit()
            .then(&to_source)
            .then(&to.unit_to_pixel()),
        _ => from
            .pixel_to_physical()
            .then(&to_source)
            .then(&to.physical_to_pixel()),
    })
}

/// The map of one frame for the resampler: an output position to its source position, both in
/// the coordinates of the resampler, whose pixel centres lie at an index plus `sample_offset`.
pub fn source_map(
    matrix: Homography,
    values: &Values,
    src: (u32, u32),
    dst: (u32, u32),
    sample_offset: f64,
) -> Result<impl Fn(f64, f64) -> Option<(f64, f64)>, Error> {
    let cfg = config(values);
    let total = chain(matrix, values, src, dst)?;
    let shift = matrix_offset(&cfg) - sample_offset;
    Ok(move |x: f64, y: f64| {
        let p = total.apply(Point::new(x + shift, y + shift), &cfg)?;
        Some((p.x - shift, p.y - shift))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Geometry, Kind, Label, StageDef};

    static PARAMS: [ParamDef; CONVENTIONS] = conventions(
        "output-to-source",
        "A made up line.",
        "pixel",
        "A made up line.",
    );

    static DEF: StageDef = StageDef {
        id: "planar-test",
        name: "Planar test",
        about: "A made up definition that holds the convention parameters.",
        label: Label::Generic,
        kind: Kind::Frame,
        spool: false,
        geometry: Geometry::Declared,
        depths: &crate::frame::DEPTHS,
        params: &PARAMS,
        layers: &[],
    };

    fn values(pairs: &[(&str, &str)]) -> Values {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Values::parse(&DEF, &pairs).unwrap()
    }

    #[test]
    fn the_defaults_are_the_defaults_of_the_crate() {
        assert_eq!(config(&Values::defaults(&DEF)), Config::default());
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        assert_eq!(
            config(&values(&[("convention", "row")])).convention,
            Convention::Row
        );
        assert_eq!(
            config(&values(&[("guard", "pass-through")])).guard,
            DivisorGuard::PassThrough
        );
        let clamp = config(&values(&[("guard", "clamp"), ("divisor_min", "0.25")]));
        assert_eq!(clamp.guard, DivisorGuard::Clamp(0.25));
        assert_eq!(clamp.divisor_min, 0.25);
        let integer = config(&values(&[("xform_center", "integer")]));
        assert_eq!(integer.center, PixelCenter::Integer);
        assert_eq!(matrix_offset(&integer), 0.0);
        assert_eq!(matrix_offset(&Config::default()), 0.5);
        assert_eq!(config(&values(&[("y_axis", "up")])).y_axis, YAxis::Up);
        assert_eq!(config(&values(&[("pixel_aspect", "2")])).pixel_aspect, 2.0);
        assert_eq!(
            config(&values(&[("singular_tolerance", "0.5")])).singular_tolerance,
            0.5
        );
    }

    #[test]
    fn the_identity_at_the_defaults_moves_nothing() {
        let identity = Homography::identity(Convention::Column);
        let map = source_map(identity, &Values::defaults(&DEF), (5, 3), (5, 3), 0.5).unwrap();
        for (x, y) in [(0.5, 0.5), (4.5, 2.5), (2.5, 1.5)] {
            assert_eq!(map(x, y), Some((x, y)));
        }
    }

    #[test]
    fn a_source_to_output_matrix_is_inverted() {
        // The matrix moves the source two pixels right: output x reads source x - 2.
        let shift = Homography::translation(Convention::Column, 2.0, 0.0);
        let forward = values(&[("maps", "source-to-output")]);
        let map = source_map(shift, &forward, (4, 4), (4, 4), 0.5).unwrap();
        assert_eq!(map(2.5, 1.5), Some((0.5, 1.5)));
        let direct = source_map(shift, &Values::defaults(&DEF), (4, 4), (4, 4), 0.5).unwrap();
        assert_eq!(direct(0.5, 1.5), Some((2.5, 1.5)));
        let flat = Homography::from_matrix([[0.0; 3]; 3], Convention::Column);
        assert_eq!(
            source_map(flat, &forward, (4, 4), (4, 4), 0.5).err(),
            Some(Error::Singular)
        );
        assert_eq!(
            refuse(3, "the matrix", Error::Singular).to_string(),
            "error: unsupported: frame 3: the matrix is singular"
        );
    }

    #[test]
    fn unit_coordinates_span_each_frame() {
        // The identity on the unit square scales an 8 by 4 output onto a 4 by 2 source.
        let identity = Homography::identity(Convention::Column);
        let unit = values(&[("coords", "unit")]);
        let map = source_map(identity, &unit, (4, 2), (8, 4), 0.5).unwrap();
        assert_eq!(map(4.0, 2.0), Some((2.0, 1.0)));
        assert_eq!(map(8.0, 4.0), Some((4.0, 2.0)));
        // With v growing up, the identity still reads the same row.
        let up = values(&[("coords", "unit"), ("y_axis", "up")]);
        let map = source_map(identity, &up, (4, 2), (4, 2), 0.5).unwrap();
        assert_eq!(map(1.0, 0.5), Some((1.0, 0.5)));
    }

    #[test]
    fn pixel_aspect_scales_x_around_the_matrix() {
        // A shift of 2 in physical x is a shift of 1 pixel when a pixel is 2 wide.
        let shift = Homography::translation(Convention::Column, 2.0, 0.0);
        let wide = values(&[("pixel_aspect", "2")]);
        let map = source_map(shift, &wide, (4, 4), (4, 4), 0.5).unwrap();
        assert_eq!(map(0.5, 0.5), Some((1.5, 0.5)));
    }

    #[test]
    fn the_two_centre_rules_are_independent() {
        // Integer centres in the matrix, half centres in the resampler: a doubling about the
        // centre of pixel 0 keeps that centre in place.
        let double = Homography::scale(Convention::Column, 2.0, 2.0);
        let integer = values(&[("xform_center", "integer")]);
        let map = source_map(double, &integer, (4, 4), (4, 4), 0.5).unwrap();
        assert_eq!(map(0.5, 0.5), Some((0.5, 0.5)));
        assert_eq!(map(1.5, 0.5), Some((2.5, 0.5)));
        // Half centres on both sides: the doubling is about the corner of the frame.
        let map = source_map(double, &Values::defaults(&DEF), (4, 4), (4, 4), 0.5).unwrap();
        assert_eq!(map(0.5, 0.5), Some((1.0, 1.0)));
    }

    #[test]
    fn the_guard_judges_the_divisor() {
        // w = x - 2: not above zero for x <= 2.
        let m = Homography::from_matrix(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, -2.0]],
            Convention::Column,
        );
        let none = source_map(m, &Values::defaults(&DEF), (4, 4), (4, 4), 0.5).unwrap();
        assert_eq!(none(1.5, 0.5), None);
        assert_eq!(none(2.5, 0.5), Some((5.0, 1.0)));
        let pass = values(&[("guard", "pass-through")]);
        let pass = source_map(m, &pass, (4, 4), (4, 4), 0.5).unwrap();
        assert_eq!(pass(1.5, 0.5), Some((1.5, 0.5)));
        let clamp = values(&[("guard", "clamp"), ("divisor_min", "0.25")]);
        let clamp = source_map(m, &clamp, (4, 4), (4, 4), 0.5).unwrap();
        assert_eq!(clamp(1.5, 0.5), Some((6.0, 2.0)));
    }
}
