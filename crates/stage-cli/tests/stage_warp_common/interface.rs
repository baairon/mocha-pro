//! Holds an interface file `docs/data/stage_<id>_interface.json` to the built program: each exit
//! code row of the file is one run of a child process, and the parameter and layer rows are
//! compared with the manifest. The file is read as text, one row per line, so the tests need no
//! JSON reader.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::manifest_entry;

/// The text of the interface file of a stage.
pub fn interface_text(stage: &str) -> String {
    let name = format!("stage_{}_interface.json", stage.replace('-', "_"));
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/data")
        .join(name);
    std::fs::read_to_string(path).expect("read the interface file")
}

/// The string value of `"key": "..."` in one line (escapes: `\n`, `\\` and `\"` only).
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

/// The bare value of `"key": value` in one line: a number, `true` or `false`.
fn bare_field(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\": ");
    let start = line.find(&pat)? + pat.len();
    let word: String = line[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    (!word.is_empty()).then_some(word)
}

fn rows(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| l.contains("\"scenario\""))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

fn fresh_dir(stage: &str, scenario: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stage-cli-warp-interface-{}-{stage}-{scenario}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the scratch folder");
    dir
}

/// Run every exit code row of the file of `stage` and compare the exit code and both streams.
/// `expect` is the number of rows the file holds. A row runs in a fresh folder that holds the
/// layer files the row names: `keys.txt` for its `keys` text and `mesh.txt` for its `mesh` text.
pub fn rows_equal_the_program(stage: &str, expect: usize) {
    let text = interface_text(stage);
    let rows = rows(&text);
    assert_eq!(rows.len(), expect, "{stage}: rows of the interface file");
    let mut names = BTreeSet::new();
    for row in rows {
        let scenario = str_field(row, "scenario").expect("scenario");
        assert!(names.insert(scenario.clone()), "{scenario} comes twice");
        assert_eq!(str_field(row, "tool").as_deref(), Some("mocha-stage"));
        let args = str_field(row, "args").expect("args");
        assert!(
            args.starts_with(&format!("apply {stage} ")),
            "{scenario}: {args}"
        );
        let dir = fresh_dir(stage, &scenario);
        for layer in ["keys", "mesh"] {
            let content = str_field(row, layer).expect(layer);
            if content != "none" {
                std::fs::write(dir.join(format!("{layer}.txt")), content).expect("write a layer");
            }
        }
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
        // the stage may exit before it reads everything: ignore a closed pipe
        child.stdin.take().unwrap().write_all(&feed).ok();
        let out = child.wait_with_output().expect("run");
        let _ = std::fs::remove_dir_all(&dir);
        let code = bare_field(row, "code").expect("code");
        assert_eq!(
            out.status.code().map(|c| c.to_string()),
            Some(code),
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

/// The rows use the exit codes 0, 2, 3 and 4; the table lists 0 to 4; exit 1 is named as a path
/// no row makes.
pub fn rows_cover_the_exit_codes(stage: &str) {
    let text = interface_text(stage);
    let seen: BTreeSet<String> = rows(&text)
        .iter()
        .filter_map(|r| bare_field(r, "code"))
        .collect();
    let codes =
        |list: &[&str]| -> BTreeSet<String> { list.iter().map(|c| c.to_string()).collect() };
    assert_eq!(seen, codes(&["0", "2", "3", "4"]), "{stage}");
    let listed: BTreeSet<String> = text
        .lines()
        .filter(|l| l.contains("\"table\""))
        .filter_map(|l| bare_field(l, "code"))
        .collect();
    assert_eq!(listed, codes(&["0", "1", "2", "3", "4"]), "{stage}");
    let unpinned: Vec<&str> = text.lines().filter(|l| l.contains("\"why\"")).collect();
    assert!(unpinned.iter().any(|l| {
        str_field(l, "path").as_deref() == Some("stage_failed")
            && bare_field(l, "exit").as_deref() == Some("1")
    }));
}

/// The id, kind, default and keyed word of every parameter of a manifest entry, in order.
fn manifest_params(entry: &str) -> Vec<[String; 4]> {
    let params = &entry[entry.find("\"params\":[").unwrap()..entry.find("\"layers\":[").unwrap()];
    let after = |part: &str, key: &str| -> String {
        let at = part.find(key).unwrap_or_else(|| panic!("{key} in {part}")) + key.len();
        let rest = &part[at..];
        match rest.strip_prefix('"') {
            Some(text) => text[..text.find('"').unwrap()].to_string(),
            None => rest[..rest.find(',').unwrap()].to_string(),
        }
    };
    params
        .split("{\"id\":")
        .skip(1)
        .map(|part| {
            [
                after(part, ""),
                after(part, "\"kind\":"),
                after(part, "\"default\":"),
                after(part, "\"keyed\":"),
            ]
        })
        .collect()
}

/// The parameter rows and the layer rows of the file are those of the manifest entry.
pub fn parameters_and_layers_equal_the_manifest(stage: &str) {
    let text = interface_text(stage);
    let entry = manifest_entry(stage);
    let listed: Vec<[String; 4]> = text
        .lines()
        .filter(|l| l.contains("\"maps_to\""))
        .map(|l| {
            [
                str_field(l, "id").expect("id"),
                str_field(l, "kind").expect("kind"),
                str_field(l, "default").expect("default"),
                bare_field(l, "keyed").expect("keyed"),
            ]
        })
        .collect();
    assert_eq!(listed, manifest_params(&entry), "{stage}: parameters");
    for line in text.lines().filter(|l| l.contains("\"maps_to\"")) {
        // the text of the row is the description the manifest prints for the parameter
        let maps_to = str_field(line, "maps_to").expect("maps_to");
        assert!(
            entry.contains(&format!("\"description\":\"{maps_to}\"")),
            "{line}"
        );
    }
    let layers: Vec<String> = text
        .lines()
        .filter(|l| l.contains("\"layer\""))
        .map(|l| {
            format!(
                "{{\"id\":\"{}\",\"optional\":{},\"form\":\"{}\",",
                str_field(l, "layer").expect("layer"),
                bare_field(l, "optional").expect("optional"),
                str_field(l, "form").expect("form")
            )
        })
        .collect();
    let in_manifest = &entry[entry.find("\"layers\":[").unwrap()..];
    assert_eq!(
        layers.len(),
        in_manifest.matches("{\"id\":").count(),
        "{stage}: layers"
    );
    for layer in layers {
        assert!(in_manifest.contains(&layer), "{stage}: {layer}");
    }
    assert!(text.contains(&format!("\"name\": \"stage-{stage}\"")));
    assert!(text.contains("\"schema_version\": 1,"));
}
