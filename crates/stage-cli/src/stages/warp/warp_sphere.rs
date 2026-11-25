//! `warp-sphere`: a generic stage. It turns a full sphere image by a rotation, with the
//! `sphere-reorient` crate for the coordinates and the `resample` crate for the samples. It
//! computes a published method under explicit configuration and claims nothing about any
//! product.
//!
//! The map of an output pixel: its direction on the sphere under `out_layout`, the rotation of
//! `yaw`, `pitch` and `roll` (degrees), and the pixel of that direction under `in_layout`. Both
//! layouts equirectangular is `sphere_reorient::inverse_map`. A cube layout uses
//! `cube::pixel_to_dir` and `cube::dir_to_pixel` with the face order and the face turns of
//! `CubeConfig::default()`.
//!
//! Rules of the stage:
//!
//! - `stereo` splits the input frame and the output canvas into views with
//!   `StereoLayout::rect`, the first eye first. An output pixel reads the same view of the
//!   input. A filter tap near the edge of a view reads the neighbouring view.
//! - A layout must fit its view: a cube layout needs square faces, and a stereo layout needs
//!   two views of at least one pixel. A size that does not fit is `unsupported`, also for
//!   `--describe`.
//! - The seam. For an equirectangular input the crate answers a column from 0 up to the width.
//!   The stage moves a column in the last half pixel, from the width minus 0.5 on, back by one
//!   width, so that it lies within half a pixel of the first column. `border` then decides what
//!   a tap beyond the first or the last column reads; `repeat` joins the two edges.
//! - The crate addresses pixel i at the coordinate i. The resampler adds its own offset
//!   (`center`), which the stage removes before the map and adds back after it.

use std::io::{Read, Write};

use sphere_reorient::{
    cube, equirect, inverse_map, Angle, AngleSign, AxisOrder, Config, CubeConfig, CubeGrid,
    LonDirection, Mat3, NorthEdge, PixelCenter, PoleRows, RotationUse, Size, StereoLayout,
};

use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::params::Values;
use crate::registry::{
    FrameSpec, Geometry, Kind, Label, LayerDef, ParamDef, Plan, Stage, StageDef,
};

use super::sampler::{self, choice, float, SHARED};

/// The sphere warp stage.
pub struct WarpSphere;

const ANGLE: (f64, f64, f64) = (0.0, -36000.0, 36000.0);
const LAYOUTS: &[&str] = &[
    "equirect",
    "cube-row6",
    "cube-column6",
    "cube-3x2",
    "cube-2x3",
];

const OWN: [ParamDef; 14] = [
    float("yaw", "Yaw", ANGLE, true, "The turn about the up axis in degrees; may be keyed."),
    float("pitch", "Pitch", ANGLE, true, "The turn about the x axis in degrees; may be keyed."),
    float("roll", "Roll", ANGLE, true, "The turn about the axis of longitude zero in degrees; may be keyed."),
    choice("in_layout", "Input layout", "equirect", LAYOUTS, "How the input stores the sphere: equirectangular, or six cube faces in one of four grids (CubeGrid)."),
    choice("out_layout", "Output layout", "equirect", LAYOUTS, "How the output stores the sphere: equirectangular, or six cube faces in one of four grids (CubeGrid)."),
    choice("stereo", "Stereo layout", "mono", &["mono", "over-under", "side-by-side"], "Whether a frame holds one view or two, and how the two are placed (StereoLayout)."),
    float("lon_zero", "Longitude zero", (0.5, 0.0, 1.0), false, "Where longitude zero lies along the width, as a fraction of the width (Config::lon_zero)."),
    choice("lon_direction", "Longitude direction", "increasing-right", &["increasing-right", "increasing-left"], "Whether longitude grows toward larger or smaller columns (Config::lon_direction)."),
    choice("sphere_center", "Column sample", "centre", &["centre", "corner"], "Whether column i is sampled at (i + 0.5) / width or at i / width of a turn (Config::pixel_center)."),
    choice("pole_rows", "Pole rows", "inset", &["on-pole", "inset"], "Whether the first and the last row lie on the poles or half a row inside them (Config::pole_rows)."),
    choice("north", "North", "top", &["top", "bottom"], "Which pole is at the first row (Config::north)."),
    choice("axis_order", "Axis order", "yaw-pitch-roll", &["yaw-pitch-roll", "yaw-roll-pitch", "pitch-yaw-roll", "pitch-roll-yaw", "roll-yaw-pitch", "roll-pitch-yaw"], "The order in which the three rotations are multiplied; the last named acts first (Config::axis_order)."),
    choice("angle_sign", "Angle sign", "positive", &["positive", "negated"], "Whether the angles are used as given or negated (Config::angle_sign)."),
    choice("rotation_use", "Matrix use", "direct", &["direct", "transposed"], "Whether the matrix or its transpose takes an output direction to a source direction (Config::rotation_use)."),
];

