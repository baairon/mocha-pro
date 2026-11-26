#![allow(clippy::field_reassign_with_default)]
mod support;
use mod_remove::config::MatrixLayout;
use mod_remove::illum::*;
use mod_remove::*;
use support::*;

fn rd() -> FillConfig {
    FillConfig::default()
}
fn sample(x: f32, y: f32) -> ContextSample {
    ContextSample {
        x: [x, x, x, 1.0],
        y: [y, y, y, 1.0],
        geometric_reject: false,
    }
}

#[test]
fn constants_are_the_read_values() {
    assert_eq!(INLIER_FRACTION, 0.8);
    assert_eq!(GAIN_VARIANCE, 0.001);
}
#[test]
fn none_model_is_identity() {
    let c = [0.1, 0.2, 0.3, 0.4];
    assert_eq!(apply_illum(&IllumSet::None, 0, c, 0, 0, &rd()).unwrap(), c);
}
#[test]
fn linear_keeps_alpha_and_layout() {
    let mut m = [[0.0f32; 4]; 4];
    m[0][1] = 2.0; // column layout: out0 = 2 * c1
    m[3][3] = 9.0;
    let c = [1.0, 3.0, 5.0, 0.5];
    let col = apply_illum(&IllumSet::Linear(vec![m]), 0, c, 0, 0, &rd()).unwrap();
    assert_eq!(col, [6.0, 0.0, 0.0, 0.5]);
    let mut r = rd();
    r.matrix_layout = MatrixLayout::RowVector; // out1 = 2 * c0
    let row = apply_illum(&IllumSet::Linear(vec![m]), 0, c, 0, 0, &r).unwrap();
    assert_eq!(row, [0.0, 2.0, 0.0, 0.5]);
}
#[test]
fn linear_missing_matrix_invalid() {
    let e = apply_illum(&IllumSet::Linear(vec![]), 0, [0.0; 4], 0, 0, &rd());
    assert!(matches!(e, Err(RemoveError::Invalid(_))));
}
#[test]
fn matmul_transposes() {
    let a = [
        [1.0, 2.0, 0.0, 0.0],
        [3.0, 4.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ];
    let id = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    assert_eq!(mat4_mul(&a, &id, false, false), a);
    assert_eq!(mat4_mul(&a, &id, true, false)[0][1], 3.0);
    assert_eq!(mat4_mul(&id, &a, false, true)[0][1], 3.0);
    assert_eq!(mat4_mul(&a, &a, false, false)[0][0], 7.0);
    assert_eq!(mat4_mul(&a, &a, true, false)[0][0], 10.0); // 1*1 + 3*3
}
#[test]
fn gram_and_exclusion() {
    let a = vec![[1.0, 2.0, 0.0, 0.0], [3.0, 1.0, 0.0, 0.0]];
    let g = gram(&a, &a, &[true, true]);
    assert_eq!(g[0][0], 10.0);
    assert_eq!(g[0][1], 5.0);
    let g1 = gram(&a, &a, &[true, false]);
    assert_eq!(g1[0][0], 1.0);
}
#[test]
fn diagonal_quotient_and_zero_divisor() {
    let mut yty = [[0.0f32; 4]; 4];
    let mut ytx = [[7.0f32; 4]; 4];
    yty[0][0] = 4.0;
    ytx[0][0] = 2.0;
    yty[2][2] = 0.0;
    let m = solve_normal_eq(&yty, &ytx);
    assert_eq!(m[0][0], 0.5);
    assert_eq!(m[2][2], 1.0);
    assert_eq!(m[0][1], 0.0);
}
#[test]
fn residual_geometric_is_max_and_first_three_lanes() {
    let mut a = sample(1.0, 2.0);
    a.x[3] = 100.0;
    let mut b = sample(1.0, 2.0);
    b.geometric_reject = true;
    let ident = solve_normal_eq(&[[1.0; 4]; 4], &[[1.0; 4]; 4]);
    let r = residuals(&[a, b], &ident, &rd());
    // A = Y (matrix identity on the diagonal 1.0), B = X: three components of (2-1)^2
    assert_eq!(r[0], 3.0);
    assert_eq!(r[1], f32::MAX);
}
#[test]
fn trim_limit_index_is_floor_of_point_eight() {
    let ten: Vec<f32> = (0..10).map(|i| i as f32).collect();
    assert_eq!(trim_limit(&ten).unwrap(), 8.0);
    let three = [0.0, 1.0, 2.0];
    assert_eq!(trim_limit(&three).unwrap(), 2.0); // floor(2.4)
    let mut with_geo = vec![f32::MAX, 5.0, 1.0, 3.0, 2.0];
    assert_eq!(trim_limit(&with_geo).unwrap(), 5.0); // floor(4*0.8) = 3 -> sorted [1,2,3,5]
    with_geo.retain(|&v| v == f32::MAX);
    assert!(trim_limit(&with_geo).is_err());
}
#[test]
fn v6_fit_gain_is_one_over_g() {
    let s: Vec<ContextSample> = (1..=10)
        .map(|i| sample(i as f32 / 8.0, i as f32 / 4.0))
        .collect();
    let m = fit_linear(&s, &rd()).unwrap();
    assert_eq!(m[0][0], 0.5);
    assert_eq!(m[2][2], 0.5);
    assert_eq!(m[0][1], 0.0);
}
#[test]
fn v6_reject_removed_by_second_pass() {
    let mut s: Vec<ContextSample> = (1..=9)
        .map(|i| sample(i as f32 / 8.0, i as f32 / 4.0))
        .collect();
    s.push(sample(5.0, 2.0));
    let two = fit_linear(&s, &rd()).unwrap();
    assert_eq!(two[1][1], 0.5);
    let mut r = rd();
    r.fit_passes = 1;
    let one = fit_linear(&s, &r).unwrap();
    assert_ne!(one[1][1], 0.5);
}
#[test]
fn fit_skips_geometric_and_errors_without_samples() {
    let mut g = sample(1.0, 100.0);
    g.geometric_reject = true;
    let s = vec![g, sample(1.0, 2.0), sample(2.0, 4.0)];
    assert_eq!(fit_linear(&s, &rd()).unwrap()[0][0], 0.5);
    assert!(matches!(
        fit_linear(&[g], &rd()),
        Err(RemoveError::Refused(_))
    ));
}
#[test]
fn fit_refuses_when_every_sample_is_a_geometric_reject() {
    let mut a = sample(1.0, 2.0);
    a.geometric_reject = true;
    let mut b = sample(2.0, 4.0);
    b.geometric_reject = true;
    let mut c = sample(3.0, 6.0);
    c.geometric_reject = true;
    // one pass and the default passes: the refusal comes from the fit itself, not only from
    // the trim step of a second pass
    let mut one_pass = rd();
    one_pass.fit_passes = 1;
    for s in [vec![a, b], vec![a, b, c]] {
        for r in [rd(), one_pass] {
            assert!(matches!(fit_linear(&s, &r), Err(RemoveError::Refused(_))));
        }
    }
}
#[test]
fn v7_interpolated_equidistant_is_mean() {
    let ctx = [
        CtxGain {
            x: 0,
            y: 0,
            w: [1.0, 2.0, 4.0],
        },
        CtxGain {
            x: 2,
            y: 0,
            w: [3.0, 2.0, 8.0],
        },
    ];
    assert_eq!(interpolated_gain(&ctx, 1, 0), [2.0, 2.0, 6.0]);
}
#[test]
fn v7_interpolated_on_context_pixel_is_exact() {
    let ctx = [
        CtxGain {
            x: 0,
            y: 0,
            w: [1.0, 2.0, 4.0],
        },
        CtxGain {
            x: 2,
            y: 0,
            w: [3.0, 2.0, 8.0],
        },
    ];
    assert_eq!(interpolated_gain(&ctx, 2, 0), [3.0, 2.0, 8.0]);
    assert_eq!(interpolated_gain(&ctx, 0, 0), [1.0, 2.0, 4.0]);
}
#[test]
fn interpolated_inverse_square_distance_weights() {
    let ctx = [
        CtxGain {
            x: 0,
            y: 0,
            w: [1.0; 3],
        },
        CtxGain {
            x: 4,
            y: 0,
            w: [5.0; 3],
        },
    ];
    // pixel (1,0): d2 = 1 and 9, weights 1 and 1/81
    let g = interpolated_gain(&ctx, 1, 0);
    let w2 = 1.0f32 / (9.0f32 * 9.0f32);
    let expect = (1.0f32 + 5.0f32 * w2) / (1.0f32 + w2);
    assert_eq!(g[0].to_bits(), expect.to_bits());
}
#[test]
fn interpolated_no_nonzero_w_gives_one() {
    let ctx = [CtxGain {
        x: 0,
        y: 0,
        w: [0.0; 3],
    }];
    assert_eq!(interpolated_gain(&ctx, 3, 3), [1.0; 3]);
    assert_eq!(interpolated_gain(&[], 3, 3), [1.0; 3]);
}
#[test]
fn interpolated_apply_keeps_alpha() {
    let set = IllumSet::Interpolated(vec![vec![CtxGain {
        x: 1,
        y: 1,
        w: [2.0, 2.0, 2.0],
    }]]);
    let c = apply_illum(&set, 0, [1.0, 2.0, 3.0, 0.5], 1, 1, &rd()).unwrap();
    assert_eq!(c, [2.0, 4.0, 6.0, 0.5]);
}
#[test]
fn context_weight_formula() {
    assert_eq!(gain_ratio([0.5; 4], [0.5; 4]), [1.0; 3]);
    assert_eq!(gain_ratio([0.5; 4], [0.0, 0.0, 0.0, 1.0]), [0.0; 3]);
    let w = gain_ratio([2.0, 0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 0.0]);
    assert_eq!(
        w[0].to_bits(),
        ((4.0f32 + 0.001f32) / (1.0f32 + 0.001f32)).sqrt().to_bits()
    );
}
fn bad_exp(_: f32) -> f32 {
    panic!("exp must not be called below sigma 0.5")
}
fn one_exp(_: f32) -> f32 {
    1.0
}
#[test]
fn smoothing_below_half_is_single_tap() {
    let img = img_fn(Win::new(0, 0, 4, 4), |x, _| [x as f32, 0.0, 0.0, 1.0]);
    let mut r = rd();
    r.exp = bad_exp;
    r.smoothing_sigma = 0.49;
    assert_eq!(smoothed_sample(&img, 1.5, 1.5, &r)[0], 1.0);
}
#[test]
fn smoothing_radius_is_ceil_two_sigma() {
    let mut img = Image::new(Win::new(0, 0, 9, 9), [0.0; 4]);
    img.set(4, 4, [7.0, 0.0, 0.0, 0.0]);
    let mut r = rd();
    r.exp = one_exp;
    r.smoothing_sigma = 0.75; // ceil(1.5) = 2, a full square of taps
    assert_eq!(smoothed_sample(&img, 4.5, 4.5, &r)[0], 7.0 / (5.0 * 5.0));
    r.smoothing_sigma = 0.5; // ceil(1.0) = 1, 9 taps
    assert_eq!(smoothed_sample(&img, 4.5, 4.5, &r)[0], 7.0 / 9.0);
}
#[test]
fn smoothing_weights_use_exp_of_minus_d2_over_two_sigma2() {
    fn echo(x: f32) -> f32 {
        x
    }
    let mut img = Image::new(Win::new(0, 0, 5, 5), [0.0; 4]);
    img.set(3, 2, [1.0, 0.0, 0.0, 0.0]);
    let mut r = rd();
    r.exp = echo;
    r.smoothing_sigma = 0.5;
    // weight of tap d2 is -d2 / 0.5: spike at d2 = 1 gives -2, all nine taps sum to minus eight times three
    assert_eq!(
        smoothed_sample(&img, 2.5, 2.5, &r)[0],
        (-2.0f32) / (-8.0f32 * 3.0f32)
    );
}
fn ctx_inputs<'a>(t: &'a Image, s: &'a Source, c: &'a Matte) -> ContextInputs<'a> {
    ContextInputs {
        target: t,
        tc: Camera::identity(),
        source: s,
        context: c,
    }
}
#[test]
fn build_linear_samples_scaled_source() {
    let w = Win::new(0, 0, 16, 16);
    let t = img_fn(w, |x, y| {
        [(x + 1) as f32 / 32.0, (y + 1) as f32 / 32.0, 0.25, 1.0]
    });
    let s = src_of(img_fn(w, |x, y| {
        [(x + 1) as f32 / 16.0, (y + 1) as f32 / 16.0, 0.5, 1.0]
    }));
    let ring = {
        let mut m = rect_matte(w, 4, 4, 11, 11);
        for y in 6..=9 {
            for x in 6..=9 {
                m.set(x, y, 0);
            }
        }
        m
    };
    let samples = build_linear_samples(&ctx_inputs(&t, &s, &ring), &rd()).unwrap();
    assert_eq!(samples.len(), 64 - 16);
    assert!(samples.iter().all(|s| !s.geometric_reject));
    let m = fit_linear(&samples, &rd()).unwrap();
    assert_eq!(m[0][0], 0.5);
    assert_eq!(m[2][2], 0.5);
}
#[test]
fn context_step_grid_thins_samples() {
    let w = Win::new(0, 0, 16, 16);
    let t = const_img(w, [0.5; 4]);
    let s = src_of(const_img(w, [0.5; 4]));
    let ring = rect_matte(w, 4, 4, 11, 11);
    let mut r = rd();
    r.context_step = 2;
    let samples = build_linear_samples(&ctx_inputs(&t, &s, &ring), &r).unwrap();
    assert_eq!(samples.len(), 16); // x, y in 4,6,8,10
}
#[test]
fn context_geometric_rejects_cover_mask_and_margin() {
    let w = Win::new(0, 0, 16, 16);
    let t = const_img(w, [0.5; 4]);
    let mut s = src_of(const_img(w, [0.5; 4]));
    s.h = translate(8.0, 0.0);
    let ctx = rect_matte(w, 6, 4, 6, 4);
    let ci = ctx_inputs(&t, &s, &ctx);
    // 6 + 8 = 14: q = 14.5, inside (margin 1: 14.5 <= 15)
    assert!(!context_pair(&ci, 6, 4, &rd()).unwrap().geometric_reject);
    // 7 + 8 = 15: q = 15.5 > 15, not one pixel inside
    assert!(context_pair(&ci, 7, 4, &rd()).unwrap().geometric_reject);
    // zero source mask
    s.cover.set(14, 4, 0);
    let ci2 = ctx_inputs(&t, &s, &ctx);
    assert!(context_pair(&ci2, 6, 4, &rd()).unwrap().geometric_reject);
}
#[test]
fn interpolated_gains_use_internal_boundary_only() {
    let w = Win::new(0, 0, 16, 16);
    let t = const_img(w, [1.0, 1.0, 1.0, 1.0]);
    let s = src_of(const_img(w, [1.0, 1.0, 1.0, 1.0]));
    let mut ctx = rect_matte(w, 3, 3, 12, 12);
    for y in 5..=10 {
        for x in 5..=10 {
            ctx.set(x, y, 0);
        }
    }
    let fill = rect_matte(w, 5, 5, 10, 10);
    let g = build_interpolated_gains(&ctx_inputs(&t, &s, &ctx), &fill, &rd()).unwrap();
    // sample points with a 4 neighbour in the target: the ring just outside it (4,*) etc
    assert!(g.iter().all(|c| c.w == [1.0; 3]));
    assert!(g.iter().any(|c| c.x == 4 && c.y == 7));
    assert!(!g.iter().any(|c| c.x == 3));
    assert!(!g.iter().any(|c| c.x == 4 && c.y == 4)); // diagonal only
    assert_eq!(g.len(), 8 * 3);
}
#[test]
fn context_matte_construction_unspecified() {
    assert!(matches!(
        context_matte_from_layers(),
        Err(RemoveError::Unspecified(_))
    ));
}
#[test]
fn context_low_edge_margin() {
    let w = Win::new(0, 0, 16, 16);
    let t = const_img(w, [0.5; 4]);
    let mut s = src_of(const_img(w, [0.5; 4]));
    s.h = translate(-6.0, 0.0);
    let ctx = rect_matte(w, 6, 4, 6, 4);
    let ci = ctx_inputs(&t, &s, &ctx);
    // 6 - 6 = 0: q = 0.5 is less than one pixel inside
    assert!(context_pair(&ci, 6, 4, &rd()).unwrap().geometric_reject);
    // 7 - 6 = 1: q = 1.5 inside
    assert!(!context_pair(&ci, 7, 4, &rd()).unwrap().geometric_reject);
}
