//! `alpha-premultiply` on the built program: its manifest entry, its arguments, every refusal,
//! and what it writes for made up frames, against values worked out by hand and against the
//! `pixel-format` crate called directly.

mod image_support;

use image_support::{
    frames, manifest_entry, param_ids, params, planar32, reader_leaves, refused, rgba16, rgba8, run,
};
use pixel_format::{AlphaOp, Clamp, Config, Depth, Precision, PremulOrder, Rounding, ZeroAlpha};
use stage_cli::registry::GENERIC_CLAIM;

const STAGE: &str = "alpha-premultiply";

/// One 8 bit pixel as a 1 by 1 frame through the stage.
fn pixel8(words: &[&str], pixel: [u8; 4]) -> Vec<u8> {
    frames(STAGE, ["1", "1", "8"], &params(words), &pixel)
}

// -- the manifest entry and the describe answer ----------------------------------------------

#[test]
fn the_stage_is_listed_as_generic_and_says_that_its_output_is_premultiplied() {
    let entry = manifest_entry(STAGE);
    assert!(entry.starts_with(
        "{\"id\":\"alpha-premultiply\",\"kind\":\"frame\",\"spool\":false,\"geometry\":\"preserve\",\"depths\":[8,16,32],\"params\":["
    ));
    assert_eq!(
        param_ids(&entry),
        [
            "op",
            "rounding",
            "precision",
            "clamp",
            "zero_alpha",
            "order"
        ]
    );
    assert!(entry.contains(
        "{\"id\":\"op\",\"name\":\"Operation\",\"kind\":\"popup\",\"default\":\"premultiply\",\"choices\":[\"premultiply\",\"unpremultiply\"],\"keyed\":false,"
    ));
    assert!(
        entry.contains("\"default\":\"zero-colour\",\"choices\":[\"zero-colour\",\"keep-colour\"]")
    );
    assert!(entry
        .contains("\"default\":\"convert-first\",\"choices\":[\"convert-first\",\"alpha-first\"]"));
    assert_eq!(entry.matches("\"keyed\":true").count(), 0);
    assert!(entry.contains("\"layers\":[],\"name\":\"Alpha premultiply\""));
    assert!(entry.contains(
        "The frame contract carries straight alpha: the output of op=premultiply is premultiplied and outside the contract until a later stage divides by alpha again."
    ));
    assert!(entry.contains(&format!("{GENERIC_CLAIM}\",\"family\":\"generic\"}}")));
}

#[test]
fn describe_answers_the_input_geometry_and_the_depth_of_the_request() {
    for depth in ["8", "16", "32"] {
        let answer = run(
            STAGE,
            ["7", "4", depth],
            &["--describe", "--frames", "3", "--param", "op=unpremultiply"],
            b"",
        );
        assert_eq!((answer.code, answer.err.as_str()), (0, ""));
        assert_eq!(
            String::from_utf8(answer.out).unwrap(),
            format!(
                "{{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"alpha-premultiply\",\"width\":7,\"height\":4,\"depth\":{depth},\"fps\":\"24/1\",\"frames\":3}}\n"
            )
        );
    }
}

// -- what the stage computes -----------------------------------------------------------------

#[test]
fn colour_is_multiplied_by_alpha_at_the_defaults() {
    // colour * alpha / 255 rounded half up: 200 * 128 / 255 = 100.39, 100 * 128 / 255 = 50.20,
    // 50 * 128 / 255 = 25.10. Alpha is not changed.
    assert_eq!(pixel8(&[], [200, 100, 50, 128]), [100, 50, 25, 128]);
    assert_eq!(
        pixel8(&["op=premultiply"], [200, 100, 50, 128]),
        [100, 50, 25, 128]
    );
    // full alpha keeps the colour, zero alpha clears it
    assert_eq!(pixel8(&[], [255, 0, 128, 255]), [255, 0, 128, 255]);
    assert_eq!(pixel8(&[], [10, 20, 30, 0]), [0, 0, 0, 0]);
}

