//! The stage registry: what a stage declares, the provider that holds the stages, and the
//! documents generated from them (the stage manifest, the provider file, the `--describe` answer).
//!
//! A stage is one module under [`crate::stages`] that implements [`Stage`] and one line in
//! [`crate::stages::provider`]. Everything a peer reads about the stage is generated from its
//! [`StageDef`]: the manifest entry, the parameter and layer checks and the refusals.
//!
//! Every stage carries a [`Label`]. A self test computes nothing. A generic stage computes a
//! published method under explicit configuration and claims nothing about any product; its
//! manifest text ends with [`GENERIC_CLAIM`]. A unit stage carries the id and the status of a
//! unit that has been compared; the provider has none.

use std::io::{Read, Write};

use crate::args::{Apply, Fps};
use crate::error::StageError;
use crate::frame::{frame_bytes, MAX_FRAME_BYTES};
use crate::keys::{self, KeySet};
use crate::params::{Layers, Values};
use crate::stage::json_string;

/// The contract every provider built on this crate speaks.
pub const CONTRACT: &str = "frames/2";

/// The sentence that ends the manifest description of every generic stage.
pub const GENERIC_CLAIM: &str =
    "A generic stage: a published method under explicit configuration, not compared with any product.";

/// Whether an output frame depends on one input frame or on several.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Output frame N depends on input frame N only (and on its index when a key file is given).
    Frame,
    /// Output frames depend on several input frames.
    Temporal,
}

impl Kind {
    /// The manifest word.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Frame => "frame",
            Kind::Temporal => "temporal",
        }
    }
}

/// The geometry rule of a stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Geometry {
    /// The output has the size and the frame count of the input.
    Preserve,
    /// The stage answers `--describe` with another width, height or frame count.
    Declared,
}

impl Geometry {
    /// The manifest word.
    pub fn name(self) -> &'static str {
        match self {
            Geometry::Preserve => "preserve",
            Geometry::Declared => "declared",
        }
    }
}

/// What a stage claims. The manifest carries it as the `family` key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Label {
    /// A self test: it computes nothing and reimplements nothing.
    SelfTest,
    /// A published method under explicit configuration, not compared with any product.
    Generic,
    /// A unit that has been compared under the parity rules of the repository. The stage carries
    /// the id of its unit and the status of that unit, and the manifest prints both.
    Unit {
        /// The id of the unit.
        unit: &'static str,
        /// The status of the unit, as its result file gives it.
        status: &'static str,
    },
}

impl Label {
    /// The `family` word of the manifest.
    pub fn family(self) -> &'static str {
        match self {
            Label::SelfTest => "selftest",
            Label::Generic => "generic",
            Label::Unit { .. } => "unit",
        }
    }
}

/// The type of a parameter with its default and its accepted values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamKind {
    /// A whole number from `min` to `max` inclusive.
    Int {
        /// The value when the parameter is not given.
        default: i64,
        /// The lowest accepted value.
        min: i64,
        /// The highest accepted value.
        max: i64,
    },
    /// A finite decimal number from `min` to `max` inclusive.
    Float {
        /// The value when the parameter is not given.
        default: f64,
        /// The lowest accepted value.
        min: f64,
        /// The highest accepted value.
        max: f64,
    },
    /// One of a list of words, given by name.
    Choice {
        /// The value when the parameter is not given; one of `choices`.
        default: &'static str,
        /// The accepted words.
        choices: &'static [&'static str],
    },
    /// `1`, `true`, `on` or `0`, `false`, `off`.
    Checkbox {
        /// The value when the parameter is not given.
        default: bool,
    },
    /// Any text.
    Text {
        /// The value when the parameter is not given.
        default: &'static str,
    },
}

impl ParamKind {
    /// The manifest word of the kind.
    pub fn name(&self) -> &'static str {
        match self {
            ParamKind::Int { .. } => "int",
            ParamKind::Float { .. } => "float",
            ParamKind::Choice { .. } => "popup",
            ParamKind::Checkbox { .. } => "checkbox",
            ParamKind::Text { .. } => "text",
        }
    }
}

