//! `soft-edge-blur` on the built program: its manifest entry, its arguments, every refusal, and
//! what it writes for made up frames, against values worked out by hand and against the
//! `soft-edge` crate called directly.

mod image_support;

use image_support::{
    alpha8, frames, manifest_entry, param_ids, params, planar32, reader_leaves, refused, rgba16,
    rgba8, run, with_alpha8, Scratch,
};
use soft_edge::{Config, GaussSpec};
use stage_cli::pixels;
use stage_cli::registry::GENERIC_CLAIM;

const STAGE: &str = "soft-edge-blur";

/// The alpha samples of a 5 by 1 frame with one full pixel in the middle.
const SPIKE: [u8; 5] = [0, 0, 255, 0, 0];

// -- the manifest entry and the describe answer ----------------------------------------------

#[test]
fn the_stage_is_listed_as_generic_with_its_parameters_and_its_layer() {
    let entry = manifest_entry(STAGE);
    assert!(entry.starts_with(
        "{\"id\":\"soft-edge-blur\",\"kind\":\"frame\",\"spool\":false,\"geometry\":\"preserve\",\"depths\":[8,16,32],\"params\":["
    ));
    assert_eq!(
        param_ids(&entry),
        [
            "channels",
            "method",
            "radius",
            "passes",
            "gauss_spec",
            "border",
            "frac_rule",
            "trunc_k",
            "exp_method",
            "exp_terms",
            "radius_to_sigma"
        ]
    );
    assert!(entry.contains(
        "{\"id\":\"radius\",\"name\":\"Radius\",\"kind\":\"float\",\"default\":0,\"min\":0,\"max\":4096,\"keyed\":true,"
    ));
    assert!(entry.contains(
        "{\"id\":\"channels\",\"name\":\"Channels\",\"kind\":\"popup\",\"default\":\"alpha\",\"choices\":[\"alpha\",\"all\"],\"keyed\":false,"
    ));
    assert!(entry.contains(
        "{\"id\":\"passes\",\"name\":\"Passes\",\"kind\":\"int\",\"default\":3,\"min\":1,\"max\":16,\"keyed\":false,"
    ));
    assert_eq!(entry.matches("\"keyed\":true").count(), 1);
    assert!(entry.contains("\"layers\":[{\"id\":\"keys\",\"optional\":true,\"form\":\"keys-1\","));
    assert!(entry.contains("\"name\":\"Soft edge blur\""));
    assert!(entry.contains(&format!("{GENERIC_CLAIM}\",\"family\":\"generic\"}}")));
}

#[test]
fn describe_answers_the_input_geometry_and_reads_nothing() {
    for depth in ["8", "16", "32"] {
        let answer = run(
            STAGE,
            ["7", "4", depth],
            &["--describe", "--frames", "9", "--param", "radius=2.5"],
            b"",
        );
        assert_eq!((answer.code, answer.err.as_str()), (0, ""));
        assert_eq!(
            String::from_utf8(answer.out).unwrap(),
            format!(
                "{{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"soft-edge-blur\",\"width\":7,\"height\":4,\"depth\":{depth},\"fps\":\"24/1\",\"frames\":9}}\n"
            )
        );
    }
}

// -- what the stage computes -----------------------------------------------------------------

#[test]
fn the_defaults_return_the_input() {
    let input = rgba8(&[[1, 2, 3, 4], [250, 128, 0, 255], [9, 9, 9, 0]]);
    assert_eq!(frames(STAGE, ["3", "1", "8"], &[], &input), input);
    let wide = rgba16(&[[1, 2, 3, 4], [65535, 32768, 0, 65535]]);
    assert_eq!(frames(STAGE, ["2", "1", "16"], &[], &wide), wide);
}

#[test]
fn a_box_of_radius_one_spreads_a_spike_over_three_pixels() {
    // rows: the kernel is 1/3, 1/3, 1/3, so the three middle pixels get 255 / 3 = 85.
    // columns: the frame has one row and the border repeats it, so the column pass adds three
    // thirds of the same value. Colour passes.
    let input = with_alpha8(&SPIKE);
    let out = frames(STAGE, ["5", "1", "8"], &params(&["radius=1"]), &input);
    assert_eq!(out, with_alpha8(&[0, 85, 85, 85, 0]));
}

