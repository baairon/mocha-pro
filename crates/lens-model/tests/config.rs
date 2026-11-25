//! Configuration, normalisation, pixel conversion and the configuration space.
use lens_model::{Config, ConfigSpace, InverseMethod, LensError, Model, ModelMaps, Normalisation};

fn k1(v: f64) -> Model {
    Model::Radial {
        k: [v, 0.0, 0.0],
        p: [0.0; 2],
    }
}

#[test]
fn default_is_identity_two_by_two() {
    let c = Config::default();
    assert_eq!(c.size, [2.0, 2.0]);
    assert_eq!(c.centre, [1.0, 1.0]);
    assert_eq!(c.pixel_aspect, 1.0);
    assert_eq!(c.model, Model::identity());
    assert_eq!(c.to_distorted([0.25, 1.5]).unwrap(), [0.25, 1.5]);
}

#[test]
fn new_centres_the_image() {
    let c = Config::new(6.0, 8.0, Model::identity());
    assert_eq!(c.centre, [3.0, 4.0]);
    assert_eq!(c.size, [6.0, 8.0]);
}

#[test]
fn scale_half_width_height_diagonal() {
    let mut c = Config::new(6.0, 8.0, Model::identity());
    assert_eq!(c.scale(), 3.0);
    c.normalisation = Normalisation::HalfHeight;
    assert_eq!(c.scale(), 4.0);
    c.normalisation = Normalisation::HalfDiagonal;
    assert_eq!(c.scale(), 5.0);
    c.normalisation = Normalisation::Focal(1234.5);
    assert_eq!(c.scale(), 1234.5);
}

#[test]
fn scale_with_pixel_aspect() {
    let mut c = Config::new(4.0, 6.0, Model::identity());
    c.pixel_aspect = 2.0;
    assert_eq!(c.scale(), 4.0);
    c.normalisation = Normalisation::HalfHeight;
    assert_eq!(c.scale(), 3.0);
    c.normalisation = Normalisation::HalfDiagonal;
    assert_eq!(c.scale(), 5.0);
}

#[test]
fn to_normalised_values() {
    let c = Config::new(6.0, 8.0, Model::identity());
    assert_eq!(c.to_normalised([6.0, 4.0]), [1.0, 0.0]);
    assert_eq!(c.to_normalised([3.0, 8.0]), [0.0, 4.0 / 3.0]);
    assert_eq!(c.to_normalised([0.0, 0.0]), [-1.0, -4.0 / 3.0]);
}

#[test]
fn to_normalised_pixel_aspect_scales_x_only() {
    let mut c = Config::new(8.0, 8.0, Model::identity());
    c.pixel_aspect = 0.5;
    // scale = 4 * 0.5 = 2; x = (8 - 4) * 0.5 / 2 = 1; y = (8 - 4) / 2 = 2
    assert_eq!(c.to_normalised([8.0, 8.0]), [1.0, 2.0]);
}

#[test]
fn from_normalised_inverts_to_normalised() {
    let mut c = Config::new(8.0, 8.0, Model::identity());
    c.pixel_aspect = 2.0;
    c.centre = [3.0, 5.0];
    for p in [[0.0, 0.0], [8.0, 8.0], [2.5, 7.0]] {
        assert_eq!(c.from_normalised(c.to_normalised(p)), p);
    }
}

#[test]
fn centre_offset_moves_the_fixed_point() {
    let mut c = Config::new(8.0, 8.0, k1(0.5));
    c.centre = [2.0, 2.0];
    assert_eq!(c.to_distorted([2.0, 2.0]).unwrap(), [2.0, 2.0]);
    assert_ne!(c.to_distorted([4.0, 4.0]).unwrap(), [4.0, 4.0]);
}

