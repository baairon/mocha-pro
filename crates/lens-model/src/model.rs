//! Coordinate maps on normalised coordinates.
//!
//! The polynomial form follows Brown (1966, 1971) for the radial part and the OpenCV ordering of the
//! two tangential terms: Brown's own P1 multiplies `r2 + 2*x*x` in x' and P2 multiplies `2*x*y`, so
//! `p[0]` here is Brown's P2 and `p[1]` is Brown's P1. With `r2 = x*x + y*y` and
//! `g = 1 + k1*r2 + k2*r2^2 + k3*r2^3`,
//! `x' = x*g + 2*p1*x*y + p2*(r2 + 2*x*x)` and `y' = y*g + p1*(r2 + 2*y*y) + 2*p2*x*y`.
//! The division form follows Fitzgibbon (2001, section 2): `q = p / (1 + lambda*|p|^2)`.
//! The squeezed form is defined here: `r2 = (sx*x)^2 + (sy*y)^2` and `q = p * g(r2)`.

/// A distortion model. Each variant is a map `p -> q` on normalised coordinates; which of the
/// two images (distorted or undistorted) is `p` is chosen by [`crate::ModelMaps`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Model {
    /// Brown polynomial: radial terms `k` (k1, k2, k3) and tangential terms `p` (p1, p2).
    Radial { k: [f64; 3], p: [f64; 2] },
    /// Fitzgibbon division model with one parameter.
    Division { lambda: f64 },
    /// Radial polynomial on a radius computed from axis-squeezed coordinates.
    Squeezed { k: [f64; 3], squeeze: [f64; 2] },
}

/// How the inverse of the model map is computed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InverseMethod {
    /// `u <- u - (F(u) - d)` from `u = d`, a fixed number of steps.
    FixedPoint,
    /// Newton iteration with the analytic 2x2 Jacobian from `u = d`, a fixed number of steps. A step
    /// whose Jacobian determinant is zero or not finite leaves the iterate unchanged; a step whose
    /// residual is not finite is an error, as for the fixed point method.
    Newton,
    /// Closed form; only the division model has one.
    ClosedForm,
}

/// Failure of a map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LensError {
    /// Closed form requested for a model without one.
    NoClosedForm,
    /// The point lies outside the domain of the map (a zero denominator, or a negative square root argument).
    OutsideDomain,
    /// The result is not a finite number.
    NonFinite,
    /// The configuration is unusable (a non positive size, scale or aspect, or a non finite value).
    BadConfig,
}

type Jac = [[f64; 2]; 2];

impl Model {
    /// The radial model with every coefficient zero, which is the identity.
    pub fn identity() -> Model {
        Model::Radial {
            k: [0.0; 3],
            p: [0.0; 2],
        }
    }

    /// Applies the model map to a normalised point.
    pub fn apply(&self, pt: [f64; 2]) -> Result<[f64; 2], LensError> {
        finite(self.apply_jac(pt)?.0)
    }

    /// Inverts the model map with `method`; `iterations` is the fixed step count of the iterative
    /// methods and is ignored by the closed form.
    pub fn invert(
        &self,
        target: [f64; 2],
        method: InverseMethod,
        iterations: u32,
    ) -> Result<[f64; 2], LensError> {
        match method {
            InverseMethod::ClosedForm => self.closed_form(target),
            InverseMethod::FixedPoint => {
                let mut u = target;
                for _ in 0..iterations {
                    let f = self.apply_jac(u)?.0;
                    u = [u[0] - (f[0] - target[0]), u[1] - (f[1] - target[1])];
                }
                finite(u)
            }
            InverseMethod::Newton => {
                let mut u = target;
                for _ in 0..iterations {
                    let (f, j) = self.apply_jac(u)?;
                    if !(f[0].is_finite() && f[1].is_finite()) {
                        return Err(LensError::NonFinite);
                    }
                    let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
                    if det == 0.0 || !det.is_finite() {
                        continue;
                    }
                    let rx = f[0] - target[0];
                    let ry = f[1] - target[1];
                    let dx = (j[1][1] * rx - j[0][1] * ry) / det;
                    let dy = (j[0][0] * ry - j[1][0] * rx) / det;
                    u = [u[0] - dx, u[1] - dy];
                }
                finite(u)
            }
        }
    }

