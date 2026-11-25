//! Exact squared Euclidean distance transform in integers.
//!
//! Source: P. F. Felzenszwalb and D. P. Huttenlocher, "Distance Transforms of Sampled Functions",
//! Theory of Computing 8(19), 2012, section 4 (the one dimensional lower envelope of parabolas,
//! applied to the columns and then to the rows, section 5). The intersections of parabolas are
//! computed as integer floor divisions, so all values are exact integers.

/// Marks a pixel for which the image holds no feature at all.
pub const NO_FEATURE: u64 = u64::MAX;

const INF: i64 = 1 << 50;
/// Largest side accepted, so that every sum stays far inside `i64`.
pub const MAX_SIDE: usize = 1 << 20;

/// One dimensional pass: `out[q] = min over p of (q - p)^2 + f[p]`.
fn pass_1d(f: &[i64], out: &mut [i64], v: &mut [usize], z: &mut [i64]) {
    let n = f.len();
    let mut k = 0usize;
    v[0] = 0;
    z[0] = i64::MIN;
    for q in 1..n {
        loop {
            let p = v[k];
            let num = (f[q] + (q * q) as i64) - (f[p] + (p * p) as i64);
            let den = 2 * (q as i64 - p as i64);
            let s = num.div_euclid(den);
            if s <= z[k] {
                // k is 0 only when z[0] is i64::MIN, which never pops.
                k -= 1;
            } else {
                k += 1;
                v[k] = q;
                z[k] = s;
                break;
            }
        }
    }
    let mut j = 0usize;
    for (q, o) in out.iter_mut().enumerate() {
        while j < k && z[j + 1] < q as i64 {
            j += 1;
        }
        let d = q as i64 - v[j] as i64;
        *o = d * d + f[v[j]];
    }
}

/// Squared distance from every pixel to the nearest pixel with `feature[i] == true`, in pixel
/// units squared; `NO_FEATURE` everywhere when no pixel is a feature. Row major, `w * h` entries.
/// Panics when the length differs from `w * h` or a side exceeds `MAX_SIDE`.
pub fn squared_distance_transform(feature: &[bool], w: usize, h: usize) -> Vec<u64> {
    assert_eq!(feature.len(), w * h, "feature length must be w * h");
    assert!(w <= MAX_SIDE && h <= MAX_SIDE, "side too large");
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let mut g: Vec<i64> = feature.iter().map(|&b| if b { 0 } else { INF }).collect();
    let m = w.max(h);
    let mut v = vec![0usize; m];
    let mut z = vec![0i64; m];
    let mut col = vec![0i64; h];
    let mut colout = vec![0i64; h];
    for x in 0..w {
        for y in 0..h {
            col[y] = g[y * w + x];
        }
        pass_1d(&col, &mut colout, &mut v, &mut z);
        for y in 0..h {
            g[y * w + x] = colout[y];
        }
    }
    let mut rowout = vec![0i64; w];
    for y in 0..h {
        pass_1d(&g[y * w..(y + 1) * w], &mut rowout, &mut v, &mut z);
        g[y * w..(y + 1) * w].copy_from_slice(&rowout);
    }
    g.iter()
        .map(|&d| if d >= INF { NO_FEATURE } else { d as u64 })
        .collect()
}
