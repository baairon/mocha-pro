/// Whether colour channels are stored multiplied by alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Form {
    Straight,
    Premultiplied,
}

/// Where the opacity is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpacityPoint {
    /// `as = as * opacity` before blending and compositing.
    OnSourceAlpha,
    /// The full result is mixed with the premultiplied backdrop: `r * opacity + b * (1 - opacity)`.
    OnResult,
}

/// Clamping of values to 0..1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Clamp {
    None,
    /// The final output channels and alpha.
    Output,
    /// The straight inputs after conversion, and the final output.
    InputAndOutput,
}

/// The soft light function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoftLight {
    /// W3C Compositing and Blending Level 1, section 9.1.3 (`D(x)` is a cubic up to 0.25).
    W3c,
    /// ISO 32000-1:2008 section 11.3.5: `D(x) = sqrt(x)` for every `x`.
    Legacy,
}

/// A value returned by a division guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Guard {
    Zero,
    One,
}

/// Which guard condition of colour dodge and colour burn is tested first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GuardOrder {
    BackdropFirst,
    SourceFirst,
}

/// Whether the blended colour is mixed with the source colour by backdrop alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackdropMix {
    /// `Cs' = (1 - ab) * Cs + ab * B(Cb, Cs)` (W3C section 5.1).
    W3c,
    /// `Cs' = B(Cb, Cs)`.
    Off,
}

/// Every convention a later calibration may vary. The default follows the published formulas
/// where they exist and claims nothing else: straight in, straight out, opacity on the source
/// alpha, no clamping, W3C soft light, W3C guard values tested backdrop first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Config {
    pub input_form: Form,
    pub output_form: Form,
    pub opacity_point: OpacityPoint,
    pub clamp: Clamp,
    pub soft_light: SoftLight,
    /// Colour dodge, backdrop 0 (W3C: `Zero`).
    pub dodge_backdrop_zero: Guard,
    /// Colour dodge, source 1 (W3C: `One`).
    pub dodge_source_one: Guard,
    /// Colour burn, backdrop 1 (W3C: `One`).
    pub burn_backdrop_one: Guard,
    /// Colour burn, source 0 (W3C: `Zero`).
    pub burn_source_zero: Guard,
    pub guard_order: GuardOrder,
    pub backdrop_mix: BackdropMix,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            input_form: Form::Straight,
            output_form: Form::Straight,
            opacity_point: OpacityPoint::OnSourceAlpha,
            clamp: Clamp::None,
            soft_light: SoftLight::W3c,
            dodge_backdrop_zero: Guard::Zero,
            dodge_source_one: Guard::One,
            burn_backdrop_one: Guard::One,
            burn_source_zero: Guard::Zero,
            guard_order: GuardOrder::BackdropFirst,
            backdrop_mix: BackdropMix::W3c,
        }
    }
}

impl Config {
    /// The number of configurations: 2 * 2 * 2 * 3 * 2 * 2^4 * 2 * 2.
    pub const SPACE_SIZE: usize = 3072;

    /// Every configuration, each once, in a fixed order (the last field varies fastest).
    pub fn enumerate() -> Vec<Config> {
        let forms = [Form::Straight, Form::Premultiplied];
        let points = [OpacityPoint::OnSourceAlpha, OpacityPoint::OnResult];
        let clamps = [Clamp::None, Clamp::Output, Clamp::InputAndOutput];
        let softs = [SoftLight::W3c, SoftLight::Legacy];
        let guards = [Guard::Zero, Guard::One];
        let orders = [GuardOrder::BackdropFirst, GuardOrder::SourceFirst];
        let mixes = [BackdropMix::W3c, BackdropMix::Off];
        let mut v = Vec::with_capacity(Self::SPACE_SIZE);
        for &input_form in &forms {
            for &output_form in &forms {
                for &opacity_point in &points {
                    for &clamp in &clamps {
                        for &soft_light in &softs {
                            for &dodge_backdrop_zero in &guards {
                                for &dodge_source_one in &guards {
                                    for &burn_backdrop_one in &guards {
                                        for &burn_source_zero in &guards {
                                            for &guard_order in &orders {
                                                for &backdrop_mix in &mixes {
                                                    v.push(Config {
                                                        input_form,
                                                        output_form,
                                                        opacity_point,
                                                        clamp,
                                                        soft_light,
                                                        dodge_backdrop_zero,
                                                        dodge_source_one,
                                                        burn_backdrop_one,
                                                        burn_source_zero,
                                                        guard_order,
                                                        backdrop_mix,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        v
    }
}
