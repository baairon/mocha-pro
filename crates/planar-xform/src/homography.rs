use crate::{Config, Convention, DivisorGuard, Point, Rect};

/// Why a transform could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The matrix or the correspondences are singular (within the tolerance).
    Singular,
    /// An input value is NaN or infinite.
    NonFinite,
}

/// A 3 by 3 matrix `m[row][col]` and the vector convention it is meant for.
/// Column: `[x' y' w']^T = M [x y 1]^T`. Row: `[x' y' w'] = [x y 1] M`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Homography {
    pub m: [[f64; 3]; 3],
    pub convention: Convention,
}

fn transpose(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut t = [[0.0; 3]; 3];
    for (r, row) in m.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            t[c][r] = *v;
        }
    }
    t
}

/// `a * b`, each entry summed left to right: `(a_i0 b_0j + a_i1 b_1j) + a_i2 b_2j`.
fn matmul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut c = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            c[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    c
}

/// `2^k` for `k` in `-1022..=1023`, exact.
fn pow2(k: i32) -> f64 {
    f64::from_bits(((k + 1023) as u64) << 52)
}

/// `v * 2^k` in two exact steps so that a large `|k|` does not leave the range of `pow2`.
fn mul_pow2(v: f64, k: i32) -> f64 {
    let half = k / 2;
    v * pow2(half) * pow2(k - half)
}

impl Homography {
    /// From a matrix in the stated convention (taken as is, no check).
    pub fn from_matrix(m: [[f64; 3]; 3], convention: Convention) -> Homography {
        Homography { m, convention }
    }

    fn from_column(m: [[f64; 3]; 3], convention: Convention) -> Homography {
        match convention {
            Convention::Column => Homography { m, convention },
            Convention::Row => Homography {
                m: transpose(&m),
                convention,
            },
        }
    }

    /// The matrix in the column convention, whatever this transform stores.
    pub fn column_matrix(&self) -> [[f64; 3]; 3] {
        match self.convention {
            Convention::Column => self.m,
            Convention::Row => transpose(&self.m),
        }
    }

    /// The same transform stored for another convention (a transpose when they differ).
    pub fn with_convention(&self, convention: Convention) -> Homography {
        Homography::from_column(self.column_matrix(), convention)
    }

    pub fn identity(convention: Convention) -> Homography {
        Homography::from_column(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            convention,
        )
    }

    pub fn translation(convention: Convention, tx: f64, ty: f64) -> Homography {
        Homography::from_column(
            [[1.0, 0.0, tx], [0.0, 1.0, ty], [0.0, 0.0, 1.0]],
            convention,
        )
    }

    pub fn scale(convention: Convention, sx: f64, sy: f64) -> Homography {
        Homography::from_column(
            [[sx, 0.0, 0.0], [0.0, sy, 0.0], [0.0, 0.0, 1.0]],
            convention,
        )
    }

    /// Rotation by `c, s` as given, with no normalisation: column form
    /// `[[c, -s, 0], [s, c, 0], [0, 0, 1]]` (H&Z section 2.4.1, isometries). With Y pointing
    /// up and `c, s` the cosine and sine of an angle it turns counter clockwise.
    pub fn rotation_cos_sin(convention: Convention, c: f64, s: f64) -> Homography {
        Homography::from_column([[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]], convention)
    }

    /// Rotation by `turns` quarter turns (any integer, taken modulo 4) with exact entries
    /// `0`, `1` and `-1`. One turn is `(c, s) = (0, 1)`.
    pub fn rotation_quarter_turns(convention: Convention, turns: i32) -> Homography {
        let (c, s) = match turns.rem_euclid(4) {
            0 => (1.0, 0.0),
            1 => (0.0, 1.0),
            2 => (-1.0, 0.0),
            _ => (0.0, -1.0),
        };
        Homography::rotation_cos_sin(convention, c, s)
    }

    /// Composition `self` first, then `next`. Column: `M = Mnext * Mself`. Row: `M = Mself * Mnext`.
    /// A `next` stored for the other convention is converted to the convention of `self`.
    pub fn then(&self, next: &Homography) -> Homography {
        let n = next.with_convention(self.convention);
        let m = match self.convention {
            Convention::Column => matmul(&n.m, &self.m),
            Convention::Row => matmul(&self.m, &n.m),
        };
        Homography {
            m,
            convention: self.convention,
        }
    }

