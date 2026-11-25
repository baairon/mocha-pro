//! `raster-polygon` on the built program: what it computes on small made up frames, against
//! values worked out by hand and against the `coverage-raster` crate called directly, and every
//! refusal of its own. Every frame, every shape and every key is made up.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use coverage_raster::{
    rasterise_buffer, Buffer, Combine, Config, Depth, FillRule, Method, Pattern, Polygon,
    Precision, Quantise, Rounding, Scale, Target,
};
use stage_cli::registry::GENERIC_CLAIM;

const STAGE: &str = "raster-polygon";

struct Run {
    code: i32,
    out: Vec<u8>,
    err: String,
}

fn spawn(args: &[&str], input: &[u8]) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mocha-stage"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let data = input.to_vec();
    let feeder = std::thread::spawn(move || {
        // the stage may exit before it reads everything: ignore a closed pipe
        let _ = stdin.write_all(&data);
    });
    let out = child.wait_with_output().expect("wait");
    feeder.join().unwrap();
    Run {
        code: out.status.code().expect("exit code"),
        out: out.stdout,
        err: String::from_utf8(out.stderr).unwrap(),
    }
}

/// A file with made up content in the temporary folder, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(content: &str) -> Scratch {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "stage-cli-raster-polygon-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, content).expect("write the scratch file");
        Scratch(path)
    }

    fn layer(&self, id: &str) -> String {
        format!("{id}={}", self.0.display())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// A `rings-1` text of polygons with one ring each.
fn rings(polygons: &[&[(f64, f64)]]) -> String {
    let mut text = String::from("rings-1\n");
    for ring in polygons {
        text.push_str("polygon\nring\n");
        for (x, y) in *ring {
            text.push_str(&format!("point {x} {y}\n"));
        }
    }
    text
}

/// The ring of the rectangle from `(x0, y0)` to `(x1, y1)`.
fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> [(f64, f64); 4] {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

/// A depth 8 frame: pixel `k` is R `k + 1`, G 100, B 200 and the alpha given.
fn frame8(alphas: &[u8]) -> Vec<u8> {
    alphas
        .iter()
        .enumerate()
        .flat_map(|(k, a)| [k as u8 + 1, 100, 200, *a])
        .collect()
}

fn alpha8(frame: &[u8]) -> Vec<u8> {
    frame.as_chunks::<4>().0.iter().map(|p| p[3]).collect()
}

/// Run the stage at depth 8 on `frames` copies of a frame whose alpha is 77 everywhere, and
/// return the alpha of every output frame after checking that colour passed.
fn matte8(size: (usize, usize), shape: &str, params: &[&str], frames: usize) -> Vec<Vec<u8>> {
    let file = Scratch::new(shape);
    let layer = file.layer("shape");
    let (w, h) = (size.0.to_string(), size.1.to_string());
    let mut args = vec![
        "apply", STAGE, "--width", &w, "--height", &h, "--depth", "8", "--fps", "24/1", "--layer",
        &layer,
    ];
    for p in params {
        args.extend(["--param", p]);
    }
    let one = frame8(&vec![77; size.0 * size.1]);
    let run = spawn(&args, &one.repeat(frames));
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{params:?}");
    assert_eq!(run.out.len(), one.len() * frames);
    run.out
        .chunks_exact(one.len())
        .map(|frame| {
            for (made, read) in frame.as_chunks::<4>().0.iter().zip(one.as_chunks::<4>().0) {
                assert_eq!(made[..3], read[..3], "colour passes");
            }
            alpha8(frame)
        })
        .collect()
}

fn one8(size: (usize, usize), shape: &str, params: &[&str]) -> Vec<u8> {
    matte8(size, shape, params, 1).remove(0)
}

// -- what the stage computes, by hand -----------------------------------------------------------

#[test]
fn a_rectangle_on_pixel_edges_is_full_coverage_in_every_frame() {
    let shape = rings(&[&rect(1.0, 1.0, 3.0, 2.0)]);
    let frames = matte8((4, 3), &shape, &[], 3);
    for alpha in frames {
        assert_eq!(alpha, [0, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0, 0]);
    }
}

/// Four pixels in a row with the coverages 5/512, 3/512, 1 and 1/2.
fn fractions() -> String {
    rings(&[
        &rect(0.0, 0.0, 5.0 / 512.0, 1.0),
        &rect(1.0, 0.0, 1.0 + 3.0 / 512.0, 1.0),
        &rect(2.0, 0.0, 3.0, 1.0),
        &rect(3.0, 0.0, 3.5, 1.0),
    ])
}

#[test]
fn fractional_coverage_is_quantised_as_the_parameters_say() {
    // scale 255: 5/512 is 2.49, 3/512 is 1.49, 1/2 is 127.5
    // scale 256: 5/512 is 2.5, 3/512 is 1.5, 1 is 256 and clamps, 1/2 is 128
    let cases: [(&[&str], [u8; 4]); 9] = [
        (&[], [2, 1, 255, 128]),
        (&["quantise_rounding=nearest-even"], [2, 1, 255, 128]),
        (&["quantise_rounding=floor"], [2, 1, 255, 127]),
        (&["quantise_rounding=ceil"], [3, 2, 255, 128]),
        (&["quantise_scale=power-of-two"], [3, 2, 255, 128]),
        (
            &[
                "quantise_scale=power-of-two",
                "quantise_rounding=nearest-even",
            ],
            [2, 2, 255, 128],
        ),
        (
            &["quantise_scale=power-of-two", "quantise_rounding=floor"],
            [2, 1, 255, 128],
        ),
        (
            &["quantise_scale=power-of-two", "quantise_rounding=ceil"],
            [3, 2, 255, 128],
        ),
        // off: the scale and the rounding are not read, and the sample rule is
        // floor(x * 255 + 0.5)
        (
            &[
                "quantise=off",
                "quantise_scale=power-of-two",
                "quantise_rounding=ceil",
            ],
            [2, 1, 255, 128],
        ),
    ];
    for (params, want) in cases {
        assert_eq!(one8((4, 1), &fractions(), params), want, "{params:?}");
    }
}

fn run_depth(depth: &str, size: (&str, &str), shape: &str, params: &[&str], input: &[u8]) -> Run {
    let file = Scratch::new(shape);
    let layer = file.layer("shape");
    let mut args = vec![
        "apply", STAGE, "--width", size.0, "--height", size.1, "--depth", depth, "--fps", "24/1",
        "--layer", &layer,
    ];
    for p in params {
        args.extend(["--param", p]);
    }
    spawn(&args, input)
}

#[test]
fn sixteen_bit_frames_take_sixteen_bit_codes() {
    // pixel k: R k, G 1000, B 2000, A 30000
    let input: Vec<u8> = (0..4u16)
        .flat_map(|k| [k, 1000, 2000, 30000])
        .flat_map(u16::to_le_bytes)
        .collect();
    let run = run_depth("16", ("4", "1"), &fractions(), &[], &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    let made: Vec<u16> = run
        .out
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    // 5/512 * 65535 is 639.99, 3/512 * 65535 is 383.99, 1/2 * 65535 is 32767.5
    let want: Vec<u16> = [640u16, 384, 65535, 32768]
        .iter()
        .enumerate()
        .flat_map(|(k, a)| [k as u16, 1000, 2000, *a])
        .collect();
    assert_eq!(made, want);
    let floor = run_depth(
        "16",
        ("4", "1"),
        &fractions(),
        &["quantise_rounding=floor"],
        &input,
    );
    let alpha: Vec<u16> = floor
        .out
        .as_chunks::<8>()
        .0
        .iter()
        .map(|p| u16::from_le_bytes([p[6], p[7]]))
        .collect();
    assert_eq!(alpha, [639, 383, 65535, 32767]);
}

#[test]
fn float_frames_take_the_coverage_itself_in_the_alpha_plane() {
    // the planes G, B, R, A of four pixels; colour holds values outside 0 to 1 on purpose
    let planes: [f32; 16] = [
        0.1, 0.2, 0.3, 0.4, // G
        -1.0, 2.5, 0.0, 1.0, // B
        7.0, 8.0, 9.0, 10.0, // R
        0.9, 0.9, 0.9, 0.9, // A
    ];
    let input: Vec<u8> = planes.iter().flat_map(|v| v.to_le_bytes()).collect();
    for params in [
        &[][..],
        &["quantise=off"][..],
        &["quantise_scale=power-of-two", "quantise_rounding=floor"][..],
    ] {
        let run = run_depth("32", ("4", "1"), &fractions(), params, &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""));
        let made: Vec<f32> = run
            .out
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        assert_eq!(made[..12], planes[..12], "the colour planes pass");
        assert_eq!(made[12..], [5.0 / 512.0, 3.0 / 512.0, 1.0, 0.5]);
    }
}

#[test]
fn carry_says_which_channels_hold_the_matte() {
    // two pixels with the coverages 1/2 and 0
    let shape = rings(&[&rect(0.0, 0.0, 0.5, 1.0)]);
    let input = [1, 2, 3, 77, 4, 5, 6, 77];
    let cases: [(&[&str], [u8; 8]); 4] = [
        (&[], [1, 2, 3, 128, 4, 5, 6, 0]),
        (&["carry=alpha"], [1, 2, 3, 128, 4, 5, 6, 0]),
        (&["carry=luma"], [128, 128, 128, 255, 0, 0, 0, 255]),
        (&["carry=all"], [128, 128, 128, 128, 0, 0, 0, 0]),
    ];
    for (params, want) in cases {
        let run = run_depth("8", ("2", "1"), &shape, params, &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""), "{params:?}");
        assert_eq!(run.out, want, "{params:?}");
    }
}

#[test]
fn apply_says_how_the_coverage_meets_the_alpha_of_the_input() {
    // three pixels with the coverages 1/2, 1 and 0 over the alphas 128, 51 and 200
    let shape = rings(&[&rect(0.0, 0.0, 0.5, 1.0), &rect(1.0, 0.0, 2.0, 1.0)]);
    let input = frame8(&[128, 51, 200]);
    let cases: [(&[&str], [u8; 3]); 5] = [
        (&["apply=replace"], [128, 255, 0]),
        // 128/255 * 128/255 * 255 is 64.25; 51/255 * 1 * 255 is 51
        (&["apply=multiply"], [64, 51, 0]),
        (&["apply=max"], [128, 255, 200]),
        (&["apply=multiply", "carry=luma"], [255, 255, 255]),
        (&["apply=max", "carry=all"], [128, 255, 200]),
    ];
    for (params, want) in cases {
        let run = run_depth("8", ("3", "1"), &shape, params, &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""), "{params:?}");
        assert_eq!(alpha8(&run.out), want, "{params:?}");
    }
    let luma = run_depth(
        "8",
        ("3", "1"),
        &shape,
        &["apply=multiply", "carry=luma"],
        &input,
    );
    assert_eq!(
        luma.out,
        [64, 64, 64, 255, 51, 51, 51, 255, 0, 0, 0, 255],
        "the matte of multiply in the colour channels"
    );
}

