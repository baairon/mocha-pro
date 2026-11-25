//! The stage registry end to end: the manifest and the provider file, the checks every stage
//! shares, and the worked example `resample-scale` with parameters, declared geometry and a key
//! file. Every frame and every key is made up.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use stage_cli::error::StageError;
use stage_cli::registry::{
    Geometry, Kind, Label, ParamDef, ParamKind, Plan, Provider, Stage, StageDef, GENERIC_CLAIM,
};

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

fn text(run: Run) -> String {
    String::from_utf8(run.out).unwrap()
}

fn stage_ids() -> Vec<String> {
    text(spawn(&["list"], b""))
        .lines()
        .map(String::from)
        .collect()
}

/// `apply STAGE` at a size and depth, then the extra words.
fn apply<'a>(stage: &'a str, size: [&'a str; 3], extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "apply", stage, "--width", size[0], "--height", size[1], "--depth", size[2], "--fps",
        "24/1",
    ];
    args.extend_from_slice(extra);
    args
}

fn number(json: &str, key: &str) -> usize {
    let at = json.find(&format!("\"{key}\":")).expect(key) + key.len() + 3;
    json[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .expect(key)
}

fn clip(bytes: usize, seed: u8) -> Vec<u8> {
    (0..bytes)
        .map(|i| (i as u8).wrapping_mul(29).wrapping_add(seed))
        .collect()
}

/// A file with made up content in the temporary folder, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str, content: &str) -> Scratch {
        let path =
            std::env::temp_dir().join(format!("stage-cli-framework-{}-{name}", std::process::id()));
        std::fs::write(&path, content).expect("write the scratch file");
        Scratch(path)
    }

    fn layer(&self) -> String {
        format!("keys={}", self.0.display())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// -- the manifest and the provider file -------------------------------------------------------

#[test]
fn the_registry_follows_its_rules() {
    let problems = stage_cli::stages::provider().problems();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn the_self_test_entry_of_the_manifest_keeps_its_text() {
    let manifest = text(spawn(&["list", "--json"], b""));
    assert!(manifest.contains(
        "\"stages\":[{\"id\":\"selftest-copy\",\"kind\":\"frame\",\"spool\":false,\"geometry\":\"preserve\",\"depths\":[8,16,32],\"params\":[],\"layers\":[],\"name\":\"Self test copy\",\"description\":\"A self test that copies frames unchanged and reimplements nothing.\",\"family\":\"selftest\"}"
    ));
}

#[test]
fn every_generic_stage_says_what_it_is() {
    let provider = stage_cli::stages::provider();
    let manifest = provider.manifest_json(None);
    let mut generic = 0;
    for stage in &provider.stages {
        let def = stage.def();
        match def.label {
            Label::Generic => {
                generic += 1;
                assert!(def.description().ends_with(GENERIC_CLAIM), "{}", def.id);
                assert!(!def.id.starts_with("selftest"), "{}", def.id);
            }
            Label::SelfTest => assert!(def.id.starts_with("selftest-"), "{}", def.id),
            Label::Unit { unit, .. } => panic!(
                "{}: the provider offers a stage of unit {unit} only once that unit has a result file",
                def.id
            ),
        }
    }
    assert_eq!(manifest.matches(GENERIC_CLAIM).count(), generic);
    assert_eq!(manifest.matches("\"family\":\"generic\"").count(), generic);
    assert_eq!(
        manifest.matches("\"family\":").count(),
        provider.stages.len()
    );
}

#[test]
fn the_provider_command_prints_the_manifest_as_a_provider_file() {
    let listed = spawn(&["list", "--json"], b"");
    let file = spawn(&["provider"], b"");
    assert_eq!((file.code, file.err.as_str()), (0, ""));
    assert_eq!(file.out, listed.out);
    let pointed = text(spawn(&["provider", "--program", "../bin/mocha-stage"], b""));
    assert!(pointed.contains("\"command\":[\"../bin/mocha-stage\"]"));
    assert_eq!(
        pointed
            .replace("../bin/mocha-stage", "mocha-stage")
            .as_bytes(),
        listed.out
    );
    assert_eq!(spawn(&["provider", "--json"], b"").code, 2);
}

#[test]
fn the_shipped_provider_file_is_what_the_program_prints() {
    let shipped = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("providers/mocha_pro.stage.json"),
    )
    .expect("read the shipped provider file");
    let printed = spawn(&["provider"], b"").out;
    assert!(
        shipped == printed,
        "providers/mocha_pro.stage.json differs from `mocha-stage provider`: write the output of that command to the file"
    );
}

// -- what every stage shares ------------------------------------------------------------------

#[test]
fn every_stage_answers_describe_and_runs_at_its_defaults() {
    for id in stage_ids() {
        for (depth, bytes) in [("8", 4), ("16", 8), ("32", 16)] {
            let size = ["5", "3", depth];
            let answer = spawn(&apply(&id, size, &["--describe", "--frames", "2"]), b"");
            if answer.code == 3 {
                // the stage does not take this depth: the manifest says which it takes
                assert!(answer.err.starts_with("error: unsupported: depth "), "{id}");
                continue;
            }
            assert_eq!((answer.code, answer.err.as_str()), (0, ""), "{id} {depth}");
            let json = text(answer);
            assert!(
                json.starts_with(&format!(
                    "{{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"{id}\",\"width\":"
                )),
                "{json}"
            );
            assert!(json.ends_with(&format!(
                ",\"depth\":{depth},\"fps\":\"24/1\",\"frames\":2}}\n"
            )));
            let input = clip(5 * 3 * bytes * 2, 7);
            let run = spawn(&apply(&id, size, &[]), &input);
            assert_eq!((run.code, run.err.as_str()), (0, ""), "{id} {depth}");
            assert_eq!(
                run.out.len(),
                number(&json, "width") * number(&json, "height") * bytes * 2,
                "{id} {depth}: the output has the size the stage declared"
            );
            let again = spawn(&apply(&id, size, &[]), &input);
            assert_eq!(
                again.out, run.out,
                "{id} {depth}: two runs give the same bytes"
            );
        }
    }
}

#[test]
fn every_stage_shares_the_refusals() {
    for id in stage_ids() {
        let size = ["3", "2", "8"];
        let frame = clip(3 * 2 * 4, 1);
        let unknown_option = spawn(&apply(&id, size, &["--no-such-option"]), &frame);
        assert_eq!(unknown_option.code, 2, "{id}");
        let depth = spawn(&apply(&id, ["3", "2", "12"], &[]), &frame);
        assert_eq!(depth.code, 3, "{id}");
        let param = spawn(&apply(&id, size, &["--param", "no_such_param=1"]), &frame);
        assert_eq!(param.code, 3, "{id}: {}", param.err);
        assert!(param.out.is_empty());
        let layer = spawn(&apply(&id, size, &["--layer", "no_such_layer=x"]), &frame);
        assert_eq!(layer.code, 3, "{id}: {}", layer.err);
        let partial = spawn(&apply(&id, size, &[]), &frame[..10]);
        assert_eq!(partial.code, 4, "{id}");
        assert!(partial.err.starts_with("error: partial_frame: "), "{id}");
        let empty = spawn(&apply(&id, size, &[]), b"");
        assert_eq!((empty.code, empty.out.len()), (0, 0), "{id}");
        let count = spawn(&apply(&id, size, &["--frames", "2"]), &frame);
        assert_eq!(count.code, 4, "{id}");
        for run in [&unknown_option, &depth, &param, &layer, &partial, &count] {
            assert!(run.err.starts_with("error: "), "{id}: {}", run.err);
            assert_eq!(run.err.matches('\n').count(), 1, "{id}: {}", run.err);
        }
    }
}

// -- the worked example ------------------------------------------------------------------------

const SCALE: &str = "resample-scale";

#[test]
fn resample_scale_is_listed_as_generic_with_declared_geometry() {
    let manifest = text(spawn(&["list", "--json"], b""));
    let at = manifest.find("{\"id\":\"resample-scale\"").expect("listed");
    let entry = &manifest[at..];
    assert!(entry.starts_with(
        "{\"id\":\"resample-scale\",\"kind\":\"frame\",\"spool\":false,\"geometry\":\"declared\",\"depths\":[8,16,32],\"params\":[{\"id\":\"width\",\"name\":\"Width\",\"kind\":\"int\",\"default\":0,\"min\":0,\"max\":65536,\"keyed\":false,"
    ));
    assert!(entry.contains(
        "{\"id\":\"filter\",\"name\":\"Filter\",\"kind\":\"popup\",\"default\":\"bilinear\",\"choices\":[\"nearest\",\"bilinear\",\"keys\",\"mitchell\",\"lanczos\",\"box\"],\"keyed\":false,"
    ));
    assert!(entry.contains(
        "{\"id\":\"keys_a\",\"name\":\"Keys a\",\"kind\":\"float\",\"default\":-0.5,\"min\":-1,\"max\":0,\"keyed\":false,"
    ));
    assert!(entry.contains("{\"id\":\"shift_x\",\"name\":\"Shift X\",\"kind\":\"float\",\"default\":0,\"min\":-65536,\"max\":65536,\"keyed\":true,"));
    assert!(entry.contains("\"layers\":[{\"id\":\"keys\",\"optional\":true,\"form\":\"keys-1\","));
    assert!(entry.contains("not compared with any product.\",\"family\":\"generic\"}"));
}

#[test]
fn resample_scale_declares_its_output_size() {
    let args = apply(
        SCALE,
        ["4", "3", "16"],
        &[
            "--describe",
            "--frames",
            "7",
            "--param",
            "width=8",
            "--param",
            "height=1",
        ],
    );
    let answer = spawn(&args, b"");
    assert_eq!(
        (answer.code, text(answer).as_str()),
        (
            0,
            "{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"resample-scale\",\"width\":8,\"height\":1,\"depth\":16,\"fps\":\"24/1\",\"frames\":7}\n"
        )
    );
    let kept = text(spawn(
        &apply(
            SCALE,
            ["4", "3", "8"],
            &["--describe", "--param", "width=9"],
        ),
        b"",
    ));
    assert!(kept.contains("\"width\":9,\"height\":3,\"depth\":8,\"fps\":\"24/1\",\"frames\":null}"));
}

#[test]
fn resample_scale_at_its_defaults_returns_the_input() {
    for (depth, bytes) in [("8", 4), ("16", 8)] {
        let input = clip(6 * 4 * bytes * 3, 11);
        let run = spawn(&apply(SCALE, ["6", "4", depth], &[]), &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""));
        assert!(run.out == input, "depth {depth}");
    }
    let floats: Vec<u8> = (0..6 * 4 * 4)
        .flat_map(|i| (i as f32 * 0.03125 - 0.5).to_le_bytes())
        .collect();
    let run = spawn(&apply(SCALE, ["6", "4", "32"], &[]), &floats);
    assert_eq!(run.code, 0);
    assert!(run.out == floats, "depth 32");
}

