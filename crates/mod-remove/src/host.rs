//! Host side policy: preconditions, matte expansion, candidates, composite, and the stages the
//! specification does not give.

use crate::config::*;
use crate::fill::*;
use crate::geom::round_px;
use crate::illum::IllumSet;
use crate::types::*;

/// Preconditions of the entry point and the command, in the order the specification gives.
#[derive(Clone, Copy, Debug)]
pub struct Preconditions {
    pub tracked_frames: u32,
    pub tracked_in_range: bool,
    pub target_enabled: bool,
    pub lower_layer_ready: bool,
    pub in_stream_ok: bool,
    pub out_stream_ok: bool,
    pub out_stream_found: bool,
    pub out_stream_locked: bool,
    pub output_equals_input: bool,
    pub plates_exclusive: bool,
    pub has_plate_clip: bool,
    pub range_ok: bool,
    pub something_to_render: bool,
}

impl Preconditions {
    pub fn all_good() -> Preconditions {
        Preconditions {
            tracked_frames: 2,
            tracked_in_range: true,
            target_enabled: true,
            lower_layer_ready: true,
            in_stream_ok: true,
            out_stream_ok: true,
            out_stream_found: true,
            out_stream_locked: false,
            output_equals_input: false,
            plates_exclusive: false,
            has_plate_clip: false,
            range_ok: true,
            something_to_render: true,
        }
    }
}

pub fn check_preconditions(p: &Preconditions) -> Res<()> {
    let e = |c: Precondition| Err(RemoveError::Precondition(c));
    if p.tracked_frames <= 1 {
        return e(Precondition::TooFewTrackedFrames);
    }
    if !p.tracked_in_range {
        return e(Precondition::CoverageMissing);
    }
    if !p.target_enabled {
        return e(Precondition::LayerInactive);
    }
    if !p.lower_layer_ready {
        return e(Precondition::LowerLayerMissing);
    }
    if !p.in_stream_ok {
        return e(Precondition::InStreamMissing);
    }
    if !p.out_stream_ok {
        return e(Precondition::OutStreamUndeclared);
    }
    if !p.out_stream_found {
        return e(Precondition::OutStreamAbsent);
    }
    if p.out_stream_locked {
        return e(Precondition::OutStreamLocked);
    }
    if p.output_equals_input {
        return e(Precondition::OutStreamAliasesIn);
    }
    if p.plates_exclusive && !p.has_plate_clip {
        return e(Precondition::PlateClipMissing);
    }
    if !p.range_ok {
        return e(Precondition::RangeNotMet);
    }
    if !p.something_to_render {
        return e(Precondition::EmptyRegion);
    }
    Ok(())
}

/// Expand a binary matte by `amount` pixels and give it the bounding window of
/// the expanded matte. Values: 255 where set, else 0.
pub fn widen_mask(m: &Matte, amount: i32, shape: ExpandShape) -> Res<Matte> {
    let grown = m.win.grow(amount);
    let mut g = Matte::new(grown, 0);
    for y in grown.t..grown.t + grown.h {
        for x in grown.l..grown.l + grown.w {
            let mut hit = false;
            for dy in -amount..=amount {
                for dx in -amount..=amount {
                    if shape == ExpandShape::Cross && dx != 0 && dy != 0 {
                        continue;
                    }
                    if m.get(x + dx, y + dy) != 0 {
                        hit = true;
                    }
                }
            }
            g.set(x, y, if hit { 255 } else { 0 });
        }
    }
    let b = g
        .nonzero_bounds()
        .ok_or(RemoveError::Precondition(Precondition::EmptyRegion))?;
    let mut out = Matte::new(b, 0);
    for y in b.t..b.t + b.h {
        for x in b.l..b.l + b.w {
            out.set(x, y, g.get(x, y));
        }
    }
    Ok(out)
}

/// Candidate source frame times (INFERRED): t + j*step for j in -before..=after,
/// j nonzero, within the available times, ordered by distance.
pub fn candidate_times(
    t: i32,
    before: i32,
    after: i32,
    step: i32,
    range: (i32, i32),
    r: &FillConfig,
) -> Res<Vec<i32>> {
    if step <= 0 {
        return Err(RemoveError::Unspecified("automatic search step"));
    }
    let mut v: Vec<(i32, i32)> = vec![];
    for j in -before..=after {
        if j == 0 {
            continue;
        }
        let ft = t + j * step;
        if ft >= range.0 && ft <= range.1 {
            v.push((j.abs(), ft));
        }
    }
    v.sort_by(|a, b| {
        a.0.cmp(&b.0).then(match r.tie_break {
            TieBreak::EarlierFirst => a.1.cmp(&b.1),
            TieBreak::LaterFirst => b.1.cmp(&a.1),
        })
    });
    Ok(v.into_iter().map(|p| p.1).collect())
}

