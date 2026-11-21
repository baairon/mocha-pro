//! Checks `docs/data/shape_predict_interface.json` against the built `shape-predict` binary: each
//! exit code row of the file is reproduced by a child process, so the file cannot drift from the
//! behaviour that `tests/cli.rs` pins. The file is read as text; there is no JSON dependency.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const VALID: &str = "case=1\n\
bounds=1\n\
meta=0\n\
time=0x0.0p+0\n\
time=0x1.0000000000000p+0\n\
key=0x0.0p+0\n\
pt=0,1,0x0.0p+0,0x0.0p+0,0x0.0p+0,none,0x0.0p+0,0x0.0p+0,0x0.0p+0,0x0.0p+0\n\
key=0x1.0000000000000p+1\n\
pt=0,1,0x1.0000000000000p+2,0x1.0000000000000p+3,0x0.0p+0,none,0x0.0p+0,0x0.0p+0,0x0.0p+0,0x0.0p+0\n";

fn interface_text() -> String {
    let p =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/data/shape_predict_interface.json");
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

fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "shape-predict-iface-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create folder");
    dir
}

fn run(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_shape-predict"))
        .args(args)
        .output()
        .expect("run shape-predict")
}

fn run_scenario(name: &str) -> Output {
    let dir = fresh_dir(name);
    let case = dir.join("case.txt");
    let out = match name {
        "valid" => {
            std::fs::write(&case, VALID).unwrap();
            run(&[&case])
        }
        "bad_line" => {
            std::fs::write(&case, b"case=1\nbogus=1\n").unwrap();
            run(&[&case])
        }
        "refused_contour" => {
            std::fs::write(&case, b"case=1\ntime=0x0.0p+0\n").unwrap();
            run(&[&case])
        }
        "no_argument" => run(&[]),
        "missing_file" => run(&[&dir.join("absent.txt")]),
        "non_utf8_file" => {
            std::fs::write(&case, [b'c', b'a', 0xff, 0xfe, b'\n']).unwrap();
            run(&[&case])
        }
        other => panic!("unknown scenario {other:?} in the interface file"),
    };
    let _ = std::fs::remove_dir_all(&dir);
    out
}

#[test]
fn exit_code_rows_match_the_binary() {
    let text = interface_text();
    let rows: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("\"scenario\""))
        .collect();
    assert_eq!(rows.len(), 6, "the file lists six scenarios");
    for row in rows {
        let scenario = str_field(row, "scenario").expect("scenario");
        let code = int_field(row, "code").expect("code");
        let kind = str_field(row, "stderr_kind").expect("stderr_kind");
        let out = run_scenario(&scenario);
        assert_eq!(out.status.code(), Some(code), "exit code of {scenario}");
        let err = String::from_utf8(out.stderr).expect("utf-8");
        match kind.as_str() {
            "empty" => {
                assert!(err.is_empty(), "stderr of {scenario} was {err:?}");
                assert!(!out.stdout.is_empty(), "stdout of {scenario}");
            }
            "prefix" => {
                let p = str_field(row, "stderr_prefix").expect("stderr_prefix");
                assert!(err.starts_with(&p), "stderr of {scenario} was {err:?}");
                assert!(out.stdout.is_empty(), "stdout of {scenario}");
            }
            "exact" => {
                let p = str_field(row, "stderr_prefix").expect("stderr_prefix");
                assert_eq!(err, p, "stderr of {scenario}");
                assert!(out.stdout.is_empty(), "stdout of {scenario}");
            }
            other => panic!("unknown stderr_kind {other:?}"),
        }
    }
}

#[test]
fn only_documented_exit_codes_and_dump_name() {
    let text = interface_text();
    let codes: Vec<i32> = text
        .lines()
        .filter(|l| l.contains("\"scenario\""))
        .filter_map(|l| int_field(l, "code"))
        .collect();
    for want in [0, 1, 2] {
        assert!(codes.contains(&want), "exit code {want} missing");
    }
    assert!(codes.iter().all(|c| (0..=2).contains(c)));
    assert!(text.contains("\"dump_form\": \"shape-1\""));
    let out = run_scenario("valid");
    assert!(
        out.stdout.starts_with(b"format=shape-1\n"),
        "the dump starts with the form name"
    );
}
