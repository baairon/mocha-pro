//! The `mocha-stage` program: a thin shell over [`stage_cli`].
//!
//! Standard input and output are raw byte streams on every platform: the Rust standard library
//! does no newline translation on any of them. A closed reader surfaces as a write error, which
//! `stage_cli::run` reports as `broken_pipe` (exit 4); Rust ignores SIGPIPE in a binary.

use std::io::IsTerminal;

fn main() {
    let mut args = Vec::new();
    for a in std::env::args_os().skip(1) {
        match a.into_string() {
            Ok(s) => args.push(s),
            Err(_) => {
                eprintln!("error: usage: an argument is not valid UTF-8 text");
                std::process::exit(2);
            }
        }
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let input_is_terminal = stdin.is_terminal();
    let output_is_terminal = stdout.is_terminal();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let mut err = std::io::stderr();
    let code = {
        let mut io = stage_cli::Io {
            input: &mut input,
            output: &mut output,
            input_is_terminal,
            output_is_terminal,
        };
        stage_cli::run_cli(&args, &mut io, &mut err)
    };
    std::process::exit(code);
}
