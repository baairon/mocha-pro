//! Shared helpers. All data is invented and made at run time.
#![allow(dead_code)]

use frame_io::{Depth, Geometry, Pattern, Rate};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

pub fn geom(w: u32, h: u32, depth: Depth) -> Geometry {
    Geometry::new(w, h, depth, Rate { num: 24, den: 1 }, None).expect("geometry")
}

pub fn geom_n(w: u32, h: u32, depth: Depth, frames: u64) -> Geometry {
    Geometry::new(w, h, depth, Rate { num: 24, den: 1 }, Some(frames)).expect("geometry")
}

/// `frames` frames of a pattern as one byte vector.
pub fn stream(g: &Geometry, p: &Pattern, frames: u64) -> Vec<u8> {
    let mut all = Vec::new();
    let mut buf = Vec::new();
    for i in 0..frames {
        p.fill_frame(g, i, &mut buf);
        all.extend_from_slice(&buf);
    }
    all
}

pub fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "frame-io-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create folder");
    dir
}

pub fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(|t| t.to_string()).collect()
}

/// Runs `frame-gen` in process: (exit, stdout bytes, stderr text).
pub fn gen(a: &str) -> (i32, Vec<u8>, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = frame_io::cli::run_gen(&args(a), &mut out, &mut err);
    (code, out, String::from_utf8(err).expect("utf-8"))
}

/// Runs `frame-cmp` in process with the given stdin: (exit, stdout text, stderr text).
pub fn cmp(a: &str, stdin: &[u8]) -> (i32, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut input = stdin;
    let code = frame_io::cli::run_cmp(&args(a), &mut input, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).expect("utf-8"),
        String::from_utf8(err).expect("utf-8"),
    )
}
