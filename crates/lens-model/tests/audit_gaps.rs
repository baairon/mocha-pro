//! Cases found by a mutation audit to be unpinned by the first test set.
use lens_model::{Config, Direction, InverseMethod, LensError, Model, Normalisation};

#[test]
fn division_negative_denominator_is_a_valid_point() {
    // lambda = -0.25 at (3, 0): d = 1 - 2.25 = -1.25, a negative but nonzero denominator.
    let m = Model::Division { lambda: -0.25 };
    assert_eq!(m.apply([3.0, 0.0]), Ok([3.0 / -1.25, 0.0 / -1.25]));
}

#[test]
fn one_non_finite_component_is_an_error() {
    // y overflows (2*y*y = inf) while x stays finite: f = (0, inf).
    let m = Model::Radial {
        k: [0.0; 3],
        p: [1.0, 0.0],
    };
    let t = [0.0, 1e154];
    assert_eq!(m.apply(t), Err(LensError::NonFinite));
    assert_eq!(
        m.invert(t, InverseMethod::Newton, 4),
        Err(LensError::NonFinite)
    );
    assert_eq!(
        m.invert(t, InverseMethod::FixedPoint, 4),
        Err(LensError::NonFinite)
    );
}

#[test]
fn focal_length_normalisation_ignores_the_aspect_divisor() {
    // The horizontal offset is multiplied by the aspect, then divided by the focal length as given.
    let mut c = Config::new(96.0, 48.0, Model::identity());
    c.normalisation = Normalisation::Focal(10.0);
    c.pixel_aspect = 2.0;
    assert_eq!(c.scale(), 10.0);
    assert_eq!(c.to_normalised([58.0, 29.0]), [2.0, 0.5]);
    assert_eq!(c.from_normalised([2.0, 0.5]), [58.0, 29.0]);
}

#[test]
fn validate_rejects_infinite_values_whatever_the_normalisation() {
    let mut c = Config::new(96.0, 48.0, Model::identity());
    c.normalisation = Normalisation::HalfHeight;
    assert_eq!(c.validate(), Ok(()));
    let mut w = c;
    w.size[0] = f64::INFINITY;
    assert_eq!(w.validate(), Err(LensError::BadConfig));
    let mut a = c;
    a.pixel_aspect = f64::INFINITY;
    assert_eq!(a.validate(), Err(LensError::BadConfig));
    let mut f = c;
    f.normalisation = Normalisation::Focal(f64::INFINITY);
    assert_eq!(f.validate(), Err(LensError::BadConfig));
}

#[test]
fn validate_accepts_a_width_below_one_pixel() {
    assert_eq!(Config::new(0.5, 4.0, Model::identity()).validate(), Ok(()));
}

#[test]
fn bounds_of_an_all_negative_image_are_negative() {
    let mut c = Config::new(
        10.0,
        10.0,
        Model::Radial {
            k: [1.0, 0.0, 0.0],
            p: [0.0; 2],
        },
    );
    c.centre = [90.0, 90.0];
    let b = c.mapped_bounds(Direction::ToDistorted, 2).unwrap();
    assert!(b.max[0] < 0.0 && b.max[1] < 0.0 && b.min[0] < b.max[0]);
}
