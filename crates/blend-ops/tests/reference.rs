//! Independent slow references written differently from the crate, compared on a grid.
use blend_ops::*;

const GRID: usize = 17;

fn v(i: usize) -> f64 {
    i as f64 / (GRID - 1) as f64
}

fn ref_sep(mode: BlendMode, cb: f64, cs: f64) -> f64 {
    match mode {
        BlendMode::Normal => cs,
        BlendMode::Multiply => cb * cs,
        BlendMode::Screen => 1.0 - (1.0 - cb) * (1.0 - cs),
        BlendMode::Overlay => ref_sep(BlendMode::HardLight, cs, cb),
        BlendMode::Darken => {
            if cb < cs {
                cb
            } else {
                cs
            }
        }
        BlendMode::Lighten => {
            if cb > cs {
                cb
            } else {
                cs
            }
        }
        BlendMode::ColorDodge => {
            if cb == 0.0 {
                0.0
            } else if cs >= 1.0 {
                1.0
            } else {
                (cb / (1.0 - cs)).min(1.0)
            }
        }
        BlendMode::ColorBurn => {
            if cb >= 1.0 {
                1.0
            } else if cs <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - cb) / cs).min(1.0)
            }
        }
        BlendMode::HardLight => {
            if cs <= 0.5 {
                cb * 2.0 * cs
            } else {
                1.0 - (1.0 - cb) * (1.0 - (2.0 * cs - 1.0))
            }
        }
        BlendMode::SoftLight => {
            if cs <= 0.5 {
                cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
            } else {
                let d = if cb <= 0.25 {
                    cb * (4.0 + cb * (16.0 * cb - 12.0))
                } else {
                    cb.powf(0.5)
                };
                cb + (2.0 * cs - 1.0) * (d - cb)
            }
        }
        BlendMode::Difference => (cb - cs).abs(),
        BlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
        BlendMode::Add => cb + cs,
        BlendMode::Subtract => cb - cs,
        _ => unreachable!(),
    }
}

#[test]
fn separable_modes_match_reference_on_grid() {
    let cfg = Config::default();
    for mode in BlendMode::ALL.into_iter().filter(|m| m.is_separable()) {
        for i in 0..GRID {
            for j in 0..GRID {
                let (cb, cs) = (v(i), v(j));
                let got = blend_channel(mode, cb, cs, &cfg);
                let want = ref_sep(mode, cb, cs);
                assert!(
                    (got - want).abs() <= 1e-12,
                    "{:?} cb={} cs={} got={} want={}",
                    mode,
                    cb,
                    cs,
                    got,
                    want
                );
            }
        }
    }
}

fn lum_ref(c: [f64; 3]) -> f64 {
    (30.0 * c[0] + 59.0 * c[1] + 11.0 * c[2]) / 100.0
}
fn sat_ref(c: [f64; 3]) -> f64 {
    let mut s = c;
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    s[2] - s[0]
}
fn set_sat_ref(c: [f64; 3], s: f64) -> [f64; 3] {
    let mut s_sorted = c;
    s_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let (mn, mx) = (s_sorted[0], s_sorted[2]);
    if mx == mn {
        return [0.0; 3];
    }
    let f = |x: f64| (x - mn) * s / (mx - mn);
    [f(c[0]), f(c[1]), f(c[2])]
}
fn set_lum_ref(c: [f64; 3], l: f64) -> [f64; 3] {
    let d = l - lum_ref(c);
    let mut o = [c[0] + d, c[1] + d, c[2] + d];
    let lo = lum_ref(o);
    let n = o.iter().cloned().fold(f64::INFINITY, f64::min);
    let x = o.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if n < 0.0 {
        for e in o.iter_mut() {
            *e = lo + (*e - lo) * lo / (lo - n);
        }
    }
    if x > 1.0 {
        for e in o.iter_mut() {
            *e = lo + (*e - lo) * (1.0 - lo) / (x - lo);
        }
    }
    o
}
fn ref_nonsep(mode: BlendMode, cb: [f64; 3], cs: [f64; 3]) -> [f64; 3] {
    match mode {
        BlendMode::Hue => set_lum_ref(set_sat_ref(cs, sat_ref(cb)), lum_ref(cb)),
        BlendMode::Saturation => set_lum_ref(set_sat_ref(cb, sat_ref(cs)), lum_ref(cb)),
        BlendMode::Color => set_lum_ref(cs, lum_ref(cb)),
        BlendMode::Luminosity => set_lum_ref(cb, lum_ref(cs)),
        _ => unreachable!(),
    }
}

