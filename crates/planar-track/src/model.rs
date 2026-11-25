//! The planar motion model: degrees of freedom, parameterisation and composition.
//!
//! A pose is a 3 by 3 homography `H` (row major, `h33 = 1` after [`Pose::normalised`]) that takes
//! a point of the template plane to a point of a frame: `x_frame = H x_template` in homogeneous
//! coordinates, pixel centre convention of [`crate::image`]. Composition is matrix product:
//! `a.compose(b)` applies `b` first and then `a` (`a * b`).
//!
//! Parameterisation (stated). Every model is a linear subspace of the eight numbers
//! `q = (tx, ty, a11, a12, a21, a22, g1, g2)`, which form the homography
//!
//! ```text
//! [ 1 + a11   a12     tx ]
//! [ a21       1 + a22 ty ]
//! [ g1        g2      1  ]
//! ```
//!
//! and the parameter vector `p` of a model maps to `q` by a fixed matrix `M`, `q = M p`, with the
//! identity at `p = 0`:
//!
//! * translation (2): `p = (tx, ty)`;
//! * translation and scale (3): `p = (tx, ty, s)`, `a11 = a22 = s`;
//! * similarity (4): `p = (tx, ty, a, b)`, `a11 = a22 = a`, `a21 = b`, `a12 = -b`;
//! * affine (6): `p = (tx, ty, a11, a12, a21, a22)`;
//! * homography (8): `p = q`.
//!
//! This is the parameterisation of Baker and Matthews, "Lucas-Kanade 20 Years On: A Unifying
//! Framework", IJCV 56(3), 2004, section 2.1 (the warp and its affine example) and its natural extension
//! to eight parameters, written so that the parameters are additive (needed by the forward
//! additive algorithm, section 2.2 of that paper). Similarity and scale models are subspaces of the
//! affine one and are closed under composition and inversion, which the compositional algorithms
//! (sections 3.1 and 3.2) require of the set of warps (their sections 3.1.3 and 3.2.3).

use crate::error::Error;
use crate::linalg::{apply3, inv3, mul3, Mat3, IDENTITY};

/// Degrees of freedom of the motion model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dof {
    /// Translation, 2 parameters.
    Translation,
    /// Translation and uniform scale, 3 parameters.
    TransScale,
    /// Translation, rotation and uniform scale, 4 parameters.
    Similarity,
    /// Affine, 6 parameters.
    Affine,
    /// Projective homography, 8 parameters.
    Homography,
}

impl Dof {
    /// All settings, in increasing number of parameters.
    pub const ALL: [Dof; 5] = [
        Dof::Translation,
        Dof::TransScale,
        Dof::Similarity,
        Dof::Affine,
        Dof::Homography,
    ];

    /// Number of parameters.
    pub fn count(self) -> usize {
        match self {
            Dof::Translation => 2,
            Dof::TransScale => 3,
            Dof::Similarity => 4,
            Dof::Affine => 6,
            Dof::Homography => 8,
        }
    }

    /// Short name.
    pub fn name(self) -> &'static str {
        match self {
            Dof::Translation => "translation",
            Dof::TransScale => "translation-scale",
            Dof::Similarity => "similarity",
            Dof::Affine => "affine",
            Dof::Homography => "homography",
        }
    }

    /// The matrix `M` as eight rows (one per entry of `q`) of `count()` columns, flattened: the
    /// entry for `q[j]` and parameter `k` is `m[j * count + k]`.
    pub fn matrix(self) -> Vec<f64> {
        let n = self.count();
        let mut m = vec![0.0; 8 * n];
        let mut set = |j: usize, k: usize, v: f64| m[j * n + k] = v;
        // q index: 0 tx, 1 ty, 2 a11, 3 a12, 4 a21, 5 a22, 6 g1, 7 g2.
        set(0, 0, 1.0);
        set(1, 1, 1.0);
        match self {
            Dof::Translation => {}
            Dof::TransScale => {
                set(2, 2, 1.0);
                set(5, 2, 1.0);
            }
            Dof::Similarity => {
                set(2, 2, 1.0);
                set(5, 2, 1.0);
                set(4, 3, 1.0);
                set(3, 3, -1.0);
            }
            Dof::Affine => {
                for k in 2..6 {
                    set(k, k, 1.0);
                }
            }
            Dof::Homography => {
                for k in 2..8 {
                    set(k, k, 1.0);
                }
            }
        }
        m
    }

    /// `q = M p`.
    pub fn expand(self, p: &[f64]) -> [f64; 8] {
        let n = self.count();
        let m = self.matrix();
        let mut q = [0.0; 8];
        for (j, qj) in q.iter_mut().enumerate() {
            let mut s = 0.0;
            for k in 0..n {
                s += m[j * n + k] * p[k];
            }
            *qj = s;
        }
        q
    }

    /// The least squares projection of `q` onto the model: `p = (M^T q) / diag(M^T M)` (the columns
    /// of `M` are orthogonal).
    pub fn project(self, q: &[f64; 8]) -> Vec<f64> {
        let n = self.count();
        let m = self.matrix();
        let mut p = vec![0.0; n];
        for (k, pk) in p.iter_mut().enumerate() {
            let mut num = 0.0;
            let mut den = 0.0;
            for j in 0..8 {
                num += m[j * n + k] * q[j];
                den += m[j * n + k] * m[j * n + k];
            }
            *pk = num / den;
        }
        p
    }
}

