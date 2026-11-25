//! How a one channel coverage plane is carried in a four channel frame.
//!
//! A rasterisation stage computes one coverage value per pixel, from 0 to 1. The frame contract
//! has four channels, so the stage has to say where the value goes. Two parameters say it, and
//! this module is the one place that applies them:
//!
//! - `apply` gives the matte value `m` of a pixel from its coverage `c` and the alpha `a` that
//!   the input frame has there: `replace` is `c`, `multiply` is `a * c`, `max` is the larger of
//!   `a` and `c` (and `c` when `a` is not a number).
//! - `carry` says which channels hold `m`: `alpha` writes it to the alpha channel and leaves the
//!   three colour channels as they were read; `luma` writes it to the three colour channels and
//!   sets alpha to full scale; `all` writes it to the four channels.
//!
//! The arithmetic is in `f32` on the samples of [`crate::pixels`]: an integer sample is its
//! value divided by full scale, and the result is clamped and rounded half up on the way out.
//! Depth 32 is not clamped.

use crate::params::Values;
use crate::pixels::{Layout, ALPHA, CHANNELS};
use crate::registry::{ParamDef, ParamKind};

/// Which channels hold the matte value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carry {
    /// The alpha channel; colour passes.
    Alpha,
    /// The three colour channels; alpha is full scale.
    Luma,
    /// The four channels.
    All,
}

/// How the coverage meets the alpha of the input frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Apply {
    /// The coverage alone.
    Replace,
    /// The alpha of the input times the coverage.
    Multiply,
    /// The larger of the alpha of the input and the coverage.
    Max,
}

/// The parameter `carry`.
pub const CARRY: ParamDef = ParamDef {
    id: "carry",
    name: "Carry",
    kind: ParamKind::Choice {
        default: "alpha",
        choices: &["alpha", "luma", "all"],
    },
    keyed: false,
    about: "Which channels hold the matte: alpha (colour passes), luma (the three colour channels, alpha at full scale) or all four.",
};

/// The parameter `apply`.
pub const APPLY: ParamDef = ParamDef {
    id: "apply",
    name: "Apply",
    kind: ParamKind::Choice {
        default: "replace",
        choices: &["replace", "multiply", "max"],
    },
    keyed: false,
    about: "How the coverage meets the alpha of the input frame: it replaces it, multiplies it or the larger of the two is kept.",
};

/// The value of `carry` in a set of values.
pub fn carry(values: &Values) -> Carry {
    match values.choice(CARRY.id) {
        "luma" => Carry::Luma,
        "all" => Carry::All,
        _ => Carry::Alpha,
    }
}

/// The value of `apply` in a set of values.
pub fn apply(values: &Values) -> Apply {
    match values.choice(APPLY.id) {
        "multiply" => Apply::Multiply,
        "max" => Apply::Max,
        _ => Apply::Replace,
    }
}

/// The matte value of one pixel from its coverage and the alpha of the input frame.
pub fn meet(apply: Apply, coverage: f32, alpha: f32) -> f32 {
    match apply {
        Apply::Replace => coverage,
        Apply::Multiply => alpha * coverage,
        Apply::Max => alpha.max(coverage),
    }
}

/// Write a coverage plane into the decoded samples of one frame.
///
/// `samples` holds four channels per pixel in `layout`, and `coverage` one value per pixel, row
/// by row. Both layouts keep alpha at channel index 3, so the three other indexes are the colour
/// channels in either one.
pub fn write(samples: &mut [f32], layout: Layout, coverage: &[f32], carry: Carry, apply: Apply) {
    let pixels = coverage.len();
    debug_assert_eq!(samples.len(), pixels * CHANNELS);
    let index = |channel: usize, pixel: usize| match layout {
        Layout::Packed => pixel * CHANNELS + channel,
        Layout::Planar => channel * pixels + pixel,
    };
    for (pixel, &c) in coverage.iter().enumerate() {
        let m = meet(apply, c, samples[index(ALPHA, pixel)]);
        match carry {
            Carry::Alpha => samples[index(ALPHA, pixel)] = m,
            Carry::Luma => {
                for channel in 0..ALPHA {
                    samples[index(channel, pixel)] = m;
                }
                samples[index(ALPHA, pixel)] = 1.0;
            }
            Carry::All => {
                for channel in 0..CHANNELS {
                    samples[index(channel, pixel)] = m;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_rules_of_apply() {
        assert_eq!(meet(Apply::Replace, 0.25, 0.5), 0.25);
        assert_eq!(meet(Apply::Multiply, 0.25, 0.5), 0.125);
        assert_eq!(meet(Apply::Max, 0.25, 0.5), 0.5);
        assert_eq!(meet(Apply::Max, 0.75, 0.5), 0.75);
        assert_eq!(meet(Apply::Max, 0.75, f32::NAN), 0.75);
    }

    #[test]
    fn alpha_keeps_colour_in_a_packed_frame() {
        // two pixels, R G B A each
        let mut samples = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        write(
            &mut samples,
            Layout::Packed,
            &[1.0, 0.25],
            Carry::Alpha,
            Apply::Replace,
        );
        assert_eq!(samples, [0.1, 0.2, 0.3, 1.0, 0.5, 0.6, 0.7, 0.25]);
    }

    #[test]
    fn alpha_keeps_colour_in_a_planar_frame() {
        // two pixels, the planes G, B, R, A
        let mut samples = [0.1, 0.5, 0.2, 0.6, 0.3, 0.7, 0.4, 0.8];
        write(
            &mut samples,
            Layout::Planar,
            &[1.0, 0.25],
            Carry::Alpha,
            Apply::Replace,
        );
        assert_eq!(samples, [0.1, 0.5, 0.2, 0.6, 0.3, 0.7, 1.0, 0.25]);
    }

    #[test]
    fn luma_fills_colour_and_sets_alpha_to_full_scale() {
        let mut packed = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        write(
            &mut packed,
            Layout::Packed,
            &[0.5, 0.0],
            Carry::Luma,
            Apply::Replace,
        );
        assert_eq!(packed, [0.5, 0.5, 0.5, 1.0, 0.0, 0.0, 0.0, 1.0]);
        let mut planar = [0.1, 0.5, 0.2, 0.6, 0.3, 0.7, 0.4, 0.8];
        write(
            &mut planar,
            Layout::Planar,
            &[0.5, 0.0],
            Carry::Luma,
            Apply::Replace,
        );
        assert_eq!(planar, [0.5, 0.0, 0.5, 0.0, 0.5, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn all_fills_the_four_channels() {
        let mut samples = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        write(
            &mut samples,
            Layout::Packed,
            &[0.5, 0.75],
            Carry::All,
            Apply::Replace,
        );
        assert_eq!(samples, [0.5, 0.5, 0.5, 0.5, 0.75, 0.75, 0.75, 0.75]);
    }

    #[test]
    fn apply_reads_the_alpha_of_the_input_under_every_carry() {
        for carry in [Carry::Alpha, Carry::Luma, Carry::All] {
            let mut samples = [0.0, 0.0, 0.0, 0.5];
            write(&mut samples, Layout::Packed, &[0.5], carry, Apply::Multiply);
            let want = match carry {
                Carry::Alpha => [0.0, 0.0, 0.0, 0.25],
                Carry::Luma => [0.25, 0.25, 0.25, 1.0],
                Carry::All => [0.25, 0.25, 0.25, 0.25],
            };
            assert_eq!(samples, want, "{carry:?}");
        }
    }
}