#[test]
fn resample_scale_doubles_a_frame_with_the_nearest_filter() {
    let input = [10, 20, 30, 40, 50, 60, 70, 80];
    let args = apply(
        SCALE,
        ["2", "1", "8"],
        &[
            "--param",
            "width=4",
            "--param",
            "height=2",
            "--param",
            "filter=nearest",
        ],
    );
    let run = spawn(&args, &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    let row = [
        10, 20, 30, 40, 10, 20, 30, 40, 50, 60, 70, 80, 50, 60, 70, 80,
    ];
    assert_eq!(run.out, [row, row].concat());
}

#[test]
fn resample_scale_halves_a_frame_with_the_box_filter() {
    // 4 by 1 pixels at depth 16, alpha at full scale; each output pixel is the mean of two
    let pixel = |v: u16| [v, v, v, 65535].map(u16::to_le_bytes).concat();
    let input = [pixel(100), pixel(300), pixel(1000), pixel(3000)].concat();
    let args = apply(
        SCALE,
        ["4", "1", "16"],
        &[
            "--param",
            "width=2",
            "--param",
            "filter=box",
            "--param",
            "footprint=jacobian",
        ],
    );
    let run = spawn(&args, &input);
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert_eq!(run.out, [pixel(200), pixel(2000)].concat());
}

#[test]
fn a_key_file_moves_a_parameter_from_frame_to_frame() {
    let frame = [1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3];
    let shifted = [2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3];
    let input = [frame, frame, frame].concat();
    let keys = Scratch::new(
        "moves",
        "keys-1\nkey shift_x 0 0 hold\nkey shift_x 1 1 hold\n",
    );
    let layer = keys.layer();
    let size = ["3", "1", "8"];
    let run = spawn(
        &apply(
            SCALE,
            size,
            &["--param", "filter=nearest", "--layer", layer.as_str()],
        ),
        &input,
    );
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert_eq!(run.out, [frame, shifted, shifted].concat());
    let fixed = spawn(
        &apply(
            SCALE,
            size,
            &["--param", "filter=nearest", "--param", "shift_x=1"],
        ),
        &input,
    );
    assert_eq!(fixed.out, [shifted, shifted, shifted].concat());
}

#[test]
fn a_key_file_start_names_the_time_of_the_first_frame() {
    let frame = [1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3];
    let shifted = [2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3];
    let keys = Scratch::new(
        "start",
        "keys-1\nstart 100\nkey shift_x 100 1 hold\nkey shift_x 101 0 hold\n",
    );
    let layer = keys.layer();
    let run = spawn(
        &apply(
            SCALE,
            ["3", "1", "8"],
            &["--param", "filter=nearest", "--layer", layer.as_str()],
        ),
        &[frame, frame].concat(),
    );
    assert_eq!(run.code, 0, "{}", run.err);
    assert_eq!(run.out, [shifted, frame].concat());
}

#[test]
fn key_file_refusals() {
    let frame = clip(3 * 4, 3);
    let size = ["3", "1", "8"];
    let missing = spawn(
        &apply(SCALE, size, &["--layer", "keys=no/such/key/file"]),
        &frame,
    );
    assert_eq!(missing.code, 4);
    assert!(
        missing
            .err
            .starts_with("error: bad_input: cannot read the key file"),
        "{}",
        missing.err
    );
    assert!(missing.out.is_empty());
    for (name, content, code, needle) in [
        (
            "form",
            "keys-2\n",
            4,
            "line 1: the first line is not `keys-1`",
        ),
        ("kind", "keys-1\nkey shift_x 0 1 bezier\n", 4, "line 2: "),
        (
            "param",
            "keys-1\nkey width 0 1 hold\n",
            3,
            "no keyed parameter `width`",
        ),
        (
            "other",
            "keys-1\nkey gain 0 1 hold\n",
            3,
            "no keyed parameter `gain`",
        ),
        (
            "range",
            "keys-1\nkey shift_y 0 70000 hold\n",
            3,
            "is outside",
        ),
    ] {
        let keys = Scratch::new(name, content);
        let layer = keys.layer();
        let run = spawn(&apply(SCALE, size, &["--layer", layer.as_str()]), &frame);
        assert_eq!(run.code, code, "{name}: {}", run.err);
        assert!(run.err.contains(needle), "{name}: {}", run.err);
        assert!(run.out.is_empty(), "{name}");
    }
    let twice = spawn(
        &apply(SCALE, size, &["--layer", "keys=a", "--layer", "keys=b"]),
        &frame,
    );
    assert_eq!(twice.code, 2);
    let described = spawn(
        &apply(
            SCALE,
            size,
            &["--describe", "--layer", "keys=no/such/key/file"],
        ),
        b"",
    );
    assert_eq!(described.code, 0, "describe opens no layer");
}

#[test]
fn parameter_refusals_follow_the_convention() {
    let frame = clip(3 * 4, 5);
    let size = ["3", "1", "8"];
    for (params, code, needle) in [
        (
            &["width=-1"][..],
            3,
            "bad value for `width`: -1 is outside 0 to 65536",
        ),
        (
            &["width=x"][..],
            3,
            "bad value for `width`: cannot parse `x`",
        ),
        (
            &["filter=cubic"][..],
            3,
            "`cubic` is not one of nearest, bilinear",
        ),
        (&["lobes=0"][..], 3, "bad value for `lobes`"),
        (&["shift_x=1@0;2@5"][..], 3, "--layer keys=PATH"),
        (
            &["scale=2"][..],
            3,
            "resample-scale has no parameter `scale`: it takes width, height",
        ),
        (
            &["width=4", "width=5"][..],
            2,
            "parameter `width` given more than once",
        ),
    ] {
        let mut extra = Vec::new();
        for p in params {
            extra.push("--param");
            extra.push(*p);
        }
        for describe in [false, true] {
            let mut words = extra.clone();
            if describe {
                words.push("--describe");
            }
            let run = spawn(&apply(SCALE, size, &words), &frame);
            assert_eq!(run.code, code, "{params:?}: {}", run.err);
            assert!(run.err.contains(needle), "{params:?}: {}", run.err);
            assert!(run.out.is_empty());
        }
    }
}

#[test]
fn an_output_frame_above_four_gib_is_refused() {
    let args = apply(
        SCALE,
        ["3", "1", "32"],
        &[
            "--describe",
            "--param",
            "width=65536",
            "--param",
            "height=65536",
        ],
    );
    let run = spawn(&args, b"");
    assert_eq!(run.code, 3);
    assert!(run
        .err
        .starts_with("error: unsupported: an output frame of 65536x65536"));
}

// -- a provider built on the same library -----------------------------------------------------

struct SampleNegate;

static SAMPLE: StageDef = StageDef {
    id: "sample-negate",
    name: "Sample negate",
    about: "A made up stage of a made up provider: it writes 255 minus every byte, or the bytes as they are.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Preserve,
    depths: &[8],
    params: &[ParamDef {
        id: "active",
        name: "Active",
        kind: ParamKind::Checkbox { default: true },
        keyed: false,
        about: "Whether the bytes are changed.",
    }],
    layers: &[],
};

impl Stage for SampleNegate {
    fn def(&self) -> &'static StageDef {
        &SAMPLE
    }

    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError> {
        let active = plan.values.flag("active");
        let bytes = plan.input.buffer_len()?;
        stage_cli::run::map_frames(input, output, bytes, bytes, plan.frames, |_, frame, out| {
            out.clear();
            out.extend(frame.iter().map(|b| if active { 255 - b } else { *b }));
            Ok(())
        })
    }
}