#[test]
fn to_distorted_forward_by_hand() {
    // size 6x8, scale 3, centre (3,4): (4.5,4) -> x 0.5 -> 0.5625 -> 4.6875
    let c = Config::new(6.0, 8.0, k1(0.5));
    assert_eq!(c.to_distorted([4.5, 4.0]).unwrap(), [4.6875, 4.0]);
}

#[test]
fn swapped_maps_make_the_formula_the_undistortion() {
    let mut c = Config::new(6.0, 8.0, k1(0.5));
    c.maps = ModelMaps::DistortedToUndistorted;
    assert_eq!(c.to_undistorted([4.5, 4.0]).unwrap(), [4.6875, 4.0]);
}

#[test]
fn to_undistorted_inverts_to_distorted() {
    let mut c = Config::new(6.0, 8.0, k1(0.0625));
    c.inverse_iterations = 12;
    for p in [[4.5, 4.0], [1.5, 2.0], [3.0, 7.0]] {
        let d = c.to_distorted(p).unwrap();
        let u = c.to_undistorted(d).unwrap();
        assert!((u[0] - p[0]).abs() < 1e-13 && (u[1] - p[1]).abs() < 1e-13);
    }
}

#[test]
fn closed_form_division_through_config() {
    // lambda -8, scale 3, centre (3,4): (4.5, 4) -> n = (0.5, 0) -> 0.25 -> 3.75
    let mut c = Config::new(6.0, 8.0, Model::Division { lambda: -8.0 });
    c.maps = ModelMaps::DistortedToUndistorted;
    c.inverse_method = InverseMethod::ClosedForm;
    assert_eq!(c.to_distorted([4.5, 4.0]).unwrap(), [3.75, 4.0]);
    assert_eq!(c.to_undistorted([3.75, 4.0]).unwrap(), [4.5, 4.0]);
}

#[test]
fn closed_form_domain_error_through_config() {
    let mut c = Config::new(2.0, 2.0, Model::Division { lambda: 1.0 });
    c.normalisation = Normalisation::Focal(1.0);
    c.centre = [0.0, 0.0];
    c.inverse_method = InverseMethod::ClosedForm;
    assert_eq!(c.to_undistorted([1.0, 0.0]), Err(LensError::OutsideDomain));
}

#[test]
fn closed_form_on_polynomial_through_config() {
    let mut c = Config::new(2.0, 2.0, k1(0.5));
    c.inverse_method = InverseMethod::ClosedForm;
    assert_eq!(c.to_undistorted([1.0, 1.0]), Err(LensError::NoClosedForm));
    assert!(c.to_distorted([1.0, 1.0]).is_ok());
}

#[test]
fn validate_rejects_bad_fields() {
    let good = Config::new(6.0, 8.0, Model::identity());
    assert_eq!(good.validate(), Ok(()));
    let mut c = good;
    c.size = [0.0, 8.0];
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.size = [6.0, -1.0];
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.size = [f64::INFINITY, 8.0];
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.pixel_aspect = 0.0;
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.pixel_aspect = f64::NAN;
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.normalisation = Normalisation::Focal(0.0);
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.normalisation = Normalisation::Focal(f64::INFINITY);
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.centre = [f64::NAN, 0.0];
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.centre = [0.0, f64::INFINITY];
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.size = [f64::NAN, 8.0];
    assert!(c.validate().is_err());
    let mut c = good;
    c.size = [6.0, f64::INFINITY];
    assert!(c.validate().is_err());
}

#[test]
fn maps_refuse_bad_config() {
    let mut c = Config::new(6.0, 8.0, Model::identity());
    c.pixel_aspect = -1.0;
    assert_eq!(c.to_distorted([1.0, 1.0]), Err(LensError::BadConfig));
    assert_eq!(c.to_undistorted([1.0, 1.0]), Err(LensError::BadConfig));
}

