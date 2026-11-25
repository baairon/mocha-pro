//! `soft-edge-blur`: a generic stage. It blurs the alpha channel, or all four channels, of each
//! frame with a separable box blur, a repeated box blur or a Gaussian blur of the `soft-edge`
//! crate, every choice of the method given as a parameter. It computes a published method under
//! explicit configuration and claims nothing about any product.
//!
//! Each chosen channel is taken out of the frame as a one channel buffer, blurred with
//! `soft_edge::box_blur`, `soft_edge::box_blur_repeated` or `soft_edge::gaussian_blur`, and
//! written back. The other channels pass. Samples are converted by the rule of
//! [`crate::pixels`]; integer depths clamp to their range on the way out, depth 32 does not.
//!
//! `radius` may be keyed. The crate bounds some values by others (a Gaussian needs a positive
//! truncation, a Gaussian given by radius needs a positive divisor, a kernel has at most 2^20
//! taps per side). The stage asks the crate for the kernel of the command line values before it
//! reads a frame, so such a request is refused as `unsupported` by `--describe` and by a run
//! alike; a keyed radius that the crate refuses ends the run at that frame with the same code.

use std::io::{Read, Write};

use soft_edge::{Border, Config, ExpMethod, FracRule, GaussSpec};

use super::shared::{choice, float, int, put_plane, take_plane};
use crate::error::{Code, StageError};
use crate::frame::DEPTHS;
use crate::keys;
use crate::params::Values;
use crate::pixels;
use crate::registry::{FrameSpec, Geometry, Kind, Label, LayerDef, Plan, Stage, StageDef};
use crate::run::map_frames;

/// The soft edge blur stage.
pub struct SoftEdgeBlur;

/// What the stage declares. Every default of a configuration field is the default of
/// `soft_edge::Config`.
pub static DEF: StageDef = StageDef {
    id: "soft-edge-blur",
    name: "Soft edge blur",
    about: "Blurs the alpha channel, or all four channels, of each frame with a separable box blur, a repeated box blur or a truncated Gaussian blur under an explicit border, fractional radius and kernel rule.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Preserve,
    depths: &DEPTHS,
    params: &[
        choice("channels", "Channels", "alpha", &["alpha", "all"], "Which channels are blurred: the alpha channel, or all four; the others pass."),
        choice("method", "Method", "box", &["box", "box-repeated", "gaussian"], "The blur: one box blur, passes box blurs in a row, or a Gaussian blur."),
        float("radius", "Radius", (0.0, 0.0, 4096.0), true, "The box radius in samples, or the Gaussian sigma or radius as gauss_spec says; 0 is no blur; may be keyed."),
        int("passes", "Passes", (3, 1, 16), "The number of box blurs in a row; read when method is box-repeated."),
        choice("gauss_spec", "Gaussian given by", "sigma", &["sigma", "radius"], "Whether radius is the standard deviation of the Gaussian or a radius divided by radius_to_sigma (GaussSpec); read when method is gaussian."),
        choice("border", "Border", "clamp", &["zero", "clamp", "mirror", "mirror-no-repeat"], "What a tap outside the frame reads (Config::border)."),
        choice("frac_rule", "Fractional radius", "end-taps", &["end-taps", "lerp"], "How a box radius with a fraction becomes weights (Config::frac_rule)."),
        float("trunc_k", "Truncation", (3.0, 0.0, 64.0), false, "The Gaussian is cut at this many standard deviations (Config::trunc_k); the Gaussian needs a value above 0."),
        choice("exp_method", "Exponential", "series", &["series", "recurrence"], "How the Gaussian weights are evaluated (Config::exp_method)."),
        int("exp_terms", "Series terms", (12, 1, 64), "The number of terms of the exponential series (Config::exp_terms)."),
        float("radius_to_sigma", "Radius to sigma", (2.0, 0.0, 4096.0), false, "A Gaussian given by radius has sigma = radius / this value (Config::radius_to_sigma); it needs a value above 0."),
    ],
    layers: &[LayerDef {
        id: keys::LAYER_ID,
        optional: true,
        form: keys::FORM,
        about: "A key file with a value per frame for radius.",
    }],
};

