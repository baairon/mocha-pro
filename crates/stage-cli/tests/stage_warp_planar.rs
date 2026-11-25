//! `warp-planar` on the built program: its manifest entry, its arguments, every refusal, and
//! what it writes for small made up frames, against values worked out by hand and against the
//! `planar-xform` and `resample` crates called directly.

mod stage_warp_common;

use planar_xform::{Config, Convention, DivisorGuard, Homography, Point};
use stage_warp_common::*;

const STAGE: &str = "warp-planar";

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

#[test]
fn the_manifest_entry_declares_the_stage() {
    check_the_entry(
        STAGE,
        "Planar warp",
        &[
            "m00",
            "m01",
            "m02",
            "m10",
            "m11",
            "m12",
            "m20",
            "m21",
            "m22",
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
    assert!(entry.contains("{\"id\":\"m00\",\"name\":\"m00\",\"kind\":\"float\",\"default\":1,\"min\":-1000000000,\"max\":1000000000,\"keyed\":true,"));
    assert!(entry.contains("{\"id\":\"m01\",\"name\":\"m01\",\"kind\":\"float\",\"default\":0,"));
    assert!(entry.contains("{\"id\":\"maps\",\"name\":\"Maps\",\"kind\":\"popup\",\"default\":\"output-to-source\",\"choices\":[\"output-to-source\",\"source-to-output\"],\"keyed\":false,"));
    assert!(entry.contains("{\"id\":\"coords\",\"name\":\"Coordinates\",\"kind\":\"popup\",\"default\":\"pixel\",\"choices\":[\"pixel\",\"unit\"],"));
    assert_eq!(entry.matches("\"keyed\":true").count(), 9);
}

#[test]
fn describe_answers_the_declared_size() {
    check_describe(STAGE);
}

#[test]
fn the_defaults_return_the_input() {
    for depth in [8, 16, 32] {
        let size = (6, 4, depth);
        let input = clip(size, 3, 11);
        let run = apply(STAGE, size, &[], &[], &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""));
        assert!(run.out == input, "depth {depth}");
    }
}

#[test]
fn a_shift_by_whole_pixels_moves_the_frame() {
    // output-to-source: output x reads source x + 1
    assert_eq!(
        run_small(&[("m02", "1")]),
        grey_or_zero(&[2, 3, 4, 0, 6, 7, 8, 0])
    );
    // the same shift with the edge pixel repeated
    let clamp = apply(
        STAGE,
        SMALL,
        &[("filter", "nearest"), ("m02", "1")],
        &[],
        &small(),
    );
    assert_eq!(clamp.out, grey(&[2, 3, 4, 4, 6, 7, 8, 8]));
    // source-to-output: the source lands one pixel right and one row down
    assert_eq!(
        run_small(&[("maps", "source-to-output"), ("m02", "1"), ("m12", "1")]),
        grey_or_zero(&[0, 0, 0, 0, 0, 1, 2, 3])
    );
}

#[test]
fn a_mirror_matrix_reverses_each_row() {
    // x' = 4 - x
    assert_eq!(
        run_small(&[("m00", "-1"), ("m02", "4")]),
        grey(&[4, 3, 2, 1, 8, 7, 6, 5])
    );
    // y' = 2 - y
    assert_eq!(
        run_small(&[("m11", "-1"), ("m12", "2")]),
        grey(&[5, 6, 7, 8, 1, 2, 3, 4])
    );
}

#[test]
fn a_half_pixel_shift_is_the_mean_of_two_pixels() {
    // bilinear, clamp: output x reads halfway between source x and x + 1
    let input = grey(&[10, 20, 30, 40]);
    let run = apply(STAGE, (4, 1, 8), &[("m02", "0.5")], &[], &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert_eq!(run.out, grey(&[15, 25, 35, 40]));
}

#[test]
fn the_row_convention_reads_the_transposed_matrix() {
    let column = run_small(&[("m02", "1"), ("m12", "1")]);
    let row = run_small(&[("convention", "row"), ("m20", "1"), ("m21", "1")]);
    assert_eq!(row, column);
    assert_eq!(column, grey_or_zero(&[6, 7, 8, 0, 0, 0, 0, 0]));
}

#[test]
fn unit_coordinates_fill_a_canvas_of_another_size() {
    // the identity on the unit square: an 8 by 4 output shows each source pixel four times
    let out = run_small(&[("coords", "unit"), ("width", "8"), ("height", "4")]);
    let top = [1, 1, 2, 2, 3, 3, 4, 4];
    let bottom = [5, 5, 6, 6, 7, 7, 8, 8];
    assert_eq!(out, grey(&[top, top, bottom, bottom].concat()));
    // v growing up with a matrix that flips v: the rows swap
    let flipped = run_small(&[("coords", "unit"), ("m11", "-1"), ("m12", "1")]);
    assert_eq!(flipped, grey(&[5, 6, 7, 8, 1, 2, 3, 4]));
    let up = run_small(&[("coords", "unit"), ("y_axis", "up")]);
    assert_eq!(up, small());
}

#[test]
fn pixel_coordinates_keep_the_scale_on_a_larger_canvas() {
    // the identity on pixels: a 6 by 3 output holds the frame at its first pixel, zero beyond
    let out = run_small(&[("width", "6"), ("height", "3")]);
    assert_eq!(
        out,
        grey_or_zero(&[1, 2, 3, 4, 0, 0, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0])
    );
}

#[test]
fn the_matrix_pixel_centre_is_a_choice_of_its_own() {
    // a doubling about the centre of pixel 0: output X reads source 2 X
    let integer = run_small(&[("xform_center", "integer"), ("m00", "2"), ("m11", "2")]);
    assert_eq!(integer, grey_or_zero(&[1, 3, 0, 0, 0, 0, 0, 0]));
    // a doubling about the corner of the frame: output centre X + 0.5 reads 2 X + 1
    let half = run_small(&[("m00", "2"), ("m11", "2")]);
    assert_eq!(half, grey_or_zero(&[6, 8, 0, 0, 0, 0, 0, 0]));
    // the resampler rule does not change the answer of the integer matrix
    let both = run_small(&[
        ("xform_center", "integer"),
        ("center", "corner"),
        ("m00", "2"),
        ("m11", "2"),
    ]);
    assert_eq!(both, integer);
}

#[test]
fn pixel_aspect_scales_x_around_the_matrix() {
    // a shift of 2 in x where a pixel is 2 wide is a shift of one pixel
    assert_eq!(
        run_small(&[("pixel_aspect", "2"), ("m02", "2")]),
        grey_or_zero(&[2, 3, 4, 0, 6, 7, 8, 0])
    );
}

#[test]
fn the_guard_decides_where_the_divisor_is_not_valid() {
    // w = x - 2 at the pixel centres 0.5, 1.5, 2.5, 3.5 is -1.5, -0.5, 0.5, 1.5. Where it is
    // valid the source position is (x / w, y / w): (5, ..) for the third pixel, outside the
    // frame, and (2.33, 0.33) or (2.33, 1) for the fourth, which is pixel 2 of the same row.
    let w = [("m20", "1"), ("m22", "-2")];
    assert_eq!(run_small(&w), grey_or_zero(&[0, 0, 0, 3, 0, 0, 0, 7]));
    // pass-through uses the undivided position for the first two pixels: the input itself
    let pass: Vec<(&str, &str)> = w
        .iter()
        .copied()
        .chain([("guard", "pass-through")])
        .collect();
    assert_eq!(run_small(&pass), grey_or_zero(&[1, 2, 0, 3, 5, 6, 0, 7]));
    // divisor_min 1 refuses w = 0.5 as well; clamp then divides by 1: pixels 0 to 2 of a row
    let clamp: Vec<(&str, &str)> = w
        .iter()
        .copied()
        .chain([("guard", "clamp"), ("divisor_min", "1")])
        .collect();
    assert_eq!(run_small(&clamp), grey_or_zero(&[1, 2, 3, 3, 5, 6, 7, 7]));
}

#[test]
fn a_projective_matrix_equals_the_crates_called_directly() {
    let params = [
        ("m00", "0.9"),
        ("m01", "0.1"),
        ("m02", "0.25"),
        ("m10", "-0.05"),
        ("m11", "1.1"),
        ("m12", "0.5"),
        ("m20", "0.01"),
        ("m21", "0.02"),
        ("m22", "1"),
    ];
    let matrix = [[0.9, 0.1, 0.25], [-0.05, 1.1, 0.5], [0.01, 0.02, 1.0]];
    for depth in [8, 16, 32] {
        let size = (7, 5, depth);
        let input = clip(size, 2, 3);
        // output-to-source, as given
        let h = Homography::from_matrix(matrix, Convention::Column);
        let cfg = Config::default();
        let want = reference(
            &input,
            size,
            (7, 5),
            &resample::Config::default(),
            |_, x, y| h.apply(Point::new(x, y), &cfg).map(|p| (p.x, p.y)),
        );
        let run = apply(STAGE, size, &params, &[], &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""));
        assert!(run.out == want, "depth {depth}: output-to-source");
        // source-to-output with the Keys cubic and a zero border, on a 9 by 4 canvas
        let inverse = h.inverse().unwrap();
        let cubic = resample::Config {
            filter: resample::Filter::Keys { a: -0.5 },
            border: resample::Border::Zero,
            ..resample::Config::default()
        };
        let want = reference(&input, size, (9, 4), &cubic, |_, x, y| {
            inverse.apply(Point::new(x, y), &cfg).map(|p| (p.x, p.y))
        });
        let mut forward = params.to_vec();
        forward.extend([
            ("maps", "source-to-output"),
            ("filter", "keys"),
            ("border", "zero"),
            ("width", "9"),
            ("height", "4"),
        ]);
        let run = apply(STAGE, size, &forward, &[], &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""));
        assert!(run.out == want, "depth {depth}: source-to-output");
        // the clamp guard with a minimum the divisor does not reach everywhere
        let guarded = Config {
            guard: DivisorGuard::Clamp(1.1),
            divisor_min: 1.1,
            ..Config::default()
        };
        let want = reference(
            &input,
            size,
            (7, 5),
            &resample::Config::default(),
            |_, x, y| h.apply(Point::new(x, y), &guarded).map(|p| (p.x, p.y)),
        );
        let mut clamp = params.to_vec();
        clamp.extend([("guard", "clamp"), ("divisor_min", "1.1")]);
        let run = apply(STAGE, size, &clamp, &[], &input);
        assert!(run.out == want, "depth {depth}: clamp");
    }
}

