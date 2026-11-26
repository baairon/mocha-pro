#![allow(clippy::field_reassign_with_default)]
mod support;
use mod_remove::geom::*;
use mod_remove::*;
use support::*;

#[test]
fn rounding_halves_away_from_zero() {
    assert_eq!(round_px(0.5), 1);
    assert_eq!(round_px(-0.5), -1);
    assert_eq!(round_px(1.5), 2);
    assert_eq!(round_px(-1.5), -2);
    assert_eq!(round_px(2.4), 2);
    assert_eq!(host::round_half_away(-2.6), -3);
}
#[test]
fn transfer_identity_and_translation() {
    let c = Camera::identity();
    assert_eq!(transfer(&c, &c, &IDENT, 5, 7).unwrap(), (5.0, 7.0));
    assert_eq!(
        transfer(&c, &c, &translate(2.0, -1.0), 5, 7).unwrap(),
        (7.0, 6.0)
    );
}
#[test]
fn transfer_scale_and_projective_divisor() {
    let c = Camera::identity();
    let s = [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0];
    assert_eq!(transfer(&c, &c, &s, 3, 4).unwrap(), (6.0, 8.0));
    let d = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0];
    assert_eq!(transfer(&c, &c, &d, 6, 8).unwrap(), (3.0, 4.0));
}
#[test]
fn transfer_with_centre_and_focal() {
    let c = Camera {
        c0: [8.0, 8.0],
        f: 2.0,
        zh: 1.0,
        kind: CamKind::Linear,
    };
    assert_eq!(transfer(&c, &c, &IDENT, 3, 12).unwrap(), (3.0, 12.0));
    let t = translate(1.0, 0.0);
    // back project (3-8)/2 = -2.5, plus 1 = -1.5, project -1.5*2+8 = 5
    assert_eq!(transfer(&c, &c, &t, 3, 8).unwrap(), (5.0, 8.0));
}
#[test]
fn transfer_zh_is_third_coordinate() {
    let c = Camera {
        c0: [0.0, 0.0],
        f: 1.0,
        zh: 2.0,
        kind: CamKind::Linear,
    };
    let h = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    // vx = x + 1*zh = x + 2, vz = zh = 2, result = (x+2)/2
    assert_eq!(transfer(&c, &c, &h, 4, 6).unwrap().0, 3.0);
}
#[test]
fn transfer_zero_divisor_unspecified() {
    let c = Camera::identity();
    let h = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
    assert!(matches!(
        transfer(&c, &c, &h, 1, 1),
        Err(RemoveError::Unspecified(_))
    ));
}
#[test]
fn transfer_non_linear_camera_unspecified() {
    let c = Camera::identity();
    for k in [
        CamKind::Radial1,
        CamKind::Radial2,
        CamKind::FourCoefficient,
        CamKind::DistortionMap,
        CamKind::Equirectangular,
    ] {
        let n = Camera { kind: k, ..c };
        assert!(matches!(
            transfer(&n, &c, &IDENT, 1, 1),
            Err(RemoveError::Unspecified(_))
        ));
        assert!(matches!(
            transfer(&c, &n, &IDENT, 1, 1),
            Err(RemoveError::Unspecified(_))
        ));
    }
}
#[test]
fn transfer_px_rounds() {
    let c = Camera::identity();
    assert_eq!(
        transfer_px(&c, &c, &translate(0.5, -0.5), 2, 2).unwrap(),
        (3, 2)
    );
}
#[test]
fn bilinear_exact_at_pixel_centre() {
    let img = img_fn(Win::new(0, 0, 4, 4), |x, y| [x as f32, y as f32, 0.25, 1.0]);
    assert_eq!(
        bits(bilinear_clamp(&img, 2.5, 1.5)),
        bits([2.0, 1.0, 0.25, 1.0])
    );
}
#[test]
fn bilinear_half_pixel_is_mean() {
    let img = img_fn(Win::new(0, 0, 4, 4), |x, y| {
        [x as f32, y as f32 * 2.0, 0.0, 1.0]
    });
    let c = bilinear_clamp(&img, 3.0, 2.0);
    assert_eq!(c[0], 2.5);
    assert_eq!(c[1], 3.0);
}
#[test]
fn bilinear_clamps_to_edge() {
    let img = img_fn(Win::new(0, 0, 4, 4), |x, _| [x as f32, 0.0, 0.0, 1.0]);
    assert_eq!(bilinear_clamp(&img, -3.0, 0.5)[0], 0.0);
    assert_eq!(bilinear_clamp(&img, 32.0, 0.5)[0], 3.0);
}
#[test]
fn sample_position_subtracts_window_and_adds_half() {
    let img = const_img(Win::new(3, 5, 8, 8), [1.0; 4]);
    let s = src_of(img);
    let q = sample_position(&Camera::identity(), &s, 10, 10).unwrap();
    assert_eq!(q, (7.5, 5.5));
}
