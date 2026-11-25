//! Border rules: how a source index outside `0..n` is mapped back into the image.

/// Rule for taps that fall outside the source image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Border {
    /// Outside samples are zero in every channel, alpha included (transparent).
    Zero,
    /// The nearest edge sample is repeated.
    Clamp,
    /// The image repeats with period `n`.
    Repeat,
    /// Reflection that repeats the edge sample: `... 1 0 | 0 1 2 | 2 1 ...` (period `2n`).
    Mirror,
    /// Reflection that does not repeat the edge sample: `... 2 1 | 0 1 2 | 1 0 ...`
    /// (period `2n - 2`; a one sample axis maps to index 0).
    MirrorNoEdge,
}

/// Map index `i` on an axis of length `n` (greater than zero) to an in range index, or `None`
/// when the rule is [`Border::Zero`] and `i` is outside.
pub fn wrap(i: i64, n: usize, rule: Border) -> Option<usize> {
    let len = n as i64;
    if (0..len).contains(&i) {
        return Some(i as usize);
    }
    match rule {
        Border::Zero => None,
        Border::Clamp => Some(if i < 0 { 0 } else { n - 1 }),
        Border::Repeat => Some(i.rem_euclid(len) as usize),
        Border::Mirror => {
            let m = i.rem_euclid(2 * len);
            Some(if m >= len { 2 * len - 1 - m } else { m } as usize)
        }
        Border::MirrorNoEdge => {
            if len == 1 {
                return Some(0);
            }
            let m = i.rem_euclid(2 * len - 2);
            Some(if m >= len { 2 * len - 2 - m } else { m } as usize)
        }
    }
}
