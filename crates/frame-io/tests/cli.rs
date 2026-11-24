mod common;

use common::{cmp, fresh_dir, gen, geom, stream};
use frame_io::cli::{format_value, CMP_USAGE, GEN_USAGE};
use frame_io::{write_sample, Depth, Pattern};

fn write_pair(name: &str, a: &[u8], b: &[u8]) -> (String, String) {
    let dir = fresh_dir(name);
    let (pa, pb) = (dir.join("a.raw"), dir.join("b.raw"));
    std::fs::write(&pa, a).unwrap();
    std::fs::write(&pb, b).unwrap();
    (
        pa.to_str().unwrap().to_string(),
        pb.to_str().unwrap().to_string(),
    )
}

const G8: &str = "--width 2 --height 1 --depth 8";

#[test]
fn gen_writes_the_library_bytes() {
    let g = geom(3, 2, Depth::Sixteen);
    let (code, out, err) = gen("--width 3 --height 2 --depth 16 --frames 2 --pattern random:4");
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(out, stream(&g, &Pattern::Random(4), 2));
}

#[test]
fn gen_default_pattern_is_hramp_and_flags_accept_equals() {
    let g = geom(3, 1, Depth::Eight);
    let (code, out, _) = gen("--width=3 --height=1 --depth=8 --frames=1");
    assert_eq!(code, 0);
    assert_eq!(out, stream(&g, &Pattern::HRamp, 1));
}

#[test]
fn gen_zero_frames_writes_nothing_and_succeeds() {
    let (code, out, err) = gen("--width 2 --height 2 --depth 8 --frames 0");
    assert_eq!((code, out.len(), err.as_str()), (0, 0, ""));
}

#[test]
fn gen_accepts_an_exact_rate_and_refuses_a_decimal_one() {
    assert_eq!(
        gen("--width 1 --height 1 --depth 8 --frames 1 --fps 30000/1001").0,
        0
    );
    let (code, out, err) = gen("--width 1 --height 1 --depth 8 --frames 1 --fps 29.97");
    assert_eq!((code, out.len()), (2, 0));
    assert!(err.starts_with("error: usage: "), "{err}");
}

#[test]
fn gen_help_prints_usage_on_standard_output() {
    for flag in ["--help", "-h"] {
        let (code, out, err) = gen(flag);
        assert_eq!(code, 0);
        assert_eq!(String::from_utf8(out).unwrap(), GEN_USAGE);
        assert_eq!(err, "");
    }
}

#[test]
fn gen_usage_errors_exit_2_with_one_error_line() {
    for a in [
        "",
        "--width 2 --height 2 --depth 8",
        "--height 2 --depth 8 --frames 1",
        "--width 2 --depth 8 --frames 1",
        "--width 2 --height 2 --frames 1",
        "--width 0 --height 2 --depth 8 --frames 1",
        "--width 2 --height 0 --depth 8 --frames 1",
        "--width x --height 2 --depth 8 --frames 1",
        "--width -2 --height 2 --depth 8 --frames 1",
        "--width 99999999999 --height 2 --depth 8 --frames 1",
        "--width 2 --height 2 --depth abc --frames 1",
        "--width 2 --height 2 --depth 8 --frames 1 --pattern nope",
        "--width 2 --height 2 --depth 8 --frames 1 --bogus 3",
        "--width 2 --width 3 --height 2 --depth 8 --frames 1",
        "--width 2 --height 2 --depth 8 --frames 1 extra",
        "--width 2 --height 2 --depth 8 --frames",
    ] {
        let (code, out, err) = gen(a);
        assert_eq!(code, 2, "{a:?}");
        assert!(out.is_empty(), "{a:?}");
        assert!(
            err.starts_with("error: usage: ")
                && err.ends_with('\n')
                && err.matches('\n').count() == 1,
            "{a:?}: {err:?}"
        );
    }
}

#[test]
fn gen_refuses_valid_requests_it_cannot_serve_with_exit_3() {
    let (code, out, err) = gen("--width 2 --height 2 --depth 12 --frames 1");
    assert_eq!((code, out.len()), (3, 0));
    assert!(err.starts_with("error: unsupported: "), "{err}");
    let (code, out, err) = gen("--width 32768 --height 32769 --depth 8 --frames 1");
    assert_eq!((code, out.len()), (3, 0));
    assert!(err.starts_with("error: unsupported: "), "{err}");
}

struct Closed;

