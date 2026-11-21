//! Exit codes and streams of the `shape-predict` binary, run as a child process on files written
//! into a fresh folder under the system temporary directory. All values are invented.

use shape_canon::{predict, Case};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VALID: &str = "case=1\n\
bounds=1\n\
meta=0\n\
time=0x0.0p+0\n\
time=0x1.0000000000000p+0\n\
key=0x0.0p+0\n\
pt=0,1,0x0.0p+0,0x0.0p+0,0x0.0p+0,none,0x0.0p+0,0x0.0p+0,0x0.0p+0,0x0.0p+0\n\
key=0x1.0000000000000p+1\n\
pt=0,1,0x1.0000000000000p+2,0x1.0000000000000p+3,0x0.0p+0,none,0x0.0p+0,0x0.0p+0,0x0.0p+0,0x0.0p+0\n";

const USAGE: &str = "usage: shape-predict <case.txt>\n";

/// A fresh, empty folder for one test.
fn fresh_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("shape-predict-cli-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create folder");
    dir
}

fn write(dir: &Path, file: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(file);
    std::fs::write(&p, bytes).expect("write case file");
    p
}

fn run(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_shape-predict"))
        .args(args)
        .output()
        .expect("run shape-predict")
}

fn library_dump(text: &str) -> Vec<u8> {
    let case = Case::parse(text).expect("parse");
    predict(&case).expect("predict")
}

#[test]
fn valid_case_exit_0_dump_on_stdout() {
    let dir = fresh_dir("valid");
    let f = write(&dir, "case.txt", VALID.as_bytes());
    let out = run(&[&f]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(out.stdout, library_dump(VALID));
    assert!(!out.stdout.is_empty());
    assert!(out.stderr.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_line_exit_1() {
    let dir = fresh_dir("badline");
    let f = write(&dir, "case.txt", b"case=1\nbogus=1\n");
    let out = run(&[&f]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let err = String::from_utf8(out.stderr).expect("utf-8");
    assert!(err.starts_with("error: line "), "stderr was {err:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn refused_contour_exit_1() {
    let dir = fresh_dir("refused");
    // a well-formed case without any key: Contour::new refuses it
    let f = write(&dir, "case.txt", b"case=1\ntime=0x0.0p+0\n");
    let out = run(&[&f]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let err = String::from_utf8(out.stderr).expect("utf-8");
    assert!(err.starts_with("error: "), "stderr was {err:?}");
    assert!(!err.starts_with("error: line "), "stderr was {err:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_argument_exit_2_usage() {
    let out = run(&[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8(out.stderr).expect("utf-8"), USAGE);
}

#[test]
fn missing_file_exit_2() {
    let dir = fresh_dir("missing");
    let out = run(&[&dir.join("absent.txt")]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let err = String::from_utf8(out.stderr).expect("utf-8");
    assert!(err.starts_with("read failed: "), "stderr was {err:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn non_utf8_file_exit_2() {
    let dir = fresh_dir("nonutf8");
    let f = write(&dir, "case.txt", &[b'c', b'a', 0xff, 0xfe, b'\n']);
    let out = run(&[&f]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let err = String::from_utf8(out.stderr).expect("utf-8");
    assert!(err.starts_with("read failed: "), "stderr was {err:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn second_argument_is_ignored() {
    let dir = fresh_dir("second");
    let f = write(&dir, "case.txt", VALID.as_bytes());
    let other = dir.join("never-read.txt");
    let one = run(&[&f]);
    let two = run(&[&f, &other]);
    assert_eq!(two.status.code(), Some(0));
    assert_eq!(two.stdout, one.stdout);
    assert!(two.stderr.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
