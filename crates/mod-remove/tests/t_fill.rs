#![allow(clippy::field_reassign_with_default)]
mod support;
use mod_remove::fill::*;
use mod_remove::illum::IllumSet;
use mod_remove::*;
use support::*;

fn rd() -> FillConfig {
    FillConfig::default()
}
fn w16() -> Win {
    Win::new(0, 0, 16, 16)
}

// ---- test vectors of the specification ----
#[test]
fn v1_constant_plate_constant_frame() {
    let c = [0.25, 0.5, 0.75, 1.0];
    let src = src_of(const_img(w16(), c));
    let frame = const_img(w16(), [0.125; 4]);
    let layer = rect_matte(w16(), 6, 6, 9, 9);
    let mut r = rd();
    r.composite = config::CompositeMode::MatteLerp;
    let out = host::render_frame(
        &host::FrameInputs::sample(&frame, &layer, &[src], &IllumSet::None),
        &r,
    )
    .unwrap();
    // inside the one pixel expanded matte (5..=10) every pixel equals C (alpha component = weight 1)
    for y in 5..=10 {
        for x in 5..=10 {
            assert_eq!(bits(out.at(x, y)), bits([0.25, 0.5, 0.75, 1.0]), "{x},{y}");
        }
    }
    // outside the fill window the frame is unchanged
    assert_eq!(bits(out.at(4, 4)), bits([0.125; 4]));
    assert_eq!(bits(out.at(11, 11)), bits([0.125; 4]));
    assert_eq!(bits(out.at(0, 15)), bits([0.125; 4]));
}
#[test]
fn v1_zero_colour_gives_zero_not_nan() {
    let src = src_of(const_img(w16(), [0.0; 4]));
    let fill = rect_matte(w16(), 6, 6, 9, 9);
    let out = run(&fill, &[src], false, &rd());
    let p = out.image.at(7, 7);
    assert_eq!(bits([p[0], p[1], p[2], 0.0]), bits([0.0; 4]));
    assert!(p.iter().all(|v| !v.is_nan()));
}
#[test]
fn v2_gradient_identity() {
    let g = img_fn(w16(), |x, y| [x as f32 / 16.0, y as f32 / 16.0, 0.5, 1.0]);
    let fill = rect_matte(w16(), 3, 3, 12, 12);
    let out = run(&fill, &[src_of(g.clone())], false, &rd());
    for y in 3..=12 {
        for x in 3..=12 {
            let p = out.image.at(x, y);
            let q = g.at(x, y);
            assert_eq!(bits([p[0], p[1], p[2], 0.0]), bits([q[0], q[1], q[2], 0.0]));
        }
    }
}
#[test]
fn v3_integer_translation() {
    let g = img_fn(w16(), |x, y| [x as f32 / 16.0, y as f32 / 16.0, 0.5, 1.0]);
    let mut s = src_of(g.clone());
    s.h = translate(2.0, 1.0);
    let fill = rect_matte(w16(), 3, 3, 10, 10);
    let out = run(&fill, &[s], false, &rd());
    for y in 3..=10 {
        for x in 3..=10 {
            let p = out.image.at(x, y);
            let q = g.at(x + 2, y + 1);
            assert_eq!(bits([p[0], p[1], p[2], 0.0]), bits([q[0], q[1], q[2], 0.0]));
        }
    }
}
#[test]
fn v3_half_pixel_translation_is_mean() {
    let g = img_fn(w16(), |x, _| [x as f32 / 16.0, 0.0, 0.0, 1.0]);
    let mut s = src_of(g);
    s.h = translate(0.5, 0.0);
    let fill = rect_matte(w16(), 4, 4, 6, 6);
    let out = run(&fill, &[s], false, &rd());
    // transfer 4.5 rounds to 5 (inside), sample between pixels 4 and 5
    assert_eq!(out.image.at(4, 4)[0], (4.0f32 / 16.0 + 5.0 / 16.0) * 0.5);
}
#[test]
fn v3_border_clamp_with_larger_cover_mask() {
    let g = img_fn(w16(), |x, _| [x as f32 / 16.0, 0.0, 0.0, 1.0]);
    let mut s = src_of(g);
    s.cover = Matte::new(Win::new(-4, -4, 32, 32), 255);
    s.h = translate(3.0, 0.0);
    let fill = rect_matte(w16(), 13, 4, 14, 4);
    let out = run(&fill, &[s], false, &rd());
    // the last column maps beyond the image: clamp to the edge value
    assert_eq!(out.image.at(14, 4)[0], 15.0 / 16.0);
}
#[test]
fn v4_two_sources_overlapping_seam_matches_reference() {
    let w = Win::new(0, 0, 16, 4);
    let a = src_of(img_fn(w, |x, y| {
        [0.1 * x as f32, 0.2, 0.3 + y as f32 * 0.01, 1.0]
    }));
    let b = src_of(img_fn(w, |x, y| {
        [0.9 - 0.05 * x as f32, 0.6, 0.1 * y as f32, 1.0]
    }));
    let mut a = a;
    let mut b = b;
    a.cover = rect_matte(w, 0, 0, 9, 3);
    b.cover = rect_matte(w, 6, 0, 15, 3);
    let fill = rect_matte(w, 2, 0, 13, 3);
    let mut r = rd();
    r.ramp_len = 3;
    let out = run(&fill, &[a.clone(), b.clone()], false, &r);
    let refc = reference_fill(&fill, &[a, b], 3);
    for y in 0..4 {
        for x in 2..=13 {
            let p = out.image.at(x, y);
            let q = refc[&(x, y)];
            assert_eq!(bits(p), bits(q), "pixel {x},{y}");
        }
    }
}
#[test]
fn v4_seam_weights_fall_in_steps() {
    let w = Win::new(0, 0, 16, 1);
    let mut a = src_of(const_img(w, [1.0, 0.0, 0.0, 1.0]));
    let mut b = src_of(const_img(w, [0.0, 1.0, 0.0, 1.0]));
    a.cover = rect_matte(w, 0, 0, 9, 0);
    b.cover = rect_matte(w, 6, 0, 15, 0);
    let fill = rect_matte(w, 2, 0, 13, 0);
    let mut r = rd();
    r.ramp_len = 3;
    let out = run(&fill, &[a, b], false, &r);
    let wsum = |x: i32| out.weights[out.image.win.idx(x, 0)];
    // x = 9: A 255 plus B ramp 170; x = 8: A plus B 85; x = 7: A only
    assert_eq!(wsum(9), 1.0 + (170.0f32) / 255.0);
    assert_eq!(wsum(8), 1.0 + (85.0f32) / 255.0);
    assert_eq!(wsum(7), 1.0);
    assert_eq!(wsum(10), 1.0);
}
#[test]
fn v5_border_no_source_zero_then_spread() {
    let w = Win::new(0, 0, 8, 8);
    let g = img_fn(w, |x, _| [x as f32 / 8.0, 0.0, 0.0, 1.0]);
    let mut s = src_of(g);
    s.h = translate(2.0, 0.0);
    let fill = rect_matte(w, 4, 2, 7, 4);
    let off = run(&fill, &[s.clone()], false, &rd());
    assert_eq!(bits(off.image.at(6, 3)), bits([0.0; 4]));
    assert!(!off.filled[off.image.win.idx(6, 3)]);
    let on = run(&fill, &[s], true, &rd());
    // pass 1 fills x = 6 with the mean of the three x = 5 pixels (plate column 7), pass 2 x = 7
    assert_eq!(bits(on.image.at(6, 3)), bits([0.875, 0.0, 0.0, 1.0]));
    assert_eq!(bits(on.image.at(7, 2)), bits([0.875, 0.0, 0.0, 1.0]));
    assert!(on.filled[on.image.win.idx(7, 4)]);
}
#[test]
fn v6_illumination_linear_gain_applied() {
    let src = src_of(const_img(w16(), [1.0, 2.0, 4.0, 0.75]));
    let fill = rect_matte(w16(), 6, 6, 9, 9);
    let mut m = [[0.0f32; 4]; 4];
    for (i, g) in [0.5f32, 0.25, 0.125, 1.0].iter().enumerate() {
        m[i][i] = *g;
    }
    let out = fill::run_fill(
        &FillInputs {
            fill: &fill,
            tc: Camera::identity(),
            sources: &[src],
            illum: &IllumSet::Linear(vec![m]),
            layer_filter: None,
            separate_weight: false,
            spread: false,
        },
        &rd(),
    )
    .unwrap();
    let p = out.image.at(7, 7);
    assert_eq!(bits([p[0], p[1], p[2], 0.0]), bits([0.5, 0.5, 0.5, 0.0]));
}
#[test]
fn v8_no_source_zero_and_composite_policy() {
    let frame = const_img(w16(), [0.5; 4]);
    let layer = rect_matte(w16(), 6, 6, 9, 9);
    let mk = |r: &FillConfig| {
        host::render_frame(
            &host::FrameInputs::sample(&frame, &layer, &[], &IllumSet::None),
            r,
        )
    };
    assert!(matches!(mk(&rd()), Err(RemoveError::Unspecified(_))));
    let mut r = rd();
    r.zero_weight = config::ZeroWeightPolicy::KeepFrame;
    assert_eq!(mk(&r).unwrap(), frame);
    r.zero_weight = config::ZeroWeightPolicy::FillOnly;
    assert_eq!(bits(mk(&r).unwrap().at(7, 7)), bits([0.0; 4]));
    let fill = rect_matte(w16(), 6, 6, 9, 9);
    let none = run(&fill, &[], false, &rd());
    assert!(none.image.data.iter().all(|p| bits(*p) == bits([0.0; 4])));
}

