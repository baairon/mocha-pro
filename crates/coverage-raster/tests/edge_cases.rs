//! Errors, large coordinates, the configuration space and determinism.

mod common;
use common::*;
use coverage_raster::*;
use std::collections::HashSet;

fn err(polys: &[Polygon], c: &Config) -> Error {
    rasterise(polys, &Target::new(4, 4), c).unwrap_err()
}

#[test]
fn nan_coordinate_is_a_typed_error() {
    let p = Polygon::from_ring(vec![(0.0, 0.0), (f64::NAN, 1.0), (1.0, 1.0)]);
    assert_eq!(
        err(&[rect(0.0, 0.0, 1.0, 1.0), p], &exact()),
        Error::NonFinite {
            polygon: 1,
            ring: 0,
            point: 1
        }
    );
}

#[test]
fn infinite_coordinate_is_a_typed_error() {
    let p = Polygon(vec![
        vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],
        vec![(0.0, 0.0), (1.0, f64::INFINITY), (1.0, 1.0)],
    ]);
    assert_eq!(
        err(&[p], &exact()),
        Error::NonFinite {
            polygon: 0,
            ring: 1,
            point: 1
        }
    );
}

#[test]
fn coordinate_beyond_the_limit_is_a_typed_error() {
    let p = rect(0.0, 0.0, MAX_COORD * 2.0, 1.0);
    assert_eq!(
        err(&[p], &exact()),
        Error::OutOfRange {
            polygon: 0,
            ring: 0,
            point: 1
        }
    );
}

#[test]
fn errors_are_the_same_for_every_method() {
    let p = Polygon::from_ring(vec![(f64::NAN, 0.0)]);
    for m in [Method::Point, Method::Samples(Pattern::Grid(2))] {
        assert!(matches!(
            err(std::slice::from_ref(&p), &cfg(m)),
            Error::NonFinite { .. }
        ));
    }
}

#[test]
fn bad_configurations_are_refused() {
    let r = rect(0.0, 0.0, 1.0, 1.0);
    let mut c = exact();
    c.centre = f64::NAN;
    assert!(matches!(
        err(std::slice::from_ref(&r), &c),
        Error::BadConfig(_)
    ));
    c.centre = 2.0;
    assert!(matches!(
        err(std::slice::from_ref(&r), &c),
        Error::BadConfig(_)
    ));
    let mut c = exact();
    c.snap_bits = Some(31);
    assert!(matches!(
        err(std::slice::from_ref(&r), &c),
        Error::BadConfig(_)
    ));
    for m in [
        Method::Samples(Pattern::Grid(0)),
        Method::Samples(Pattern::Grid(251)),
        Method::Samples(Pattern::Rotated(0)),
        Method::Samples(Pattern::Rotated(4097)),
        Method::Samples(Pattern::Offsets(vec![])),
        Method::Samples(Pattern::Offsets(vec![(1.5, 0.5)])),
        Method::Samples(Pattern::Offsets(vec![(0.5, f64::NAN)])),
        Method::Samples(Pattern::Offsets(vec![(-0.1, 0.5)])),
    ] {
        assert!(matches!(
            err(std::slice::from_ref(&r), &cfg(m)),
            Error::BadConfig(_)
        ));
    }
    let mut c = exact();
    c.quantise = Some(Quantise {
        depth: Depth::Bits8,
        scale: Scale::Custom(0.0),
        rounding: Rounding::Floor,
    });
    assert!(matches!(err(&[r], &c), Error::BadConfig(_)));
}

#[test]
fn bad_targets_are_refused() {
    let r = rect(0.0, 0.0, 1.0, 1.0);
    for t in [
        Target::new(MAX_SIDE + 1, 1),
        Target::new(1, MAX_SIDE + 1),
        Target::new(MAX_SIDE, MAX_SIDE),
        Target {
            x: i64::MAX,
            y: 0,
            width: 1,
            height: 1,
        },
    ] {
        assert_eq!(
            rasterise(std::slice::from_ref(&r), &t, &exact()),
            Err(Error::BadTarget)
        );
    }
}

