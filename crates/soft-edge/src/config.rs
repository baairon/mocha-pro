//! The configuration of every choice that a later calibration may need to vary.
//!
//! Every default is a neutral placeholder and claims nothing about any other system.

use crate::border::Border;

/// How a fractional box radius `r = n + f` (integer part `n`, fraction `f`) is turned into weights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FracRule {
    /// A kernel of `2n + 1` taps of weight 1 and two extra end taps at offsets `n + 1` and
    /// `-(n + 1)` of weight `f`, all divided by the sum `2n + 1 + 2f`.
    EndTaps,
    /// The weighted mix `(1 - f) * box(n) + f * box(n + 1)` of two normalised integer box kernels.
    Lerp,
}

/// How `exp(-t)` is evaluated for the Gaussian weights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpMethod {
    /// Every weight from the series of `exp(-u)` after halving `t` to `u <= 0.5` and squaring back.
    Series,
    /// One series evaluation of `q = exp(-1 / (2 sigma^2))`, then the weights `q^(x^2)` by the
    /// recurrence `g(x+1) = g(x) * r(x)`, `r(x+1) = r(x) * q^2`, `g(0) = 1`, `r(0) = q`.
    Recurrence,
}

/// The shape of the ramp from 0 to 1 over the parameter `t` in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// `t`.
    Linear,
    /// `t * t * (3 - 2 t)`.
    Smoothstep,
    /// `0.5 - 0.5 cos(pi t)`, evaluated as `sin^2(pi t / 2)` by a fixed odd polynomial.
    Cosine,
}

/// Which side of the zero crossing of the hard mask the feather ramp occupies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// The ramp lies inside the mask: 0 at the crossing, 1 at distance `width` inside.
    Inward,
    /// The ramp lies outside: 1 at the crossing, 0 at distance `width` outside.
    Outward,
    /// The ramp is centred on the crossing: 0.5 (before the profile) there, `width / 2` each side.
    Centred,
}

/// The full configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// Border rule for the blurs and the feather.
    pub border: Border,
    /// Fractional radius rule of the box blur.
    pub frac_rule: FracRule,
    /// Truncation of the Gaussian at `k * sigma` taps (half width `ceil(k * sigma)`).
    pub trunc_k: f32,
    /// Exponential method of the Gaussian.
    pub exp_method: ExpMethod,
    /// Number of series terms (at least 1) for `exp`.
    pub exp_terms: u32,
    /// A Gaussian given by radius uses `sigma = radius / radius_to_sigma`.
    pub radius_to_sigma: f32,
    /// Ramp profile of the feather.
    pub profile: Profile,
    /// Ramp direction of the feather.
    pub direction: Direction,
    /// A mask sample is inside when it is at least this value.
    pub threshold: f32,
    /// Signed distance is the distance between sample centres minus this offset.
    pub edge_offset: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            border: Border::Clamp,
            frac_rule: FracRule::EndTaps,
            trunc_k: 3.0,
            exp_method: ExpMethod::Series,
            exp_terms: 12,
            radius_to_sigma: 2.0,
            profile: Profile::Linear,
            direction: Direction::Centred,
            threshold: 0.5,
            edge_offset: 0.5,
        }
    }
}

impl Config {
    /// Every combination of the discrete choices (border, fractional rule, exponential method,
    /// profile, direction, edge offset of 0 or 0.5), the other fields at their defaults. The order
    /// is fixed.
    pub fn enumerate() -> Vec<Config> {
        let mut out = Vec::new();
        for &border in &Border::ALL {
            for &frac_rule in &[FracRule::EndTaps, FracRule::Lerp] {
                for &exp_method in &[ExpMethod::Series, ExpMethod::Recurrence] {
                    for &profile in &[Profile::Linear, Profile::Smoothstep, Profile::Cosine] {
                        for &direction in
                            &[Direction::Inward, Direction::Outward, Direction::Centred]
                        {
                            for &edge_offset in &[0.0f32, 0.5] {
                                out.push(Config {
                                    border,
                                    frac_rule,
                                    exp_method,
                                    profile,
                                    direction,
                                    edge_offset,
                                    ..Config::default()
                                });
                            }
                        }
                    }
                }
            }
        }
        out
    }
}
