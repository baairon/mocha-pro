//! Region spread fallback.

use crate::config::*;
use crate::types::*;

/// Fill still unfilled pixels of `fill` by diffusion from the boundary. `filled` is indexed over
/// `img.win`; it is updated. Passes read the state at the start of the pass.
pub fn diffuse_gaps(
    img: &mut Image,
    target: &Matte,
    filled: &mut [bool],
    r: &FillConfig,
) -> Res<()> {
    if r.spread_level != 0 {
        return Err(RemoveError::Unspecified(
            "spread pass level other than the default",
        ));
    }
    let w = img.win;
    let todo = |x: i32, y: i32, f: &[bool]| target.get(x, y) != 0 && !f[w.idx(x, y)];
    loop {
        let snap = filled.to_vec();
        let mut remaining = 0;
        let mut changes: Vec<(i32, i32, [f32; 4])> = vec![];
        for y in w.t..w.t + w.h {
            for x in w.l..w.l + w.w {
                if !todo(x, y, &snap) {
                    continue;
                }
                remaining += 1;
                let mut nb = false;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (x + dx, y + dy);
                        if w.contains(nx, ny) && snap[w.idx(nx, ny)] {
                            nb = true;
                        }
                    }
                }
                if !nb {
                    continue;
                }
                let mut sum = [0.0f32; 4];
                let mut n = 0u32;
                for dy in -2..=2 {
                    for dx in -2..=2 {
                        let (nx, ny) = (x + dx, y + dy);
                        if !w.contains(nx, ny) || !snap[w.idx(nx, ny)] {
                            continue;
                        }
                        let boundary = match r.spread_boundary {
                            SpreadBoundary::AnyFilled => true,
                            SpreadBoundary::FilledNextToUnfilled => {
                                let mut b = false;
                                for ey in -1..=1 {
                                    for ex in -1..=1 {
                                        let (mx, my) = (nx + ex, ny + ey);
                                        if w.contains(mx, my) && todo(mx, my, &snap) {
                                            b = true;
                                        }
                                    }
                                }
                                b
                            }
                        };
                        if boundary {
                            let c = img.at(nx, ny);
                            for k in 0..4 {
                                sum[k] += c[k];
                            }
                            n += 1;
                        }
                    }
                }
                if n > 0 {
                    let nf = n as f32;
                    changes.push((x, y, [sum[0] / nf, sum[1] / nf, sum[2] / nf, 1.0]));
                }
            }
        }
        if remaining == 0 {
            return Ok(());
        }
        if changes.is_empty() {
            return Err(RemoveError::Refused(
                "spread pass cannot proceed: no filled pixel next to the remaining area",
            ));
        }
        for (x, y, c) in changes {
            img.set(x, y, c);
            filled[w.idx(x, y)] = true;
        }
    }
}
