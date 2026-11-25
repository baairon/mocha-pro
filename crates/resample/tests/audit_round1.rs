//! Independent audit tests (A-RS): exact oracle kernel tables, identity, borders, ties,
//! premultiplied against straight, all with values worked out by hand or by an exact
//! rational oracle outside this crate.

#![allow(clippy::approx_constant, clippy::too_many_arguments)]

use resample::kernel::{bilinear, keys, lanczos, mitchell, sinc};
use resample::{
    resample, AlphaMode, Border, Config, Filter, Footprint, Image, Layout, Normalize, PixelCenter,
    TieRule,
};

// exact oracle values (Fraction and 60 digit Decimal arithmetic in Python), offsets k/8, k in -17..=17
const OFFS: [f64; 35] = [
    -2.125, -2.0, -1.875, -1.75, -1.625, -1.5, -1.375, -1.25, -1.125, -1.0, -0.875, -0.75, -0.625,
    -0.5, -0.375, -0.25, -0.125, 0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0, 1.125,
    1.25, 1.375, 1.5, 1.625, 1.75, 1.875, 2.0, 2.125,
];
const T_BIL: [f64; 35] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875,
    1.0, 0.875, 0.75, 0.625, 0.5, 0.375, 0.25, 0.125, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    0.0,
];
const T_KEYS_M05: [f64; 35] = [
    0.0,
    0.0,
    -0.0068359375,
    -0.0234375,
    -0.0439453125,
    -0.0625,
    -0.0732421875,
    -0.0703125,
    -0.0478515625,
    0.0,
    0.0908203125,
    0.2265625,
    0.3896484375,
    0.5625,
    0.7275390625,
    0.8671875,
    0.9638671875,
    1.0,
    0.9638671875,
    0.8671875,
    0.7275390625,
    0.5625,
    0.3896484375,
    0.2265625,
    0.0908203125,
    0.0,
    -0.0478515625,
    -0.0703125,
    -0.0732421875,
    -0.0625,
    -0.0439453125,
    -0.0234375,
    -0.0068359375,
    0.0,
    0.0,
];
const T_KEYS_M1: [f64; 35] = [
    0.0,
    0.0,
    -0.013671875,
    -0.046875,
    -0.087890625,
    -0.125,
    -0.146484375,
    -0.140625,
    -0.095703125,
    0.0,
    0.138671875,
    0.296875,
    0.462890625,
    0.625,
    0.771484375,
    0.890625,
    0.970703125,
    1.0,
    0.970703125,
    0.890625,
    0.771484375,
    0.625,
    0.462890625,
    0.296875,
    0.138671875,
    0.0,
    -0.095703125,
    -0.140625,
    -0.146484375,
    -0.125,
    -0.087890625,
    -0.046875,
    -0.013671875,
    0.0,
    0.0,
];
const T_MIT_13: [f64; 35] = [
    0.0,
    0.0,
    -0.004448784722222222,
    -0.014756944444444444,
    -0.0263671875,
    -0.034722222222222224,
    -0.03526475694444445,
    -0.0234375,
    0.005316840277777778,
    0.05555555555555555,
    0.1392144097222222,
    0.2560763888888889,
    0.3924696180555556,
    0.5347222222222222,
    0.6691623263888888,
    0.7821180555555556,
    0.8599175347222222,
    0.8888888888888888,
    0.8599175347222222,
    0.7821180555555556,
    0.6691623263888888,
    0.5347222222222222,
    0.3924696180555556,
    0.2560763888888889,
    0.1392144097222222,
    0.05555555555555555,
    0.005316840277777778,
    -0.0234375,
    -0.03526475694444445,
    -0.034722222222222224,
    -0.0263671875,
    -0.014756944444444444,
    -0.004448784722222222,
    0.0,
    0.0,
];
const T_MIT_B1: [f64; 35] = [
    0.0,
    0.0,
    0.0003255208333333333,
    0.0026041666666666665,
    0.0087890625,
    0.020833333333333332,
    0.040690104166666664,
    0.0703125,
    0.11165364583333333,
    0.16666666666666666,
    0.23600260416666666,
    0.3151041666666667,
    0.3981119791666667,
    0.4791666666666667,
    0.5524088541666666,
    0.6119791666666666,
    0.6520182291666666,
    0.6666666666666666,
    0.6520182291666666,
    0.6119791666666666,
    0.5524088541666666,
    0.4791666666666667,
    0.3981119791666667,
    0.3151041666666667,
    0.23600260416666666,
    0.16666666666666666,
    0.11165364583333333,
    0.0703125,
    0.040690104166666664,
    0.020833333333333332,
    0.0087890625,
    0.0026041666666666665,
    0.0003255208333333333,
    0.0,
    0.0,
];
const T_LAN3: [f64; 35] = [
    0.020436661694717504,
    1.5941088673559535e-59,
    -0.030568488973373668,
    -0.06779133590054288,
    -0.10543837179043845,
    -0.13509491152311703,
    -0.14726516390565372,
    -0.13287101836506404,
    -0.08491246937048241,
    -6.174577500866905e-60,
    0.12053459652598691,
    0.27018982304623407,
    0.437646992543001,
    0.6079271018540267,
    0.7642122243343417,
    0.8900670517104946,
    0.9717147892357164,
    1.0,
    0.9717147892357164,
    0.8900670517104946,
    0.7642122243343417,
    0.6079271018540267,
    0.437646992543001,
    0.27018982304623407,
    0.12053459652598691,
    -6.174577500866905e-60,
    -0.08491246937048241,
    -0.13287101836506404,
    -0.14726516390565372,
    -0.13509491152311703,
    -0.10543837179043845,
    -0.06779133590054288,
    -0.030568488973373668,
    1.5941088673559535e-59,
    0.020436661694717504,
];
const T_LAN2: [f64; 35] = [
    0.0,
    0.0,
    -0.004303314553829798,
    -0.01790518512634444,
    -0.039389261112414115,
    -0.06368435202786181,
    -0.08233539655692322,
    -0.08472480390689066,
    -0.06009506445419018,
    -4.7531919763436194e-60,
    0.09934082083243684,
    0.23534667751914073,
    0.39850331933550837,
    0.5731591682507563,
    0.7396427919997761,
    0.8773540711908776,
    0.9682457746117045,
    1.0,
    0.9682457746117045,
    0.8773540711908776,
    0.7396427919997761,
    0.5731591682507563,
    0.39850331933550837,
    0.23534667751914073,
    0.09934082083243684,
    -4.7531919763436194e-60,
    -0.06009506445419018,
    -0.08472480390689066,
    -0.08233539655692322,
    -0.06368435202786181,
    -0.039389261112414115,
    -0.01790518512634444,
    -0.004303314553829798,
    0.0,
    0.0,
];
const T_SINC: [f64; 35] = [
    0.05732325637673133,
    3.8551915335071436e-59,
    -0.06496635722696217,
    -0.1286166165938723,
    -0.18097230082535473,
    -0.2122065907891938,
    -0.21387635552087375,
    -0.18006326323142122,
    -0.10827726204493696,
    -7.466296496991532e-60,
    0.13921362262920467,
    0.30010543871903533,
    0.4705279821459223,
    0.6366197723675814,
    0.7842133035765371,
    0.9003163161571061,
    0.9744953584044327,
    1.0,
    0.9744953584044327,
    0.9003163161571061,
    0.7842133035765371,
    0.6366197723675814,
    0.4705279821459223,
    0.30010543871903533,
    0.13921362262920467,
    -7.466296496991532e-60,
    -0.10827726204493696,
    -0.18006326323142122,
    -0.21387635552087375,
    -0.2122065907891938,
    -0.18097230082535473,
    -0.1286166165938723,
    -0.06496635722696217,
    3.8551915335071436e-59,
    0.05732325637673133,
];

