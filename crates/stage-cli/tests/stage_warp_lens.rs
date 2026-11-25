//! `warp-lens` on the built program: its manifest entry, its arguments, every refusal, and what
//! it writes for small made up frames, against values worked out by hand and against the
//! `lens-model` and `resample` crates called directly.

mod stage_warp_common;

use lens_model::{Config, Direction, InverseMethod, Model, ModelMaps, Normalisation};
use stage_warp_common::*;

const STAGE: &str = "warp-lens";

/// A 4 by 4 frame that holds 1 to 16 row by row.
fn small() -> Vec<u8> {
    grey(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
}

const SMALL: (u32, u32, u32) = (4, 4, 8);

fn run_small(params: &[(&str, &str)]) -> Vec<u8> {
    let all: Vec<(&str, &str)> = [("filter", "nearest")]
        .iter()
        .chain(params.iter())
        .copied()
        .collect();
    let run = apply(STAGE, SMALL, &all, &[], &small());
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{params:?}");
    run.out
}

/// The inner four pixels of [`small`] inside a ring of zero pixels.
fn ring() -> Vec<u8> {
    grey_or_zero(&[0, 0, 0, 0, 0, 6, 7, 0, 0, 10, 11, 0, 0, 0, 0, 0])
}

#[test]
fn the_manifest_entry_declares_the_stage() {
    check_the_entry(
        STAGE,
        "Lens distortion warp",
        &[
            "direction",
            "model",
            "k1",
            "k2",
            "k3",
            "p1",
            "p2",
            "lambda",
            "squeeze_x",
            "squeeze_y",
            "normalisation",
            "focal_length",
            "centre_x",
            "centre_y",
            "pixel_aspect",
            "maps",
            "inverse_method",
            "inverse_iterations",
        ],
    );
    let entry = manifest_entry(STAGE);
    assert!(entry.contains("{\"id\":\"direction\",\"name\":\"Direction\",\"kind\":\"popup\",\"default\":\"apply\",\"choices\":[\"apply\",\"remove\"],\"keyed\":false,"));
    assert!(entry.contains("{\"id\":\"k1\",\"name\":\"k1\",\"kind\":\"float\",\"default\":0,\"min\":-100,\"max\":100,\"keyed\":true,"));
    assert!(entry.contains("{\"id\":\"centre_x\",\"name\":\"Centre x\",\"kind\":\"float\",\"default\":0.5,\"min\":-100,\"max\":100,\"keyed\":true,"));
    assert!(entry.contains("{\"id\":\"inverse_iterations\",\"name\":\"Inverse iterations\",\"kind\":\"int\",\"default\":8,\"min\":0,\"max\":1024,\"keyed\":false,"));
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
    // a size whose half is not a power of two: the integer depths still return the input
    for depth in [8, 16] {
        let size = (7, 5, depth);
        let input = clip(size, 2, 4);
        let run = apply(STAGE, size, &[], &[], &input);
        assert!(run.out == input, "depth {depth}, 7 by 5");
    }
}

#[test]
fn a_radial_term_pushes_the_outer_pixels_off_the_frame() {
    // With direction remove and k1 = 1 the stage reads the source at p * (1 + |p|^2), p
    // measured from the centre in half widths. The four inner pixel centres (0.25, 0.25) move
    // to 0.28 and stay inside their pixel. Every other centre has a coordinate of 0.75 and a
    // factor of at least 1.625, which puts it beyond 1, outside the frame, where the border
    // is zero.
    let remove = [("direction", "remove"), ("k1", "1"), ("border", "zero")];
    assert_eq!(run_small(&remove), ring());
    // the same formula read the other way round: applying a model that maps distorted to
    // undistorted positions
    let apply_back = [
        ("maps", "distorted-to-undistorted"),
        ("k1", "1"),
        ("border", "zero"),
    ];
    assert_eq!(run_small(&apply_back), ring());
}

#[test]
fn the_division_model_pulls_the_inner_pixels_outward() {
    // With direction remove, the division model at lambda = 1 reads the source at p / (1 + |p|^2).
    // Every pixel centre of a quadrant lands inside the inner pixel of that quadrant:
    // (0.25, 0.25) to 0.22, (0.75, 0.25) to (0.46, 0.15), (0.75, 0.75) to 0.35.
    let division = [
        ("direction", "remove"),
        ("model", "division"),
        ("lambda", "1"),
    ];
    assert_eq!(
        run_small(&division),
        grey(&[6, 6, 7, 7, 6, 6, 7, 7, 10, 10, 11, 11, 10, 10, 11, 11])
    );
}

#[test]
fn a_position_the_inverse_does_not_reach_is_zero() {
    // Applying the division model at lambda = 1 with the closed form inverse: the square root
    // of 1 - 4 |q|^2 exists for the inner centres only (|q|^2 = 0.125), which stay inside
    // their pixel (0.25 to 0.29). The other centres have |q|^2 of 0.625 or more and no
    // source position. The border is clamp, so the zero pixels come from the map alone.
    let closed = [
        ("model", "division"),
        ("lambda", "1"),
        ("inverse_method", "closed-form"),
    ];
    assert_eq!(run_small(&closed), ring());
}

#[test]
fn the_canvas_is_centred_on_the_frame() {
    // a 6 by 6 canvas holds the 4 by 4 frame inside a margin of one pixel
    let wide = run_small(&[("width", "6"), ("height", "6"), ("border", "zero")]);
    let mut want = vec![0u8; 36];
    for y in 0..4 {
        for x in 0..4 {
            want[(y + 1) * 6 + x + 1] = (y * 4 + x + 1) as u8;
        }
    }
    assert_eq!(wide, grey_or_zero(&want));
    // a 2 by 2 canvas shows the inner four pixels
    let crop = run_small(&[("width", "2"), ("height", "2")]);
    assert_eq!(crop, grey(&[6, 7, 10, 11]));
}

/// One case of the comparison with the crates: the parameters, the lens configuration they
/// name for a 7 by 5 frame, the direction, the output size and the resampler configuration.
struct Case {
    params: Vec<(&'static str, &'static str)>,
    lens: Config,
    direction: Direction,
    out: (u32, u32),
    sampler: resample::Config,
    corner: bool,
}

fn cases() -> Vec<Case> {
    let frame = Config::new(7.0, 5.0, Model::identity());
    vec![
        Case {
            params: vec![
                ("k1", "0.1"),
                ("k2", "-0.02"),
                ("k3", "0.004"),
                ("p1", "0.01"),
                ("p2", "-0.005"),
            ],
            lens: Config {
                model: Model::Radial {
                    k: [0.1, -0.02, 0.004],
                    p: [0.01, -0.005],
                },
                ..frame
            },
            direction: Direction::ToDistorted,
            out: (7, 5),
            sampler: resample::Config::default(),
            corner: false,
        },
        Case {
            params: vec![
                ("model", "division"),
                ("lambda", "0.2"),
                ("inverse_method", "closed-form"),
                ("normalisation", "focal-length"),
                ("focal_length", "6"),
            ],
            lens: Config {
                model: Model::Division { lambda: 0.2 },
                inverse_method: InverseMethod::ClosedForm,
                normalisation: Normalisation::Focal(6.0),
                ..frame
            },
            direction: Direction::ToDistorted,
            out: (7, 5),
            sampler: resample::Config::default(),
            corner: false,
        },
        Case {
            params: vec![
                ("direction", "remove"),
                ("model", "squeezed"),
                ("k1", "0.2"),
                ("squeeze_x", "1.5"),
                ("squeeze_y", "0.75"),
                ("normalisation", "half-diagonal"),
                ("centre_x", "0.4"),
                ("centre_y", "0.6"),
                ("pixel_aspect", "1.5"),
                ("maps", "distorted-to-undistorted"),
                ("inverse_method", "fixed-point"),
                ("inverse_iterations", "5"),
                ("width", "9"),
                ("height", "7"),
                ("filter", "keys"),
                ("border", "zero"),
            ],
            lens: Config {
                model: Model::Squeezed {
                    k: [0.2, 0.0, 0.0],
                    squeeze: [1.5, 0.75],
                },
                normalisation: Normalisation::HalfDiagonal,
                centre: [7.0 * 0.4, 5.0 * 0.6],
                pixel_aspect: 1.5,
                maps: ModelMaps::DistortedToUndistorted,
                inverse_method: InverseMethod::FixedPoint,
                inverse_iterations: 5,
                ..frame
            },
            direction: Direction::ToUndistorted,
            out: (9, 7),
            sampler: resample::Config {
                filter: resample::Filter::Keys { a: -0.5 },
                border: resample::Border::Zero,
                ..resample::Config::default()
            },
            corner: false,
        },
        Case {
            params: vec![
                ("direction", "remove"),
                ("k1", "-0.15"),
                ("normalisation", "half-height"),
                ("center", "corner"),
                ("height", "6"),
            ],
            lens: Config {
                model: Model::Radial {
                    k: [-0.15, 0.0, 0.0],
                    p: [0.0, 0.0],
                },
                normalisation: Normalisation::HalfHeight,
                ..frame
            },
            direction: Direction::ToUndistorted,
            out: (7, 6),
            sampler: resample::Config {
                center: resample::PixelCenter::Corner,
                ..resample::Config::default()
            },
            corner: true,
        },
    ]
}

#[test]
fn every_model_equals_the_crates_called_directly() {
    for (n, case) in cases().iter().enumerate() {
        for depth in [8, 16, 32] {
            let size = (7, 5, depth);
            let input = clip(size, 2, 5);
            let shift = if case.corner { 0.5 } else { 0.0 };
            let margin_x = (f64::from(case.out.0) - 7.0) / 2.0;
            let margin_y = (f64::from(case.out.1) - 5.0) / 2.0;
            let want = reference(&input, size, case.out, &case.sampler, |_, x, y| {
                let read = case.lens.source_map(case.direction);
                let q = read([x + shift - margin_x, y + shift - margin_y])?;
                Some((q[0] - shift, q[1] - shift))
            });
            let run = apply(STAGE, size, &case.params, &[], &input);
            assert_eq!((run.code, run.err.as_str()), (0, ""), "case {n}");
            assert!(run.out == want, "case {n}, depth {depth}");
            assert!(
                run.out != input,
                "case {n}, depth {depth}: the case moves pixels"
            );
        }
    }
}

#[test]
fn a_key_file_gives_coefficients_per_frame() {
    let frame = small();
    let keys = Scratch::new("lens-moves", "keys-1\nkey k1 0 0 hold\nkey k1 1 1 hold\n");
    let layer = keys.layer("keys");
    let run = apply(
        STAGE,
        SMALL,
        &[
            ("filter", "nearest"),
            ("direction", "remove"),
            ("border", "zero"),
        ],
        &["--layer", layer.as_str()],
        &[frame.clone(), frame.clone(), frame.clone()].concat(),
    );
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert_eq!(run.out, [frame, ring(), ring()].concat());
}

#[test]
fn a_closed_form_inverse_of_a_model_without_one_is_refused() {
    let frame = small();
    for model in ["radial", "squeezed"] {
        let params = [("inverse_method", "closed-form"), ("model", model)];
        for extra in [&[][..], &["--describe"][..]] {
            for input in [&frame[..], &b""[..]] {
                let run = apply(STAGE, SMALL, &params, extra, input);
                assert_eq!(run.code, 3, "{model}");
                assert_eq!(
                    run.err,
                    format!("error: unsupported: inverse_method closed-form needs model division: {model} has no closed form inverse\n")
                );
                assert!(run.out.is_empty());
            }
        }
        // where the stage reads through the formula itself, the inverse method is not used
        for forward in [
            [
                ("direction", "remove"),
                ("maps", "undistorted-to-distorted"),
            ],
            [("direction", "apply"), ("maps", "distorted-to-undistorted")],
        ] {
            let mut all = params.to_vec();
            all.extend(forward);
            let run = apply(STAGE, SMALL, &all, &[], &frame);
            assert_eq!((run.code, run.err.as_str()), (0, ""), "{model} {forward:?}");
            assert_eq!(run.out, frame);
        }
    }
    let division = [("inverse_method", "closed-form"), ("model", "division")];
    let run = apply(STAGE, SMALL, &division, &[], &frame);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert_eq!(run.out, frame);
}

#[test]
fn every_parameter_refuses_a_bad_value() {
    check_refusals(
        STAGE,
        &[
            (
                &[("direction", "undo")],
                3,
                "`undo` is not one of apply, remove",
            ),
            (
                &[("model", "fisheye")],
                3,
                "`fisheye` is not one of radial, division, squeezed",
            ),
            (
                &[("k1", "101")],
                3,
                "bad value for `k1`: 101 is outside -100 to 100",
            ),
            (
                &[("k2", "x")],
                3,
                "bad value for `k2`: cannot parse `x` as a number",
            ),
            (&[("k3", "-101")], 3, "bad value for `k3`"),
            (&[("p1", "1e3")], 3, "bad value for `p1`"),
            (&[("p2", "inf")], 3, "bad value for `p2`"),
            (&[("lambda", "-101")], 3, "bad value for `lambda`"),
            (
                &[("squeeze_x", "0")],
                3,
                "bad value for `squeeze_x`: 0 is outside 0.01 to 100",
            ),
            (&[("squeeze_y", "1000")], 3, "bad value for `squeeze_y`"),
            (
                &[("normalisation", "radius")],
                3,
                "`radius` is not one of half-width, half-height, half-diagonal, focal-length",
            ),
            (&[("focal_length", "0")], 3, "bad value for `focal_length`"),
            (&[("centre_x", "101")], 3, "bad value for `centre_x`"),
            (&[("centre_y", "-101")], 3, "bad value for `centre_y`"),
            (
                &[("pixel_aspect", "0")],
                3,
                "bad value for `pixel_aspect`: 0 is outside 0.01 to 100",
            ),
            (
                &[("maps", "forward")],
                3,
                "`forward` is not one of undistorted-to-distorted, distorted-to-undistorted",
            ),
            (
                &[("inverse_method", "bisection")],
                3,
                "`bisection` is not one of fixed-point, newton, closed-form",
            ),
            (
                &[("inverse_iterations", "-1")],
                3,
                "bad value for `inverse_iterations`: -1 is outside 0 to 1024",
            ),
            (
                &[("inverse_iterations", "1025")],
                3,
                "bad value for `inverse_iterations`",
            ),
            (
                &[("inverse_iterations", "1.5")],
                3,
                "cannot parse `1.5` as a whole number",
            ),
            (
                &[("k1", "0.1"), ("k1", "0.2")],
                2,
                "parameter `k1` given more than once",
            ),
            (&[("m00", "1")], 3, "warp-lens has no parameter `m00`"),
        ],
    );
}

#[test]
fn the_shared_refusals_hold() {
    check_the_shared_refusals(STAGE, ("k1", "101"), "squeeze_x");
}

#[test]
fn frames_are_written_as_they_are_read() {
    check_it_streams(STAGE, &[("k1", "0.05"), ("p1", "0.01")], (64, 48));
}
