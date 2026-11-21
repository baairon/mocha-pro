//! Checks `docs/data/mask_validate_interface.json` against the built `mask-validate` binary and the
//! library: each exit code row of the file is reproduced by a child process, so the file cannot
//! drift from the behaviour that `tests/cli.rs` pins. The file is read as text; there is no JSON
//! dependency. The three unpinned paths of the file are not exercised by any test.

mod common;

use common::{build_scenario, fresh_dir, SCENARIOS};
use mask_seq::{Reason, FORMAT, MANIFEST_NAME, NAMING};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn interface_text() -> String {
    let p =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/data/mask_validate_interface.json");
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

#[test]
fn exit_code_rows_equal_the_binary() {
    let text = interface_text();
    let rows = rows(&text);
    assert_eq!(rows.len(), 17, "the file lists 17 scenarios");
    for row in rows {
        let scenario = str_field(row, "scenario").expect("scenario");
        assert!(SCENARIOS.contains(&scenario.as_str()), "{scenario}");
        let code = int_field(row, "code").expect("code");
        let dir = fresh_dir(&scenario);
        let args = build_scenario(&scenario, &dir);
        let out = Command::new(env!("CARGO_BIN_EXE_mask-validate"))
            .args(&args)
            .output()
            .expect("run mask-validate");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(out.status.code(), Some(code), "exit code of {scenario}");
        let err = String::from_utf8(out.stderr).expect("utf-8");
        let want_err = str_field(row, "stderr").expect("stderr");
        match str_field(row, "stderr_kind").expect("stderr_kind").as_str() {
            "empty" => assert!(err.is_empty(), "stderr of {scenario} was {err:?}"),
            "exact" => assert_eq!(err, want_err, "stderr of {scenario}"),
            "prefix" => assert!(
                err.starts_with(&want_err),
                "stderr of {scenario} was {err:?}"
            ),
            other => panic!("unknown stderr_kind {other:?}"),
        }
        let so = String::from_utf8(out.stdout).expect("utf-8");
        let want_out = str_field(row, "stdout").expect("stdout");
        match str_field(row, "stdout_kind").expect("stdout_kind").as_str() {
            "empty" => assert!(so.is_empty(), "stdout of {scenario} was {so:?}"),
            "exact" => assert_eq!(so, want_out, "stdout of {scenario}"),
            other => panic!("unknown stdout_kind {other:?}"),
        }
    }
}

#[test]
fn reason_words_equal_the_library() {
    let text = interface_text();
    let mut words = BTreeSet::new();
    for l in text.lines().filter(|l| l.contains("\"word\"")) {
        assert_eq!(int_field(l, "code"), Some(1), "code of {l}");
        let word = str_field(l, "word").expect("word");
        let detail = match word.as_str() {
            "manifest_missing" => "manifest.txt",
            "manifest_format" => "line 1",
            "manifest_syntax" | "manifest_value" => "line N",
            "frame_read" => "FILE or .",
            _ => "FILE",
        };
        assert_eq!(
            str_field(l, "detail").as_deref(),
            Some(detail),
            "detail of {word}"
        );
        assert!(
            words.insert(str_field(l, "word").expect("word")),
            "duplicate word"
        );
    }
    let lib: BTreeSet<String> = Reason::ALL.iter().map(|r| r.as_str().to_string()).collect();
    assert_eq!(lib.len(), 13);
    assert_eq!(words, lib);
    let mut used: BTreeSet<String> = rows(&text)
        .iter()
        .map(|r| str_field(r, "reason").expect("reason"))
        .filter(|r| r != "none")
        .collect();
    assert!(used.iter().all(|r| lib.contains(r)));
    used.insert("frame_read".to_string());
    assert_eq!(used, lib);
    assert!(text.lines().any(|l| l.contains("\"path\": \"frame_read\"")));
    let error_line = text
        .lines()
        .find_map(|l| str_field(l, "error_line"))
        .expect("error_line");
    assert!(error_line.contains("manifest.txt for manifest_missing"));
    assert!(error_line.contains("line 1 for manifest_format"));
    assert!(error_line.contains("line N for manifest_syntax and manifest_value"));
}

#[test]
fn documented_codes_and_forms() {
    let text = interface_text();
    let codes: Vec<i32> = rows(&text)
        .iter()
        .filter_map(|l| int_field(l, "code"))
        .collect();
    for want in [0, 1, 2] {
        assert!(codes.contains(&want), "exit code {want} missing");
    }
    assert!(codes.iter().all(|c| (0..=2).contains(c)));
    assert!(text.contains(&format!("\"manifest_form\": \"{FORMAT}\"")));
    assert!(text.contains(&format!("\"manifest_name\": \"{MANIFEST_NAME}\"")));
    assert!(text.contains(&format!("\"frame_names\": \"{NAMING}\"")));
}
