//! `track-planar` on the built program: the manifest entry, the frames that pass unchanged, the
//! poses of the data file against values worked out by hand and against the `planar-track` crate
//! called directly, the report, and every refusal of the stage. Every frame is made up.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use planar_track::{Config, Image, Pose, Region, Tracker, Velocity};
use stage_cli::keys::{KeyKind, KeySet};
use stage_cli::registry::GENERIC_CLAIM;
use stage_cli::stages::data::track_planar::{DEF, TRACKS, WINDOW};

const STAGE: &str = "track-planar";

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

/// `apply track-planar` at a size and depth, then one `--param` per pair.
fn track(size: (usize, usize, u32), params: &[(&str, &str)], input: &[u8]) -> Run {
    let (width, height, depth) = (size.0.to_string(), size.1.to_string(), size.2.to_string());
    let pairs: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
    let mut args = vec![
        "apply", STAGE, "--width", &width, "--height", &height, "--depth", &depth, "--fps", "24/1",
    ];
    for pair in &pairs {
        args.push("--param");
        args.push(pair);
    }
    spawn(&args, input)
}

/// A folder of its own in the temporary folder, removed when dropped.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let path = std::env::temp_dir().join(format!(
            "stage-cli-track-planar-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create the scratch folder");
        Folder(path)
    }

    fn path(&self, name: &str) -> String {
        self.0.join(name).to_str().expect("a text path").to_string()
    }

    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .expect("list the scratch folder")
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const W: usize = 40;
const H: usize = 32;

/// The made up scene at a pixel: three smooth channels in 0 to 1, moved by a whole pixel shift.
fn scene(x: usize, y: usize, dx: i64, dy: i64) -> [f64; 3] {
    let (u, v) = ((x as i64 - dx) as f64, (y as i64 - dy) as f64);
    [
        0.5 + ((u * 0.35).sin() + (v * 0.27).cos() + ((u + v) * 0.11).sin()) / 6.0,
        0.5 + ((u * 0.21).cos() + (v * 0.33).sin() + ((u - v) * 0.13).cos()) / 6.0,
        0.5 + ((u * 0.17).sin() + (v * 0.19).sin()) / 4.0,
    ]
}

/// One raw frame of the scene at a depth, alpha at full scale.
fn frame(depth: u32, dx: i64, dy: i64) -> Vec<u8> {
    let mut out = Vec::new();
    match depth {
        8 => {
            for y in 0..H {
                for x in 0..W {
                    let c = scene(x, y, dx, dy);
                    out.extend(c.iter().map(|v| (v * 255.0 + 0.5).floor() as u8));
                    out.push(255);
                }
            }
        }
        16 => {
            for y in 0..H {
                for x in 0..W {
                    for v in scene(x, y, dx, dy) {
                        out.extend_from_slice(&((v * 65535.0 + 0.5).floor() as u16).to_le_bytes());
                    }
                    out.extend_from_slice(&u16::MAX.to_le_bytes());
                }
            }
        }
        _ => {
            // the planes G, B, R, A
            for channel in [Some(1), Some(2), Some(0), None] {
                for y in 0..H {
                    for x in 0..W {
                        let v = channel.map_or(1.0, |c| scene(x, y, dx, dy)[c]);
                        out.extend_from_slice(&(v as f32).to_le_bytes());
                    }
                }
            }
        }
    }
    out
}

/// The shift of the scene in frame `i` of the made up clip: at most two pixels from the start.
fn shift(i: usize) -> (i64, i64) {
    (
        [0, 1, 2, 1][i - (i / 4) * 4],
        [0, 0, 1, 1][(i / 2) - (i / 8) * 4],
    )
}

fn clip(depth: u32, frames: usize) -> Vec<u8> {
    (0..frames)
        .flat_map(|i| {
            let (dx, dy) = shift(i);
            frame(depth, dx, dy)
        })
        .collect()
}