/// One parameter of a stage, given as `--param ID=VALUE`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamDef {
    /// The id used on the command line: lower case letters, digits and `_`.
    pub id: &'static str,
    /// The display name.
    pub name: &'static str,
    /// The type, the default and the accepted values.
    pub kind: ParamKind,
    /// A key file may give this parameter a value per frame. Only a float can be keyed, and a
    /// keyed parameter never changes the output geometry.
    pub keyed: bool,
    /// One line that says what the parameter configures.
    pub about: &'static str,
}

/// One extra input of a stage, given as `--layer ID=PATH`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerDef {
    /// The id used on the command line.
    pub id: &'static str,
    /// The stage runs without it.
    pub optional: bool,
    /// The form of the file: `keys-1` for a key file, `frames` for a raw frame file.
    pub form: &'static str,
    /// One line that says what the layer is for.
    pub about: &'static str,
}

/// Everything a stage declares. The manifest entry is generated from it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageDef {
    /// The id passed to `apply`.
    pub id: &'static str,
    /// The display name.
    pub name: &'static str,
    /// What the stage computes, in one sentence.
    pub about: &'static str,
    /// What the stage claims.
    pub label: Label,
    /// Frame or temporal.
    pub kind: Kind,
    /// The stage reads its whole input before it writes the first frame.
    pub spool: bool,
    /// Preserve or declared.
    pub geometry: Geometry,
    /// The depths the stage accepts, from 8, 16 and 32.
    pub depths: &'static [u32],
    /// The parameters.
    pub params: &'static [ParamDef],
    /// The extra inputs.
    pub layers: &'static [LayerDef],
}

impl StageDef {
    /// The manifest description: the sentence of the stage, and for a generic stage the claim.
    pub fn description(&self) -> String {
        match self.label {
            Label::SelfTest => self.about.to_string(),
            Label::Generic => format!("{} {GENERIC_CLAIM}", self.about),
            Label::Unit { unit, status } => format!("{} Unit {unit}: {status}.", self.about),
        }
    }
}

/// The size and depth of one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameSpec {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bits per channel: 8, 16 or 32.
    pub depth: u32,
    /// Bytes of one frame.
    pub frame_bytes: u64,
}

impl FrameSpec {
    /// The bytes of one frame as a buffer length.
    pub fn buffer_len(&self) -> Result<usize, StageError> {
        usize::try_from(self.frame_bytes).map_err(|_| {
            StageError::usage("a frame of this size does not fit in memory on this platform")
        })
    }
}

/// A request that passed every check, with its values and its output geometry.
#[derive(Clone, Debug)]
pub struct Plan {
    /// The definition of the stage that runs.
    pub def: &'static StageDef,
    /// The frames read.
    pub input: FrameSpec,
    /// The frames written.
    pub output: FrameSpec,
    /// The rate, as given.
    pub fps: Fps,
    /// `--frames`: the expected input frame count.
    pub frames: Option<u64>,
    /// The output frame count, when the input count was given.
    pub out_frames: Option<u64>,
    /// The parameter values of the command line, defaults filled in.
    pub values: Values,
    /// The layer paths of the command line.
    pub layers: Layers,
}

impl Plan {
    /// Read the key file named by `--layer keys=PATH`, or an empty set when none was given.
    ///
    /// A file that cannot be read or does not follow the form is `bad_input`. A track for a
    /// parameter the stage does not key, and a key value outside the range of its parameter, are
    /// `unsupported`.
    pub fn keys(&self) -> Result<KeySet, StageError> {
        let Some(path) = self.layers.path(keys::LAYER_ID) else {
            return Ok(KeySet::empty());
        };
        let text = std::fs::read_to_string(path).map_err(|e| {
            StageError::bad_input(format!("cannot read the key file `{path}`: {e}"))
        })?;
        let set = KeySet::parse(&text)
            .map_err(|why| StageError::bad_input(format!("key file `{path}`: {why}")))?;
        for track in set.tracks() {
            let param = self
                .def
                .params
                .iter()
                .find(|p| p.id == track.param && p.keyed);
            let Some(param) = param else {
                return Err(StageError::unsupported(format!(
                    "key file `{path}`: {} has no keyed parameter `{}`",
                    self.def.id, track.param
                )));
            };
            if let ParamKind::Float { min, max, .. } = param.kind {
                if let Some(bad) = track.keys.iter().find(|k| !(min..=max).contains(&k.value)) {
                    return Err(StageError::unsupported(format!(
                        "key file `{path}`: value {} of `{}` at time {} is outside {min} to {max}",
                        bad.value, track.param, bad.time
                    )));
                }
            }
        }
        Ok(set)
    }