fn sample_provider() -> Provider {
    Provider {
        id: "sample_provider",
        program: "sample-stage",
        version: "sample-stage 9.9.9".to_string(),
        stages: vec![&SampleNegate],
    }
}

fn sample(args: &[&str], input: &[u8]) -> (i32, Vec<u8>, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut inp = input;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = {
        let mut io = stage_cli::Io {
            input: &mut inp,
            output: &mut out,
            input_is_terminal: false,
            output_is_terminal: false,
        };
        stage_cli::run_cli_with(&sample_provider(), &args, &mut io, &mut err)
    };
    (code, out, String::from_utf8(err).unwrap())
}

#[test]
fn a_second_provider_runs_on_the_same_library() {
    assert!(sample_provider().problems().is_empty());
    let (code, out, _) = sample(&["list", "--json"], b"");
    let manifest = String::from_utf8(out).unwrap();
    assert_eq!(code, 0);
    assert!(manifest.starts_with(
        "{\"contract\":\"frames/2\",\"provider\":{\"id\":\"sample_provider\",\"version\":\"sample-stage 9.9.9\",\"command\":[\"sample-stage\"]},\"stages\":[{\"id\":\"sample-negate\","
    ));
    assert!(!manifest.contains("selftest-copy"));
    let (_, out, _) = sample(&["--version"], b"");
    assert_eq!(out, b"sample-stage 9.9.9\n");
    let (_, out, _) = sample(&["--help"], b"");
    assert!(String::from_utf8(out)
        .unwrap()
        .starts_with("usage: sample-stage list"));
    let run = apply("sample-negate", ["1", "1", "8"], &[]);
    assert_eq!(sample(&run, &[0, 1, 254, 255]).1, [255, 254, 1, 0]);
    let off = apply("sample-negate", ["1", "1", "8"], &["--param", "active=0"]);
    assert_eq!(sample(&off, &[0, 1, 254, 255]).1, [0, 1, 254, 255]);
    let (code, _, err) = sample(&apply("selftest-copy", ["1", "1", "8"], &[]), b"");
    assert_eq!(code, 3);
    assert_eq!(
        err,
        "error: unsupported: unknown stage `selftest-copy`: this provider has sample-negate\n"
    );
}

