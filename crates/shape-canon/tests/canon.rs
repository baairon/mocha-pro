use shape_canon::{fmt_hex, parse_hex, predict, Case};
use shape_model::{Contour, Keyframe, ModelError, PointData};

fn pt(x: f64, y: f64) -> PointData {
    PointData {
        cusp: false,
        enabled: true,
        x,
        y,
        edge_w: Some(2.0),
        edge_r: None,
        tan_in: (-1.0, 0.5),
        tan_out: (1.0, -0.5),
    }
}

fn key(t: f64, pts: Vec<PointData>) -> Keyframe {
    Keyframe {
        time: t,
        points: pts,
    }
}

#[test]
fn hex_known_values() {
    assert_eq!(fmt_hex(1.0), "0x1.0000000000000p+0");
    assert_eq!(fmt_hex(-2.5), "-0x1.4000000000000p+1");
    assert_eq!(fmt_hex(0.1), "0x1.999999999999ap-4");
    assert_eq!(fmt_hex(0.0), "0x0.0p+0");
    assert_eq!(fmt_hex(-0.0), "-0x0.0p+0");
    assert_eq!(fmt_hex(f64::INFINITY), "inf");
    assert_eq!(fmt_hex(f64::NEG_INFINITY), "-inf");
    assert_eq!(fmt_hex(f64::NAN), "nan");
    assert_eq!(fmt_hex(f64::from_bits(1)), "0x0.0000000000001p-1022");
    assert_eq!(fmt_hex(f64::MAX), "0x1.fffffffffffffp+1023");
}

#[test]
fn hex_round_trips_bits() {
    let samples = [
        0.0,
        -0.0,
        1.0,
        0.1,
        -3.75,
        1e300,
        -1e-300,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
        f64::NEG_INFINITY,
    ];
    for v in samples {
        let back = parse_hex(&fmt_hex(v)).expect("parse");
        assert_eq!(back.to_bits(), v.to_bits());
    }
    assert!(parse_hex("nan").expect("nan").is_nan());
}

#[test]
fn hex_rejects_noncanonical() {
    for bad in [
        "",
        "1.0",
        "0x1p+0",
        "0x1.0p+0",
        "0x1.0000000000000p0",
        "0x1.0000000000000p+01",
        "0X1.0000000000000p+0",
        "0x1.A000000000000p+0",
        "0x1.0000000000000P+0",
        "0x2.0000000000000p+0",
        "0x0.0000000000001p-1021",
        "0x1.0000000000000p+1024",
    ] {
        assert!(parse_hex(bad).is_none(), "accepted {bad}");
    }
}

#[test]
fn hex_reads_minus_zero_exponent() {
    // The one spelling parse_hex accepts that fmt_hex never writes: an exponent written -0.
    for (text, want) in [
        ("0x1.0000000000000p-0", 1.0f64),
        ("0x1.8000000000000p-0", 1.5),
        ("0x0.0p-0", 0.0),
        ("-0x0.0p-0", -0.0),
    ] {
        let got = parse_hex(text).expect(text);
        assert_eq!(got.to_bits(), want.to_bits(), "{text}");
        assert_ne!(fmt_hex(got), text);
    }
}

#[test]
fn model_validates_keys() {
    assert_eq!(Contour::new(vec![]), Err(ModelError::NoKeys));
    let a = key(0.0, vec![pt(0.0, 0.0)]);
    let b = key(0.0, vec![pt(1.0, 1.0)]);
    assert_eq!(
        Contour::new(vec![a.clone(), b]),
        Err(ModelError::KeysNotIncreasing)
    );
    let c = key(1.0, vec![pt(1.0, 1.0), pt(2.0, 2.0)]);
    assert_eq!(
        Contour::new(vec![a.clone(), c]),
        Err(ModelError::PointCountMismatch)
    );
    let d = key(f64::NAN, vec![pt(1.0, 1.0)]);
    assert_eq!(Contour::new(vec![a, d]), Err(ModelError::NonFiniteTime));
}