#[test]
fn the_fill_rule_decides_a_ring_inside_a_ring_of_the_same_direction() {
    let shape = "rings-1\npolygon\nring\npoint 0 0\npoint 5 0\npoint 5 5\npoint 0 5\nring\npoint 1 1\npoint 4 1\npoint 4 4\npoint 1 4\n";
    assert_eq!(one8((5, 5), shape, &[]), [255; 25]);
    assert_eq!(one8((5, 5), shape, &["fill=non-zero"]), [255; 25]);
    let hole = [
        255, 255, 255, 255, 255, //
        255, 0, 0, 0, 255, //
        255, 0, 0, 0, 255, //
        255, 0, 0, 0, 255, //
        255, 255, 255, 255, 255,
    ];
    assert_eq!(one8((5, 5), shape, &["fill=even-odd"]), hole);
}

#[test]
fn the_method_decides_the_coverage_of_a_partly_covered_pixel() {
    // x from 11/16 to 37/16: 5/16 of the first and of the third pixel
    let shape = rings(&[&rect(0.6875, 0.0, 2.3125, 1.0)]);
    let cases: [(&[&str], [u8; 4]); 6] = [
        // 5/16 * 255 is 79.69
        (&["method=exact-area"], [80, 255, 80, 0]),
        // the centres 0.5 and 2.5 are outside
        (&["method=point"], [0, 255, 0, 0]),
        // samples at 0.25 and 0.75 of a pixel: one of two columns is inside
        (&["method=samples-grid", "samples=2"], [128, 255, 128, 0]),
        // samples at 0.125, 0.375, 0.625, 0.875: one of four columns is inside
        (&["method=samples-grid", "samples=4"], [64, 255, 64, 0]),
        (&["method=samples-grid"], [64, 255, 64, 0]),
        // one sample per column of a 4 by 4 grid: one of four is inside
        (&["method=samples-rotated", "samples=4"], [64, 255, 64, 0]),
    ];
    for (params, want) in cases {
        assert_eq!(one8((4, 1), &shape, params), want, "{params:?}");
    }
}