/// The grey level rule of the stage, stated again here: the weighted sum of R, G and B on the
/// samples scaled to full scale 1.
fn grey(raw: &[u8], depth: u32, weights: [f64; 3]) -> Image {
    let luma = |r: f64, g: f64, b: f64| weights[0] * r + weights[1] * g + weights[2] * b;
    let count = W * H;
    let data: Vec<f64> = match depth {
        8 => raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| {
                luma(
                    f64::from(p[0]) / 255.0,
                    f64::from(p[1]) / 255.0,
                    f64::from(p[2]) / 255.0,
                )
            })
            .collect(),
        16 => raw
            .as_chunks::<8>()
            .0
            .iter()
            .map(|p| {
                let c =
                    |i: usize| f64::from(u16::from_le_bytes([p[2 * i], p[2 * i + 1]])) / 65535.0;
                luma(c(0), c(1), c(2))
            })
            .collect(),
        _ => {
            let at = |plane: usize, i: usize| {
                let o = (plane * count + i) * 4;
                f64::from(f32::from_le_bytes([
                    raw[o],
                    raw[o + 1],
                    raw[o + 2],
                    raw[o + 3],
                ]))
            };
            (0..count)
                .map(|i| luma(at(2, i), at(0, i), at(1, i)))
                .collect()
        }
    };
    Image::new(W, H, data).unwrap()
}

const BT709: [f64; 3] = [0.2126, 0.7152, 0.0722];
const REGION: [(f64, f64); 4] = [(8.0, 6.0), (31.0, 6.0), (31.0, 25.0), (8.0, 25.0)];
const REGION_PARAMS: [(&str, &str); 8] = [
    ("x0", "8"),
    ("y0", "6"),
    ("x1", "31"),
    ("y1", "6"),
    ("x2", "31"),
    ("y2", "25"),
    ("x3", "8"),
    ("y3", "25"),
];

/// The poses the `planar-track` crate gives when one tracker holds every frame: the identity at
/// the reference, then forwards to the last frame and backwards to the first.
fn crate_poses(
    images: &[Image],
    reference: usize,
    cfg: &Config,
    region: [(f64, f64); 4],
) -> Vec<Pose> {
    let region = Region::from_quad(W, H, region).unwrap();
    let mut tracker = Tracker::new(cfg.clone(), &images[reference], &region).unwrap();
    for image in images {
        tracker.add_frame(image).unwrap();
    }
    tracker.set_pose(reference, Pose::identity()).unwrap();
    tracker.track(reference, images.len() - 1).unwrap();
    tracker.track(reference, 0).unwrap();
    (0..images.len())
        .map(|i| tracker.pose(i).unwrap())
        .collect()
}

fn read_keys(path: &str) -> KeySet {
    KeySet::parse(&std::fs::read_to_string(path).expect("read the data file")).expect("keys-1")
}

/// Every value of the data file against a list of poses, bit for bit.
fn assert_poses(path: &str, expected: &[Pose]) {
    let set = read_keys(path);
    assert_eq!(set.start(), 0.0);
    let ids: Vec<&str> = set.tracks().iter().map(|t| t.param.as_str()).collect();
    assert_eq!(ids, TRACKS);
    for (entry, track) in set.tracks().iter().enumerate() {
        assert_eq!(track.keys.len(), expected.len(), "one key per frame");
        for (index, key) in track.keys.iter().enumerate() {
            assert_eq!(key.time, index as f64);
            assert_eq!(key.kind, KeyKind::Linear);
            assert_eq!(
                key.value.to_bits(),
                expected[index].h[entry].to_bits(),
                "{} of frame {index}: {} against {}",
                track.param,
                key.value,
                expected[index].h[entry]
            );
        }
    }
}

// -- the manifest and the geometry -------------------------------------------------------------

