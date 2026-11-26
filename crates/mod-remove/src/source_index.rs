//! Source index, windows, counts and reduction.

use crate::geom::*;
use crate::types::*;

/// source index value meaning "none".
pub const NO_SOURCE: u16 = 65535;
/// Fixed batch size: the kernels take ten matte and ten image slots.
pub const BATCH: usize = 10;

/// Stack of 16 bit planes over the fill window; `data[c * area + idx]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceIndex {
    pub win: Win,
    pub channels: usize,
    pub data: Vec<u16>,
}

impl SourceIndex {
    pub fn new(win: Win, channels: usize) -> SourceIndex {
        SourceIndex {
            win,
            channels,
            data: vec![NO_SOURCE; win.area() * channels],
        }
    }
    pub fn get(&self, c: usize, x: i32, y: i32) -> u16 {
        if self.win.contains(x, y) {
            self.data[c * self.win.area() + self.win.idx(x, y)]
        } else {
            NO_SOURCE
        }
    }
    pub fn set(&mut self, c: usize, x: i32, y: i32, v: u16) {
        let a = self.win.area();
        let i = c * a + self.win.idx(x, y);
        self.data[i] = v;
    }
    pub fn uses(&self, x: i32, y: i32, k: u16) -> bool {
        (0..self.channels).any(|c| self.get(c, x, y) == k)
    }
}

/// Update the source index for one batch. `base` is the global index of `batch[0]`.
pub fn index_map_update(
    sidx: &mut SourceIndex,
    fill: &Matte,
    batch: &[Source],
    base: usize,
    tc: &Camera,
) -> Res<()> {
    if base + batch.len() > NO_SOURCE as usize {
        return Err(RemoveError::Invalid("source index does not fit 16 bits"));
    }
    let w = sidx.win;
    for y in w.t..w.t + w.h {
        for x in w.l..w.l + w.w {
            if fill.get(x, y) == 0 {
                continue;
            }
            for (m, src) in batch.iter().enumerate() {
                let free = (0..sidx.channels).find(|&c| sidx.get(c, x, y) == NO_SOURCE);
                let Some(c) = free else { break };
                let (px, py) = transfer_px(tc, &src.cam, &src.h, x, y)?;
                if src.cover.win.contains(px, py) && src.cover.get(px, py) != 0 {
                    sidx.set(c, x, y, (base + m) as u16);
                }
            }
        }
    }
    Ok(())
}

/// Number of pixels to fill that still lack any source.
pub fn count_pending(sidx: &SourceIndex, fill: &Matte) -> usize {
    let w = sidx.win;
    let mut n = 0;
    for y in w.t..w.t + w.h {
        for x in w.l..w.l + w.w {
            if fill.get(x, y) != 0 && (0..sidx.channels).all(|c| sidx.get(c, x, y) == NO_SOURCE) {
                n += 1;
            }
        }
    }
    n
}

/// Per source count of the pixels to fill it can supply (integer, order free).
pub fn count_per_source(sidx: &SourceIndex, fill: &Matte, nsources: usize) -> Vec<usize> {
    let mut v = vec![0usize; nsources];
    let w = sidx.win;
    for y in w.t..w.t + w.h {
        for x in w.l..w.l + w.w {
            if fill.get(x, y) == 0 {
                continue;
            }
            for (k, slot) in v.iter_mut().enumerate() {
                if sidx.uses(x, y, k as u16) {
                    *slot += 1;
                }
            }
        }
    }
    v
}

/// Running index: a source gets the next number if its count is non zero, else -1.
pub fn compact_index(counts: &[usize]) -> Vec<i32> {
    let mut next = 0;
    counts
        .iter()
        .map(|&c| {
            if c != 0 {
                next += 1;
                next - 1
            } else {
                -1
            }
        })
        .collect()
}

/// Rewrite the source index with compacted numbers; an index mapped to -1 becomes 65535.
pub fn compact_map(sidx: &mut SourceIndex, map: &[i32]) {
    for v in sidx.data.iter_mut() {
        if *v != NO_SOURCE {
            let m = map.get(*v as usize).copied().unwrap_or(-1);
            *v = if m < 0 { NO_SOURCE } else { m as u16 };
        }
    }
}

/// Source window in the target frame grown from source index use: union over all pixels using
/// source `k` of the transferred corners of the square of half size `width`, grown by one.
pub fn source_region(
    sidx: &SourceIndex,
    k: u16,
    width: i32,
    tc: &Camera,
    src: &Source,
) -> Res<Option<Win>> {
    let mut acc: Option<Win> = None;
    let w = sidx.win;
    for y in w.t..w.t + w.h {
        for x in w.l..w.l + w.w {
            if !sidx.uses(x, y, k) {
                continue;
            }
            let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
            for (dx, dy) in [
                (-width, -width),
                (width, -width),
                (-width, width),
                (width, width),
            ] {
                let (a, b) = transfer_px(tc, &src.cam, &src.h, x + dx, y + dy)?;
                x0 = x0.min(a);
                y0 = y0.min(b);
                x1 = x1.max(a);
                y1 = y1.max(b);
            }
            let grown = Win::from_bounds(x0 - 1, y0 - 1, x1 + 1, y1 + 1);
            acc = Some(match acc {
                None => grown,
                Some(a) => a.union(&grown),
            });
        }
    }
    Ok(acc)
}

/// source index window variant: plain bounding box of the pixels that use source `k`, no growth.
pub fn index_window(sidx: &SourceIndex, k: u16) -> Option<Win> {
    let mut b: Option<(i32, i32, i32, i32)> = None;
    let w = sidx.win;
    for y in w.t..w.t + w.h {
        for x in w.l..w.l + w.w {
            if sidx.uses(x, y, k) {
                b = Some(match b {
                    None => (x, y, x, y),
                    Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x), e.max(y)),
                });
            }
        }
    }
    b.map(|(a, c, d, e)| Win::from_bounds(a, c, d, e))
}

/// Context window variant: the source index window grown by one pixel.
pub fn sampling_window(sidx: &SourceIndex, k: u16) -> Option<Win> {
    index_window(sidx, k).map(|w| w.grow(1))
}

/// Distortion map window variant: not described by the specification.
pub fn source_frame_window(_sidx: &SourceIndex, _k: u16) -> Res<Option<Win>> {
    Err(RemoveError::Unspecified(
        "formula of the distortion map window variant",
    ))
}
