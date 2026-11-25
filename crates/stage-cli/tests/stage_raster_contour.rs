//! `raster-contour` on the built program: what it computes on small made up frames, against
//! values worked out by hand, against `raster-polygon` and against the `contour-eval` and
//! `coverage-raster` crates called directly, and every refusal of its own. Every frame and every
//! shape is made up.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use contour_eval::{
    Config, Contour, EvalForm, FlatTest, Flatten, Kind, OpenEnd, Point, Precision, TangentMode,
    WeightMap, XVariant,
};
use coverage_raster::{rasterise_buffer, Buffer, Polygon, Target};
use stage_cli::registry::GENERIC_CLAIM;

const STAGE: &str = "raster-contour";

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
            "stage-cli-raster-contour-{}-{}",
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

/// Run a stage at depth 8 on one frame of zeros and return the alpha of the output.
fn alpha(stage: &str, size: (usize, usize), shape: &str, params: &[&str]) -> Vec<u8> {
    let file = Scratch::new(shape);
    let layer = file.layer("shape");
    let (w, h) = (size.0.to_string(), size.1.to_string());
    let mut args = vec![
        "apply", stage, "--width", &w, "--height", &h, "--depth", "8", "--fps", "24/1", "--layer",
        &layer,
    ];
    for p in params {
        args.extend(["--param", p]);
    }
    let run = spawn(&args, &vec![0; size.0 * size.1 * 4]);
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{stage} {params:?}");
    assert_eq!(run.out.len(), size.0 * size.1 * 4);
    run.out.as_chunks::<4>().0.iter().map(|p| p[3]).collect()
}

// -- what the stage computes, by hand -----------------------------------------------------------

/// A closed Bezier contour of two points. The span from the first to the second has the control
/// points (0, 0), (0, 4), (6, 4), (6, 0), so its point at 1/2 is (3, 3); the span back has
/// handles of length 0 and is the straight line, with (3, 0) at 1/2.
const ARCH: &str =
    "contours-1\npolygon\ncontour bezier closed\npoint 0 0 0 0 0 4\npoint 6 0 0 4 0 0\n";

#[test]
fn two_segments_per_span_give_the_triangle_through_the_point_at_one_half() {
    // the polyline is (0, 0), (3, 3), (6, 0), (3, 0): the left edge is x = y, the right
    // edge x = 6 - y, so an edge pixel is covered by 1/2
    let want = [
        128, 255, 255, 255, 255, 128, //
        0, 128, 255, 255, 128, 0, //
        0, 0, 128, 128, 0, 0,
    ];
    assert_eq!(alpha(STAGE, (6, 3), ARCH, &["segments=2"]), want);
    for form in ["form=bernstein", "form=de-casteljau", "form=horner"] {
        assert_eq!(
            alpha(STAGE, (6, 3), ARCH, &["segments=2", form]),
            want,
            "{form}"
        );
    }
    assert_eq!(
        alpha(STAGE, (6, 3), ARCH, &["segments=2", "eval_precision=f32"]),
        want
    );
    // the same handles as positions
    let absolute =
        "contours-1\npolygon\ncontour bezier closed\npoint 0 0 0 0 0 4\npoint 6 0 6 4 6 0\n";
    assert_eq!(
        alpha(
            STAGE,
            (6, 3),
            absolute,
            &["segments=2", "tangents=absolute"]
        ),
        want
    );
}

#[test]
fn one_segment_per_span_is_the_polygon_of_the_anchors() {
    // at one segment per span only the anchors are used, whatever the handles are
    let contour = "contours-1\npolygon\ncontour bezier closed\npoint 1 1 9 9 -3 2\npoint 3 1 0 5 1 1\npoint 3 2 2 2 2 2\npoint 1 2 0 -7 4 0\n";
    let ring = "rings-1\npolygon\nring\npoint 1 1\npoint 3 1\npoint 3 2\npoint 1 2\n";
    let want = [0, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0, 0];
    assert_eq!(alpha(STAGE, (4, 3), contour, &["segments=1"]), want);
    assert_eq!(alpha("raster-polygon", (4, 3), ring, &[]), want);
}

