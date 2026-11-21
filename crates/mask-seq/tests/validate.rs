//! The library validator on folders made at run time. All values are invented.

mod common;

use common::{base_frame, build_scenario, fresh_dir, write_base};
use mask_seq::{validate, write_sequence, Depth, Frame, Outcome, Reason, SequenceError};

fn invalid(o: Outcome) -> (Reason, String) {
    match o {
        Outcome::Invalid(i) => (i.reason, i.detail),
        other => panic!("expected an invalid sequence, got {other:?}"),
    }
}

fn check(dir: &std::path::Path, reason: Reason, detail: &str) {
    assert_eq!(invalid(validate(dir)), (reason, detail.to_string()));
}

#[test]
fn each_reason_word() {
    let rows: [(&str, Reason, &str); 12] = [
        ("manifest_missing", Reason::ManifestMissing, "manifest.txt"),
        ("manifest_syntax", Reason::ManifestSyntax, "line 4"),
        ("manifest_format", Reason::ManifestFormat, "line 1"),
        ("manifest_value", Reason::ManifestValue, "line 6"),
        ("frame_missing", Reason::FrameMissing, "frame_000001.pgm"),
        ("frame_header", Reason::FrameHeader, "frame_000000.pgm"),
        ("frame_size", Reason::FrameSize, "frame_000002.pgm"),
        ("frame_depth", Reason::FrameDepth, "frame_000001.pgm"),
        (
            "frame_truncated",
            Reason::FrameTruncated,
            "frame_000002.pgm",
        ),
        ("frame_trailing", Reason::FrameTrailing, "frame_000000.pgm"),
        ("frame_hash", Reason::FrameHash, "frame_000001.pgm"),
        ("frame_extra", Reason::FrameExtra, "frame_000003.pgm"),
    ];
    for (name, reason, detail) in rows {
        let dir = fresh_dir(name);
        let args = build_scenario(name, &dir);
        assert_eq!(args.len(), 1);
        assert_eq!(
            invalid(validate(&args[0])),
            (reason, detail.to_string()),
            "{name}"
        );
        assert_eq!(reason.as_str(), name);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn valid_folder_and_manifest_path() {
    let dir = fresh_dir("valid");
    write_base(&dir);
    let line = "ok format=mask-1 frames=3 first=0 width=4 height=2 depth=8";
    let a = match validate(&dir) {
        Outcome::Valid(s) => s,
        other => panic!("{other:?}"),
    };
    let b = match validate(&dir.join("manifest.txt")) {
        Outcome::Valid(s) => s,
        other => panic!("{other:?}"),
    };
    assert_eq!(a, b);
    assert_eq!(a.line(), line);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_path_is_unreadable() {
    let dir = fresh_dir("unreadable");
    match validate(&dir.join("absent")) {
        Outcome::Unreadable(_) => {}
        other => panic!("{other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn first_fault_wins() {
    // A missing frame 0 comes before a changed frame 1.
    let dir = fresh_dir("order_missing");
    write_base(&dir);
    std::fs::remove_file(dir.join("frame_000000.pgm")).unwrap();
    let mut b = std::fs::read(dir.join("frame_000001.pgm")).unwrap();
    *b.last_mut().unwrap() ^= 1;
    std::fs::write(dir.join("frame_000001.pgm"), b).unwrap();
    check(&dir, Reason::FrameMissing, "frame_000000.pgm");
    let _ = std::fs::remove_dir_all(&dir);

    // A size fault and a hash fault in one frame give the size fault.
    let dir = fresh_dir("order_size");
    write_base(&dir);
    let f = Frame::new(2, 4, Depth::Eight, vec![1; 8]).unwrap();
    std::fs::write(dir.join("frame_000001.pgm"), f.to_pgm()).unwrap();
    check(&dir, Reason::FrameSize, "frame_000001.pgm");
    let _ = std::fs::remove_dir_all(&dir);

    // A frame fault and an extra name give the frame fault.
    let dir = fresh_dir("order_extra");
    write_base(&dir);
    std::fs::copy(dir.join("frame_000000.pgm"), dir.join("frame_000005.pgm")).unwrap();
    let mut b = std::fs::read(dir.join("frame_000002.pgm")).unwrap();
    *b.last_mut().unwrap() ^= 1;
    std::fs::write(dir.join("frame_000002.pgm"), b).unwrap();
    check(&dir, Reason::FrameHash, "frame_000002.pgm");
    let _ = std::fs::remove_dir_all(&dir);

    // A manifest fault and a frame fault give the manifest fault.
    let dir = fresh_dir("order_manifest");
    write_base(&dir);
    std::fs::remove_file(dir.join("frame_000000.pgm")).unwrap();
    let p = dir.join("manifest.txt");
    let t = std::fs::read_to_string(&p)
        .unwrap()
        .replace("channels=1", "channels=2");
    std::fs::write(&p, t).unwrap();
    check(&dir, Reason::ManifestValue, "line 7");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn extra_frames_sorted_other_files_ignored() {
    let dir = fresh_dir("extras");
    write_base(&dir);
    std::fs::write(dir.join("notes.txt"), b"x").unwrap();
    std::fs::write(dir.join("frame_000009.PGM"), b"x").unwrap();
    std::fs::write(dir.join("frame.pgm"), b"x").unwrap();
    std::fs::create_dir(dir.join("sub")).unwrap();
    match validate(&dir) {
        Outcome::Valid(_) => {}
        other => panic!("{other:?}"),
    }
    std::fs::write(dir.join("frame_000007.pgm"), b"x").unwrap();
    std::fs::write(dir.join("frame_000004.pgm"), b"x").unwrap();
    check(&dir, Reason::FrameExtra, "frame_000004.pgm");
    let _ = std::fs::remove_dir_all(&dir);

    let dir = fresh_dir("extra_dot");
    write_base(&dir);
    std::fs::write(dir.join("frame_.pgm"), b"x").unwrap();
    check(&dir, Reason::FrameExtra, "frame_.pgm");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nonzero_first_and_16_bit() {
    let dir = fresh_dir("sixteen");
    let frames: Vec<Frame> = (0..3u16)
        .map(|k| Frame::new(3, 1, Depth::Sixteen, vec![k * 1000, 65535, k]).unwrap())
        .collect();
    write_sequence(&dir, 999_997, &frames).unwrap();
    match validate(&dir) {
        Outcome::Valid(s) => assert_eq!(
            s.line(),
            "ok format=mask-1 frames=3 first=999997 width=3 height=1 depth=16"
        ),
        other => panic!("{other:?}"),
    }
    for name in ["frame_999997.pgm", "frame_999998.pgm", "frame_999999.pgm"] {
        assert!(dir.join(name).is_file(), "{name}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_sequence_refusals() {
    let dir = fresh_dir("refusals");
    match write_sequence(&dir, 0, &[]) {
        Err(SequenceError::Empty) => {}
        other => panic!("{other:?}"),
    }
    let other_size = Frame::new(2, 2, Depth::Eight, vec![0; 4]).unwrap();
    match write_sequence(&dir, 0, &[base_frame(0), other_size]) {
        Err(SequenceError::Unequal { index: 1 }) => {}
        other => panic!("{other:?}"),
    }
    let other_depth = Frame::new(4, 2, Depth::Sixteen, vec![0; 8]).unwrap();
    match write_sequence(&dir, 0, &[base_frame(0), other_depth]) {
        Err(SequenceError::Unequal { index: 1 }) => {}
        other => panic!("{other:?}"),
    }
    match write_sequence(&dir, 999_999, &[base_frame(0), base_frame(1)]) {
        Err(SequenceError::Range) => {}
        other => panic!("{other:?}"),
    }
    assert!(!dir.join("manifest.txt").exists());
    let m = write_sequence(&dir, 0, &[base_frame(0), base_frame(1)]).unwrap();
    assert_eq!(
        std::fs::read(dir.join("manifest.txt")).unwrap(),
        m.to_bytes()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Runs the binary on `dir` and checks exit code 1, empty stdout and exactly `want` on stderr,
/// beside the library's own answer.
fn assert_error_line(dir: &std::path::Path, reason: Reason, detail: &str) {
    check(dir, reason, detail);
    let want = format!("error: {}: {}", reason.as_str(), detail);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mask-validate"))
        .arg(dir)
        .output()
        .expect("run mask-validate");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8(out.stderr).unwrap(), format!("{want}\n"));
}

#[test]
fn size_fault_in_one_side_only() {
    // A frame that differs in width only, and one that differs in height only; the new files are
    // canonical, so only the size comparison can name them.
    for (name, w, h) in [("narrower", 3u32, 2u32), ("taller", 4, 3)] {
        let dir = fresh_dir(name);
        write_base(&dir);
        let samples = vec![5u16; (w * h) as usize];
        let f = Frame::new(w, h, Depth::Eight, samples).unwrap();
        std::fs::write(dir.join("frame_000001.pgm"), f.to_pgm()).unwrap();
        assert_error_line(&dir, Reason::FrameSize, "frame_000001.pgm");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn frame_named_files_outside_the_range_are_extra() {
    // Below the first index.
    let dir = fresh_dir("below_first");
    let frames = [base_frame(0), base_frame(1)];
    write_sequence(&dir, 1, &frames).unwrap();
    match validate(&dir) {
        Outcome::Valid(_) => {}
        other => panic!("{other:?}"),
    }
    std::fs::write(dir.join("frame_000000.pgm"), b"x").unwrap();
    assert_error_line(&dir, Reason::FrameExtra, "frame_000000.pgm");
    let _ = std::fs::remove_dir_all(&dir);

    // Seven digits for an index of the sequence is not the name of that frame.
    let dir = fresh_dir("seven_digits");
    write_base(&dir);
    std::fs::write(dir.join("frame_0000001.pgm"), b"x").unwrap();
    assert_error_line(&dir, Reason::FrameExtra, "frame_0000001.pgm");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn folder_in_place_of_a_frame_is_missing() {
    let dir = fresh_dir("folder_frame");
    write_base(&dir);
    std::fs::remove_file(dir.join("frame_000001.pgm")).unwrap();
    std::fs::create_dir(dir.join("frame_000001.pgm")).unwrap();
    assert_error_line(&dir, Reason::FrameMissing, "frame_000001.pgm");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn size_is_checked_before_depth() {
    // A frame that is wrong in size and in depth is reported as a size fault.
    let dir = fresh_dir("size_before_depth");
    write_base(&dir);
    let f = Frame::new(2, 4, Depth::Sixteen, vec![3; 8]).unwrap();
    std::fs::write(dir.join("frame_000001.pgm"), f.to_pgm()).unwrap();
    assert_error_line(&dir, Reason::FrameSize, "frame_000001.pgm");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bare_manifest_name_from_inside_the_folder() {
    // The argument `manifest.txt` has an empty parent, which stands for the current folder.
    let dir = fresh_dir("bare_name");
    write_base(&dir);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mask-validate"))
        .current_dir(&dir)
        .arg("manifest.txt")
        .output()
        .expect("run mask-validate");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "ok format=mask-1 frames=3 first=0 width=4 height=2 depth=8\n"
    );
    assert!(out.stderr.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn names_without_the_exact_suffix_are_ignored() {
    let dir = fresh_dir("suffix");
    write_base(&dir);
    std::fs::write(dir.join("frame_000009xpgm"), b"x").unwrap();
    std::fs::write(dir.join("frame_000010.pg"), b"x").unwrap();
    match validate(&dir) {
        Outcome::Valid(_) => {}
        other => panic!("{other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn name_that_is_not_utf8_is_skipped() {
    // A file name that is not valid UTF-8 can be made on Linux; other systems refuse to create
    // one, and the test then stops with a note.
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let dir = fresh_dir("non_utf8_name");
        write_base(&dir);
        let name = std::ffi::OsStr::from_bytes(b"frame_\xff\xfe.pgm");
        if std::fs::write(dir.join(name), b"x").is_err() {
            eprintln!("skipped: this file system refuses a name that is not UTF-8");
        } else {
            match validate(&dir) {
                Outcome::Valid(_) => {}
                other => panic!("{other:?}"),
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[cfg(not(unix))]
    eprintln!("skipped: no way to name a file with bytes that are not UTF-8 here");
}

#[test]
fn reason_all_order() {
    let words: Vec<&str> = Reason::ALL.iter().map(|r| r.as_str()).collect();
    assert_eq!(
        words,
        [
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
            "frame_read"
        ]
    );
}
