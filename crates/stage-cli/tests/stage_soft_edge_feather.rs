//! `soft-edge-feather` on the built program: its manifest entry, its arguments, every refusal,
//! and what it writes for made up frames, against values worked out by hand and against the
//! `soft-edge` crate called directly.
//!
//! The hand worked frame is 5 by 1 with the alpha samples 0, 0, 255, 255, 255. At the default
//! threshold 0.5 the last three pixels are inside. With the default edge offset 0.5 the signed
//! distances are -1.5, -0.5, 0.5, 1.5, 2.5.

mod image_support;

use image_support::{
    alpha8, frames, manifest_entry, param_ids, params, planar32, reader_leaves, refused, rgba16,
    run, with_alpha8, Scratch,
};
use soft_edge::{Config, Direction, Profile};
use stage_cli::pixels;
use stage_cli::registry::GENERIC_CLAIM;

const STAGE: &str = "soft-edge-feather";
const SIZE: [&str; 3] = ["5", "1", "8"];

/// The alpha samples of the hand worked frame.
const STEP: [u8; 5] = [0, 0, 255, 255, 255];

fn feathered(words: &[&str], alpha: &[u8]) -> Vec<u8> {
    let out = frames(STAGE, SIZE, &params(words), &with_alpha8(alpha));
    assert_eq!(
        out.as_chunks::<4>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2]])
            .collect::<Vec<_>>(),
        vec![[10, 20, 30]; alpha.len()],
        "colour passes"
    );
    alpha8(&out)
}

// -- the manifest entry and the describe answer ----------------------------------------------

#[test]
fn the_stage_is_listed_as_generic_with_its_parameters_and_its_layer() {
    let entry = manifest_entry(STAGE);
    assert!(entry.starts_with(
        "{\"id\":\"soft-edge-feather\",\"kind\":\"frame\",\"spool\":false,\"geometry\":\"preserve\",\"depths\":[8,16,32],\"params\":["
    ));
    assert_eq!(
        param_ids(&entry),
        [
            "width",
            "profile",
            "direction",
            "threshold",
            "edge_offset",
            "border"
        ]
    );
    assert!(entry.contains(
        "{\"id\":\"width\",\"name\":\"Width\",\"kind\":\"float\",\"default\":0,\"min\":0,\"max\":4096,\"keyed\":true,"
    ));
    assert!(entry.contains(
        "{\"id\":\"direction\",\"name\":\"Direction\",\"kind\":\"popup\",\"default\":\"centred\",\"choices\":[\"inward\",\"outward\",\"centred\"],\"keyed\":false,"
    ));
    assert!(entry.contains(
        "{\"id\":\"threshold\",\"name\":\"Threshold\",\"kind\":\"float\",\"default\":0.5,\"min\":0,\"max\":1,\"keyed\":false,"
    ));
    assert!(entry.contains(
        "{\"id\":\"edge_offset\",\"name\":\"Edge offset\",\"kind\":\"float\",\"default\":0.5,\"min\":-4096,\"max\":4096,\"keyed\":false,"
    ));
    assert_eq!(entry.matches("\"keyed\":true").count(), 1);
    assert!(entry.contains("\"layers\":[{\"id\":\"keys\",\"optional\":true,\"form\":\"keys-1\","));
    assert!(entry.contains("\"name\":\"Soft edge feather\""));
    assert!(entry.contains(&format!("{GENERIC_CLAIM}\",\"family\":\"generic\"}}")));
}

#[test]
fn describe_answers_the_input_geometry_and_reads_nothing() {
    for depth in ["8", "16", "32"] {
        let answer = run(
            STAGE,
            ["7", "4", depth],
            &["--describe", "--param", "width=3"],
            b"",
        );
        assert_eq!((answer.code, answer.err.as_str()), (0, ""));
        assert_eq!(
            String::from_utf8(answer.out).unwrap(),
            format!(
                "{{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"soft-edge-feather\",\"width\":7,\"height\":4,\"depth\":{depth},\"fps\":\"24/1\",\"frames\":null}}\n"
            )
        );
    }
}

// -- what the stage computes -----------------------------------------------------------------