#[test]
fn y_down_says_which_row_is_the_smallest_y() {
    let shape = rings(&[&rect(0.0, 0.0, 2.0, 1.0)]);
    assert_eq!(one8((2, 2), &shape, &[]), [255, 255, 0, 0]);
    assert_eq!(one8((2, 2), &shape, &["y_down=1"]), [255, 255, 0, 0]);
    assert_eq!(one8((2, 2), &shape, &["y_down=0"]), [0, 0, 255, 255]);
    assert_eq!(one8((2, 2), &shape, &["y_down=off"]), [0, 0, 255, 255]);
}

#[test]
fn centre_says_where_a_pixel_lies() {
    let shape = rings(&[&rect(0.5, 0.5, 1.5, 1.5)]);
    // centre 0.5: the unit square straddles four pixels, a quarter of each
    assert_eq!(one8((3, 3), &shape, &[]), [64, 64, 0, 64, 64, 0, 0, 0, 0]);
    // centre 0: pixel 1, 1 is the square from 0.5 to 1.5
    assert_eq!(
        one8((3, 3), &shape, &["centre=0"]),
        [0, 0, 0, 0, 255, 0, 0, 0, 0]
    );
    // centre 1: pixel 0, 0 is the square from 0.5 to 1.5
    assert_eq!(
        one8((3, 3), &shape, &["centre=1"]),
        [255, 0, 0, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn combine_says_how_two_polygons_meet() {
    // one pixel: the first polygon covers 1/2 of it, the second 1/4
    let shape = rings(&[&rect(0.0, 0.0, 0.5, 1.0), &rect(0.0, 0.0, 1.0, 0.25)]);
    let cases = [
        ("combine=max", 128),       // 1/2
        ("combine=add-clamp", 191), // 3/4 is 191.25
        ("combine=over", 159),      // 1/2 + 1/4 * 1/2 is 5/8, 159.375
        ("combine=replace", 64),    // 1/4 is 63.75
    ];
    for (param, want) in cases {
        assert_eq!(one8((1, 1), &shape, &[param]), [want], "{param}");
    }
    assert_eq!(one8((1, 1), &shape, &[]), [128]);
}

#[test]
fn snap_bits_rounds_coordinates_first() {
    // the left edge at 5/16: 11/16 of the pixel is covered, 175.3
    let shape = rings(&[&rect(0.3125, 0.0, 1.0, 1.0)]);
    assert_eq!(one8((1, 1), &shape, &[]), [175]);
    assert_eq!(one8((1, 1), &shape, &["snap_bits=0"]), [175]);
    // multiples of 1/2: 5/16 goes to 1/2
    assert_eq!(one8((1, 1), &shape, &["snap_bits=1"]), [128]);
    // multiples of 1/4: 5/16 goes to 1/4, 3/4 is 191.25
    assert_eq!(one8((1, 1), &shape, &["snap_bits=2"]), [191]);
    assert_eq!(one8((1, 1), &shape, &["snap_bits=30"]), [175]);
}

#[test]
fn the_offsets_move_every_point() {
    let shape = rings(&[&rect(0.0, 0.0, 1.0, 1.0)]);
    assert_eq!(one8((3, 2), &shape, &[]), [255, 0, 0, 0, 0, 0]);
    assert_eq!(
        one8((3, 2), &shape, &["offset_x=2", "offset_y=1"]),
        [0, 0, 0, 0, 0, 255]
    );
    assert_eq!(
        one8((3, 2), &shape, &["offset_x=0.5"]),
        [128, 128, 0, 0, 0, 0]
    );
    assert_eq!(one8((3, 2), &shape, &["offset_x=-1"]), [0; 6]);
}

#[test]
fn a_key_file_moves_the_polygons_from_frame_to_frame() {
    let shape = Scratch::new(&rings(&[&rect(0.0, 0.0, 1.0, 1.0)]));
    let keys = Scratch::new(
        "keys-1\nkey offset_x 0 0 linear\nkey offset_x 4 2 hold\nkey offset_y 3 0 hold\nkey offset_y 5 1 hold\n",
    );
    let (shape_layer, keys_layer) = (shape.layer("shape"), keys.layer("keys"));
    let args = [
        "apply",
        STAGE,
        "--width",
        "3",
        "--height",
        "2",
        "--depth",
        "8",
        "--fps",
        "24/1",
        "--layer",
        &shape_layer,
        "--layer",
        &keys_layer,
        "--param",
        "offset_x=1",
    ];
    let one = frame8(&[77; 6]);
    let run = spawn(&args, &one.repeat(7));
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    let made: Vec<Vec<u8>> = run.out.chunks_exact(one.len()).map(alpha8).collect();
    let want: [[u8; 6]; 7] = [
        [255, 0, 0, 0, 0, 0],   // frame 0: x 0
        [128, 128, 0, 0, 0, 0], // frame 1: x 0.5
        [0, 255, 0, 0, 0, 0],   // frame 2: x 1
        [0, 128, 128, 0, 0, 0], // frame 3: x 1.5
        [0, 0, 255, 0, 0, 0],   // frame 4: x 2, held from here
        [0, 0, 0, 0, 0, 255],   // frame 5: y 1
        [0, 0, 0, 0, 0, 255],   // frame 6: both held
    ];
    assert_eq!(made, want);
}

#[test]
fn without_a_shape_nothing_is_covered() {
    let input = frame8(&[10, 20, 30, 40]);
    let base = [
        "apply", STAGE, "--width", "2", "--height", "2", "--depth", "8", "--fps", "24/1",
    ];
    let run = spawn(&base, &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert_eq!(run.out, frame8(&[0, 0, 0, 0]));
    let mut kept = base.to_vec();
    kept.extend(["--param", "apply=max"]);
    assert_eq!(spawn(&kept, &input).out, input, "max with no coverage");
    let empty = Scratch::new("# no polygon\nrings-1\n");
    let layer = empty.layer("shape");
    let mut with_file = base.to_vec();
    with_file.extend(["--layer", &layer]);
    assert_eq!(spawn(&with_file, &input).out, frame8(&[0, 0, 0, 0]));
}

// -- against the crate called directly ----------------------------------------------------------

/// Two polygons: the first has two rings that overlap, the second overlaps both.
const SHAPE: &str = "rings-1\npolygon\nring\npoint 0.1 0.2\npoint 4.7 0.9\npoint 1.3 3.8\nring\npoint 2.2 0.4\npoint 4.9 3.6\npoint 0.3 2.9\npolygon\nring\npoint 0.5 0.5\npoint 3.5 1.5\npoint 2.5 3.9\n";

fn polygons() -> Vec<Polygon> {
    vec![
        Polygon(vec![
            vec![(0.1, 0.2), (4.7, 0.9), (1.3, 3.8)],
            vec![(2.2, 0.4), (4.9, 3.6), (0.3, 2.9)],
        ]),
        Polygon::from_ring(vec![(0.5, 0.5), (3.5, 1.5), (2.5, 3.9)]),
    ]
}

fn expected(cfg: &Config, depth: u32) -> Vec<f64> {
    match rasterise_buffer(&polygons(), &Target::new(5, 4), cfg).unwrap() {
        Buffer::U8(codes) => codes.iter().map(|c| f64::from(*c)).collect(),
        Buffer::U16(codes) => codes.iter().map(|c| f64::from(*c)).collect(),
        Buffer::F32(values) if depth == 32 => values.iter().map(|v| f64::from(*v)).collect(),
        Buffer::F32(values) => {
            let max = if depth == 8 { 255.0 } else { 65535.0 };
            values
                .iter()
                .map(|v| (f64::from(*v).clamp(0.0, 1.0) * max + 0.5).floor())
                .collect()
        }
    }
}

fn alpha_of(out: &[u8], depth: u32) -> Vec<f64> {
    match depth {
        8 => out
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| f64::from(p[3]))
            .collect(),
        16 => out
            .as_chunks::<8>()
            .0
            .iter()
            .map(|p| f64::from(u16::from_le_bytes([p[6], p[7]])))
            .collect(),
        _ => out[out.len() / 4 * 3..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])))
            .collect(),
    }
}