fn close(got: f64, want: f64, tol: f64, what: &str, x: f64) {
    assert!(
        (got - want).abs() <= tol,
        "{what} at {x}: got {got:e} want {want:e}"
    );
}

#[test]
fn kernels_against_exact_oracle() {
    for (i, &x) in OFFS.iter().enumerate() {
        close(bilinear(x), T_BIL[i], 1e-15, "bilinear", x);
        close(keys(x, -0.5), T_KEYS_M05[i], 1e-14, "keys -0.5", x);
        close(keys(x, -1.0), T_KEYS_M1[i], 1e-14, "keys -1", x);
        close(
            mitchell(x, 1.0 / 3.0, 1.0 / 3.0),
            T_MIT_13[i],
            1e-14,
            "mitchell 1/3",
            x,
        );
        close(mitchell(x, 1.0, 0.0), T_MIT_B1[i], 1e-14, "mitchell b1", x);
        close(lanczos(x, 3), T_LAN3[i], 1e-13, "lanczos 3", x);
        close(lanczos(x, 2), T_LAN2[i], 1e-13, "lanczos 2", x);
        close(sinc(x), T_SINC[i], 1e-13, "sinc", x);
    }
}

fn cfg(filter: Filter, border: Border, center: PixelCenter) -> Config {
    Config {
        filter,
        border,
        center,
        ..Config::default()
    }
}