// ---- accumulate, normalise ----
#[test]
fn normalise_zero_weight_zeroes_pixel() {
    let mut a = Accum::new(Win::new(0, 0, 1, 1), false);
    a.color[0] = [1.0, 2.0, 3.0, 0.0];
    normalise(&mut a, true);
    assert_eq!(bits(a.color[0]), bits([0.0; 4]));
}
#[test]
fn normalise_divides_not_multiplies_reciprocal() {
    let mut a = Accum::new(Win::new(0, 0, 1, 1), false);
    let w = 1.1f32;
    a.color[0] = [3.0, 0.0, 0.0, w];
    normalise(&mut a, false);
    assert_eq!(a.color[0][0].to_bits(), (3.0f32 / w).to_bits());
    assert_ne!(a.color[0][0].to_bits(), (3.0f32 * (1.0f32 / w)).to_bits());
}
#[test]
fn normalise_preserve_flags() {
    let w = Win::new(0, 0, 1, 1);
    let mut a = Accum::new(w, false);
    a.color[0] = [2.0, 2.0, 2.0, 4.0];
    normalise(&mut a, true);
    assert_eq!(bits(a.color[0]), bits([0.5, 0.5, 0.5, 4.0]));
    let mut b = Accum::new(w, false);
    b.color[0] = [2.0, 2.0, 2.0, 4.0];
    normalise(&mut b, false);
    assert_eq!(bits(b.color[0]), bits([0.5, 0.5, 0.5, 1.0]));
}
#[test]
fn normalise_with_weight_buffer() {
    let w = Win::new(0, 0, 1, 1);
    let mk = || {
        let mut a = Accum::new(w, true);
        a.color[0] = [2.0, 4.0, 6.0, 8.0];
        a.weight.as_mut().unwrap()[0] = [0.0, 0.0, 0.0, 2.0];
        a
    };
    let mut keep = mk();
    normalise(&mut keep, true);
    assert_eq!(bits(keep.color[0]), bits([1.0, 2.0, 3.0, 4.0]));
    assert_eq!(keep.weight.unwrap()[0][3], 2.0);
    let mut drop = mk();
    normalise(&mut drop, false);
    assert_eq!(drop.weight.unwrap()[0][3], 1.0);
}
fn one_source_ctx(wt: f32, use_buffer: bool) -> Accum {
    let w = Win::new(0, 0, 1, 1);
    let mut s = src_of(const_img(w, [1.0, 2.0, 4.0, 0.5]));
    s.weight = wt;
    let fill = Matte::new(w, 255);
    let p = sidx_with(w, 1, &[(0, 0, 0, 0)]);
    let mix = Matte::new(w, 1);
    let sb = SourceMix {
        k: 0,
        orig: 0,
        src: &s,
        mix: Some(&mix),
        mix_win: w,
    };
    let mut acc = Accum::new(w, use_buffer);
    let cx = AccumCtx {
        fill: &fill,
        sidx: &p,
        tc: Camera::identity(),
        illum: &IllumSet::None,
        layer_filter: None,
    };
    accumulate_batch(&mut acc, &cx, 0, &[sb], &rd()).unwrap();
    acc
}
#[test]
fn accumulate_weight_is_weight_times_b_over_255() {
    let acc = one_source_ctx(0.7, false);
    let expect = (0.7f32 * 1.0f32) / 255.0f32;
    assert_eq!(acc.color[0][3].to_bits(), expect.to_bits());
    assert_ne!(expect.to_bits(), (0.7f32 * (1.0f32 / 255.0f32)).to_bits());
}
#[test]
fn accumulate_lane_four_modes() {
    let a = one_source_ctx(1.0, false);
    // no weight buffer: component 3 is the weight sum, not alpha
    assert_eq!(a.color[0][3], 1.0 / 255.0);
    let b = one_source_ctx(1.0, true);
    // weight buffer: colour keeps alpha times w, buffer component 3 gets w
    assert_eq!(b.color[0][3], 0.5 * (1.0 / 255.0));
    assert_eq!(b.weight.unwrap()[0][3], 1.0 / 255.0);
}
#[test]
fn accumulate_batch_local_sum_vs_per_source() {
    let w = Win::new(0, 0, 1, 1);
    let s1 = src_of(const_img(w, [4e-8, 0.0, 0.0, 1.0]));
    let s2 = s1.clone();
    let fill = Matte::new(w, 255);
    let p = sidx_with(w, 1, &[(0, 0, 0, 0)]);
    let mix = Matte::new(w, 255);
    let run1 = |local: bool| {
        let b1 = SourceMix {
            k: 0,
            orig: 0,
            src: &s1,
            mix: Some(&mix),
            mix_win: w,
        };
        let mix_b = SourceMix {
            k: 1,
            orig: 1,
            src: &s2,
            mix: Some(&mix),
            mix_win: w,
        };
        let mut acc = Accum::new(w, false);
        acc.color[0][0] = 1.0;
        let cx = AccumCtx {
            fill: &fill,
            sidx: &p,
            tc: Camera::identity(),
            illum: &IllumSet::None,
            layer_filter: None,
        };
        let mut r = rd();
        r.batch_local_sum = local;
        accumulate_batch(&mut acc, &cx, 0, &[b1, mix_b], &r).unwrap();
        acc.color[0][0]
    };
    assert_eq!(run1(true), 1.0f32 + (4e-8f32 + 4e-8f32));
    assert_eq!(run1(false), (1.0f32 + 4e-8f32) + 4e-8f32);
    assert_ne!(run1(true), run1(false));
}
#[test]
fn pixel_sample_rules() {
    let w = Win::new(0, 0, 4, 4);
    let s = src_of(const_img(w, [0.0; 4]));
    let p = sidx_with(w, 1, &[(1, 1, 0, 0), (2, 2, 0, 1)]);
    let tc = Camera::identity();
    let sb = SourceMix {
        k: 0,
        orig: 0,
        src: &s,
        mix: None,
        mix_win: w,
    };
    // no mix matte: 255 when the channel equals k, no sample when it does not
    assert_eq!(
        pixel_sample(&tc, &p, 0, &sb, 1, 1, None).unwrap(),
        Some((1.5, 1.5, 255))
    );
    assert_eq!(pixel_sample(&tc, &p, 0, &sb, 2, 2, None).unwrap(), None);
    assert_eq!(pixel_sample(&tc, &p, 0, &sb, 0, 0, None).unwrap(), None);
    // mix window excludes the pixel
    let sb_b = SourceMix {
        mix_win: Win::new(2, 2, 2, 2),
        ..sb
    };
    assert_eq!(pixel_sample(&tc, &p, 0, &sb_b, 1, 1, None).unwrap(), None);
}
#[test]
fn layer_filter_restricts_sources() {
    let w = Win::new(0, 0, 4, 4);
    let mut s = src_of(const_img(w, [0.0; 4]));
    s.layer_id = 5;
    let p = sidx_with(w, 1, &[(1, 1, 0, 0)]);
    let tc = Camera::identity();
    let sb = SourceMix {
        k: 0,
        orig: 0,
        src: &s,
        mix: None,
        mix_win: w,
    };
    assert!(pixel_sample(&tc, &p, 0, &sb, 1, 1, Some(5))
        .unwrap()
        .is_some());
    assert!(pixel_sample(&tc, &p, 0, &sb, 1, 1, Some(4))
        .unwrap()
        .is_none());
    assert!(pixel_sample(&tc, &p, 0, &sb, 1, 1, Some(0))
        .unwrap()
        .is_some());
}
#[test]
fn source_weight_scales_contribution() {
    let w = w16();
    let mut a = src_of(const_img(w, [1.0, 1.0, 1.0, 1.0]));
    let mut b = src_of(const_img(w, [0.0, 0.0, 0.0, 1.0]));
    a.weight = 0.75;
    b.weight = 0.25;
    let fill = rect_matte(w, 4, 4, 6, 6);
    let mut r = rd();
    r.index_channels = 2;
    let out = run(&fill, &[a, b], false, &r);
    assert_eq!(out.image.at(5, 5)[0], 0.75);
}
#[test]
fn scatter_flag_changes_mix_selection() {
    let w = Win::new(0, 0, 8, 1);
    let mut a = src_of(const_img(w, [1.0, 0.0, 0.0, 1.0]));
    let mut b = src_of(const_img(w, [0.0, 1.0, 0.0, 1.0]));
    a.cover = rect_matte(w, 0, 0, 5, 0);
    b.cover = rect_matte(w, 2, 0, 7, 0);
    let fill = rect_matte(w, 0, 0, 7, 0);
    let mut r = rd();
    r.ramp_len = 3;
    let plain = run(&fill, &[a.clone(), b.clone()], false, &r);
    r.scatter = true;
    let rnd = run(&fill, &[a, b], false, &r);
    assert_ne!(plain.weights, rnd.weights);
    // every scatterd seam pixel keeps exactly one source
    assert!(rnd.weights[5] == 1.0 || rnd.weights[5] == 170.0f32 / 255.0);
}

