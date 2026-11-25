//! Combination of polygons, precision and quantisation.

mod common;
use common::*;
use coverage_raster::*;

fn two(a: f64, b: f64, comb: Combine) -> f64 {
    // Two one pixel targets with coverage a and b from rectangles of height 1 and width a, b.
    let mut c = exact();
    c.combine = comb;
    let got = run(&[rect(0.0, 0.0, a, 1.0), rect(0.0, 0.0, b, 1.0)], 1, 1, &c);
    got.at(0, 0)
}

#[test]
fn combine_max() {
    assert_eq!(two(0.5, 0.25, Combine::Max), 0.5);
    assert_eq!(two(0.25, 0.5, Combine::Max), 0.5);
}

#[test]
fn combine_add_clamp() {
    assert_eq!(two(0.5, 0.25, Combine::AddClamp), 0.75);
    assert_eq!(two(0.75, 0.5, Combine::AddClamp), 1.0);
}

#[test]
fn combine_over() {
    assert_eq!(two(0.5, 0.25, Combine::Over), 0.625);
    assert_eq!(two(0.5, 0.5, Combine::Over), 0.75);
}

#[test]
fn combine_replace() {
    assert_eq!(two(0.5, 0.25, Combine::Replace), 0.25);
    // A later polygon with no coverage leaves the earlier value.
    let mut c = exact();
    c.combine = Combine::Replace;
    let got = run(
        &[rect(0.0, 0.0, 0.5, 1.0), rect(5.0, 5.0, 6.0, 6.0)],
        1,
        1,
        &c,
    );
    assert_eq!(got.at(0, 0), 0.5);
}

#[test]
fn combine_of_one_polygon_is_that_polygon() {
    for comb in [
        Combine::Max,
        Combine::AddClamp,
        Combine::Over,
        Combine::Replace,
    ] {
        assert_eq!(two(0.5, 0.0, comb), 0.5, "{comb:?}");
    }
}

#[test]
fn combine_order_matters_only_for_replace() {
    for comb in [Combine::Max, Combine::AddClamp, Combine::Over] {
        assert_eq!(two(0.5, 0.25, comb), two(0.25, 0.5, comb), "{comb:?}");
    }
    assert_ne!(
        two(0.5, 0.25, Combine::Replace),
        two(0.25, 0.5, Combine::Replace)
    );
}

#[test]
fn combine_disjoint_polygons_is_union_for_every_rule() {
    for comb in [
        Combine::Max,
        Combine::AddClamp,
        Combine::Over,
        Combine::Replace,
    ] {
        let mut c = exact();
        c.combine = comb;
        let got = run(
            &[rect(0.0, 0.0, 1.0, 1.0), rect(1.0, 0.0, 2.0, 1.0)],
            2,
            1,
            &c,
        );
        assert_eq!(got.data, vec![1.0, 1.0]);
    }
}

#[test]
fn single_precision_rounds_coverage_to_f32() {
    let mut c = exact();
    c.precision = Precision::Single;
    let got = one(rect(0.0, 0.0, 0.1, 1.0), 1, 1, &c).at(0, 0);
    assert_eq!(got, (got as f32) as f64);
    let mut d = exact();
    d.precision = Precision::Double;
    let want = one(rect(0.0, 0.0, 0.1, 1.0), 1, 1, &d).at(0, 0);
    assert!((want - 0.1).abs() < 1e-15);
    assert_ne!(want, (want as f32) as f64);
    assert_eq!(got, (want as f32) as f64);
}

fn q(depth: Depth, scale: Scale, rounding: Rounding) -> Quantise {
    Quantise {
        depth,
        scale,
        rounding,
    }
}

#[test]
fn code_half_at_eight_bits_for_each_rounding() {
    let r = |m| code(0.5, &q(Depth::Bits8, Scale::Full, m));
    assert_eq!(r(Rounding::NearestHalfUp), 128);
    assert_eq!(r(Rounding::NearestEven), 128);
    assert_eq!(r(Rounding::Floor), 127);
    assert_eq!(r(Rounding::Ceil), 128);
}