#[test]
fn a_registry_that_breaks_the_rules_is_reported() {
    struct Twice;
    static BAD: StageDef = StageDef {
        id: "Bad Id",
        name: "",
        about: "A made up stage that breaks the rules.",
        label: Label::Generic,
        kind: Kind::Frame,
        spool: false,
        geometry: Geometry::Preserve,
        depths: &[8, 12],
        params: &[
            ParamDef {
                id: "gain",
                name: "Gain",
                kind: ParamKind::Float {
                    default: 3.0,
                    min: 0.0,
                    max: 1.0,
                },
                keyed: true,
                about: "A made up float with a default outside its range.",
            },
            ParamDef {
                id: "gain",
                name: "Gain",
                kind: ParamKind::Int {
                    default: 0,
                    min: 0,
                    max: 1,
                },
                keyed: true,
                about: "A made up repeated and keyed whole number.",
            },
        ],
        layers: &[],
    };
    impl Stage for Twice {
        fn def(&self) -> &'static StageDef {
            &BAD
        }
        fn run(&self, _: &Plan, _: &mut dyn Read, _: &mut dyn Write) -> Result<u64, StageError> {
            Ok(0)
        }
    }
    let provider = Provider {
        id: "sample_provider",
        program: "sample-stage",
        version: "sample-stage 9.9.9".to_string(),
        stages: vec![&Twice, &Twice],
    };
    let problems = provider.problems().join("\n");
    for needle in [
        "the stage id is not lower case",
        "registered twice",
        "the name or the sentence is empty",
        "the depths are not a list from 8, 16 and 32",
        "parameter `gain` has a bad or repeated id",
        "parameter `gain` has a default outside its values",
        "parameter `gain` is keyed but not a float",
        "a stage with a keyed parameter has the optional layer `keys`",
    ] {
        assert!(problems.contains(needle), "{needle}: {problems}");
    }
}
