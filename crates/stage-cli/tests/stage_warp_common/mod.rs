//! What the tests of the warp stages share: running the built program, made up frames, scratch
//! files, the reference a stage is compared with, and the checks every warp stage passes.
#![allow(dead_code)]

pub mod interface;

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use stage_cli::pixels;

/// What one run of the program left.
pub struct Run {
    pub code: i32,
    pub out: Vec<u8>,
    pub err: String,
}

/// Run the built program with `args`, feeding `input` on standard input.
pub fn spawn(args: &[&str], input: &[u8]) -> Run {
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

/// The words of `apply STAGE` at a size and depth, then one `--param` per pair, then `extra`.
pub fn words(
    stage: &str,
    size: (u32, u32, u32),
    params: &[(&str, &str)],
    extra: &[&str],
) -> Vec<String> {
    let mut args: Vec<String> = ["apply", stage, "--width"].map(String::from).to_vec();
    args.push(size.0.to_string());
    args.push("--height".into());
    args.push(size.1.to_string());
    args.push("--depth".into());
    args.push(size.2.to_string());
    args.push("--fps".into());
    args.push("24/1".into());
    for (k, v) in params {
        args.push("--param".into());
        args.push(format!("{k}={v}"));
    }
    args.extend(extra.iter().map(|w| w.to_string()));
    args
}

/// Run `apply STAGE` as [`words`] builds it.
pub fn apply(
    stage: &str,
    size: (u32, u32, u32),
    params: &[(&str, &str)],
    extra: &[&str],
    input: &[u8],
) -> Run {
    let args = words(stage, size, params, extra);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    spawn(&refs, input)
}

/// Bytes per pixel of a depth.
pub fn bytes_per_pixel(depth: u32) -> usize {
    match depth {
        8 => 4,
        16 => 8,
        _ => 16,
    }
}

/// A made up clip of `frames` frames. Depth 8 and 16 hold a byte pattern; depth 32 holds
/// floats on a grid of 1/32 from -0.5 to 2.5.
pub fn clip(size: (u32, u32, u32), frames: usize, seed: u8) -> Vec<u8> {
    let (w, h, depth) = size;
    let samples = (w * h) as usize * 4 * frames;
    match depth {
        8 => (0..samples)
            .map(|i| (i as u8).wrapping_mul(29).wrapping_add(seed))
            .collect(),
        16 => (0..samples * 2)
            .map(|i| (i as u8).wrapping_mul(29).wrapping_add(seed))
            .collect(),
        _ => (0..samples)
            .flat_map(|i| (((i + seed as usize) % 97) as f32 * 0.03125 - 0.5).to_le_bytes())
            .collect(),
    }
}

/// One depth 8 frame from pixels given as one value each: R, G and B hold the value, alpha is
/// 255, so a frame can be written as a small grid of numbers.
pub fn grey(values: &[u8]) -> Vec<u8> {
    values.iter().flat_map(|v| [*v, *v, *v, 255]).collect()
}

/// As [`grey`], with a zero value standing for a pixel that is zero in all four channels.
pub fn grey_or_zero(values: &[u8]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|v| {
            if *v == 0 {
                [0, 0, 0, 0]
            } else {
                [*v, *v, *v, 255]
            }
        })
        .collect()
}

