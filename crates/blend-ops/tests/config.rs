//! Opacity, matte, forms, clamping, enumeration and determinism.
use blend_ops::*;
use std::collections::HashSet;

fn px(v: f64, a: f64) -> Rgba<f64> {
    Rgba::new(v, v, v, a)
}

#[test]
fn enumeration_size() {
    assert_eq!(Config::enumerate().len(), Config::SPACE_SIZE);
}
#[test]
fn enumeration_distinct() {
    let v = Config::enumerate();
    let s: HashSet<Config> = v.iter().copied().collect();
    assert_eq!(s.len(), v.len());
}
#[test]
fn enumeration_contains_default() {
    assert!(Config::enumerate().contains(&Config::default()));
}
#[test]
fn enumeration_is_stable() {
    assert_eq!(Config::enumerate(), Config::enumerate());
}
#[test]
fn enumeration_first_and_last() {
    let v = Config::enumerate();
    assert_eq!(v[0].input_form, Form::Straight);
    assert_eq!(v[0].dodge_source_one, Guard::Zero);
    assert_eq!(v[v.len() - 1].input_form, Form::Premultiplied);
    assert_eq!(v[v.len() - 1].backdrop_mix, BackdropMix::Off);
}
#[test]
fn default_values() {
    let c = Config::default();
    assert_eq!(c.input_form, Form::Straight);
    assert_eq!(c.output_form, Form::Straight);
    assert_eq!(c.opacity_point, OpacityPoint::OnSourceAlpha);
    assert_eq!(c.clamp, Clamp::None);
    assert_eq!(c.soft_light, SoftLight::W3c);
    assert_eq!(c.guard_order, GuardOrder::BackdropFirst);
    assert_eq!(c.backdrop_mix, BackdropMix::W3c);
}

#[test]
fn opacity_zero_gives_backdrop_both_points() {
    for point in [OpacityPoint::OnSourceAlpha, OpacityPoint::OnResult] {
        for mode in BlendMode::ALL {
            let cfg = Config {
                opacity_point: point,
                ..Config::default()
            };
            let o = composite_pixel(
                Rgba::new(0.25, 0.5, 1.0, 1.0),
                Rgba::new(0.75, 0.5, 0.25, 0.5),
                mode,
                PorterDuff::SrcOver,
                0.0,
                &cfg,
            );
            assert_eq!(o, Rgba::new(0.75, 0.5, 0.25, 0.5), "{:?} {:?}", point, mode);
        }
    }
}
#[test]
fn opacity_zero_on_result_gives_backdrop_for_every_operator() {
    let cfg = Config {
        opacity_point: OpacityPoint::OnResult,
        ..Config::default()
    };
    for pd in PorterDuff::ALL {
        let o = composite_pixel(
            px(0.25, 1.0),
            px(0.75, 0.5),
            BlendMode::Multiply,
            pd,
            0.0,
            &cfg,
        );
        assert_eq!(o, px(0.75, 0.5), "{:?}", pd);
    }
}
#[test]
fn opacity_zero_on_source_alpha_clear_is_still_clear() {
    let o = composite_pixel(
        px(0.25, 1.0),
        px(0.75, 0.5),
        BlendMode::Normal,
        PorterDuff::Clear,
        0.0,
        &Config::default(),
    );
    assert_eq!(o, px(0.0, 0.0));
}
#[test]
fn opacity_one_equals_published_result_for_every_config() {
    let s: Rgba<f64> = Rgba::new(0.75, 0.375, 0.125, 0.5);
    let d: Rgba<f64> = Rgba::new(0.25, 0.5, 0.875, 0.75);
    for cfg in Config::enumerate() {
        let (a, b) = (
            Config {
                opacity_point: OpacityPoint::OnSourceAlpha,
                ..cfg
            },
            Config {
                opacity_point: OpacityPoint::OnResult,
                ..cfg
            },
        );
        for mode in BlendMode::ALL {
            for pd in [PorterDuff::SrcOver, PorterDuff::Xor, PorterDuff::SrcIn] {
                let x = composite_pixel(s, d, mode, pd, 1.0, &a);
                let y = composite_pixel(s, d, mode, pd, 1.0, &b);
                for (p, q) in [(x.r, y.r), (x.g, y.g), (x.b, y.b), (x.a, y.a)] {
                    assert_eq!(p.to_bits(), q.to_bits());
                }
            }
        }
    }
}
#[test]
fn opacity_points_differ_for_src_operator() {
    let s = px(1.0, 1.0);
    let d = px(0.0, 0.5);
    let on_src = Config {
        output_form: Form::Premultiplied,
        ..Config::default()
    };
    let on_res = Config {
        opacity_point: OpacityPoint::OnResult,
        ..on_src
    };
    let a = composite_pixel(s, d, BlendMode::Normal, PorterDuff::Src, 0.5, &on_src);
    let b = composite_pixel(s, d, BlendMode::Normal, PorterDuff::Src, 0.5, &on_res);
    assert_eq!((a.r, a.a), (0.5, 0.5));
    assert_eq!((b.r, b.a), (0.5, 0.75));
}
#[test]
fn opacity_half_on_source_alpha_src_over() {
    let cfg = Config {
        output_form: Form::Premultiplied,
        ..Config::default()
    };
    let o = composite_pixel(
        px(1.0, 1.0),
        px(0.0, 0.5),
        BlendMode::Normal,
        PorterDuff::SrcOver,
        0.5,
        &cfg,
    );
    assert_eq!((o.r, o.a), (0.5, 0.75));
}