fn run(
    data: &[f32],
    w: usize,
    h: usize,
    ch: usize,
    lay: Layout,
    dw: usize,
    dh: usize,
    c: &Config,
    m: impl Fn(f64, f64) -> (f64, f64),
) -> Vec<f32> {
    let img = Image {
        data,
        width: w,
        height: h,
        channels: ch,
        layout: lay,
    };
    resample(&img, dw, dh, m, c).unwrap()
}

#[test]
fn identity_is_bit_exact() {
    let (w, h, ch) = (5usize, 4usize, 2usize);
    let data: Vec<f32> = (0..w * h * ch)
        .map(|i| ((i * 37 % 101) as f32) * 0.173 - 3.1)
        .collect();
    let filters = [
        Filter::Nearest,
        Filter::Bilinear,
        Filter::Keys { a: -0.5 },
        Filter::Keys { a: -0.75 },
        Filter::Mitchell { b: 0.0, c: 0.5 },
        Filter::Mitchell { b: 0.0, c: 0.0 },
        Filter::Lanczos { lobes: 1 },
        Filter::Lanczos { lobes: 2 },
        Filter::Lanczos { lobes: 3 },
        Filter::Lanczos { lobes: 5 },
        Filter::Box,
    ];
    let borders = [
        Border::Zero,
        Border::Clamp,
        Border::Repeat,
        Border::Mirror,
        Border::MirrorNoEdge,
    ];
    for &f in &filters {
        for &b in &borders {
            for &c in &[PixelCenter::Half, PixelCenter::Corner] {
                for &lay in &[Layout::Packed, Layout::Planar] {
                    let conf = cfg(f, b, c);
                    let got = run(&data, w, h, ch, lay, w, h, &conf, |x, y| (x, y));
                    assert_eq!(
                        got.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                        data.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                        "{f:?} {b:?} {c:?} {lay:?}"
                    );
                }
            }
        }
    }
}

const ROW: [f32; 4] = [10.0, 20.0, 30.0, 40.0];

fn shifted(border: Border, shift: f64) -> Vec<f32> {
    let c = cfg(Filter::Nearest, border, PixelCenter::Corner);
    run(&ROW, 4, 1, 1, Layout::Packed, 4, 1, &c, |x, y| {
        (x + shift, y)
    })
}

