//! Accumulate, normalise and the per frame fill driver.

use crate::config::*;
use crate::geom::*;
use crate::illum::*;
use crate::mix::*;
use crate::source_index::*;
use crate::spread::diffuse_gaps;
use crate::types::*;

/// Accumulator over the fill window; `weight` is the optional separate weight buffer whose
/// fourth component receives w.
#[derive(Clone, Debug, PartialEq)]
pub struct Accum {
    pub win: Win,
    pub color: Vec<[f32; 4]>,
    pub weight: Option<Vec<[f32; 4]>>,
}

impl Accum {
    pub fn new(win: Win, separate_weight: bool) -> Accum {
        Accum {
            win,
            color: vec![[0.0; 4]; win.area()],
            weight: if separate_weight {
                Some(vec![[0.0; 4]; win.area()])
            } else {
                None
            },
        }
    }
    /// The weight sum of a pixel (fourth component of the weight buffer or of the colour).
    pub fn weight_at(&self, i: usize) -> f32 {
        match &self.weight {
            Some(w) => w[i][3],
            None => self.color[i][3],
        }
    }
}

/// One source of a batch with its mix matte, as the kernel sees it.
pub struct SourceMix<'a> {
    pub k: u16,
    pub orig: usize,
    pub src: &'a Source,
    pub mix: Option<&'a Matte>,
    pub mix_win: Win,
}

/// Sample position and mix value for pixel (x, y), or None for no sample.
pub fn pixel_sample(
    tc: &Camera,
    sidx: &SourceIndex,
    channel: usize,
    sb: &SourceMix,
    x: i32,
    y: i32,
    layer_filter: Option<u32>,
) -> Res<Option<(f32, f32, u8)>> {
    if let Some(l) = layer_filter {
        if l > 0 && sb.src.layer_id != l {
            return Ok(None);
        }
    }
    let p = sidx.get(channel, x, y);
    if p == NO_SOURCE || !sb.mix_win.contains(x, y) {
        return Ok(None);
    }
    let b = match sb.mix {
        Some(m) => m.get(x, y),
        None => {
            if p == sb.k {
                255
            } else {
                0
            }
        }
    };
    if b == 0 {
        return Ok(None);
    }
    let (qx, qy) = sample_position(tc, sb.src, x, y)?;
    Ok(Some((qx, qy, b)))
}

/// Inputs shared by the accumulate stage.
pub struct AccumCtx<'a> {
    pub fill: &'a Matte,
    pub sidx: &'a SourceIndex,
    pub tc: Camera,
    pub illum: &'a IllumSet,
    pub layer_filter: Option<u32>,
}

/// Accumulate one batch for one source index channel into `acc`.
pub fn accumulate_batch(
    acc: &mut Accum,
    cx: &AccumCtx,
    channel: usize,
    batch: &[SourceMix],
    r: &FillConfig,
) -> Res<()> {
    let w = acc.win;
    for y in w.t..w.t + w.h {
        for x in w.l..w.l + w.w {
            if cx.fill.get(x, y) == 0 {
                continue;
            }
            let i = w.idx(x, y);
            let mut local = [0.0f32; 4];
            let mut local_w = 0.0f32;
            for sb in batch {
                let Some((qx, qy, b)) =
                    pixel_sample(&cx.tc, cx.sidx, channel, sb, x, y, cx.layer_filter)?
                else {
                    continue;
                };
                let c0 = (r.sampler)(&sb.src.pixels, qx, qy);
                let c = apply_illum(cx.illum, sb.orig, c0, x, y, r)?;
                let wt = (sb.src.weight * b as f32) / 255.0f32;
                let contrib = if acc.weight.is_some() {
                    [c[0] * wt, c[1] * wt, c[2] * wt, c[3] * wt]
                } else {
                    [c[0] * wt, c[1] * wt, c[2] * wt, wt]
                };
                if r.batch_local_sum {
                    for k in 0..4 {
                        local[k] += contrib[k];
                    }
                    local_w += wt;
                } else {
                    for k in 0..4 {
                        acc.color[i][k] += contrib[k];
                    }
                    if let Some(wb) = acc.weight.as_mut() {
                        wb[i][3] += wt;
                    }
                }
            }
            if r.batch_local_sum {
                for k in 0..4 {
                    acc.color[i][k] += local[k];
                }
                if let Some(wb) = acc.weight.as_mut() {
                    wb[i][3] += local_w;
                }
            }
        }
    }
    Ok(())
}

