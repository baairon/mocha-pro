//! `warp-four-point` on the built program: its manifest entry, its arguments, every refusal,
//! and what it writes for small made up frames, against values worked out by hand and against
//! the `planar-xform` and `resample` crates called directly.

mod stage_warp_common;

use planar_xform::{Config, Homography, PixelSpace, Point};
use stage_warp_common::*;

const STAGE: &str = "warp-four-point";

/// A 4 by 2 frame: the first row holds 1 to 4, the second 5 to 8.
fn small() -> Vec<u8> {
    grey(&[1, 2, 3, 4, 5, 6, 7, 8])
}

const SMALL: (u32, u32, u32) = (4, 2, 8);
const NEAREST_ZERO: [(&str, &str); 2] = [("filter", "nearest"), ("border", "zero")];

fn run_small(params: &[(&str, &str)]) -> Vec<u8> {
    let all: Vec<(&str, &str)> = NEAREST_ZERO.iter().chain(params.iter()).copied().collect();
    let run = apply(STAGE, SMALL, &all, &[], &small());
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{params:?}");
    run.out
}

/// The eight corner parameters from four points.
fn corners(points: [(&'static str, &'static str); 4]) -> Vec<(&'static str, &'static str)> {
    let ids = [("x0", "y0"), ("x1", "y1"), ("x2", "y2"), ("x3", "y3")];
    ids.iter()
        .zip(points)
        .flat_map(|((x, y), (px, py))| [(*x, px), (*y, py)])
        .collect()
}

#[test]
fn the_manifest_entry_declares_the_stage() {
    check_the_entry(
        STAGE,
        "Four point warp",
        &[
            "x0",
            "y0",
            "x1",
            "y1",
            "x2",
            "y2",
            "x3",
            "y3",
            "maps",
            "convention",
            "guard",
            "divisor_min",
            "coords",
            "xform_center",
            "y_axis",
            "pixel_aspect",
            "singular_tolerance",
        ],
    );
    let entry = manifest_entry(STAGE);
    assert!(entry.contains("{\"id\":\"x1\",\"name\":\"Corner 1 x\",\"kind\":\"float\",\"default\":1,\"min\":-1000000000,\"max\":1000000000,\"keyed\":true,"));
    assert!(
        entry.contains("{\"id\":\"y1\",\"name\":\"Corner 1 y\",\"kind\":\"float\",\"default\":0,")
    );
    assert!(entry.contains(
        "{\"id\":\"maps\",\"name\":\"Maps\",\"kind\":\"popup\",\"default\":\"source-to-output\","
    ));
    assert!(entry.contains("{\"id\":\"coords\",\"name\":\"Coordinates\",\"kind\":\"popup\",\"default\":\"unit\",\"choices\":[\"pixel\",\"unit\"],"));
    assert_eq!(entry.matches("\"keyed\":true").count(), 8);
}

#[test]
fn describe_answers_the_declared_size() {
    check_describe(STAGE);
}

#[test]
fn the_defaults_return_the_input() {
    for depth in [8, 16, 32] {
        let size = (8, 4, depth);
        let input = clip(size, 3, 11);
        let run = apply(STAGE, size, &[], &[], &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""));
        assert!(run.out == input, "depth {depth}");
    }
    // a size whose reciprocal is not exact: the integer depths still return the input
    for depth in [8, 16] {
        let size = (7, 5, depth);
        let input = clip(size, 2, 4);
        let run = apply(STAGE, size, &[], &[], &input);
        assert!(run.out == input, "depth {depth}, 7 by 5");
    }
}

#[test]
fn swapped_corners_mirror_the_frame() {
    // the left corners of the source go to the right corners and the reverse
    let mirror = corners([("1", "0"), ("0", "0"), ("0", "1"), ("1", "1")]);
    assert_eq!(run_small(&mirror), grey(&[4, 3, 2, 1, 8, 7, 6, 5]));
    // the upper corners go to the lower corners and the reverse
    let flip = corners([("0", "1"), ("1", "1"), ("1", "0"), ("0", "0")]);
    assert_eq!(run_small(&flip), grey(&[5, 6, 7, 8, 1, 2, 3, 4]));
    // with v growing up the frame corners are named from the lower row, so the default
    // values, which name the upper row first, swap the rows as well
    assert_eq!(
        run_small(&[("y_axis", "up")]),
        grey(&[5, 6, 7, 8, 1, 2, 3, 4])
    );
}

