//! Shared helpers: fresh folders, the base sequence and the scenario mutations. All data is
//! invented and made at run time.
#![allow(dead_code)]

use mask_seq::{write_sequence, Depth, Frame};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A fresh, empty folder named with the process id, a counter and the test name.
pub fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "mask-seq-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create folder");
    dir
}

/// Frame `k` of the base sequence: 4 by 2, depth 8, sample at column x and row y is
/// `(k * 8 + y * 4 + x) * 9`.
pub fn base_frame(k: u32) -> Frame {
    let samples = (0..8u32).map(|i| ((k * 8 + i) * 9) as u16).collect();
    Frame::new(4, 2, Depth::Eight, samples).expect("base frame")
}

/// Writes the base sequence (first 0, three frames) into `dir`.
pub fn write_base(dir: &Path) {
    let frames: Vec<Frame> = (0..3).map(base_frame).collect();
    write_sequence(dir, 0, &frames).expect("write base sequence");
}

fn replace_line(dir: &Path, from: &str, to: &str) {
    let p = dir.join("manifest.txt");
    let text = std::fs::read_to_string(&p).expect("read manifest");
    assert!(text.contains(&format!("{from}\n")), "line {from:?} present");
    std::fs::write(
        &p,
        text.replacen(&format!("{from}\n"), &format!("{to}\n"), 1),
    )
    .expect("write manifest");
}

/// The names of the 17 scenarios of the interface file.
pub const SCENARIOS: [&str; 17] = [
    "valid_folder",
    "valid_manifest_path",
    "manifest_missing",
    "manifest_syntax",
    "manifest_format",
    "manifest_value",
    "frame_missing",
    "frame_header",
    "frame_size",
    "frame_depth",
    "frame_truncated",
    "frame_trailing",
    "frame_hash",
    "frame_extra",
    "no_argument",
    "two_arguments",
    "missing_path",
];

/// Builds the scenario in `dir` (a fresh folder) and returns the arguments to give the validator.
pub fn build_scenario(name: &str, dir: &Path) -> Vec<PathBuf> {
    write_base(dir);
    let f0 = dir.join("frame_000000.pgm");
    let f1 = dir.join("frame_000001.pgm");
    let f2 = dir.join("frame_000002.pgm");
    match name {
        "valid_folder" => {}
        "valid_manifest_path" => return vec![dir.join("manifest.txt")],
        "manifest_missing" => std::fs::remove_file(dir.join("manifest.txt")).unwrap(),
        "manifest_syntax" => replace_line(dir, "width=4", "width 4"),
        "manifest_format" => replace_line(dir, "format=mask-1", "format=mask-2"),
        "manifest_value" => replace_line(dir, "depth=8", "depth=12"),
        "frame_missing" => std::fs::remove_file(&f1).unwrap(),
        "frame_header" => {
            let raster = base_frame(0).to_pgm();
            let mut b = b"P5\n# c\n4 2\n255\n".to_vec();
            b.extend_from_slice(&raster[raster.len() - 8..]);
            std::fs::write(&f0, b).unwrap();
        }
        "frame_size" => {
            let f = Frame::new(2, 4, Depth::Eight, base_frame(2).samples().to_vec()).unwrap();
            std::fs::write(&f2, f.to_pgm()).unwrap();
        }
        "frame_depth" => {
            let f = Frame::new(4, 2, Depth::Sixteen, base_frame(1).samples().to_vec()).unwrap();
            std::fs::write(&f1, f.to_pgm()).unwrap();
        }
        "frame_truncated" => {
            let mut b = std::fs::read(&f2).unwrap();
            b.pop();
            std::fs::write(&f2, b).unwrap();
        }
        "frame_trailing" => {
            let mut b = std::fs::read(&f0).unwrap();
            b.push(0);
            std::fs::write(&f0, b).unwrap();
        }
        "frame_hash" => {
            let mut b = std::fs::read(&f1).unwrap();
            *b.last_mut().unwrap() ^= 1;
            std::fs::write(&f1, b).unwrap();
        }
        "frame_extra" => {
            std::fs::copy(&f0, dir.join("frame_000003.pgm")).unwrap();
        }
        "no_argument" => return vec![],
        "two_arguments" => return vec![dir.to_path_buf(), dir.to_path_buf()],
        "missing_path" => return vec![dir.join("absent")],
        other => panic!("unknown scenario {other:?}"),
    }
    vec![dir.to_path_buf()]
}
