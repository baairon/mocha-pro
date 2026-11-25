//! Keys and key lists.

use crate::config::{Coincident, Config};

/// The interpolation kind of a key; it governs the segment that starts at the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The value of the key is held until the next key.
    Hold,
    /// Straight line to the next key.
    Linear,
    /// Cubic Hermite from the out slope of this key to the in slope of the next key (Farin, Curves and
    /// Surfaces for CAGD, 5th ed., 2002, the chapter on cubic Hermite interpolation). Slopes are value
    /// per time unit.
    Hermite,
    /// Cubic Bezier in the (time, value) plane from the out handle of this key to the in handle of the
    /// next key (Farin 2002, the chapter on Bezier curves).
    Bezier,
}

/// One key of `N` components.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key<const N: usize> {
    /// Key time.
    pub time: f64,
    /// Key value.
    pub value: [f64; N],
    /// Kind of the segment after this key.
    pub kind: Kind,
    /// Hermite in slope per component (used by the segment before this key when that key is Hermite).
    pub in_slope: [f64; N],
    /// Hermite out slope per component.
    pub out_slope: [f64; N],
    /// Bezier in handle per component as `[time, value]` (see `HandleMode`).
    pub in_handle: [[f64; 2]; N],
    /// Bezier out handle per component as `[time, value]` (see `HandleMode`).
    pub out_handle: [[f64; 2]; N],
}

impl<const N: usize> Key<N> {
    fn base(time: f64, value: [f64; N], kind: Kind) -> Self {
        Key {
            time,
            value,
            kind,
            in_slope: [0.0; N],
            out_slope: [0.0; N],
            in_handle: [[0.0; 2]; N],
            out_handle: [[0.0; 2]; N],
        }
    }

    /// A hold key.
    pub fn hold(time: f64, value: [f64; N]) -> Self {
        Key::base(time, value, Kind::Hold)
    }

    /// A linear key.
    pub fn linear(time: f64, value: [f64; N]) -> Self {
        Key::base(time, value, Kind::Linear)
    }

    /// A Hermite key with in and out slopes.
    pub fn hermite(time: f64, value: [f64; N], in_slope: [f64; N], out_slope: [f64; N]) -> Self {
        let mut k = Key::base(time, value, Kind::Hermite);
        k.in_slope = in_slope;
        k.out_slope = out_slope;
        k
    }

    /// A Bezier key with in and out handles, each `[time, value]` per component.
    pub fn bezier(
        time: f64,
        value: [f64; N],
        in_handle: [[f64; 2]; N],
        out_handle: [[f64; 2]; N],
    ) -> Self {
        let mut k = Key::base(time, value, Kind::Bezier);
        k.in_handle = in_handle;
        k.out_handle = out_handle;
        k
    }

    fn all_finite(&self) -> bool {
        self.time.is_finite()
            && self.value.iter().all(|v| v.is_finite())
            && self.in_slope.iter().all(|v| v.is_finite())
            && self.out_slope.iter().all(|v| v.is_finite())
            && self.in_handle.iter().flatten().all(|v| v.is_finite())
            && self.out_handle.iter().flatten().all(|v| v.is_finite())
    }
}

/// Why a key list was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyError {
    /// The key at this input index holds a NaN or infinite field.
    NonFinite(usize),
}

/// A normalized key list: sorted by time (stable), equal times resolved by the coincident rule.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyList<const N: usize> {
    pub(crate) keys: Vec<Key<N>>,
}

impl<const N: usize> KeyList<N> {
    /// Sorts the keys by time with a stable sort and applies `cfg.coincident`. Refuses non finite fields.
    pub fn new(keys: Vec<Key<N>>, cfg: &Config) -> Result<Self, KeyError> {
        if let Some(i) = keys.iter().position(|k| !k.all_finite()) {
            return Err(KeyError::NonFinite(i));
        }
        let mut keys = keys;
        keys.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let keys = match cfg.coincident {
            Coincident::Jump => keys,
            Coincident::KeepFirst => {
                let mut out: Vec<Key<N>> = Vec::with_capacity(keys.len());
                for k in keys {
                    if out.last().is_none_or(|l| l.time != k.time) {
                        out.push(k);
                    }
                }
                out
            }
            Coincident::KeepLast => {
                let mut out: Vec<Key<N>> = Vec::with_capacity(keys.len());
                for k in keys {
                    match out.last_mut() {
                        Some(l) if l.time == k.time => *l = k,
                        _ => out.push(k),
                    }
                }
                out
            }
        };
        Ok(KeyList { keys })
    }

    /// The normalized keys.
    pub fn keys(&self) -> &[Key<N>] {
        &self.keys
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// True when there is no key.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}
