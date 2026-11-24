//! The two tools as functions over explicit streams, so that tests run them in process and the
//! binaries stay thin.
//!
//! Exit codes follow the exit table of the handoff convention (frames/2): 0 success, 2 `usage`,
//! 3 `unsupported`, 4 `bad_input`, `partial_frame` and `broken_pipe`. `frame-cmp` adds one choice:
//! exit 1 means the two streams differ. The table's 1 is `stage_failed`; the tools never fail in
//! that way (a failure to write standard output is 4, as `broken_pipe`), so 1 is free, and it is
//! the exit that `diff` and `cmp` give for a difference. A failure prints one line
//! `error: CODE: MESSAGE` on standard error; a difference prints its report on standard output and
//! leaves standard error empty.

use crate::compare::{compare, Comparison, FrameVerdict};
use crate::error::{FrameError, GeometryError};
use crate::geometry::{Depth, Geometry, Rate};
use crate::io::FrameWriter;
use crate::pattern::Pattern;
use crate::sample::CHANNEL_NAMES;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{BufReader, ErrorKind, Read, Write};

/// Exit: success, or the streams are equal.
pub const EXIT_OK: i32 = 0;
/// Exit of `frame-cmp`: the streams differ.
pub const EXIT_DIFFER: i32 = 1;
/// Exit: a bad flag or flag combination.
pub const EXIT_USAGE: i32 = 2;
/// Exit: a valid request that is refused (an unsupported depth, a frame over 4 GiB).
pub const EXIT_UNSUPPORTED: i32 = 3;
/// Exit: unreadable input, a partial frame, a frame count mismatch, a closed output.
pub const EXIT_BAD_INPUT: i32 = 4;

/// Usage text of `frame-gen`.
pub const GEN_USAGE: &str = "usage: frame-gen --width W --height H --depth 8|16|32 --frames N [--fps N/D] [--pattern SPEC]\n\
patterns: constant:R,G,B,A | hramp | vramp | checker:CELL | random:SEED | aramp (default hramp)\n";

/// Usage text of `frame-cmp`.
pub const CMP_USAGE: &str =
    "usage: frame-cmp --width W --height H --depth 8|16|32 [--fps N/D] [--frames N] [-v] A B\n\
A and B are files; one of them may be - for standard input\n";

struct CliError {
    exit: i32,
    code: &'static str,
    message: String,
}

fn usage(message: impl Into<String>) -> CliError {
    CliError {
        exit: EXIT_USAGE,
        code: "usage",
        message: message.into(),
    }
}

fn bad_input(message: impl Into<String>) -> CliError {
    CliError {
        exit: EXIT_BAD_INPUT,
        code: "bad_input",
        message: message.into(),
    }
}

impl From<GeometryError> for CliError {
    fn from(e: GeometryError) -> CliError {
        let (exit, code) = match e {
            GeometryError::UnsupportedDepth(_) | GeometryError::FrameTooLarge(_) => {
                (EXIT_UNSUPPORTED, "unsupported")
            }
            _ => (EXIT_USAGE, "usage"),
        };
        CliError {
            exit,
            code,
            message: e.to_string(),
        }
    }
}

/// Splits arguments into flags and positionals. `valued` are flags that take a value (as
/// `--flag value` or `--flag=value`), `switches` take none. A repeated flag is a usage error.
struct Parsed {
    flags: BTreeMap<String, String>,
    positional: Vec<String>,
}

fn parse(args: &[String], valued: &[&str], switches: &[&str]) -> Result<Parsed, CliError> {
    let mut flags = BTreeMap::new();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if arg == "-" || !arg.starts_with('-') {
            positional.push(arg.clone());
            continue;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let value = if valued.contains(&name.as_str()) {
            match inline {
                Some(v) => v,
                None => {
                    let v = args
                        .get(i)
                        .ok_or_else(|| usage(format!("{name} needs a value")))?;
                    i += 1;
                    v.clone()
                }
            }
        } else if switches.contains(&name.as_str()) {
            if inline.is_some() {
                return Err(usage(format!("{name} takes no value")));
            }
            String::new()
        } else {
            return Err(usage(format!("unknown flag {name}")));
        };
        if flags.insert(name.clone(), value).is_some() {
            return Err(usage(format!("{name} given twice")));
        }
    }
    Ok(Parsed { flags, positional })
}