#[test]
fn colour_is_divided_by_alpha_and_capped_at_full_scale() {
    // colour * 255 / alpha rounded half up: 100 * 255 / 128 = 199.22, 50 * 255 / 128 = 99.61,
    // 25 * 255 / 128 = 49.80
    assert_eq!(
        pixel8(&["op=unpremultiply"], [100, 50, 25, 128]),
        [199, 100, 50, 128]
    );
    // 200 * 255 / 100 = 510 is capped at 255
    assert_eq!(
        pixel8(&["op=unpremultiply"], [200, 0, 100, 100]),
        [255, 0, 255, 100]
    );
}

#[test]
fn a_tie_follows_the_rounding() {
    // 30 * 255 / 100 = 76.5 exactly, 10 * 255 / 100 = 25.5 exactly
    assert_eq!(
        pixel8(&["op=unpremultiply"], [30, 10, 0, 100]),
        [77, 26, 0, 100]
    );
    assert_eq!(
        pixel8(
            &["op=unpremultiply", "rounding=ties-even"],
            [30, 10, 0, 100]
        ),
        [76, 26, 0, 100]
    );
}

#[test]
fn zero_alpha_follows_the_rule_chosen() {
    assert_eq!(pixel8(&["op=unpremultiply"], [10, 20, 30, 0]), [0, 0, 0, 0]);
    assert_eq!(
        pixel8(
            &["op=unpremultiply", "zero_alpha=zero-colour"],
            [10, 20, 30, 0]
        ),
        [0, 0, 0, 0]
    );
    assert_eq!(
        pixel8(
            &["op=unpremultiply", "zero_alpha=keep-colour"],
            [10, 20, 30, 0]
        ),
        [10, 20, 30, 0]
    );
    // the rule is read by the division only
    assert_eq!(
        pixel8(
            &["op=premultiply", "zero_alpha=keep-colour"],
            [10, 20, 30, 0]
        ),
        [0, 0, 0, 0]
    );
}

#[test]
fn sixteen_bit_frames_use_the_full_scale_of_the_contract() {
    // colour * alpha / 65535: 40000 * 32768 / 65535 = 20000.31, 20000 * 32768 / 65535 = 10000.15,
    // 10000 * 32768 / 65535 = 5000.08
    let input = rgba16(&[[40000, 20000, 10000, 32768], [65535, 1, 0, 65535]]);
    let out = frames(STAGE, ["2", "1", "16"], &[], &input);
    assert_eq!(
        out,
        rgba16(&[[20000, 10000, 5000, 32768], [65535, 1, 0, 65535]])
    );
    // back: 20000 * 65535 / 32768 = 39999.39, 10000 * 65535 / 32768 = 19999.69,
    // 5000 * 65535 / 32768 = 9999.85
    let back = frames(
        STAGE,
        ["2", "1", "16"],
        &params(&["op=unpremultiply"]),
        &out,
    );
    assert_eq!(
        back,
        rgba16(&[[39999, 20000, 10000, 32768], [65535, 1, 0, 65535]])
    );
}