#[test]
fn border_rules_by_hand() {
    // output x reads source x + 3: indices 3 4 5 6
    assert_eq!(shifted(Border::Zero, 3.0), [40.0, 0.0, 0.0, 0.0]);
    assert_eq!(shifted(Border::Clamp, 3.0), [40.0, 40.0, 40.0, 40.0]);
    assert_eq!(shifted(Border::Repeat, 3.0), [40.0, 10.0, 20.0, 30.0]);
    // mirror with repeated edge: ... 3 | 0 1 2 3 | 3 2 1 0 | 0 ...: 4->40 5->30 6->20
    assert_eq!(shifted(Border::Mirror, 3.0), [40.0, 40.0, 30.0, 20.0]);
    // mirror without repeated edge: 4->30 5->20 6->10
    assert_eq!(shifted(Border::MirrorNoEdge, 3.0), [40.0, 30.0, 20.0, 10.0]);
    // indices -3 -2 -1 0
    assert_eq!(shifted(Border::Zero, -3.0), [0.0, 0.0, 0.0, 10.0]);
    assert_eq!(shifted(Border::Clamp, -3.0), [10.0, 10.0, 10.0, 10.0]);
    assert_eq!(shifted(Border::Repeat, -3.0), [20.0, 30.0, 40.0, 10.0]);
    assert_eq!(shifted(Border::Mirror, -3.0), [30.0, 20.0, 10.0, 10.0]);
    assert_eq!(
        shifted(Border::MirrorNoEdge, -3.0),
        [40.0, 30.0, 20.0, 10.0]
    );
    // far away: index 100 = 24*4+4... 100 mod 4 = 0
    assert_eq!(shifted(Border::Repeat, 100.0)[0], 10.0);
    // period 8 for mirror: 8 -> 0 ; 9 -> 1 ; 12 -> 40 (12 mod 8 = 4 -> 3)
    assert_eq!(shifted(Border::Mirror, 8.0), [10.0, 20.0, 30.0, 40.0]);
    assert_eq!(shifted(Border::Mirror, 12.0), [40.0, 30.0, 20.0, 10.0]);
    // period 6 for no edge: 6 -> 0
    assert_eq!(shifted(Border::MirrorNoEdge, 6.0), [10.0, 20.0, 30.0, 40.0]);
    assert_eq!(shifted(Border::MirrorNoEdge, 9.0), [40.0, 30.0, 20.0, 10.0]);
}

#[test]
fn single_sample_axis() {
    let d = [7.0f32];
    for b in [
        Border::Clamp,
        Border::Repeat,
        Border::Mirror,
        Border::MirrorNoEdge,
    ] {
        let c = cfg(Filter::Nearest, b, PixelCenter::Corner);
        let got = run(&d, 1, 1, 1, Layout::Packed, 3, 1, &c, |x, y| (x - 1.0, y));
        assert_eq!(got, [7.0, 7.0, 7.0], "{b:?}");
    }
}

#[test]
fn ties_by_hand() {
    // source x + 0.5: u = 0.5 1.5 2.5 3.5 (Corner centre)
    let t = |tie| {
        let c = Config {
            tie,
            ..cfg(Filter::Nearest, Border::Clamp, PixelCenter::Corner)
        };
        run(&ROW, 4, 1, 1, Layout::Packed, 4, 1, &c, |x, y| (x + 0.5, y))
    };
    assert_eq!(t(TieRule::HalfUp), [20.0, 30.0, 40.0, 40.0]);
    assert_eq!(t(TieRule::HalfDown), [10.0, 20.0, 30.0, 40.0]);
    assert_eq!(t(TieRule::HalfEven), [10.0, 30.0, 30.0, 40.0]);
    // negative ties: u = -1.5 -0.5 0.5 (Corner), clamp border, indices -1,0 / 0,1 / 0,1
    let t2 = |tie| {
        let c = Config {
            tie,
            ..cfg(Filter::Nearest, Border::Zero, PixelCenter::Corner)
        };
        run(&ROW, 4, 1, 1, Layout::Packed, 3, 1, &c, |x, y| (x - 1.5, y))
    };
    assert_eq!(t2(TieRule::HalfUp), [0.0, 10.0, 20.0]);
    assert_eq!(t2(TieRule::HalfDown), [0.0, 0.0, 10.0]);
    assert_eq!(t2(TieRule::HalfEven), [0.0, 10.0, 10.0]);
    // Half centre: output centre 0.5 maps to 0.5 + 0.5 = source 1.0, index space 0.5: tie
    let c = Config {
        tie: TieRule::HalfDown,
        ..cfg(Filter::Nearest, Border::Clamp, PixelCenter::Half)
    };
    assert_eq!(
        run(&ROW, 4, 1, 1, Layout::Packed, 1, 1, &c, |x, y| (x + 0.5, y)),
        [10.0]
    );
}