#[test]
fn a_width_of_zero_writes_the_frame_as_it_was_read() {
    let soft = with_alpha8(&[0, 60, 130, 200, 255]);
    assert_eq!(frames(STAGE, SIZE, &[], &soft), soft);
    assert_eq!(frames(STAGE, SIZE, &params(&["width=0"]), &soft), soft);
    // at depth 32 too, bit for bit, whatever the samples are
    let odd = planar32(&[
        [2.5, -0.0, f32::NAN, 0.3],
        [-1.0, 1.0e-30, 7.0, f32::INFINITY],
    ]);
    assert_eq!(frames(STAGE, ["2", "1", "32"], &[], &odd), odd);
}

#[test]
fn a_centred_linear_ramp_of_width_two() {
    // lo = -1, so t = (s + 1) / 2 = -0.25, 0.25, 0.75, 1.25, 1.75, clamped to 0 and 1.
    // 0.25 * 255 = 63.75, written as 64; 0.75 * 255 = 191.25, written as 191.
    assert_eq!(feathered(&["width=2"], &STEP), [0, 64, 191, 255, 255]);
    assert_eq!(
        feathered(&["width=2", "direction=centred", "profile=linear"], &STEP),
        [0, 64, 191, 255, 255]
    );
}

#[test]
fn the_ramp_lies_where_the_direction_says() {
    // inward: lo = 0, t = s / 2: the ramp starts at the contour and runs inside
    assert_eq!(
        feathered(&["width=2", "direction=inward"], &STEP),
        [0, 0, 64, 191, 255]
    );
    // outward: lo = -2, t = (s + 2) / 2: the ramp ends at the contour
    assert_eq!(
        feathered(&["width=2", "direction=outward"], &STEP),
        [64, 191, 255, 255, 255]
    );
}

#[test]
fn the_profile_shapes_the_ramp() {
    // smoothstep t * t * (3 - 2 t): 0.25 gives 0.15625 (39.84, written as 40),
    // 0.75 gives 0.84375 (215.16, written as 215)
    assert_eq!(
        feathered(&["width=2", "profile=smoothstep"], &STEP),
        [0, 40, 215, 255, 255]
    );
    // cosine 0.5 - 0.5 cos(pi t): 0.25 gives 0.14645 (37.34, written as 37),
    // 0.75 gives 0.85355 (217.66, written as 218)
    assert_eq!(
        feathered(&["width=2", "profile=cosine"], &STEP),
        [0, 37, 218, 255, 255]
    );
}

#[test]
fn the_edge_offset_moves_the_contour() {
    // offset 0: the signed distances are -2, -1, 1, 2, 3. width 2, centred: t = (s + 1) / 2.
    assert_eq!(
        feathered(&["width=2", "edge_offset=0"], &STEP),
        [0, 0, 255, 255, 255]
    );
    // width 4, centred: t = (s + 2) / 4 = 0, 0.25, 0.75, 1, 1.25
    assert_eq!(
        feathered(&["width=4", "edge_offset=0"], &STEP),
        [0, 64, 191, 255, 255]
    );
}

#[test]
fn the_threshold_decides_what_is_inside() {
    let soft = [0, 100, 200, 255, 255];
    // 100 / 255 = 0.39 is outside at 0.5: the hard mask is that of the hand worked frame
    assert_eq!(feathered(&["width=2"], &soft), [0, 64, 191, 255, 255]);
    // at 0.3 it is inside: the contour moves one pixel to the left
    assert_eq!(
        feathered(&["width=2", "threshold=0.3"], &soft),
        [64, 191, 255, 255, 255]
    );
    // at 1 only full alpha is inside: the contour moves one pixel to the right
    assert_eq!(
        feathered(&["width=2", "threshold=1"], &soft),
        [0, 0, 64, 191, 255]
    );
    // at 0 every pixel is inside and there is no contour: the distance is infinite
    assert_eq!(
        feathered(&["width=2", "threshold=0"], &soft),
        [255, 255, 255, 255, 255]
    );
}

#[test]
fn the_zero_border_puts_an_edge_around_the_frame() {
    let full = [255u8; 3];
    let size = ["3", "1", "8"];
    let words = |border: &'static str| ["width=2", "direction=inward", border];
    // clamp: no pixel is outside, so there is no contour and alpha stays full
    let clamp = frames(
        STAGE,
        size,
        &params(&words("border=clamp")),
        &with_alpha8(&full),
    );
    assert_eq!(alpha8(&clamp), [255, 255, 255]);
    for border in ["border=mirror", "border=mirror-no-repeat"] {
        let out = frames(STAGE, size, &params(&words(border)), &with_alpha8(&full));
        assert_eq!(out, clamp, "{border}");
    }
    // zero: the row above is outside at distance 1, so s = 0.5 and t = 0.25 for every pixel
    let zero = frames(
        STAGE,
        size,
        &params(&words("border=zero")),
        &with_alpha8(&full),
    );
    assert_eq!(alpha8(&zero), [64, 64, 64]);
    // nothing inside: the distance is minus infinity under both rules
    for border in ["border=clamp", "border=zero"] {
        let out = frames(STAGE, size, &params(&words(border)), &with_alpha8(&[0; 3]));
        assert_eq!(alpha8(&out), [0, 0, 0], "{border}");
    }
}