#[test]
fn every_parameter_reaches_the_rasteriser() {
    let canonical = Config::canonical;
    let cases: Vec<(Vec<&str>, Config)> = vec![
        (vec![], canonical()),
        (
            vec!["method=point"],
            Config {
                method: Method::Point,
                ..canonical()
            },
        ),
        (
            vec!["method=samples-grid", "samples=3"],
            Config {
                method: Method::Samples(Pattern::Grid(3)),
                ..canonical()
            },
        ),
        (
            vec!["method=samples-rotated", "samples=16"],
            Config {
                method: Method::Samples(Pattern::Rotated(16)),
                ..canonical()
            },
        ),
        (
            vec!["fill=even-odd"],
            Config {
                fill: FillRule::EvenOdd,
                ..canonical()
            },
        ),
        (
            vec!["centre=0.125"],
            Config {
                centre: 0.125,
                ..canonical()
            },
        ),
        (
            vec!["y_down=false"],
            Config {
                y_down: false,
                ..canonical()
            },
        ),
        (
            vec!["combine=add-clamp"],
            Config {
                combine: Combine::AddClamp,
                ..canonical()
            },
        ),
        (
            vec!["combine=over"],
            Config {
                combine: Combine::Over,
                ..canonical()
            },
        ),
        (
            vec!["combine=replace"],
            Config {
                combine: Combine::Replace,
                ..canonical()
            },
        ),
        (
            vec!["precision=single", "combine=over"],
            Config {
                precision: Precision::Single,
                combine: Combine::Over,
                ..canonical()
            },
        ),
        (
            vec!["snap_bits=3"],
            Config {
                snap_bits: Some(3),
                ..canonical()
            },
        ),
        (
            vec!["quantise=off"],
            Config {
                quantise: None,
                ..canonical()
            },
        ),
        (
            vec!["quantise_scale=power-of-two", "quantise_rounding=floor"],
            Config {
                quantise: Some(Quantise {
                    depth: Depth::Bits8,
                    scale: Scale::PowerOfTwo,
                    rounding: Rounding::Floor,
                }),
                ..canonical()
            },
        ),
        (
            vec!["quantise_rounding=nearest-even"],
            Config {
                quantise: Some(Quantise {
                    depth: Depth::Bits8,
                    scale: Scale::Full,
                    rounding: Rounding::NearestEven,
                }),
                ..canonical()
            },
        ),
        (
            vec!["quantise_rounding=ceil"],
            Config {
                quantise: Some(Quantise {
                    depth: Depth::Bits8,
                    scale: Scale::Full,
                    rounding: Rounding::Ceil,
                }),
                ..canonical()
            },
        ),
    ];
    let shape = SHAPE.to_string();
    let mut differing = 0;
    for (params, cfg) in &cases {
        for (depth, word, bytes) in [(8u32, "8", 4usize), (16, "16", 8), (32, "32", 16)] {
            let cfg = Config {
                quantise: cfg.quantise.map(|q| Quantise {
                    depth: match depth {
                        8 => Depth::Bits8,
                        16 => Depth::Bits16,
                        _ => Depth::Float32,
                    },
                    ..q
                }),
                ..cfg.clone()
            };
            let input = vec![0u8; 5 * 4 * bytes];
            let run = run_depth(word, ("5", "4"), &shape, params, &input);
            assert_eq!((run.code, run.err.as_str()), (0, ""), "{params:?} {depth}");
            let want = expected(&cfg, depth);
            assert_eq!(
                alpha_of(&run.out, depth),
                want,
                "{params:?} at depth {depth}"
            );
            if depth == 16 && want != expected(&canonical_at_sixteen(), 16) {
                differing += 1;
            }
        }
    }
    // the table is not the default in disguise: at depth 16 every row but the first, the one
    // without quantisation and the one with the other tie rule gives another plane
    assert!(differing >= cases.len() - 3, "{differing}");
}