#[test]
fn an_xspline_with_sharp_points_is_the_polygon_of_its_points() {
    // weight 0 is a sharp point on the curve, and a span between two sharp points starts at
    // its first point
    for kind in ["xspline-basic", "xspline-general"] {
        let contour = format!(
            "contours-1\npolygon\ncontour {kind} closed\npoint 1 1 0\npoint 3 1 0\npoint 3 2 0\npoint 1 2 0\n"
        );
        let want = [0, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0, 0];
        assert_eq!(
            alpha(STAGE, (4, 3), &contour, &["segments=1"]),
            want,
            "{kind}"
        );
    }
}

#[test]
fn an_open_contour_is_filled_as_the_ring_of_its_polyline() {
    // three anchors, open: the polyline ends with the last anchor and the ring closes it
    let contour =
        "contours-1\npolygon\ncontour bezier open\npoint 0 0 0 0 0 0\npoint 4 0 0 0 0 0\npoint 0 4 0 0 0 0\n";
    let ring = "rings-1\npolygon\nring\npoint 0 0\npoint 4 0\npoint 0 4\n";
    let made = alpha(STAGE, (4, 4), contour, &["segments=1"]);
    assert_eq!(made, alpha("raster-polygon", (4, 4), ring, &[]));
    // the triangle under x + y = 4: the pixels on the diagonal are covered by 1/2
    let want = [
        255, 255, 255, 128, //
        255, 255, 128, 0, //
        255, 128, 0, 0, //
        128, 0, 0, 0,
    ];
    assert_eq!(made, want);
}

#[test]
fn the_contours_of_one_polygon_are_filled_together() {
    // a square and a square inside it, the same direction
    let shape = "contours-1\npolygon\ncontour bezier closed\npoint 0 0 0 0 0 0\npoint 5 0 0 0 0 0\npoint 5 5 0 0 0 0\npoint 0 5 0 0 0 0\ncontour xspline-basic closed\npoint 1 1 0\npoint 4 1 0\npoint 4 4 0\npoint 1 4 0\n";
    assert_eq!(alpha(STAGE, (5, 5), shape, &["segments=1"]), [255; 25]);
    let hole = [
        255, 255, 255, 255, 255, //
        255, 0, 0, 0, 255, //
        255, 0, 0, 0, 255, //
        255, 0, 0, 0, 255, //
        255, 255, 255, 255, 255,
    ];
    assert_eq!(
        alpha(STAGE, (5, 5), shape, &["segments=1", "fill=even-odd"]),
        hole
    );
}

#[test]
fn the_parameters_of_raster_polygon_act_on_the_polylines() {
    let square =
        "contours-1\npolygon\ncontour bezier closed\npoint 0 0 0 0 0 0\npoint 1 0 0 0 0 0\npoint 1 1 0 0 0 0\npoint 0 1 0 0 0 0\n";
    let run = |params: &[&str]| {
        let mut all = vec!["segments=1"];
        all.extend_from_slice(params);
        alpha(STAGE, (3, 2), square, &all)
    };
    assert_eq!(run(&[]), [255, 0, 0, 0, 0, 0]);
    assert_eq!(run(&["offset_x=2", "offset_y=1"]), [0, 0, 0, 0, 0, 255]);
    assert_eq!(run(&["offset_x=0.5"]), [128, 128, 0, 0, 0, 0]);
    assert_eq!(
        run(&["offset_x=0.5", "quantise_rounding=floor"]),
        [127, 127, 0, 0, 0, 0]
    );
    assert_eq!(run(&["y_down=0"]), [0, 0, 0, 255, 0, 0]);
    // carry and apply on a frame that is not zero
    let file = Scratch::new(square);
    let layer = file.layer("shape");
    let luma = spawn(
        &[
            "apply",
            STAGE,
            "--width",
            "2",
            "--height",
            "1",
            "--depth",
            "8",
            "--fps",
            "24/1",
            "--layer",
            &layer,
            "--param",
            "segments=1",
            "--param",
            "carry=luma",
            "--param",
            "apply=multiply",
        ],
        &[1, 2, 3, 51, 4, 5, 6, 51],
    );
    assert_eq!((luma.code, luma.err.as_str()), (0, ""));
    assert_eq!(luma.out, [51, 51, 51, 255, 0, 0, 0, 255]);
}