/// A homography pose, template plane to frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// Row major matrix.
    pub h: Mat3,
}

impl Pose {
    /// The identity pose.
    pub fn identity() -> Pose {
        Pose { h: IDENTITY }
    }

    /// A pure translation.
    pub fn translation(tx: f64, ty: f64) -> Pose {
        Pose {
            h: [1.0, 0.0, tx, 0.0, 1.0, ty, 0.0, 0.0, 1.0],
        }
    }

    /// The pose of a parameter vector of a model (see the module text).
    pub fn from_params(dof: Dof, p: &[f64]) -> Pose {
        Pose::from_q(&dof.expand(p))
    }

    /// The pose of the eight numbers `q`.
    pub fn from_q(q: &[f64; 8]) -> Pose {
        Pose {
            h: [
                1.0 + q[2],
                q[3],
                q[0],
                q[4],
                1.0 + q[5],
                q[1],
                q[6],
                q[7],
                1.0,
            ],
        }
    }

    /// The eight numbers `q` of this pose; refuses `h33 = 0`.
    pub fn to_q(&self) -> Result<[f64; 8], Error> {
        let n = self.normalised()?;
        let h = n.h;
        Ok([h[2], h[5], h[0] - 1.0, h[1], h[3], h[4] - 1.0, h[6], h[7]])
    }

    /// The parameters of this pose in a model (projection when the pose is not in the model).
    pub fn params(&self, dof: Dof) -> Result<Vec<f64>, Error> {
        Ok(dof.project(&self.to_q()?))
    }

    /// Scaled so that `h33 = 1`.
    pub fn normalised(&self) -> Result<Pose, Error> {
        let d = self.h[8];
        if d == 0.0 || !d.is_finite() {
            return Err(Error::Singular);
        }
        let mut h = self.h;
        for v in h.iter_mut() {
            *v /= d;
        }
        h[8] = 1.0;
        Ok(Pose { h })
    }

    /// `self * other`: `other` is applied first.
    pub fn compose(&self, other: &Pose) -> Pose {
        Pose {
            h: mul3(&self.h, &other.h),
        }
    }

    /// The inverse pose.
    pub fn inverse(&self) -> Result<Pose, Error> {
        inv3(&self.h).map(|h| Pose { h }).ok_or(Error::Singular)
    }

    /// Map a point; `None` when it goes to or behind the plane at infinity.
    pub fn apply(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        apply3(&self.h, x, y)
    }

    /// The largest Euclidean distance between the images of `points` under the two poses; `None`
    /// when a point cannot be mapped.
    pub fn max_point_distance(&self, other: &Pose, points: &[(f64, f64)]) -> Option<f64> {
        let mut worst = 0.0f64;
        for &(x, y) in points {
            let a = self.apply(x, y)?;
            let b = other.apply(x, y)?;
            let dx = a.0 - b.0;
            let dy = a.1 - b.1;
            let d = (dx * dx + dy * dy).sqrt();
            if d > worst {
                worst = d;
            }
        }
        Some(worst)
    }
}
