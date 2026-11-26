//! Mix mattes: init, grow, mask out, scatter.

use crate::config::*;
use crate::geom::*;
use crate::source_index::*;
use crate::types::*;

/// Fill a mix matte with a constant.
pub fn mix_fill(m: &mut Matte, v: u8) {
    for p in m.data.iter_mut() {
        *p = v;
    }
}

/// Initialise: 255 where the source index channel equals this source, 0 elsewhere inside the window.
pub fn matte_seed(m: &mut Matte, sidx: &SourceIndex, channel: usize, k: u16) {
    let w = m.win;
    for y in w.t..w.t + w.h {
        for x in w.l..w.l + w.w {
            let v = if sidx.get(channel, x, y) == k { 255 } else { 0 };
            m.set(x, y, v);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Down,
    Up,
}

/// One grow step.
pub fn matte_grow_step(
    m: &mut Matte,
    sidx: &SourceIndex,
    channel: usize,
    beacon: u8,
    newv: u8,
    dir: Dir,
) {
    let w = m.win;
    let src = m.clone();
    for y in w.t..w.t + w.h {
        for x in w.l..w.l + w.w {
            if !sidx.win.contains(x, y) || sidx.get(channel, x, y) == NO_SOURCE {
                continue;
            }
            let cur = m.get(x, y);
            let beyond = match dir {
                Dir::Up => cur > beacon,
                Dir::Down => cur < beacon,
            };
            if !beyond {
                continue;
            }
            let mut hit = false;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if !w.contains(nx, ny) || !sidx.win.contains(nx, ny) {
                        continue;
                    }
                    if sidx.get(channel, nx, ny) == NO_SOURCE {
                        continue;
                    }
                    if src.get(nx, ny) == beacon {
                        hit = true;
                    }
                }
            }
            if hit {
                m.set(x, y, newv);
            }
        }
    }
}

/// Host driven ramp of width `width` in one direction.
pub fn mix_ramp(m: &mut Matte, sidx: &SourceIndex, channel: usize, width: u32, dir: Dir) {
    let delta = ((255 / width.max(1)) as i32).max(1);
    let mut beacon: i32 = match dir {
        Dir::Down => 255,
        Dir::Up => 0,
    };
    for _ in 0..width.saturating_sub(1) {
        let raw = match dir {
            Dir::Down => beacon - delta,
            Dir::Up => beacon + delta,
        };
        let newv = raw.clamp(0, 255);
        if newv != beacon {
            matte_grow_step(m, sidx, channel, beacon as u8, newv as u8, dir);
        }
        beacon = newv;
    }
}

/// Mask out: zero where the transfer falls outside the source matte window or onto a zero in the
/// source mask.
pub fn matte_mask_out(m: &mut Matte, tc: &Camera, src: &Source) -> Res<()> {
    let w = m.win;
    for y in w.t..w.t + w.h {
        for x in w.l..w.l + w.w {
            if m.get(x, y) == 0 {
                continue;
            }
            let (px, py) = transfer_px(tc, &src.cam, &src.h, x, y)?;
            if !src.cover.win.contains(px, py) || src.cover.get(px, py) == 0 {
                m.set(x, y, 0);
            }
        }
    }
    Ok(())
}

/// Scatter across the mattes of a batch: d = x + 10000 y + 100000000 seed (f32),
/// r = integer part of |1234567 sin d| modulo the total; keep the source whose running total
/// interval contains r.
pub fn matte_scatter(mats: &mut [Matte], area: Win, seed: f32, sin: MathFn) {
    for y in area.t..area.t + area.h {
        for x in area.l..area.l + area.w {
            let total: u32 = mats.iter().map(|m| m.get(x, y) as u32).sum();
            if total == 0 {
                continue;
            }
            let d: f32 = x as f32 + 10000.0f32 * y as f32 + 100000000.0f32 * seed;
            let r = ((1234567.0f32 * sin(d)).abs() as u32) % total;
            let mut cum = 0u32;
            let mut kept = false;
            for m in mats.iter_mut() {
                let v = m.get(x, y) as u32;
                if !kept && v != 0 && r < cum + v {
                    kept = true;
                } else {
                    m.set(x, y, 0);
                }
                cum += v;
            }
        }
    }
}

/// Build the mix matte of one source for one source index channel, in the order the specification
/// names: fill, init, grow (per `ramp_phases`), mask out. Scatter runs across the batch afterwards.
pub fn build_mix_matte(
    win: Win,
    sidx: &SourceIndex,
    channel: usize,
    k: u16,
    tc: &Camera,
    src: &Source,
    r: &FillConfig,
) -> Res<Matte> {
    let mut m = Matte::new(win, 0);
    mix_fill(&mut m, 0);
    matte_seed(&mut m, sidx, channel, k);
    match r.ramp_phases {
        RampPhases::DownOnly => mix_ramp(&mut m, sidx, channel, r.ramp_len, Dir::Down),
        RampPhases::UpOnly => mix_ramp(&mut m, sidx, channel, r.ramp_len, Dir::Up),
        RampPhases::DownThenUp => {
            mix_ramp(&mut m, sidx, channel, r.ramp_len, Dir::Down);
            mix_ramp(&mut m, sidx, channel, r.ramp_len, Dir::Up);
        }
    }
    matte_mask_out(&mut m, tc, src)?;
    Ok(m)
}

/// The mix window is the source window mapped back to the target; the specification gives no
/// formula.
pub fn mix_window_from_source_window(_src_win: Win) -> Res<Win> {
    Err(RemoveError::Unspecified(
        "source window mapped back to the target image",
    ))
}