#[test]
fn a_key_file_gives_a_matrix_per_frame() {
    let frame = small();
    let input = [frame.clone(), frame.clone(), frame.clone(), frame.clone()].concat();
    let keys = Scratch::new(
        "planar-moves",
        "keys-1\nkey m02 0 0 linear\nkey m02 2 2 hold\nkey m12 0 0 hold\nkey m12 3 1 hold\n",
    );
    let layer = keys.layer("keys");
    let run = apply(
        STAGE,
        SMALL,
        &NEAREST_ZERO,
        &["--layer", layer.as_str(), "--frames", "4"],
        &input,
    );
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    let want = [
        small(),
        grey_or_zero(&[2, 3, 4, 0, 6, 7, 8, 0]),
        grey_or_zero(&[3, 4, 0, 0, 7, 8, 0, 0]),
        grey_or_zero(&[7, 8, 0, 0, 0, 0, 0, 0]),
    ]
    .concat();
    assert_eq!(run.out, want);
    // a start time: frame 0 of the stream is time 2 of the file
    let late = Scratch::new(
        "planar-start",
        "keys-1\nstart 2\nkey m02 0 0 linear\nkey m02 2 2 hold\n",
    );
    let layer = late.layer("keys");
    let run = apply(
        STAGE,
        SMALL,
        &NEAREST_ZERO,
        &["--layer", layer.as_str()],
        &frame,
    );
    assert_eq!(run.out, grey_or_zero(&[3, 4, 0, 0, 7, 8, 0, 0]));
}

