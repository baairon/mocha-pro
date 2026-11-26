//! Every point the specification leaves open, and every INFERRED reading, as a selectable field.

use crate::types::Image;

pub type SampleFn = fn(&Image, f32, f32) -> [f32; 4];
pub type MathFn = fn(f32) -> f32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpandShape {
    Square,
    Cross,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RampPhases {
    DownOnly,
    UpOnly,
    DownThenUp,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixLayout {
    ColumnVector,
    RowVector,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpreadBoundary {
    /// A filled pixel that has a to-fill, still unfilled pixel in its 3x3 neighbourhood.
    FilledNextToUnfilled,
    /// Any filled pixel.
    AnyFilled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositeMode {
    /// out = fill * m + frame * (1 - m), m = matte / 255 (INFERRED).
    MatteLerp,
    Unspecified,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZeroWeightPolicy {
    Unspecified,
    KeepFrame,
    FillOnly,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TieBreak {
    EarlierFirst,
    LaterFirst,
}

#[derive(Clone, Copy, Debug)]
pub struct FillConfig {
    /// Shape of the one pixel matte expansion; the specification gives the amount, the shape is open.
    pub expand_shape: ExpandShape,
    /// Source index channels per pixel (a host constant the specification does not give).
    pub index_channels: usize,
    /// Mix width W (a host value the specification does not give).
    pub ramp_len: u32,
    /// Which grow directions run (INFERRED).
    pub ramp_phases: RampPhases,
    /// Scatter switch and seed (the specification does not give the seed).
    pub scatter: bool,
    pub seed: f32,
    /// Sum a batch locally and then add to the output buffer (true) or add per source.
    pub batch_local_sum: bool,
    /// How the 4x4 matrix multiplies a colour.
    pub matrix_layout: MatrixLayout,
    /// Number of solves; the reject filter runs between solves (INFERRED).
    pub fit_passes: u32,
    /// Step of the sample points grid and smoothing sigma (the specification does not give them).
    pub context_step: i32,
    pub smoothing_sigma: f32,
    /// Margin (pixels) a source position must lie inside the source patch.
    pub inside_margin: f32,
    /// How a boundary pixel is chosen, and the level of the spread pass.
    pub spread_boundary: SpreadBoundary,
    pub spread_level: u32,
    /// Keep the weight after normalising.
    pub preserve_weight: bool,
    /// Composite of the fill over the frame.
    pub composite: CompositeMode,
    pub zero_weight: ZeroWeightPolicy,
    /// Tie order of equally distant candidate frames.
    pub tie_break: TieBreak,
    /// Platform dependent pieces behind pointers.
    pub sampler: SampleFn,
    pub sin: MathFn,
    pub exp: MathFn,
}

impl Default for FillConfig {
    fn default() -> Self {
        FillConfig {
            expand_shape: ExpandShape::Square,
            index_channels: 1,
            ramp_len: 1,
            ramp_phases: RampPhases::DownOnly,
            scatter: false,
            seed: 0.0,
            batch_local_sum: true,
            matrix_layout: MatrixLayout::ColumnVector,
            fit_passes: 2,
            context_step: 1,
            smoothing_sigma: 0.0,
            inside_margin: 1.0,
            spread_boundary: SpreadBoundary::FilledNextToUnfilled,
            spread_level: 0,
            preserve_weight: true,
            composite: CompositeMode::MatteLerp,
            zero_weight: ZeroWeightPolicy::Unspecified,
            tie_break: TieBreak::EarlierFirst,
            sampler: crate::geom::bilinear_clamp,
            sin: f32::sin,
            exp: f32::exp,
        }
    }
}
