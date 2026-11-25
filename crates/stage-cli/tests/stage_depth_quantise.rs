//! `depth-quantise` on the built program: its manifest entry, its arguments, every refusal, and
//! what it writes for made up frames, against values worked out by hand and against the
//! `pixel-format` crate called directly.

mod image_support;

use image_support::{frames, manifest_entry, param_ids, params, reader_leaves, refused, run};
use pixel_format::{Clamp, Config, Depth, Precision, Rounding};
use stage_cli::registry::GENERIC_CLAIM;

const STAGE: &str = "depth-quantise";

const ROUNDINGS: [(&str, Rounding); 5] = [
    ("rounding=half-up", Rounding::HalfUp),
    ("rounding=ties-even", Rounding::TiesEven),
    ("rounding=truncate", Rounding::Truncate),
    ("rounding=floor", Rounding::Floor),
    ("rounding=ceil", Rounding::Ceil),
];
const PRECISIONS: [(&str, Precision); 2] = [
    ("precision=f32", Precision::F32),
    ("precision=f64", Precision::F64),
];
const CLAMPS: [(&str, Clamp); 3] = [
    ("clamp=none", Clamp::None),
    ("clamp=range", Clamp::Range),
    ("clamp=range-nan-zero", Clamp::RangeNanZero),
];

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn words16(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// One 8 bit pixel as a 1 by 1 frame through the stage.
fn pixel8(words: &[&str], pixel: [u8; 4]) -> Vec<u8> {
    frames(STAGE, ["1", "1", "8"], &params(words), &pixel)
}

// -- the manifest entry and the describe answer ----------------------------------------------

#[test]
fn the_stage_is_listed_as_generic_with_its_parameters_and_no_layer() {
    let entry = manifest_entry(STAGE);
    assert!(entry.starts_with(
        "{\"id\":\"depth-quantise\",\"kind\":\"frame\",\"spool\":false,\"geometry\":\"preserve\",\"depths\":[8,16,32],\"params\":["
    ));
    assert_eq!(
        param_ids(&entry),
        ["levels", "u16_max", "rounding", "precision", "clamp"]
    );
    assert!(entry.contains(
        "{\"id\":\"levels\",\"name\":\"Levels\",\"kind\":\"popup\",\"default\":\"8\",\"choices\":[\"8\",\"16\"],\"keyed\":false,"
    ));
    assert!(entry.contains(
        "{\"id\":\"u16_max\",\"name\":\"16 bit full scale\",\"kind\":\"int\",\"default\":65535,\"min\":1,\"max\":65535,\"keyed\":false,"
    ));
    assert!(entry.contains(
        "\"kind\":\"popup\",\"default\":\"half-up\",\"choices\":[\"half-up\",\"ties-even\",\"truncate\",\"floor\",\"ceil\"]"
    ));
    assert!(entry.contains("\"default\":\"f64\",\"choices\":[\"f32\",\"f64\"]"));
    assert!(entry.contains(
        "\"default\":\"range-nan-zero\",\"choices\":[\"none\",\"range\",\"range-nan-zero\"]"
    ));
    assert_eq!(entry.matches("\"keyed\":true").count(), 0);
    assert!(entry.contains("\"layers\":[],\"name\":\"Depth quantise\""));
    assert!(entry.contains("the stream keeps its depth"));
    assert!(entry.contains(&format!("{GENERIC_CLAIM}\",\"family\":\"generic\"}}")));
}

#[test]
fn describe_answers_the_input_geometry_and_the_depth_of_the_request() {
    for depth in ["8", "16", "32"] {
        let answer = run(
            STAGE,
            ["7", "4", depth],
            &["--describe", "--frames", "3", "--param", "levels=16"],
            b"",
        );
        assert_eq!((answer.code, answer.err.as_str()), (0, ""));
        assert_eq!(
            String::from_utf8(answer.out).unwrap(),
            format!(
                "{{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"depth-quantise\",\"width\":7,\"height\":4,\"depth\":{depth},\"fps\":\"24/1\",\"frames\":3}}\n"
            )
        );
    }
}

// -- what the stage computes -----------------------------------------------------------------

#[test]
fn an_eight_bit_stream_is_on_the_eight_bit_grid_already() {
    let input: Vec<u8> = (0..=255u8).collect();
    assert_eq!(frames(STAGE, ["64", "1", "8"], &[], &input), input);
}

#[test]
fn a_grid_of_two_values_splits_an_eight_bit_stream_in_the_middle() {
    // levels 16 with full scale 1: the grid holds 0 and 1. A sample v becomes round(v / 255)
    // and comes back as 0 or 255.
    let words = |rounding: &'static str| ["levels=16", "u16_max=1", rounding];
    let pixel = [0, 127, 128, 255];
    // half up: 127 / 255 = 0.498 goes to 0, 128 / 255 = 0.502 goes to 1
    assert_eq!(pixel8(&words("rounding=half-up"), pixel), [0, 0, 255, 255]);
    assert_eq!(
        pixel8(&words("rounding=ties-even"), pixel),
        [0, 0, 255, 255]
    );
    // floor and truncate: only full scale reaches 1
    assert_eq!(pixel8(&words("rounding=floor"), pixel), [0, 0, 0, 255]);
    assert_eq!(pixel8(&words("rounding=truncate"), pixel), [0, 0, 0, 255]);
    // ceil: everything above 0 reaches 1
    assert_eq!(pixel8(&words("rounding=ceil"), pixel), [0, 255, 255, 255]);
}

#[test]
fn a_grid_of_four_values_holds_thirds() {
    // levels 16 with full scale 3: the grid holds 0, 1, 2, 3, which come back as 0, 85, 170,
    // 255. v * 3 / 255: 42 gives 0.494, 43 gives 0.506, 127 gives 1.494, 128 gives 1.506.
    let out = frames(
        STAGE,
        ["2", "1", "8"],
        &params(&["levels=16", "u16_max=3"]),
        &[0, 42, 43, 100, 127, 128, 200, 255],
    );
    assert_eq!(out, [0, 0, 85, 85, 85, 170, 170, 255]);
}

#[test]
fn a_sixteen_bit_stream_carries_only_eight_bit_values_at_the_defaults() {
    // v * 255 / 65535 rounded, times 257: 1000 gives 3.89, so 4 and 1028; 32768 gives 127.502,
    // so 128 and 32896; 300 gives 1.17, so 1 and 257.
    let input = words16(&[0, 1000, 32768, 65535, 300, 127, 128, 129]);
    let out = frames(STAGE, ["2", "1", "16"], &[], &input);
    assert_eq!(out, words16(&[0, 1028, 32896, 65535, 257, 0, 0, 257]));
    // on the 16 bit grid at the full scale of the stream every value is on the grid
    let same = frames(STAGE, ["2", "1", "16"], &params(&["levels=16"]), &input);
    assert_eq!(same, input);
}

#[test]
fn a_float_stream_is_snapped_and_clamped_as_the_clamp_says() {
    // 0.3 * 255 = 76.5 and a little, rounds to 77; back it is 77 / 255
    let step = (77.0f64 / 255.0) as f32;
    let input = floats(&[0.3, 2.0, -1.0, f32::NAN]);
    let size = ["1", "1", "32"];
    // the default clamp: above the range is full scale, below is 0, not a number is 0
    assert_eq!(
        frames(STAGE, size, &[], &input),
        floats(&[step, 1.0, 0.0, 0.0])
    );
    // range: not a number becomes full scale
    assert_eq!(
        frames(STAGE, size, &params(&["clamp=range"]), &input),
        floats(&[step, 1.0, 0.0, 1.0])
    );
    // none: 2.0 is the grid value 510, which comes back as 2.0; a negative value is 0
    assert_eq!(
        frames(STAGE, size, &params(&["clamp=none"]), &input),
        floats(&[step, 2.0, 0.0, 0.0])
    );
}

/// The words of a run and the configuration and grid they name, built here from the crate.
fn settings() -> Vec<(Vec<&'static str>, Config, Depth)> {
    let grids: [(&[&'static str], Depth, u16); 4] = [
        (&["levels=8"], Depth::U8, 65535),
        (&["levels=16"], Depth::U16 { max: 65535 }, 65535),
        (
            &["levels=16", "u16_max=1000"],
            Depth::U16 { max: 1000 },
            1000,
        ),
        (&["levels=16", "u16_max=31"], Depth::U16 { max: 31 }, 31),
    ];
    let mut out = Vec::new();
    for (grid_words, grid, u16_max) in grids {
        for (rounding_word, rounding) in ROUNDINGS {
            for (precision_word, precision) in PRECISIONS {
                let mut words = grid_words.to_vec();
                words.extend([rounding_word, precision_word]);
                let cfg = Config {
                    rounding,
                    precision,
                    u16_max,
                    ..Config::default_choice()
                };
                out.push((words, cfg, grid));
            }
        }
    }
    out
}

fn by_the_crate(cfg: &Config, stream: Depth, grid: Depth, bits: u32) -> u32 {
    cfg.convert_sample(grid, stream, cfg.convert_sample(stream, grid, bits))
}

#[test]
fn every_eight_bit_value_is_the_conversion_of_the_crate_under_every_setting() {
    let input: Vec<u8> = (0..=255u8).collect();
    for (words, cfg, grid) in settings() {
        let out = frames(STAGE, ["8", "8", "8"], &params(&words), &input);
        let want: Vec<u8> = input
            .iter()
            .map(|v| by_the_crate(&cfg, Depth::U8, grid, u32::from(*v)) as u8)
            .collect();
        assert_eq!(out, want, "{words:?}");
    }
}

#[test]
fn sixteen_bit_values_are_the_conversion_of_the_crate_under_every_setting() {
    // 4096 made up values that cover the range and both ends
    let values: Vec<u16> = (0..4096u32)
        .map(|i| ((i * 16 + i % 16) as u16) ^ if i % 97 == 0 { 0xffff } else { 0 })
        .collect();
    let input = words16(&values);
    let stream = Depth::U16 { max: 65535 };
    for (words, cfg, grid) in settings() {
        let out = frames(STAGE, ["32", "32", "16"], &params(&words), &input);
        let want: Vec<u16> = values
            .iter()
            .map(|v| by_the_crate(&cfg, stream, grid, u32::from(*v)) as u16)
            .collect();
        assert_eq!(out, words16(&want), "{words:?}");
    }
}

#[test]
fn float_values_are_the_conversion_of_the_crate_under_every_setting() {
    let values = [
        0.0f32,
        -0.0,
        1.0,
        0.5,
        0.123_456,
        0.999_999,
        1.000_001,
        7.25,
        -3.5,
        1.0e-9,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        0.501_960_8,
        0.498,
        65535.0,
    ];
    let input = floats(&values);
    for (words, cfg, grid) in settings() {
        for (clamp_word, clamp) in CLAMPS {
            let mut all = words.clone();
            all.push(clamp_word);
            let cfg = Config { clamp, ..cfg };
            let out = frames(STAGE, ["2", "2", "32"], &params(&all), &input);
            let want: Vec<u8> = values
                .iter()
                .flat_map(|v| by_the_crate(&cfg, Depth::F32, grid, v.to_bits()).to_le_bytes())
                .collect();
            assert_eq!(out, want, "{all:?}");
        }
    }
}

#[test]
fn frames_are_processed_one_by_one() {
    let a = [0u8, 42, 43, 100];
    let b = [127u8, 128, 200, 255];
    let out = frames(
        STAGE,
        ["1", "1", "8"],
        &[
            "--param",
            "levels=16",
            "--param",
            "u16_max=3",
            "--frames",
            "3",
        ],
        &[a, b, a].concat(),
    );
    assert_eq!(
        out,
        [[0u8, 0, 85, 85], [85, 170, 170, 255], [0, 0, 85, 85]].concat()
    );
}

// -- the arguments ---------------------------------------------------------------------------

#[test]
fn every_value_of_every_parameter_is_taken() {
    let frame = [9u8, 99, 199, 250];
    for word in [
        "levels=8",
        "levels=16",
        "u16_max=1",
        "u16_max=65535",
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
    ] {
        let out = frames(STAGE, ["1", "1", "8"], &params(&[word]), &frame);
        assert_eq!(out.len(), frame.len(), "{word}");
    }
}

#[test]
fn parameter_refusals_follow_the_convention() {
    let frame = [9u8, 99, 199, 250];
    let size = ["1", "1", "8"];
    let cases: [(&[&str], i32, &str); 12] = [
        (&["depth=8"], 3, "error: unsupported: depth-quantise has no parameter `depth`: it takes levels, u16_max, rounding, precision, clamp"),
        (&["levels=32"], 3, "error: unsupported: bad value for `levels`: `32` is not one of 8, 16"),
        (&["levels=10"], 3, "error: unsupported: bad value for `levels`: `10` is not one of 8, 16"),
        (&["u16_max=0"], 3, "error: unsupported: bad value for `u16_max`: 0 is outside 1 to 65535"),
        (&["u16_max=65536"], 3, "error: unsupported: bad value for `u16_max`: 65536 is outside 1 to 65535"),
        (&["u16_max=full"], 3, "error: unsupported: bad value for `u16_max`: cannot parse `full` as a whole number"),
        (&["u16_max=1.5"], 3, "error: unsupported: bad value for `u16_max`: cannot parse `1.5` as a whole number"),
        (&["u16_max=100@0"], 3, "error: unsupported: bad value for `u16_max`: a keyed value is not taken on the command line"),
        (&["rounding=nearest"], 3, "error: unsupported: bad value for `rounding`: `nearest` is not one of half-up, ties-even, truncate, floor, ceil"),
        (&["precision=f16"], 3, "error: unsupported: bad value for `precision`: `f16` is not one of f32, f64"),
        (&["clamp=wrap"], 3, "error: unsupported: bad value for `clamp`: `wrap` is not one of none, range, range-nan-zero"),
        (&["levels=8", "levels=16"], 2, "error: usage: parameter `levels` given more than once"),
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
        "error: unsupported: depth-quantise takes no layers: `keys` is not one",
    );
}

#[test]
fn the_shared_refusals_hold_for_this_stage() {
    let frame = [0u8, 42, 43, 100];
    let size = ["1", "1", "8"];
    refused(
        STAGE,
        ["1", "1", "12"],
        &[],
        &frame,
        3,
        "error: unsupported: depth 12 is not supported: depth-quantise takes 8, 16 or 32",
    );
    refused(
        STAGE,
        size,
        &["--levels", "8"],
        &frame,
        2,
        "error: usage: unknown option `--levels`",
    );
    let partial = run(
        STAGE,
        size,
        &params(&["levels=16", "u16_max=3"]),
        &[&frame[..], &frame[..2]].concat(),
    );
    assert_eq!(partial.code, 4);
    assert!(partial.err.starts_with(
        "error: partial_frame: frame 1: the input ends inside a frame (2 of 4 bytes) after 1 whole frames"
    ));
    assert_eq!(partial.out, [0, 0, 85, 85]);
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
    let input = [0u8, 42, 43, 100].repeat(2000);
    let (code, err) = reader_leaves(STAGE, ["1", "1", "8"], &params(&["levels=16"]), &input);
    assert_eq!(code, 4, "{err}");
    assert!(err.starts_with("error: broken_pipe: frame "), "{err}");
    assert!(err.contains(": cannot write the output: "), "{err}");
    assert_eq!(err.matches('\n').count(), 1, "{err}");
}
