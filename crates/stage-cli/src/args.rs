//! The command line: `list`, `apply STAGE ...`, `--version` and `--help`.
//!
//! Parsing only checks the form of the command line (`usage` errors). Whether the stage, the
//! depth and the parameters are supported is decided afterwards by [`crate::stage`].

use crate::error::StageError;

/// A frame rate as the exact fraction it was given in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fps {
    /// The numerator, at least 1.
    pub num: u64,
    /// The denominator, at least 1.
    pub den: u64,
}

impl Fps {
    /// Parse `N` or `N/D`: ASCII digits only, both parts at least 1.
    pub fn parse(text: &str) -> Result<Fps, StageError> {
        let bad = || {
            StageError::usage(format!(
                "--fps expects N/D with positive whole numbers, got `{text}`"
            ))
        };
        let (n, d) = match text.split_once('/') {
            Some((n, d)) => (n, d),
            None => (text, "1"),
        };
        let num = positive(n).ok_or_else(bad)?;
        let den = positive(d).ok_or_else(bad)?;
        Ok(Fps { num, den })
    }

    /// The `N/D` text.
    pub fn text(&self) -> String {
        format!("{}/{}", self.num, self.den)
    }
}

/// What the command line asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `--help` or `-h`.
    Help,
    /// `--version`.
    Version,
    /// `list [--json]`.
    List {
        /// Print the manifest instead of one stage id per line.
        json: bool,
    },
    /// `apply STAGE ...`.
    Apply(Apply),
}

/// The request of `apply`, as typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Apply {
    /// The stage id.
    pub stage: String,
    /// `--width`.
    pub width: u32,
    /// `--height`.
    pub height: u32,
    /// `--depth`, any positive whole number (the stage decides whether it supports it).
    pub depth: u64,
    /// `--fps`.
    pub fps: Fps,
    /// `--frames`, the expected frame count.
    pub frames: Option<u64>,
    /// `--describe`.
    pub describe: bool,
    /// Every `--param K=V`.
    pub params: Vec<(String, String)>,
    /// Every `--layer ID=PATH`.
    pub layers: Vec<(String, String)>,
}

/// The usage text.
pub const HELP: &str = "usage: mocha-stage list [--json]
       mocha-stage apply STAGE --width W --height H --depth 8|16|32 --fps N/D [--frames N]
       mocha-stage apply STAGE --describe --width W --height H --depth D --fps N/D [--frames N]
       mocha-stage --version | --help

list prints the stages of this provider (--json: a stage manifest of the handoff convention).
apply reads raw frames on standard input and writes them on standard output; STAGE is
selftest-copy, a self test that copies frames unchanged. It takes no --param and no --layer.
--describe answers the geometry of the run as one JSON document and reads no input.
errors: one line `error: <code>: <message>` on standard error; exit 2 usage, 3 unsupported,
4 bad input (also a stream that ends inside a frame), 1 any other failure.
";

/// `ASCII digits only` text to a positive number.
fn positive(text: &str) -> Option<u64> {
    let n = whole(text)?;
    if n == 0 {
        None
    } else {
        Some(n)
    }
}

/// ASCII digits only (no sign, no space) to a number.
fn whole(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>().ok()
}

fn dimension(name: &str, text: &str) -> Result<u32, StageError> {
    let n = positive(text).ok_or_else(|| {
        StageError::usage(format!(
            "{name} expects a positive whole number, got `{text}`"
        ))
    })?;
    u32::try_from(n).map_err(|_| StageError::usage(format!("{name} is too large: `{text}`")))
}

fn pair(name: &str, text: &str) -> Result<(String, String), StageError> {
    match text.split_once('=') {
        Some((k, v)) if !k.is_empty() => Ok((k.to_string(), v.to_string())),
        _ => Err(StageError::usage(format!(
            "{name} expects KEY=VALUE, got `{text}`"
        ))),
    }
}

/// Parse the arguments after the program name.
pub fn parse(args: &[String]) -> Result<Command, StageError> {
    let first = args
        .first()
        .ok_or_else(|| StageError::usage("no command given: use list or apply (see --help)"))?;
    match first.as_str() {
        "--help" | "-h" => {
            if args.len() > 1 {
                return Err(StageError::usage("--help takes no other argument"));
            }
            Ok(Command::Help)
        }
        "--version" => {
            if args.len() > 1 {
                return Err(StageError::usage("--version takes no other argument"));
            }
            Ok(Command::Version)
        }
        "list" => {
            let mut json = false;
            for a in &args[1..] {
                match a.as_str() {
                    "--json" if !json => json = true,
                    "--json" => return Err(StageError::usage("--json given twice")),
                    other => {
                        return Err(StageError::usage(format!("list does not take `{other}`")))
                    }
                }
            }
            Ok(Command::List { json })
        }
        "apply" => parse_apply(&args[1..]).map(Command::Apply),
        other => Err(StageError::usage(format!(
            "unknown command `{other}`: use list or apply (see --help)"
        ))),
    }
}