#[test]
fn the_source_lands_in_the_right_half_of_a_wider_canvas() {
    // the source corners go to the right half of the unit square; on an 8 by 2 canvas that
    // half is 4 pixels wide, so the source lands there pixel for pixel
    let mut params = corners([("0.5", "0"), ("1", "0"), ("1", "1"), ("0.5", "1")]);
    params.push(("width", "8"));
    assert_eq!(
        run_small(&params),
        grey_or_zero(&[0, 0, 0, 0, 1, 2, 3, 4, 0, 0, 0, 0, 5, 6, 7, 8])
    );
}

#[test]
fn output_to_source_reads_the_source_inside_the_given_corners() {
    // the corners of the output read the right half of the source: each of its two columns
    // is shown twice
    let mut params = corners([("0.5", "0"), ("1", "0"), ("1", "1"), ("0.5", "1")]);
    params.push(("maps", "output-to-source"));
    assert_eq!(run_small(&params), grey(&[3, 3, 4, 4, 7, 7, 8, 8]));
}

#[test]
fn pixel_coordinates_take_pixel_positions() {
    // the corners of the 4 by 2 source, moved one pixel right and one row down
    let mut params = corners([("1", "1"), ("5", "1"), ("5", "3"), ("1", "3")]);
    params.push(("coords", "pixel"));
    assert_eq!(run_small(&params), grey_or_zero(&[0, 0, 0, 0, 0, 1, 2, 3]));
    // with integer pixel centres the corners of the source lie half a pixel earlier
    let mut integer = corners([
        ("0.5", "0.5"),
        ("4.5", "0.5"),
        ("4.5", "2.5"),
        ("0.5", "2.5"),
    ]);
    integer.extend([("coords", "pixel"), ("xform_center", "integer")]);
    assert_eq!(run_small(&integer), grey_or_zero(&[0, 0, 0, 0, 0, 1, 2, 3]));
    // the pixel aspect scales x on both sides of each pair, so the same pixel positions give
    // the same picture
    let mut wide = corners([("1", "1"), ("5", "1"), ("5", "3"), ("1", "3")]);
    wide.extend([("coords", "pixel"), ("pixel_aspect", "2")]);
    assert_eq!(run_small(&wide), grey_or_zero(&[0, 0, 0, 0, 0, 1, 2, 3]));
}

#[test]
fn a_quad_equals_the_crates_called_directly() {
    let given = [(0.1, 0.05), (0.9, 0.0), (1.0, 0.95), (0.0, 0.8)];
    let params = corners([("0.1", "0.05"), ("0.9", "0"), ("1", "0.95"), ("0", "0.8")]);
    let cfg = Config::default();
    let unit = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| Point::new(x, y));
    let points = given.map(|(x, y)| Point::new(x, y));
    for depth in [8, 16, 32] {
        let size = (7, 5, depth);
        let input = clip(size, 2, 6);
        // source-to-output on the input size
        let solved = Homography::from_four_points(&unit, &points, &cfg).unwrap();
        let total = |m: Homography, dst: (u32, u32)| {
            PixelSpace::new(f64::from(dst.0), f64::from(dst.1), cfg)
                .pixel_to_unit()
                .then(&m)
                .then(&PixelSpace::new(7.0, 5.0, cfg).unit_to_pixel())
        };
        let forward = total(solved.inverse().unwrap(), (7, 5));
        let want = reference(
            &input,
            size,
            (7, 5),
            &resample::Config::default(),
            |_, x, y| forward.apply(Point::new(x, y), &cfg).map(|p| (p.x, p.y)),
        );
        let run = apply(STAGE, size, &params, &[], &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""));
        assert!(run.out == want, "depth {depth}: source-to-output");
        // output-to-source on a 10 by 6 canvas with the Lanczos filter and a mirror border
        let read = total(solved, (10, 6));
        let lanczos = resample::Config {
            filter: resample::Filter::Lanczos { lobes: 2 },
            border: resample::Border::Mirror,
            ..resample::Config::default()
        };
        let want = reference(&input, size, (10, 6), &lanczos, |_, x, y| {
            read.apply(Point::new(x, y), &cfg).map(|p| (p.x, p.y))
        });
        let mut back = params.clone();
        back.extend([
            ("maps", "output-to-source"),
            ("filter", "lanczos"),
            ("lobes", "2"),
            ("border", "mirror"),
            ("width", "10"),
            ("height", "6"),
        ]);
        let run = apply(STAGE, size, &back, &[], &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""));
        assert!(run.out == want, "depth {depth}: output-to-source");
    }
}