    pub fn determinant(&self) -> f64 {
        let m = &self.m;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }

    /// The adjugate (transposed cofactor matrix), same convention.
    pub fn adjugate(&self) -> Homography {
        let m = &self.m;
        let a = [
            [
                m[1][1] * m[2][2] - m[1][2] * m[2][1],
                m[0][2] * m[2][1] - m[0][1] * m[2][2],
                m[0][1] * m[1][2] - m[0][2] * m[1][1],
            ],
            [
                m[1][2] * m[2][0] - m[1][0] * m[2][2],
                m[0][0] * m[2][2] - m[0][2] * m[2][0],
                m[0][2] * m[1][0] - m[0][0] * m[1][2],
            ],
            [
                m[1][0] * m[2][1] - m[1][1] * m[2][0],
                m[0][1] * m[2][0] - m[0][0] * m[2][1],
                m[0][0] * m[1][1] - m[0][1] * m[1][0],
            ],
        ];
        Homography {
            m: a,
            convention: self.convention,
        }
    }

    /// The inverse, adjugate divided by determinant entry by entry. `Singular` when the
    /// determinant is zero, `NonFinite` when it or an entry of the result is not finite.
    /// When the plain computation fails, it is retried once on the matrix scaled by an exact
    /// power of two (so that the largest entry is in `[1, 2)`), because the determinant of a
    /// matrix of very small or very large entries can underflow or overflow although the
    /// inverse is representable. A plain success is never changed.
    pub fn inverse(&self) -> Result<Homography, Error> {
        match self.inverse_plain() {
            Ok(h) => Ok(h),
            Err(first) => match self.inverse_scaled() {
                Some(Ok(h)) => Ok(h),
                Some(Err(Error::NonFinite)) => Err(Error::NonFinite),
                _ => Err(first),
            },
        }
    }

    fn inverse_plain(&self) -> Result<Homography, Error> {
        let det = self.determinant();
        if !det.is_finite() {
            return Err(Error::NonFinite);
        }
        if det == 0.0 {
            return Err(Error::Singular);
        }
        let mut a = self.adjugate();
        for row in a.m.iter_mut() {
            for v in row.iter_mut() {
                *v /= det;
            }
        }
        if a.m.iter().flatten().any(|v| !v.is_finite()) {
            return Err(Error::NonFinite);
        }
        Ok(a)
    }

    /// `None` when the scaling does not apply (an entry is not finite or all are zero).
    fn inverse_scaled(&self) -> Option<Result<Homography, Error>> {
        let mut mx = 0.0f64;
        for v in self.m.iter().flatten() {
            if !v.is_finite() {
                return None;
            }
            if v.abs() > mx {
                mx = v.abs();
            }
        }
        if mx == 0.0 {
            return None;
        }
        // Bring a subnormal maximum into the normal range first (exact), then read the exponent.
        let (norm, bias) = if mx < f64::MIN_POSITIVE {
            (mx * pow2(1022), 1022)
        } else {
            (mx, 0)
        };
        let e = (norm.to_bits() >> 52) as i32 - 1023 - bias;
        let mut scaled = *self;
        for v in scaled.m.iter_mut().flatten() {
            *v = mul_pow2(*v, -e);
        }
        Some(scaled.inverse_plain().and_then(|mut inv| {
            for v in inv.m.iter_mut().flatten() {
                *v = mul_pow2(*v, -e);
            }
            if inv.m.iter().flatten().any(|v| !v.is_finite()) {
                Err(Error::NonFinite)
            } else {
                Ok(inv)
            }
        }))
    }

    /// The homogeneous image `(x', y', w')` before the divide.
    pub fn apply_homogeneous(&self, p: Point) -> [f64; 3] {
        let m = &self.m;
        match self.convention {
            Convention::Column => [
                m[0][0] * p.x + m[0][1] * p.y + m[0][2],
                m[1][0] * p.x + m[1][1] * p.y + m[1][2],
                m[2][0] * p.x + m[2][1] * p.y + m[2][2],
            ],
            Convention::Row => [
                p.x * m[0][0] + p.y * m[1][0] + m[2][0],
                p.x * m[0][1] + p.y * m[1][1] + m[2][1],
                p.x * m[0][2] + p.y * m[1][2] + m[2][2],
            ],
        }
    }