fn parse_apply(args: &[String]) -> Result<Apply, StageError> {
    let mut stage: Option<String> = None;
    let mut width = None;
    let mut height = None;
    let mut depth = None;
    let mut fps = None;
    let mut frames = None;
    let mut describe = false;
    let mut params = Vec::new();
    let mut layers = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if !arg.starts_with("--") {
            if stage.is_some() {
                return Err(StageError::usage(format!("unexpected argument `{arg}`")));
            }
            stage = Some(arg.clone());
            continue;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (arg.as_str(), None),
        };
        if name == "--describe" {
            if inline.is_some() {
                return Err(StageError::usage("--describe takes no value"));
            }
            if describe {
                return Err(StageError::usage("--describe given twice"));
            }
            describe = true;
            continue;
        }
        if !matches!(
            name,
            "--width" | "--height" | "--depth" | "--fps" | "--frames" | "--param" | "--layer"
        ) {
            return Err(StageError::usage(format!("unknown option `{name}`")));
        }
        let value = match inline {
            Some(v) => v,
            None => {
                let v = args
                    .get(i)
                    .ok_or_else(|| StageError::usage(format!("{name} needs a value")))?;
                i += 1;
                v.clone()
            }
        };
        let twice = || StageError::usage(format!("{name} given twice"));
        match name {
            "--width" => {
                if width.replace(dimension(name, &value)?).is_some() {
                    return Err(twice());
                }
            }
            "--height" => {
                if height.replace(dimension(name, &value)?).is_some() {
                    return Err(twice());
                }
            }
            "--depth" => {
                let n = positive(&value).ok_or_else(|| {
                    StageError::usage(format!(
                        "--depth expects a positive whole number, got `{value}`"
                    ))
                })?;
                if depth.replace(n).is_some() {
                    return Err(twice());
                }
            }
            "--fps" => {
                if fps.replace(Fps::parse(&value)?).is_some() {
                    return Err(twice());
                }
            }
            "--frames" => {
                let n = whole(&value).ok_or_else(|| {
                    StageError::usage(format!("--frames expects a whole number, got `{value}`"))
                })?;
                if frames.replace(n).is_some() {
                    return Err(twice());
                }
            }
            "--param" => params.push(pair(name, &value)?),
            _ => layers.push(pair(name, &value)?),
        }
    }
    let stage = stage.ok_or_else(|| StageError::usage("apply needs a stage id"))?;
    let need = |name: &str| StageError::usage(format!("apply needs {name}"));
    Ok(Apply {
        stage,
        width: width.ok_or_else(|| need("--width"))?,
        height: height.ok_or_else(|| need("--height"))?,
        depth: depth.ok_or_else(|| need("--depth"))?,
        fps: fps.ok_or_else(|| need("--fps"))?,
        frames,
        describe,
        params,
        layers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Code;

    fn v(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    fn apply(s: &str) -> Apply {
        match parse(&v(s)).expect("parses") {
            Command::Apply(a) => a,
            other => panic!("not apply: {other:?}"),
        }
    }

    fn usage(s: &str) -> String {
        let e = parse(&v(s)).expect_err("refused");
        assert_eq!(e.code, Code::Usage, "{s}: {e}");
        e.message
    }

    #[test]
    fn fps_forms() {
        assert_eq!(Fps::parse("24").unwrap(), Fps { num: 24, den: 1 });
        assert_eq!(Fps::parse("30000/1001").unwrap().text(), "30000/1001");
        assert_eq!(Fps::parse("50/2").unwrap().text(), "50/2");
        for bad in [
            "", "0", "0/1", "24/0", "24/", "/1", "1/2/3", "-24", "+24", "24.0", "a/b", " 24",
            "24 /1",
        ] {
            assert!(Fps::parse(bad).is_err(), "{bad:?}");
        }
        assert!(Fps::parse("99999999999999999999999/1").is_err());
    }

    #[test]
    fn full_apply() {
        let a = apply("apply selftest-copy --width 4 --height 3 --depth 16 --fps 24/1 --frames 5");
        assert_eq!(a.stage, "selftest-copy");
        assert_eq!((a.width, a.height, a.depth, a.frames), (4, 3, 16, Some(5)));
        assert_eq!(a.fps.text(), "24/1");
        assert!(!a.describe && a.params.is_empty() && a.layers.is_empty());
    }

    #[test]
    fn stage_may_come_after_the_flags() {
        let a = apply("apply --width 4 --height 3 --depth 8 --fps 24 selftest-copy");
        assert_eq!(a.stage, "selftest-copy");
    }

    #[test]
    fn equals_form() {
        let a = apply("apply s --width=4 --height=3 --depth=8 --fps=25/1 --frames=0 --param=a=b");
        assert_eq!((a.width, a.frames), (4, Some(0)));
        assert_eq!(a.params, vec![("a".to_string(), "b".to_string())]);
    }

    #[test]
    fn params_and_layers_are_collected() {
        let a = apply(
            "apply s --width 1 --height 1 --depth 8 --fps 1 --param a=1 --param b= --layer x=p/q",
        );
        assert_eq!(a.params.len(), 2);
        assert_eq!(a.params[1], ("b".to_string(), String::new()));
        assert_eq!(a.layers, vec![("x".to_string(), "p/q".to_string())]);
    }

    #[test]
    fn describe_flag() {
        assert!(apply("apply s --describe --width 1 --height 1 --depth 8 --fps 1").describe);
    }

    #[test]
    fn missing_pieces_are_usage() {
        for s in [
            "apply",
            "apply s",
            "apply s --height 1 --depth 8 --fps 1",
            "apply s --width 1 --depth 8 --fps 1",
            "apply s --width 1 --height 1 --fps 1",
            "apply s --width 1 --height 1 --depth 8",
            "apply --width 1 --height 1 --depth 8 --fps 1",
        ] {
            usage(s);
        }
    }

    #[test]
    fn bad_numbers_are_usage() {
        for w in [
            "0",
            "-1",
            "+4",
            "4x",
            "",
            "4.5",
            "4294967296",
            "99999999999999999999",
        ] {
            let a = vec![
                "apply".to_string(),
                "s".into(),
                "--width".into(),
                w.into(),
                "--height".into(),
                "1".into(),
                "--depth".into(),
                "8".into(),
                "--fps".into(),
                "1".into(),
            ];
            assert_eq!(parse(&a).unwrap_err().code, Code::Usage, "{w:?}");
        }
        usage("apply s --width 1 --height 1 --depth 0 --fps 1");
        usage("apply s --width 1 --height 1 --depth x --fps 1");
        usage("apply s --width 1 --height 1 --depth 8 --fps 1 --frames -1");
        usage("apply s --width 1 --height 1 --depth 8 --fps 1 --frames x");
    }

    #[test]
    fn depth_is_not_judged_here() {
        assert_eq!(
            apply("apply s --width 1 --height 1 --depth 12 --fps 1").depth,
            12
        );
    }

    #[test]
    fn unknown_and_repeated_options_are_usage() {
        usage("apply s --width 1 --height 1 --depth 8 --fps 1 --bogus 1");
        usage("apply s --width 1 --width 2 --height 1 --depth 8 --fps 1");
        usage("apply s --width 1 --height 1 --depth 8 --depth 8 --fps 1");
        usage("apply s --width 1 --height 1 --depth 8 --fps 1 --describe --describe");
        usage("apply s --width 1 --height 1 --depth 8 --fps 1 --describe=1");
        usage("apply s t --width 1 --height 1 --depth 8 --fps 1");
        usage("apply s --width");
        usage("apply s --width 1 --height 1 --depth 8 --fps 1 --param novalue");
        usage("apply s --width 1 --height 1 --depth 8 --fps 1 --param =x");
        usage("apply s --width 1 --height 1 --depth 8 --fps 1 --layer x");
    }

    #[test]
    fn other_commands() {
        assert_eq!(parse(&v("list")).unwrap(), Command::List { json: false });
        assert_eq!(
            parse(&v("list --json")).unwrap(),
            Command::List { json: true }
        );
        assert_eq!(parse(&v("--help")).unwrap(), Command::Help);
        assert_eq!(parse(&v("-h")).unwrap(), Command::Help);
        assert_eq!(parse(&v("--version")).unwrap(), Command::Version);
        usage("");
        usage("bogus");
        usage("list --json --json");
        usage("list --all");
        usage("--help list");
        usage("--version x");
    }
}