fn canonical_at_sixteen() -> Config {
    Config {
        quantise: Some(Quantise {
            depth: Depth::Bits16,
            scale: Scale::Full,
            rounding: Rounding::NearestHalfUp,
        }),
        ..Config::canonical()
    }
}

// -- the manifest and --describe ----------------------------------------------------------------

#[test]
fn the_manifest_entry_says_what_the_stage_is() {
    let manifest = String::from_utf8(spawn(&["list", "--json"], b"").out).unwrap();
    let at = manifest
        .find("{\"id\":\"raster-polygon\"")
        .expect("the stage is listed");
    let end = manifest[at..]
        .find("\"family\":")
        .expect("the entry has a family");
    let entry = &manifest[at..at + end + 20];
    assert!(entry.starts_with(
        "{\"id\":\"raster-polygon\",\"kind\":\"frame\",\"spool\":false,\"geometry\":\"preserve\",\"depths\":[8,16,32],\"params\":[{\"id\":\"carry\","
    ));
    assert!(entry.contains("\"layers\":[{\"id\":\"shape\",\"optional\":true,\"form\":\"rings-1\","));
    assert!(entry.contains("{\"id\":\"keys\",\"optional\":true,\"form\":\"keys-1\","));
    assert!(entry.contains(&format!("{GENERIC_CLAIM}\",\"family\":\"generic\"")));
    assert_eq!(entry.matches("\"keyed\":true").count(), 2);
    for id in [
        "carry",
        "apply",
        "method",
        "samples",
        "fill",
        "centre",
        "y_down",
        "combine",
        "precision",
        "snap_bits",
        "quantise",
        "quantise_scale",
        "quantise_rounding",
        "offset_x",
        "offset_y",
    ] {
        assert_eq!(
            entry.matches(&format!("{{\"id\":\"{id}\",")).count(),
            1,
            "{id}"
        );
    }
    assert_eq!(entry.matches("\"keyed\":").count(), 15);
}