#[test]
fn the_manifest_lists_a_generic_temporal_stage_that_preserves_geometry() {
    let manifest = String::from_utf8(spawn(&["list", "--json"], b"").out).unwrap();
    let at = manifest.find("{\"id\":\"track-planar\"").expect("listed");
    let entry = &manifest[at..];
    assert!(entry.starts_with(
        "{\"id\":\"track-planar\",\"kind\":\"temporal\",\"spool\":false,\"geometry\":\"preserve\",\"depths\":[8,16,32],\"params\":[{\"id\":\"data\",\"name\":\"Data file\",\"kind\":\"text\",\"default\":\"\","
    ));
    assert!(entry.contains("\"layers\":[],\"name\":\"Track planar\",\"description\":\""));
    assert!(entry.contains(&format!("{GENERIC_CLAIM}\",\"family\":\"generic\"}}")));
    assert_eq!(DEF.params.len(), 48);
    for param in DEF.params {
        assert!(
            entry.contains(&format!("{{\"id\":\"{}\",", param.id)),
            "{}",
            param.id
        );
        assert!(!param.keyed, "{}", param.id);
    }
    assert!(String::from_utf8(spawn(&["list"], b"").out)
        .unwrap()
        .lines()
        .any(|l| l == STAGE));
}

#[test]
fn describe_answers_the_request_and_opens_no_file() {
    let folder = Folder::new("describe");
    let data = format!("data={}", folder.path("poses.keys"));
    for depth in ["8", "16", "32"] {
        let run = spawn(
            &[
                "apply",
                STAGE,
                "--describe",
                "--width",
                "7",
                "--height",
                "5",
                "--depth",
                depth,
                "--fps",
                "30000/1001",
                "--frames",
                "9",
                "--param",
                &data,
            ],
            b"",
        );
        assert_eq!((run.code, run.err.as_str()), (0, ""));
        assert_eq!(
            String::from_utf8(run.out).unwrap(),
            format!(
                "{{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"track-planar\",\"width\":7,\"height\":5,\"depth\":{depth},\"fps\":\"30000/1001\",\"frames\":9}}\n"
            )
        );
    }
    assert!(folder.names().is_empty());
}

// -- the frames ----------------------------------------------------------------------------------

#[test]
fn the_frames_pass_unchanged_at_every_depth() {
    let folder = Folder::new("pass");
    let data = folder.path("poses.keys");
    let report = folder.path("report.txt");
    for depth in [8, 16, 32] {
        let input = clip(depth, 3);
        let plain = track((W, H, depth), &[], &input);
        assert_eq!((plain.code, plain.err.as_str()), (0, ""), "{depth}");
        assert!(plain.out == input, "{depth}: no data file asked for");
        let tracked = track(
            (W, H, depth),
            &[("data", &data), ("report", &report)],
            &input,
        );
        assert_eq!((tracked.code, tracked.err.as_str()), (0, ""), "{depth}");
        assert!(tracked.out == input, "{depth}: with a data file");
    }
    assert_eq!(folder.names(), ["poses.keys", "report.txt"]);
}

#[test]
fn bytes_that_are_no_picture_pass_unchanged_too() {
    let folder = Folder::new("noise");
    let data = folder.path("poses.keys");
    for (depth, bytes) in [(8u32, 4usize), (16, 8), (32, 16)] {
        let input: Vec<u8> = (0..6 * 5 * bytes * 4)
            .map(|i| (i as u8).wrapping_mul(29).wrapping_add(7))
            .collect();
        let run = track((6, 5, depth), &[("data", &data)], &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""), "{depth}");
        assert!(run.out == input, "{depth}");
        assert_eq!(read_keys(&data).tracks().len(), 9, "{depth}");
    }
}

// -- the poses -----------------------------------------------------------------------------------

