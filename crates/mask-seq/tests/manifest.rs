//! The `mask-1` manifest: exact bytes, the example on the reference page, round trips and every
//! refusal with its line. All values are invented.

use mask_seq::sha256::to_hex;
use mask_seq::{digest, Depth, Frame, Manifest, ManifestError, ManifestErrorKind};

const EXAMPLE: &str = "format=mask-1\n\
frames=2\n\
first=0\n\
width=3\n\
height=2\n\
depth=8\n\
channels=1\n\
naming=frame_######.pgm\n\
frame.0.sha256=17e174a819c06c75145d49ea023065f4c4216cd19e8a48b0a0b4942f94e6f74d\n\
frame.1.sha256=20f63f9a860eb8170b76e74883f4db9312bdec2d35f41c1236d93158d5a53d8f\n";

fn example_lines() -> Vec<String> {
    EXAMPLE.lines().map(str::to_string).collect()
}

fn join(lines: &[String]) -> Vec<u8> {
    let mut s = lines.join("\n");
    s.push('\n');
    s.into_bytes()
}

fn err(kind: ManifestErrorKind, line: usize) -> Result<Manifest, ManifestError> {
    Err(ManifestError { kind, line })
}

#[test]
fn example_exact_bytes() {
    let a = Frame::new(3, 2, Depth::Eight, vec![0, 128, 255, 255, 128, 0]).unwrap();
    let b = Frame::new(3, 2, Depth::Eight, vec![0, 0, 0, 255, 255, 255]).unwrap();
    let hashes = vec![digest(&a.to_pgm()), digest(&b.to_pgm())];
    assert_eq!(
        to_hex(&hashes[0]),
        "17e174a819c06c75145d49ea023065f4c4216cd19e8a48b0a0b4942f94e6f74d"
    );
    let m = Manifest::new(0, 3, 2, Depth::Eight, hashes).unwrap();
    assert_eq!(m.to_bytes(), EXAMPLE.as_bytes());
    assert_eq!(m.to_bytes().len(), 251);
    assert_eq!(Manifest::parse(EXAMPLE.as_bytes()), Ok(m));
}

#[test]
fn reference_example_is_canonical() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/reference.md");
    let page = std::fs::read_to_string(p).expect("read the reference page");
    let mut block = None;
    let mut lines = page.lines();
    while let Some(l) = lines.next() {
        if l.starts_with("```") {
            let body: Vec<&str> = lines
                .by_ref()
                .take_while(|l| !l.starts_with("```"))
                .collect();
            if body.first() == Some(&"format=mask-1") {
                block = Some(body.join("\n") + "\n");
                break;
            }
        }
    }
    let block = block.expect("a fenced block that starts with format=mask-1");
    assert!(Manifest::parse(block.as_bytes()).is_ok());
    assert_eq!(block, EXAMPLE);
}

#[test]
fn round_trip() {
    let hashes = vec![digest(b"one"), digest(b"two")];
    let m = Manifest::new(999_998, 65535, 1, Depth::Sixteen, hashes).unwrap();
    let b = m.to_bytes();
    let back = Manifest::parse(&b).unwrap();
    assert_eq!(back, m);
    assert_eq!(back.to_bytes(), b);
    assert_eq!(back.frames(), 2);
    assert_eq!(back.first(), 999_998);
    assert_eq!(back.frame_names(), ["frame_999998.pgm", "frame_999999.pgm"]);
}

#[test]
fn refuses_syntax() {
    use ManifestErrorKind::Syntax;
    let base = example_lines();
    let with = |f: &dyn Fn(&mut Vec<String>)| {
        let mut l = base.clone();
        f(&mut l);
        join(&l)
    };
    let mut cases: Vec<(&str, Vec<u8>, usize)> = Vec::new();
    cases.push(("crlf", EXAMPLE.replace('\n', "\r\n").into_bytes(), 1));
    cases.push(("no final lf", EXAMPLE.trim_end().as_bytes().to_vec(), 10));
    cases.push(("empty line", with(&|l| l.insert(3, String::new())), 4));
    cases.push(("no equal sign", with(&|l| l[1] = "frames 2".into()), 2));
    cases.push(("swapped", with(&|l| l.swap(3, 4)), 4));
    cases.push(("unknown key", with(&|l| l.insert(1, "title=x".into())), 2));
    cases.push(("repeated", with(&|l| l.insert(3, "first=0".into())), 4));
    cases.push((
        "upper case hex",
        with(&|l| {
            l[8] = l[8]
                .to_uppercase()
                .replace("FRAME.0.SHA256", "frame.0.sha256")
        }),
        9,
    ));
    cases.push((
        "63 digits",
        with(&|l| {
            let n = l[8].len();
            l[8].truncate(n - 1)
        }),
        9,
    ));
    cases.push(("leading zero", with(&|l| l[1] = "frames=02".into()), 2));
    cases.push(("plus sign", with(&|l| l[3] = "width=+3".into()), 4));
    let mut high = with(&|l| l[7] = "naming=XX".into());
    let at = high.windows(2).position(|w| w == b"XX").unwrap();
    high[at] = 0xC3;
    cases.push(("byte above 0x7e", high, 8));
    cases.push(("tab", with(&|l| l[5] = "depth=\t8".into()), 6));
    cases.push((
        "extra line",
        with(&|l| l.push("frame.2.sha256=0".into())),
        11,
    ));
    cases.push((
        "line removed",
        with(&|l| {
            l.pop();
        }),
        10,
    ));
    cases.push(("frame key out of order", with(&|l| l[8] = l[9].clone()), 9));
    cases.push(("empty input", Vec::new(), 1));
    for (name, bytes, line) in cases {
        assert_eq!(Manifest::parse(&bytes), err(Syntax, line), "{name}");
    }
}