#[test]
fn errors_display_as_text() {
    assert!(Error::BadTarget.to_string().contains("target"));
    assert!(Error::BadConfig("x").to_string().contains('x'));
    assert!(Error::NonFinite {
        polygon: 1,
        ring: 2,
        point: 3
    }
    .to_string()
    .contains("polygon 1"));
    assert!(Error::OutOfRange {
        polygon: 0,
        ring: 0,
        point: 0
    }
    .to_string()
    .contains("range"));
}

#[test]
fn huge_rectangle_covers_everything() {
    for m in [
        Method::ExactArea,
        Method::Point,
        Method::Samples(Pattern::Grid(2)),
    ] {
        let got = one(rect(-1e12, -1e12, 1e12, 1e12), 4, 4, &cfg(m));
        assert!(got.data.iter().all(|&v| v == 1.0));
    }
}

#[test]
fn huge_triangle_edge_through_the_target() {
    // Edge from (0, 0) to (1e9, 1e9): the fill is on the side x > y.
    let p = tri((0.0, 0.0), (1e9, 0.0), (1e9, 1e9));
    let got = one(p, 3, 3, &exact());
    assert_rows(
        &got,
        &[&[0.5, 1.0, 1.0], &[0.0, 0.5, 1.0], &[0.0, 0.0, 0.5]],
    );
}

#[test]
fn huge_triangle_by_samples() {
    let p = tri((0.0, 0.0), (1e9, 0.0), (1e9, 1e9));
    let mut c = cfg(Method::Samples(Pattern::Grid(2)));
    c.fill = FillRule::EvenOdd;
    let got = one(p, 3, 3, &c);
    assert_eq!(got.at(2, 0), 1.0);
    assert_eq!(got.at(0, 2), 0.0);
    // The diagonal is a left edge of the fill, so the two samples on it are inside.
    assert_eq!(got.at(1, 1), 0.75);
}

#[test]
fn polygon_far_from_the_origin_with_a_matching_target() {
    let far = 1e15;
    let t = Target {
        x: far as i64,
        y: far as i64,
        width: 3,
        height: 2,
    };
    let got = rasterise(&[rect(far + 0.5, far, far + 2.0, far + 1.0)], &t, &exact()).unwrap();
    assert_rows(&got, &[&[0.5, 1.0, 0.0], &[0.0, 0.0, 0.0]]);
}

#[test]
fn coordinates_at_the_limit_are_accepted() {
    let p = rect(-MAX_COORD, -MAX_COORD, MAX_COORD, MAX_COORD);
    let got = one(p, 2, 2, &exact());
    assert!(got.data.iter().all(|&v| v == 1.0));
}

#[test]
fn very_thin_far_sliver_inside_the_target() {
    let got = one(rect(0.0, 0.0, 1e9, 1.0 / 1024.0), 2, 1, &exact());
    assert_eq!(got.data, vec![1.0 / 1024.0, 1.0 / 1024.0]);
}

#[test]
fn canonical_is_documented() {
    let c = Config::canonical();
    assert_eq!(c.method, Method::ExactArea);
    assert_eq!(c.fill, FillRule::NonZero);
    assert_eq!(c.centre, 0.5);
    assert!(c.y_down);
    assert_eq!(c.combine, Combine::Max);
    assert_eq!(c.precision, Precision::Double);
    assert_eq!(c.snap_bits, None);
    assert_eq!(
        c.quantise,
        Some(Quantise {
            depth: Depth::Bits8,
            scale: Scale::Full,
            rounding: Rounding::NearestHalfUp
        })
    );
    assert!(c.validate().is_ok());
}

#[test]
fn default_space_enumerates_its_stated_size_in_a_fixed_order() {
    let s = Space::default();
    let all = s.enumerate();
    assert_eq!(all.len(), s.len());
    assert_eq!(all.len(), 7 * 2 * 2 * 2 * 17 * 4 * 2);
    assert!(!s.is_empty());
    assert_eq!(all, s.enumerate());
    assert_eq!(all[0].method, Method::ExactArea);
    assert_eq!(all[0].snap_bits, None);
    assert_eq!(all[1].snap_bits, Some(8));
}