impl std::io::Write for Closed {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn gen_reports_a_closed_output_as_broken_pipe_exit_4() {
    let mut err = Vec::new();
    let a: Vec<String> = "--width 2 --height 2 --depth 8 --frames 3"
        .split(' ')
        .map(String::from)
        .collect();
    let code = frame_io::cli::run_gen(&a, &mut Closed, &mut err);
    assert_eq!(code, 4);
    assert!(String::from_utf8(err)
        .unwrap()
        .starts_with("error: broken_pipe: "));
}

#[test]
fn cmp_equal_files_exit_0() {
    let g = geom(2, 1, Depth::Eight);
    let s = stream(&g, &Pattern::HRamp, 2);
    let (a, b) = write_pair("equal", &s, &s);
    let (code, out, err) = cmp(&format!("{G8} {a} {b}"), &[]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, "equal frames=2 width=2 height=1 depth=8\n");
}

#[test]
fn cmp_difference_exits_1_and_reports_the_first_sample() {
    let g = geom(2, 1, Depth::Eight);
    let s = stream(&g, &Pattern::HRamp, 2);
    let mut t = s.clone();
    write_sample(&g, &mut t[8..], 1, 0, 1, 21);
    let (a, b) = write_pair("differ", &s, &t);
    let (code, out, err) = cmp(&format!("{G8} {a} {b}"), &[]);
    assert_eq!((code, err.as_str()), (1, ""));
    assert_eq!(
        out,
        "differ frames_a=2 frames_b=2 compared=2 equal=1 different=1 samples=1\nfirst frame=1 x=1 y=0 channel=G a=255 b=21\n"
    );
}

#[test]
fn cmp_verbose_lists_every_common_frame() {
    let g = geom(2, 1, Depth::Eight);
    let s = stream(&g, &Pattern::HRamp, 3);
    let mut t = s.clone();
    t[8] ^= 1;
    let (a, b) = write_pair("verbose", &s, &t);
    let (code, out, _) = cmp(&format!("{G8} -v {a} {b}"), &[]);
    assert_eq!(code, 1);
    assert!(
        out.starts_with("frame 0 equal\nframe 1 differ samples=1\nframe 2 equal\ndiffer "),
        "{out}"
    );
}

#[test]
fn cmp_length_difference_has_no_first_sample() {
    let g = geom(2, 1, Depth::Eight);
    let s = stream(&g, &Pattern::HRamp, 3);
    let (a, b) = write_pair("length", &s, &s[..16]);
    let (code, out, _) = cmp(&format!("{G8} {a} {b}"), &[]);
    assert_eq!(code, 1);
    assert_eq!(
        out,
        "differ frames_a=3 frames_b=2 compared=2 equal=2 different=0 samples=0\nfirst none\n"
    );
}

#[test]
fn cmp_reads_standard_input_as_either_argument() {
    let g = geom(2, 1, Depth::Eight);
    let s = stream(&g, &Pattern::VRamp, 2);
    let (a, _) = write_pair("stdin", &s, &s);
    assert_eq!(cmp(&format!("{G8} {a} -"), &s).0, 0);
    assert_eq!(cmp(&format!("{G8} - {a}"), &s).0, 0);
    let mut other = s.clone();
    other[0] ^= 1;
    assert_eq!(cmp(&format!("{G8} - {a}"), &other).0, 1);
    assert_eq!(cmp(&format!("{G8} {a} -"), &other).0, 1);
}

#[test]
fn cmp_partial_frame_exits_4_and_names_the_stream() {
    let g = geom(2, 1, Depth::Eight);
    let s = stream(&g, &Pattern::HRamp, 2);
    let (a, b) = write_pair("partial", &s, &s[..11]);
    let (code, out, err) = cmp(&format!("{G8} {a} {b}"), &[]);
    assert_eq!((code, out.as_str()), (4, ""));
    assert!(err.starts_with("error: partial_frame: stream b: "), "{err}");
    let (code, _, err) = cmp(&format!("{G8} {b} {a}"), &[]);
    assert_eq!(code, 4);
    assert!(err.contains("stream a: "), "{err}");
}

#[test]
fn cmp_missing_file_exits_4() {
    let (a, _) = write_pair("missing", &[], &[]);
    let (code, out, err) = cmp(&format!("{G8} {a} {a}.absent"), &[]);
    assert_eq!((code, out.as_str()), (4, ""));
    assert!(err.starts_with("error: bad_input: cannot read "), "{err}");
}

#[test]
fn cmp_directory_argument_exits_4() {
    let dir = fresh_dir("dirarg");
    let d = dir.to_str().unwrap();
    let (code, _, err) = cmp(&format!("{G8} {d} {d}"), &[]);
    assert_eq!(code, 4);
    assert!(err.starts_with("error: bad_input: "), "{err}");
}

#[test]
fn cmp_declared_frame_count_mismatch_exits_4() {
    let g = geom(2, 1, Depth::Eight);
    let s = stream(&g, &Pattern::HRamp, 2);
    let (a, b) = write_pair("count", &s, &s);
    assert_eq!(cmp(&format!("{G8} --frames 2 {a} {b}"), &[]).0, 0);
    let (code, _, err) = cmp(&format!("{G8} --frames 3 {a} {b}"), &[]);
    assert_eq!(code, 4);
    assert!(err.starts_with("error: bad_input: stream a: "), "{err}");
}

#[test]
fn cmp_usage_errors_exit_2() {
    let (a, b) = write_pair("usage", &[], &[]);
    for line in [
        String::new(),
        G8.to_string(),
        format!("{G8} {a}"),
        format!("{G8} {a} {b} {a}"),
        format!("{G8} - -"),
        format!("--width 2 --height 1 {a} {b}"),
        format!("{G8} --fps 29.97 {a} {b}"),
        format!("{G8} --frames x {a} {b}"),
        format!("{G8} -v -v {a} {b}"),
        format!("{G8} --nope {a} {b}"),
        format!("--width 0 --height 1 --depth 8 {a} {b}"),
    ] {
        let (code, out, err) = cmp(&line, &[]);
        assert_eq!(code, 2, "{line:?}");
        assert_eq!(out, "");
        assert!(err.starts_with("error: usage: "), "{line:?}: {err}");
    }
}

#[test]
fn cmp_depth_and_size_refusals_exit_3() {
    let (a, b) = write_pair("refuse", &[], &[]);
    assert_eq!(
        cmp(&format!("--width 2 --height 1 --depth 10 {a} {b}"), &[]).0,
        3
    );
    assert_eq!(
        cmp(
            &format!("--width 40000 --height 40000 --depth 8 {a} {b}"),
            &[]
        )
        .0,
        3
    );
}

#[test]
fn cmp_help_prints_usage() {
    let (code, out, err) = cmp("--help", &[]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, CMP_USAGE, ""));
}