#[test]
fn refuses_format_and_values() {
    use ManifestErrorKind::{Format, Value};
    let base = example_lines();
    let change = |idx: usize, text: &str| {
        let mut l = base.clone();
        l[idx] = text.to_string();
        join(&l)
    };
    let table: [(usize, &str, ManifestErrorKind, usize); 16] = [
        (0, "format=mask-2", Format, 1),
        (0, "format=", Format, 1),
        (1, "frames=0", Value, 2),
        (1, "frames=1000001", Value, 2),
        (2, "first=1000000", Value, 3),
        (3, "width=0", Value, 4),
        (3, "width=65536", Value, 4),
        (4, "height=0", Value, 5),
        (5, "depth=12", Value, 6),
        (5, "depth=0", Value, 6),
        (6, "channels=3", Value, 7),
        (7, "naming=frame_%06d.pgm", Value, 8),
        (1, "frames=99999999999999999999999", Value, 2),
        (2, "first=999999", Value, 3),
        (4, "height=65536", Value, 5),
        (6, "channels=0", Value, 7),
    ];
    for (idx, text, kind, line) in table {
        assert_eq!(
            Manifest::parse(&change(idx, text)),
            err(kind, line),
            "{text}"
        );
    }
}

#[test]
fn first_fault_in_line_order() {
    use ManifestErrorKind::{Syntax, Value};
    let mut l = example_lines();
    l[3] = "width=0".into();
    l[8] = "frame.0.sha256=zz".into();
    assert_eq!(Manifest::parse(&join(&l)), err(Value, 4));

    let mut l = example_lines();
    l[1] = "frames=1000000".into();
    l[8] = "frame.0.sha256=zz".into();
    assert_eq!(Manifest::parse(&join(&l)), err(Syntax, 9));

    let mut l = example_lines();
    l[1] = "frames=1".into();
    l[2] = "first=999999".into();
    l[8] = "frame.999999.sha256=".to_string() + &to_hex(&digest(b"x"));
    l.truncate(9);
    assert!(Manifest::parse(&join(&l)).is_ok());

    let h = |n: usize| vec![digest(b"h"); n];
    let new = |first, w, ht, n| Manifest::new(first, w, ht, Depth::Eight, h(n));
    assert_eq!(new(0, 1, 1, 0), err(Value, 2));
    assert_eq!(new(999_999, 1, 1, 2), err(Value, 3));
    assert_eq!(new(1_000_000, 1, 1, 1), err(Value, 3));
    assert_eq!(new(0, 0, 1, 1), err(Value, 4));
    assert_eq!(new(0, 1, 65536, 1), err(Value, 5));
    assert!(new(999_999, 1, 1, 1).is_ok());
}

#[test]
fn line_content_edge_cases() {
    use ManifestErrorKind::{Format, Syntax, Value};
    let base = example_lines();
    let change = |idx: usize, text: &str| {
        let mut l = base.clone();
        l[idx] = text.to_string();
        join(&l)
    };
    // A DEL byte is outside 0x20 to 0x7E.
    assert_eq!(
        Manifest::parse(&change(0, "format=mask-1\x7f")),
        err(Syntax, 1)
    );
    // A space is allowed in a value; the form name differs.
    assert_eq!(Manifest::parse(&change(0, "format=mask 1")), err(Format, 1));
    // The key ends at the first equals sign.
    assert_eq!(Manifest::parse(&change(7, "naming=a=b")), err(Value, 8));
}

#[test]
fn one_million_hashes_are_allowed() {
    let hashes = vec![[0u8; 32]; 1_000_000];
    let m = Manifest::new(0, 1, 1, Depth::Eight, hashes).unwrap();
    assert_eq!(m.frames(), 1_000_000);
    let more = vec![[0u8; 32]; 1_000_001];
    assert_eq!(
        Manifest::new(0, 1, 1, Depth::Eight, more),
        err(ManifestErrorKind::Value, 2)
    );
}