#[test]
fn float_frames_are_multiplied_and_divided_without_a_clamp() {
    // depth 32 is planar G, B, R, A: the alpha plane is the fourth plane
    let straight = planar32(&[
        [0.5, 0.25, 1.0, 0.5],
        [3.0, -2.0, 0.0, 0.5],
        [0.75, 0.5, 0.25, 0.0],
        [0.75, 0.5, 0.25, 1.0],
    ]);
    let multiplied = planar32(&[
        [0.25, 0.125, 0.5, 0.5],
        [1.5, -1.0, 0.0, 0.5],
        [0.0, 0.0, 0.0, 0.0],
        [0.75, 0.5, 0.25, 1.0],
    ]);
    assert_eq!(frames(STAGE, ["4", "1", "32"], &[], &straight), multiplied);
    // the division gives the colour back where alpha is not 0, and 0 where it is
    let divided = planar32(&[
        [0.5, 0.25, 1.0, 0.5],
        [3.0, -2.0, 0.0, 0.5],
        [0.0, 0.0, 0.0, 0.0],
        [0.75, 0.5, 0.25, 1.0],
    ]);
    assert_eq!(
        frames(
            STAGE,
            ["4", "1", "32"],
            &params(&["op=unpremultiply"]),
            &multiplied
        ),
        divided
    );
    // keep-colour leaves the colour of a pixel with alpha 0, also with alpha -0.0
    let clear = planar32(&[[0.75, 0.5, 0.25, 0.0], [0.75, 0.5, 0.25, -0.0]]);
    assert_eq!(
        frames(
            STAGE,
            ["2", "1", "32"],
            &params(&["op=unpremultiply", "zero_alpha=keep-colour"]),
            &clear
        ),
        clear
    );
}

/// Every pixel of a frame through `convert_pixel` of the crate, at one depth.
fn by_the_crate(cfg: &Config, depth: Depth, op: AlphaOp, pixels: &[[u32; 4]]) -> Vec<[u32; 4]> {
    pixels
        .iter()
        .map(|px| cfg.convert_pixel(depth, depth, *px, op))
        .collect()
}

/// The words of a run and the configuration and operation they name, built here from the crate.
fn settings() -> Vec<(Vec<&'static str>, Config, AlphaOp)> {
    let mut out = Vec::new();
    for (op_word, op) in [
        ("op=premultiply", AlphaOp::Premultiply),
        ("op=unpremultiply", AlphaOp::Unpremultiply),
    ] {
        for (rounding_word, rounding) in [
            ("rounding=half-up", Rounding::HalfUp),
            ("rounding=ties-even", Rounding::TiesEven),
            ("rounding=truncate", Rounding::Truncate),
            ("rounding=floor", Rounding::Floor),
            ("rounding=ceil", Rounding::Ceil),
        ] {
            for (precision_word, precision) in [
                ("precision=f32", Precision::F32),
                ("precision=f64", Precision::F64),
            ] {
                for (order_word, order) in [
                    ("order=convert-first", PremulOrder::ConvertFirst),
                    ("order=alpha-first", PremulOrder::AlphaFirst),
                ] {
                    for (zero_word, zero_alpha) in [
                        ("zero_alpha=zero-colour", ZeroAlpha::ZeroColour),
                        ("zero_alpha=keep-colour", ZeroAlpha::KeepColour),
                    ] {
                        for (clamp_word, clamp) in [
                            ("clamp=none", Clamp::None),
                            ("clamp=range-nan-zero", Clamp::RangeNanZero),
                        ] {
                            out.push((
                                vec![
                                    op_word,
                                    rounding_word,
                                    precision_word,
                                    order_word,
                                    zero_word,
                                    clamp_word,
                                ],
                                Config {
                                    rounding,
                                    precision,
                                    clamp,
                                    zero_alpha,
                                    order,
                                    u16_max: 65535,
                                },
                                op,
                            ));
                        }
                    }
                }
            }
        }
    }
    out
}

#[test]
fn eight_bit_pixels_are_the_conversion_of_the_crate_under_every_setting() {
    // 64 made up pixels: every sixteenth alpha with spread colours, alpha 0 and full included
    let pixels: Vec<[u8; 4]> = (0..64u32)
        .map(|i| {
            [
                (i * 37) as u8,
                (i * 101) as u8,
                (255 - i * 4) as u8,
                [0u8, 1, 17, 100, 128, 200, 254, 255][(i % 8) as usize],
            ]
        })
        .collect();
    let input = rgba8(&pixels);
    let bits: Vec<[u32; 4]> = pixels.iter().map(|p| p.map(u32::from)).collect();
    for (words, cfg, op) in settings() {
        let out = frames(STAGE, ["8", "8", "8"], &params(&words), &input);
        let want: Vec<[u8; 4]> = by_the_crate(&cfg, Depth::U8, op, &bits)
            .iter()
            .map(|p| p.map(|v| v as u8))
            .collect();
        assert_eq!(out, rgba8(&want), "{words:?}");
    }
}

