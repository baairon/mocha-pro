//! Euclidean distance transform in linear time.
//!
//! Felzenszwalb and Huttenlocher, "Distance Transforms of Sampled Functions", Theory of
//! Computing 8(19), 2012: the one dimensional squared distance transform as the lower
//! envelope of parabolas (linear time) and the two dimensional
//! transform as the one dimensional transform along columns and then along rows.

/// A value large enough to stand for infinity and still square without overflow.
const INF: f64 = 1e20;

/// One dimensional squared distance transform of `f` (Felzenszwalb and Huttenlocher 2012,
/// the one dimensional algorithm): `d[q] = min over p of ((q - p)^2 + f[p])`.
pub fn dt1d(f: &[f64]) -> Vec<f64> {
    let n = f.len();
    let mut d = vec![0.0; n];
    if n == 0 {
        return d;
    }
    let mut v = vec![0usize; n];
    let mut z = vec![0.0; n + 1];
    let mut k = 0usize;
    z[0] = -INF;
    z[1] = INF;
    for q in 1..n {
        let qf = q as f64;
        let mut s;
        loop {
            let pf = v[k] as f64;
            s = ((f[q] + qf * qf) - (f[v[k]] + pf * pf)) / (2.0 * qf - 2.0 * pf);
            if s <= z[k] && k > 0 {
                k -= 1;
            } else {
                break;
            }
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = INF;
    }
    k = 0;
    for (q, out) in d.iter_mut().enumerate() {
        let qf = q as f64;
        while z[k + 1] < qf {
            k += 1;
        }
        let diff = qf - v[k] as f64;
        *out = diff * diff + f[v[k]];
    }
    d
}

/// Distance, in pixels, from every pixel to the nearest pixel that is not set in `mask`
/// (`width * height`, row major). A pixel not in the mask has distance 0; a pixel in the mask has
/// the Euclidean distance (an IEEE square root of the exact squared distance) to the nearest
/// pixel outside it. When `border_is_outside` is true the pixels just beyond the image edge count
/// as outside, so the distance of an edge pixel is at most 1. When no outside pixel exists the
/// distance is a large finite number.
pub fn distance_to_outside(
    mask: &[bool],
    width: usize,
    height: usize,
    border_is_outside: bool,
) -> Vec<f64> {
    let (w, h) = if border_is_outside {
        (width + 2, height + 2)
    } else {
        (width, height)
    };
    let off = usize::from(border_is_outside);
    let mut grid = vec![0.0; w * h];
    for y in 0..height {
        for x in 0..width {
            grid[(y + off) * w + x + off] = if mask[y * width + x] { INF } else { 0.0 };
        }
    }
    let mut col = vec![0.0; h];
    for x in 0..w {
        for y in 0..h {
            col[y] = grid[y * w + x];
        }
        let d = dt1d(&col);
        for y in 0..h {
            grid[y * w + x] = d[y];
        }
    }
    for y in 0..h {
        let d = dt1d(&grid[y * w..(y + 1) * w]);
        grid[y * w..(y + 1) * w].copy_from_slice(&d);
    }
    let mut out = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            out.push(grid[(y + off) * w + x + off].sqrt());
        }
    }
    out
}