static PARAMS: [ParamDef; SHARED + 14] = sampler::params(OWN);

static LAYERS: [LayerDef; 1] = [sampler::keys_layer(
    "A key file with a value per frame for yaw, pitch and roll.",
)];

/// What the stage declares. Every default is the default of `sphere_reorient::Config` or of
/// `resample::Config`; the default rotation is none.
pub static DEF: StageDef = StageDef {
    id: "warp-sphere",
    name: "Sphere reorientation warp",
    about: "Turns each full sphere frame by a yaw, pitch and roll rotation between equirectangular (Snyder 1987) or cube face layouts, mono or stereo, sampled with a selectable reconstruction filter.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Declared,
    depths: &DEPTHS,
    params: &PARAMS,
    layers: &LAYERS,
};

/// How a view stores the sphere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Longitude along the columns, latitude along the rows.
    Equirect,
    /// Six cube faces in a grid.
    Cube(CubeConfig),
}

/// The layout a choice word names.
pub fn layout(word: &str) -> Layout {
    let grid = match word {
        "cube-row6" => CubeGrid::Row6,
        "cube-column6" => CubeGrid::Column6,
        "cube-3x2" => CubeGrid::Grid3x2,
        "cube-2x3" => CubeGrid::Grid2x3,
        _ => return Layout::Equirect,
    };
    Layout::Cube(CubeConfig {
        grid,
        ..CubeConfig::default()
    })
}

/// The stereo layout of a set of values.
pub fn stereo(values: &Values) -> StereoLayout {
    match values.choice("stereo") {
        "over-under" => StereoLayout::OverUnder,
        "side-by-side" => StereoLayout::SideBySide,
        _ => StereoLayout::Mono,
    }
}

/// The configuration of `sphere_reorient` for a set of values.
pub fn config(values: &Values) -> Config {
    Config {
        lon_zero: values.float("lon_zero"),
        lon_direction: match values.choice("lon_direction") {
            "increasing-left" => LonDirection::IncreasingLeft,
            _ => LonDirection::IncreasingRight,
        },
        pixel_center: match values.choice("sphere_center") {
            "corner" => PixelCenter::Corner,
            _ => PixelCenter::Centre,
        },
        pole_rows: match values.choice("pole_rows") {
            "on-pole" => PoleRows::OnPole,
            _ => PoleRows::Inset,
        },
        north: match values.choice("north") {
            "bottom" => NorthEdge::Bottom,
            _ => NorthEdge::Top,
        },
        axis_order: match values.choice("axis_order") {
            "yaw-roll-pitch" => AxisOrder::YawRollPitch,
            "pitch-yaw-roll" => AxisOrder::PitchYawRoll,
            "pitch-roll-yaw" => AxisOrder::PitchRollYaw,
            "roll-yaw-pitch" => AxisOrder::RollYawPitch,
            "roll-pitch-yaw" => AxisOrder::RollPitchYaw,
            _ => AxisOrder::YawPitchRoll,
        },
        angle_sign: match values.choice("angle_sign") {
            "negated" => AngleSign::Negated,
            _ => AngleSign::Positive,
        },
        rotation_use: match values.choice("rotation_use") {
            "transposed" => RotationUse::Transposed,
            _ => RotationUse::Direct,
        },
    }
}