    /// The image of a point with the homogeneous divide. The divisor is valid when it is
    /// greater than `cfg.divisor_min` (so a NaN divisor is invalid); otherwise `cfg.guard`
    /// decides. The convention used is that of `self`.
    pub fn apply(&self, p: Point, cfg: &Config) -> Option<Point> {
        let [x, y, w] = self.apply_homogeneous(p);
        if w > cfg.divisor_min {
            return Some(Point::new(x / w, y / w));
        }
        match cfg.guard {
            DivisorGuard::ReturnNone => None,
            DivisorGuard::Clamp(d) => Some(Point::new(x / d, y / d)),
            DivisorGuard::PassThrough => Some(Point::new(x, y)),
        }
    }

    /// The image quad of a rectangle, corners in the order of [`Rect::corners`].
    /// `None` when a corner has no image.
    pub fn map_rect(&self, r: &Rect, cfg: &Config) -> Option<[Point; 4]> {
        let c = r.corners();
        Some([
            self.apply(c[0], cfg)?,
            self.apply(c[1], cfg)?,
            self.apply(c[2], cfg)?,
            self.apply(c[3], cfg)?,
        ])
    }

    /// The bounding box of the image quad. It bounds the image of the rectangle only when the
    /// divisor stays positive over the whole rectangle (H&Z section 2.3: lines map to lines).
    pub fn map_rect_bounds(&self, r: &Rect, cfg: &Config) -> Option<Rect> {
        Rect::bounding(&self.map_rect(r, cfg)?)
    }

    /// The transform taking `src[i]` to `dst[i]` for four points, by the direct linear
    /// solution (H&Z section 4.1, algorithm 4.1 with four points and `h22 = 1`). The 8 by 8
    /// system, rows `[x y 1 0 0 0 -u x -u y | u]` and `[0 0 0 x y 1 -v x -v y | v]` for
    /// point `i` in rows `2i` and `2i+1`, is solved by Gaussian elimination with partial
    /// pivoting: the pivot of column `k` is the entry of largest magnitude among rows `k..8`,
    /// the lowest row winning a tie. A pivot of magnitude at most
    /// `cfg.singular_tolerance * (largest magnitude of the system)` gives `Singular`.
    /// Back substitution sums `a[i][j] x[j]` for `j` ascending. The result is stored for
    /// `cfg.convention`.
    ///
    /// Two refusals beyond the pivot test: the solved `h20 x + h21 y + 1` at a source point
    /// must not vanish within `cfg.singular_tolerance` of its terms (a solution that sends a
    /// source point to infinity, as when three source points are collinear and the
    /// destination points are not, is not a transform of the correspondences), and
    /// the solve is retried once on translated and scaled points when the first pass gives
    /// `Singular`, so that the answer does not depend on the origin or the unit of the
    /// coordinates (H&Z section 4.4, normalisation). The retry result is divided by `|h22|`
    /// (by its largest entry when `h22` is zero within the tolerance) and by the sign that makes
    /// the divisor positive at the centroid of the source points, so `h22` is `1`, `-1` or
    /// small. A first pass that
    /// succeeds is returned unchanged.
    pub fn from_four_points(
        src: &[Point; 4],
        dst: &[Point; 4],
        cfg: &Config,
    ) -> Result<Homography, Error> {
        match solve_h22_one(src, dst, cfg) {
            Ok(m) => Ok(Homography::from_column(m, cfg.convention)),
            Err(Error::Singular) => {
                solve_normalised(src, dst, cfg).map(|m| Homography::from_column(m, cfg.convention))
            }
            Err(e) => Err(e),
        }
    }
}

/// Centroid and a scale `1 / mean(max(|dx|, |dy|))` of four points; `None` when the points
/// coincide or the values are not finite.
fn centroid_scale(p: &[Point; 4]) -> Option<(f64, f64, f64)> {
    let cx = (p[0].x + p[1].x + p[2].x + p[3].x) / 4.0;
    let cy = (p[0].y + p[1].y + p[2].y + p[3].y) / 4.0;
    let mut sum = 0.0;
    for q in p {
        let (dx, dy) = ((q.x - cx).abs(), (q.y - cy).abs());
        sum += if dx > dy { dx } else { dy };
    }
    let mean = sum / 4.0;
    if mean.is_nan() || mean <= 0.0 || !mean.is_finite() || !cx.is_finite() || !cy.is_finite() {
        return None;
    }
    Some((cx, cy, 1.0 / mean))
}

