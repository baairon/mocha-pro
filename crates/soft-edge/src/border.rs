//! Border rules for reading a sample outside a row or column of `n` samples.

/// How a read outside `0..n` is resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Border {
    /// Samples outside the buffer read as zero.
    Zero,
    /// The nearest edge sample is repeated.
    Clamp,
    /// Reflection about the edge of the first and last sample, so the edge sample is repeated once
    /// (index -1 reads index 0, index -2 reads index 1), with period `2n`.
    Mirror,
    /// Reflection about the centre of the edge sample (index -1 reads index 1), with period
    /// `2n - 2`; a single sample reads itself.
    MirrorNoRepeat,
}

impl Border {
    /// All rules, in a fixed order.
    pub const ALL: [Border; 4] = [
        Border::Zero,
        Border::Clamp,
        Border::Mirror,
        Border::MirrorNoRepeat,
    ];
}

/// Resolves index `i` against `n` samples. `None` means the read is zero (only for `Border::Zero`).
/// `n` must be at least 1.
pub fn resolve_index(i: i64, n: usize, border: Border) -> Option<usize> {
    let len = n as i64;
    if (0..len).contains(&i) {
        return Some(i as usize);
    }
    match border {
        Border::Zero => None,
        Border::Clamp => Some(i.clamp(0, len - 1) as usize),
        Border::Mirror => {
            let m = i.rem_euclid(2 * len);
            Some(if m < len { m } else { 2 * len - 1 - m } as usize)
        }
        Border::MirrorNoRepeat => {
            if len == 1 {
                return Some(0);
            }
            let p = 2 * len - 2;
            let m = i.rem_euclid(p);
            Some(if m < len { m } else { p - m } as usize)
        }
    }
}