#[test]
fn non_separable_modes_match_reference() {
    let cfg = Config::default();
    let pts = [0.0, 0.125, 0.25, 0.5, 0.75, 1.0];
    for mode in [
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
    ] {
        for &a in &pts {
            for &b in &pts {
                for &c in &pts {
                    for &d in &pts {
                        let cb = [a, b, c];
                        let cs = [c, d, a];
                        let got = blend_rgb(mode, cb, cs, &cfg);
                        let want = ref_nonsep(mode, cb, cs);
                        for k in 0..3 {
                            assert!(
                                (got[k] - want[k]).abs() <= 1e-9,
                                "{:?} {:?} {:?} {:?} {:?}",
                                mode,
                                cb,
                                cs,
                                got,
                                want
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn non_separable_results_stay_in_unit_range() {
    let cfg = Config::default();
    let pts = [0.0, 0.25, 0.5, 1.0];
    for mode in [
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
    ] {
        for &a in &pts {
            for &b in &pts {
                for &c in &pts {
                    let o = blend_rgb(mode, [a, b, c], [c, a, b], &cfg);
                    for e in o {
                        assert!((-1e-12..=1.0 + 1e-12).contains(&e));
                    }
                }
            }
        }
    }
}

#[test]
fn lum_hand() {
    assert!((lum([1.0, 0.0, 0.0]) - 0.3).abs() < 1e-16);
    assert!((lum([0.0, 1.0, 0.0]) - 0.59).abs() < 1e-16);
    assert!((lum([0.0, 0.0, 1.0]) - 0.11).abs() < 1e-16);
    assert_eq!(lum([0.0f64; 3]), 0.0);
}
#[test]
fn lum_weights_exact_bits() {
    assert_eq!(lum([1.0, 0.0, 0.0f64]).to_bits(), 0.3f64.to_bits());
    assert_eq!(lum([0.0, 1.0, 0.0f64]).to_bits(), 0.59f64.to_bits());
    assert_eq!(lum([0.0, 0.0, 1.0f64]).to_bits(), 0.11f64.to_bits());
}
#[test]
fn sat_hand() {
    assert_eq!(sat([0.25, 0.5, 1.0]), 0.75);
    assert_eq!(sat([0.5, 0.5, 0.5]), 0.0);
    assert_eq!(sat([1.0, 0.0, 0.5]), 1.0);
}
#[test]
fn set_sat_hand() {
    assert_eq!(
        set_sat([0.25, 0.5, 1.0], 0.5),
        [0.0, 0.5 * 0.25 / 0.75, 0.5]
    );
    assert_eq!(
        set_sat([1.0, 0.25, 0.5], 0.75),
        [0.75, 0.0, 0.75 * 0.25 / 0.75]
    );
    assert_eq!(set_sat([0.5, 0.5, 0.5], 0.75), [0.0, 0.0, 0.0]);
}
#[test]
fn set_sat_ties() {
    assert_eq!(set_sat([1.0, 1.0, 0.0], 0.5), [0.5, 0.5, 0.0]);
    assert_eq!(set_sat([0.0, 1.0, 0.0], 0.5), [0.0, 0.5, 0.0]);
    assert_eq!(set_sat([0.0, 0.0, 1.0], 1.0), [0.0, 0.0, 1.0]);
}
#[test]
fn set_lum_hand_with_clip_high() {
    // (1, 0, 0) to luminosity 0.5: d = 0.2 -> (1.2, 0.2, 0.2), clipped about the luminosity
    let o = set_lum([1.0, 0.0, 0.0], 0.5);
    assert!((o[0] - 1.0).abs() < 1e-15);
    assert!((o[1] - 0.2857142857142857).abs() < 1e-15);
    assert!((o[2] - o[1]).abs() < 1e-15);
}
#[test]
fn set_lum_hand_with_clip_low() {
    // (1, 0, 0) to luminosity 0.1: d = -0.2 -> (0.8, -0.2, -0.2), clipped about the luminosity
    let o = set_lum([1.0, 0.0, 0.0], 0.1);
    assert!((o[1]).abs() < 1e-15);
    assert!((o[0] - 0.1 - 0.7 * 0.1 / 0.3).abs() < 1e-15);
}
#[test]
fn set_lum_no_clip_is_shift() {
    let o = set_lum([0.25, 0.25, 0.25], 0.5);
    for e in o {
        assert!((e - 0.5).abs() < 1e-15);
    }
}
#[test]
fn clip_color_inside_is_identity() {
    let c = [0.25, 0.5, 0.75];
    assert_eq!(clip_color(c), c);
}
#[test]
fn clip_color_degenerate_divisor() {
    // all channels equal and above 1: the divisor is zero, the colour becomes Lum
    let o = clip_color([2.0f64, 2.0, 2.0]);
    assert!(o.iter().all(|e| e.is_finite()));
}
#[test]
fn luminosity_hand() {
    let o = blend_rgb(
        BlendMode::Luminosity,
        [0.5; 3],
        [1.0, 0.0, 0.0],
        &Config::default(),
    );
    for e in o {
        assert!((e - 0.3).abs() < 1e-15);
    }
}
#[test]
fn color_mode_keeps_backdrop_luminosity() {
    let cb = [0.5, 0.25, 0.75];
    let o = blend_rgb(BlendMode::Color, cb, [0.9, 0.1, 0.2], &Config::default());
    assert!((lum(o) - lum(cb)).abs() < 1e-14);
}
#[test]
fn hue_mode_keeps_backdrop_saturation_when_unclipped() {
    let cb = [0.5, 0.25, 0.375];
    let o = blend_rgb(BlendMode::Hue, cb, [0.5, 0.25, 0.0], &Config::default());
    assert!((sat(o) - sat(cb)).abs() < 1e-14);
}
#[test]
fn blend_mode_lists() {
    assert_eq!(BlendMode::ALL.len(), 18);
    assert_eq!(
        BlendMode::ALL.iter().filter(|m| m.is_separable()).count(),
        14
    );
    assert!(!BlendMode::Hue.is_separable());
    assert!(BlendMode::Normal.is_separable());
}
#[test]
fn rgba_helpers() {
    let p = Rgba::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(p.rgb(), [1.0, 2.0, 3.0]);
}
