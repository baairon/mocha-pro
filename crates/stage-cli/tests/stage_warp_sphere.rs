//! `warp-sphere` on the built program: its manifest entry, its arguments, every refusal, and
//! what it writes for small made up frames, against values worked out by hand and against the
//! `sphere-reorient` and `resample` crates called directly.

mod stage_warp_common;

use sphere_reorient::{
    cube, equirect, inverse_map, Angle, AngleSign, AxisOrder, Config, CubeConfig, CubeGrid,
    LonDirection, Mat3, NorthEdge, PixelCenter, PoleRows, RotationUse, Size, StereoLayout,
};
use stage_warp_common::*;

const STAGE: &str = "warp-sphere";

/// An 8 by 2 frame: each row holds 1 to 8 and 11 to 18.
fn small() -> Vec<u8> {
    grey(&[1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14, 15, 16, 17, 18])
}

const SMALL: (u32, u32, u32) = (8, 2, 8);

fn run_small(params: &[(&str, &str)]) -> Vec<u8> {
    let all: Vec<(&str, &str)> = [("filter", "nearest")]
        .iter()
        .chain(params.iter())
        .copied()
        .collect();
    let run = apply(STAGE, SMALL, &all, &[], &small());
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{params:?}");
    run.out
}

/// [`small`] with each row turned left by `by` columns.
fn turned(by: usize) -> Vec<u8> {
    let row = |first: u8| -> Vec<u8> { (0..8).map(|i| first + ((i + by) % 8) as u8).collect() };
    grey(&[row(1), row(11)].concat())
}

#[test]
fn the_manifest_entry_declares_the_stage() {
    check_the_entry(
        STAGE,
        "Sphere reorientation warp",
        &[
            "yaw",
            "pitch",
            "roll",
            "in_layout",
            "out_layout",
            "stereo",
            "lon_zero",
            "lon_direction",
            "sphere_center",
            "pole_rows",
            "north",
            "axis_order",
            "angle_sign",
            "rotation_use",
        ],
    );
    let entry = manifest_entry(STAGE);
    assert!(entry.contains("{\"id\":\"yaw\",\"name\":\"Yaw\",\"kind\":\"float\",\"default\":0,\"min\":-36000,\"max\":36000,\"keyed\":true,"));
    assert!(entry.contains("{\"id\":\"in_layout\",\"name\":\"Input layout\",\"kind\":\"popup\",\"default\":\"equirect\",\"choices\":[\"equirect\",\"cube-row6\",\"cube-column6\",\"cube-3x2\",\"cube-2x3\"],\"keyed\":false,"));
    assert!(entry.contains("{\"id\":\"lon_zero\",\"name\":\"Longitude zero\",\"kind\":\"float\",\"default\":0.5,\"min\":0,\"max\":1,\"keyed\":false,"));
    assert_eq!(entry.matches("\"keyed\":true").count(), 3);
}

#[test]
fn describe_answers_the_declared_size() {
    check_describe(STAGE);
}

#[test]
fn the_defaults_return_the_input() {
    for depth in [8, 16] {
        for (w, h) in [(8, 4), (7, 5), (16, 9)] {
            let size = (w, h, depth);
            let input = clip(size, 2, 11);
            let run = apply(STAGE, size, &[], &[], &input);
            assert_eq!((run.code, run.err.as_str()), (0, ""));
            assert!(run.out == input, "depth {depth}, {w} by {h}");
        }
    }
}

#[test]
fn yaw_turns_the_columns() {
    // Longitude grows to the right and the source direction is the output direction turned by
    // the yaw, so output column i reads source column i + yaw / 360 * width.
    assert_eq!(run_small(&[("yaw", "180")]), turned(4));
    assert_eq!(run_small(&[("yaw", "90")]), turned(2));
    assert_eq!(run_small(&[("yaw", "-90")]), turned(6));
    assert_eq!(run_small(&[("yaw", "45")]), turned(1));
    assert_eq!(run_small(&[("yaw", "360")]), small());
    // each of these three reverses the turn
    assert_eq!(
        run_small(&[("yaw", "90"), ("angle_sign", "negated")]),
        turned(6)
    );
    assert_eq!(
        run_small(&[("yaw", "90"), ("rotation_use", "transposed")]),
        turned(6)
    );
    assert_eq!(
        run_small(&[("yaw", "90"), ("lon_direction", "increasing-left")]),
        turned(6)
    );
    // where longitude zero lies does not matter for a turn about the up axis
    assert_eq!(run_small(&[("yaw", "90"), ("lon_zero", "0.25")]), turned(2));
}