#[test]
fn forms_agree_on_opaque_pixels() {
    let s = Rgba::new(0.75, 0.375, 0.125, 1.0);
    let d = Rgba::new(0.25, 0.5, 0.875, 1.0);
    for mode in BlendMode::ALL {
        for pd in PorterDuff::ALL {
            let mut outs = Vec::new();
            for fi in [Form::Straight, Form::Premultiplied] {
                for fo in [Form::Straight, Form::Premultiplied] {
                    let cfg = Config {
                        input_form: fi,
                        output_form: fo,
                        ..Config::default()
                    };
                    outs.push(composite_pixel(s, d, mode, pd, 1.0, &cfg));
                }
            }
            for o in &outs[1..] {
                assert_eq!(o, &outs[0], "{:?} {:?}", mode, pd);
            }
        }
    }
}
#[test]
fn premultiplied_input_matches_straight_input() {
    let straight = Config::default();
    let pre = Config {
        input_form: Form::Premultiplied,
        ..straight
    };
    let a = composite_pixel(
        Rgba::new(0.5, 0.25, 1.0, 0.5),
        px(0.75, 0.5),
        BlendMode::Screen,
        PorterDuff::SrcOver,
        1.0,
        &straight,
    );
    let b = composite_pixel(
        Rgba::new(0.25, 0.125, 0.5, 0.5),
        px(0.375, 0.5),
        BlendMode::Screen,
        PorterDuff::SrcOver,
        1.0,
        &pre,
    );
    assert_eq!(a, b);
}
#[test]
fn output_forms_relate_by_alpha() {
    let s = Rgba::new(0.5, 0.25, 1.0, 0.5);
    let d = px(0.75, 0.5);
    let st = composite_pixel(
        s,
        d,
        BlendMode::Normal,
        PorterDuff::SrcOver,
        1.0,
        &Config::default(),
    );
    let pr = composite_pixel(
        s,
        d,
        BlendMode::Normal,
        PorterDuff::SrcOver,
        1.0,
        &Config {
            output_form: Form::Premultiplied,
            ..Config::default()
        },
    );
    assert_eq!(st.a, pr.a);
    assert_eq!(pr.r / pr.a, st.r);
}
#[test]
fn backdrop_mix_off_changes_translucent_backdrop() {
    let s = px(1.0, 1.0);
    let d = px(0.0, 0.5);
    let on = composite_pixel(
        s,
        d,
        BlendMode::Multiply,
        PorterDuff::SrcOver,
        1.0,
        &Config::default(),
    );
    let off = composite_pixel(
        s,
        d,
        BlendMode::Multiply,
        PorterDuff::SrcOver,
        1.0,
        &Config {
            backdrop_mix: BackdropMix::Off,
            ..Config::default()
        },
    );
    assert_eq!(on.r, 0.5); // (1 - 0.5) * 1 + 0.5 * 0
    assert_eq!(off.r, 0.0);
}
#[test]
fn backdrop_mix_irrelevant_on_opaque_backdrop() {
    let s = px(0.75, 1.0);
    let d = px(0.5, 1.0);
    let a = composite_pixel(
        s,
        d,
        BlendMode::Multiply,
        PorterDuff::SrcOver,
        1.0,
        &Config::default(),
    );
    let b = composite_pixel(
        s,
        d,
        BlendMode::Multiply,
        PorterDuff::SrcOver,
        1.0,
        &Config {
            backdrop_mix: BackdropMix::Off,
            ..Config::default()
        },
    );
    assert_eq!(a, b);
}