#[test]
fn sixteen_bit_pixels_are_the_conversion_of_the_crate_under_every_setting() {
    let pixels: Vec<[u16; 4]> = (0..64u32)
        .map(|i| {
            [
                (i * 9973) as u16,
                (i * 30011) as u16,
                (65535 - i * 1000) as u16,
                [0u16, 1, 257, 30000, 32768, 50000, 65534, 65535][(i % 8) as usize],
            ]
        })
        .collect();
    let input = rgba16(&pixels);
    let bits: Vec<[u32; 4]> = pixels.iter().map(|p| p.map(u32::from)).collect();
    let depth = Depth::U16 { max: 65535 };
    for (words, cfg, op) in settings() {
        let out = frames(STAGE, ["8", "8", "16"], &params(&words), &input);
        let want: Vec<[u16; 4]> = by_the_crate(&cfg, depth, op, &bits)
            .iter()
            .map(|p| p.map(|v| v as u16))
            .collect();
        assert_eq!(out, rgba16(&want), "{words:?}");
    }
}

#[test]
fn float_pixels_are_the_conversion_of_the_crate_under_every_setting() {
    // no infinite alpha: zero times infinity is not a number, and its sign is not the same on
    // every processor
    let alphas = [0.0f32, -0.0, 1.0, 0.5, 0.123_456, 2.0, 1.0e-30, 1.0e30];
    let pixels: Vec<[f32; 4]> = (0..16usize)
        .map(|i| {
            [
                i as f32 / 7.0,
                1.0 - i as f32 / 3.0,
                0.333_333 * (i % 5) as f32,
                alphas[i % 8],
            ]
        })
        .collect();
    let input = planar32(&pixels);
    let bits: Vec<[u32; 4]> = pixels.iter().map(|p| p.map(f32::to_bits)).collect();
    for (words, cfg, op) in settings() {
        let out = frames(STAGE, ["4", "4", "32"], &params(&words), &input);
        let want: Vec<[f32; 4]> = by_the_crate(&cfg, Depth::F32, op, &bits)
            .iter()
            .map(|p| p.map(f32::from_bits))
            .collect();
        assert_eq!(out, planar32(&want), "{words:?}");
    }
}

#[test]
fn frames_are_processed_one_by_one() {
    let a = [200u8, 100, 50, 128];
    let b = [10u8, 20, 30, 0];
    let out = frames(
        STAGE,
        ["1", "1", "8"],
        &["--frames", "3"],
        &[a, b, a].concat(),
    );
    assert_eq!(
        out,
        [[100u8, 50, 25, 128], [0, 0, 0, 0], [100, 50, 25, 128]].concat()
    );
}

// -- the arguments ---------------------------------------------------------------------------

#[test]
fn every_value_of_every_parameter_is_taken() {
    let frame = [9u8, 99, 199, 250];
    for word in [
        "op=premultiply",
        "op=unpremultiply",
        "rounding=half-up",
        "rounding=ties-even",
        "rounding=truncate",
        "rounding=floor",
        "rounding=ceil",
        "precision=f32",
        "precision=f64",
        "clamp=none",
        "clamp=range",
        "clamp=range-nan-zero",
        "zero_alpha=zero-colour",
        "zero_alpha=keep-colour",
        "order=convert-first",
        "order=alpha-first",
    ] {
        let out = frames(STAGE, ["1", "1", "8"], &params(&[word]), &frame);
        assert_eq!(out.len(), frame.len(), "{word}");
    }
}