/// Normalise.
pub fn normalise(acc: &mut Accum, preserve: bool) {
    let n = acc.color.len();
    for i in 0..n {
        let w = acc.weight_at(i);
        if w != 0.0 {
            for c in acc.color[i].iter_mut() {
                *c /= w;
            }
            if preserve && acc.weight.is_none() {
                acc.color[i][3] = w;
            }
            if !preserve {
                if let Some(wb) = acc.weight.as_mut() {
                    wb[i][3] = 1.0;
                }
            }
        } else {
            acc.color[i] = [0.0; 4];
        }
    }
}

/// Inputs of one fill.
pub struct FillInputs<'a> {
    pub fill: &'a Matte,
    pub tc: Camera,
    pub sources: &'a [Source],
    pub illum: &'a IllumSet,
    pub layer_filter: Option<u32>,
    pub separate_weight: bool,
    pub spread: bool,
}

#[derive(Clone, Debug)]
pub struct SynthResult {
    pub image: Image,
    /// Weight sum per pixel before normalisation.
    pub weights: Vec<f32>,
    /// Pixels holding a fill (weight non zero or spreaded).
    pub filled: Vec<bool>,
}

/// The synthesis for one layer over its window: source index over all batches, compaction, per batch and channel
/// mix mattes then accumulate, normalise, optional spread pass. The order of
/// loops over batches and channels is an implementation choice.
pub fn run_fill(inp: &FillInputs, r: &FillConfig) -> Res<SynthResult> {
    let win = inp.fill.win;
    let mut sidx = SourceIndex::new(win, r.index_channels.max(1));
    for (batch_no, chunk) in inp.sources.chunks(BATCH).enumerate() {
        index_map_update(&mut sidx, inp.fill, chunk, batch_no * BATCH, &inp.tc)?;
    }
    let counts = count_per_source(&sidx, inp.fill, inp.sources.len());
    let map = compact_index(&counts);
    compact_map(&mut sidx, &map);
    let kept: Vec<usize> = (0..inp.sources.len()).filter(|&i| map[i] >= 0).collect();
    let mut acc = Accum::new(win, inp.separate_weight);
    let cx = AccumCtx {
        fill: inp.fill,
        sidx: &sidx,
        tc: inp.tc,
        illum: inp.illum,
        layer_filter: inp.layer_filter,
    };
    for (batch_no, chunk) in kept.chunks(BATCH).enumerate() {
        for channel in 0..sidx.channels {
            let mut mats = vec![];
            for (m, &orig) in chunk.iter().enumerate() {
                let k = (batch_no * BATCH + m) as u16;
                mats.push(build_mix_matte(
                    win,
                    &sidx,
                    channel,
                    k,
                    &inp.tc,
                    &inp.sources[orig],
                    r,
                )?);
            }
            if r.scatter {
                matte_scatter(&mut mats, win, r.seed, r.sin);
            }
            let batch: Vec<SourceMix> = chunk
                .iter()
                .enumerate()
                .map(|(m, &orig)| SourceMix {
                    k: (batch_no * BATCH + m) as u16,
                    orig,
                    src: &inp.sources[orig],
                    mix: Some(&mats[m]),
                    mix_win: win,
                })
                .collect();
            accumulate_batch(&mut acc, &cx, channel, &batch, r)?;
        }
    }
    let weights: Vec<f32> = (0..win.area()).map(|i| acc.weight_at(i)).collect();
    let mut filled: Vec<bool> = weights.iter().map(|&w| w != 0.0).collect();
    normalise(&mut acc, r.preserve_weight);
    let mut image = Image {
        win,
        data: acc.color,
    };
    if inp.spread {
        diffuse_gaps(&mut image, inp.fill, &mut filled, r)?;
    }
    Ok(SynthResult {
        image,
        weights,
        filled,
    })
}
