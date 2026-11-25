//! `soft-edge-feather`: a generic stage. It replaces the alpha channel of each frame by a ramp
//! over the signed distance to the contour where alpha crosses a threshold, with the `soft-edge`
//! crate, every choice of the method given as a parameter. It computes a published method under
//! explicit configuration and claims nothing about any product.
//!
//! The alpha channel is taken out of the frame as a one channel buffer and given to
//! `soft_edge::feather` with the width of the frame. The result, a value from 0 to 1 per pixel,
//! is written back as the alpha channel. Colour passes. Samples are converted by the rule of
//! [`crate::pixels`].
//!
//! `width` may be keyed. The crate defines the ramp for a width above 0 only, so a frame whose
//! width is 0 is written as it was read: a ramp of no width is no feather. The crate takes
//! frames of at most `soft_edge::edt::MAX_SIDE - 2` pixels per side; a larger frame is refused as
//! `unsupported` by `--describe` and by a run alike.

use std::io::{Read, Write};

use soft_edge::{Border, Config, Direction, Profile};

use super::shared::{choice, float, put_plane, take_plane};
use crate::error::{Code, StageError};
use crate::frame::DEPTHS;
use crate::keys;
use crate::params::Values;
use crate::pixels;
use crate::registry::{FrameSpec, Geometry, Kind, Label, LayerDef, Plan, Stage, StageDef};
use crate::run::map_frames;

/// The soft edge feather stage.
pub struct SoftEdgeFeather;

/// The longest side the distance transform of the crate takes.
pub const MAX_SIDE: usize = soft_edge::edt::MAX_SIDE - 2;

/// What the stage declares. Every default of a configuration field is the default of
/// `soft_edge::Config`.
pub static DEF: StageDef = StageDef {
    id: "soft-edge-feather",
    name: "Soft edge feather",
    about: "Replaces the alpha channel of each frame by a linear, smoothstep or cosine ramp over the signed Euclidean distance to the contour where alpha crosses a threshold, inward, outward or centred on the contour; colour passes.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Preserve,
    depths: &DEPTHS,
    params: &[
        float("width", "Width", (0.0, 0.0, 4096.0), true, "The width of the ramp in samples of distance; at 0 the frame is written as it was read; may be keyed."),
        choice("profile", "Profile", "linear", &["linear", "smoothstep", "cosine"], "The shape of the ramp from 0 to 1 (Config::profile)."),
        choice("direction", "Direction", "centred", &["inward", "outward", "centred"], "Which side of the contour the ramp lies on (Config::direction)."),
        float("threshold", "Threshold", (0.5, 0.0, 1.0), false, "A pixel is inside when its alpha is at least this value (Config::threshold)."),
        float("edge_offset", "Edge offset", (0.5, -4096.0, 4096.0), false, "The signed distance is the distance between sample centres minus this value (Config::edge_offset)."),
        choice("border", "Border", "clamp", &["zero", "clamp", "mirror", "mirror-no-repeat"], "Whether everything beyond the frame is outside (zero) or the frame border is no edge (Config::border)."),
    ],
    layers: &[LayerDef {
        id: keys::LAYER_ID,
        optional: true,
        form: keys::FORM,
        about: "A key file with a value per frame for width.",
    }],
};

