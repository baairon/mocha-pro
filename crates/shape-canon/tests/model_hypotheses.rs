//! Unit tests for the shape model's hypotheses: interpolation weight and rounding, segment choice,
//! NaN and signed zero handling, flag holds, bounds ties and the text of a query time.
use shape_model::{Contour, Keyframe, PointData};

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
fn quarter_point_not_symmetric() {
    // u = 0.25 distinguishes u from 1-u, and a non-dyadic span pins the rounding form
    let c = Contour::new(vec![
        key(0.0, vec![pt(0.0, 0.0)]),
        key(4.0, vec![pt(8.0, 100.0)]),
    ])
    .unwrap();
    let m = &c.points_at(1.0)[0];
    assert_eq!((m.x, m.y), (2.0, 25.0));
    // The next assertion pins the crate's chosen rounding form, a + (b - a) * u, bit for bit. The
    // form is a hypothesis (INFERRED) of the crate; it has not been compared with the original.
    let c = Contour::new(vec![
        key(0.0, vec![pt(0.3, 0.0)]),
        key(3.0, vec![pt(-7.7, 0.0)]),
    ])
    .unwrap();
    assert_eq!(
        c.points_at(1.0)[0].x.to_bits(),
        (0.3f64 + (-7.7f64 - 0.3) * (1.0 / 3.0)).to_bits()
    );
    // late in the segment (u = 0.75) and the y of tan_in
    let mut q = pt(8.0, 0.0);
    q.tan_in = (-1.0, 4.5);
    let c = Contour::new(vec![key(0.0, vec![pt(0.0, 0.0)]), key(4.0, vec![q])]).unwrap();
    let m = &c.points_at(3.0)[0];
    assert_eq!(m.x, 6.0);
    assert_eq!(m.tan_in, (-1.0, 0.5 + 4.0 * 0.75));
}
#[test]
fn three_keys_pick_segment() {
    let c = Contour::new(vec![
        key(0.0, vec![pt(0.0, 0.0)]),
        key(1.0, vec![pt(10.0, 0.0)]),
        key(2.0, vec![pt(10.0, 40.0)]),
    ])
    .unwrap();
    assert_eq!(c.points_at(1.5)[0].y, 20.0);
    assert_eq!(c.points_at(0.5)[0].x, 5.0);
}
#[test]
fn nan_query_holds_first() {
    let c = Contour::new(vec![
        key(0.0, vec![pt(1.0, 1.0)]),
        key(1.0, vec![pt(2.0, 2.0)]),
    ])
    .unwrap();
    assert_eq!(c.points_at(f64::NAN)[0], c.keys()[0].points[0]);
}
#[test]
fn negative_zero_at_inner_key_kept() {
    let c = Contour::new(vec![
        key(0.0, vec![pt(-0.0, 0.0)]),
        key(1.0, vec![pt(5.0, 0.0)]),
        key(2.0, vec![pt(5.0, 0.0)]),
    ])
    .unwrap();
    assert_eq!(c.points_at(0.0)[0].x.to_bits(), (-0.0f64).to_bits());
    let c = Contour::new(vec![
        key(0.0, vec![pt(1.0, 0.0)]),
        key(1.0, vec![pt(-0.0, 0.0)]),
        key(2.0, vec![pt(5.0, 0.0)]),
    ])
    .unwrap();
    assert_eq!(c.points_at(1.0)[0].x.to_bits(), (-0.0f64).to_bits());
}
#[test]
fn flags_hold_earlier_key() {
    let mut q = pt(1.0, 1.0);
    q.enabled = false;
    q.cusp = true;
    let c = Contour::new(vec![key(0.0, vec![pt(0.0, 0.0)]), key(2.0, vec![q])]).unwrap();
    let m = &c.points_at(1.0)[0];
    assert!(m.enabled && !m.cusp);
}
#[test]
fn bounds_signed_zero() {
    // Pins the crate's tie rule: of equal extreme coordinates the first point's value is kept (strict
    // comparisons), which fixes the sign of a zero extreme and of a zero span. The rule is a
    // hypothesis (INFERRED) of the crate; it has not been compared with the original.
    let c = Contour::new(vec![key(
        0.0,
        vec![pt(0.0, 0.0), pt(-0.0, -0.0), pt(1.0, 1.0)],
    )])
    .unwrap();
    let r = c.bounds_at(0.0);
    assert_eq!(
        (r.left.to_bits(), r.top.to_bits()),
        (0.0f64.to_bits(), 0.0f64.to_bits())
    );
    // equal extremes in order 0.0, -0.0: strict comparisons keep 0.0, so the span is +0.0
    let c = Contour::new(vec![key(0.0, vec![pt(0.0, 0.0), pt(-0.0, -0.0)])]).unwrap();
    let r = c.bounds_at(0.0);
    assert_eq!(
        (r.width.to_bits(), r.height.to_bits()),
        (0.0f64.to_bits(), 0.0f64.to_bits())
    );
}
#[test]
fn negative_zero_query_time_text() {
    let c = Contour::new(vec![key(0.0, vec![pt(1.0, 1.0)])]).unwrap();
    let out = shape_canon::render_dump(&[c], &[-0.0], false, None);
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("c0.t0.time=-0x0.0p+0\n"));
}