/// The rotation of a set of values, before `rotation_use`.
pub fn rotation(values: &Values) -> Mat3 {
    Mat3::from_yaw_pitch_roll(
        &config(values),
        Angle::from_degrees(values.float("yaw")),
        Angle::from_degrees(values.float("pitch")),
        Angle::from_degrees(values.float("roll")),
    )
}

/// Whether a frame of `size` holds the views of `stereo`, each fit for `layout`. The error is
/// the text of the refusal.
pub fn fits(
    what: &str,
    word: &str,
    layout: Layout,
    stereo: StereoLayout,
    size: (u32, u32),
) -> Result<(), String> {
    for eye in 0..stereo.views() {
        let view = stereo.rect(size.0, size.1, false, eye);
        if view.w == 0 || view.h == 0 {
            return Err(format!(
                "the {what} of {}x{} is too small for two stereo views",
                size.0, size.1
            ));
        }
        if let Layout::Cube(grid) = layout {
            if grid
                .face_size(Size {
                    w: view.w,
                    h: view.h,
                })
                .is_none()
            {
                return Err(format!(
                    "the {what} view of {}x{} has no square faces for {word}",
                    view.w, view.h
                ));
            }
        }
    }
    Ok(())
}

/// The map of one frame for the resampler: an output position to its source position, both in
/// the coordinates of the resampler, whose pixel centres lie at an index plus `sample_offset`.
/// The sizes fit their layouts ([`fits`]).
pub fn source_map(
    values: &Values,
    src: (u32, u32),
    dst: (u32, u32),
    sample_offset: f64,
) -> impl Fn(f64, f64) -> Option<(f64, f64)> {
    let cfg = config(values);
    let rot = rotation(values);
    let turn = rot.for_use(&cfg);
    let (from, to) = (
        layout(values.choice("out_layout")),
        layout(values.choice("in_layout")),
    );
    let stereo = stereo(values);
    let views = [0, 1].map(|eye| {
        let out = stereo.rect(dst.0, dst.1, false, eye);
        let read = stereo.rect(src.0, src.1, false, eye);
        let out_size = Size { w: out.w, h: out.h };
        let read_size = Size {
            w: read.w,
            h: read.h,
        };
        (
            out,
            read,
            out_size,
            read_size,
            inverse_map(cfg, out_size, read_size, rot),
        )
    });
    move |x: f64, y: f64| {
        let (xi, yi) = (x - sample_offset, y - sample_offset);
        let eye = usize::from(stereo.views() == 2 && !views[0].0.contains(xi, yi));
        let (out, read, out_size, read_size, both_equirect) = &views[eye];
        let (lx, ly) = out.to_local(xi, yi);
        let (px, py) = match (from, to) {
            (Layout::Equirect, Layout::Equirect) => {
                let p = both_equirect(lx, ly);
                (p.x, p.y)
            }
            _ => {
                let direction = match from {
                    Layout::Equirect => equirect::pixel_to_dir(&cfg, *out_size, lx, ly),
                    Layout::Cube(c) => cube::pixel_to_dir(&c, *out_size, lx, ly)?,
                };
                let turned = turn.apply(direction);
                let p = match to {
                    Layout::Equirect => equirect::dir_to_pixel(&cfg, *read_size, turned),
                    Layout::Cube(c) => cube::dir_to_pixel(&c, *read_size, turned)?,
                };
                (p.x, p.y)
            }
        };
        let width = f64::from(read.w);
        let px = if to == Layout::Equirect && px >= width - 0.5 {
            px - width
        } else {
            px
        };
        let (gx, gy) = read.to_global(px, py);
        Some((gx + sample_offset, gy + sample_offset))
    }
}