#[test]
fn a_key_file_moves_the_contours_from_frame_to_frame() {
    let shape = Scratch::new(
        "contours-1\npolygon\ncontour xspline-basic closed\npoint 0 0 0\npoint 1 0 0\npoint 1 1 0\npoint 0 1 0\n",
    );
    let keys = Scratch::new("keys-1\nstart 10\nkey offset_x 10 0 linear\nkey offset_x 12 1 hold\n");
    let (shape_layer, keys_layer) = (shape.layer("shape"), keys.layer("keys"));
    let run = spawn(
        &[
            "apply",
            STAGE,
            "--width",
            "2",
            "--height",
            "1",
            "--depth",
            "8",
            "--fps",
            "24/1",
            "--layer",
            &shape_layer,
            "--layer",
            &keys_layer,
            "--param",
            "segments=1",
        ],
        &[0; 2 * 4 * 4],
    );
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    let made: Vec<u8> = run.out.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
    assert_eq!(made, [255, 0, 128, 128, 0, 255, 0, 255]);
}

#[test]
fn without_a_shape_nothing_is_covered() {
    let input = [1, 2, 3, 40, 5, 6, 7, 80];
    let run = spawn(
        &[
            "apply", STAGE, "--width", "2", "--height", "1", "--depth", "8", "--fps", "24/1",
        ],
        &input,
    );
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert_eq!(run.out, [1, 2, 3, 0, 5, 6, 7, 0]);
    let empty = Scratch::new("contours-1\n# no polygon\n");
    let layer = empty.layer("shape");
    let run = spawn(
        &[
            "apply", STAGE, "--width", "2", "--height", "1", "--depth", "8", "--fps", "24/1",
            "--layer", &layer,
        ],
        &input,
    );
    assert_eq!(run.out, [1, 2, 3, 0, 5, 6, 7, 0]);
}

// -- against the crates called directly ---------------------------------------------------------

fn bezier(x: f64, y: f64, tan_in: (f64, f64), tan_out: (f64, f64)) -> Point {
    Point {
        tan_in,
        tan_out,
        ..Point::new(x, y)
    }
}

fn spline(x: f64, y: f64, s: f64) -> Point {
    Point {
        s,
        ..Point::new(x, y)
    }
}

/// One polygon with three contours: a curved closed Bezier contour, an open general X-spline
/// and a closed basic X-spline. The text and the values hold the same numbers.
const CURVES: &str = "contours-1\npolygon\ncontour bezier closed\npoint 1 1 -0.5 1.5 2 -1\npoint 8 1.5 -1 -2 0.5 2\npoint 6.5 6 2 1 -3 0.5\npoint 1.5 5 -1 1 0.25 -2\ncontour xspline-general open\npoint 2 2 0.5\npoint 7 2.5 -0.5\npoint 5 5 1\npoint 2.5 4 -1\ncontour xspline-basic closed\npoint 3 0.5 1\npoint 8.5 3 0.5\npoint 4 6.5 0\npoint 0.5 3 0.75\n";

fn curves() -> Vec<Contour> {
    vec![
        Contour {
            kind: Kind::Bezier,
            closed: true,
            points: vec![
                bezier(1.0, 1.0, (-0.5, 1.5), (2.0, -1.0)),
                bezier(8.0, 1.5, (-1.0, -2.0), (0.5, 2.0)),
                bezier(6.5, 6.0, (2.0, 1.0), (-3.0, 0.5)),
                bezier(1.5, 5.0, (-1.0, 1.0), (0.25, -2.0)),
            ],
        },
        Contour {
            kind: Kind::XSpline {
                variant: XVariant::General,
            },
            closed: false,
            points: vec![
                spline(2.0, 2.0, 0.5),
                spline(7.0, 2.5, -0.5),
                spline(5.0, 5.0, 1.0),
                spline(2.5, 4.0, -1.0),
            ],
        },
        Contour {
            kind: Kind::XSpline {
                variant: XVariant::Basic,
            },
            closed: true,
            points: vec![
                spline(3.0, 0.5, 1.0),
                spline(8.5, 3.0, 0.5),
                spline(4.0, 6.5, 0.0),
                spline(0.5, 3.0, 0.75),
            ],
        },
    ]
}

