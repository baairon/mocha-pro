//! Exit codes and streams of the `mask-validate` binary, run as a child process on folders made at
//! run time. All values are invented.

mod common;

use common::{build_scenario, fresh_dir};
use mask_seq::{validate, Outcome};
use std::path::PathBuf;
use std::process::{Command, Output};

const USAGE: &str = "usage: mask-validate <folder|manifest.txt>\n";

fn run(args: &[PathBuf]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mask-validate"))
        .args(args)
        .output()
        .expect("run mask-validate")
}

fn valid_run(name: &str) {
    let dir = fresh_dir(name);
    let args = build_scenario(name, &dir);
    let line = match validate(&args[0]) {
        Outcome::Valid(s) => s.line(),
        other => panic!("{other:?}"),
    };
    let out = run(&args);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), format!("{line}\n"));
    assert!(out.stderr.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn valid_folder_exit_0() {
    valid_run("valid_folder");
}

#[test]
fn valid_manifest_path_exit_0() {
    valid_run("valid_manifest_path");
}

#[test]
fn invalid_exit_1_one_line() {
    let dir = fresh_dir("frame_hash");
    let args = build_scenario("frame_hash", &dir);
    let line = match validate(&args[0]) {
        Outcome::Invalid(i) => i.line(),
        other => panic!("{other:?}"),
    };
    let out = run(&args);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8(out.stderr).unwrap(), format!("{line}\n"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_argument_exit_2_usage() {
    let out = run(&[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8(out.stderr).unwrap(), USAGE);
}

#[test]
fn two_arguments_exit_2_usage() {
    let dir = fresh_dir("two_arguments");
    let args = build_scenario("two_arguments", &dir);
    let out = run(&args);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8(out.stderr).unwrap(), USAGE);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_path_exit_2() {
    let dir = fresh_dir("missing_path");
    let args = build_scenario("missing_path", &dir);
    let out = run(&args);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.starts_with("read failed: "), "{err:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