#[test]
fn space_len_and_enumerate_agree() {
    let s = ConfigSpace::new(vec![k1(0.5), Model::Division { lambda: 1.0 }]);
    // 2 models * (3 + 1) norms * 2 maps * 1 * 1 * 2 methods * 1
    assert_eq!(s.len(), 32);
    assert_eq!(s.enumerate(6.0, 8.0).len(), 32);
    assert!(!s.is_empty());
}

#[test]
fn space_order_is_fixed_innermost_iterations() {
    let mut s = ConfigSpace::new(vec![k1(0.5)]);
    s.iterations = vec![3, 9];
    let v = s.enumerate(6.0, 8.0);
    assert_eq!(v[0].inverse_iterations, 3);
    assert_eq!(v[1].inverse_iterations, 9);
    assert_eq!(v[0].inverse_method, InverseMethod::FixedPoint);
    assert_eq!(v[2].inverse_method, InverseMethod::Newton);
    assert_eq!(v[0].normalisation, Normalisation::HalfWidth);
    assert_eq!(v[0].maps, ModelMaps::UndistortedToDistorted);
    assert_eq!(v[4].maps, ModelMaps::DistortedToUndistorted);
    assert_eq!(v[8].normalisation, Normalisation::HalfHeight);
    assert_eq!(v[16].normalisation, Normalisation::HalfDiagonal);
    assert_eq!(v[24].normalisation, Normalisation::Focal(1.0));
}

#[test]
fn space_centre_offsets_and_aspects() {
    let mut s = ConfigSpace::new(vec![k1(0.5)]);
    s.centre_offsets = vec![[0.0, 0.0], [0.5, -1.0]];
    s.pixel_aspects = vec![1.0, 2.0];
    s.focal_lengths = vec![];
    s.maps = vec![ModelMaps::UndistortedToDistorted];
    s.methods = vec![InverseMethod::Newton];
    let v = s.enumerate(6.0, 8.0);
    assert_eq!(v.len(), 12);
    assert_eq!(v[0].centre, [3.0, 4.0]);
    assert_eq!(v[1].centre, [3.5, 3.0]);
    assert_eq!(v[0].pixel_aspect, 1.0);
    assert_eq!(v[2].pixel_aspect, 2.0);
    assert!(v.iter().all(|c| c.size == [6.0, 8.0]));
}

#[test]
fn space_empty_axis_gives_empty() {
    let s = ConfigSpace::new(vec![]);
    assert!(s.is_empty());
    assert!(s.enumerate(6.0, 8.0).is_empty());
    let mut s = ConfigSpace::new(vec![k1(0.5)]);
    s.iterations = vec![];
    assert!(s.is_empty());
}

#[test]
fn space_every_config_validates() {
    let mut s = ConfigSpace::new(vec![k1(0.5)]);
    s.focal_lengths = vec![96.0, 2048.0];
    for c in s.enumerate(64.0, 48.0) {
        assert_eq!(c.validate(), Ok(()));
    }
}

#[test]
fn space_conventions_change_results() {
    let mut s = ConfigSpace::new(vec![k1(0.5)]);
    s.focal_lengths = vec![64.0];
    let v = s.enumerate(64.0, 48.0);
    let mut seen = Vec::new();
    for c in &v {
        if let Ok(r) = c.to_distorted([40.0, 30.0]) {
            seen.push([r[0].to_bits(), r[1].to_bits()]);
        }
    }
    seen.sort();
    seen.dedup();
    assert!(seen.len() >= 6);
}

#[test]
fn validate_rejects_zero_aspect_and_zero_height_with_focal_length() {
    let mut good = Config::new(6.0, 8.0, Model::identity());
    good.normalisation = Normalisation::Focal(1.0);
    assert_eq!(good.validate(), Ok(()));
    let mut c = good;
    c.pixel_aspect = 0.0;
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.size = [6.0, 0.0];
    assert_eq!(c.validate(), Err(LensError::BadConfig));
    let mut c = good;
    c.size = [0.0, 8.0];
    assert_eq!(c.validate(), Err(LensError::BadConfig));
}