/// The configuration of the feather for a set of values. The blur fields keep the defaults of
/// the crate; the feather does not read them.
pub fn config(values: &Values) -> Config {
    Config {
        border: match values.choice("border") {
            "zero" => Border::Zero,
            "mirror" => Border::Mirror,
            "mirror-no-repeat" => Border::MirrorNoRepeat,
            _ => Border::Clamp,
        },
        profile: match values.choice("profile") {
            "smoothstep" => Profile::Smoothstep,
            "cosine" => Profile::Cosine,
            _ => Profile::Linear,
        },
        direction: match values.choice("direction") {
            "inward" => Direction::Inward,
            "outward" => Direction::Outward,
            _ => Direction::Centred,
        },
        threshold: values.float("threshold") as f32,
        edge_offset: values.float("edge_offset") as f32,
        ..Config::default()
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

impl Stage for SoftEdgeFeather {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    fn output_size(&self, input: &FrameSpec, _values: &Values) -> Result<(u32, u32), StageError> {
        if input.width as usize > MAX_SIDE || input.height as usize > MAX_SIDE {
            return Err(StageError::unsupported(format!(
                "{} takes frames of at most {MAX_SIDE} pixels per side, not {}x{}",
                DEF.id, input.width, input.height
            )));
        }
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
        let frame_bytes = plan.input.buffer_len()?;
        map_frames(
            input,
            output,
            frame_bytes,
            frame_bytes,
            plan.frames,
            |index, frame, out| {
                let ramp_width = plan.values_at(&keys, index).float("width") as f32;
                if ramp_width == 0.0 {
                    out.clear();
                    out.extend_from_slice(frame);
                    return Ok(());
                }
                let mut samples = pixels::decode(frame, depth);
                let alpha = take_plane(&samples, depth, pixels::ALPHA);
                let made =
                    soft_edge::feather(&alpha, width, height, ramp_width, &cfg).map_err(|e| {
                        let e = refusal(e);
                        StageError::new(e.code, format!("frame {index}: {}", e.message))
                    })?;
                put_plane(&mut samples, depth, pixels::ALPHA, &made);
                pixels::encode(&samples, depth, out);
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(pairs: &[(&str, &str)]) -> Config {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        config(&Values::parse(&DEF, &pairs).unwrap())
    }

    #[test]
    fn the_defaults_are_the_defaults_of_the_crate() {
        assert_eq!(config(&Values::defaults(&DEF)), Config::default());
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        assert_eq!(set(&[("profile", "linear")]).profile, Profile::Linear);
        assert_eq!(
            set(&[("profile", "smoothstep")]).profile,
            Profile::Smoothstep
        );
        assert_eq!(set(&[("profile", "cosine")]).profile, Profile::Cosine);
        assert_eq!(set(&[("direction", "inward")]).direction, Direction::Inward);
        assert_eq!(
            set(&[("direction", "outward")]).direction,
            Direction::Outward
        );
        assert_eq!(
            set(&[("direction", "centred")]).direction,
            Direction::Centred
        );
        assert_eq!(set(&[("border", "zero")]).border, Border::Zero);
        assert_eq!(set(&[("border", "clamp")]).border, Border::Clamp);
        assert_eq!(set(&[("border", "mirror")]).border, Border::Mirror);
        assert_eq!(
            set(&[("border", "mirror-no-repeat")]).border,
            Border::MirrorNoRepeat
        );
        assert_eq!(set(&[("threshold", "0.25")]).threshold, 0.25);
        assert_eq!(set(&[("edge_offset", "-1.5")]).edge_offset, -1.5);
    }

    #[test]
    fn the_side_limit_is_the_limit_of_the_crate() {
        let spec = |width, height| FrameSpec {
            width,
            height,
            depth: 8,
            frame_bytes: u64::from(width) * u64::from(height) * 4,
        };
        let values = Values::defaults(&DEF);
        let side = MAX_SIDE as u32;
        assert_eq!(
            SoftEdgeFeather.output_size(&spec(side, 1), &values),
            Ok((side, 1))
        );
        for (width, height) in [(side + 1, 1), (1, side + 1)] {
            let e = SoftEdgeFeather
                .output_size(&spec(width, height), &values)
                .unwrap_err();
            assert_eq!(e.code, Code::Unsupported);
        }
        // the crate itself refuses the first side above the limit
        let wide = vec![0.0f32; MAX_SIDE + 1];
        assert_eq!(
            soft_edge::signed_distance(&wide, MAX_SIDE + 1, 1, &Config::default()),
            Err(soft_edge::Error::BadParameter("side too large"))
        );
    }

    #[test]
    fn a_refusal_of_the_crate_is_unsupported() {
        let e = refusal(soft_edge::Error::BadParameter(
            "width must be finite and positive",
        ));
        assert_eq!(e.code, Code::Unsupported);
        assert_eq!(
            e.message,
            "soft-edge-feather refuses these values: width must be finite and positive"
        );
        assert_eq!(
            refusal(soft_edge::Error::SizeMismatch).code,
            Code::StageFailed
        );
    }
}