/// The even odd fill rule, so that a change of any one contour changes the plane.
const FILL: &str = "fill=even-odd";

/// The codes the two crates give for the contours under a rule and a configuration, with the
/// rasteriser at its documented default but for the even odd fill rule.
fn expected(contours: &[Contour], rule: &Flatten, cfg: &Config) -> Vec<u8> {
    let rings: Vec<Vec<(f64, f64)>> = contours
        .iter()
        .map(|c| contour_eval::flatten(c, rule, cfg).unwrap())
        .collect();
    let raster = coverage_raster::Config {
        fill: coverage_raster::FillRule::EvenOdd,
        ..coverage_raster::Config::canonical()
    };
    match rasterise_buffer(&[Polygon(rings)], &Target::new(9, 7), &raster).unwrap() {
        Buffer::U8(codes) => codes,
        other => panic!(
            "the default buffer is 8 bit, got {} samples of another type",
            other.len()
        ),
    }
}

#[test]
fn every_parameter_reaches_the_flattening() {
    let canonical = Config::canonical;
    let uniform = Flatten::Uniform(16);
    let cases: Vec<(Vec<&str>, Flatten, Config)> = vec![
        (vec![], uniform, canonical()),
        (
            vec!["flatten=uniform", "segments=3"],
            Flatten::Uniform(3),
            canonical(),
        ),
        (
            vec!["flatten=subdivide"],
            Flatten::Subdivide {
                tol: 0.25,
                max_depth: 10,
            },
            canonical(),
        ),
        (
            vec!["flatten=subdivide", "tolerance=0.03125", "max_depth=3"],
            Flatten::Subdivide {
                tol: 0.03125,
                max_depth: 3,
            },
            canonical(),
        ),
        (
            vec!["flatten=by-length"],
            Flatten::ByLength {
                step: 1.0,
                samples: 16,
            },
            canonical(),
        ),
        (
            vec!["flatten=by-length", "step=2.5", "length_samples=3"],
            Flatten::ByLength {
                step: 2.5,
                samples: 3,
            },
            canonical(),
        ),
        (
            vec!["segments=5", "form=de-casteljau"],
            Flatten::Uniform(5),
            Config {
                form: EvalForm::DeCasteljau,
                ..canonical()
            },
        ),
        (
            vec!["segments=5", "form=horner", "eval_precision=f32"],
            Flatten::Uniform(5),
            Config {
                form: EvalForm::Horner,
                precision: Precision::F32,
                ..canonical()
            },
        ),
        (
            vec!["tangents=absolute"],
            uniform,
            Config {
                tangents: TangentMode::Absolute,
                ..canonical()
            },
        ),
        (
            vec!["open_end=repeat-end-sharp"],
            uniform,
            Config {
                open_end: OpenEnd::RepeatEndSharp,
                ..canonical()
            },
        ),
        (
            vec!["closing_span=0"],
            uniform,
            Config {
                closing_span: false,
                ..canonical()
            },
        ),
        (
            vec!["repeat_first=1"],
            uniform,
            Config {
                repeat_first: true,
                ..canonical()
            },
        ),
    ];
    let default_plane = expected(&curves(), &uniform, &canonical());
    let mut differing = 0;
    for (params, rule, cfg) in &cases {
        let want = expected(&curves(), rule, cfg);
        let mut all = params.clone();
        all.push(FILL);
        assert_eq!(alpha(STAGE, (9, 7), CURVES, &all), want, "{params:?}");
        if want != default_plane {
            differing += 1;
        }
    }
    // the table is not the default in disguise: the first row is the default and a repeated
    // first point adds no area, and the other rows give another plane (one may coincide)
    assert!(differing >= cases.len() - 3, "{differing}");
}

