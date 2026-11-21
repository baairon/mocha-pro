//! `shape-predict <case.txt>`: print the predicted dump bytes of a case to stdout.

use std::io::Write;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: shape-predict <case.txt>");
        std::process::exit(2);
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("read failed: {e}");
            std::process::exit(2);
        }
    };
    let out = shape_canon::Case::parse(&text)
        .map_err(|e| e.to_string())
        .and_then(|c| shape_canon::predict(&c).map_err(|e| e.to_string()));
    match out {
        Ok(bytes) => {
            let mut so = std::io::stdout().lock();
            if so.write_all(&bytes).is_err() {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