#[test]
fn each_stereo_view_turns_inside_itself() {
    // side by side: two views of 4 columns; a half turn moves a column by 2 inside its view
    let out = run_small(&[("stereo", "side-by-side"), ("yaw", "180")]);
    assert_eq!(
        out,
        grey(&[3, 4, 1, 2, 7, 8, 5, 6, 13, 14, 11, 12, 17, 18, 15, 16])
    );
    // over under: two views of one row; the columns turn as in the whole frame
    let out = run_small(&[("stereo", "over-under"), ("yaw", "180")]);
    assert_eq!(out, turned(4));
}

#[test]
fn a_cube_layout_at_the_defaults_returns_the_input() {
    for (word, w, h) in [
        ("cube-row6", 12, 2),
        ("cube-column6", 3, 18),
        ("cube-3x2", 6, 4),
        ("cube-2x3", 4, 6),
    ] {
        let size = (w, h, 8);
        let input = clip(size, 1, 3);
        let params = [("in_layout", word), ("out_layout", word)];
        let run = apply(STAGE, size, &params, &[], &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""), "{word}");
        assert!(run.out == input, "{word}");
    }
}

/// One case of the comparison with the crates.
struct Case {
    params: Vec<(&'static str, &'static str)>,
    cfg: Config,
    angles: [f64; 3],
    from: Option<CubeGrid>,
    to: Option<CubeGrid>,
    stereo: StereoLayout,
    src: (u32, u32),
    dst: (u32, u32),
    sampler: resample::Config,
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            params: vec![("yaw", "33"), ("pitch", "-12"), ("roll", "7")],
            cfg: Config::default(),
            angles: [33.0, -12.0, 7.0],
            from: None,
            to: None,
            stereo: StereoLayout::Mono,
            src: (8, 4),
            dst: (8, 4),
            sampler: resample::Config::default(),
        },
        Case {
            params: vec![
                ("yaw", "200"),
                ("pitch", "45"),
                ("roll", "-80"),
                ("lon_zero", "0.25"),
                ("lon_direction", "increasing-left"),
                ("sphere_center", "corner"),
                ("pole_rows", "on-pole"),
                ("north", "bottom"),
                ("axis_order", "roll-yaw-pitch"),
                ("angle_sign", "negated"),
                ("rotation_use", "transposed"),
                ("width", "10"),
                ("height", "6"),
                ("filter", "keys"),
                ("border", "repeat"),
            ],
            cfg: Config {
                lon_zero: 0.25,
                lon_direction: LonDirection::IncreasingLeft,
                pixel_center: PixelCenter::Corner,
                pole_rows: PoleRows::OnPole,
                north: NorthEdge::Bottom,
                axis_order: AxisOrder::RollYawPitch,
                angle_sign: AngleSign::Negated,
                rotation_use: RotationUse::Transposed,
            },
            angles: [200.0, 45.0, -80.0],
            from: None,
            to: None,
            stereo: StereoLayout::Mono,
            src: (8, 4),
            dst: (10, 6),
            sampler: resample::Config {
                filter: resample::Filter::Keys { a: -0.5 },
                border: resample::Border::Repeat,
                ..resample::Config::default()
            },
        },
        Case {
            params: vec![
                ("yaw", "20"),
                ("pitch", "10"),
                ("out_layout", "cube-3x2"),
                ("width", "6"),
                ("height", "4"),
            ],
            cfg: Config::default(),
            angles: [20.0, 10.0, 0.0],
            from: Some(CubeGrid::Grid3x2),
            to: None,
            stereo: StereoLayout::Mono,
            src: (8, 4),
            dst: (6, 4),
            sampler: resample::Config::default(),
        },
        Case {
            params: vec![
                ("roll", "30"),
                ("in_layout", "cube-2x3"),
                ("out_layout", "cube-row6"),
                ("width", "12"),
                ("height", "2"),
            ],
            cfg: Config::default(),
            angles: [0.0, 0.0, 30.0],
            from: Some(CubeGrid::Row6),
            to: Some(CubeGrid::Grid2x3),
            stereo: StereoLayout::Mono,
            src: (8, 12),
            dst: (12, 2),
            sampler: resample::Config::default(),
        },
        Case {
            params: vec![("yaw", "-50"), ("pitch", "25"), ("stereo", "over-under")],
            cfg: Config::default(),
            angles: [-50.0, 25.0, 0.0],
            from: None,
            to: None,
            stereo: StereoLayout::OverUnder,
            src: (8, 7),
            dst: (8, 7),
            sampler: resample::Config::default(),
        },
    ]
}