/// The configuration of the blurs for a set of values. The feather fields keep the defaults of
/// the crate; no blur reads them.
pub fn config(values: &Values) -> Config {
    Config {
        border: match values.choice("border") {
            "zero" => Border::Zero,
            "mirror" => Border::Mirror,
            "mirror-no-repeat" => Border::MirrorNoRepeat,
            _ => Border::Clamp,
        },
        frac_rule: match values.choice("frac_rule") {
            "lerp" => FracRule::Lerp,
            _ => FracRule::EndTaps,
        },
        trunc_k: values.float("trunc_k") as f32,
        exp_method: match values.choice("exp_method") {
            "recurrence" => ExpMethod::Recurrence,
            _ => ExpMethod::Series,
        },
        exp_terms: values.int("exp_terms") as u32,
        radius_to_sigma: values.float("radius_to_sigma") as f32,
        ..Config::default()
    }
}

/// The Gaussian parameter for a set of values: `radius` read as `gauss_spec` says.
pub fn gauss_spec(values: &Values) -> GaussSpec {
    let radius = values.float("radius") as f32;
    match values.choice("gauss_spec") {
        "radius" => GaussSpec::Radius(radius),
        _ => GaussSpec::Sigma(radius),
    }
}

/// The channels the stage blurs, as indices of a decoded frame.
pub fn channels(values: &Values) -> &'static [usize] {
    match values.choice("channels") {
        "all" => &[0, 1, 2, 3],
        _ => &[pixels::ALPHA],
    }
}

/// Blur one channel buffer of `width` by `height` samples as the values say.
pub fn blur(
    plane: &[f32],
    width: usize,
    height: usize,
    values: &Values,
    cfg: &Config,
) -> Result<Vec<f32>, soft_edge::Error> {
    let radius = values.float("radius") as f32;
    match values.choice("method") {
        "box-repeated" => soft_edge::box_blur_repeated(
            plane,
            width,
            height,
            radius,
            values.int("passes") as u32,
            cfg,
        ),
        "gaussian" => soft_edge::gaussian_blur(plane, width, height, gauss_spec(values), cfg),
        _ => soft_edge::box_blur(plane, width, height, radius, cfg),
    }
}

/// Ask the crate for the kernel of a set of values, so that a combination it refuses is known
/// before a frame is read.
fn check(values: &Values, cfg: &Config) -> Result<(), soft_edge::Error> {
    match values.choice("method") {
        "gaussian" => soft_edge::gaussian_kernel(gauss_spec(values), cfg).map(|_| ()),
        _ => soft_edge::box_kernel(values.float("radius") as f32, cfg.frac_rule).map(|_| ()),
    }
}

fn refusal(e: soft_edge::Error) -> StageError {
    match e {
        soft_edge::Error::BadParameter(why) => {
            StageError::unsupported(format!("{} refuses these values: {why}", DEF.id))
        }
        soft_edge::Error::SizeMismatch => StageError::new(Code::StageFailed, e.to_string()),
    }
}

fn at_frame(index: u64, e: StageError) -> StageError {
    StageError::new(e.code, format!("frame {index}: {}", e.message))
}