#[test]
fn describe_answers_the_request_and_opens_no_layer() {
    let args = [
        "apply",
        STAGE,
        "--describe",
        "--width",
        "5",
        "--height",
        "3",
        "--depth",
        "16",
        "--fps",
        "30000/1001",
        "--frames",
        "7",
        "--layer",
        "shape=/no/such/folder/shape.txt",
        "--layer",
        "keys=/no/such/folder/keys.txt",
        "--param",
        "carry=luma",
    ];
    let run = spawn(&args, b"");
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert_eq!(
        String::from_utf8(run.out).unwrap(),
        "{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"raster-polygon\",\"width\":5,\"height\":3,\"depth\":16,\"fps\":\"30000/1001\",\"frames\":7}\n"
    );
}

// -- the refusals ---------------------------------------------------------------------------------

fn base<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "apply", STAGE, "--width", "2", "--height", "2", "--depth", "8", "--fps", "24/1",
    ];
    args.extend_from_slice(extra);
    args
}

/// A refusal before the first frame: the exit, the start of the one error line, no output.
fn refused(run: &Run, code: i32, start: &str, what: &str) {
    assert_eq!(run.code, code, "{what}: {}", run.err);
    assert!(run.err.starts_with(start), "{what}: {}", run.err);
    assert_eq!(run.err.matches('\n').count(), 1, "{what}: {}", run.err);
    assert!(run.err.ends_with('\n'), "{what}");
    assert!(run.out.is_empty(), "{what}: no frame is written");
}

#[test]
fn a_frame_the_rasteriser_does_not_take_is_unsupported() {
    let describe = |w: &'static str, h: &'static str| {
        spawn(
            &[
                "apply",
                STAGE,
                "--describe",
                "--width",
                w,
                "--height",
                h,
                "--depth",
                "8",
                "--fps",
                "24/1",
            ],
            b"",
        )
    };
    assert_eq!(describe("8192", "8192").code, 0);
    assert_eq!(describe("65536", "1024").code, 0);
    refused(
        &describe("8193", "8192"),
        3,
        "error: unsupported: a 8193x8192 frame is more than the rasteriser takes: 65536 on a side and 67108864 pixels\n",
        "pixels",
    );
    refused(
        &describe("65537", "1"),
        3,
        "error: unsupported: a 65537x1 frame is more than the rasteriser takes",
        "side",
    );
    let apply = spawn(
        &[
            "apply", STAGE, "--width", "65537", "--height", "1", "--depth", "8", "--fps", "24/1",
        ],
        &[0; 64],
    );
    refused(&apply, 3, "error: unsupported: a 65537x1 frame", "apply");
}

