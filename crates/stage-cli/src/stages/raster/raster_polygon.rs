//! `raster-polygon`: a generic stage. It rasterises closed polygons to a coverage matte with the
//! `coverage-raster` crate, every choice of the method given as a parameter, and carries the
//! matte in the frame. It computes a published method under explicit configuration and claims
//! nothing about any product.
//!
//! - The polygons come from `--layer shape=PATH`, a file of the form `rings-1`
//!   ([`super::shape`]). Without the layer there is no polygon and the coverage is 0 everywhere.
//! - The parameters are those of [`super::coverage`]; they map one to one to
//!   `coverage_raster::Config`.
//! - `offset_x` and `offset_y` move every point and may be keyed, so a key file moves the
//!   polygons from frame to frame.
//! - `carry` and `apply` say how the one channel matte is written into the four channel frame
//!   ([`super::carry`]).
//!
//! The geometry is preserved. `--describe` answers the input size, or refuses a frame the
//! rasteriser does not take; it opens no layer.

use std::io::{Read, Write};

use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::params::Values;
use crate::registry::{FrameSpec, Geometry, Kind, Label, Plan, Stage, StageDef};

use super::{coverage, shape};

/// The polygon rasterisation stage.
pub struct RasterPolygon;

/// What the stage declares.
pub static DEF: StageDef = StageDef {
    id: "raster-polygon",
    name: "Raster polygon",
    about: "Rasterises closed polygons from a shape file to a coverage matte, by exact area (Duff 1989) or by point samples, under an explicit fill rule, pixel centre, row direction, combine rule and quantisation, and carries the matte in the frame as the carry parameter says.",
    label: Label::Generic,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Preserve,
    depths: &DEPTHS,
    params: &coverage::PARAMS,
    layers: &[shape::RINGS_LAYER, coverage::KEYS_LAYER],
};