#[test]
fn every_layout_equals_the_crates_called_directly() {
    for (n, case) in cases().iter().enumerate() {
        let [yaw, pitch, roll] = case.angles.map(Angle::from_degrees);
        let rot = Mat3::from_yaw_pitch_roll(&case.cfg, yaw, pitch, roll);
        let turn = rot.for_use(&case.cfg);
        let grid = |g: CubeGrid| CubeConfig {
            grid: g,
            ..CubeConfig::default()
        };
        for depth in [8, 16, 32] {
            let size = (case.src.0, case.src.1, depth);
            let input = clip(size, 2, 5);
            let want = reference(&input, size, case.dst, &case.sampler, |_, x, y| {
                let (xi, yi) = (x - 0.5, y - 0.5);
                let first = case.stereo.rect(case.dst.0, case.dst.1, false, 0);
                let eye = usize::from(case.stereo.views() == 2 && !first.contains(xi, yi));
                let out = case.stereo.rect(case.dst.0, case.dst.1, false, eye);
                let read = case.stereo.rect(case.src.0, case.src.1, false, eye);
                let out_size = Size { w: out.w, h: out.h };
                let read_size = Size {
                    w: read.w,
                    h: read.h,
                };
                let (lx, ly) = out.to_local(xi, yi);
                let (px, py) = match (case.from, case.to) {
                    (None, None) => {
                        let p = inverse_map(case.cfg, out_size, read_size, rot)(lx, ly);
                        (p.x, p.y)
                    }
                    _ => {
                        let direction = match case.from {
                            None => equirect::pixel_to_dir(&case.cfg, out_size, lx, ly),
                            Some(g) => cube::pixel_to_dir(&grid(g), out_size, lx, ly)?,
                        };
                        let turned = turn.apply(direction);
                        let p = match case.to {
                            None => equirect::dir_to_pixel(&case.cfg, read_size, turned),
                            Some(g) => cube::dir_to_pixel(&grid(g), read_size, turned)?,
                        };
                        (p.x, p.y)
                    }
                };
                let width = f64::from(read.w);
                let px = if case.to.is_none() && px >= width - 0.5 {
                    px - width
                } else {
                    px
                };
                let (gx, gy) = read.to_global(px, py);
                Some((gx + 0.5, gy + 0.5))
            });
            let run = apply(STAGE, size, &case.params, &[], &input);
            assert_eq!((run.code, run.err.as_str()), (0, ""), "case {n}");
            assert!(run.out == want, "case {n}, depth {depth}");
        }
    }
}

#[test]
fn a_key_file_gives_angles_per_frame() {
    let frame = small();
    let keys = Scratch::new(
        "sphere-moves",
        "keys-1\nkey yaw 0 0 linear\nkey yaw 2 180 hold\n",
    );
    let layer = keys.layer("keys");
    let run = apply(
        STAGE,
        SMALL,
        &[("filter", "nearest")],
        &["--layer", layer.as_str()],
        &[frame.clone(), frame.clone(), frame.clone(), frame.clone()].concat(),
    );
    assert_eq!((run.code, run.err.as_str()), (0, ""));
    assert_eq!(run.out, [frame, turned(2), turned(4), turned(4)].concat());
}