#[test]
fn bilinear_half_pixel_and_premultiplied() {
    // two pixels RGBA; Corner centre; output 0 samples at u = 0.5
    let img = [1.0f32, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.5];
    let mut c = cfg(Filter::Bilinear, Border::Clamp, PixelCenter::Corner);
    c.alpha_channel = Some(3);
    let m = |x: f64, y: f64| (x + 0.5, y);
    let s = run(&img, 2, 1, 4, Layout::Packed, 1, 1, &c, m);
    assert_eq!(s, [0.5, 0.5, 0.0, 0.75]);
    c.alpha = AlphaMode::Premultiplied;
    let p = run(&img, 2, 1, 4, Layout::Packed, 1, 1, &c, m);
    // premultiplied colour 0.5*(1,0,0) + 0.5*(0,0.5,0) = (0.5,0.25,0), alpha 0.75
    let want = [(0.5f64 / 0.75) as f32, (0.25f64 / 0.75) as f32, 0.0, 0.75];
    assert_eq!(p, want);
    // zero alpha everywhere: colour is zero
    let z = [0.9f32, 0.9, 0.9, 0.0, 0.9, 0.9, 0.9, 0.0];
    assert_eq!(
        run(&z, 2, 1, 4, Layout::Packed, 1, 1, &c, m),
        [0.0, 0.0, 0.0, 0.0]
    );
    // straight keeps the colour of a transparent pixel
    c.alpha = AlphaMode::Straight;
    assert_eq!(
        run(&z, 2, 1, 4, Layout::Packed, 1, 1, &c, m),
        [0.9, 0.9, 0.9, 0.0]
    );
}

#[test]
fn box_downscale_by_two() {
    let mut c = cfg(Filter::Box, Border::Clamp, PixelCenter::Half);
    c.footprint = Footprint::Jacobian;
    let got = run(&ROW, 4, 1, 1, Layout::Packed, 2, 1, &c, |x, y| {
        (x * 2.0, y * 2.0)
    });
    assert_eq!(got, [15.0, 35.0]);
    c.footprint = Footprint::Fixed { w: 2.0, h: 1.0 };
    let got = run(&ROW, 4, 1, 1, Layout::Packed, 2, 1, &c, |x, y| {
        (x * 2.0, y * 2.0)
    });
    assert_eq!(got, [15.0, 35.0]);
    // Zero border, extent 4 centred on the image edge: half the weight lies outside
    let mut z = cfg(Filter::Box, Border::Zero, PixelCenter::Corner);
    z.footprint = Footprint::Fixed { w: 4.0, h: 1.0 };
    // u = -0.5: [-2.5, 1.5] covers pixels -2 -1 0 1 with weight 1/4 each: (10 + 20) / 4
    let got = run(&ROW, 4, 1, 1, Layout::Packed, 1, 1, &z, |_, y| (-0.5, y));
    assert_eq!(got, [7.5]);
    z.normalize = Normalize::SumInBounds;
    let got = run(&ROW, 4, 1, 1, Layout::Packed, 1, 1, &z, |_, y| (-0.5, y));
    assert_eq!(got, [15.0]);
}

#[test]
fn bilinear_values_and_normalize() {
    // 0.25 along: 10 + 0.25 * 10
    let c = cfg(Filter::Bilinear, Border::Clamp, PixelCenter::Corner);
    let got = run(&ROW, 4, 1, 1, Layout::Packed, 3, 1, &c, |x, y| {
        (x + 0.25, y)
    });
    assert_eq!(got, [12.5, 22.5, 32.5]);
    // at the right edge u = 3.5: clamp gives 40
    let got = run(&ROW, 4, 1, 1, Layout::Packed, 1, 1, &c, |_, y| (3.5, y));
    assert_eq!(got, [40.0]);
    // Zero border at u = 3.5 : 0.5 * 40
    let z = cfg(Filter::Bilinear, Border::Zero, PixelCenter::Corner);
    assert_eq!(
        run(&ROW, 4, 1, 1, Layout::Packed, 1, 1, &z, |_, y| (3.5, y)),
        [20.0]
    );
    let mut zs = z;
    zs.normalize = Normalize::SumInBounds;
    assert_eq!(
        run(&ROW, 4, 1, 1, Layout::Packed, 1, 1, &zs, |_, y| (3.5, y)),
        [40.0]
    );
}