#[test]
fn parameter_refusals_follow_the_convention() {
    let frame = [9u8, 99, 199, 250];
    let size = ["1", "1", "8"];
    let cases: [(&[&str], i32, &str); 10] = [
        (&["alpha=premultiplied"], 3, "error: unsupported: alpha-premultiply has no parameter `alpha`: it takes op, rounding, precision, clamp, zero_alpha, order"),
        (&["op=multiply"], 3, "error: unsupported: bad value for `op`: `multiply` is not one of premultiply, unpremultiply"),
        (&["op="], 3, "error: unsupported: bad value for `op`: `` is not one of premultiply, unpremultiply"),
        (&["op=premultiply@0"], 3, "error: unsupported: bad value for `op`: a keyed value is not taken on the command line"),
        (&["rounding=nearest"], 3, "error: unsupported: bad value for `rounding`: `nearest` is not one of half-up, ties-even, truncate, floor, ceil"),
        (&["precision=f16"], 3, "error: unsupported: bad value for `precision`: `f16` is not one of f32, f64"),
        (&["clamp=wrap"], 3, "error: unsupported: bad value for `clamp`: `wrap` is not one of none, range, range-nan-zero"),
        (&["zero_alpha=black"], 3, "error: unsupported: bad value for `zero_alpha`: `black` is not one of zero-colour, keep-colour"),
        (&["order=last"], 3, "error: unsupported: bad value for `order`: `last` is not one of convert-first, alpha-first"),
        (&["op=premultiply", "op=unpremultiply"], 2, "error: usage: parameter `op` given more than once"),
    ];
    for (words, code, line) in cases {
        refused(STAGE, size, &params(words), &frame, code, line);
    }
}

#[test]
fn the_stage_takes_no_layer() {
    let frame = [9u8, 99, 199, 250];
    refused(
        STAGE,
        ["1", "1", "8"],
        &["--layer", "keys=x"],
        &frame,
        3,
        "error: unsupported: alpha-premultiply takes no layers: `keys` is not one",
    );
}

#[test]
fn the_shared_refusals_hold_for_this_stage() {
    let frame = [200u8, 100, 50, 128];
    let size = ["1", "1", "8"];
    refused(
        STAGE,
        ["1", "1", "12"],
        &[],
        &frame,
        3,
        "error: unsupported: depth 12 is not supported: alpha-premultiply takes 8, 16 or 32",
    );
    refused(
        STAGE,
        size,
        &["--op", "premultiply"],
        &frame,
        2,
        "error: usage: unknown option `--op`",
    );
    let partial = run(STAGE, size, &[], &[&frame[..], &frame[..3]].concat());
    assert_eq!(partial.code, 4);
    assert!(partial.err.starts_with(
        "error: partial_frame: frame 1: the input ends inside a frame (3 of 4 bytes) after 1 whole frames"
    ));
    assert_eq!(partial.out, [100, 50, 25, 128]);
    let count = run(STAGE, size, &["--frames", "2"], &frame);
    assert_eq!(count.code, 4);
    assert!(count
        .err
        .starts_with("error: bad_input: the input holds 1 frames, but --frames says 2"));
    let empty = run(STAGE, size, &[], b"");
    assert_eq!(
        (empty.code, empty.out.len(), empty.err.as_str()),
        (0, 0, "")
    );
}

#[test]
fn a_reader_that_leaves_is_a_broken_pipe() {
    // 2000 frames; the reader closes the output before the stage writes
    let input = [200u8, 100, 50, 128].repeat(2000);
    let (code, err) = reader_leaves(
        STAGE,
        ["1", "1", "8"],
        &params(&["op=unpremultiply"]),
        &input,
    );
    assert_eq!(code, 4, "{err}");
    assert!(err.starts_with("error: broken_pipe: frame "), "{err}");
    assert!(err.contains(": cannot write the output: "), "{err}");
    assert_eq!(err.matches('\n').count(), 1, "{err}");
}