impl Stage for RasterPolygon {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    fn output_size(&self, input: &FrameSpec, values: &Values) -> Result<(u32, u32), StageError> {
        coverage::check(input, values)?;
        Ok((input.width, input.height))
    }

    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError> {
        let polygons = shape::rings(plan)?;
        coverage::run(plan, input, output, &polygons)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use coverage_raster::{
        Combine, Config, Depth, FillRule, Method, Pattern, Precision, Quantise, Rounding, Scale,
    };

    use super::*;
    use crate::args::{self, Command};
    use crate::stages::raster::carry::{self, Apply, Carry};

    fn values(pairs: &[(&str, &str)]) -> Values {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Values::parse(&DEF, &pairs).unwrap()
    }

    #[test]
    fn the_defaults_are_the_defaults_of_the_rasteriser() {
        assert_eq!(
            coverage::config(&Values::defaults(&DEF), 8),
            Config::canonical()
        );
        assert_eq!(carry::carry(&Values::defaults(&DEF)), Carry::Alpha);
        assert_eq!(carry::apply(&Values::defaults(&DEF)), Apply::Replace);
    }

    #[test]
    fn the_depth_of_the_stream_is_the_depth_of_the_codes() {
        let defaults = Values::defaults(&DEF);
        let depth = |d| coverage::config(&defaults, d).quantise.map(|q| q.depth);
        assert_eq!(depth(8), Some(Depth::Bits8));
        assert_eq!(depth(16), Some(Depth::Bits16));
        assert_eq!(depth(32), Some(Depth::Float32));
        for d in [8, 16, 32] {
            let other = Config {
                quantise: Config::canonical().quantise.map(|q| Quantise {
                    depth: depth(d).unwrap(),
                    ..q
                }),
                ..Config::canonical()
            };
            assert_eq!(coverage::config(&defaults, d), other);
        }
    }

    #[test]
    fn every_choice_maps_to_a_configuration() {
        let set = |pairs: &[(&str, &str)]| coverage::config(&values(pairs), 8);
        assert_eq!(set(&[("method", "exact-area")]).method, Method::ExactArea);
        assert_eq!(set(&[("method", "point")]).method, Method::Point);
        assert_eq!(
            set(&[("method", "samples-grid")]).method,
            Method::Samples(Pattern::Grid(4))
        );
        assert_eq!(
            set(&[("method", "samples-grid"), ("samples", "7")]).method,
            Method::Samples(Pattern::Grid(7))
        );
        assert_eq!(
            set(&[("method", "samples-rotated"), ("samples", "64")]).method,
            Method::Samples(Pattern::Rotated(64))
        );
        assert_eq!(set(&[("fill", "non-zero")]).fill, FillRule::NonZero);
        assert_eq!(set(&[("fill", "even-odd")]).fill, FillRule::EvenOdd);
        assert_eq!(set(&[("centre", "0")]).centre, 0.0);
        assert_eq!(set(&[("centre", "0.25")]).centre, 0.25);
        assert!(set(&[("y_down", "1")]).y_down);
        assert!(!set(&[("y_down", "off")]).y_down);
        assert_eq!(set(&[("combine", "max")]).combine, Combine::Max);
        assert_eq!(set(&[("combine", "add-clamp")]).combine, Combine::AddClamp);
        assert_eq!(set(&[("combine", "over")]).combine, Combine::Over);
        assert_eq!(set(&[("combine", "replace")]).combine, Combine::Replace);
        assert_eq!(set(&[("precision", "double")]).precision, Precision::Double);
        assert_eq!(set(&[("precision", "single")]).precision, Precision::Single);
        assert_eq!(set(&[("snap_bits", "0")]).snap_bits, None);
        assert_eq!(set(&[("snap_bits", "1")]).snap_bits, Some(1));
        assert_eq!(set(&[("snap_bits", "30")]).snap_bits, Some(30));
        assert_eq!(set(&[("quantise", "off")]).quantise, None);
        let scale = |word| set(&[("quantise_scale", word)]).quantise.unwrap().scale;
        assert_eq!(scale("full"), Scale::Full);
        assert_eq!(scale("power-of-two"), Scale::PowerOfTwo);
        let rounding = |word| {
            set(&[("quantise_rounding", word)])
                .quantise
                .unwrap()
                .rounding
        };
        assert_eq!(rounding("nearest-half-up"), Rounding::NearestHalfUp);
        assert_eq!(rounding("nearest-even"), Rounding::NearestEven);
        assert_eq!(rounding("floor"), Rounding::Floor);
        assert_eq!(rounding("ceil"), Rounding::Ceil);
        let carried = |word| carry::carry(&values(&[("carry", word)]));
        assert_eq!(carried("alpha"), Carry::Alpha);
        assert_eq!(carried("luma"), Carry::Luma);
        assert_eq!(carried("all"), Carry::All);
        let applied = |word| carry::apply(&values(&[("apply", word)]));
        assert_eq!(applied("replace"), Apply::Replace);
        assert_eq!(applied("multiply"), Apply::Multiply);
        assert_eq!(applied("max"), Apply::Max);
    }

    #[test]
    fn every_configuration_the_parameters_can_reach_is_valid() {
        for method in ["exact-area", "point", "samples-grid", "samples-rotated"] {
            for samples in ["1", "64"] {
                for (centre, snap) in [("0", "0"), ("1", "30")] {
                    let v = values(&[
                        ("method", method),
                        ("samples", samples),
                        ("centre", centre),
                        ("snap_bits", snap),
                    ]);
                    for depth in [8, 16, 32] {
                        assert!(coverage::config(&v, depth).validate().is_ok(), "{method}");
                    }
                }
            }
        }
    }

    /// Hands out `left` copies of one frame and counts the frames it has handed out whole.
    struct Feed {
        frame: Vec<u8>,
        at: usize,
        left: u64,
        served: Rc<Cell<u64>>,
    }

    impl Read for Feed {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.left == 0 {
                return Ok(0);
            }
            let n = buf.len().min(self.frame.len() - self.at);
            buf[..n].copy_from_slice(&self.frame[self.at..self.at + n]);
            self.at += n;
            if self.at == self.frame.len() {
                self.at = 0;
                self.left -= 1;
                self.served.set(self.served.get() + 1);
            }
            Ok(n)
        }
    }

    /// Counts the bytes written and records how far the reader ever was ahead of the writer.
    struct Sink {
        frame_bytes: u64,
        bytes: u64,
        served: Rc<Cell<u64>>,
        ahead: u64,
    }

    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let written = self.bytes / self.frame_bytes;
            self.ahead = self.ahead.max(self.served.get() - written);
            self.bytes += buf.len() as u64;
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_frame_is_written_before_the_next_one_is_read() {
        let words = [
            "apply",
            "raster-polygon",
            "--width",
            "8",
            "--height",
            "4",
            "--depth",
            "8",
            "--fps",
            "24/1",
        ];
        let words: Vec<String> = words.iter().map(|w| w.to_string()).collect();
        let Command::Apply(request) = args::parse(&words).unwrap() else {
            panic!("not an apply command");
        };
        let (stage, plan) = crate::stages::provider().plan(&request).unwrap();
        let served = Rc::new(Cell::new(0));
        let frames = 3000u64;
        let mut feed = Feed {
            frame: vec![9; 8 * 4 * 4],
            at: 0,
            left: frames,
            served: Rc::clone(&served),
        };
        let mut sink = Sink {
            frame_bytes: 8 * 4 * 4,
            bytes: 0,
            served: Rc::clone(&served),
            ahead: 0,
        };
        assert_eq!(stage.run(&plan, &mut feed, &mut sink).unwrap(), frames);
        assert_eq!(sink.bytes, frames * 8 * 4 * 4);
        assert_eq!(sink.ahead, 1, "the stage never holds a second input frame");
    }
}