#[test]
fn the_flatness_test_and_the_weight_map_reach_the_flattening() {
    // the control polygon test takes Bezier contours only
    let bezier_only = "contours-1\npolygon\ncontour bezier closed\npoint 1 1 -0.5 1.5 2 -1\npoint 8 1.5 -1 -2 0.5 2\npoint 6.5 6 2 1 -3 0.5\npoint 1.5 5 -1 1 0.25 -2\n";
    let contour = &curves()[..1];
    let rule = Flatten::Subdivide {
        tol: 0.5,
        max_depth: 2,
    };
    let params = ["flatten=subdivide", "tolerance=0.5", "max_depth=2"];
    let mut planes = Vec::new();
    for (word, flat_test) in [
        ("flat_test=sampled", FlatTest::Sampled),
        ("flat_test=control-polygon", FlatTest::ControlPolygon),
    ] {
        let cfg = Config {
            flat_test,
            ..Config::canonical()
        };
        let want = expected(contour, &rule, &cfg);
        let mut all = params.to_vec();
        all.extend([word, FILL]);
        assert_eq!(alpha(STAGE, (9, 7), bezier_only, &all), want, "{word}");
        planes.push(want);
    }
    assert_ne!(
        planes[0], planes[1],
        "the two tests stop at other depths here"
    );

    // the same curve stored with weights from 0 to 1 and from 0 to 100, and the four maps in
    // the order of the crate
    let unit = ("contours-1\npolygon\ncontour xspline-general closed\npoint 1 1 0.5\npoint 8 1.5 1\npoint 6.5 6 0.75\npoint 1.5 5 0.6\n", [0.5, 1.0, 0.75, 0.6]);
    let hundred = ("contours-1\npolygon\ncontour xspline-general closed\npoint 1 1 50\npoint 8 1.5 100\npoint 6.5 6 75\npoint 1.5 5 60\n", [50.0, 100.0, 75.0, 60.0]);
    let contour = |weights: [f64; 4]| {
        let at = [(1.0, 1.0), (8.0, 1.5), (6.5, 6.0), (1.5, 5.0)];
        vec![Contour {
            kind: Kind::XSpline {
                variant: XVariant::General,
            },
            closed: true,
            points: at
                .iter()
                .zip(weights)
                .map(|(&(x, y), w)| spline(x, y, w))
                .collect(),
        }]
    };
    let maps = [
        ("weight_map=identity", unit, WeightMap { a: 1.0, b: 0.0 }),
        (
            "weight_map=unit-to-signed",
            unit,
            WeightMap { a: 2.0, b: -1.0 },
        ),
        (
            "weight_map=hundred-to-unit",
            hundred,
            WeightMap { a: 0.01, b: 0.0 },
        ),
        (
            "weight_map=hundred-to-signed",
            hundred,
            WeightMap { a: 0.02, b: -1.0 },
        ),
    ];
    let mut planes = Vec::new();
    for (word, (shape, weights), weight_map) in maps {
        assert!(Config::WEIGHT_MAPS.contains(&weight_map));
        let cfg = Config {
            weight_map,
            ..Config::canonical()
        };
        let want = expected(&contour(weights), &Flatten::Uniform(16), &cfg);
        assert_eq!(alpha(STAGE, (9, 7), shape, &[word, FILL]), want, "{word}");
        planes.push(want);
    }
    assert_ne!(planes[0], planes[1], "the signed map gives another curve");
}

// -- the manifest and --describe ----------------------------------------------------------------

