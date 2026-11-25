//! Small dense linear algebra on `f64`: 3 by 3 matrices and a Cholesky solver. Only IEEE
//! addition, subtraction, multiplication, division and square root are used, always in the
//! fixed order written here, so results are identical on every platform.

/// A row major 3 by 3 matrix.
pub type Mat3 = [f64; 9];

/// The identity matrix.
pub const IDENTITY: Mat3 = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

/// Matrix product `a * b`.
pub fn mul3(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut r = [0.0; 9];
    for i in 0..3 {
        for j in 0..3 {
            let mut s = 0.0;
            for k in 0..3 {
                s += a[i * 3 + k] * b[k * 3 + j];
            }
            r[i * 3 + j] = s;
        }
    }
    r
}

/// Inverse by the adjugate over the determinant; `None` when the determinant is zero or not finite.
pub fn inv3(m: &Mat3) -> Option<Mat3> {
    let c00 = m[4] * m[8] - m[5] * m[7];
    let c01 = m[5] * m[6] - m[3] * m[8];
    let c02 = m[3] * m[7] - m[4] * m[6];
    let det = m[0] * c00 + m[1] * c01 + m[2] * c02;
    if det == 0.0 || !det.is_finite() {
        return None;
    }
    let c10 = m[2] * m[7] - m[1] * m[8];
    let c11 = m[0] * m[8] - m[2] * m[6];
    let c12 = m[1] * m[6] - m[0] * m[7];
    let c20 = m[1] * m[5] - m[2] * m[4];
    let c21 = m[2] * m[3] - m[0] * m[5];
    let c22 = m[0] * m[4] - m[1] * m[3];
    let d = 1.0 / det;
    Some([
        c00 * d,
        c10 * d,
        c20 * d,
        c01 * d,
        c11 * d,
        c21 * d,
        c02 * d,
        c12 * d,
        c22 * d,
    ])
}

/// Apply a homography to a point; `None` when the homogeneous divisor is not above `1e-9`.
pub fn apply3(m: &Mat3, x: f64, y: f64) -> Option<(f64, f64)> {
    let w = m[6] * x + m[7] * y + m[8];
    if w.is_nan() || w <= 1e-9 {
        return None;
    }
    Some((
        (m[0] * x + m[1] * y + m[2]) / w,
        (m[3] * x + m[4] * y + m[5]) / w,
    ))
}

/// Solve `a x = b` for a symmetric positive definite `n` by `n` matrix `a` (row major, `n * n`
/// entries) by Cholesky factorisation. `None` when a pivot is not positive or not finite.
pub fn solve_spd(a: &[f64], b: &[f64], n: usize) -> Option<Vec<f64>> {
    let mut l = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..=i {
            let mut s = a[i * n + j];
            for k in 0..j {
                s -= l[i * n + k] * l[j * n + k];
            }
            if i == j {
                if s.is_nan() || s <= 0.0 {
                    return None;
                }
                l[i * n + i] = s.sqrt();
            } else {
                l[i * n + j] = s / l[j * n + j];
            }
        }
    }
    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut s = b[i];
        for k in 0..i {
            s -= l[i * n + k] * y[k];
        }
        y[i] = s / l[i * n + i];
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = y[i];
        for k in (i + 1)..n {
            s -= l[k * n + i] * x[k];
        }
        x[i] = s / l[i * n + i];
    }
    if x.iter().all(|v| v.is_finite()) {
        Some(x)
    } else {
        None
    }
}