#[test]
fn clamp_none_keeps_overshoot() {
    let o = composite_pixel(
        px(0.75, 1.0),
        px(0.5, 1.0),
        BlendMode::Add,
        PorterDuff::SrcOver,
        1.0,
        &Config::default(),
    );
    assert_eq!(o.r, 1.25);
}
#[test]
fn clamp_output_clamps_high() {
    let cfg = Config {
        clamp: Clamp::Output,
        ..Config::default()
    };
    let o = composite_pixel(
        px(0.75, 1.0),
        px(0.5, 1.0),
        BlendMode::Add,
        PorterDuff::SrcOver,
        1.0,
        &cfg,
    );
    assert_eq!(o.r, 1.0);
}
#[test]
fn clamp_output_clamps_low() {
    let cfg = Config {
        clamp: Clamp::Output,
        ..Config::default()
    };
    let o = composite_pixel(
        px(0.75, 1.0),
        px(0.5, 1.0),
        BlendMode::Subtract,
        PorterDuff::SrcOver,
        1.0,
        &cfg,
    );
    assert_eq!(o.r, 0.0);
}
#[test]
fn clamp_output_leaves_inputs() {
    let cfg = Config {
        clamp: Clamp::Output,
        ..Config::default()
    };
    let o = composite_pixel(
        px(1.5, 1.0),
        px(0.5, 1.0),
        BlendMode::Subtract,
        PorterDuff::SrcOver,
        1.0,
        &cfg,
    );
    assert_eq!(o.r, 0.0);
    let o = composite_pixel(
        px(0.5, 1.0),
        px(1.5, 1.0),
        BlendMode::Subtract,
        PorterDuff::SrcOver,
        1.0,
        &cfg,
    );
    assert_eq!(o.r, 1.0);
}
#[test]
fn clamp_inputs_clamps_before_blend() {
    let cfg = Config {
        clamp: Clamp::InputAndOutput,
        ..Config::default()
    };
    let o = composite_pixel(
        px(1.5, 1.0),
        px(0.5, 1.0),
        BlendMode::Subtract,
        PorterDuff::SrcOver,
        1.0,
        &cfg,
    );
    assert_eq!(o.r, 0.0); // 0.5 - 1.0 = -0.5 then clamped
    let o = composite_pixel(
        px(0.25, 1.0),
        px(1.5, 1.0),
        BlendMode::Subtract,
        PorterDuff::SrcOver,
        1.0,
        &cfg,
    );
    assert_eq!(o.r, 0.75); // 1.0 - 0.25
}
#[test]
fn clamp_inputs_clamps_negative_input() {
    let cfg = Config {
        clamp: Clamp::InputAndOutput,
        ..Config::default()
    };
    let o = composite_pixel(
        px(-0.5, 1.0),
        px(0.5, 1.0),
        BlendMode::Add,
        PorterDuff::SrcOver,
        1.0,
        &cfg,
    );
    assert_eq!(o.r, 0.5);
}
#[test]
fn clamp_alpha_output() {
    let cfg = Config {
        clamp: Clamp::Output,
        ..Config::default()
    };
    let o = composite_pixel(
        px(0.5, 1.0),
        px(0.5, 1.0),
        BlendMode::Normal,
        PorterDuff::Xor,
        1.0,
        &cfg,
    );
    assert_eq!(o.a, 0.0);
}