#[test]
fn code_ties_differ_between_half_up_and_even() {
    let r = |m| code(0.5, &q(Depth::Bits8, Scale::Custom(5.0), m));
    assert_eq!(r(Rounding::NearestHalfUp), 3);
    assert_eq!(r(Rounding::NearestEven), 2);
    let r = |m| code(0.7, &q(Depth::Bits8, Scale::Custom(5.0), m));
    assert_eq!(r(Rounding::NearestHalfUp), 4);
    assert_eq!(r(Rounding::NearestEven), 4);
}

#[test]
fn code_endpoints_for_each_scale() {
    for (depth, max) in [(Depth::Bits8, 255), (Depth::Bits16, 65535)] {
        for scale in [Scale::Full, Scale::PowerOfTwo] {
            let z = code(0.0, &q(depth, scale, Rounding::NearestHalfUp));
            let o = code(1.0, &q(depth, scale, Rounding::NearestHalfUp));
            assert_eq!((z, o), (0, max));
        }
    }
}

#[test]
fn code_power_of_two_scale_half_is_128() {
    assert_eq!(
        code(0.5, &q(Depth::Bits8, Scale::PowerOfTwo, Rounding::Floor)),
        128
    );
    assert_eq!(
        code(0.5, &q(Depth::Bits16, Scale::PowerOfTwo, Rounding::Floor)),
        32768
    );
}

#[test]
fn code_custom_fifteen_bit_scale() {
    let k = q(
        Depth::Bits16,
        Scale::Custom(32768.0),
        Rounding::NearestHalfUp,
    );
    assert_eq!(code(1.0, &k), 32768);
    assert_eq!(code(0.5, &k), 16384);
    assert_eq!(code(0.25, &k), 8192);
}

#[test]
fn code_clamps_to_the_depth_maximum() {
    let k = q(Depth::Bits8, Scale::Custom(999.0), Rounding::NearestHalfUp);
    assert_eq!(code(1.0, &k), 255);
    assert_eq!(code(-1.0, &k), 0);
}

#[test]
fn buffer_u8_from_a_run() {
    let mut c = Config::canonical();
    c.quantise = Some(q(Depth::Bits8, Scale::Full, Rounding::NearestHalfUp));
    let b = rasterise_buffer(&[rect(0.0, 0.0, 0.5, 1.0)], &Target::new(2, 1), &c).unwrap();
    assert_eq!(b, Buffer::U8(vec![128, 0]));
}

#[test]
fn buffer_u16_from_a_run() {
    let mut c = Config::canonical();
    c.quantise = Some(q(Depth::Bits16, Scale::Full, Rounding::Floor));
    let b = rasterise_buffer(&[rect(0.0, 0.0, 1.0, 1.0)], &Target::new(2, 1), &c).unwrap();
    assert_eq!(b, Buffer::U16(vec![65535, 0]));
}

#[test]
fn buffer_f32_from_a_run_and_from_none() {
    let mut c = Config::canonical();
    c.quantise = Some(q(Depth::Float32, Scale::Full, Rounding::Floor));
    let a = rasterise_buffer(&[rect(0.0, 0.0, 0.25, 1.0)], &Target::new(1, 1), &c).unwrap();
    c.quantise = None;
    let b = rasterise_buffer(&[rect(0.0, 0.0, 0.25, 1.0)], &Target::new(1, 1), &c).unwrap();
    assert_eq!(a, Buffer::F32(vec![0.25]));
    assert_eq!(a, b);
}

#[test]
fn buffer_accessors() {
    let b = Buffer::U8(vec![1, 2, 3]);
    assert_eq!(b.len(), 3);
    assert!(!b.is_empty());
    assert_eq!(b.get(2), 3.0);
    assert!(Buffer::F32(vec![]).is_empty());
}