#[test]
fn every_enumerated_configuration_is_valid_and_has_a_unique_label() {
    let all = Space::default().enumerate();
    let mut seen = HashSet::new();
    for c in &all {
        assert!(c.validate().is_ok());
        assert!(seen.insert(c.label()), "duplicate label {}", c.label());
    }
}

#[test]
fn empty_space_enumerates_nothing() {
    let mut s = Space::default();
    s.fills.clear();
    assert!(s.is_empty());
    assert!(s.enumerate().is_empty());
}

#[test]
fn label_of_the_canonical_configuration() {
    assert_eq!(
        Config::canonical().label(),
        "exact/nz/c0.5/ydown/u8-full-up/max/f64/nosnap"
    );
}

#[test]
fn every_configuration_of_the_space_runs_on_a_small_shape() {
    let p = tri((0.2, 0.3), (3.6, 0.9), (1.4, 3.7));
    let t = Target::new(4, 4);
    let mut n = 0;
    for c in Space::default().enumerate().iter().step_by(37) {
        let b = rasterise_buffer(std::slice::from_ref(&p), &t, c).unwrap();
        assert_eq!(b.len(), 16);
        n += 1;
    }
    assert!(n > 99);
}

#[test]
fn repeated_runs_are_bit_identical_for_every_method() {
    let p = Polygon(vec![
        vec![(0.1, 0.2), (7.7, 0.9), (6.3, 7.8), (1.9, 6.1)],
        vec![(2.2, 2.3), (4.4, 2.1), (3.3, 5.5)],
    ]);
    for m in [
        Method::ExactArea,
        Method::Point,
        Method::Samples(Pattern::Grid(5)),
        Method::Samples(Pattern::Rotated(7)),
    ] {
        let c = cfg(m);
        let a = one(p.clone(), 8, 8, &c);
        let b = one(p.clone(), 8, 8, &c);
        let ba: Vec<u64> = a.data.iter().map(|v| v.to_bits()).collect();
        let bb: Vec<u64> = b.data.iter().map(|v| v.to_bits()).collect();
        assert_eq!(ba, bb);
    }
}

#[test]
fn pinned_bit_pattern_of_one_exact_run() {
    // A fixed checksum of the bits of one run: it must be the same on every platform.
    let p = Polygon(vec![
        vec![(0.1, 0.2), (7.7, 0.9), (6.3, 7.8), (1.9, 6.1)],
        vec![(2.2, 2.3), (4.4, 2.1), (3.3, 5.5)],
    ]);
    let a = one(p, 8, 8, &exact());
    let mut h: u64 = 1469598103934665603;
    for v in &a.data {
        h ^= v.to_bits();
        h = h.wrapping_mul(1099511628211);
    }
    assert_eq!(h, PINNED, "checksum is {h}");
}

const PINNED: u64 = 17374936429132525966;

#[test]
fn limits_are_inclusive() {
    let r = rect(0.0, 0.0, 1.0, 1.0);
    let t = Target::new(MAX_SIDE, 1);
    assert!(rasterise(std::slice::from_ref(&r), &t, &exact()).is_ok());
    let t = Target::new(1, MAX_SIDE);
    assert!(rasterise(std::slice::from_ref(&r), &t, &exact()).is_ok());
    let mut c = cfg(Method::Samples(Pattern::Grid(250)));
    assert!(rasterise(std::slice::from_ref(&r), &Target::new(1, 1), &c).is_ok());
    c = exact();
    for centre in [1.0, -1.0] {
        c.centre = centre;
        assert!(rasterise(std::slice::from_ref(&r), &Target::new(1, 1), &c).is_ok());
    }
    c = exact();
    c.snap_bits = Some(30);
    assert!(rasterise(std::slice::from_ref(&r), &Target::new(1, 1), &c).is_ok());
    let c = cfg(Method::Samples(Pattern::Rotated(4096)));
    assert!(rasterise(&[r], &Target::new(1, 1), &c).is_ok());
}