fn number(p: &Parsed, name: &str) -> Result<Option<u64>, CliError> {
    match p.flags.get(name) {
        None => Ok(None),
        Some(v) if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) => v
            .parse::<u64>()
            .map(Some)
            .map_err(|_| usage(format!("{name} {v:?} is too large"))),
        Some(v) => Err(usage(format!("{name} {v:?} is not a whole number"))),
    }
}

fn required(p: &Parsed, name: &str) -> Result<u64, CliError> {
    number(p, name)?.ok_or_else(|| usage(format!("{name} is required")))
}

fn small(v: u64, name: &str) -> Result<u32, CliError> {
    u32::try_from(v).map_err(|_| usage(format!("{name} {v} is too large")))
}

fn geometry_of(p: &Parsed, with_frames: bool) -> Result<Geometry, CliError> {
    let width = small(required(p, "--width")?, "--width")?;
    let height = small(required(p, "--height")?, "--height")?;
    let depth = Depth::from_bits(small(required(p, "--depth")?, "--depth")?)?;
    let fps = match p.flags.get("--fps") {
        Some(s) => s.parse::<Rate>()?,
        None => Rate { num: 30, den: 1 },
    };
    let frames = if with_frames {
        number(p, "--frames")?
    } else {
        None
    };
    Ok(Geometry::new(width, height, depth, fps, frames)?)
}

fn fail(err: &mut dyn Write, e: &CliError) -> i32 {
    let _ = writeln!(err, "error: {}: {}", e.code, e.message);
    e.exit
}

/// `frame-gen`: writes `--frames` frames of a pattern to `out`. Returns the exit code.
pub fn run_gen(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        let _ = out.write_all(GEN_USAGE.as_bytes());
        let _ = out.flush();
        return EXIT_OK;
    }
    match gen(args, out) {
        Ok(()) => EXIT_OK,
        Err(e) => fail(err, &e),
    }
}

fn gen(args: &[String], out: &mut dyn Write) -> Result<(), CliError> {
    let p = parse(
        args,
        &[
            "--width",
            "--height",
            "--depth",
            "--fps",
            "--frames",
            "--pattern",
        ],
        &[],
    )?;
    if let Some(extra) = p.positional.first() {
        return Err(usage(format!("unexpected argument {extra:?}")));
    }
    let frames = required(&p, "--frames")?;
    let geometry = geometry_of(&p, false)?;
    let pattern = match p.flags.get("--pattern") {
        Some(s) => s.parse::<Pattern>().map_err(|e| usage(e.to_string()))?,
        None => Pattern::HRamp,
    };
    let mut writer = FrameWriter::new(out, geometry);
    let mut buf = Vec::new();
    for index in 0..frames {
        pattern.fill_frame(&geometry, index, &mut buf);
        writer.write_frame(&buf).map_err(output_error)?;
    }
    writer.flush().map_err(output_error)?;
    Ok(())
}

fn output_error(e: FrameError) -> CliError {
    let code = match &e {
        FrameError::Io(io) if io.kind() == ErrorKind::BrokenPipe => "broken_pipe",
        _ => "bad_input",
    };
    CliError {
        exit: EXIT_BAD_INPUT,
        code,
        message: format!("cannot write standard output: {e}"),
    }
}

/// Formats a raw sample for a report: decimal at depth 8 and 16, `FLOAT:BITS` (the float in Rust
/// debug form and the bit pattern in eight hex digits) at depth 32.
pub fn format_value(depth: Depth, bits: u32) -> String {
    match depth {
        Depth::ThirtyTwo => format!("{:?}:{:08x}", f32::from_bits(bits), bits),
        _ => bits.to_string(),
    }
}

