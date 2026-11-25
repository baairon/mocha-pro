//! `mocha-stage`: a stage program for the handoff convention.
//!
//! It implements step 1 of joining the convention: a stage manifest (`list --json`) and
//! `apply STAGE ...` over raw frames on standard input and output with the stage exits. The
//! stages come from a registry. `selftest-copy` is a self test that copies frames unchanged and
//! reimplements nothing. Every other stage is generic: it computes a published method under
//! explicit configuration and claims nothing about any plug-in.
//!
//! The modules: [`args`] (command line), [`registry`] (what a stage declares, the provider, the
//! manifest and the describe answer), [`params`] (typed `--param` and `--layer`), [`keys`] (the
//! key file for values that change per frame), [`stages`] (one module per stage), [`run`] (the
//! frame loops), [`pixels`] (frames as float samples), [`frame`] (the raw frame sizes),
//! [`stage`] (the provider constants and the documents of the provider) and [`error`]
//! (the error rows and their exits).
//!
//! A program can build its own [`registry::Provider`] and call [`run_cli_with`].

pub mod args;
pub mod error;
pub mod frame;
pub mod keys;
pub mod params;
pub mod pixels;
pub mod registry;
pub mod run;
pub mod stage;
pub mod stages;

use std::io::{Read, Write};

use args::Command;
use error::StageError;
use registry::Provider;

/// The streams of a run and what is known about them.
pub struct Io<'a> {
    /// Standard input.
    pub input: &'a mut dyn Read,
    /// Standard output.
    pub output: &'a mut dyn Write,
    /// Standard input is a terminal: `apply` refuses to read frames from it.
    pub input_is_terminal: bool,
    /// Standard output is a terminal: `apply` refuses to write frames to it.
    pub output_is_terminal: bool,
}

fn emit(io: &mut Io<'_>, text: &str) -> Result<(), StageError> {
    io.output
        .write_all(text.as_bytes())
        .and_then(|()| io.output.flush())
        .map_err(|e| {
            let code = if e.kind() == std::io::ErrorKind::BrokenPipe {
                error::Code::BrokenPipe
            } else {
                error::Code::StageFailed
            };
            StageError::new(code, format!("cannot write the output: {e}"))
        })
}

/// Run one command line (the arguments after the program name) for the provider.
pub fn execute(args: &[String], io: &mut Io<'_>) -> Result<(), StageError> {
    execute_with(&stages::provider(), args, io)
}

/// Run one command line for `provider`.
pub fn execute_with(
    provider: &Provider,
    args: &[String],
    io: &mut Io<'_>,
) -> Result<(), StageError> {
    match args::parse(args)? {
        Command::Help => emit(io, &args::HELP.replace(stage::PROGRAM, provider.program)),
        Command::Version => emit(io, &format!("{}\n", provider.version)),
        Command::List { json: true } => emit(io, &format!("{}\n", provider.manifest_json(None))),
        Command::List { json: false } => emit(io, &provider.list_text()),
        Command::Provider { program } => emit(
            io,
            &format!("{}\n", provider.manifest_json(program.as_deref())),
        ),
        Command::Apply(request) => {
            let (stage, plan) = provider.plan(&request)?;
            if request.describe {
                return emit(io, &format!("{}\n", registry::describe_json(&plan)));
            }
            if io.input_is_terminal {
                return Err(StageError::usage(
                    "refusing to read raw frames from a terminal: pipe them in",
                ));
            }
            if io.output_is_terminal {
                return Err(StageError::usage(
                    "refusing to write raw frames to a terminal: pipe them out",
                ));
            }
            stage.run(&plan, io.input, io.output).map(|_| ())
        }
    }
}

/// Run one command line for the provider, print an error as `error: <code>: <message>`
/// on `err` and return the process exit status.
pub fn run_cli(args: &[String], io: &mut Io<'_>, err: &mut dyn Write) -> i32 {
    run_cli_with(&stages::provider(), args, io, err)
}

