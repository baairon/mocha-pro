//! Checks `docs/data/stage_track_planar_interface.json` against the built `mocha-stage`: each
//! exit code row of the file is reproduced by a child process and each parameter row is compared
//! with the definition of the stage, so the file cannot drift from the behaviour. The file is
//! read as text; there is no JSON dependency.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use stage_cli::registry::ParamKind;
use stage_cli::stages::data::track_planar::DEF;

const SCENARIOS: [&str; 20] = [
    "pass_through_at_the_defaults",
    "pass_through_with_a_data_file",
    "describe_answers_the_request",
    "empty_input",
    "data_file_cannot_be_written",
    "one_path_for_both_files",
    "parameter_given_twice",
    "unknown_option",
    "depth_not_taken",
    "unknown_parameter",
    "choice_not_offered",
    "value_out_of_range",
    "keyed_value_on_the_command_line",
    "configuration_the_crate_refuses",
    "corners_not_convex",
    "layer_not_taken",
    "refused_frame_with_refusal_fail",
    "input_ends_inside_a_frame",
    "frame_count_differs",
    "reference_beyond_the_input",
];

fn interface_text() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/data/stage_track_planar_interface.json");
    std::fs::read_to_string(&p).expect("read the interface file")
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

fn rows<'a>(text: &'a str, key: &str) -> Vec<&'a str> {
    let mark = format!("\"{key}\": ");
    text.lines().filter(|l| l.contains(&mark)).collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stage-cli-interface-track-planar-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the scratch folder");
    dir
}

#[test]
fn exit_code_rows_equal_the_program() {
    let text = interface_text();
    let rows = rows(&text, "scenario");
    assert_eq!(rows.len(), SCENARIOS.len());
    for row in rows {
        let scenario = str_field(row, "scenario").expect("scenario");
        assert!(SCENARIOS.contains(&scenario.as_str()), "{scenario}");
        assert_eq!(str_field(row, "tool").as_deref(), Some("mocha-stage"));
        let dir = fresh_dir(&scenario);
        let args = str_field(row, "args").expect("args");
        let stdin = str_field(row, "stdin").expect("stdin");
        let feed = if stdin == "none" {
            Vec::new()
        } else {
            unhex(&stdin)
        };
        let mut child = Command::new(env!("CARGO_BIN_EXE_mocha-stage"))
            .args(args.split_whitespace())
            .current_dir(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
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
            "hex" => assert_eq!(hex(&out.stdout), want_out, "stdout of {scenario}"),
            other => panic!("unknown stdout_kind {other:?}"),
        }
    }
}

#[test]
fn rows_cover_every_exit_code_of_the_table() {
    let text = interface_text();
    let seen: BTreeSet<i32> = rows(&text, "scenario")
        .iter()
        .filter_map(|r| int_field(r, "code"))
        .collect();
    assert_eq!(seen, BTreeSet::from([0, 1, 2, 3, 4]));
    let listed: BTreeSet<i32> = rows(&text, "table")
        .iter()
        .filter_map(|l| int_field(l, "code"))
        .collect();
    assert_eq!(listed, seen);
}

#[test]
fn a_frame_that_passes_is_written_byte_for_byte() {
    let text = interface_text();
    for row in rows(&text, "scenario") {
        if int_field(row, "code") == Some(0)
            && str_field(row, "stdout_kind").as_deref() == Some("hex")
        {
            assert_eq!(str_field(row, "stdout"), str_field(row, "stdin"));
        }
    }
}

#[test]
fn parameter_rows_equal_the_definition_of_the_stage() {
    let text = interface_text();
    let rows = rows(&text, "param");
    assert_eq!(rows.len(), DEF.params.len());
    for (row, param) in rows.iter().zip(DEF.params) {
        assert_eq!(str_field(row, "param").as_deref(), Some(param.id));
        assert_eq!(
            str_field(row, "kind").as_deref(),
            Some(param.kind.name()),
            "{}",
            param.id
        );
        let default = str_field(row, "default").expect("default");
        match param.kind {
            ParamKind::Int {
                default: d,
                min,
                max,
            } => {
                assert_eq!(default, d.to_string(), "{}", param.id);
                assert_eq!(
                    str_field(row, "range"),
                    Some(format!("{min} to {max}")),
                    "{}",
                    param.id
                );
            }
            ParamKind::Float { default: d, .. } => {
                assert_eq!(default.parse::<f64>().ok(), Some(d), "{}", param.id);
                assert!(str_field(row, "range").is_some(), "{}", param.id);
            }
            ParamKind::Choice {
                default: d,
                choices,
            } => {
                assert_eq!(default, d, "{}", param.id);
                assert_eq!(
                    str_field(row, "choices"),
                    Some(choices.join(", ")),
                    "{}",
                    param.id
                );
            }
            ParamKind::Checkbox { default: d } => {
                assert_eq!(default, d.to_string(), "{}", param.id)
            }
            ParamKind::Text { default: d } => assert_eq!(default, d, "{}", param.id),
        }
        let maps_to = str_field(row, "maps_to").expect("maps_to");
        assert!(
            maps_to == "the stage" || param.about.contains(&format!("({maps_to})")),
            "{}: {maps_to}",
            param.id
        );
    }
}

#[test]
fn the_file_states_the_frame_contract_and_the_data_file() {
    let text = interface_text();
    for word in [
        "4 GiB",
        "partial frame",
        "keys-1",
        "m00",
        "track-report-1",
        "temporal, spool false",
    ] {
        assert!(text.contains(word), "{word}");
    }
    assert!(text.contains("\"geometry\": \"preserve: "));
    assert!(text.is_ascii());
}