#[test]
fn a_matrix_without_an_inverse_is_refused() {
    let frame = small();
    let flat = [("maps", "source-to-output"), ("m00", "0")];
    // known before the first frame: no output, also on an empty input
    for input in [&frame[..], &b""[..]] {
        let run = apply(STAGE, SMALL, &flat, &[], input);
        assert_eq!(run.code, 3);
        assert_eq!(
            run.err,
            "error: unsupported: frame 0: the matrix is singular\n"
        );
        assert!(run.out.is_empty());
    }
    // as output-to-source the same matrix needs no inverse
    let direct = apply(STAGE, SMALL, &[("m00", "0")], &[], &frame);
    assert_eq!(direct.code, 0, "{}", direct.err);
    // the geometry does not depend on the matrix
    let described = apply(STAGE, SMALL, &flat, &["--describe"], b"");
    assert_eq!(described.code, 0);
    // from a key file: the frames before the refusal are written
    let keys = Scratch::new(
        "planar-flat",
        "keys-1\nkey m00 0 1 hold\nkey m00 2 0 hold\n",
    );
    let layer = keys.layer("keys");
    let input = [frame.clone(), frame.clone(), frame.clone()].concat();
    let run = apply(
        STAGE,
        SMALL,
        &[("maps", "source-to-output")],
        &["--layer", layer.as_str()],
        &input,
    );
    assert_eq!(run.code, 3);
    assert_eq!(
        run.err,
        "error: unsupported: frame 2: the matrix is singular\n"
    );
    assert_eq!(run.out, [frame.clone(), frame].concat());
}

#[test]
fn every_parameter_refuses_a_bad_value() {
    check_refusals(
        STAGE,
        &[
            (
                &[("m00", "1e10")],
                3,
                "bad value for `m00`: 10000000000 is outside",
            ),
            (
                &[("m11", "x")],
                3,
                "bad value for `m11`: cannot parse `x` as a number",
            ),
            (&[("m22", "-1e10")], 3, "bad value for `m22`"),
            (&[("m21", "inf")], 3, "bad value for `m21`"),
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
                &[("m02", "1"), ("m02", "2")],
                2,
                "parameter `m02` given more than once",
            ),
        ],
    );
}

#[test]
fn the_shared_refusals_hold() {
    check_the_shared_refusals(STAGE, ("m00", "2000000000"), "pixel_aspect");
}

#[test]
fn frames_are_written_as_they_are_read() {
    check_it_streams(STAGE, &[("m02", "1.5"), ("m10", "0.1")], (64, 48));
}