#[test]
fn parameter_refusals() {
    let frame = frame8(&[1, 2, 3, 4]);
    let unsupported = [
        "carry=red",
        "apply=min",
        "method=area",
        "samples=0",
        "samples=65",
        "samples=4.5",
        "fill=odd",
        "centre=1.5",
        "centre=-0.1",
        "centre=nan",
        "y_down=maybe",
        "combine=add",
        "precision=half",
        "snap_bits=31",
        "snap_bits=-1",
        "quantise=yes",
        "quantise_scale=custom",
        "quantise_rounding=nearest",
        "offset_x=inf",
        "offset_x=2e9",
        "offset_y=-2e9",
        "offset_x=1@0;2@5",
        "flatten=uniform",
        "no_such_param=1",
    ];
    for param in unsupported {
        let run = spawn(&base(&["--param", param]), &frame);
        refused(&run, 3, "error: unsupported: ", param);
    }
    let keyed = spawn(&base(&["--param", "offset_x=1@0;2@5"]), &frame);
    assert_eq!(
        keyed.err,
        "error: unsupported: bad value for `offset_x`: a keyed value is not taken on the command line; give a key file with --layer keys=PATH\n"
    );
    let twice = spawn(
        &base(&["--param", "carry=alpha", "--param", "carry=luma"]),
        &frame,
    );
    refused(
        &twice,
        2,
        "error: usage: parameter `carry` given more than once\n",
        "twice",
    );
    let bare = spawn(&base(&["--param", "carry"]), &frame);
    refused(&bare, 2, "error: usage: ", "no value");
}

#[test]
fn layer_refusals() {
    let frame = frame8(&[1, 2, 3, 4]);
    let good = Scratch::new(&rings(&[&rect(0.0, 0.0, 1.0, 1.0)]));
    let good_layer = good.layer("shape");
    let twice = spawn(
        &base(&["--layer", &good_layer, "--layer", &good_layer]),
        &frame,
    );
    refused(
        &twice,
        2,
        "error: usage: layer `shape` given more than once\n",
        "twice",
    );
    let empty_path = spawn(&base(&["--layer", "shape="]), &frame);
    refused(
        &empty_path,
        2,
        "error: usage: layer `shape` has an empty path\n",
        "empty path",
    );
    let unknown = spawn(&base(&["--layer", "matte=x"]), &frame);
    refused(
        &unknown,
        3,
        "error: unsupported: raster-polygon has no layer `matte`: it takes shape, keys\n",
        "unknown layer",
    );
    let missing = spawn(
        &base(&["--layer", "shape=/no/such/folder/shape.txt"]),
        &frame,
    );
    refused(
        &missing,
        4,
        "error: bad_input: cannot read the shape file `/no/such/folder/shape.txt`: ",
        "missing file",
    );
    let malformed = [
        ("", "the file has no `rings-1` line"),
        ("contours-1\n", "line 1: the first line is not `rings-1`"),
        ("keys-1\n", "line 1: the first line is not `rings-1`"),
        (
            "rings-1\nring\n",
            "line 2: `ring` comes after a `polygon` line",
        ),
        (
            "rings-1\npolygon\npoint 1 1\n",
            "line 3: `point` comes after a `ring` line",
        ),
        (
            "rings-1\npolygon\nring\npoint 1 1 1\n",
            "line 4: expected `polygon`, `ring` or `point X Y`",
        ),
        (
            "rings-1\npolygon\nring\npoint 1 one\n",
            "line 4: `one` is not a number",
        ),
        (
            "rings-1\npolygon\nring\npoint 1 +1\n",
            "line 4: `+1` is not a number",
        ),
        ("rings-1\npolygon\n", "polygon 0 has no ring"),
        (
            "rings-1\npolygon\nring\npoint 0 0\npoint 1 0\npoint 1 1\npolygon\nring\npoint 0 0\n",
            "ring 0 of polygon 1 has 1 points: a ring has at least 3",
        ),
    ];
    for (text, why) in malformed {
        let file = Scratch::new(text);
        let run = spawn(&base(&["--layer", &file.layer("shape")]), &frame);
        let want = format!(
            "error: bad_input: shape file `{}`: {why}\n",
            file.0.display()
        );
        refused(&run, 4, &want, why);
    }
    let binary = Scratch(std::env::temp_dir().join(format!(
        "stage-cli-raster-polygon-{}-binary",
        std::process::id()
    )));
    std::fs::write(&binary.0, [0xff, 0xfe, 0x00]).unwrap();
    let run = spawn(&base(&["--layer", &binary.layer("shape")]), &frame);
    refused(
        &run,
        4,
        "error: bad_input: cannot read the shape file ",
        "not text",
    );
    let far = Scratch::new("rings-1\npolygon\nring\npoint 0 0\npoint 1 0\npoint 0 -3e9\n");
    let run = spawn(&base(&["--layer", &far.layer("shape")]), &frame);
    let want = format!(
        "error: unsupported: shape file `{}`: line 6: -3000000000 is outside -1000000000 to 1000000000\n",
        far.0.display()
    );
    refused(&run, 3, &want, "a number above the bound");
}