impl Stage for WarpSphere {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    fn output_size(&self, input: &FrameSpec, values: &Values) -> Result<(u32, u32), StageError> {
        let out = sampler::output_size(input, values);
        let stereo = stereo(values);
        let (in_word, out_word) = (values.choice("in_layout"), values.choice("out_layout"));
        let src = (input.width, input.height);
        fits("input", in_word, layout(in_word), stereo, src)
            .and_then(|()| fits("output", out_word, layout(out_word), stereo, out))
            .map_err(StageError::unsupported)?;
        Ok(out)
    }

    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError> {
        let src = (plan.input.width, plan.input.height);
        let dst = (plan.output.width, plan.output.height);
        let offset = sampler::centre_offset(&plan.values);
        sampler::warp_frames(plan, input, output, |_, values| {
            Ok(source_map(values, src, dst, offset))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> Values {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Values::parse(&DEF, &pairs).unwrap()
    }

    #[test]
    fn the_defaults_are_the_defaults_of_the_crates() {
        let defaults = Values::defaults(&DEF);
        assert_eq!(config(&defaults), Config::default());
        assert_eq!(rotation(&defaults), Mat3::IDENTITY);
        assert_eq!(layout(defaults.choice("in_layout")), Layout::Equirect);
        assert_eq!(layout(defaults.choice("out_layout")), Layout::Equirect);
        assert_eq!(stereo(&defaults), StereoLayout::Mono);
        assert_eq!(sampler::config(&defaults), resample::Config::default());
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        let c = |pairs: &[(&str, &str)]| config(&values(pairs));
        assert_eq!(c(&[("lon_zero", "0.25")]).lon_zero, 0.25);
        assert_eq!(
            c(&[("lon_direction", "increasing-left")]).lon_direction,
            LonDirection::IncreasingLeft
        );
        assert_eq!(
            c(&[("sphere_center", "corner")]).pixel_center,
            PixelCenter::Corner
        );
        assert_eq!(c(&[("pole_rows", "on-pole")]).pole_rows, PoleRows::OnPole);
        assert_eq!(c(&[("north", "bottom")]).north, NorthEdge::Bottom);
        let orders = [
            "yaw-pitch-roll",
            "yaw-roll-pitch",
            "pitch-yaw-roll",
            "pitch-roll-yaw",
            "roll-yaw-pitch",
            "roll-pitch-yaw",
        ];
        for (word, order) in orders.iter().zip(AxisOrder::ALL) {
            assert_eq!(c(&[("axis_order", *word)]).axis_order, order);
        }
        assert_eq!(
            c(&[("angle_sign", "negated")]).angle_sign,
            AngleSign::Negated
        );
        assert_eq!(
            c(&[("rotation_use", "transposed")]).rotation_use,
            RotationUse::Transposed
        );
        for (word, grid) in LAYOUTS[1..].iter().zip(CubeGrid::ALL) {
            assert_eq!(
                layout(word),
                Layout::Cube(CubeConfig {
                    grid,
                    ..CubeConfig::default()
                })
            );
        }
        assert_eq!(
            stereo(&values(&[("stereo", "over-under")])),
            StereoLayout::OverUnder
        );
        assert_eq!(
            stereo(&values(&[("stereo", "side-by-side")])),
            StereoLayout::SideBySide
        );
    }

    #[test]
    fn the_angles_are_degrees_in_the_order_of_the_configuration() {
        let v = values(&[("yaw", "90"), ("pitch", "30"), ("roll", "-45")]);
        let want = Mat3::from_yaw_pitch_roll(
            &Config::default(),
            Angle::from_degrees(90.0),
            Angle::from_degrees(30.0),
            Angle::from_degrees(-45.0),
        );
        assert_eq!(rotation(&v), want);
        let quarter = rotation(&values(&[("yaw", "90")]));
        assert_eq!(quarter, Mat3::about_y(Angle::from_degrees(90.0)));
    }

    #[test]
    fn a_size_must_fit_its_layout() {
        let cube = layout("cube-3x2");
        assert_eq!(
            fits("input", "cube-3x2", cube, StereoLayout::Mono, (6, 4)),
            Ok(())
        );
        assert_eq!(
            fits("input", "cube-3x2", cube, StereoLayout::Mono, (6, 5)),
            Err("the input view of 6x5 has no square faces for cube-3x2".to_string())
        );
        assert_eq!(
            fits("output", "cube-3x2", cube, StereoLayout::OverUnder, (6, 8)),
            Ok(())
        );
        assert_eq!(
            fits(
                "output",
                "equirect",
                Layout::Equirect,
                StereoLayout::OverUnder,
                (6, 1)
            ),
            Err("the output of 6x1 is too small for two stereo views".to_string())
        );
        assert_eq!(
            fits(
                "output",
                "equirect",
                Layout::Equirect,
                StereoLayout::Mono,
                (1, 1)
            ),
            Ok(())
        );
    }

    #[test]
    fn two_equirectangular_layouts_use_the_inverse_map_of_the_crate() {
        let v = values(&[("yaw", "33"), ("pitch", "-12"), ("roll", "7")]);
        let map = source_map(&v, (16, 8), (12, 6), 0.5);
        let want = inverse_map(
            Config::default(),
            Size { w: 12, h: 6 },
            Size { w: 16, h: 8 },
            rotation(&v),
        );
        for (x, y) in [(0.5, 0.5), (3.5, 2.5), (11.5, 5.5), (6.5, 0.5)] {
            let p = want(x - 0.5, y - 0.5);
            let px = if p.x >= 15.5 { p.x - 16.0 } else { p.x };
            assert_eq!(map(x, y), Some((px + 0.5, p.y + 0.5)));
        }
    }

    #[test]
    fn a_half_turn_of_yaw_moves_the_columns_by_half_the_width() {
        // 8 columns: a yaw of 180 degrees reads column i at column i + 4 or i - 4.
        let v = values(&[("yaw", "180")]);
        let map = source_map(&v, (8, 4), (8, 4), 0.5);
        for column in 0..8u32 {
            let (x, y) = map(f64::from(column) + 0.5, 1.5).unwrap();
            let read = (column + 4) % 8;
            assert!((x - (f64::from(read) + 0.5)).abs() < 1e-9, "{column}: {x}");
            assert!((y - 1.5).abs() < 1e-9, "{column}: {y}");
        }
    }

    #[test]
    fn the_last_half_pixel_of_the_seam_lies_before_the_first_column() {
        // 8 columns of 45 degrees. A yaw of a quarter column makes output column 0 read a
        // quarter column beside the centre of source column 0: on one side for one sign of the
        // angle, on the other side for the other. The side before column 0 is the last half
        // pixel of the seam; the stage answers it as -0.25 (0.25 with the resampler offset),
        // not as 7.75.
        let read = |yaw: &str| {
            let v = values(&[("yaw", yaw)]);
            source_map(&v, (8, 4), (8, 4), 0.5)(0.5, 1.5).unwrap().0
        };
        let mut both = [read("-11.25"), read("11.25")];
        both.sort_by(f64::total_cmp);
        assert!((both[0] - 0.25).abs() < 1e-9, "{both:?}");
        assert!((both[1] - 0.75).abs() < 1e-9, "{both:?}");
    }

    #[test]
    fn each_stereo_view_reads_its_own_view() {
        let v = values(&[("stereo", "side-by-side"), ("yaw", "180")]);
        let map = source_map(&v, (8, 2), (8, 2), 0.5);
        // each view is 4 columns: a half turn moves a column by 2 inside its view
        for (column, read) in [
            (0, 2),
            (1, 3),
            (2, 0),
            (3, 1),
            (4, 6),
            (5, 7),
            (6, 4),
            (7, 5),
        ] {
            let (x, _) = map(f64::from(column) + 0.5, 0.5).unwrap();
            assert!((x - (f64::from(read) + 0.5)).abs() < 1e-9, "{column}: {x}");
        }
        let v = values(&[("stereo", "over-under"), ("pitch", "180"), ("roll", "180")]);
        let map = source_map(&v, (4, 4), (4, 4), 0.5);
        let (_, y) = map(0.5, 0.5).unwrap();
        assert!(y < 2.0, "the first view reads the first view: {y}");
        let (_, y) = map(0.5, 2.5).unwrap();
        assert!(y >= 2.0, "the second view reads the second view: {y}");
    }

    #[test]
    fn the_definition_holds_the_shared_parameters_first() {
        assert_eq!(&DEF.params[..SHARED], &sampler::SHARED_PARAMS[..]);
        assert_eq!(DEF.params.len(), 28);
        let keyed: Vec<&str> = DEF
            .params
            .iter()
            .filter(|p| p.keyed)
            .map(|p| p.id)
            .collect();
        assert_eq!(keyed, ["yaw", "pitch", "roll"]);
    }
}
