//! The configuration space. Every choice a later calibration may need to vary is a field of [`Config`].
//! The defaults are one arbitrary point of the space and claim nothing about any product.

/// Direction in which longitude grows along the image columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LonDirection {
    /// Longitude grows toward larger column numbers.
    IncreasingRight,
    /// Longitude grows toward smaller column numbers.
    IncreasingLeft,
}

/// Where the sample of a pixel sits horizontally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelCenter {
    /// Pixel `i` is sampled at `(i + 0.5) / width` of a turn: the image columns tile the circle.
    Centre,
    /// Pixel `i` is sampled at `i / width` of a turn: column 0 sits exactly on the column origin.
    Corner,
}

/// Where the first and last rows sit vertically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoleRows {
    /// Row 0 is on the pole and row `height - 1` on the opposite pole: `v = y / (height - 1)`.
    OnPole,
    /// The rows tile the latitude range: `v = (y + 0.5) / height`.
    Inset,
}

/// Which pole is at the first row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NorthEdge {
    /// Latitude +0.25 turn at row 0.
    Top,
    /// Latitude -0.25 turn at row 0.
    Bottom,
}

/// Order in which the elementary rotations are multiplied: `R = R_first * R_second * R_third`, so the
/// third named rotation acts on a vector first. Yaw turns about the up axis (y), pitch about the x
/// axis and roll about the z axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisOrder {
    YawPitchRoll,
    YawRollPitch,
    PitchYawRoll,
    PitchRollYaw,
    RollYawPitch,
    RollPitchYaw,
}

impl AxisOrder {
    pub const ALL: [AxisOrder; 6] = [
        AxisOrder::YawPitchRoll,
        AxisOrder::YawRollPitch,
        AxisOrder::PitchYawRoll,
        AxisOrder::PitchRollYaw,
        AxisOrder::RollYawPitch,
        AxisOrder::RollPitchYaw,
    ];
}

/// Sign convention of the angles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngleSign {
    /// A positive angle turns counter clockwise seen from the positive end of the axis.
    Positive,
    /// Every angle is negated before use.
    Negated,
}

/// Which way the matrix is used when mapping an output direction to a source direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationUse {
    /// The source direction is `R d`.
    Direct,
    /// The source direction is `R^T d`.
    Transposed,
}

/// All conventions of the equirectangular map and of the rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// Position of longitude zero along the width as a fraction of the width in `[0, 1)`.
    pub lon_zero: f64,
    pub lon_direction: LonDirection,
    pub pixel_center: PixelCenter,
    pub pole_rows: PoleRows,
    pub north: NorthEdge,
    pub axis_order: AxisOrder,
    pub angle_sign: AngleSign,
    pub rotation_use: RotationUse,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            lon_zero: 0.5,
            lon_direction: LonDirection::IncreasingRight,
            pixel_center: PixelCenter::Centre,
            pole_rows: PoleRows::Inset,
            north: NorthEdge::Top,
            axis_order: AxisOrder::YawPitchRoll,
            angle_sign: AngleSign::Positive,
            rotation_use: RotationUse::Direct,
        }
    }
}

impl Config {
    /// The values of `lon_zero` in the enumerated space.
    pub const LON_ZERO_SET: [f64; 4] = [0.0, 0.25, 0.5, 0.75];

    /// Number of configurations of [`Config::all`]: 4 * 2 * 2 * 2 * 2 * 6 * 2 * 2.
    pub const COUNT: usize = 1536;

    /// Every configuration of the space, with `lon_zero` taken from [`Config::LON_ZERO_SET`], in a
    /// fixed order (the first field varies slowest).
    pub fn all() -> Vec<Config> {
        let mut out = Vec::with_capacity(Self::COUNT);
        for &lon_zero in &Self::LON_ZERO_SET {
            for lon_direction in [LonDirection::IncreasingRight, LonDirection::IncreasingLeft] {
                for pixel_center in [PixelCenter::Centre, PixelCenter::Corner] {
                    for pole_rows in [PoleRows::OnPole, PoleRows::Inset] {
                        for north in [NorthEdge::Top, NorthEdge::Bottom] {
                            for axis_order in AxisOrder::ALL {
                                for angle_sign in [AngleSign::Positive, AngleSign::Negated] {
                                    for rotation_use in
                                        [RotationUse::Direct, RotationUse::Transposed]
                                    {
                                        out.push(Config {
                                            lon_zero,
                                            lon_direction,
                                            pixel_center,
                                            pole_rows,
                                            north,
                                            axis_order,
                                            angle_sign,
                                            rotation_use,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }
}