    /// The parameter values of input frame `index`: the command line values, with every keyed
    /// parameter that has a track replaced by the track's value at that frame.
    pub fn values_at(&self, keys: &KeySet, index: u64) -> Values {
        let mut values = self.values.clone();
        for track in keys.tracks() {
            if let Some(v) = keys.value(&track.param, index) {
                values.set_float(&track.param, v);
            }
        }
        values
    }
}

/// One stage: its definition, its output geometry and its frame loop.
pub trait Stage: Sync {
    /// What the stage declares.
    fn def(&self) -> &'static StageDef;

    /// The output width and height for an input and a set of values. A preserve stage keeps this
    /// default; a declared stage overrides it. The values are those of the command line: a key
    /// file is not consulted, so a keyed parameter cannot change the geometry.
    fn output_size(&self, input: &FrameSpec, _values: &Values) -> Result<(u32, u32), StageError> {
        Ok((input.width, input.height))
    }

    /// The output frame count for an input frame count. A stage that keeps the count keeps this
    /// default.
    fn output_frames(&self, frames: Option<u64>, _values: &Values) -> Option<u64> {
        frames
    }

    /// Read the input frames, write the output frames and return how many were written.
    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError>;
}

/// A stage provider: its identity and its stages, in manifest order.
pub struct Provider {
    /// The provider id of the manifest: `^[a-z][a-z0-9_]{0,31}$`.
    pub id: &'static str,
    /// The program name, the argv prefix of the manifest command.
    pub program: &'static str,
    /// The version text, also printed by `--version`.
    pub version: String,
    /// The stages, one registration each.
    pub stages: Vec<&'static dyn Stage>,
}

fn depth_list(depths: &[u32]) -> String {
    let words: Vec<String> = depths.iter().map(|d| d.to_string()).collect();
    match words.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => words.join(""),
    }
}

/// A JSON number for a finite float: the shortest text that reads back as the same value.
fn json_float(v: f64) -> String {
    format!("{v}")
}

fn param_json(p: &ParamDef) -> String {
    let typed = match p.kind {
        ParamKind::Int { default, min, max } => {
            format!("\"default\":{default},\"min\":{min},\"max\":{max}")
        }
        ParamKind::Float { default, min, max } => format!(
            "\"default\":{},\"min\":{},\"max\":{}",
            json_float(default),
            json_float(min),
            json_float(max)
        ),
        ParamKind::Choice { default, choices } => {
            let list: Vec<String> = choices.iter().map(|c| json_string(c)).collect();
            format!(
                "\"default\":{},\"choices\":[{}]",
                json_string(default),
                list.join(",")
            )
        }
        ParamKind::Checkbox { default } => format!("\"default\":{default}"),
        ParamKind::Text { default } => format!("\"default\":{}", json_string(default)),
    };
    format!(
        "{{\"id\":{},\"name\":{},\"kind\":{},{typed},\"keyed\":{},\"description\":{}}}",
        json_string(p.id),
        json_string(p.name),
        json_string(p.kind.name()),
        p.keyed,
        json_string(p.about)
    )
}

fn layer_json(l: &LayerDef) -> String {
    format!(
        "{{\"id\":{},\"optional\":{},\"form\":{},\"description\":{}}}",
        json_string(l.id),
        l.optional,
        json_string(l.form),
        json_string(l.about)
    )
}

fn stage_json(def: &StageDef) -> String {
    let depths: Vec<String> = def.depths.iter().map(|d| d.to_string()).collect();
    let params: Vec<String> = def.params.iter().map(param_json).collect();
    let layers: Vec<String> = def.layers.iter().map(layer_json).collect();
    let unit = match def.label {
        Label::Unit { unit, status } => format!(
            ",\"unit\":{},\"status\":{}",
            json_string(unit),
            json_string(status)
        ),
        Label::SelfTest | Label::Generic => String::new(),
    };
    format!(
        "{{\"id\":{},\"kind\":{},\"spool\":{},\"geometry\":{},\"depths\":[{}],\"params\":[{}],\"layers\":[{}],\"name\":{},\"description\":{},\"family\":{}{unit}}}",
        json_string(def.id),
        json_string(def.kind.name()),
        def.spool,
        json_string(def.geometry.name()),
        depths.join(","),
        params.join(","),
        layers.join(","),
        json_string(def.name),
        json_string(&def.description()),
        json_string(def.label.family()),
    )
}