#[test]
fn the_manifest_entry_says_what_the_stage_is() {
    let manifest = String::from_utf8(spawn(&["list", "--json"], b"").out).unwrap();
    let at = manifest
        .find("{\"id\":\"raster-contour\"")
        .expect("the stage is listed");
    let end = manifest[at..]
        .find("\"family\":")
        .expect("the entry has a family");
    let entry = &manifest[at..at + end + 18];
    assert!(entry.starts_with(
        "{\"id\":\"raster-contour\",\"kind\":\"frame\",\"spool\":false,\"geometry\":\"preserve\",\"depths\":[8,16,32],\"params\":[{\"id\":\"carry\","
    ));
    assert!(
        entry.contains("\"layers\":[{\"id\":\"shape\",\"optional\":true,\"form\":\"contours-1\",")
    );
    assert!(entry.contains("{\"id\":\"keys\",\"optional\":true,\"form\":\"keys-1\","));
    assert!(entry.ends_with(&format!("{GENERIC_CLAIM}\",\"family\":\"generic\"")));
    assert_eq!(entry.matches("\"keyed\":true").count(), 2);
    assert_eq!(entry.matches("\"keyed\":").count(), 29);
    for id in [
        "flatten",
        "segments",
        "tolerance",
        "max_depth",
        "step",
        "length_samples",
        "form",
        "eval_precision",
        "tangents",
        "open_end",
        "closing_span",
        "repeat_first",
        "flat_test",
        "weight_map",
    ] {
        assert_eq!(
            entry.matches(&format!("{{\"id\":\"{id}\",")).count(),
            1,
            "{id}"
        );
    }
    let polygon = manifest
        .find("{\"id\":\"raster-polygon\"")
        .expect("raster-polygon is listed");
    assert!(polygon < at, "raster-polygon comes first");
}