#[test]
fn a_box_on_a_three_by_three_frame_spreads_the_centre_evenly() {
    // one full pixel in the middle of nine: rows give 1/3 on the middle row, columns give 1/9
    // everywhere (the border repeats the edge rows). 255 / 9 = 28.33, written as 28.
    let mut alpha = [0u8; 9];
    alpha[4] = 255;
    let out = frames(
        STAGE,
        ["3", "3", "8"],
        &params(&["radius=1"]),
        &with_alpha8(&alpha),
    );
    assert_eq!(out, with_alpha8(&[28; 9]));
}

#[test]
fn the_zero_border_reads_nothing_outside_the_frame() {
    // rows as before: 0, 85, 85, 85, 0. columns: the two taps outside the one row read zero, so
    // one third of each value stays: 85 / 3 = 28.33, written as 28.
    let out = frames(
        STAGE,
        ["5", "1", "8"],
        &params(&["radius=1", "border=zero"]),
        &with_alpha8(&SPIKE),
    );
    assert_eq!(out, with_alpha8(&[0, 28, 28, 28, 0]));
}

#[test]
fn the_mirror_borders_differ_at_the_edge() {
    // a spike at the left edge, radius 1, one row (the column pass keeps the row under both
    // mirror rules: every tap reads the one row).
    // mirror: index -1 reads index 0, so pixel 0 gets two thirds: 170; pixel 1 one third: 85.
    // mirror-no-repeat: index -1 reads index 1, so pixel 0 gets one third: 85; pixel 1: 85.
    let input = with_alpha8(&[255, 0, 0, 0]);
    let size = ["4", "1", "8"];
    let mirror = frames(STAGE, size, &params(&["radius=1", "border=mirror"]), &input);
    assert_eq!(alpha8(&mirror), [170, 85, 0, 0]);
    let no_repeat = frames(
        STAGE,
        size,
        &params(&["radius=1", "border=mirror-no-repeat"]),
        &input,
    );
    assert_eq!(alpha8(&no_repeat), [85, 85, 0, 0]);
    let clamp = frames(STAGE, size, &params(&["radius=1", "border=clamp"]), &input);
    assert_eq!(alpha8(&clamp), [170, 85, 0, 0]);
}

#[test]
fn a_fractional_radius_follows_the_rule_chosen() {
    // radius 0.5 under end-taps: weights 0.5, 1, 0.5 over the sum 2, so 0.25, 0.5, 0.25.
    // 255 * 0.25 = 63.75, written as 64; 255 * 0.5 = 127.5, written as 128.
    let out = frames(
        STAGE,
        ["5", "1", "8"],
        &params(&["radius=0.5", "frac_rule=end-taps"]),
        &with_alpha8(&SPIKE),
    );
    assert_eq!(alpha8(&out), [0, 64, 128, 64, 0]);
    // radius 0.5 under lerp: half of the one tap box and half of the three tap box, so
    // 1/6, 2/3, 1/6. With a spike of 240: 40, 160, 40.
    let out = frames(
        STAGE,
        ["5", "1", "8"],
        &params(&["radius=0.5", "frac_rule=lerp"]),
        &with_alpha8(&[0, 0, 240, 0, 0]),
    );
    assert_eq!(alpha8(&out), [0, 40, 160, 40, 0]);
}

#[test]
fn all_channels_are_blurred_when_asked() {
    // nine pixels, the middle one is 90, 180, 45, 255: every channel becomes a ninth of it.
    let mut pixels = [[0u8; 4]; 9];
    pixels[4] = [90, 180, 45, 255];
    let input = rgba8(&pixels);
    let all = frames(
        STAGE,
        ["3", "3", "8"],
        &params(&["radius=1", "channels=all"]),
        &input,
    );
    assert_eq!(all, rgba8(&[[10, 20, 5, 28]; 9]));
    let alpha_only = frames(
        STAGE,
        ["3", "3", "8"],
        &params(&["radius=1", "channels=alpha"]),
        &input,
    );
    let mut want = pixels;
    for p in &mut want {
        p[3] = 28;
    }
    assert_eq!(alpha_only, rgba8(&want));
}

