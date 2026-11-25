//! Checks `docs/data/stage_raster_polygon_interface.json` and
//! `docs/data/stage_raster_contour_interface.json` against the built program: every scenario of
//! the two files is run and compared, the scenarios cover the exit codes of the table, and the
//! parameter and layer rows equal the manifest entry of the stage. The files are read as plain
//! text, one row per line. A scenario runs in a scratch folder that holds its layer files as
//! `shape.txt` and `keys.txt`; `{shape}` and `{keys}` in its arguments stand for those names.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const STAGES: [&str; 2] = ["raster-polygon", "raster-contour"];

fn interface_text(stage: &str) -> String {
    let name = format!("stage_{}_interface.json", stage.replace('-', "_"));
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/data")
        .join(name);
    std::fs::read_to_string(&path).expect("read the interface file")
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

/// The bare value of `"key": value` in one line: a number, `true` or `false`.
fn bare_field(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\": ");
    let start = line.find(&pat)? + pat.len();
    let value: String = line[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    (!value.is_empty()).then_some(value)
}

fn rows<'a>(text: &'a str, mark: &str) -> Vec<&'a str> {
    text.lines().filter(|l| l.contains(mark)).collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

struct Run {
    code: i32,
    out: Vec<u8>,
    err: String,
}

fn spawn(args: &[String], input: &[u8], folder: &Path) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mocha-stage"))
        .args(args)
        .current_dir(folder)
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

/// A folder of its own in the temporary folder, removed when dropped.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let path = std::env::temp_dir().join(format!(
            "stage-cli-raster-interface-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("make the scratch folder");
        Folder(path)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn exit_code_rows_equal_the_program() {
    for stage in STAGES {
        let text = interface_text(stage);
        let scenarios = rows(&text, "\"scenario\"");
        assert!(scenarios.len() >= 25, "{stage}: {} rows", scenarios.len());
        let folder = Folder::new(stage);
        for line in scenarios {
            let name = str_field(line, "scenario").unwrap();
            assert_eq!(str_field(line, "tool").unwrap(), "mocha-stage", "{name}");
            let mut args = str_field(line, "args").unwrap();
            for layer in ["shape", "keys"] {
                let content = str_field(line, layer).unwrap();
                if !content.is_empty() {
                    let name = format!("{layer}.txt");
                    std::fs::write(folder.0.join(&name), content).unwrap();
                    args = args.replace(&format!("{{{layer}}}"), &name);
                }
            }
            assert!(!args.contains('{'), "{stage} {name}: {args}");
            let args: Vec<String> = args.split(' ').map(String::from).collect();
            assert_eq!(args[..2], ["apply", stage], "{name}");
            let stdin = str_field(line, "stdin").unwrap();
            let input = if stdin == "none" {
                Vec::new()
            } else {
                unhex(&stdin)
            };
            let run = spawn(&args, &input, &folder.0);
            let code: i32 = bare_field(line, "code").unwrap().parse().unwrap();
            assert_eq!(run.code, code, "{stage} {name}: {}", run.err);
            let stderr = str_field(line, "stderr").unwrap();
            match str_field(line, "stderr_kind").unwrap().as_str() {
                "empty" => assert_eq!(run.err, "", "{stage} {name}"),
                "exact" => assert_eq!(run.err, stderr, "{stage} {name}"),
                "prefix" => {
                    assert!(run.err.starts_with(&stderr), "{stage} {name}: {}", run.err);
                    assert_eq!(run.err.matches('\n').count(), 1, "{stage} {name}");
                }
                other => panic!("{stage} {name}: unknown stderr_kind {other}"),
            }
            let stdout = str_field(line, "stdout").unwrap();
            match str_field(line, "stdout_kind").unwrap().as_str() {
                "empty" => assert!(run.out.is_empty(), "{stage} {name}"),
                "hex" => assert_eq!(hex(&run.out), stdout, "{stage} {name}"),
                other => panic!("{stage} {name}: unknown stdout_kind {other}"),
            }
        }
    }
}

#[test]
fn rows_cover_every_exit_code_a_scenario_can_reach() {
    for stage in STAGES {
        let text = interface_text(stage);
        let table: Vec<String> = rows(&text, "\"means\"")
            .iter()
            .map(|l| bare_field(l, "code").unwrap())
            .collect();
        assert_eq!(table, ["0", "1", "2", "3", "4"], "{stage}");
        let mut seen: Vec<String> = rows(&text, "\"scenario\"")
            .iter()
            .map(|l| bare_field(l, "code").unwrap())
            .collect();
        seen.sort();
        seen.dedup();
        // exit 1 is a failed write or a failure of the rasteriser on a frame: no scenario
        assert_eq!(seen, ["0", "2", "3", "4"], "{stage}");
        assert!(text.contains("\"schema_version\": 1"), "{stage}");
        assert!(
            text.contains(&format!("\"name\": \"stage-{stage}\"")),
            "{stage}"
        );
        assert!(text.contains("not compared with any product"), "{stage}");
    }
}

/// The text between `open` and the next `close` after it.
fn between<'a>(text: &'a str, open: &str, close: &str) -> &'a str {
    let start = text.find(open).expect(open) + open.len();
    let end = text[start..].find(close).expect(close);
    &text[start..start + end]
}

#[test]
fn parameter_and_layer_rows_equal_the_manifest_entry() {
    let listed = spawn(
        &["list".to_string(), "--json".to_string()],
        b"",
        &std::env::temp_dir(),
    );
    let manifest = String::from_utf8(listed.out).unwrap();
    for stage in STAGES {
        let at = manifest
            .find(&format!("{{\"id\":\"{stage}\""))
            .expect("the stage is listed");
        let entry = &manifest[at..];
        let params = between(entry, "\"params\":[", "],\"layers\":[");
        let from_manifest: Vec<[String; 4]> = params
            .split("{\"id\":\"")
            .skip(1)
            .map(|p| {
                let id = p[..p.find('"').unwrap()].to_string();
                let kind = between(p, "\"kind\":\"", "\"").to_string();
                let default = between(p, "\"default\":", ",\"")
                    .trim_matches('"')
                    .to_string();
                let keyed = between(p, "\"keyed\":", ",").to_string();
                [id, kind, default, keyed]
            })
            .collect();
        let text = interface_text(stage);
        let from_file: Vec<[String; 4]> = rows(&text, "\"maps_to\"")
            .iter()
            .map(|l| {
                [
                    str_field(l, "id").unwrap(),
                    str_field(l, "kind").unwrap(),
                    str_field(l, "default").unwrap(),
                    bare_field(l, "keyed").unwrap(),
                ]
            })
            .collect();
        assert!(from_file.len() >= 15, "{stage}");
        assert_eq!(from_file, from_manifest, "{stage}");
        let layers = between(entry, "\"layers\":[", "],\"name\":");
        let layers_of_manifest: Vec<[String; 3]> = layers
            .split("{\"id\":\"")
            .skip(1)
            .map(|l| {
                [
                    l[..l.find('"').unwrap()].to_string(),
                    between(l, "\"optional\":", ",").to_string(),
                    between(l, "\"form\":\"", "\"").to_string(),
                ]
            })
            .collect();
        let layers_of_file: Vec<[String; 3]> = rows(&text, "\"holds\"")
            .iter()
            .map(|l| {
                [
                    str_field(l, "id").unwrap(),
                    bare_field(l, "optional").unwrap(),
                    str_field(l, "form").unwrap(),
                ]
            })
            .collect();
        assert_eq!(layers_of_file, layers_of_manifest, "{stage}");
        assert_eq!(layers_of_file.len(), 2, "{stage}");
    }
}