#[test]
fn a_key_file_moves_the_corners_from_frame_to_frame() {
    let frame = small();
    let keys = Scratch::new(
        "four-moves",
        "keys-1\n\
         key x0 0 0 hold\nkey x0 1 1 hold\n\
         key x1 0 1 hold\nkey x1 1 0 hold\n\
         key x2 0 1 hold\nkey x2 1 0 hold\n\
         key x3 0 0 hold\nkey x3 1 1 hold\n",
    );
    let layer = keys.layer("keys");
    let run = apply(
        STAGE,
        SMALL,
        &NEAREST_ZERO,
        &["--layer", layer.as_str()],
        &[frame.clone(), frame.clone(), frame.clone()].concat(),
    );
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    let mirrored = grey(&[4, 3, 2, 1, 8, 7, 6, 5]);
    assert_eq!(run.out, [frame, mirrored.clone(), mirrored].concat());
}

#[test]
fn corners_without_a_transform_are_refused() {
    let frame = small();
    // two corners at one point, and four corners on one line
    for flat in [[("x1", "0"), ("y1", "0")], [("y2", "0"), ("y3", "0")]] {
        for input in [&frame[..], &b""[..]] {
            let run = apply(STAGE, SMALL, &flat, &[], input);
            assert_eq!(run.code, 3, "{flat:?}");
            assert_eq!(
                run.err,
                "error: unsupported: frame 0: the transform of the four corners is singular\n",
                "{flat:?}"
            );
            assert!(run.out.is_empty());
        }
        // the geometry does not depend on the corners
        let described = apply(STAGE, SMALL, &flat, &["--describe"], b"");
        assert_eq!(described.code, 0);
    }
    // from a key file: the frames before the refusal are written
    let keys = Scratch::new(
        "four-flat",
        "keys-1\nkey y2 0 1 hold\nkey y2 1 0 hold\nkey y3 0 1 hold\nkey y3 1 0 hold\n",
    );
    let layer = keys.layer("keys");
    let run = apply(
        STAGE,
        SMALL,
        &[],
        &["--layer", layer.as_str()],
        &[frame.clone(), frame.clone()].concat(),
    );
    assert_eq!(run.code, 3);
    assert_eq!(
        run.err,
        "error: unsupported: frame 1: the transform of the four corners is singular\n"
    );
    assert_eq!(run.out, frame);
}

#[test]
fn every_parameter_refuses_a_bad_value() {
    check_refusals(
        STAGE,
        &[
            (
                &[("x0", "1e10")],
                3,
                "bad value for `x0`: 10000000000 is outside",
            ),
            (&[("y0", "-1e10")], 3, "bad value for `y0`"),
            (
                &[("x1", "x")],
                3,
                "bad value for `x1`: cannot parse `x` as a number",
            ),
            (&[("y1", "inf")], 3, "bad value for `y1`"),
            (&[("x2", "1e10")], 3, "bad value for `x2`"),
            (&[("y2", "1e10")], 3, "bad value for `y2`"),
            (&[("x3", "1e10")], 3, "bad value for `x3`"),
            (&[("y3", "1e10")], 3, "bad value for `y3`"),
            (
                &[("maps", "inverse")],
                3,
                "`inverse` is not one of output-to-source, source-to-output",
            ),
            (
                &[("convention", "diagonal")],
                3,
                "`diagonal` is not one of column, row",
            ),
            (
                &[("guard", "none")],
                3,
                "`none` is not one of return-none, clamp, pass-through",
            ),
            (&[("divisor_min", "1e10")], 3, "bad value for `divisor_min`"),
            (&[("coords", "uv")], 3, "`uv` is not one of pixel, unit"),
            (
                &[("xform_center", "corner")],
                3,
                "`corner` is not one of integer, half",
            ),
            (&[("y_axis", "left")], 3, "`left` is not one of down, up"),
            (
                &[("pixel_aspect", "0")],
                3,
                "bad value for `pixel_aspect`: 0 is outside 0.01 to 100",
            ),
            (
                &[("singular_tolerance", "2")],
                3,
                "bad value for `singular_tolerance`",
            ),
            (
                &[("x0", "1"), ("x0", "2")],
                2,
                "parameter `x0` given more than once",
            ),
            (&[("m00", "1")], 3, "warp-four-point has no parameter `m00`"),
        ],
    );
}

#[test]
fn the_shared_refusals_hold() {
    check_the_shared_refusals(STAGE, ("x0", "2000000000"), "pixel_aspect");
}

#[test]
fn frames_are_written_as_they_are_read() {
    check_it_streams(STAGE, &[("x0", "0.1"), ("y2", "0.9")], (64, 48));
}