/// The alpha channel of a made up 4 by 3 frame at depth 16, blurred by the crate directly.
fn by_the_crate(
    alpha: &[u16],
    blur: impl Fn(&[f32]) -> Result<Vec<f32>, soft_edge::Error>,
) -> Vec<u16> {
    let bytes: Vec<u8> = alpha.iter().flat_map(|v| v.to_le_bytes()).collect();
    let made = blur(&pixels::decode(&bytes, 16)).unwrap();
    let mut out = Vec::new();
    pixels::encode(&made, 16, &mut out);
    out.as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect()
}

const ALPHA16: [u16; 12] = [
    0, 65535, 1200, 40000, 512, 0, 65535, 65535, 30000, 7, 0, 22222,
];

fn frame16(alpha: &[u16]) -> Vec<u8> {
    let pixels: Vec<[u16; 4]> = alpha.iter().map(|a| [100, 2000, 60000, *a]).collect();
    rgba16(&pixels)
}

#[test]
fn the_repeated_box_is_the_function_of_the_crate() {
    let cfg = Config::default();
    for passes in [1u32, 3, 16] {
        let passes_text = format!("passes={passes}");
        let out = frames(
            STAGE,
            ["4", "3", "16"],
            &params(&["method=box-repeated", "radius=1.25", &passes_text]),
            &frame16(&ALPHA16),
        );
        let want = by_the_crate(&ALPHA16, |plane| {
            soft_edge::box_blur_repeated(plane, 4, 3, 1.25, passes, &cfg)
        });
        assert_eq!(out, frame16(&want), "passes {passes}");
    }
    // one pass of the repeated box is the box
    let one = frames(
        STAGE,
        ["4", "3", "16"],
        &params(&["method=box-repeated", "radius=1.25", "passes=1"]),
        &frame16(&ALPHA16),
    );
    let boxed = frames(
        STAGE,
        ["4", "3", "16"],
        &params(&["method=box", "radius=1.25"]),
        &frame16(&ALPHA16),
    );
    assert_eq!(one, boxed);
}

#[test]
fn the_gaussian_is_the_function_of_the_crate_under_every_setting() {
    let base = Config::default();
    let cases: [(&[&str], GaussSpec, Config); 6] = [
        (&["radius=0.8"], GaussSpec::Sigma(0.8), base),
        (
            &["radius=1.6", "gauss_spec=radius"],
            GaussSpec::Radius(1.6),
            base,
        ),
        (
            &["radius=1.6", "gauss_spec=radius", "radius_to_sigma=4"],
            GaussSpec::Radius(1.6),
            Config {
                radius_to_sigma: 4.0,
                ..base
            },
        ),
        (
            &["radius=0.8", "trunc_k=1.5"],
            GaussSpec::Sigma(0.8),
            Config {
                trunc_k: 1.5,
                ..base
            },
        ),
        (
            &["radius=0.8", "exp_method=recurrence", "exp_terms=3"],
            GaussSpec::Sigma(0.8),
            Config {
                exp_method: soft_edge::ExpMethod::Recurrence,
                exp_terms: 3,
                ..base
            },
        ),
        (
            &["radius=0.8", "exp_terms=2", "border=zero"],
            GaussSpec::Sigma(0.8),
            Config {
                exp_terms: 2,
                border: soft_edge::Border::Zero,
                ..base
            },
        ),
    ];
    let mut seen = Vec::new();
    for (words, spec, cfg) in cases {
        let mut all = vec!["method=gaussian"];
        all.extend_from_slice(words);
        let out = frames(STAGE, ["4", "3", "16"], &params(&all), &frame16(&ALPHA16));
        let want = by_the_crate(&ALPHA16, |plane| {
            soft_edge::gaussian_blur(plane, 4, 3, spec, &cfg)
        });
        assert_eq!(out, frame16(&want), "{words:?}");
        seen.push(out);
    }
    // a radius of 1.6 at the default divisor 2 is the sigma 0.8
    assert_eq!(seen[0], seen[1]);
    // the other settings are not dead: each changes the frame
    for (i, frame) in seen.iter().enumerate().skip(2) {
        assert_ne!(frame, &seen[0], "case {i}");
    }
}

