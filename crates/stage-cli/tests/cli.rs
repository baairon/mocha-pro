//! The `mocha-stage` binary end to end, on made up frames.

use std::io::{Read, Write};
use std::process::{Command, Stdio};

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

fn clip(bytes: usize, seed: u8) -> Vec<u8> {
    (0..bytes)
        .map(|i| {
            (i as u8)
                .wrapping_mul(31)
                .wrapping_add(seed)
                .wrapping_add((i >> 8) as u8)
        })
        .collect()
}

fn apply(depth: &str) -> Vec<String> {
    [
        "apply",
        "selftest-copy",
        "--width",
        "3",
        "--height",
        "2",
        "--depth",
        depth,
        "--fps",
        "24/1",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn go(args: &[String], input: &[u8]) -> Run {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    spawn(&a, input)
}

fn bytes_per_frame(depth: usize) -> usize {
    3 * 2 * (depth / 2)
}

#[test]
fn list_json_is_the_manifest() {
    let r = spawn(&["list", "--json"], b"");
    assert_eq!(r.code, 0);
    assert_eq!(r.err, "");
    let text = String::from_utf8(r.out).unwrap();
    assert_eq!(text.matches('\n').count(), 1);
    assert!(text.ends_with("}\n"));
    for needle in [
        "\"contract\":\"frames/2\"",
        "\"provider\":{\"id\":\"mocha_pro\"",
        "\"command\":[\"mocha-stage\"]",
        "\"id\":\"selftest-copy\"",
        "\"kind\":\"frame\"",
        "\"spool\":false",
        "\"geometry\":\"preserve\"",
        "\"depths\":[8,16,32]",
        "\"params\":[]",
        "\"layers\":[]",
        "copies frames unchanged and reimplements nothing",
    ] {
        assert!(text.contains(needle), "{needle} in {text}");
    }
}

#[test]
fn list_json_names_exactly_one_stage() {
    let text = String::from_utf8(spawn(&["list", "--json"], b"").out).unwrap();
    assert_eq!(text.matches("\"kind\"").count(), 1);
    assert_eq!(text.matches("\"geometry\"").count(), 1);
}

#[test]
fn list_text_has_one_line_per_stage() {
    let r = spawn(&["list"], b"");
    assert_eq!(
        (r.code, String::from_utf8(r.out).unwrap()),
        (0, "selftest-copy\n".to_string())
    );
}

#[test]
fn version_line() {
    let r = spawn(&["--version"], b"");
    assert_eq!(r.code, 0);
    assert_eq!(
        String::from_utf8(r.out).unwrap(),
        format!("mocha-stage {}\n", env!("CARGO_PKG_VERSION"))
    );
    let manifest = String::from_utf8(spawn(&["list", "--json"], b"").out).unwrap();
    assert!(manifest.contains(&format!(
        "\"version\":\"mocha-stage {}\"",
        env!("CARGO_PKG_VERSION")
    )));
}

#[test]
fn help_exits_zero_on_the_output() {
    let r = spawn(&["--help"], b"");
    assert_eq!((r.code, r.err.as_str()), (0, ""));
    assert!(String::from_utf8(r.out)
        .unwrap()
        .starts_with("usage: mocha-stage"));
}

#[test]
fn no_arguments_is_usage() {
    let r = spawn(&[], b"");
    assert_eq!(r.code, 2);
    assert!(r.out.is_empty());
    assert!(r.err.starts_with("error: usage: "), "{}", r.err);
}

#[test]
fn copy_at_every_depth() {
    for depth in [8usize, 16, 32] {
        let data = clip(bytes_per_frame(depth) * 5, depth as u8);
        let r = go(&apply(&depth.to_string()), &data);
        assert_eq!(r.code, 0, "depth {depth}: {}", r.err);
        assert_eq!(r.out, data, "depth {depth}");
        assert_eq!(r.err, "");
    }
}

#[test]
fn copy_keeps_every_byte_value_including_line_ends_and_ctrl_z() {
    let mut data: Vec<u8> = (0..=255u8).collect();
    data.extend_from_slice(b"\r\n\r\n\x1a\x00\n\n");
    data.resize(bytes_per_frame(8) * 64, 0x0a);
    let r = go(&apply("8"), &data);
    assert_eq!((r.code, r.out), (0, data));
}

#[test]
fn copy_of_large_frames_through_the_pipe() {
    let args: Vec<String> = [
        "apply",
        "selftest-copy",
        "--width",
        "300",
        "--height",
        "200",
        "--depth",
        "32",
        "--fps",
        "25",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let data = clip(300 * 200 * 16 * 3, 9);
    let r = go(&args, &data);
    assert_eq!((r.code, r.out == data), (0, true));
}

#[test]
fn one_by_one_frame() {
    let args: Vec<String> = [
        "apply",
        "selftest-copy",
        "--width",
        "1",
        "--height",
        "1",
        "--depth",
        "8",
        "--fps",
        "1",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let r = go(&args, &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!((r.code, r.out), (0, vec![1, 2, 3, 4, 5, 6, 7, 8]));
}

#[test]
fn empty_input_is_exit_zero_and_empty_output() {
    for depth in ["8", "16", "32"] {
        let r = go(&apply(depth), b"");
        assert_eq!((r.code, r.out.len(), r.err.as_str()), (0, 0, ""));
    }
}

#[test]
fn partial_last_frame_is_exit_four_and_whole_frames_stay_written() {
    let frame = bytes_per_frame(8);
    let data = clip(frame * 2 + 5, 1);
    let r = go(&apply("8"), &data);
    assert_eq!(r.code, 4);
    assert_eq!(r.out, data[..frame * 2]);
    assert!(r.err.starts_with("error: partial_frame: "), "{}", r.err);
    assert_eq!(r.err.matches('\n').count(), 1);
}

#[test]
fn input_shorter_than_one_frame_is_exit_four() {
    let r = go(&apply("16"), &[1, 2, 3]);
    assert_eq!((r.code, r.out.len()), (4, 0));
    assert!(r.err.starts_with("error: partial_frame: "));
}

#[test]
fn unknown_stage_is_exit_three() {
    let mut a = apply("8");
    a[1] = "selftest-invert".into();
    let r = go(&a, b"");
    assert_eq!(r.code, 3);
    assert!(
        r.err
            .starts_with("error: unsupported: unknown stage `selftest-invert`"),
        "{}",
        r.err
    );
    assert!(r.out.is_empty());
}

#[test]
fn unsupported_depths_are_exit_three() {
    for depth in ["12", "1", "24", "64"] {
        let r = go(&apply(depth), &clip(64, 0));
        assert_eq!(r.code, 3, "depth {depth}");
        assert!(r.err.starts_with("error: unsupported: depth "), "{}", r.err);
        assert!(r.out.is_empty());
    }
}

#[test]
fn malformed_depths_are_exit_two() {
    for depth in ["0", "x", "-8", "8.0", ""] {
        assert_eq!(go(&apply(depth), b"").code, 2, "depth {depth:?}");
    }
}

#[test]
fn a_parameter_is_exit_three() {
    let mut a = apply("8");
    a.extend(["--param".into(), "mode=fast".into()]);
    let r = go(&a, b"");
    assert_eq!(r.code, 3);
    assert!(
        r.err
            .starts_with("error: unsupported: selftest-copy takes no parameters"),
        "{}",
        r.err
    );
}

#[test]
fn a_layer_is_exit_three() {
    let mut a = apply("8");
    a.extend(["--layer".into(), "alt=some.bin".into()]);
    let r = go(&a, b"");
    assert_eq!(r.code, 3);
    assert!(r.err.contains("takes no layers"), "{}", r.err);
}

#[test]
fn unknown_option_is_exit_two() {
    let mut a = apply("8");
    a.push("--conformance-unknown-option".into());
    let r = go(&a, b"");
    assert_eq!(r.code, 2);
    assert!(
        r.err.starts_with("error: usage: unknown option"),
        "{}",
        r.err
    );
}

#[test]
fn missing_and_bad_geometry_are_exit_two() {
    for args in [
        vec![
            "apply",
            "selftest-copy",
            "--height",
            "2",
            "--depth",
            "8",
            "--fps",
            "24",
        ],
        vec![
            "apply",
            "selftest-copy",
            "--width",
            "3",
            "--depth",
            "8",
            "--fps",
            "24",
        ],
        vec![
            "apply",
            "selftest-copy",
            "--width",
            "3",
            "--height",
            "2",
            "--fps",
            "24",
        ],
        vec![
            "apply",
            "selftest-copy",
            "--width",
            "3",
            "--height",
            "2",
            "--depth",
            "8",
        ],
        vec![
            "apply", "--width", "3", "--height", "2", "--depth", "8", "--fps", "24",
        ],
        vec![
            "apply",
            "selftest-copy",
            "--width",
            "0",
            "--height",
            "2",
            "--depth",
            "8",
            "--fps",
            "24",
        ],
        vec![
            "apply",
            "selftest-copy",
            "--width",
            "3",
            "--height",
            "-2",
            "--depth",
            "8",
            "--fps",
            "24",
        ],
        vec![
            "apply",
            "selftest-copy",
            "--width",
            "3",
            "--height",
            "2",
            "--depth",
            "8",
            "--fps",
            "29.97",
        ],
        vec![
            "apply",
            "selftest-copy",
            "--width",
            "3",
            "--height",
            "2",
            "--depth",
            "8",
            "--fps",
            "24/0",
        ],
        vec![
            "apply",
            "selftest-copy",
            "--width",
            "3",
            "--height",
            "2",
            "--depth",
            "8",
            "--fps",
            "0",
        ],
    ] {
        let r = spawn(&args, b"");
        assert_eq!(r.code, 2, "{args:?}");
        assert!(r.err.starts_with("error: usage: "), "{args:?}: {}", r.err);
        assert!(r.out.is_empty());
    }
}

#[test]
fn a_frame_above_four_gib_is_exit_two() {
    let r = spawn(
        &[
            "apply",
            "selftest-copy",
            "--width",
            "65536",
            "--height",
            "65536",
            "--depth",
            "32",
            "--fps",
            "24",
        ],
        b"",
    );
    assert_eq!(r.code, 2);
    assert!(r.err.contains("4 GiB"), "{}", r.err);
}

#[test]
fn frames_count_matches_or_is_exit_four() {
    let data = clip(bytes_per_frame(8) * 3, 2);
    let mut ok = apply("8");
    ok.extend(["--frames".into(), "3".into()]);
    assert_eq!(go(&ok, &data).code, 0);
    let mut bad = apply("8");
    bad.extend(["--frames".into(), "4".into()]);
    let r = go(&bad, &data);
    assert_eq!(r.code, 4);
    assert!(r.err.starts_with("error: bad_input: "), "{}", r.err);
}

#[test]
fn rate_forms_are_accepted() {
    for fps in ["24", "24/1", "30000/1001", "7200/137"] {
        let mut a = apply("8");
        let at = a.iter().position(|s| s == "--fps").unwrap();
        a[at + 1] = fps.into();
        assert_eq!(go(&a, b"").code, 0, "{fps}");
    }
}

#[test]
fn describe_preserves_the_request() {
    let r = spawn(
        &[
            "apply",
            "selftest-copy",
            "--describe",
            "--width",
            "80",
            "--height",
            "48",
            "--depth",
            "16",
            "--fps",
            "30000/1001",
            "--frames",
            "12",
        ],
        b"",
    );
    assert_eq!(r.code, 0);
    assert_eq!(
        String::from_utf8(r.out).unwrap(),
        "{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"selftest-copy\",\"width\":80,\"height\":48,\"depth\":16,\"fps\":\"30000/1001\",\"frames\":12}\n"
    );
}

#[test]
fn describe_without_frames_says_null() {
    let r = spawn(
        &[
            "apply",
            "selftest-copy",
            "--describe",
            "--width",
            "5",
            "--height",
            "4",
            "--depth",
            "32",
            "--fps",
            "24",
        ],
        b"",
    );
    assert_eq!(r.code, 0);
    assert!(String::from_utf8(r.out)
        .unwrap()
        .ends_with("\"fps\":\"24/1\",\"frames\":null}\n"));
}

#[test]
fn describe_ignores_the_input() {
    let r = spawn(
        &[
            "apply",
            "selftest-copy",
            "--describe",
            "--width",
            "5",
            "--height",
            "4",
            "--depth",
            "8",
            "--fps",
            "24",
        ],
        &clip(999, 3),
    );
    assert_eq!(r.code, 0);
    assert!(String::from_utf8(r.out)
        .unwrap()
        .contains("\"width\":5,\"height\":4"));
}

#[test]
fn describe_refuses_like_apply() {
    for (args, code) in [
        (
            vec![
                "apply",
                "selftest-copy",
                "--describe",
                "--width",
                "5",
                "--height",
                "4",
                "--depth",
                "12",
                "--fps",
                "24",
            ],
            3,
        ),
        (
            vec![
                "apply",
                "nope",
                "--describe",
                "--width",
                "5",
                "--height",
                "4",
                "--depth",
                "8",
                "--fps",
                "24",
            ],
            3,
        ),
        (
            vec![
                "apply",
                "selftest-copy",
                "--describe",
                "--width",
                "5",
                "--height",
                "4",
                "--depth",
                "8",
                "--fps",
                "24",
                "--param",
                "a=b",
            ],
            3,
        ),
        (
            vec![
                "apply",
                "selftest-copy",
                "--describe",
                "--width",
                "5",
                "--height",
                "4",
                "--depth",
                "8",
            ],
            2,
        ),
        (
            vec![
                "apply",
                "selftest-copy",
                "--describe",
                "--width",
                "32768",
                "--height",
                "32769",
                "--depth",
                "8",
                "--fps",
                "24",
            ],
            2,
        ),
    ] {
        let r = spawn(&args, b"");
        assert_eq!(r.code, code, "{args:?}: {}", r.err);
        assert!(r.out.is_empty());
    }
}

#[test]
fn describe_at_the_four_gib_boundary() {
    let r = spawn(
        &[
            "apply",
            "selftest-copy",
            "--describe",
            "--width",
            "32768",
            "--height",
            "32768",
            "--depth",
            "8",
            "--fps",
            "24",
        ],
        b"",
    );
    assert_eq!(r.code, 0);
}

#[test]
fn equals_form_works() {
    let r = spawn(
        &[
            "apply",
            "selftest-copy",
            "--width=1",
            "--height=1",
            "--depth=8",
            "--fps=24",
        ],
        &[9, 8, 7, 6],
    );
    assert_eq!((r.code, r.out), (0, vec![9, 8, 7, 6]));
}

#[test]
fn repeated_option_is_exit_two() {
    let mut a = apply("8");
    a.extend(["--width".into(), "4".into()]);
    assert_eq!(go(&a, b"").code, 2);
}

#[test]
fn reader_that_leaves_is_broken_pipe_exit_four() {
    let args = apply("8");
    let mut child = Command::new(env!("CARGO_BIN_EXE_mocha-stage"))
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // close the reading end of the output at once, then feed more than a pipe holds
    drop(child.stdout.take());
    let mut stdin = child.stdin.take().unwrap();
    let data = clip(bytes_per_frame(8) * 40000, 5);
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
}

#[test]
fn errors_are_one_line_in_the_text_form() {
    let mut a = apply("8");
    a[1] = "bad\nname".into();
    let r = go(&a, b"");
    assert_eq!(r.code, 3);
    assert_eq!(r.err.matches('\n').count(), 1);
    assert!(r.err.starts_with("error: unsupported: "));
}

#[test]
fn unknown_command_is_exit_two() {
    let r = spawn(&["render"], b"");
    assert_eq!(r.code, 2);
    assert!(
        r.err.starts_with("error: usage: unknown command"),
        "{}",
        r.err
    );
}

#[test]
fn copying_twice_is_still_identity_and_deterministic() {
    let data = clip(bytes_per_frame(16) * 4, 11);
    let once = go(&apply("16"), &data);
    let twice = go(&apply("16"), &once.out);
    assert_eq!(twice.out, data);
}