    /// The closed form inverse, for the division model: `p = 2*q / (1 + sqrt(1 - 4*lambda*|q|^2))`,
    /// which solves `|q| = |p| / (1 + lambda*|p|^2)` for `|p|` taking the root that tends to `q`
    /// as `lambda` tends to zero (Fitzgibbon 2001, section 2, solved for the radius).
    pub fn closed_form(&self, q: [f64; 2]) -> Result<[f64; 2], LensError> {
        match *self {
            Model::Division { lambda } => {
                let disc = 1.0 - 4.0 * lambda * (q[0] * q[0] + q[1] * q[1]);
                if disc < 0.0 {
                    return Err(LensError::OutsideDomain);
                }
                let den = 1.0 + disc.sqrt();
                finite([2.0 * q[0] / den, 2.0 * q[1] / den])
            }
            _ => Err(LensError::NoClosedForm),
        }
    }

    fn apply_jac(&self, pt: [f64; 2]) -> Result<([f64; 2], Jac), LensError> {
        match *self {
            Model::Radial { k, p } => Ok(poly(k, p, [1.0, 1.0], pt)),
            Model::Squeezed { k, squeeze } => Ok(poly(k, [0.0, 0.0], squeeze, pt)),
            Model::Division { lambda } => {
                let (x, y) = (pt[0], pt[1]);
                let d = 1.0 + lambda * (x * x + y * y);
                if d == 0.0 {
                    return Err(LensError::OutsideDomain);
                }
                let d2 = d * d;
                let l2 = 2.0 * lambda;
                let j = [
                    [(d - l2 * x * x) / d2, -(l2 * x * y) / d2],
                    [-(l2 * x * y) / d2, (d - l2 * y * y) / d2],
                ];
                Ok(([x / d, y / d], j))
            }
        }
    }
}

fn finite(v: [f64; 2]) -> Result<[f64; 2], LensError> {
    if v[0].is_finite() && v[1].is_finite() {
        Ok(v)
    } else {
        Err(LensError::NonFinite)
    }
}

/// Polynomial map and analytic Jacobian. `s` is the per axis squeeze (1 for the Brown model).
fn poly(k: [f64; 3], p: [f64; 2], s: [f64; 2], pt: [f64; 2]) -> ([f64; 2], Jac) {
    let (x, y) = (pt[0], pt[1]);
    let (xs, ys) = (s[0] * x, s[1] * y);
    let r2 = xs * xs + ys * ys;
    let g = 1.0 + r2 * (k[0] + r2 * (k[1] + r2 * k[2]));
    let gp = k[0] + r2 * (2.0 * k[1] + r2 * (3.0 * k[2]));
    let tx = 2.0 * p[0] * x * y + p[1] * (r2 + 2.0 * x * x);
    let ty = p[0] * (r2 + 2.0 * y * y) + 2.0 * p[1] * x * y;
    let drx = 2.0 * s[0] * xs;
    let dry = 2.0 * s[1] * ys;
    let jxx = g + x * gp * drx + 2.0 * p[0] * y + 6.0 * p[1] * x;
    let jxy = x * gp * dry + 2.0 * p[0] * x + 2.0 * p[1] * y;
    let jyx = y * gp * drx + 2.0 * p[0] * x + 2.0 * p[1] * y;
    let jyy = g + y * gp * dry + 6.0 * p[0] * y + 2.0 * p[1] * x;
    ([x * g + tx, y * g + ty], [[jxx, jxy], [jyx, jyy]])
}