#[test]
fn interpolation_holds_and_lines() {
    let mut q = pt(10.0, 20.0);
    q.cusp = true;
    q.edge_w = Some(4.0);
    q.edge_r = Some(1.0);
    let c = Contour::new(vec![key(2.0, vec![pt(0.0, 0.0)]), key(4.0, vec![q])]).expect("contour");
    assert_eq!(c.points_at(-5.0)[0], c.keys()[0].points[0]);
    assert_eq!(c.points_at(9.0)[0], c.keys()[1].points[0]);
    assert_eq!(c.points_at(2.0)[0], c.keys()[0].points[0]);
    assert_eq!(c.points_at(4.0)[0], c.keys()[1].points[0]);
    let m = &c.points_at(3.0)[0];
    assert_eq!((m.x, m.y), (5.0, 10.0));
    assert!(!m.cusp);
    assert_eq!(m.edge_w, Some(3.0));
    assert_eq!(m.edge_r, None);
    assert_eq!(m.tan_out, (1.0, -0.5));
}

#[test]
fn bounds_span_anchors() {
    let c = Contour::new(vec![key(
        0.0,
        vec![pt(-1.0, 4.0), pt(3.0, -2.0), pt(0.5, 0.5)],
    )])
    .expect("contour");
    let r = c.bounds_at(0.0);
    assert_eq!((r.left, r.top, r.width, r.height), (-1.0, -2.0, 4.0, 6.0));
}

const CASE: &str = "case=1\nbounds=1\nmeta=1\ntime=0x0.0p+0\ntime=0x1.0000000000000p+0\nkey=0x0.0p+0\n\
pt=1,1,0x1.0000000000000p+0,-0x0.0p+0,none,0x1.0000000000000p+1,0x0.0p+0,0x0.0p+0,0x0.0p+0,0x0.0p+0\n\
meta.length=1\n";

#[test]
fn predict_exact_bytes() {
    let case = Case::parse(CASE).expect("case");
    let out = String::from_utf8(predict(&case).expect("predict")).expect("utf8");
    let want = "format=shape-1\nsection=shape\ncontours=1\nc0.points=1\n\
c0.t0.time=0x0.0p+0\nc0.t0.p0.cusp=1\nc0.t0.p0.enabled=1\n\
c0.t0.p0.x=0x1.0000000000000p+0\nc0.t0.p0.y=-0x0.0p+0\n\
c0.t0.p0.edge_w=none\nc0.t0.p0.edge_r=0x1.0000000000000p+1\n\
c0.t0.p0.tan_in=0x0.0p+0,0x0.0p+0\nc0.t0.p0.tan_out=0x0.0p+0,0x0.0p+0\n\
c0.t0.bounds=0x1.0000000000000p+0,-0x0.0p+0,0x0.0p+0,0x0.0p+0\n\
c0.t1.time=0x1.0000000000000p+0\nc0.t1.p0.cusp=1\nc0.t1.p0.enabled=1\n\
c0.t1.p0.x=0x1.0000000000000p+0\nc0.t1.p0.y=-0x0.0p+0\n\
c0.t1.p0.edge_w=none\nc0.t1.p0.edge_r=0x1.0000000000000p+1\n\
c0.t1.p0.tan_in=0x0.0p+0,0x0.0p+0\nc0.t1.p0.tan_out=0x0.0p+0,0x0.0p+0\n\
c0.t1.bounds=0x1.0000000000000p+0,-0x0.0p+0,0x0.0p+0,0x0.0p+0\n\
section=meta\nproject.length=1\nproject.rate=UNKNOWN\nlayer.span_start=UNKNOWN\nlayer.span_end=UNKNOWN\n";
    assert_eq!(out, want);
}

#[test]
fn meta_off_leaves_section_empty() {
    let text = CASE.replace("meta=1\n", "meta=0\n");
    let case = Case::parse(&text).expect("case");
    let out = String::from_utf8(predict(&case).expect("predict")).expect("utf8");
    assert!(out.ends_with("section=meta\n"));
}

#[test]
fn case_parse_errors() {
    assert!(Case::parse("").is_err());
    assert!(Case::parse("case=1\npt=1\n").is_err());
    assert!(Case::parse("case=1\nkey=0x0.0p+0\npt=1,1\n").is_err());
    assert!(Case::parse("case=1\nwhat=1\n").is_err());
    assert!(Case::parse("case=1\ntime=1.0\n").is_err());
    assert!(predict(&Case::parse("case=1\n").expect("header only")).is_err());
}