/// Worked by hand: a clip of one repeated frame has zero residual at the identity, so every
/// level stops at once with the pose it was given, and every pose is the identity exactly.
#[test]
fn a_still_clip_gives_the_identity_pose_exactly() {
    let folder = Folder::new("still");
    let data = folder.path("poses.keys");
    let report = folder.path("report.txt");
    let still = frame(8, 0, 0);
    let input: Vec<u8> = still
        .iter()
        .copied()
        .cycle()
        .take(still.len() * 3)
        .collect();
    let run = track((W, H, 8), &[("data", &data), ("report", &report)], &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    let mut expected = String::from(
        "keys-1\n# track-planar: m00 to m22, row major, from the reference frame to frame N\nstart 0\n",
    );
    for index in 0..3 {
        for (track, value) in TRACKS.iter().zip([1, 0, 0, 0, 1, 0, 0, 0, 1]) {
            expected.push_str(&format!("key {track} {index} {value} linear\n"));
        }
    }
    assert_eq!(std::fs::read_to_string(&data).unwrap(), expected);
    let table = std::fs::read_to_string(&report).unwrap();
    let rows: Vec<Vec<&str>> = table
        .lines()
        .map(|l| l.split(' ').collect::<Vec<&str>>())
        .collect();
    assert_eq!(rows[0], ["track-report-1"]);
    assert_eq!(
        table.lines().nth(1).unwrap(),
        "# frame source stop converged iterations rms valid_fraction inlier_fraction gain offset"
    );
    assert_eq!(
        rows[2],
        ["0", "reference", "none", "1", "0", "0", "1", "1", "0", "0"]
    );
    for (row, index) in rows[3..].iter().zip(["1", "2"]) {
        assert_eq!(row.len(), 10);
        assert_eq!(row[..4], [index, "tracked", "cost-tol", "1"]);
        assert_eq!(row[5..], ["0", "1", "1", "0", "0"]);
    }
    assert_eq!(rows.len(), 5);
}

/// Worked by hand: the scene of frame 1 is the scene of frame 0 moved two pixels right and one
/// down, so a point of the reference is found two pixels right and one down.
#[test]
fn a_known_shift_is_recovered() {
    let folder = Folder::new("shift");
    let data = folder.path("poses.keys");
    let mut input = frame(8, 0, 0);
    input.extend(frame(8, 2, 1));
    let mut params = REGION_PARAMS.to_vec();
    params.extend([("data", data.as_str()), ("dof", "translation")]);
    let run = track((W, H, 8), &params, &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    let set = read_keys(&data);
    let at = |id: &str| set.value(id, 1).unwrap();
    assert!((at("m02") - 2.0).abs() < 0.05, "{}", at("m02"));
    assert!((at("m12") - 1.0).abs() < 0.05, "{}", at("m12"));
    for (id, want) in [("m00", 1.0), ("m01", 0.0), ("m10", 0.0), ("m11", 1.0)] {
        assert_eq!(at(id), want, "{id}: a translation keeps the linear part");
    }
    assert_eq!((at("m20"), at("m21"), at("m22")), (0.0, 0.0, 1.0));
}

#[test]
fn the_data_file_holds_the_poses_of_the_crate_bit_for_bit() {
    let folder = Folder::new("crate");
    let data = folder.path("poses.keys");
    let frames = 2 * WINDOW + 3;
    let input = clip(8, frames);
    let images: Vec<Image> = input
        .as_chunks::<{ W * H * 4 }>()
        .0
        .iter()
        .map(|raw| grey(raw, 8, BT709))
        .collect();
    let mut params = REGION_PARAMS.to_vec();
    params.push(("data", data.as_str()));
    let run = track((W, H, 8), &params, &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert!(run.out == input);
    assert_poses(&data, &crate_poses(&images, 0, &Config::default(), REGION));
    let first = std::fs::read(&data).unwrap();
    let again = track((W, H, 8), &params, &input);
    assert_eq!(again.code, 0);
    assert!(std::fs::read(&data).unwrap() == first, "two runs agree");
}

#[test]
fn a_later_reference_tracks_backwards_and_forwards_as_the_crate_does() {
    let folder = Folder::new("reference");
    let data = folder.path("poses.keys");
    let report = folder.path("report.txt");
    let frames = WINDOW + 9;
    let input = clip(8, frames);
    let images: Vec<Image> = input
        .as_chunks::<{ W * H * 4 }>()
        .0
        .iter()
        .map(|raw| grey(raw, 8, BT709))
        .collect();
    let cfg = Config {
        velocity: Velocity::ConstantImageSpace,
        ..Config::default()
    };
    let mut params = REGION_PARAMS.to_vec();
    params.extend([
        ("data", data.as_str()),
        ("report", report.as_str()),
        ("reference", "5"),
        ("velocity", "constant-image-space"),
    ]);
    let run = track((W, H, 8), &params, &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert!(run.out == input);
    assert_poses(&data, &crate_poses(&images, 5, &cfg, REGION));
    let table = std::fs::read_to_string(&report).unwrap();
    let rows: Vec<&str> = table.lines().skip(2).collect();
    assert_eq!(rows.len(), frames);
    for (index, row) in rows.iter().enumerate() {
        let source = if index == 5 { "reference" } else { "tracked" };
        assert!(row.starts_with(&format!("{index} {source} ")), "{row}");
    }
}

#[test]
fn the_grey_rule_holds_at_sixteen_and_thirty_two_bits_and_with_other_weights() {
    let folder = Folder::new("depths");
    let data = folder.path("poses.keys");
    let weights = [0.5, 0.25, 0.125];
    for (depth, bytes) in [(16u32, 8usize), (32, 16)] {
        let input = clip(depth, 5);
        let images: Vec<Image> = input
            .chunks_exact(W * H * bytes)
            .map(|raw| grey(raw, depth, weights))
            .collect();
        let mut params = REGION_PARAMS.to_vec();
        params.extend([
            ("data", data.as_str()),
            ("luma_r", "0.5"),
            ("luma_g", "0.25"),
            ("luma_b", "0.125"),
        ]);
        let run = track((W, H, depth), &params, &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""), "{depth}");
        assert_poses(&data, &crate_poses(&images, 0, &Config::default(), REGION));
    }
}

#[test]
fn a_parameter_of_the_method_reaches_the_tracker() {
    let folder = Folder::new("method");
    let data = folder.path("poses.keys");
    let input = clip(8, 4);
    let images: Vec<Image> = input
        .as_chunks::<{ W * H * 4 }>()
        .0
        .iter()
        .map(|raw| grey(raw, 8, BT709))
        .collect();
    let cfg = Config {
        dof: planar_track::Dof::Homography,
        rule: planar_track::UpdateRule::Esm,
        illumination: planar_track::Illumination::Joint,
        ..Config::default()
    };
    let mut params = REGION_PARAMS.to_vec();
    params.extend([
        ("data", data.as_str()),
        ("dof", "homography"),
        ("rule", "esm"),
        ("illumination", "joint"),
    ]);
    let run = track((W, H, 8), &params, &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    let expected = crate_poses(&images, 0, &cfg, REGION);
    assert_poses(&data, &expected);
    assert_ne!(
        expected,
        crate_poses(&images, 0, &Config::default(), REGION),
        "the clip tells the two configurations apart"
    );
}

#[test]
fn the_whole_frame_is_the_region_when_no_corner_is_given() {
    let folder = Folder::new("whole");
    let data = folder.path("poses.keys");
    let input = clip(8, 3);
    let images: Vec<Image> = input
        .as_chunks::<{ W * H * 4 }>()
        .0
        .iter()
        .map(|raw| grey(raw, 8, BT709))
        .collect();
    let run = track((W, H, 8), &[("data", &data)], &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    let whole = [
        (0.0, 0.0),
        (W as f64 - 1.0, 0.0),
        (W as f64 - 1.0, H as f64 - 1.0),
        (0.0, H as f64 - 1.0),
    ];
    assert_poses(&data, &crate_poses(&images, 0, &Config::default(), whole));
}

// -- the sink ------------------------------------------------------------------------------------

#[test]
fn an_empty_input_writes_files_without_rows() {
    let folder = Folder::new("empty");
    let data = folder.path("poses.keys");
    let report = folder.path("report.txt");
    let run = track((W, H, 8), &[("data", &data), ("report", &report)], b"");
    assert_eq!((run.code, run.out.len(), run.err.as_str()), (0, 0, ""));
    assert_eq!(
        std::fs::read_to_string(&data).unwrap(),
        "keys-1\n# track-planar: m00 to m22, row major, from the reference frame to frame N\nstart 0\n"
    );
    assert!(read_keys(&data).tracks().is_empty());
    assert_eq!(std::fs::read_to_string(&report).unwrap().lines().count(), 2);
}

#[test]
fn a_failed_run_leaves_no_data_file_and_no_temporary_file() {
    let folder = Folder::new("failed");
    let data = folder.path("poses.keys");
    let input = clip(8, 2);
    let partial = track((W, H, 8), &[("data", &data)], &input[..input.len() - 5]);
    assert_eq!(partial.code, 4);
    assert!(
        partial.err.starts_with("error: partial_frame: "),
        "{}",
        partial.err
    );
    assert!(
        partial.out == input[..W * H * 4],
        "the whole frame before it is written"
    );
    assert!(folder.names().is_empty(), "{:?}", folder.names());
    let count = spawn(
        &[
            "apply",
            STAGE,
            "--width",
            "40",
            "--height",
            "32",
            "--depth",
            "8",
            "--fps",
            "24",
            "--frames",
            "3",
            "--param",
            &format!("data={data}"),
        ],
        &input,
    );
    assert_eq!(count.code, 4);
    assert!(
        count
            .err
            .starts_with("error: bad_input: the input holds 2 frames"),
        "{}",
        count.err
    );
    assert!(folder.names().is_empty());
    let late = track((W, H, 8), &[("data", &data), ("reference", "2")], &input);
    assert_eq!(late.code, 4);
    assert_eq!(
        late.err,
        "error: bad_input: the reference frame 2 is not among the 2 frames read\n"
    );
    assert!(
        late.out == input,
        "the frames passed before the input ended"
    );
    assert!(folder.names().is_empty());
}

#[test]
fn a_failed_run_keeps_an_earlier_data_file() {
    let folder = Folder::new("earlier");
    let data = folder.path("poses.keys");
    std::fs::write(&data, "earlier\n").unwrap();
    let input = clip(8, 1);
    let run = track((W, H, 8), &[("data", &data)], &input[..100]);
    assert_eq!(run.code, 4);
    assert_eq!(std::fs::read_to_string(&data).unwrap(), "earlier\n");
    assert_eq!(folder.names(), ["poses.keys"]);
    let run = track((W, H, 8), &[("data", &data)], &input);
    assert_eq!(run.code, 0);
    assert!(std::fs::read_to_string(&data)
        .unwrap()
        .starts_with("keys-1\n"));
}

#[test]
fn a_data_file_that_cannot_be_written_is_stage_failed_before_the_first_frame() {
    let folder = Folder::new("unwritable");
    let input = clip(8, 1);
    let missing = folder.path("no-such-folder/poses.keys");
    let run = track((W, H, 8), &[("data", &missing)], &input);
    assert_eq!((run.code, run.out.len()), (1, 0));
    assert!(
        run.err.starts_with("error: stage_failed: cannot create `"),
        "{}",
        run.err
    );
    let as_folder = folder.0.to_str().unwrap().to_string();
    let run = track((W, H, 8), &[("report", &as_folder)], &input);
    assert_eq!((run.code, run.out.len()), (1, 0));
    assert!(run.err.starts_with("error: stage_failed: "), "{}", run.err);
    assert!(run.err.ends_with("the path is a folder\n"), "{}", run.err);
    assert_eq!(run.err.matches('\n').count(), 1);
}

// -- the refusals --------------------------------------------------------------------------------

#[test]
fn one_path_for_both_files_is_usage() {
    let folder = Folder::new("same");
    let path = folder.path("both.txt");
    let run = track(
        (W, H, 8),
        &[("data", &path), ("report", &path)],
        &clip(8, 1),
    );
    assert_eq!((run.code, run.out.len()), (2, 0));
    assert_eq!(
        run.err,
        "error: usage: track-planar: data and report name the same file\n"
    );
    assert!(folder.names().is_empty());
}

#[test]
fn a_configuration_the_crate_refuses_is_unsupported_for_a_run_and_for_describe() {
    for (id, value) in [
        ("pyramid_taps", "4"),
        ("pyramid_scale", "1"),
        ("lambda_up", "1"),
        ("lambda_down", "0"),
        ("selection_fraction", "0"),
        ("min_weight", "0"),
        ("ncc_min_valid_fraction", "0"),
    ] {
        let run = track((W, H, 8), &[(id, value)], &clip(8, 1));
        assert_eq!((run.code, run.out.len()), (3, 0), "{id}");
        assert!(
            run.err
                .starts_with("error: unsupported: track-planar: bad configuration: "),
            "{id}: {}",
            run.err
        );
        assert_eq!(run.err.matches('\n').count(), 1);
        let pair = format!("{id}={value}");
        let describe = spawn(
            &[
                "apply",
                STAGE,
                "--describe",
                "--width",
                "40",
                "--height",
                "32",
                "--depth",
                "8",
                "--fps",
                "24",
                "--param",
                &pair,
            ],
            b"",
        );
        assert_eq!((describe.code, describe.out.len()), (3, 0), "{id}");
    }
}

#[test]
fn corners_that_do_not_go_around_a_convex_region_are_unsupported() {
    let crossed = [
        ("x0", "8"),
        ("y0", "6"),
        ("x1", "31"),
        ("y1", "25"),
        ("x2", "31"),
        ("y2", "6"),
        ("x3", "8"),
        ("y3", "25"),
    ];
    let run = track((W, H, 8), &crossed, &clip(8, 1));
    assert_eq!((run.code, run.out.len()), (3, 0));
    assert_eq!(
        run.err,
        "error: unsupported: track-planar: the four corners do not go around a convex region\n"
    );
    let line = [("x1", "5"), ("x2", "10"), ("x3", "15")];
    assert_eq!(track((W, H, 8), &line, &clip(8, 1)).code, 3);
}

#[test]
fn values_follow_the_rows_of_the_convention() {
    let input = clip(8, 1);
    for (id, value) in [
        ("dof", "rigid"),
        ("rule", "newton"),
        ("illumination", "on"),
        ("damping", "none"),
        ("acceptance", "always"),
        ("boundary", "cubic"),
        ("velocity", "on"),
        ("refusal", "skip"),
        ("reference", "-1"),
        ("reference", "1.5"),
        ("luma_r", "2"),
        ("gain_ratio", "1"),
        ("boundary_ramp", "0"),
        ("pyramid_levels", "0"),
        ("pyramid_taps", "17"),
        ("max_iterations", "0"),
        ("ncc", "maybe"),
        ("ncc_level", "-2"),
        ("x0", "inf"),
        ("x0", "1@0;2@5"),
        ("no_such_param", "1"),
    ] {
        let run = track((W, H, 8), &[(id, value)], &input);
        assert_eq!(
            (run.code, run.out.len()),
            (3, 0),
            "{id}={value}: {}",
            run.err
        );
        assert!(
            run.err.starts_with("error: unsupported: "),
            "{id}: {}",
            run.err
        );
        assert_eq!(run.err.matches('\n').count(), 1);
    }
    let twice = track((W, H, 8), &[("dof", "affine"), ("dof", "affine")], &input);
    assert_eq!(twice.code, 2);
    assert!(twice
        .err
        .starts_with("error: usage: parameter `dof` given more than once"));
    let layer = spawn(
        &[
            "apply",
            STAGE,
            "--width",
            "40",
            "--height",
            "32",
            "--depth",
            "8",
            "--fps",
            "24",
            "--layer",
            "keys=poses.keys",
        ],
        &input,
    );
    assert_eq!(layer.code, 3);
    assert!(
        layer
            .err
            .starts_with("error: unsupported: track-planar takes no layers"),
        "{}",
        layer.err
    );
    let depth = spawn(
        &[
            "apply", STAGE, "--width", "40", "--height", "32", "--depth", "12", "--fps", "24",
        ],
        &input,
    );
    assert_eq!(depth.code, 3);
}

/// Worked by hand: a region that covers no pixel of the frame cannot be tracked, so every frame
/// is refused and holds the pose of the reference, the identity.
#[test]
fn a_region_the_tracker_refuses_holds_the_pose_or_fails_the_run() {
    let folder = Folder::new("refused");
    let data = folder.path("poses.keys");
    let report = folder.path("report.txt");
    let input = clip(8, 3);
    let outside = [
        ("x0", "100"),
        ("y0", "100"),
        ("x1", "110"),
        ("y1", "100"),
        ("x2", "110"),
        ("y2", "110"),
        ("x3", "100"),
        ("y3", "110"),
    ];
    let mut params = outside.to_vec();
    params.extend([
        ("data", data.as_str()),
        ("report", report.as_str()),
        ("reference", "1"),
    ]);
    let held = track((W, H, 8), &params, &input);
    assert_eq!((held.code, held.err.as_str()), (0, ""));
    assert!(held.out == input);
    let mut expected = String::from(
        "keys-1\n# track-planar: m00 to m22, row major, from the reference frame to frame N\nstart 0\n",
    );
    for index in 0..3 {
        for (track, value) in TRACKS.iter().zip([1, 0, 0, 0, 1, 0, 0, 0, 1]) {
            expected.push_str(&format!("key {track} {index} {value} linear\n"));
        }
    }
    assert_eq!(std::fs::read_to_string(&data).unwrap(), expected);
    let mut table = String::from(
        "track-report-1\n# frame source stop converged iterations rms valid_fraction inlier_fraction gain offset\n",
    );
    for index in 0..3 {
        table.push_str(&format!(
            "# frame {index}: bad image or region: region contains no pixel\n{index} held refused 0 0 0 0 0 0 0\n"
        ));
    }
    assert_eq!(std::fs::read_to_string(&report).unwrap(), table);
    std::fs::remove_file(&data).unwrap();
    std::fs::remove_file(&report).unwrap();
    params.push(("refusal", "fail"));
    let failed = track((W, H, 8), &params, &input);
    assert_eq!(failed.code, 3);
    assert_eq!(
        failed.err,
        "error: unsupported: frame 0: the tracker refuses the frame: bad image or region: region contains no pixel\n"
    );
    assert!(
        failed.out == input[..W * H * 4],
        "the frame before the reference was written before the refusal"
    );
    assert!(folder.names().is_empty(), "{:?}", folder.names());
}

#[test]
fn a_frame_of_one_pixel_runs() {
    let folder = Folder::new("pixel");
    let data = folder.path("poses.keys");
    let input = [10u8, 20, 30, 255, 40, 50, 60, 255];
    for refusal in ["hold", "fail"] {
        let run = track((1, 1, 8), &[("data", &data), ("refusal", refusal)], &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""), "{refusal}");
        assert!(run.out == input);
        let set = read_keys(&data);
        assert_eq!(set.tracks().len(), 9);
        assert!(set.tracks().iter().all(|t| t.keys.len() == 2));
    }
}

#[test]
fn a_sample_that_is_not_a_number_is_a_refused_frame() {
    let folder = Folder::new("nan");
    let data = folder.path("poses.keys");
    let report = folder.path("report.txt");
    let mut input = clip(32, 3);
    let one = W * H * 16;
    input[one..one + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    let mut params = REGION_PARAMS.to_vec();
    params.extend([("data", data.as_str()), ("report", report.as_str())]);
    let run = track((W, H, 32), &params, &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert!(run.out == input, "the frame itself passes unchanged");
    let set = read_keys(&data);
    let pose: Vec<f64> = TRACKS.iter().map(|id| set.value(id, 1).unwrap()).collect();
    assert_eq!(
        pose,
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        "frame 1 holds the pose of frame 0"
    );
    let table = std::fs::read_to_string(&report).unwrap();
    assert!(
        table.contains(
            "\n# frame 1: bad image or region: non finite sample\n1 held refused 0 0 0 0 0 0 0\n"
        ),
        "{table}"
    );
    assert!(
        table.lines().last().unwrap().starts_with("2 tracked "),
        "{table}"
    );
    // frame 2 is tracked from frame 0, as a tracker that never saw frame 1 does
    let images: Vec<Image> = [&input[..one], &input[2 * one..]]
        .iter()
        .map(|raw| grey(raw, 32, BT709))
        .collect();
    let expected = crate_poses(&images, 0, &Config::default(), REGION);
    let second: Vec<f64> = TRACKS.iter().map(|id| set.value(id, 2).unwrap()).collect();
    assert_eq!(second, expected[1].h);
    params.push(("refusal", "fail"));
    let failed = track((W, H, 32), &params, &input);
    assert_eq!(failed.code, 3);
    assert_eq!(
        failed.err,
        "error: unsupported: frame 1: the tracker refuses the frame: bad image or region: non finite sample\n"
    );
    assert!(
        failed.out == input[..one],
        "frame 0 was written before the refusal"
    );
}