#[test]
fn a_float_frame_keeps_its_colour_planes_bit_for_bit() {
    // depth 32: planes G, B, R, A. Colour holds values outside 0 to 1 and a negative zero.
    let px = [
        [2.5f32, -0.0, 0.25, 0.0],
        [-7.0, 1.0e-20, 0.5, 1.0],
        [0.125, 3.0, 0.75, 0.0],
    ];
    let input = planar32(&px);
    let out = frames(STAGE, ["3", "1", "32"], &params(&["radius=1"]), &input);
    assert_eq!(out[..36], input[..36], "the three colour planes pass");
    let alpha: Vec<f32> = out[36..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let want = soft_edge::box_blur(&[0.0, 1.0, 0.0], 3, 1, 1.0, &Config::default()).unwrap();
    assert_eq!(alpha, want);
    // float samples are not clamped: a blurred value above 1 stays above 1
    let hot = planar32(&[[0.0, 0.0, 0.0, 3.0]]);
    let out = frames(STAGE, ["1", "1", "32"], &params(&["radius=1"]), &hot);
    let a = f32::from_le_bytes([out[12], out[13], out[14], out[15]]);
    assert!(a > 2.9, "{a}");
}

#[test]
fn frames_are_processed_one_by_one() {
    let a = with_alpha8(&SPIKE);
    let b = with_alpha8(&[255, 0, 0, 0, 0]);
    let input = [a.clone(), b.clone(), a.clone()].concat();
    let out = frames(
        STAGE,
        ["5", "1", "8"],
        &["--param", "radius=1", "--frames", "3"],
        &input,
    );
    let one = |frame: &[u8]| frames(STAGE, ["5", "1", "8"], &params(&["radius=1"]), frame);
    assert_eq!(out, [one(&a), one(&b), one(&a)].concat());
}

// -- the key file ----------------------------------------------------------------------------

#[test]
fn a_key_file_moves_the_radius_from_frame_to_frame() {
    let frame = with_alpha8(&SPIKE);
    let blurred = with_alpha8(&[0, 85, 85, 85, 0]);
    let half = with_alpha8(&[0, 64, 128, 64, 0]);
    let keys = Scratch::new(
        "blur-moves",
        "keys-1\nkey radius 0 0 linear\nkey radius 2 1 hold\n",
    );
    let layer = keys.layer();
    let out = frames(
        STAGE,
        ["5", "1", "8"],
        &["--layer", layer.as_str()],
        &[frame.clone(), frame.clone(), frame.clone(), frame.clone()].concat(),
    );
    // radius 0, 0.5, 1, then the last key holds
    assert_eq!(out, [frame, half, blurred.clone(), blurred].concat());
}

#[test]
fn key_file_refusals() {
    let frame = with_alpha8(&SPIKE);
    let size = ["5", "1", "8"];
    refused(
        STAGE,
        size,
        &["--layer", "keys=/no/such/folder/keys.txt"],
        &frame,
        4,
        "error: bad_input: cannot read the key file `/no/such/folder/keys.txt`: ",
    );
    let malformed = Scratch::new("blur-malformed", "keys-2\nkey radius 0 1 hold\n");
    refused(
        STAGE,
        size,
        &["--layer", &malformed.layer()],
        &frame,
        4,
        "error: bad_input: key file `",
    );
    let not_keyed = Scratch::new("blur-not-keyed", "keys-1\nkey passes 0 2 hold\n");
    refused(
        STAGE,
        size,
        &["--layer", &not_keyed.layer()],
        &frame,
        3,
        "error: unsupported: key file `",
    );
    let outside = Scratch::new("blur-outside", "keys-1\nkey radius 0 5000 hold\n");
    refused(
        STAGE,
        size,
        &["--layer", &outside.layer()],
        &frame,
        3,
        "error: unsupported: key file `",
    );
    refused(
        STAGE,
        size,
        &["--layer", "keys=a", "--layer", "keys=b"],
        &frame,
        2,
        "error: usage: layer `keys` given more than once",
    );
    refused(
        STAGE,
        size,
        &["--layer", "keys="],
        &frame,
        2,
        "error: usage: layer `keys` has an empty path",
    );
    refused(
        STAGE,
        size,
        &["--layer", "source=x"],
        &frame,
        3,
        "error: unsupported: soft-edge-blur has no layer `source`: it takes keys",
    );
}

#[test]
fn a_keyed_radius_the_crate_refuses_ends_the_run_at_that_frame() {
    // the command line values pass the check; the key at frame 1 asks for a Gaussian kernel of
    // more than 2^20 taps per side (sigma 4096 / 0.001), which the crate refuses.
    let frame = with_alpha8(&SPIKE);
    let keys = Scratch::new(
        "blur-late",
        "keys-1\nkey radius 0 0 hold\nkey radius 1 4096 hold\n",
    );
    let layer = keys.layer();
    let run = run(
        STAGE,
        ["5", "1", "8"],
        &[
            "--param",
            "method=gaussian",
            "--param",
            "gauss_spec=radius",
            "--param",
            "radius_to_sigma=0.001",
            "--layer",
            layer.as_str(),
        ],
        &[frame.clone(), frame.clone()].concat(),
    );
    assert_eq!(run.code, 3, "{}", run.err);
    assert_eq!(
        run.err,
        "error: unsupported: frame 1: soft-edge-blur refuses these values: kernel half width above 2^20\n"
    );
    assert_eq!(run.out, frame, "frame 0 was written before the refusal");
}

// -- the arguments ---------------------------------------------------------------------------

#[test]
fn every_value_of_every_parameter_is_taken() {
    let frame = with_alpha8(&SPIKE);
    for word in [
        "channels=alpha",
        "channels=all",
        "method=box",
        "method=box-repeated",
        "method=gaussian",
        "radius=0",
        "radius=4096",
        "radius=1e1",
        "passes=1",
        "passes=16",
        "gauss_spec=sigma",
        "gauss_spec=radius",
        "border=zero",
        "border=clamp",
        "border=mirror",
        "border=mirror-no-repeat",
        "frac_rule=end-taps",
        "frac_rule=lerp",
        "trunc_k=0",
        "trunc_k=64",
        "exp_method=series",
        "exp_method=recurrence",
        "exp_terms=1",
        "exp_terms=64",
        "radius_to_sigma=0",
        "radius_to_sigma=4096",
    ] {
        let out = frames(STAGE, ["5", "1", "8"], &params(&[word]), &frame);
        assert_eq!(out.len(), frame.len(), "{word}");
    }
}

#[test]
fn parameter_refusals_follow_the_convention() {
    let frame = with_alpha8(&SPIKE);
    let size = ["5", "1", "8"];
    let cases: [(&[&str], i32, &str); 16] = [
        (&["blur=1"], 3, "error: unsupported: soft-edge-blur has no parameter `blur`: it takes channels, method, radius, passes, gauss_spec, border, frac_rule, trunc_k, exp_method, exp_terms, radius_to_sigma"),
        (&["channels=red"], 3, "error: unsupported: bad value for `channels`: `red` is not one of alpha, all"),
        (&["method=median"], 3, "error: unsupported: bad value for `method`: `median` is not one of box, box-repeated, gaussian"),
        (&["radius=-1"], 3, "error: unsupported: bad value for `radius`: -1 is outside 0 to 4096"),
        (&["radius=4096.5"], 3, "error: unsupported: bad value for `radius`: 4096.5 is outside 0 to 4096"),
        (&["radius=wide"], 3, "error: unsupported: bad value for `radius`: cannot parse `wide` as a number"),
        (&["radius=inf"], 3, "error: unsupported: bad value for `radius`: cannot parse `inf` as a number"),
        (&["radius=1@0;2@5"], 3, "error: unsupported: bad value for `radius`: a keyed value is not taken on the command line"),
        (&["passes=0"], 3, "error: unsupported: bad value for `passes`: 0 is outside 1 to 16"),
        (&["passes=17"], 3, "error: unsupported: bad value for `passes`: 17 is outside 1 to 16"),
        (&["passes=2.5"], 3, "error: unsupported: bad value for `passes`: cannot parse `2.5` as a whole number"),
        (&["gauss_spec=fwhm"], 3, "error: unsupported: bad value for `gauss_spec`: `fwhm` is not one of sigma, radius"),
        (&["border=repeat"], 3, "error: unsupported: bad value for `border`: `repeat` is not one of zero, clamp, mirror, mirror-no-repeat"),
        (&["frac_rule=round"], 3, "error: unsupported: bad value for `frac_rule`: `round` is not one of end-taps, lerp"),
        (&["exp_terms=0"], 3, "error: unsupported: bad value for `exp_terms`: 0 is outside 1 to 64"),
        (&["radius=1", "radius=2"], 2, "error: usage: parameter `radius` given more than once"),
    ];
    for (words, code, line) in cases {
        refused(STAGE, size, &params(words), &frame, code, line);
    }
    refused(
        STAGE,
        size,
        &params(&["trunc_k=65"]),
        &frame,
        3,
        "error: unsupported: bad value for `trunc_k`: 65 is outside 0 to 64",
    );
    refused(
        STAGE,
        size,
        &params(&["exp_method=table"]),
        &frame,
        3,
        "error: unsupported: bad value for `exp_method`: `table` is not one of series, recurrence",
    );
    refused(
        STAGE,
        size,
        &params(&["radius_to_sigma=5000"]),
        &frame,
        3,
        "error: unsupported: bad value for `radius_to_sigma`: 5000 is outside 0 to 4096",
    );
}

#[test]
fn values_the_crate_refuses_are_unsupported_before_a_frame_is_read() {
    let frame = with_alpha8(&SPIKE);
    let size = ["5", "1", "8"];
    let cases: [(&[&str], &str); 3] = [
        (
            &["method=gaussian", "radius=1", "trunc_k=0"],
            "error: unsupported: soft-edge-blur refuses these values: trunc_k must be finite and positive\n",
        ),
        (
            &["method=gaussian", "gauss_spec=radius", "radius=1", "radius_to_sigma=0"],
            "error: unsupported: soft-edge-blur refuses these values: radius_to_sigma must be finite and positive\n",
        ),
        (
            &["method=gaussian", "gauss_spec=radius", "radius=4096", "radius_to_sigma=0.001"],
            "error: unsupported: soft-edge-blur refuses these values: kernel half width above 2^20\n",
        ),
    ];
    for (words, line) in cases {
        refused(STAGE, size, &params(words), &frame, 3, line);
        // the describe answer is refused in the same way
        let mut describe = params(words);
        describe.push("--describe");
        refused(STAGE, size, &describe, b"", 3, line);
    }
}

#[test]
fn the_shared_refusals_hold_for_this_stage() {
    let frame = with_alpha8(&SPIKE);
    let size = ["5", "1", "8"];
    refused(
        STAGE,
        ["5", "1", "12"],
        &[],
        &frame,
        3,
        "error: unsupported: depth 12 is not supported: soft-edge-blur takes 8, 16 or 32",
    );
    refused(
        STAGE,
        size,
        &["--radius", "1"],
        &frame,
        2,
        "error: usage: unknown option `--radius`",
    );
    let partial = run(
        STAGE,
        size,
        &params(&["radius=1"]),
        &[frame.clone(), frame[..7].to_vec()].concat(),
    );
    assert_eq!(partial.code, 4);
    assert!(partial.err.starts_with("error: partial_frame: frame 1: the input ends inside a frame (7 of 20 bytes) after 1 whole frames"));
    assert_eq!(
        partial.out,
        with_alpha8(&[0, 85, 85, 85, 0]),
        "the whole frame before it was written"
    );
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
    let input = with_alpha8(&SPIKE).repeat(2000);
    let (code, err) = reader_leaves(STAGE, ["5", "1", "8"], &params(&["radius=1"]), &input);
    assert_eq!(code, 4, "{err}");
    assert!(err.starts_with("error: broken_pipe: frame "), "{err}");
    assert!(err.contains(": cannot write the output: "), "{err}");
    assert_eq!(err.matches('\n').count(), 1, "{err}");
}