/// Composite of the fill over the frame (INFERRED).
pub fn composite(frame: &Image, synth: &SynthResult, region: &Matte, r: &FillConfig) -> Res<Image> {
    if r.composite == CompositeMode::Unspecified {
        return Err(RemoveError::Unspecified("composite over the frame"));
    }
    let mut out = frame.clone();
    let fw = synth.image.win;
    for y in fw.t..fw.t + fw.h {
        for x in fw.l..fw.l + fw.w {
            let mv = region.get(x, y);
            if mv == 0 || !frame.win.contains(x, y) {
                continue;
            }
            let i = fw.idx(x, y);
            let f = synth.image.data[i];
            if !synth.filled[i] {
                match r.zero_weight {
                    ZeroWeightPolicy::Unspecified => {
                        return Err(RemoveError::Unspecified(
                            "composite of a pixel with no fill (zero weight)",
                        ))
                    }
                    ZeroWeightPolicy::KeepFrame => continue,
                    ZeroWeightPolicy::FillOnly => {}
                }
            }
            let m = mv as f32 / 255.0f32;
            let o = frame.at(x, y);
            let mut p = [0.0f32; 4];
            for k in 0..4 {
                p[k] = f[k] * m + o[k] * (1.0f32 - m);
            }
            out.set(x, y, p);
        }
    }
    Ok(out)
}

/// Inputs of one frame render.
pub struct FrameInputs<'a> {
    pub pre: Preconditions,
    pub frame: &'a Image,
    /// Binary matte of the layer at the frame time.
    pub layer_matte: &'a Matte,
    pub tc: Camera,
    pub sources: &'a [Source],
    pub illum: &'a IllumSet,
    pub layer_filter: Option<u32>,
    pub spread: bool,
    /// Carry-over rendering mode, taken from the caller's configuration.
    pub carry_over_mode: bool,
}

impl<'a> FrameInputs<'a> {
    /// A made up sample of the caller's frame inputs for tests, examples and docs. Every value
    /// here is a sample chosen for this crate; the carry-over value is [`SAMPLE_CARRY_OVER`].
    /// A caller that needs another value sets the field itself.
    pub fn sample(
        frame: &'a Image,
        layer_matte: &'a Matte,
        sources: &'a [Source],
        illum: &'a IllumSet,
    ) -> FrameInputs<'a> {
        FrameInputs {
            pre: Preconditions::all_good(),
            frame,
            layer_matte,
            tc: Camera::identity(),
            sources,
            illum,
            layer_filter: None,
            spread: false,
            carry_over_mode: SAMPLE_CARRY_OVER,
        }
    }
}

/// The carry-over value of [`FrameInputs::sample`]: a made up sample, not a default of any type.
pub const SAMPLE_CARRY_OVER: bool = false;

/// Frame level driver: preconditions, expand, fill, composite with the expanded matte.
pub fn render_frame(inp: &FrameInputs, r: &FillConfig) -> Res<Image> {
    check_preconditions(&inp.pre)?;
    if inp.carry_over_mode {
        return Err(RemoveError::Unspecified("carry-over rendering mode"));
    }
    let fill_matte = widen_mask(inp.layer_matte, 1, r.expand_shape)?;
    let fr = run_fill(
        &FillInputs {
            fill: &fill_matte,
            tc: inp.tc,
            sources: inp.sources,
            illum: inp.illum,
            layer_filter: inp.layer_filter,
            separate_weight: false,
            spread: inp.spread,
        },
        r,
    )?;
    composite(inp.frame, &fr, &fill_matte, r)
}

/// Stages the specification does not give, as named errors.
pub fn fill_kind_selector(_kind: u8) -> Res<()> {
    Err(RemoveError::Unspecified(
        "meaning of the fill kind selector values",
    ))
}
pub fn tracks_to_homographies() -> Res<Vec<[f32; 9]>> {
    Err(RemoveError::Unspecified(
        "tracks to homographies and the two layer logic",
    ))
}
pub fn plate_join() -> Res<Vec<Source>> {
    Err(RemoveError::Unspecified(
        "how plates join the candidate set, plate combination",
    ))
}
pub fn derive_source_region() -> Res<Matte> {
    Err(RemoveError::Unspecified("derivation of the source masks"))
}
pub fn colour_range_and_premultiplication() -> Res<()> {
    Err(RemoveError::Unspecified(
        "colour range and premultiplication of the input images",
    ))
}
pub fn soften_interior_stage() -> Res<()> {
    Err(RemoveError::Unspecified(
        "softening mode, softening width, interior toggle",
    ))
}
/// Rounding helper exposed for tests of the specification rounding rule.
pub fn round_half_away(v: f32) -> i32 {
    round_px(v)
}