/// [`run_cli`] for `provider`.
pub fn run_cli_with(
    provider: &Provider,
    args: &[String],
    io: &mut Io<'_>,
    err: &mut dyn Write,
) -> i32 {
    match execute_with(provider, args, io) {
        Ok(()) => 0,
        Err(e) => {
            // a failing standard error cannot be reported anywhere else
            let _ = writeln!(err, "{e}");
            let _ = err.flush();
            e.exit()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &str, input: &[u8], tty_in: bool, tty_out: bool) -> (i32, Vec<u8>, String) {
        let args: Vec<String> = args.split_whitespace().map(String::from).collect();
        let mut inp = input;
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = {
            let mut io = Io {
                input: &mut inp,
                output: &mut out,
                input_is_terminal: tty_in,
                output_is_terminal: tty_out,
            };
            run_cli(&args, &mut io, &mut err)
        };
        (code, out, String::from_utf8(err).unwrap())
    }

    const COPY8: &str = "apply selftest-copy --width 2 --height 1 --depth 8 --fps 24/1";

    #[test]
    fn copy_run_is_exit_zero_and_identity() {
        let data: Vec<u8> = (0..24).collect();
        let (code, out, err) = run(COPY8, &data, false, false);
        assert_eq!((code, out, err), (0, data, String::new()));
    }

    #[test]
    fn terminals_are_refused_for_frames() {
        let (code, out, err) = run(COPY8, b"", true, false);
        assert_eq!((code, out.len()), (2, 0));
        assert!(err.starts_with("error: usage: refusing to read"), "{err}");
        let (code, _, err) = run(COPY8, b"", false, true);
        assert_eq!(code, 2);
        assert!(err.starts_with("error: usage: refusing to write"), "{err}");
    }

    #[test]
    fn terminals_do_not_matter_for_describe_and_list() {
        let (code, out, _) = run(
            "apply selftest-copy --describe --width 2 --height 1 --depth 8 --fps 24",
            b"",
            true,
            true,
        );
        assert_eq!(code, 0);
        assert!(String::from_utf8(out).unwrap().starts_with("{\"ok\":true"));
        assert_eq!(run("list --json", b"", true, true).0, 0);
    }

    #[test]
    fn describe_reads_nothing() {
        struct Boom;
        impl Read for Boom {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                panic!("describe must not read the input");
            }
        }
        let args: Vec<String> =
            "apply selftest-copy --describe --width 2 --height 1 --depth 32 --fps 30000/1001 --frames 4"
                .split_whitespace()
                .map(String::from)
                .collect();
        let mut out = Vec::new();
        let mut boom = Boom;
        let mut io = Io {
            input: &mut boom,
            output: &mut out,
            input_is_terminal: false,
            output_is_terminal: false,
        };
        execute(&args, &mut io).unwrap();
        assert!(String::from_utf8(out)
            .unwrap()
            .contains("\"fps\":\"30000/1001\",\"frames\":4}"));
    }

    #[test]
    fn errors_use_the_text_form_on_the_error_stream() {
        let (code, out, err) = run(
            "apply nope --width 1 --height 1 --depth 8 --fps 1",
            b"",
            false,
            false,
        );
        assert_eq!((code, out.len()), (3, 0));
        assert!(
            err.starts_with("error: unsupported: unknown stage `nope`"),
            "{err}"
        );
        assert_eq!(err.matches('\n').count(), 1);
    }

    #[test]
    fn exit_codes_by_failure() {
        assert_eq!(run("", b"", false, false).0, 2);
        assert_eq!(
            run(
                "apply selftest-copy --width 1 --height 1 --depth 12 --fps 1",
                b"",
                false,
                false
            )
            .0,
            3
        );
        assert_eq!(
            run(
                "apply selftest-copy --width 1 --height 1 --depth 8 --fps 1",
                b"abc",
                false,
                false
            )
            .0,
            4
        );
    }

    #[test]
    fn help_and_version_print_on_the_output() {
        let (code, out, _) = run("--help", b"", false, false);
        assert_eq!(code, 0);
        assert!(String::from_utf8(out)
            .unwrap()
            .starts_with("usage: mocha-stage"));
        let (_, out, _) = run("--version", b"", false, false);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            format!("mocha-stage {}\n", env!("CARGO_PKG_VERSION"))
        );
    }
}