#[test]
fn determinism_across_runs() {
    let s: Rgba<f64> = Rgba::new(0.3, 0.6, 0.9, 0.7);
    let d: Rgba<f64> = Rgba::new(0.2, 0.4, 0.1, 0.6);
    let first: Vec<u64> = BlendMode::ALL
        .iter()
        .map(|&m| {
            composite_pixel(s, d, m, PorterDuff::SrcOver, 0.37, &Config::default())
                .r
                .to_bits()
        })
        .collect();
    for _ in 0..5 {
        let again: Vec<u64> = BlendMode::ALL
            .iter()
            .map(|&m| {
                composite_pixel(s, d, m, PorterDuff::SrcOver, 0.37, &Config::default())
                    .r
                    .to_bits()
            })
            .collect();
        assert_eq!(first, again);
    }
}

#[test]
fn f32_multiply_bits() {
    let o = composite_pixel(
        Rgba::new(0.75f32, 0.75, 0.75, 1.0),
        Rgba::new(0.5f32, 0.5, 0.5, 1.0),
        BlendMode::Multiply,
        PorterDuff::SrcOver,
        1.0,
        &Config::default(),
    );
    assert_eq!(o.r.to_bits(), 0x3EC0_0000);
}
#[test]
fn f32_soft_light_sqrt() {
    let o = composite_pixel(
        Rgba::new(0.75f32, 0.75, 0.75, 1.0),
        Rgba::new(0.5625f32, 0.5625, 0.5625, 1.0),
        BlendMode::SoftLight,
        PorterDuff::SrcOver,
        1.0,
        &Config::default(),
    );
    assert_eq!(o.r.to_bits(), 0.65625f32.to_bits());
}
#[test]
fn f32_constant_conversion_is_rounded_cast() {
    assert_eq!(<f32 as Real>::from_f64(0.3).to_bits(), 0.3f32.to_bits());
}
#[test]
fn f32_pd_over() {
    let cfg = Config {
        output_form: Form::Premultiplied,
        ..Config::default()
    };
    let o = composite_pixel(
        Rgba::new(0.5f32, 0.5, 0.5, 0.5),
        Rgba::new(1.0f32, 1.0, 1.0, 0.25),
        BlendMode::Normal,
        PorterDuff::SrcOver,
        1.0,
        &cfg,
    );
    assert_eq!((o.r, o.a), (0.375, 0.625));
}

#[test]
fn real_helpers() {
    assert_eq!(Real::min(1.0f64, 2.0), 1.0);
    assert_eq!(Real::max(1.0f64, 2.0), 2.0);
    assert_eq!(Real::abs(-0.5f64), 0.5);
    assert_eq!(Real::abs(0.5f64), 0.5);
    assert_eq!(Real::sqrt(2.25f64), 1.5);
    assert_eq!(Real::max(-1.0f32, -2.0), -1.0);
    assert_eq!(Real::min(-1.0f32, -2.0), -2.0);
}

