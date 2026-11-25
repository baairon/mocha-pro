//! Helpers of the image stage tests: run the built program, check a refusal, build made up
//! frames at the three depths, hold a scratch file.

#![allow(dead_code)]

pub mod interface;

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// The result of one run of the built program.
pub struct Run {
    pub code: i32,
    pub out: Vec<u8>,
    pub err: String,
}

/// Run the built program with `args` and `input` on standard input.
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

/// `apply STAGE` at a width, height and depth, then the extra words.
pub fn run(stage: &str, size: [&str; 3], extra: &[&str], input: &[u8]) -> Run {
    let mut args = vec![
        "apply", stage, "--width", size[0], "--height", size[1], "--depth", size[2], "--fps",
        "24/1",
    ];
    args.extend_from_slice(extra);
    spawn(&args, input)
}

/// A run that must end with exit 0 and an empty error stream; returns the frames written.
pub fn frames(stage: &str, size: [&str; 3], extra: &[&str], input: &[u8]) -> Vec<u8> {
    let run = run(stage, size, extra, input);
    assert_eq!(
        (run.code, run.err.as_str()),
        (0, ""),
        "{stage} {size:?} {extra:?}"
    );
    run.out
}

/// A run that must be refused: the exit status, the start of the one error line, and no frame
/// on the output.
pub fn refused(stage: &str, size: [&str; 3], extra: &[&str], input: &[u8], code: i32, line: &str) {
    let run = run(stage, size, extra, input);
    assert_eq!(run.code, code, "{stage} {extra:?}: {}", run.err);
    assert!(
        run.err.starts_with(line),
        "{stage} {extra:?}: `{}` does not start with `{line}`",
        run.err
    );
    assert_eq!(run.err.matches('\n').count(), 1, "{}", run.err);
    assert!(run.err.ends_with('\n'));
    assert!(
        run.out.is_empty(),
        "{stage} {extra:?}: a refusal writes no frame"
    );
}

/// Run a stage whose reader closes the output at once, feed it `input`, and return the exit
/// status and the error stream.
pub fn reader_leaves(stage: &str, size: [&str; 3], extra: &[&str], input: &[u8]) -> (i32, String) {
    let mut args = vec![
        "apply", stage, "--width", size[0], "--height", size[1], "--depth", size[2], "--fps",
        "24/1",
    ];
    args.extend_from_slice(extra);
    let mut child = Command::new(env!("CARGO_BIN_EXE_mocha-stage"))
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    // close the reading end of the output before the stage has a frame to write
    drop(child.stdout.take());
    let mut stdin = child.stdin.take().unwrap();
    let data = input.to_vec();
    let feeder = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let mut err = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .expect("read the error stream");
    let status = child.wait().expect("wait");
    feeder.join().unwrap();
    (status.code().expect("exit code"), err)
}

/// The words `--param K=V` for each pair.
pub fn params<'a>(pairs: &[&'a str]) -> Vec<&'a str> {
    pairs.iter().flat_map(|p| ["--param", *p]).collect()
}

/// The manifest entry of a stage, as the program prints it.
pub fn manifest_entry(stage: &str) -> String {
    let manifest = String::from_utf8(spawn(&["list", "--json"], b"").out).unwrap();
    let start = manifest
        .find(&format!("{{\"id\":\"{stage}\",\"kind\""))
        .unwrap_or_else(|| panic!("{stage} is not in the manifest"));
    let rest = &manifest[start..];
    let end = rest.find("\"family\":").expect("family");
    let close = rest[end..].find('}').expect("end of the entry");
    rest[..end + close + 1].to_string()
}

/// The parameter ids of a manifest entry, in order.
pub fn param_ids(entry: &str) -> Vec<String> {
    let params = &entry[entry.find("\"params\":[").unwrap()..entry.find("\"layers\":[").unwrap()];
    params
        .split("{\"id\":\"")
        .skip(1)
        .map(|part| part[..part.find('"').unwrap()].to_string())
        .collect()
}

/// A depth 8 frame from pixels given as R, G, B, A.
pub fn rgba8(pixels: &[[u8; 4]]) -> Vec<u8> {
    pixels.concat()
}

/// A depth 16 frame from pixels given as R, G, B, A.
pub fn rgba16(pixels: &[[u16; 4]]) -> Vec<u8> {
    pixels
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// A depth 32 frame from pixels given as R, G, B, A: the four planes G, B, R, A.
pub fn planar32(pixels: &[[f32; 4]]) -> Vec<u8> {
    [1usize, 2, 0, 3]
        .iter()
        .flat_map(|c| pixels.iter().flat_map(move |p| p[*c].to_le_bytes()))
        .collect()
}

/// The alpha samples of a depth 8 frame.
pub fn alpha8(frame: &[u8]) -> Vec<u8> {
    frame.as_chunks::<4>().0.iter().map(|p| p[3]).collect()
}

/// A depth 8 frame with the colour 10, 20, 30 and the alpha samples given.
pub fn with_alpha8(alpha: &[u8]) -> Vec<u8> {
    alpha.iter().flat_map(|a| [10, 20, 30, *a]).collect()
}

/// A file with made up content in the temporary folder, removed when dropped.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(name: &str, content: &str) -> Scratch {
        let path =
            std::env::temp_dir().join(format!("stage-cli-image-{}-{name}", std::process::id()));
        std::fs::write(&path, content).expect("write the scratch file");
        Scratch(path)
    }

    /// The word `keys=PATH` for `--layer`.
    pub fn layer(&self) -> String {
        format!("keys={}", self.0.display())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