/// The report of a comparison, as printed on standard output (verbose adds the per frame lines
/// separately).
pub fn format_report(g: &Geometry, c: &Comparison) -> String {
    if c.is_equal() {
        return format!(
            "equal frames={} width={} height={} depth={}\n",
            c.frames_a,
            g.width,
            g.height,
            g.depth.bits()
        );
    }
    let mut s = format!(
        "differ frames_a={} frames_b={} compared={} equal={} different={} samples={}\n",
        c.frames_a,
        c.frames_b,
        c.frames_compared,
        c.frames_equal,
        c.frames_different,
        c.samples_different
    );
    match &c.first {
        Some(d) => {
            let _ = writeln!(
                s,
                "first frame={} x={} y={} channel={} a={} b={}",
                d.frame,
                d.x,
                d.y,
                CHANNEL_NAMES[d.channel],
                format_value(g.depth, d.a),
                format_value(g.depth, d.b)
            );
        }
        None => s.push_str("first none\n"),
    }
    s
}

fn verdict_line(v: &FrameVerdict) -> String {
    if v.equal {
        format!("frame {} equal\n", v.frame)
    } else {
        format!("frame {} differ samples={}\n", v.frame, v.samples_different)
    }
}

/// `frame-cmp`: compares two streams. `stdin` serves the argument `-`. Returns the exit code.
pub fn run_cmp(
    args: &[String],
    stdin: &mut dyn Read,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        let _ = out.write_all(CMP_USAGE.as_bytes());
        return EXIT_OK;
    }
    match cmp(args, stdin) {
        Ok((text, equal)) => {
            if let Err(e) = out.write_all(text.as_bytes()).and_then(|_| out.flush()) {
                let code = if e.kind() == ErrorKind::BrokenPipe {
                    "broken_pipe"
                } else {
                    "bad_input"
                };
                let _ = writeln!(err, "error: {code}: cannot write standard output: {e}");
                return EXIT_BAD_INPUT;
            }
            if equal {
                EXIT_OK
            } else {
                EXIT_DIFFER
            }
        }
        Err(e) => fail(err, &e),
    }
}

fn open_file(path: &str) -> Result<Box<dyn Read>, CliError> {
    match std::fs::File::open(path) {
        Ok(f) => Ok(Box::new(BufReader::new(f))),
        Err(e) => Err(bad_input(format!("cannot read {path}: {e}"))),
    }
}

fn cmp(args: &[String], stdin: &mut dyn Read) -> Result<(String, bool), CliError> {
    let p = parse(
        args,
        &["--width", "--height", "--depth", "--fps", "--frames"],
        &["-v"],
    )?;
    if p.positional.len() != 2 {
        return Err(usage("two streams A and B are required"));
    }
    let geometry = geometry_of(&p, true)?;
    let verbose = p.flags.contains_key("-v");
    let (pa, pb) = (&p.positional[0], &p.positional[1]);
    if pa == "-" && pb == "-" {
        return Err(usage("only one stream may be standard input"));
    }
    let (a, b): (Box<dyn Read + '_>, Box<dyn Read + '_>) = if pa == "-" {
        (Box::new(stdin), open_file(pb)?)
    } else if pb == "-" {
        (open_file(pa)?, Box::new(stdin))
    } else {
        (open_file(pa)?, open_file(pb)?)
    };
    let mut lines = String::new();
    let result = compare(&geometry, a, b, |v| {
        if verbose {
            lines.push_str(&verdict_line(v));
        }
    });
    match result {
        Ok(c) => {
            let mut text = lines;
            text.push_str(&format_report(&geometry, &c));
            Ok((text, c.is_equal()))
        }
        Err(e) => {
            let code = match &e.error {
                FrameError::PartialFrame { .. } => "partial_frame",
                _ => "bad_input",
            };
            Err(CliError {
                exit: EXIT_BAD_INPUT,
                code,
                message: e.to_string(),
            })
        }
    }
}