fn solve_normalised(
    src: &[Point; 4],
    dst: &[Point; 4],
    cfg: &Config,
) -> Result<[[f64; 3]; 3], Error> {
    let (sx, sy, ss) = centroid_scale(src).ok_or(Error::Singular)?;
    let (dx, dy, ds) = centroid_scale(dst).ok_or(Error::Singular)?;
    if ss <= 0.0 || ds <= 0.0 || !ss.is_finite() || !ds.is_finite() {
        return Err(Error::Singular);
    }
    let norm = |p: &[Point; 4], cx: f64, cy: f64, s: f64| {
        p.map(|q| Point::new((q.x - cx) * s, (q.y - cy) * s))
    };
    let hn = solve_h22_one(&norm(src, sx, sy, ss), &norm(dst, dx, dy, ds), cfg)?;
    // H = Td^-1 * Hn * Ts with Ts = [[ss,0,-sx ss],[0,ss,-sy ss],[0,0,1]] and
    // Td^-1 = [[1/ds,0,dx],[0,1/ds,dy],[0,0,1]].
    let ts = [[ss, 0.0, -sx * ss], [0.0, ss, -sy * ss], [0.0, 0.0, 1.0]];
    let tdi = [[1.0 / ds, 0.0, dx], [0.0, 1.0 / ds, dy], [0.0, 0.0, 1.0]];
    let mut h = matmul(&tdi, &matmul(&hn, &ts));
    let mut big = 0.0f64;
    for v in h.iter().flatten() {
        if !v.is_finite() {
            return Err(Error::NonFinite);
        }
        if v.abs() > big {
            big = v.abs();
        }
    }
    let mag = if h[2][2].abs() > cfg.singular_tolerance * big {
        h[2][2].abs()
    } else {
        big
    };
    let pivot = if h[2][0] * sx + h[2][1] * sy + h[2][2] < 0.0 {
        -mag
    } else {
        mag
    };
    for v in h.iter_mut().flatten() {
        *v /= pivot;
    }
    if h.iter().flatten().any(|v| !v.is_finite()) {
        return Err(Error::NonFinite);
    }
    Ok(h)
}

/// The 8 by 8 solve with `h22 = 1`; the matrix is returned in the column convention.
fn solve_h22_one(src: &[Point; 4], dst: &[Point; 4], cfg: &Config) -> Result<[[f64; 3]; 3], Error> {
    {
        let mut a = [[0.0f64; 9]; 8];
        for i in 0..4 {
            let (x, y) = (src[i].x, src[i].y);
            let (u, v) = (dst[i].x, dst[i].y);
            a[2 * i] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
            a[2 * i + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
        }
        let mut scale = 0.0f64;
        for row in &a {
            for v in row {
                if !v.is_finite() {
                    return Err(Error::NonFinite);
                }
                if v.abs() > scale {
                    scale = v.abs();
                }
            }
        }
        let tol = cfg.singular_tolerance * scale;
        for k in 0..8 {
            let mut p = k;
            for r in (k + 1)..8 {
                if a[r][k].abs() > a[p][k].abs() {
                    p = r;
                }
            }
            if a[p][k].abs() <= tol || a[p][k].abs().is_nan() {
                return Err(Error::Singular);
            }
            a.swap(k, p);
            for r in (k + 1)..8 {
                let f = a[r][k] / a[k][k];
                let pivot_row = a[k];
                for (v, pv) in a[r][k..].iter_mut().zip(pivot_row[k..].iter()) {
                    *v -= f * *pv;
                }
            }
        }
        let mut h = [0.0f64; 8];
        for i in (0..8).rev() {
            let mut s = 0.0;
            for j in (i + 1)..8 {
                s += a[i][j] * h[j];
            }
            h[i] = (a[i][8] - s) / a[i][i];
        }
        if h.iter().any(|v| !v.is_finite()) {
            return Err(Error::NonFinite);
        }
        for q in src {
            let terms = (h[6] * q.x).abs() + (h[7] * q.y).abs() + 1.0;
            let w = h[6] * q.x + h[7] * q.y + 1.0;
            if w.abs() <= cfg.singular_tolerance * terms {
                return Err(Error::Singular);
            }
        }
        Ok([[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], 1.0]])
    }
}