#[test]
fn key_file_refusals() {
    let frame = frame8(&[1, 2, 3, 4]);
    let cases = [
        (
            "keys-1\nkey centre 0 0.25 hold\n",
            3,
            "error: unsupported: key file `",
            "raster-polygon has no keyed parameter `centre`",
        ),
        (
            "keys-1\nkey offset_x 0 2e9 hold\n",
            3,
            "error: unsupported: key file `",
            "of `offset_x` at time 0 is outside -1000000000 to 1000000000",
        ),
        (
            "keys-2\n",
            4,
            "error: bad_input: key file `",
            "line 1: the first line is not `keys-1`",
        ),
        (
            "keys-1\nkey offset_x 0 1 cubic\n",
            4,
            "error: bad_input: key file `",
            "is not a key kind",
        ),
    ];
    for (text, code, start, part) in cases {
        let file = Scratch::new(text);
        let run = spawn(&base(&["--layer", &file.layer("keys")]), &frame);
        refused(&run, code, start, part);
        assert!(run.err.contains(part), "{}", run.err);
    }
    let missing = spawn(&base(&["--layer", "keys=/no/such/folder/keys.txt"]), &frame);
    refused(
        &missing,
        4,
        "error: bad_input: cannot read the key file `/no/such/folder/keys.txt`: ",
        "missing key file",
    );
}

#[test]
fn stream_refusals_keep_the_whole_frames_written() {
    let shape = Scratch::new(&rings(&[&rect(0.0, 0.0, 1.0, 1.0)]));
    let layer = shape.layer("shape");
    let frame = frame8(&[9, 9, 9, 9]);
    let matte = frame8(&[255, 0, 0, 0]);
    let mut input = frame.repeat(2);
    input.extend_from_slice(&frame[..5]);
    let partial = spawn(&base(&["--layer", &layer]), &input);
    assert_eq!(partial.code, 4);
    assert_eq!(
        partial.err,
        "error: partial_frame: frame 2: the input ends inside a frame (5 of 16 bytes) after 2 whole frames\n"
    );
    assert_eq!(partial.out, matte.repeat(2));
    let count = spawn(
        &base(&["--layer", &layer, "--frames", "3"]),
        &frame.repeat(2),
    );
    assert_eq!(count.code, 4);
    assert_eq!(
        count.err,
        "error: bad_input: the input holds 2 frames, but --frames says 3\n"
    );
    assert_eq!(count.out, matte.repeat(2));
    let exact = spawn(
        &base(&["--layer", &layer, "--frames", "2"]),
        &frame.repeat(2),
    );
    assert_eq!((exact.code, exact.err.as_str()), (0, ""));
    let empty = spawn(&base(&["--layer", &layer]), b"");
    assert_eq!(
        (empty.code, empty.out.len(), empty.err.as_str()),
        (0, 0, "")
    );
    let depth = spawn(
        &[
            "apply", STAGE, "--width", "2", "--height", "2", "--depth", "12", "--fps", "24/1",
            "--layer", &layer,
        ],
        &frame,
    );
    refused(
        &depth,
        3,
        "error: unsupported: depth 12 is not supported: raster-polygon takes 8, 16 or 32\n",
        "depth",
    );
    let option = spawn(&base(&["--layer", &layer, "--shape", "x"]), &frame);
    refused(&option, 2, "error: usage: ", "unknown option");
}

#[test]
fn two_runs_give_the_same_bytes() {
    let shape = SHAPE.to_string();
    let input: Vec<u8> = (0..5 * 4 * 8 * 3)
        .map(|i| (i as u8).wrapping_mul(37).wrapping_add(11))
        .collect();
    let first = run_depth("16", ("5", "4"), &shape, &["carry=all"], &input);
    let second = run_depth("16", ("5", "4"), &shape, &["carry=all"], &input);
    assert_eq!((first.code, first.err.as_str()), (0, ""));
    assert_eq!(first.out, second.out);
    assert_eq!(first.out.len(), input.len());
}

#[test]
fn a_reader_that_leaves_is_broken_pipe() {
    let shape = Scratch::new(&rings(&[&rect(0.0, 0.0, 1.0, 1.0)]));
    let layer = shape.layer("shape");
    let mut child = Command::new(env!("CARGO_BIN_EXE_mocha-stage"))
        .args(base(&["--layer", &layer]))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // close the reading end of the output at once, then feed more than a pipe holds
    drop(child.stdout.take());
    let mut stdin = child.stdin.take().unwrap();
    let data = frame8(&[9, 9, 9, 9]).repeat(40000);
    let feeder = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let mut err = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    let status = child.wait().unwrap();
    feeder.join().unwrap();
    assert_eq!(status.code(), Some(4), "{err}");
    assert!(err.starts_with("error: broken_pipe: "), "{err}");
    assert_eq!(err.matches('\n').count(), 1, "{err}");
}

/// Linux has a device that refuses every write with "no space left on device".
#[cfg(target_os = "linux")]
#[test]
fn a_write_that_fails_for_another_reason_is_stage_failed() {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("open /dev/full");
    let shape = Scratch::new(&rings(&[&rect(0.0, 0.0, 1.0, 1.0)]));
    let layer = shape.layer("shape");
    let mut child = Command::new(env!("CARGO_BIN_EXE_mocha-stage"))
        .args(base(&["--layer", &layer]))
        .stdin(Stdio::piped())
        .stdout(Stdio::from(full))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let data = frame8(&[9, 9, 9, 9]).repeat(200);
    let feeder = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let out = child.wait_with_output().unwrap();
    feeder.join().unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.starts_with("error: stage_failed: frame "), "{err}");
    assert_eq!(err.matches('\n').count(), 1, "{err}");
}
