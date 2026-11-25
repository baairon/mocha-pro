//! Rotation matrices. Elementary rotations follow the usual right handed definitions (Strang,
//! Introduction to Linear Algebra, section 2.6): about x `[[1,0,0],[0,c,-s],[0,s,c]]`, about y
//! `[[c,0,s],[0,1,0],[-s,0,c]]`, about z `[[c,-s,0],[s,c,0],[0,0,1]]`.
//! Vectors are `[x, y, z]` with y up and z the direction of longitude zero latitude zero.

use crate::config::{AngleSign, AxisOrder, Config, RotationUse};
use crate::trig::{sin_cos_deg, sin_cos_turns};

/// An angle given by its cosine and sine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Angle {
    pub cos: f64,
    pub sin: f64,
}

impl Angle {
    pub const ZERO: Angle = Angle { cos: 1.0, sin: 0.0 };

    pub fn new(cos: f64, sin: f64) -> Angle {
        Angle { cos, sin }
    }

    pub fn from_turns(t: f64) -> Angle {
        let (sin, cos) = sin_cos_turns(t);
        Angle { cos, sin }
    }

    pub fn from_degrees(d: f64) -> Angle {
        let (sin, cos) = sin_cos_deg(d);
        Angle { cos, sin }
    }
}

/// A 3 by 3 matrix, row major.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat3(pub [[f64; 3]; 3]);

impl Mat3 {
    pub const IDENTITY: Mat3 = Mat3([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

    /// The product `self * other`: `other` acts on a vector first.
    pub fn mul(&self, other: &Mat3) -> Mat3 {
        let mut m = [[0.0; 3]; 3];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                let a = self.0[i][0] * other.0[0][j];
                let b = self.0[i][1] * other.0[1][j];
                let c = self.0[i][2] * other.0[2][j];
                *cell = (a + b) + c;
            }
        }
        Mat3(m)
    }

    pub fn transpose(&self) -> Mat3 {
        let a = &self.0;
        Mat3([
            [a[0][0], a[1][0], a[2][0]],
            [a[0][1], a[1][1], a[2][1]],
            [a[0][2], a[1][2], a[2][2]],
        ])
    }

    pub fn apply(&self, v: [f64; 3]) -> [f64; 3] {
        let mut o = [0.0; 3];
        for (i, cell) in o.iter_mut().enumerate() {
            let r = &self.0[i];
            *cell = (r[0] * v[0] + r[1] * v[1]) + r[2] * v[2];
        }
        o
    }

    pub fn det(&self) -> f64 {
        let m = &self.0;
        let a = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]);
        let b = m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]);
        let c = m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        (a - b) + c
    }

    /// Rotation about the x axis.
    pub fn about_x(a: Angle) -> Mat3 {
        Mat3([
            [1.0, 0.0, 0.0],
            [0.0, a.cos, 0.0 - a.sin],
            [0.0, a.sin, a.cos],
        ])
    }

    /// Rotation about the y axis.
    pub fn about_y(a: Angle) -> Mat3 {
        Mat3([
            [a.cos, 0.0, a.sin],
            [0.0, 1.0, 0.0],
            [0.0 - a.sin, 0.0, a.cos],
        ])
    }

    /// Rotation about the z axis.
    pub fn about_z(a: Angle) -> Mat3 {
        Mat3([
            [a.cos, 0.0 - a.sin, 0.0],
            [a.sin, a.cos, 0.0],
            [0.0, 0.0, 1.0],
        ])
    }

    /// Build the rotation from yaw (about y), pitch (about x) and roll (about z) with the axis order
    /// and the sign convention of `cfg`. The `rotation_use` field is not applied here.
    pub fn from_yaw_pitch_roll(cfg: &Config, yaw: Angle, pitch: Angle, roll: Angle) -> Mat3 {
        let fix = |a: Angle| match cfg.angle_sign {
            AngleSign::Positive => a,
            AngleSign::Negated => Angle {
                cos: a.cos,
                sin: 0.0 - a.sin,
            },
        };
        let (y, p, r) = (
            Mat3::about_y(fix(yaw)),
            Mat3::about_x(fix(pitch)),
            Mat3::about_z(fix(roll)),
        );
        let (a, b, c) = match cfg.axis_order {
            AxisOrder::YawPitchRoll => (y, p, r),
            AxisOrder::YawRollPitch => (y, r, p),
            AxisOrder::PitchYawRoll => (p, y, r),
            AxisOrder::PitchRollYaw => (p, r, y),
            AxisOrder::RollYawPitch => (r, y, p),
            AxisOrder::RollPitchYaw => (r, p, y),
        };
        a.mul(&b).mul(&c)
    }

    /// The matrix as used to map an output direction to a source direction under `cfg`.
    pub fn for_use(&self, cfg: &Config) -> Mat3 {
        match cfg.rotation_use {
            RotationUse::Direct => *self,
            RotationUse::Transposed => self.transpose(),
        }
    }
}