/// A file with made up content in the temporary folder, removed when dropped.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(name: &str, content: &str) -> Scratch {
        let path =
            std::env::temp_dir().join(format!("stage-cli-warp-{}-{name}", std::process::id()));
        std::fs::write(&path, content).expect("write the scratch file");
        Scratch(path)
    }

    /// The `ID=PATH` word of a `--layer` option.
    pub fn layer(&self, id: &str) -> String {
        format!("{id}={}", self.0.display())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// What a warp stage must write for `input`: each frame decoded by the pixel rule of the
/// program, sampled by the `resample` crate through `map` under `cfg`, and encoded. `map`
/// receives the frame index and an output position and answers the source position.
pub fn reference<M>(
    input: &[u8],
    src: (u32, u32, u32),
    dst: (u32, u32),
    cfg: &resample::Config,
    map: M,
) -> Vec<u8>
where
    M: Fn(usize, f64, f64) -> Option<(f64, f64)>,
{
    let (w, h, depth) = src;
    let frame_bytes = (w * h) as usize * bytes_per_pixel(depth);
    let layout = match pixels::layout(depth) {
        pixels::Layout::Packed => resample::Layout::Packed,
        pixels::Layout::Planar => resample::Layout::Planar,
    };
    let mut all = Vec::new();
    for (index, frame) in input.chunks_exact(frame_bytes).enumerate() {
        let samples = pixels::decode(frame, depth);
        let image = resample::Image {
            data: &samples,
            width: w as usize,
            height: h as usize,
            channels: 4,
            layout,
        };
        let made = resample::resample(
            &image,
            dst.0 as usize,
            dst.1 as usize,
            |x, y| map(index, x, y).unwrap_or((f64::NAN, f64::NAN)),
            cfg,
        )
        .expect("the reference resamples");
        let mut out = Vec::new();
        pixels::encode(&made, depth, &mut out);
        all.extend(out);
    }
    all
}

/// The manifest entry of `stage`, cut out of `list --json`.
pub fn manifest_entry(stage: &str) -> String {
    let manifest = String::from_utf8(spawn(&["list", "--json"], b"").out).unwrap();
    let start = manifest
        .find(&format!("{{\"id\":\"{stage}\","))
        .unwrap_or_else(|| panic!("{stage} is not in the manifest"));
    let rest = &manifest[start..];
    let end = rest
        .find("\"family\":\"generic\"}")
        .expect("the family key")
        + 19;
    rest[..end].to_string()
}

/// The parameter ids of a manifest entry, in order.
pub fn param_ids(entry: &str) -> Vec<String> {
    let params = &entry[entry.find("\"params\":[").unwrap()..entry.find("\"layers\":[").unwrap()];
    params
        .split("{\"id\":\"")
        .skip(1)
        .map(|p| p[..p.find('"').unwrap()].to_string())
        .collect()
}

/// The checks of the manifest entry that hold for every warp stage: a generic frame stage with
/// declared geometry, the three depths, the shared parameters first, one optional key layer.
pub fn check_the_entry(stage: &str, name: &str, own: &[&str]) {
    let keys = "{\"id\":\"keys\",\"optional\":true,\"form\":\"keys-1\",";
    check_the_entry_with_layer(stage, name, own, keys);
}

/// [`check_the_entry`] for a stage whose one layer starts with the text `layer` in the manifest.
pub fn check_the_entry_with_layer(stage: &str, name: &str, own: &[&str], layer: &str) {
    let entry = manifest_entry(stage);
    assert!(
        entry.starts_with(&format!(
            "{{\"id\":\"{stage}\",\"kind\":\"frame\",\"spool\":false,\"geometry\":\"declared\",\"depths\":[8,16,32],\"params\":["
        )),
        "{entry}"
    );
    assert!(entry.contains(&format!("\"name\":\"{name}\"")), "{entry}");
    assert!(
        entry.ends_with(
            "A generic stage: a published method under explicit configuration, not compared with any product.\",\"family\":\"generic\"}"
        ),
        "{entry}"
    );
    assert!(entry.contains(&format!("\"layers\":[{layer}")), "{entry}");
    assert_eq!(
        entry[entry.find("\"layers\":[").unwrap()..]
            .matches("{\"id\":")
            .count(),
        1,
        "one layer"
    );
    let shared = [
        "width",
        "height",
        "filter",
        "keys_a",
        "mitchell_b",
        "mitchell_c",
        "lobes",
        "border",
        "center",
        "alpha",
        "normalize",
        "tie",
        "order",
        "footprint",
    ];
    let ids = param_ids(&entry);
    let want: Vec<&str> = shared.iter().chain(own.iter()).copied().collect();
    assert_eq!(ids, want);
}

/// The `--describe` answers of a warp stage: the input size at the defaults, the asked size
/// otherwise, the depth, the rate and the frame count of the request, and no layer opened.
pub fn check_describe(stage: &str) {
    let entry = manifest_entry(stage);
    let layers = &entry[entry.find("\"layers\":[{\"id\":\"").expect("a layer") + 17..];
    let no_file = format!("{}=no/such/file", &layers[..layers.find('"').unwrap()]);
    for depth in [8, 16, 32] {
        let same = apply(stage, (5, 3, depth), &[], &["--describe"], b"");
        assert_eq!((same.code, same.err.as_str()), (0, ""));
        assert_eq!(
            String::from_utf8(same.out).unwrap(),
            format!("{{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"{stage}\",\"width\":5,\"height\":3,\"depth\":{depth},\"fps\":\"24/1\",\"frames\":null}}\n")
        );
        let asked = apply(
            stage,
            (5, 3, depth),
            &[("width", "9"), ("height", "2")],
            &["--describe", "--frames", "7", "--layer", no_file.as_str()],
            b"",
        );
        assert_eq!((asked.code, asked.err.as_str()), (0, ""));
        assert_eq!(
            String::from_utf8(asked.out).unwrap(),
            format!("{{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"{stage}\",\"width\":9,\"height\":2,\"depth\":{depth},\"fps\":\"24/1\",\"frames\":7}}\n")
        );
    }
    let one_side = apply(stage, (5, 3, 8), &[("height", "4")], &["--describe"], b"");
    assert!(String::from_utf8(one_side.out)
        .unwrap()
        .contains("\"width\":5,\"height\":4,"));
}

/// One refusal: the parameters, the exit code and a part of the error line.
pub type Refusal<'a> = (&'a [(&'a str, &'a str)], i32, &'a str);

/// Run each refusal with and without `--describe`: the same code, one error line that holds
/// `needle`, and no output.
pub fn check_refusals(stage: &str, cases: &[Refusal<'_>]) {
    let size = (3, 2, 8);
    let frame = clip(size, 1, 5);
    for (params, code, needle) in cases {
        for describe in [false, true] {
            let extra: &[&str] = if describe { &["--describe"] } else { &[] };
            let run = apply(stage, size, params, extra, &frame);
            assert_eq!(run.code, *code, "{params:?}: {}", run.err);
            assert!(run.err.starts_with("error: "), "{params:?}: {}", run.err);
            assert!(run.err.contains(needle), "{params:?}: {}", run.err);
            assert_eq!(run.err.matches('\n').count(), 1, "{params:?}: {}", run.err);
            assert!(run.out.is_empty(), "{params:?}");
        }
    }
}

/// The refusals every warp stage shares: the shared parameters, the stream and the key file.
/// `keyed` is a keyed parameter of the stage with a value outside its range, `fixed` a parameter
/// that is not keyed.
pub fn check_the_shared_refusals(stage: &str, keyed: (&str, &str), fixed: &str) {
    check_the_stream_refusals(stage);
    check_the_key_refusals(stage, keyed, fixed);
}

/// The refusals of the shared parameters and of the stream, which hold with or without a key
/// file layer.
pub fn check_the_stream_refusals(stage: &str) {
    let no_param = format!("{stage} has no parameter `gain`: it takes width, height, filter");
    check_refusals(
        stage,
        &[
            (
                &[("width", "-1")],
                3,
                "bad value for `width`: -1 is outside 0 to 65536",
            ),
            (&[("height", "65537")], 3, "bad value for `height`"),
            (
                &[("width", "x")],
                3,
                "bad value for `width`: cannot parse `x`",
            ),
            (
                &[("filter", "cubic")],
                3,
                "`cubic` is not one of nearest, bilinear",
            ),
            (&[("keys_a", "0.5")], 3, "bad value for `keys_a`"),
            (&[("mitchell_b", "2")], 3, "bad value for `mitchell_b`"),
            (&[("mitchell_c", "-1")], 3, "bad value for `mitchell_c`"),
            (&[("lobes", "0")], 3, "bad value for `lobes`"),
            (&[("border", "wrap")], 3, "bad value for `border`"),
            (&[("center", "middle")], 3, "bad value for `center`"),
            (&[("alpha", "none")], 3, "bad value for `alpha`"),
            (&[("normalize", "yes")], 3, "bad value for `normalize`"),
            (&[("tie", "up")], 3, "bad value for `tie`"),
            (&[("order", "rows")], 3, "bad value for `order`"),
            (&[("footprint", "wide")], 3, "bad value for `footprint`"),
            (&[("gain", "2")], 3, no_param.as_str()),
            (
                &[("width", "4"), ("width", "5")],
                2,
                "parameter `width` given more than once",
            ),
            (&[("width", "1@0;2@5")], 3, "--layer keys=PATH"),
        ],
    );
    let size = (3, 2, 8);
    let frame = clip(size, 1, 3);
    let depth = apply(stage, (3, 2, 12), &[], &[], &frame);
    assert_eq!(depth.code, 3);
    assert!(
        depth.err.starts_with("error: unsupported: depth 12"),
        "{}",
        depth.err
    );
    let option = apply(stage, size, &[], &["--no-such-option"], &frame);
    assert_eq!(option.code, 2);
    assert!(option.err.starts_with("error: usage: "), "{}", option.err);
    let partial = apply(stage, size, &[], &[], &[&frame[..], &frame[..7]].concat());
    assert_eq!(partial.code, 4);
    assert!(
        partial.err.starts_with("error: partial_frame: frame 1: "),
        "{}",
        partial.err
    );
    assert_eq!(
        partial.out.len(),
        frame.len(),
        "the whole frame before it is written"
    );
    let count = apply(stage, size, &[], &["--frames", "2"], &frame);
    assert_eq!(count.code, 4);
    assert!(
        count
            .err
            .starts_with("error: bad_input: the input holds 1 frames"),
        "{}",
        count.err
    );
    let empty = apply(stage, size, &[], &[], b"");
    assert_eq!(
        (empty.code, empty.out.len(), empty.err.as_str()),
        (0, 0, "")
    );
    let huge = apply(
        stage,
        size,
        &[("width", "65536"), ("height", "65536")],
        &[],
        &frame,
    );
    assert_eq!(huge.code, 3);
    assert!(huge.err.contains("more than 4 GiB"), "{}", huge.err);
    let unknown = apply(stage, size, &[], &["--layer", "matte=x"], &frame);
    assert_eq!(unknown.code, 3, "{}", unknown.err);
    assert!(unknown.out.is_empty());
}

/// The refusals of a stage with keyed parameters: a keyed value on the command line, a value
/// outside the range, and every refusal of the key file.
pub fn check_the_key_refusals(stage: &str, keyed: (&str, &str), fixed: &str) {
    check_refusals(
        stage,
        &[
            (&[(keyed.0, "1@0;2@5")], 3, "--layer keys=PATH"),
            (&[(keyed.0, keyed.1)], 3, "is outside"),
            (&[(keyed.0, "nan")], 3, "bad value for"),
        ],
    );
    let size = (3, 2, 8);
    let frame = clip(size, 1, 3);
    let twice = apply(
        stage,
        size,
        &[],
        &["--layer", "keys=a", "--layer", "keys=b"],
        &frame,
    );
    assert_eq!(twice.code, 2, "{}", twice.err);
    let missing = apply(
        stage,
        size,
        &[],
        &["--layer", "keys=no/such/key/file"],
        &frame,
    );
    assert_eq!(missing.code, 4);
    assert!(
        missing
            .err
            .starts_with("error: bad_input: cannot read the key file"),
        "{}",
        missing.err
    );
    let range = format!("keys-1\nkey {} 0 {} hold\n", keyed.0, keyed.1);
    let not_keyed = format!("keys-1\nkey {fixed} 0 1 hold\n");
    let needle = format!("no keyed parameter `{fixed}`");
    for (name, content, code, needle) in [
        (
            "form",
            "keys-2\n",
            4,
            "line 1: the first line is not `keys-1`",
        ),
        ("kind", "keys-1\nkey width 0 1 bezier\n", 4, "line 2: "),
        ("fixed", not_keyed.as_str(), 3, needle.as_str()),
        (
            "other",
            "keys-1\nkey gain 0 1 hold\n",
            3,
            "no keyed parameter `gain`",
        ),
        ("range", range.as_str(), 3, "is outside"),
    ] {
        let keys = Scratch::new(&format!("{stage}-{name}"), content);
        let layer = keys.layer("keys");
        let run = apply(stage, size, &[], &["--layer", layer.as_str()], &frame);
        assert_eq!(run.code, code, "{name}: {}", run.err);
        assert!(run.err.contains(needle), "{name}: {}", run.err);
        assert_eq!(run.err.matches('\n').count(), 1, "{name}: {}", run.err);
        assert!(run.out.is_empty(), "{name}");
    }
}

/// The stage writes frame N before it has read frame N + 2, and the reader may leave early.
///
/// The frames go in one at a time. After frame N + 1 has gone in, frame N is read back whole
/// while standard input stays open, which a stage that held the stream back could not answer.
/// The lag of one frame is the line buffer of standard output, which may keep the last bytes of
/// a frame until the next write. A stage that streams like this holds a fixed number of frames,
/// so its memory does not grow with the frame count.
pub fn check_it_streams(stage: &str, params: &[(&str, &str)], out_size: (u32, u32)) {
    check_it_streams_with(stage, params, &[], out_size);
}

/// [`check_it_streams`] with further words on the command line, for a stage with a layer.
pub fn check_it_streams_with(
    stage: &str,
    params: &[(&str, &str)],
    extra: &[&str],
    out_size: (u32, u32),
) {
    let size = (64, 48, 8);
    let out_bytes = (out_size.0 * out_size.1) as usize * 4;
    let frames = 6;
    let args = words(stage, size, params, extra);
    let mut child = Command::new(env!("CARGO_BIN_EXE_mocha-stage"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let (ask, asked) = mpsc::channel::<()>();
    let (tell, told) = mpsc::channel::<Vec<u8>>();
    let reader = std::thread::spawn(move || {
        while asked.recv().is_ok() {
            let mut frame = vec![0u8; out_bytes];
            if stdout.read_exact(&mut frame).is_err() {
                break;
            }
            if tell.send(frame).is_err() {
                break;
            }
        }
        stdout
    });
    let all = clip(size, frames, 9);
    let frame_bytes = all.len() / frames;
    let mut got = Vec::new();
    for n in 0..frames {
        stdin
            .write_all(&all[n * frame_bytes..(n + 1) * frame_bytes])
            .expect("the stage reads");
        stdin.flush().unwrap();
        if n >= 1 {
            ask.send(()).unwrap();
            let frame = told
                .recv_timeout(Duration::from_secs(60))
                .unwrap_or_else(|_| {
                    panic!(
                        "{stage}: frame {} was not written while the input was open",
                        n - 1
                    )
                });
            got.extend(frame);
        }
    }
    drop(stdin);
    ask.send(()).unwrap();
    got.extend(
        told.recv_timeout(Duration::from_secs(60))
            .expect("the last frame"),
    );
    drop(ask);
    let mut stdout = reader.join().unwrap();
    let mut rest = Vec::new();
    stdout.read_to_end(&mut rest).unwrap();
    assert!(rest.is_empty(), "{stage}: more output than frames");
    assert_eq!(child.wait().unwrap().code(), Some(0));
    // The same bytes as a run that is fed in one piece.
    let whole = apply(stage, size, params, extra, &all);
    assert_eq!(whole.code, 0, "{}", whole.err);
    assert!(
        got == whole.out,
        "{stage}: the streamed frames differ from the whole run"
    );

    // A reader that leaves after the first frame: the stage ends with broken_pipe, exit 4.
    let big = (256, 256, 8);
    let args = words(stage, big, &[], extra);
    let mut child = Command::new(env!("CARGO_BIN_EXE_mocha-stage"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    drop(child.stdout.take());
    let frame = clip(big, 1, 2);
    for _ in 0..8 {
        if stdin.write_all(&frame).is_err() {
            break;
        }
    }
    drop(stdin);
    let left = child.wait_with_output().unwrap();
    assert_eq!(left.status.code(), Some(4), "{stage}");
    let err = String::from_utf8(left.stderr).unwrap();
    assert!(err.starts_with("error: broken_pipe: "), "{stage}: {err}");
}