fn id_ok(id: &str, extra: &str) -> bool {
    let mut chars = id.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    first_ok
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || extra.contains(c))
}

impl Provider {
    /// The stage `id`, or `unsupported` with the ids this provider has.
    pub fn find(&self, id: &str) -> Result<&'static dyn Stage, StageError> {
        self.stages
            .iter()
            .copied()
            .find(|s| s.def().id == id)
            .ok_or_else(|| {
                let ids: Vec<&str> = self.stages.iter().map(|s| s.def().id).collect();
                StageError::unsupported(format!(
                    "unknown stage `{id}`: this provider has {}",
                    ids.join(", ")
                ))
            })
    }

    /// The stage manifest: one line of JSON. `program` replaces the program of the command, for
    /// a provider file that points at a built program.
    pub fn manifest_json(&self, program: Option<&str>) -> String {
        let stages: Vec<String> = self.stages.iter().map(|s| stage_json(s.def())).collect();
        format!(
            "{{\"contract\":{},\"provider\":{{\"id\":{},\"version\":{},\"command\":[{}]}},\"stages\":[{}]}}",
            json_string(CONTRACT),
            json_string(self.id),
            json_string(&self.version),
            json_string(program.unwrap_or(self.program)),
            stages.join(","),
        )
    }

    /// The plain text of `list`: one stage id per line.
    pub fn list_text(&self) -> String {
        self.stages
            .iter()
            .map(|s| format!("{}\n", s.def().id))
            .collect()
    }

    /// Judge a parsed `apply` request, in this order: the stage and the depth (`unsupported`),
    /// the parameters and the layers (`unsupported` for an unknown id or a refused value,
    /// `usage` for an id given twice or a required layer that is missing), the input frame size
    /// (`usage` above 4 GiB) and the output geometry (`unsupported` above 4 GiB).
    pub fn plan(&self, request: &Apply) -> Result<(&'static dyn Stage, Plan), StageError> {
        let stage = self.find(&request.stage)?;
        let def = stage.def();
        let depth = u32::try_from(request.depth)
            .ok()
            .filter(|d| def.depths.contains(d))
            .ok_or_else(|| {
                StageError::unsupported(format!(
                    "depth {} is not supported: {} takes {}",
                    request.depth,
                    def.id,
                    depth_list(def.depths)
                ))
            })?;
        let values = Values::parse(def, &request.params)?;
        let layers = Layers::parse(def, &request.layers)?;
        let bytes = frame_bytes(request.width, request.height, depth)
            .filter(|b| *b <= MAX_FRAME_BYTES)
            .ok_or_else(|| {
                StageError::usage(format!(
                    "a {}x{} frame at depth {depth} is more than 4 GiB",
                    request.width, request.height
                ))
            })?;
        let input = FrameSpec {
            width: request.width,
            height: request.height,
            depth,
            frame_bytes: bytes,
        };
        let (out_width, out_height) = stage.output_size(&input, &values)?;
        let out_bytes = frame_bytes(out_width, out_height, depth)
            .filter(|b| *b <= MAX_FRAME_BYTES && out_width > 0 && out_height > 0)
            .ok_or_else(|| {
                StageError::unsupported(format!(
                    "an output frame of {out_width}x{out_height} at depth {depth} is empty or more than 4 GiB"
                ))
            })?;
        let output = FrameSpec {
            width: out_width,
            height: out_height,
            depth,
            frame_bytes: out_bytes,
        };
        let out_frames = stage.output_frames(request.frames, &values);
        let plan = Plan {
            def,
            input,
            output,
            fps: request.fps,
            frames: request.frames,
            out_frames,
            values,
            layers,
        };
        Ok((stage, plan))
    }

    /// Every way the registrations break the rules of the convention or of this crate, as one
    /// line each. An empty list means every stage can be listed and parsed. A test of the crate
    /// holds this list empty for the provider.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut say = |stage: &str, what: String| out.push(format!("{stage}: {what}"));
        for (i, stage) in self.stages.iter().enumerate() {
            let def = stage.def();
            if !id_ok(def.id, "._-") {
                say(
                    def.id,
                    "the stage id is not lower case letters, digits, `.`, `_` and `-`".into(),
                );
            }
            if self.stages[..i].iter().any(|s| s.def().id == def.id) {
                say(def.id, "the stage id is registered twice".into());
            }
            if def.name.is_empty() || def.about.is_empty() {
                say(def.id, "the name or the sentence is empty".into());
            }
            if !def.name.is_ascii() || !def.about.is_ascii() {
                say(def.id, "the name or the sentence is not ASCII".into());
            }
            if let Label::Unit { unit, status } = def.label {
                if unit.is_empty() || status.is_empty() || !unit.is_ascii() || !status.is_ascii() {
                    say(
                        def.id,
                        "a unit stage lacks the id or the status of its unit".into(),
                    );
                }
            }
            if def.depths.is_empty() || def.depths.iter().any(|d| ![8, 16, 32].contains(d)) {
                say(def.id, "the depths are not a list from 8, 16 and 32".into());
            }
            for (j, p) in def.params.iter().enumerate() {
                if !id_ok(p.id, "_") || def.params[..j].iter().any(|q| q.id == p.id) {
                    say(
                        def.id,
                        format!("parameter `{}` has a bad or repeated id", p.id),
                    );
                }
                let sound = match p.kind {
                    ParamKind::Int { default, min, max } => (min..=max).contains(&default),
                    ParamKind::Float { default, min, max } => {
                        min.is_finite() && max.is_finite() && (min..=max).contains(&default)
                    }
                    ParamKind::Choice { default, choices } => {
                        choices.contains(&default) && choices.iter().all(|c| id_ok(c, "-_"))
                    }
                    ParamKind::Checkbox { .. } | ParamKind::Text { .. } => true,
                };
                if !sound {
                    say(
                        def.id,
                        format!("parameter `{}` has a default outside its values", p.id),
                    );
                }
                if p.keyed && !matches!(p.kind, ParamKind::Float { .. }) {
                    say(
                        def.id,
                        format!("parameter `{}` is keyed but not a float", p.id),
                    );
                }
                if p.name.is_empty() || p.about.is_empty() || !p.about.is_ascii() {
                    say(
                        def.id,
                        format!("parameter `{}` lacks a name or a description", p.id),
                    );
                }
            }
            for (j, l) in def.layers.iter().enumerate() {
                if !id_ok(l.id, "_") || def.layers[..j].iter().any(|m| m.id == l.id) {
                    say(def.id, format!("layer `{}` has a bad or repeated id", l.id));
                }
                if l.form.is_empty() || l.about.is_empty() {
                    say(
                        def.id,
                        format!("layer `{}` lacks a form or a description", l.id),
                    );
                }
            }
            let keyed = def.params.iter().any(|p| p.keyed);
            let key_layer = def
                .layers
                .iter()
                .find(|l| l.id == keys::LAYER_ID)
                .map(|l| (l.optional, l.form));
            match (keyed, key_layer) {
                (true, Some((true, keys::FORM))) | (false, None) => {}
                _ => say(
                    def.id,
                    "a stage with a keyed parameter has the optional layer `keys` of form `keys-1`, and no other stage has it".into(),
                ),
            }
        }
        out
    }
}

/// The `--describe` answer: one line of JSON with the keys of the convention's describe schema
/// in the order of its `required` list.
pub fn describe_json(plan: &Plan) -> String {
    let frames = match plan.out_frames {
        Some(n) => n.to_string(),
        None => "null".to_string(),
    };
    format!(
        "{{\"ok\":true,\"contract\":{},\"stage\":{},\"width\":{},\"height\":{},\"depth\":{},\"fps\":{},\"frames\":{}}}",
        json_string(CONTRACT),
        json_string(plan.def.id),
        plan.output.width,
        plan.output.height,
        plan.output.depth,
        json_string(&plan.fps.text()),
        frames
    )
}