#[test]
fn describe_answers_the_request_and_opens_no_layer() {
    let args = [
        "apply",
        STAGE,
        "--describe",
        "--width",
        "9",
        "--height",
        "7",
        "--depth",
        "32",
        "--fps",
        "25/1",
        "--layer",
        "shape=/no/such/folder/shape.txt",
        "--param",
        "flatten=subdivide",
    ];
    let run = spawn(&args, b"");
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert_eq!(
        String::from_utf8(run.out).unwrap(),
        "{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"raster-contour\",\"width\":9,\"height\":7,\"depth\":32,\"fps\":\"25/1\",\"frames\":null}\n"
    );
    let large = spawn(
        &[
            "apply",
            STAGE,
            "--describe",
            "--width",
            "8193",
            "--height",
            "8192",
            "--depth",
            "8",
            "--fps",
            "24/1",
        ],
        b"",
    );
    assert_eq!(large.code, 3);
    assert_eq!(
        large.err,
        "error: unsupported: a 8193x8192 frame is more than the rasteriser takes: 65536 on a side and 67108864 pixels\n"
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

const FRAME: [u8; 16] = [9; 16];

#[test]
fn parameter_refusals() {
    let unsupported = [
        "flatten=adaptive",
        "segments=0",
        "segments=4097",
        "tolerance=-1",
        "tolerance=inf",
        "max_depth=25",
        "max_depth=-1",
        "step=0",
        "step=-2",
        "length_samples=0",
        "length_samples=4097",
        "form=power",
        "eval_precision=f16",
        "tangents=polar",
        "open_end=clamp",
        "closing_span=2",
        "repeat_first=yes",
        "flat_test=chord",
        "weight_map=custom",
        "tolerance=0.1@0;0.2@9",
        "samples=65",
        "snap_bits=31",
        "carry=red",
        "no_such_param=1",
    ];
    for param in unsupported {
        let run = spawn(&base(&["--param", param]), &FRAME);
        refused(&run, 3, "error: unsupported: ", param);
    }
    let twice = spawn(
        &base(&["--param", "segments=2", "--param", "segments=3"]),
        &FRAME,
    );
    refused(
        &twice,
        2,
        "error: usage: parameter `segments` given more than once\n",
        "twice",
    );
}

#[test]
fn layer_refusals() {
    let missing = spawn(
        &base(&["--layer", "shape=/no/such/folder/shape.txt"]),
        &FRAME,
    );
    refused(
        &missing,
        4,
        "error: bad_input: cannot read the shape file `/no/such/folder/shape.txt`: ",
        "missing file",
    );
    let unknown = spawn(&base(&["--layer", "contour=x"]), &FRAME);
    refused(
        &unknown,
        3,
        "error: unsupported: raster-contour has no layer `contour`: it takes shape, keys\n",
        "unknown layer",
    );
    let malformed = [
        ("", "the file has no `contours-1` line"),
        (
            "rings-1\npolygon\n",
            "line 1: the first line is not `contours-1`",
        ),
        (
            "contours-1\ncontour bezier closed\n",
            "line 2: `contour` comes after a `polygon` line",
        ),
        (
            "contours-1\npolygon\ncontour nurbs closed\n",
            "line 3: `nurbs` is not a contour kind (bezier, xspline-basic or xspline-general)",
        ),
        (
            "contours-1\npolygon\ncontour bezier ajar\n",
            "line 3: `ajar` is not `closed` or `open`",
        ),
        (
            "contours-1\npolygon\nring\n",
            "line 3: expected `polygon`, `contour KIND CLOSURE` or a `point` line",
        ),
        (
            "contours-1\npolygon\npoint 1 1 0\n",
            "line 3: `point` comes after a `contour` line",
        ),
        (
            "contours-1\npolygon\ncontour bezier closed\npoint 1 1\n",
            "line 4: a point of a bezier contour is `point X Y IN_X IN_Y OUT_X OUT_Y`",
        ),
        (
            "contours-1\npolygon\ncontour xspline-general open\npoint 1 1 0 0 0 0\n",
            "line 4: a point of an xspline contour is `point X Y S`",
        ),
        (
            "contours-1\npolygon\ncontour xspline-general open\npoint 1 1 heavy\n",
            "line 4: `heavy` is not a number",
        ),
        ("contours-1\npolygon\n", "polygon 0 has no contour"),
        (
            "contours-1\npolygon\ncontour xspline-basic closed\npoint 1 1 0\n",
            "contour 0 of polygon 0 has 1 points: a contour has at least 2",
        ),
    ];
    for (text, why) in malformed {
        let file = Scratch::new(text);
        let run = spawn(&base(&["--layer", &file.layer("shape")]), &FRAME);
        let want = format!(
            "error: bad_input: shape file `{}`: {why}\n",
            file.0.display()
        );
        refused(&run, 4, &want, why);
    }
    let far = Scratch::new(
        "contours-1\npolygon\ncontour bezier open\npoint 0 0 0 0 0 0\npoint 1 1 0 0 4e9 0\n",
    );
    let run = spawn(&base(&["--layer", &far.layer("shape")]), &FRAME);
    let want = format!(
        "error: unsupported: shape file `{}`: line 5: 4000000000 is outside -1000000000 to 1000000000\n",
        far.0.display()
    );
    refused(&run, 3, &want, "a number above the bound");
}

#[test]
fn what_the_flattening_refuses_is_unsupported() {
    let spline = Scratch::new(
        "contours-1\npolygon\ncontour bezier closed\npoint 0 0 0 0 0 0\npoint 1 0 0 0 0 0\npoint 0 1 0 0 0 0\ncontour xspline-basic closed\npoint 0 0 0\npoint 1 0 0.5\npoint 0 1 0\n",
    );
    let layer = spline.layer("shape");
    let ok = spawn(&base(&["--layer", &layer]), &FRAME);
    assert_eq!((ok.code, ok.err.as_str()), (0, ""));
    let sampled = spawn(
        &base(&["--layer", &layer, "--param", "flatten=subdivide"]),
        &FRAME,
    );
    assert_eq!((sampled.code, sampled.err.as_str()), (0, ""));
    let control = spawn(
        &base(&[
            "--layer",
            &layer,
            "--param",
            "flatten=subdivide",
            "--param",
            "flat_test=control-polygon",
        ]),
        &FRAME,
    );
    refused(
        &control,
        3,
        "error: unsupported: contour 1 of polygon 0: the control polygon flatness test needs a Bezier contour\n",
        "the control polygon test on an X-spline",
    );
    // weight 0.5 goes to 2 * 0.5 - 1 = 0 under the signed map, and weight 0 to -1, which the
    // basic variant does not take
    let signed = spawn(
        &base(&["--layer", &layer, "--param", "weight_map=unit-to-signed"]),
        &FRAME,
    );
    refused(
        &signed,
        3,
        "error: unsupported: contour 1 of polygon 0: point 0 has shape parameter -1 outside the variant's range\n",
        "a weight outside the range of the variant",
    );
    let heavy = Scratch::new(
        "contours-1\npolygon\ncontour xspline-general closed\npoint 0 0 -1\npoint 1 0 1.5\npoint 0 1 0\n",
    );
    let run = spawn(&base(&["--layer", &heavy.layer("shape")]), &FRAME);
    refused(
        &run,
        3,
        "error: unsupported: contour 0 of polygon 0: point 1 has shape parameter 1.5 outside the variant's range\n",
        "a weight above 1",
    );
}

#[test]
fn stream_refusals_keep_the_whole_frames_written() {
    let shape = Scratch::new(
        "contours-1\npolygon\ncontour bezier closed\npoint 0 0 0 0 0 0\npoint 1 0 0 0 0 0\npoint 1 1 0 0 0 0\npoint 0 1 0 0 0 0\n",
    );
    let layer = shape.layer("shape");
    let matte = [9, 9, 9, 255, 9, 9, 9, 0, 9, 9, 9, 0, 9, 9, 9, 0];
    let mut input = FRAME.repeat(2);
    input.extend_from_slice(&FRAME[..7]);
    let partial = spawn(&base(&["--layer", &layer, "--param", "segments=1"]), &input);
    assert_eq!(partial.code, 4);
    assert_eq!(
        partial.err,
        "error: partial_frame: frame 2: the input ends inside a frame (7 of 16 bytes) after 2 whole frames\n"
    );
    assert_eq!(partial.out, matte.repeat(2));
    let count = spawn(
        &base(&["--layer", &layer, "--frames", "1"]),
        &FRAME.repeat(2),
    );
    assert_eq!(count.code, 4);
    assert_eq!(
        count.err,
        "error: bad_input: the input holds 2 frames, but --frames says 1\n"
    );
    let empty = spawn(&base(&["--layer", &layer]), b"");
    assert_eq!(
        (empty.code, empty.out.len(), empty.err.as_str()),
        (0, 0, "")
    );
    let depth = spawn(
        &[
            "apply", STAGE, "--width", "2", "--height", "2", "--depth", "10", "--fps", "24/1",
            "--layer", &layer,
        ],
        &FRAME,
    );
    refused(
        &depth,
        3,
        "error: unsupported: depth 10 is not supported: raster-contour takes 8, 16 or 32\n",
        "depth",
    );
    let twice = spawn(&base(&["--layer", &layer, "--layer", &layer]), &FRAME);
    refused(
        &twice,
        2,
        "error: usage: layer `shape` given more than once\n",
        "twice",
    );
}

#[test]
fn every_depth_runs_and_two_runs_give_the_same_bytes() {
    let file = Scratch::new(CURVES);
    let layer = file.layer("shape");
    for (depth, bytes) in [("8", 4usize), ("16", 8), ("32", 16)] {
        let input: Vec<u8> = (0..9 * 7 * bytes * 2)
            .map(|i| (i as u8).wrapping_mul(31).wrapping_add(7))
            .collect();
        let args = [
            "apply",
            STAGE,
            "--width",
            "9",
            "--height",
            "7",
            "--depth",
            depth,
            "--fps",
            "24/1",
            "--layer",
            &layer,
            "--param",
            "carry=all",
        ];
        let first = spawn(&args, &input);
        let second = spawn(&args, &input);
        assert_eq!((first.code, first.err.as_str()), (0, ""), "{depth}");
        assert_eq!(first.out.len(), input.len(), "{depth}");
        assert_eq!(first.out, second.out, "{depth}");
        let (a, b) = first.out.split_at(input.len() / 2);
        assert_eq!(a, b, "{depth}: with carry=all both frames are the matte");
    }
}

#[test]
fn a_reader_that_leaves_is_broken_pipe() {
    let shape = Scratch::new("contours-1\npolygon\ncontour xspline-basic closed\npoint 0 0 0\npoint 1 0 0\npoint 1 1 0\npoint 0 1 0\n");
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
    let data = FRAME.repeat(40000);
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
    let shape = Scratch::new("contours-1\npolygon\ncontour xspline-basic closed\npoint 0 0 0\npoint 1 0 0\npoint 1 1 0\npoint 0 1 0\n");
    let layer = shape.layer("shape");
    let mut child = Command::new(env!("CARGO_BIN_EXE_mocha-stage"))
        .args(base(&["--layer", &layer]))
        .stdin(Stdio::piped())
        .stdout(Stdio::from(full))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let data = FRAME.repeat(200);
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
