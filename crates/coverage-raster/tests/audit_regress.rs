//! Regression tests for two defects found by an independent audit (exact rational oracle).

mod common;
use common::*;
use coverage_raster::*;

fn q(depth: Depth, rounding: Rounding) -> Quantise {
    Quantise {
        depth,
        scale: Scale::Custom(1.0),
        rounding,
    }
}

#[test]
fn half_up_does_not_round_the_value_just_below_one_half_up() {
    // 0.49999999999999994 + 0.5 rounds to 1.0 in double precision, but the value is below one half.
    let v = 0.49999999999999994_f64;
    assert_eq!(code(v, &q(Depth::Bits8, Rounding::NearestHalfUp)), 0);
    assert_eq!(code(v, &q(Depth::Bits16, Rounding::NearestHalfUp)), 0);
    assert_eq!(code(v, &q(Depth::Bits8, Rounding::NearestEven)), 0);
    assert_eq!(code(0.5, &q(Depth::Bits8, Rounding::NearestHalfUp)), 1);
    assert_eq!(code(1.0, &q(Depth::Bits8, Rounding::NearestHalfUp)), 1);
    assert_eq!(code(0.0, &q(Depth::Bits8, Rounding::NearestHalfUp)), 0);
}

#[test]
fn half_up_just_below_a_tie_after_a_power_of_two_scale() {
    let v = 0.0019531249999999998_f64; // the double below 0.5 / 256
    let qq = Quantise {
        depth: Depth::Bits8,
        scale: Scale::PowerOfTwo,
        rounding: Rounding::NearestHalfUp,
    };
    assert_eq!(code(v, &qq), 0);
}

fn offs(o: Vec<(f64, f64)>) -> Config {
    cfg(Method::Samples(Pattern::Offsets(o)))
}

#[test]
fn offset_one_sees_an_edge_that_starts_on_the_next_row_line() {
    // The square covers y 1 to 2. A sample at y offset 1.0 of row 0 lies at y = 1, on the top edge,
    // which the top left rule counts as inside; a sample of row 1 at y = 2 is on the bottom edge.
    let c = offs(vec![(0.5, 1.0)]);
    let got = one(rect(0.0, 1.0, 2.0, 2.0), 2, 2, &c);
    assert_rows(&got, &[&[1.0, 1.0], &[0.0, 0.0]]);
}

#[test]
fn offset_one_sees_an_edge_that_starts_on_the_frame_bottom_line() {
    // The square covers y 2 to 3, below the 2 row frame; row 1's sample at y = 2 is on its top edge.
    let c = offs(vec![(0.5, 1.0)]);
    let got = one(rect(0.0, 2.0, 2.0, 3.0), 2, 2, &c);
    assert_rows(&got, &[&[0.0, 0.0], &[1.0, 1.0]]);
}

// Gaps found by own mutants: the tests below kill them.

#[test]
fn ceil_of_an_exact_integer_stays_put() {
    let qq = Quantise {
        depth: Depth::Bits8,
        scale: Scale::Custom(1.0),
        rounding: Rounding::Ceil,
    };
    assert_eq!(code(0.0, &qq), 0);
    assert_eq!(code(1.0, &qq), 1);
}

#[test]
fn crossing_edges_inside_one_pixel_row_split_the_row() {
    // A bow tie whose edges cross at (1, 0.5): each triangle has area 0.5 and lies in one pixel.
    let bow = Polygon::from_ring(vec![(0.0, 0.0), (2.0, 1.0), (2.0, 0.0), (0.0, 1.0)]);
    for fill in [FillRule::NonZero, FillRule::EvenOdd] {
        let mut c = exact();
        c.fill = fill;
        assert_rows(&one(bow.clone(), 2, 1, &c), &[&[0.5, 0.5]]);
    }
}

#[test]
fn an_edge_that_ended_earlier_in_the_row_is_not_active_later() {
    // Triangle (0,0), (2,0.5), (0,1): area 1, split 0.75 and 0.25 between the two pixels.
    let t = tri((0.0, 0.0), (2.0, 0.5), (0.0, 1.0));
    assert_rows(&one(t, 2, 1, &exact()), &[&[0.75, 0.25]]);
}

#[test]
fn an_x_offset_of_one_is_accepted() {
    let c = offs(vec![(1.0, 0.5)]);
    let got = one(rect(0.0, 0.0, 2.0, 1.0), 2, 1, &c);
    // x = 1 is on the shared pixel line: inside for pixel 0 (interior), x = 2 is on the right edge.
    assert_rows(&got, &[&[1.0, 0.0]]);
}