#[test]
fn distances_are_euclidean_in_two_dimensions() {
    // one inside pixel in the middle of nine, width 2, centred, offset 0.5:
    // the centre has s = 0.5 (t = 0.75, 191); its four neighbours have s = -0.5 (t = 0.25, 64);
    // the corners are sqrt(2) away, s = -0.9142, t = 0.0429, 10.94, written as 11.
    let mut alpha = [0u8; 9];
    alpha[4] = 255;
    let out = frames(
        STAGE,
        ["3", "3", "8"],
        &params(&["width=2"]),
        &with_alpha8(&alpha),
    );
    assert_eq!(alpha8(&out), [11, 64, 11, 64, 191, 64, 11, 64, 11]);
}

#[test]
fn sixteen_bit_and_float_frames_carry_the_same_ramp() {
    // 0.25 * 65535 = 16383.75, written as 16384; 0.75 * 65535 = 49151.25, written as 49151
    let pixels16: Vec<[u16; 4]> = [0u16, 0, 65535, 65535, 65535]
        .iter()
        .map(|a| [100, 2000, 60000, *a])
        .collect();
    let out = frames(
        STAGE,
        ["5", "1", "16"],
        &params(&["width=2"]),
        &rgba16(&pixels16),
    );
    let want: Vec<[u16; 4]> = [0u16, 16384, 49151, 65535, 65535]
        .iter()
        .map(|a| [100, 2000, 60000, *a])
        .collect();
    assert_eq!(out, rgba16(&want));
    // depth 32: the ramp is written as it is; colour passes bit for bit
    let px: Vec<[f32; 4]> = [0.0f32, 0.0, 1.0, 1.0, 1.0]
        .iter()
        .map(|a| [2.5, -0.0, 0.125, *a])
        .collect();
    let input = planar32(&px);
    let out = frames(STAGE, ["5", "1", "32"], &params(&["width=2"]), &input);
    assert_eq!(out[..60], input[..60], "the three colour planes pass");
    let alpha: Vec<f32> = out[60..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    assert_eq!(alpha, [0.0, 0.25, 0.75, 1.0, 1.0]);
}

#[test]
fn the_stage_is_the_function_of_the_crate_under_every_choice() {
    // a made up 6 by 4 alpha channel with a soft blob, at depth 8
    let alpha: [u8; 24] = [
        0, 10, 40, 30, 0, 0, 5, 140, 255, 250, 90, 0, 0, 200, 255, 255, 180, 20, 0, 60, 130, 127,
        128, 0,
    ];
    let plane = pixels::decode(&alpha, 8);
    for (profile_word, profile) in [
        ("profile=linear", Profile::Linear),
        ("profile=smoothstep", Profile::Smoothstep),
        ("profile=cosine", Profile::Cosine),
    ] {
        for (direction_word, direction) in [
            ("direction=inward", Direction::Inward),
            ("direction=outward", Direction::Outward),
            ("direction=centred", Direction::Centred),
        ] {
            for (border_word, border) in [
                ("border=zero", soft_edge::Border::Zero),
                ("border=clamp", soft_edge::Border::Clamp),
            ] {
                let cfg = Config {
                    profile,
                    direction,
                    border,
                    threshold: 0.4,
                    edge_offset: 0.25,
                    ..Config::default()
                };
                let made = soft_edge::feather(&plane, 6, 4, 2.75, &cfg).unwrap();
                let mut want = Vec::new();
                pixels::encode(&made, 8, &mut want);
                let out = frames(
                    STAGE,
                    ["6", "4", "8"],
                    &params(&[
                        "width=2.75",
                        "threshold=0.4",
                        "edge_offset=0.25",
                        profile_word,
                        direction_word,
                        border_word,
                    ]),
                    &with_alpha8(&alpha),
                );
                assert_eq!(
                    alpha8(&out),
                    want,
                    "{profile_word} {direction_word} {border_word}"
                );
            }
        }
    }
}

#[test]
fn frames_are_processed_one_by_one() {
    let a = with_alpha8(&STEP);
    let b = with_alpha8(&[255, 255, 0, 0, 0]);
    let out = frames(
        STAGE,
        SIZE,
        &["--param", "width=2", "--frames", "3"],
        &[a.clone(), b.clone(), a.clone()].concat(),
    );
    assert_eq!(
        out,
        [
            with_alpha8(&[0, 64, 191, 255, 255]),
            with_alpha8(&[255, 191, 64, 0, 0]),
            with_alpha8(&[0, 64, 191, 255, 255])
        ]
        .concat()
    );
}

// -- the key file ----------------------------------------------------------------------------

#[test]
fn a_key_file_moves_the_width_from_frame_to_frame() {
    let frame = with_alpha8(&STEP);
    let keys = Scratch::new(
        "feather-moves",
        "keys-1\nkey width 0 0 hold\nkey width 1 2 linear\nkey width 2 4 hold\n",
    );
    let layer = keys.layer();
    let out = frames(
        STAGE,
        SIZE,
        &["--layer", layer.as_str(), "--param", "edge_offset=0"],
        &[frame.clone(), frame.clone(), frame.clone()].concat(),
    );
    // width 0: as read; width 2 and width 4 at offset 0: worked out above
    assert_eq!(
        out,
        [
            frame,
            with_alpha8(&[0, 0, 255, 255, 255]),
            with_alpha8(&[0, 64, 191, 255, 255])
        ]
        .concat()
    );
}

#[test]
fn key_file_refusals() {
    let frame = with_alpha8(&STEP);
    refused(
        STAGE,
        SIZE,
        &["--layer", "keys=/no/such/folder/keys.txt"],
        &frame,
        4,
        "error: bad_input: cannot read the key file `/no/such/folder/keys.txt`: ",
    );
    let malformed = Scratch::new("feather-malformed", "keys-1\nkey width zero 1 hold\n");
    refused(
        STAGE,
        SIZE,
        &["--layer", &malformed.layer()],
        &frame,
        4,
        "error: bad_input: key file `",
    );
    let not_keyed = Scratch::new("feather-not-keyed", "keys-1\nkey threshold 0 0.2 hold\n");
    refused(
        STAGE,
        SIZE,
        &["--layer", &not_keyed.layer()],
        &frame,
        3,
        "error: unsupported: key file `",
    );
    let outside = Scratch::new("feather-outside", "keys-1\nkey width 0 -1 hold\n");
    refused(
        STAGE,
        SIZE,
        &["--layer", &outside.layer()],
        &frame,
        3,
        "error: unsupported: key file `",
    );
    refused(
        STAGE,
        SIZE,
        &["--layer", "keys=a", "--layer", "keys=b"],
        &frame,
        2,
        "error: usage: layer `keys` given more than once",
    );
    refused(
        STAGE,
        SIZE,
        &["--layer", "keys="],
        &frame,
        2,
        "error: usage: layer `keys` has an empty path",
    );
    refused(
        STAGE,
        SIZE,
        &["--layer", "mask=x"],
        &frame,
        3,
        "error: unsupported: soft-edge-feather has no layer `mask`: it takes keys",
    );
}

// -- the arguments ---------------------------------------------------------------------------

#[test]
fn every_value_of_every_parameter_is_taken() {
    let frame = with_alpha8(&STEP);
    for word in [
        "width=0",
        "width=4096",
        "width=2.5e0",
        "profile=linear",
        "profile=smoothstep",
        "profile=cosine",
        "direction=inward",
        "direction=outward",
        "direction=centred",
        "threshold=0",
        "threshold=1",
        "edge_offset=-4096",
        "edge_offset=4096",
        "border=zero",
        "border=clamp",
        "border=mirror",
        "border=mirror-no-repeat",
    ] {
        // a width above 0, so that the other parameters are read
        let mut words = vec![word];
        if !word.starts_with("width=") {
            words.push("width=1.5");
        }
        let out = frames(STAGE, SIZE, &params(&words), &frame);
        assert_eq!(out.len(), frame.len(), "{word}");
    }
}

#[test]
fn parameter_refusals_follow_the_convention() {
    let frame = with_alpha8(&STEP);
    let cases: [(&[&str], i32, &str); 14] = [
        (&["radius=1"], 3, "error: unsupported: soft-edge-feather has no parameter `radius`: it takes width, profile, direction, threshold, edge_offset, border"),
        (&["width=-0.5"], 3, "error: unsupported: bad value for `width`: -0.5 is outside 0 to 4096"),
        (&["width=4097"], 3, "error: unsupported: bad value for `width`: 4097 is outside 0 to 4096"),
        (&["width=two"], 3, "error: unsupported: bad value for `width`: cannot parse `two` as a number"),
        (&["width=nan"], 3, "error: unsupported: bad value for `width`: cannot parse `nan` as a number"),
        (&["width=2@0;4@10"], 3, "error: unsupported: bad value for `width`: a keyed value is not taken on the command line"),
        (&["profile=cubic"], 3, "error: unsupported: bad value for `profile`: `cubic` is not one of linear, smoothstep, cosine"),
        (&["direction=both"], 3, "error: unsupported: bad value for `direction`: `both` is not one of inward, outward, centred"),
        (&["threshold=1.5"], 3, "error: unsupported: bad value for `threshold`: 1.5 is outside 0 to 1"),
        (&["threshold=-0.1"], 3, "error: unsupported: bad value for `threshold`: -0.1 is outside 0 to 1"),
        (&["edge_offset=5000"], 3, "error: unsupported: bad value for `edge_offset`: 5000 is outside -4096 to 4096"),
        (&["border=repeat"], 3, "error: unsupported: bad value for `border`: `repeat` is not one of zero, clamp, mirror, mirror-no-repeat"),
        (&["width=1", "width=2"], 2, "error: usage: parameter `width` given more than once"),
        (&["profile=linear", "profile=linear"], 2, "error: usage: parameter `profile` given more than once"),
    ];
    for (words, code, line) in cases {
        refused(STAGE, SIZE, &params(words), &frame, code, line);
    }
}

#[test]
fn a_frame_above_the_side_limit_of_the_crate_is_unsupported() {
    // the limit is MAX_SIDE - 2 of the distance transform: 1048574
    let line = "error: unsupported: soft-edge-feather takes frames of at most 1048574 pixels per side, not ";
    refused(STAGE, ["1048575", "1", "8"], &["--describe"], b"", 3, line);
    refused(STAGE, ["1", "1048575", "8"], &["--describe"], b"", 3, line);
    refused(STAGE, ["1048575", "1", "8"], &[], b"", 3, line);
    let at_the_limit = run(STAGE, ["1048574", "1", "8"], &["--describe"], b"");
    assert_eq!((at_the_limit.code, at_the_limit.err.as_str()), (0, ""));
}

#[test]
fn the_shared_refusals_hold_for_this_stage() {
    let frame = with_alpha8(&STEP);
    refused(
        STAGE,
        ["5", "1", "12"],
        &[],
        &frame,
        3,
        "error: unsupported: depth 12 is not supported: soft-edge-feather takes 8, 16 or 32",
    );
    refused(
        STAGE,
        SIZE,
        &["--width2", "1"],
        &frame,
        2,
        "error: usage: unknown option `--width2`",
    );
    let partial = run(
        STAGE,
        SIZE,
        &params(&["width=2"]),
        &[frame.clone(), frame[..3].to_vec()].concat(),
    );
    assert_eq!(partial.code, 4);
    assert!(partial.err.starts_with(
        "error: partial_frame: frame 1: the input ends inside a frame (3 of 20 bytes) after 1 whole frames"
    ));
    assert_eq!(partial.out, with_alpha8(&[0, 64, 191, 255, 255]));
    let count = run(STAGE, SIZE, &["--frames", "3"], &frame);
    assert_eq!(count.code, 4);
    assert!(count
        .err
        .starts_with("error: bad_input: the input holds 1 frames, but --frames says 3"));
    let empty = run(STAGE, SIZE, &[], b"");
    assert_eq!(
        (empty.code, empty.out.len(), empty.err.as_str()),
        (0, 0, "")
    );
}

#[test]
fn a_reader_that_leaves_is_a_broken_pipe() {
    // 2000 frames; the reader closes the output before the stage writes
    let input = with_alpha8(&STEP).repeat(2000);
    let (code, err) = reader_leaves(STAGE, SIZE, &params(&["width=2"]), &input);
    assert_eq!(code, 4, "{err}");
    assert!(err.starts_with("error: broken_pipe: frame "), "{err}");
    assert!(err.contains(": cannot write the output: "), "{err}");
    assert_eq!(err.matches('\n').count(), 1, "{err}");
}
