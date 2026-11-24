//! Checks `docs/data/frame_tools_interface.json` against the built `frame-gen` and `frame-cmp`
//! binaries: each exit code row of the file is reproduced by a child process, so the file cannot
//! drift from the behaviour. The file is read as text; there is no JSON dependency.

mod common;

use common::{fresh_dir, geom, stream};
use frame_io::{write_sample, Depth, Pattern};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const SCENARIOS: [&str; 19] = [
    "gen_hramp_depth8",
    "gen_zero_frames",
    "gen_help",
    "gen_missing_frames",
    "gen_decimal_rate",
    "gen_unsupported_depth",
    "gen_frame_over_4_gib",
    "cmp_equal",
    "cmp_equal_stdin",
    "cmp_help",
    "cmp_differ_sample",
    "cmp_differ_length",
    "cmp_differ_verbose",
    "cmp_one_stream",
    "cmp_two_stdin",
    "cmp_unsupported_depth",
    "cmp_partial_frame",
    "cmp_missing_file",
    "cmp_count_mismatch",
];

fn interface_text() -> String {
    let p =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/data/frame_tools_interface.json");
    std::fs::read_to_string(&p).expect("read interface file")
}

/// The string value of `"key": "..."` in one line (escapes: \n, \\ and \" only).
fn str_field(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\": \"");
    let start = line.find(&pat)? + pat.len();
    let mut out = String::new();
    let mut chars = line[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

fn int_field(line: &str, key: &str) -> Option<i32> {
    let pat = format!("\"{key}\": ");
    let start = line.find(&pat)? + pat.len();
    let digits: String = line[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn rows(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| l.contains("\"scenario\""))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Writes a.raw and b.raw of the scenario into `dir`.
fn build(scenario: &str, dir: &Path) {
    let g = geom(2, 1, Depth::Eight);
    let s = stream(&g, &Pattern::HRamp, 2);
    let mut b = s.clone();
    match scenario {
        "cmp_differ_sample" | "cmp_differ_verbose" => write_sample(&g, &mut b[8..], 1, 0, 1, 21),
        "cmp_differ_length" => b.truncate(8),
        "cmp_partial_frame" => b.truncate(11),
        _ => {}
    }
    std::fs::write(dir.join("a.raw"), &s).unwrap();
    std::fs::write(dir.join("b.raw"), &b).unwrap();
}

#[test]
fn exit_code_rows_equal_the_binaries() {
    let text = interface_text();
    let rows = rows(&text);
    assert_eq!(rows.len(), SCENARIOS.len());
    for row in rows {
        let scenario = str_field(row, "scenario").expect("scenario");
        assert!(SCENARIOS.contains(&scenario.as_str()), "{scenario}");
        let tool = str_field(row, "tool").expect("tool");
        let exe = match tool.as_str() {
            "frame-gen" => env!("CARGO_BIN_EXE_frame-gen"),
            "frame-cmp" => env!("CARGO_BIN_EXE_frame-cmp"),
            other => panic!("unknown tool {other}"),
        };
        let dir = fresh_dir(&scenario);
        build(&scenario, &dir);
        let args = str_field(row, "args").expect("args");
        let stdin_file = str_field(row, "stdin").expect("stdin");
        let mut child = Command::new(exe)
            .args(args.split_whitespace())
            .current_dir(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let feed = if stdin_file == "none" {
            Vec::new()
        } else {
            std::fs::read(dir.join(&stdin_file)).unwrap()
        };
        child.stdin.take().unwrap().write_all(&feed).ok();
        let out = child.wait_with_output().expect("run");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            out.status.code(),
            int_field(row, "code"),
            "exit code of {scenario}"
        );
        let err = String::from_utf8(out.stderr).expect("utf-8");
        let want_err = str_field(row, "stderr").expect("stderr");
        match str_field(row, "stderr_kind").expect("stderr_kind").as_str() {
            "empty" => assert!(err.is_empty(), "stderr of {scenario} was {err:?}"),
            "exact" => assert_eq!(err, want_err, "stderr of {scenario}"),
            "prefix" => {
                assert!(
                    err.starts_with(&want_err),
                    "stderr of {scenario} was {err:?}"
                );
                assert_eq!(err.matches('\n').count(), 1, "one line in {scenario}");
            }
            other => panic!("unknown stderr_kind {other:?}"),
        }
        let want_out = str_field(row, "stdout").expect("stdout");
        match str_field(row, "stdout_kind").expect("stdout_kind").as_str() {
            "empty" => assert!(out.stdout.is_empty(), "stdout of {scenario}"),
            "exact" => assert_eq!(
                String::from_utf8(out.stdout).unwrap(),
                want_out,
                "stdout of {scenario}"
            ),
            "prefix" => assert!(
                String::from_utf8(out.stdout)
                    .unwrap()
                    .starts_with(&want_out),
                "{scenario}"
            ),
            "hex" => assert_eq!(hex(&out.stdout), want_out, "stdout of {scenario}"),
            other => panic!("unknown stdout_kind {other:?}"),
        }
    }
}

#[test]
fn rows_cover_every_exit_code_of_the_table() {
    let text = interface_text();
    let seen: BTreeSet<i32> = rows(&text)
        .iter()
        .filter_map(|r| int_field(r, "code"))
        .collect();
    assert_eq!(seen, BTreeSet::from([0, 1, 2, 3, 4]));
    let listed: BTreeSet<i32> = text
        .lines()
        .filter(|l| l.contains("\"table\""))
        .filter_map(|l| int_field(l, "code"))
        .collect();
    assert_eq!(listed, seen);
    for tool in ["frame-gen", "frame-cmp"] {
        assert!(rows(&text)
            .iter()
            .any(|r| str_field(r, "tool").as_deref() == Some(tool)));
    }
}

#[test]
fn documented_patterns_parse_and_round_trip() {
    let text = interface_text();
    let specs: Vec<String> = text
        .lines()
        .filter(|l| l.contains("\"spec\""))
        .filter_map(|l| str_field(l, "spec"))
        .collect();
    assert_eq!(specs.len(), 6);
    for spec in specs {
        // "constant:R,G,B,A" is a template; the others are concrete.
        let concrete = spec.replace("R,G,B,A", "1,2,3,4");
        let p: Pattern = concrete.parse().unwrap_or_else(|e| panic!("{spec}: {e}"));
        assert_eq!(p.to_string(), concrete);
    }
}

#[test]
fn the_file_states_the_exit_one_choice_and_the_frame_contract() {
    let text = interface_text();
    assert!(text.contains("\"exit_one\": \"frame-cmp exits 1 when the streams differ."));
    for word in [
        "rgba64le",
        "gbrapf32le",
        "4 GiB",
        "partial frame",
        "tolerance 0",
    ] {
        assert!(text.contains(word), "{word}");
    }
}

#[test]
fn generated_stream_piped_into_compare_is_equal_across_real_processes() {
    let gen = Command::new(env!("CARGO_BIN_EXE_frame-gen"))
        .args("--width 4 --height 3 --depth 32 --frames 2 --pattern random:5".split_whitespace())
        .output()
        .unwrap();
    assert_eq!(gen.status.code(), Some(0));
    assert_eq!(gen.stdout.len(), 2 * 4 * 3 * 16);
    let dir = fresh_dir("pipe");
    std::fs::write(dir.join("a.raw"), &gen.stdout).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_frame-cmp"))
        .args("--width 4 --height 3 --depth 32 a.raw -".split_whitespace())
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&gen.stdout).unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "equal frames=2 width=4 height=3 depth=32\n"
    );
}
