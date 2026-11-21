//! `mask-validate SEQUENCE`: check one mask sequence, a `mask-1` manifest and the frame files it
//! lists. Exit 0 and one `ok` line on standard output; exit 1 and one `error:` line on standard
//! error for an invalid sequence; exit 2 for a usage error or an argument that cannot be read.

use mask_seq::{validate, Outcome};
use std::io::Write;
use std::path::Path;

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: mask-validate <folder|manifest.txt>");
        std::process::exit(2);
    }
    match validate(Path::new(&args[0])) {
        Outcome::Valid(s) => {
            let mut so = std::io::stdout().lock();
            if writeln!(so, "{}", s.line()).is_err() {
                std::process::exit(1);
            }
        }
        Outcome::Invalid(i) => {
            eprintln!("{}", i.line());
            std::process::exit(1);
        }
        Outcome::Unreadable(e) => {
            eprintln!("read failed: {e}");
            std::process::exit(2);
        }
    }
}