// ---- buffers ----
fn run_buf(matte: Option<&[f64]>, op: f64, cfg: &Config) -> Vec<f64> {
    let src = [1.0, 0.5, 0.25, 1.0, 0.5, 0.5, 0.5, 0.5];
    let dst = [0.25, 0.25, 0.25, 0.5, 0.75, 0.5, 0.25, 1.0];
    let mut out = [0.0; 8];
    composite_buffers(
        &src,
        &dst,
        &mut out,
        matte,
        op,
        BlendMode::Normal,
        PorterDuff::SrcOver,
        cfg,
    )
    .unwrap();
    out.to_vec()
}
#[test]
fn buffer_matches_pixels() {
    let o = run_buf(None, 1.0, &Config::default());
    let p = composite_pixel(
        Rgba::new(0.5, 0.5, 0.5, 0.5),
        Rgba::new(0.75, 0.5, 0.25, 1.0),
        BlendMode::Normal,
        PorterDuff::SrcOver,
        1.0,
        &Config::default(),
    );
    assert_eq!(&o[4..], &[p.r, p.g, p.b, p.a]);
}
#[test]
fn buffer_matte_one_is_identity() {
    let m = [1.0, 1.0];
    assert_eq!(
        run_buf(Some(&m), 1.0, &Config::default()),
        run_buf(None, 1.0, &Config::default())
    );
}
#[test]
fn buffer_matte_zero_gives_backdrop() {
    let m = [0.0, 0.0];
    let o = run_buf(Some(&m), 1.0, &Config::default());
    assert_eq!(&o[..], &[0.25, 0.25, 0.25, 0.5, 0.75, 0.5, 0.25, 1.0]);
}
#[test]
fn buffer_matte_half_equals_opacity_half_on_source_alpha() {
    let m = [0.5, 0.5];
    assert_eq!(
        run_buf(Some(&m), 1.0, &Config::default()),
        run_buf(None, 0.5, &Config::default())
    );
}
#[test]
fn buffer_matte_per_pixel() {
    let m = [0.0, 1.0];
    let o = run_buf(Some(&m), 1.0, &Config::default());
    let full = run_buf(None, 1.0, &Config::default());
    assert_eq!(&o[..4], &[0.25, 0.25, 0.25, 0.5]);
    assert_eq!(&o[4..], &full[4..]);
}
#[test]
fn buffer_matte_premultiplied_source_scales_colour() {
    let cfg = Config {
        input_form: Form::Premultiplied,
        output_form: Form::Premultiplied,
        ..Config::default()
    };
    let src = [0.5, 0.5, 0.5, 0.5];
    let dst = [0.25, 0.25, 0.25, 0.5];
    let mut a = [0.0; 4];
    let mut b = [0.0; 4];
    composite_buffers(
        &src,
        &dst,
        &mut a,
        Some(&[0.5]),
        1.0,
        BlendMode::Normal,
        PorterDuff::SrcOver,
        &cfg,
    )
    .unwrap();
    composite_buffers(
        &[0.25, 0.25, 0.25, 0.25],
        &dst,
        &mut b,
        None,
        1.0,
        BlendMode::Normal,
        PorterDuff::SrcOver,
        &cfg,
    )
    .unwrap();
    assert_eq!(a, b);
}
#[test]
fn buffer_opacity_zero_gives_backdrop() {
    let o = run_buf(None, 0.0, &Config::default());
    assert_eq!(&o[..], &[0.25, 0.25, 0.25, 0.5, 0.75, 0.5, 0.25, 1.0]);
}
#[test]
fn buffer_error_not_whole_pixels() {
    let mut out = [0.0; 4];
    let e = composite_buffers(
        &[0.0; 3],
        &[0.0; 4],
        &mut out,
        None,
        1.0,
        BlendMode::Normal,
        PorterDuff::SrcOver,
        &Config::default(),
    );
    assert_eq!(e, Err(BufferError::NotWholePixels));
}
#[test]
fn buffer_error_not_whole_pixels_out() {
    let mut out = [0.0; 5];
    let e = composite_buffers(
        &[0.0; 4],
        &[0.0; 4],
        &mut out,
        None,
        1.0,
        BlendMode::Normal,
        PorterDuff::SrcOver,
        &Config::default(),
    );
    assert_eq!(e, Err(BufferError::NotWholePixels));
}
#[test]
fn buffer_error_length_mismatch() {
    let mut out = [0.0; 4];
    let e = composite_buffers(
        &[0.0; 4],
        &[0.0; 8],
        &mut out,
        None,
        1.0,
        BlendMode::Normal,
        PorterDuff::SrcOver,
        &Config::default(),
    );
    assert_eq!(e, Err(BufferError::LengthMismatch));
}
#[test]
fn buffer_error_matte_length() {
    let mut out = [0.0; 4];
    let e = composite_buffers(
        &[0.0; 4],
        &[0.0; 4],
        &mut out,
        Some(&[1.0, 1.0]),
        1.0,
        BlendMode::Normal,
        PorterDuff::SrcOver,
        &Config::default(),
    );
    assert_eq!(e, Err(BufferError::LengthMismatch));
}
#[test]
fn buffer_out_mismatch() {
    let mut out = [0.0; 8];
    let e = composite_buffers(
        &[0.0; 4],
        &[0.0; 4],
        &mut out,
        None,
        1.0,
        BlendMode::Normal,
        PorterDuff::SrcOver,
        &Config::default(),
    );
    assert_eq!(e, Err(BufferError::LengthMismatch));
}
#[test]
fn buffer_empty_is_ok() {
    let mut out: [f32; 0] = [];
    assert_eq!(
        composite_buffers(
            &[],
            &[],
            &mut out,
            Some(&[]),
            1.0,
            BlendMode::Normal,
            PorterDuff::SrcOver,
            &Config::default()
        ),
        Ok(())
    );
}