impl Stage for SoftEdgeBlur {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    fn output_size(&self, input: &FrameSpec, values: &Values) -> Result<(u32, u32), StageError> {
        check(values, &config(values)).map_err(refusal)?;
        Ok((input.width, input.height))
    }

    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError> {
        let keys = plan.keys()?;
        let cfg = config(&plan.values);
        let depth = plan.input.depth;
        let (width, height) = (plan.input.width as usize, plan.input.height as usize);
        let chosen = channels(&plan.values);
        let frame_bytes = plan.input.buffer_len()?;
        map_frames(
            input,
            output,
            frame_bytes,
            frame_bytes,
            plan.frames,
            |index, frame, out| {
                let values = plan.values_at(&keys, index);
                let mut samples = pixels::decode(frame, depth);
                for &channel in chosen {
                    let plane = take_plane(&samples, depth, channel);
                    let made = blur(&plane, width, height, &values, &cfg)
                        .map_err(|e| at_frame(index, refusal(e)))?;
                    put_plane(&mut samples, depth, channel, &made);
                }
                pixels::encode(&samples, depth, out);
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(pairs: &[(&str, &str)]) -> Values {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Values::parse(&DEF, &pairs).unwrap()
    }

    #[test]
    fn the_defaults_are_the_defaults_of_the_crate() {
        let values = Values::defaults(&DEF);
        assert_eq!(config(&values), Config::default());
        assert_eq!(gauss_spec(&values), GaussSpec::Sigma(0.0));
        assert_eq!(channels(&values), [pixels::ALPHA]);
        assert!(check(&values, &config(&values)).is_ok());
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        assert_eq!(config(&set(&[("border", "zero")])).border, Border::Zero);
        assert_eq!(config(&set(&[("border", "clamp")])).border, Border::Clamp);
        assert_eq!(config(&set(&[("border", "mirror")])).border, Border::Mirror);
        assert_eq!(
            config(&set(&[("border", "mirror-no-repeat")])).border,
            Border::MirrorNoRepeat
        );
        assert_eq!(
            config(&set(&[("frac_rule", "lerp")])).frac_rule,
            FracRule::Lerp
        );
        assert_eq!(
            config(&set(&[("exp_method", "recurrence")])).exp_method,
            ExpMethod::Recurrence
        );
        assert_eq!(config(&set(&[("trunc_k", "2.5")])).trunc_k, 2.5);
        assert_eq!(config(&set(&[("exp_terms", "7")])).exp_terms, 7);
        assert_eq!(
            config(&set(&[("radius_to_sigma", "4")])).radius_to_sigma,
            4.0
        );
        assert_eq!(
            gauss_spec(&set(&[("gauss_spec", "radius"), ("radius", "6")])),
            GaussSpec::Radius(6.0)
        );
        assert_eq!(
            gauss_spec(&set(&[("gauss_spec", "sigma"), ("radius", "1.5")])),
            GaussSpec::Sigma(1.5)
        );
        assert_eq!(channels(&set(&[("channels", "all")])), [0, 1, 2, 3]);
    }

    #[test]
    fn each_method_calls_its_function_of_the_crate() {
        let plane = [0.0, 0.0, 1.0, 0.0, 0.0, 0.5];
        let cfg = Config::default();
        let boxed = set(&[("radius", "1")]);
        assert_eq!(
            blur(&plane, 3, 2, &boxed, &cfg),
            soft_edge::box_blur(&plane, 3, 2, 1.0, &cfg)
        );
        let repeated = set(&[("method", "box-repeated"), ("radius", "1"), ("passes", "2")]);
        assert_eq!(
            blur(&plane, 3, 2, &repeated, &cfg),
            soft_edge::box_blur_repeated(&plane, 3, 2, 1.0, 2, &cfg)
        );
        let gaussian = set(&[("method", "gaussian"), ("radius", "0.75")]);
        assert_eq!(
            blur(&plane, 3, 2, &gaussian, &cfg),
            soft_edge::gaussian_blur(&plane, 3, 2, GaussSpec::Sigma(0.75), &cfg)
        );
    }

    #[test]
    fn values_the_crate_refuses_are_unsupported() {
        for pairs in [
            &[("method", "gaussian"), ("trunc_k", "0")][..],
            &[
                ("method", "gaussian"),
                ("gauss_spec", "radius"),
                ("radius_to_sigma", "0"),
            ][..],
            &[
                ("method", "gaussian"),
                ("gauss_spec", "radius"),
                ("radius", "4096"),
                ("radius_to_sigma", "0.001"),
            ][..],
        ] {
            let values = set(pairs);
            let e = check(&values, &config(&values))
                .map_err(refusal)
                .unwrap_err();
            assert_eq!(e.code, Code::Unsupported, "{pairs:?}");
            assert!(e
                .message
                .starts_with("soft-edge-blur refuses these values: "));
        }
        // the box blurs do not read the Gaussian fields
        let values = set(&[
            ("trunc_k", "0"),
            ("radius_to_sigma", "0"),
            ("radius", "2.5"),
        ]);
        assert!(check(&values, &config(&values)).is_ok());
    }

    #[test]
    fn a_size_mismatch_is_a_failure_of_the_stage() {
        let e = refusal(soft_edge::Error::SizeMismatch);
        assert_eq!(e.code, Code::StageFailed);
        assert_eq!(
            at_frame(3, e).message,
            "frame 3: buffer length is not width * height"
        );
    }
}