#[test]
fn identity_keeps_non_finite_samples_local() {
    // Zero weight taps must not read their sample: 0 * inf is NaN.
    let d = [1.0f32, f32::INFINITY, 3.0, f32::NAN, 5.0];
    let filters = [
        Filter::Nearest,
        Filter::Bilinear,
        Filter::Keys { a: -0.5 },
        Filter::Mitchell { b: 0.0, c: 0.5 },
        Filter::Lanczos { lobes: 3 },
        Filter::Box,
    ];
    for f in filters {
        for b in [Border::Zero, Border::Clamp, Border::Repeat, Border::Mirror] {
            let c = cfg(f, b, PixelCenter::Corner);
            let got = run(&d, 5, 1, 1, Layout::Packed, 5, 1, &c, |x, y| (x, y));
            let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&got), bits(&d), "{f:?} {b:?}");
        }
    }
}

#[test]
fn coordinate_limit_boundary() {
    let d = [9.0f32];
    let c = cfg(Filter::Nearest, Border::Clamp, PixelCenter::Corner);
    let at = |v: f64| run(&d, 1, 1, 1, Layout::Packed, 1, 1, &c, |_, y| (v, y))[0];
    assert_eq!(at(5.0e8), 9.0);
    assert_eq!(at(1073741824.0), 9.0);
    assert_eq!(at(-1073741824.0), 9.0);
    assert_eq!(at(1073741825.0), 0.0);
    assert_eq!(at(2147483648.0), 0.0);
    assert_eq!(at(f64::NAN), 0.0);
    assert_eq!(at(f64::INFINITY), 0.0);
}

#[test]
fn nearest_far_from_origin() {
    let d: Vec<f32> = (0..300).map(|i| i as f32).collect();
    let c = cfg(Filter::Nearest, Border::Clamp, PixelCenter::Corner);
    let got = run(&d, 300, 1, 1, Layout::Packed, 2, 1, &c, |x, y| {
        (x + 100.75, y)
    });
    assert_eq!(got, [101.0, 102.0]);
    let got = run(&d, 300, 1, 1, Layout::Packed, 2, 1, &c, |x, y| {
        (x + 200.25, y)
    });
    assert_eq!(got, [200.0, 201.0]);
    let c = cfg(Filter::Nearest, Border::Repeat, PixelCenter::Corner);
    let got = run(&d, 300, 1, 1, Layout::Packed, 2, 1, &c, |x, y| {
        (x - 100.25 - 300.0, y)
    });
    assert_eq!(got, [200.0, 201.0]);
}

#[test]
fn jacobian_extent_and_minimum() {
    // scale 1.5: extent 1.5 around u = 0.25 gives (10 + 0.5 * 20) / 1.5
    let mut c = cfg(Filter::Box, Border::Clamp, PixelCenter::Half);
    c.footprint = Footprint::Jacobian;
    let got = run(&ROW, 4, 1, 1, Layout::Packed, 1, 1, &c, |x, y| (x * 1.5, y));
    assert_eq!(got, [(20.0f64 / 1.5) as f32]);
    // scale 1 but minimum extent 2: u = 0, [-1, 1]: weights 0.25 0.5 0.25 on pixels -1 0 1
    c.center = PixelCenter::Corner;
    c.min_extent = 2.0;
    let got = run(&ROW, 4, 1, 1, Layout::Packed, 1, 1, &c, |x, y| (x, y));
    assert_eq!(got, [12.5]);
    // a scale between the minimum and twice the minimum is kept
    c.min_extent = 1.0;
    let got = run(&ROW, 4, 1, 1, Layout::Packed, 1, 1, &c, |x, y| {
        (x * 1.5 + 1.5, y)
    });
    // u = 1.5: [0.75, 2.25]: pixel 1 (0.5..1.5) overlap 0.75, pixel 2 overlap 0.75 -> 0.5 each
    assert_eq!(got, [25.0]);
}

#[test]
fn accumulation_order_changes_rounding() {
    // 2 by 2 bilinear at the centre, weights 0.25: row major gives 0.25, column major 0.5
    let d = [1.0e30f32, 1.0, -1.0e30, 1.0];
    let mut c = cfg(Filter::Bilinear, Border::Clamp, PixelCenter::Corner);
    let m = |x: f64, y: f64| (x + 0.5, y + 0.5);
    assert_eq!(run(&d, 2, 2, 1, Layout::Packed, 1, 1, &c, m), [0.25]);
    c.order = resample::AccumOrder::ColumnMajor;
    assert_eq!(run(&d, 2, 2, 1, Layout::Packed, 1, 1, &c, m), [0.5]);
}