#[test]
fn a_size_that_does_not_fit_its_layout_is_refused() {
    let frame = small();
    for (size, params, line) in [
        (
            SMALL,
            vec![("in_layout", "cube-row6")],
            "error: unsupported: the input view of 8x2 has no square faces for cube-row6\n",
        ),
        (
            SMALL,
            vec![("out_layout", "cube-3x2")],
            "error: unsupported: the output view of 8x2 has no square faces for cube-3x2\n",
        ),
        (
            SMALL,
            vec![("out_layout", "cube-3x2"), ("width", "6"), ("height", "5")],
            "error: unsupported: the output view of 6x5 has no square faces for cube-3x2\n",
        ),
        (
            (12, 2, 8),
            vec![("in_layout", "cube-row6"), ("stereo", "over-under")],
            "error: unsupported: the input view of 12x1 has no square faces for cube-row6\n",
        ),
        (
            (8, 1, 8),
            vec![("stereo", "over-under")],
            "error: unsupported: the input of 8x1 is too small for two stereo views\n",
        ),
        (
            SMALL,
            vec![("stereo", "side-by-side"), ("width", "1")],
            "error: unsupported: the output of 1x2 is too small for two stereo views\n",
        ),
    ] {
        for extra in [&[][..], &["--describe"][..]] {
            for input in [&frame[..], &b""[..]] {
                let run = apply(STAGE, size, &params, extra, input);
                assert_eq!(run.code, 3, "{params:?}");
                assert_eq!(run.err, line, "{params:?}");
                assert!(run.out.is_empty());
            }
        }
    }
    // a size that fits: 12 by 2 is six faces of 2 by 2 in a row
    let fits = apply(
        STAGE,
        SMALL,
        &[("out_layout", "cube-row6"), ("width", "12")],
        &["--describe"],
        b"",
    );
    assert_eq!(fits.code, 0, "{}", fits.err);
}

#[test]
fn every_parameter_refuses_a_bad_value() {
    check_refusals(
        STAGE,
        &[
            (
                &[("yaw", "36001")],
                3,
                "bad value for `yaw`: 36001 is outside -36000 to 36000",
            ),
            (
                &[("pitch", "x")],
                3,
                "bad value for `pitch`: cannot parse `x` as a number",
            ),
            (&[("roll", "-36001")], 3, "bad value for `roll`"),
            (
                &[("in_layout", "fisheye")],
                3,
                "`fisheye` is not one of equirect, cube-row6, cube-column6, cube-3x2, cube-2x3",
            ),
            (&[("out_layout", "cube")], 3, "bad value for `out_layout`"),
            (
                &[("stereo", "left-right")],
                3,
                "`left-right` is not one of mono, over-under, side-by-side",
            ),
            (
                &[("lon_zero", "1.5")],
                3,
                "bad value for `lon_zero`: 1.5 is outside 0 to 1",
            ),
            (&[("lon_zero", "-0.1")], 3, "bad value for `lon_zero`"),
            (
                &[("lon_direction", "east")],
                3,
                "`east` is not one of increasing-right, increasing-left",
            ),
            (
                &[("sphere_center", "half")],
                3,
                "`half` is not one of centre, corner",
            ),
            (
                &[("pole_rows", "outside")],
                3,
                "`outside` is not one of on-pole, inset",
            ),
            (&[("north", "left")], 3, "`left` is not one of top, bottom"),
            (
                &[("axis_order", "xyz")],
                3,
                "`xyz` is not one of yaw-pitch-roll, yaw-roll-pitch",
            ),
            (
                &[("angle_sign", "minus")],
                3,
                "`minus` is not one of positive, negated",
            ),
            (
                &[("rotation_use", "inverse")],
                3,
                "`inverse` is not one of direct, transposed",
            ),
            (
                &[("yaw", "1"), ("yaw", "2")],
                2,
                "parameter `yaw` given more than once",
            ),
            (&[("m00", "1")], 3, "warp-sphere has no parameter `m00`"),
        ],
    );
}

#[test]
fn the_shared_refusals_hold() {
    check_the_shared_refusals(STAGE, ("yaw", "36001"), "lon_zero");
}

#[test]
fn frames_are_written_as_they_are_read() {
    check_it_streams(STAGE, &[("yaw", "30"), ("pitch", "10")], (64, 48));
}