#[test]
fn cmp_report_shows_depth_32_values_as_float_and_bits() {
    let g = geom(1, 1, Depth::ThirtyTwo);
    let a = vec![0u8; 16];
    let mut b = a.clone();
    write_sample(&g, &mut b, 0, 0, 0, 0x8000_0000);
    let (a, b) = write_pair("negzero", &a, &b);
    let (code, out, _) = cmp(&format!("--width 1 --height 1 --depth 32 {a} {b}"), &[]);
    assert_eq!(code, 1);
    assert!(
        out.ends_with("first frame=0 x=0 y=0 channel=R a=0.0:00000000 b=-0.0:80000000\n"),
        "{out}"
    );
}

#[test]
fn format_value_forms() {
    assert_eq!(format_value(Depth::Eight, 200), "200");
    assert_eq!(format_value(Depth::Sixteen, 65535), "65535");
    assert_eq!(
        format_value(Depth::ThirtyTwo, 1.0f32.to_bits()),
        "1.0:3f800000"
    );
    assert_eq!(format_value(Depth::ThirtyTwo, 0x7fc0_0001), "NaN:7fc00001");
}

/// The pass through self test: a generated stream, written to a file and compared with a second
/// generation of the same pattern, is equal. It tests the tools, not any effect.
#[test]
fn pass_through_self_test_at_every_depth() {
    for depth in ["8", "16", "32"] {
        for pattern in [
            "hramp",
            "vramp",
            "aramp",
            "checker:2",
            "random:42",
            "constant:10,20,30,40",
        ] {
            let line =
                format!("--width 5 --height 4 --depth {depth} --frames 3 --pattern {pattern}");
            let (c1, first, _) = gen(&line);
            let (c2, second, _) = gen(&line);
            assert_eq!((c1, c2), (0, 0));
            let (a, b) = write_pair("selftest", &first, &second);
            let (code, out, _) = cmp(
                &format!("--width 5 --height 4 --depth {depth} --frames 3 {a} {b}"),
                &[],
            );
            assert_eq!(code, 0, "{depth} {pattern}: {out}");
        }
    }
}