// ---- independent slow reference (one channel), written again from the specification ----
fn reference_fill(
    fill: &Matte,
    srcs: &[Source],
    width: i32,
) -> std::collections::HashMap<(i32, i32), [f32; 4]> {
    let mut out = std::collections::HashMap::new();
    let delta = std::cmp::max(1, 255 / width);
    let valid = |x: i32, y: i32, k: usize| -> bool {
        let s = &srcs[k];
        let tx = x as f32 + s.h[2];
        let ty = y as f32 + s.h[5];
        let (px, py) = (tx.round() as i32, ty.round() as i32);
        s.cover.win.contains(px, py) && s.cover.get(px, py) != 0
    };
    let first = |x: i32, y: i32| -> Option<usize> { (0..srcs.len()).find(|&k| valid(x, y, k)) };
    for y in fill.win.t..fill.win.t + fill.win.h {
        for x in fill.win.l..fill.win.l + fill.win.w {
            if fill.get(x, y) == 0 {
                continue;
            }
            let mut acc = [0.0f32; 4];
            for (k, s) in srcs.iter().enumerate() {
                // mix value: distance (chebyshev, through pixels having a source) to the region of k
                let mut best: Option<i32> = None;
                for ry in fill.win.t..fill.win.t + fill.win.h {
                    for rx in fill.win.l..fill.win.l + fill.win.w {
                        if fill.get(rx, ry) != 0 && first(rx, ry) == Some(k) {
                            let d = (rx - x).abs().max((ry - y).abs());
                            if d < width {
                                best = Some(best.map_or(d, |b| b.min(d)));
                            }
                        }
                    }
                }
                let Some(d) = best else { continue };
                if first(x, y).is_none() {
                    continue;
                }
                let b = (255 - d * delta).max(0);
                if b == 0 || !valid(x, y, k) {
                    continue;
                }
                let qx = (x as f32 + s.h[2]) - s.pixels.win.l as f32 + 0.5;
                let qy = (y as f32 + s.h[5]) - s.pixels.win.t as f32 + 0.5;
                let c = mod_remove::geom::bilinear_clamp(&s.pixels, qx, qy);
                let wt = (s.weight * b as f32) / 255.0f32;
                acc[0] += c[0] * wt;
                acc[1] += c[1] * wt;
                acc[2] += c[2] * wt;
                acc[3] += wt;
            }
            let wsum = acc[3];
            let res = if wsum != 0.0 {
                [acc[0] / wsum, acc[1] / wsum, acc[2] / wsum, wsum]
            } else {
                [0.0; 4]
            };
            out.insert((x, y), res);
        }
    }
    out
}
