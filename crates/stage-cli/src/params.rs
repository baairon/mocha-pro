//! Typed parsing of `--param K=V` and `--layer ID=PATH` against the definition of a stage.
//!
//! The error rows follow the convention: an id given twice
//! is `usage` (exit 2); an unknown id and a value that does not parse, lies outside its range or
//! is not one of the choices are `unsupported` (exit 3). A value that holds `@` is a keyed value
//! of the convention's command line form; a stage here takes keys from a key file instead and
//! refuses the command line form as `unsupported`.

use crate::error::StageError;
use crate::registry::{LayerDef, ParamDef, ParamKind, StageDef};

/// One parsed parameter value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A whole number.
    Int(i64),
    /// A finite decimal number.
    Float(f64),
    /// One of the choices of the parameter.
    Choice(&'static str),
    /// A checkbox.
    Flag(bool),
    /// A text.
    Text(String),
}

/// The values of every parameter of a stage, defaults filled in, in the order of the definition.
#[derive(Clone, Debug, PartialEq)]
pub struct Values {
    defs: &'static [ParamDef],
    values: Vec<Value>,
}

fn default_of(def: &ParamDef) -> Value {
    match def.kind {
        ParamKind::Int { default, .. } => Value::Int(default),
        ParamKind::Float { default, .. } => Value::Float(default),
        ParamKind::Choice { default, .. } => Value::Choice(default),
        ParamKind::Checkbox { default } => Value::Flag(default),
        ParamKind::Text { default } => Value::Text(default.to_string()),
    }
}

/// A whole number: ASCII digits with an optional leading `-`.
fn parse_int(text: &str) -> Option<i64> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
}

/// A finite decimal: digits with an optional leading `-`, an optional fraction and an optional
/// exponent. Words such as `inf` and `nan` and a leading `+` are refused.
pub fn parse_float(text: &str) -> Option<f64> {
    let body = text.strip_prefix('-').unwrap_or(text);
    let first = body.bytes().next()?;
    if !first.is_ascii_digit() {
        return None;
    }
    if !body
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'-' | b'+'))
    {
        return None;
    }
    text.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn bad(def: &ParamDef, why: String) -> StageError {
    StageError::unsupported(format!("bad value for `{}`: {why}", def.id))
}

fn parse_value(def: &ParamDef, text: &str) -> Result<Value, StageError> {
    if text.contains('@') && !matches!(def.kind, ParamKind::Text { .. }) {
        return Err(bad(
            def,
            "a keyed value is not taken on the command line; give a key file with --layer keys=PATH"
                .to_string(),
        ));
    }
    match def.kind {
        ParamKind::Int { min, max, .. } => match parse_int(text) {
            Some(v) if (min..=max).contains(&v) => Ok(Value::Int(v)),
            Some(v) => Err(bad(def, format!("{v} is outside {min} to {max}"))),
            None => Err(bad(def, format!("cannot parse `{text}` as a whole number"))),
        },
        ParamKind::Float { min, max, .. } => match parse_float(text) {
            Some(v) if (min..=max).contains(&v) => Ok(Value::Float(v)),
            Some(v) => Err(bad(def, format!("{v} is outside {min} to {max}"))),
            None => Err(bad(def, format!("cannot parse `{text}` as a number"))),
        },
        ParamKind::Choice { choices, .. } => match choices.iter().find(|c| **c == text) {
            Some(c) => Ok(Value::Choice(c)),
            None => Err(bad(
                def,
                format!("`{text}` is not one of {}", choices.join(", ")),
            )),
        },
        ParamKind::Checkbox { .. } => match text {
            "1" | "true" | "on" => Ok(Value::Flag(true)),
            "0" | "false" | "off" => Ok(Value::Flag(false)),
            _ => Err(bad(def, format!("`{text}` is not 1 or 0"))),
        },
        ParamKind::Text { .. } => Ok(Value::Text(text.to_string())),
    }
}

impl Values {
    /// The defaults of every parameter.
    pub fn defaults(def: &'static StageDef) -> Values {
        Values {
            defs: def.params,
            values: def.params.iter().map(default_of).collect(),
        }
    }

    /// Check the `--param` pairs of a command line against a stage.
    pub fn parse(def: &'static StageDef, pairs: &[(String, String)]) -> Result<Values, StageError> {
        let mut out = Values::defaults(def);
        let mut seen: Vec<&str> = Vec::new();
        for (key, text) in pairs {
            if def.params.is_empty() {
                return Err(StageError::unsupported(format!(
                    "{} takes no parameters: `{key}` is not one",
                    def.id
                )));
            }
            if seen.contains(&key.as_str()) {
                return Err(StageError::usage(format!(
                    "parameter `{key}` given more than once"
                )));
            }
            let Some(index) = def.params.iter().position(|p| p.id == key) else {
                let ids: Vec<&str> = def.params.iter().map(|p| p.id).collect();
                return Err(StageError::unsupported(format!(
                    "{} has no parameter `{key}`: it takes {}",
                    def.id,
                    ids.join(", ")
                )));
            };
            out.values[index] = parse_value(&def.params[index], text)?;
            seen.push(key);
        }
        Ok(out)
    }

    fn get(&self, id: &str) -> &Value {
        let index = self
            .defs
            .iter()
            .position(|p| p.id == id)
            .unwrap_or_else(|| panic!("the stage definition has no parameter `{id}`"));
        &self.values[index]
    }

    /// The value of the whole number parameter `id`. Panics when the definition has no such
    /// parameter of that kind: that is an error of the stage module, not of a request.
    pub fn int(&self, id: &str) -> i64 {
        match self.get(id) {
            Value::Int(v) => *v,
            other => panic!("parameter `{id}` is not a whole number: {other:?}"),
        }
    }

    /// The value of the float parameter `id`. Panics as [`Values::int`] does.
    pub fn float(&self, id: &str) -> f64 {
        match self.get(id) {
            Value::Float(v) => *v,
            other => panic!("parameter `{id}` is not a float: {other:?}"),
        }
    }

    /// The chosen word of the choice parameter `id`. Panics as [`Values::int`] does.
    pub fn choice(&self, id: &str) -> &'static str {
        match self.get(id) {
            Value::Choice(v) => v,
            other => panic!("parameter `{id}` is not a choice: {other:?}"),
        }
    }

    /// The value of the checkbox parameter `id`. Panics as [`Values::int`] does.
    pub fn flag(&self, id: &str) -> bool {
        match self.get(id) {
            Value::Flag(v) => *v,
            other => panic!("parameter `{id}` is not a checkbox: {other:?}"),
        }
    }

    /// The value of the text parameter `id`. Panics as [`Values::int`] does.
    pub fn text(&self, id: &str) -> &str {
        match self.get(id) {
            Value::Text(v) => v,
            other => panic!("parameter `{id}` is not a text: {other:?}"),
        }
    }

    /// Replace the value of the float parameter `id`; used for the value of a key track at a
    /// frame. An id that is not a float parameter of the stage is left alone.
    pub fn set_float(&mut self, id: &str, value: f64) {
        if let Some(index) = self.defs.iter().position(|p| p.id == id) {
            if let Value::Float(slot) = &mut self.values[index] {
                *slot = value;
            }
        }
    }
}

/// The layer paths of a command line, in the order of the definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layers {
    defs: &'static [LayerDef],
    paths: Vec<Option<String>>,
}

impl Layers {
    /// Check the `--layer` pairs of a command line against a stage. No file is opened here.
    pub fn parse(def: &'static StageDef, pairs: &[(String, String)]) -> Result<Layers, StageError> {
        let mut paths: Vec<Option<String>> = vec![None; def.layers.len()];
        for (id, path) in pairs {
            if def.layers.is_empty() {
                return Err(StageError::unsupported(format!(
                    "{} takes no layers: `{id}` is not one",
                    def.id
                )));
            }
            let Some(index) = def.layers.iter().position(|l| l.id == id) else {
                let ids: Vec<&str> = def.layers.iter().map(|l| l.id).collect();
                return Err(StageError::unsupported(format!(
                    "{} has no layer `{id}`: it takes {}",
                    def.id,
                    ids.join(", ")
                )));
            };
            if paths[index].is_some() {
                return Err(StageError::usage(format!(
                    "layer `{id}` given more than once"
                )));
            }
            if path.is_empty() {
                return Err(StageError::usage(format!("layer `{id}` has an empty path")));
            }
            paths[index] = Some(path.clone());
        }
        if let Some(missing) = def
            .layers
            .iter()
            .zip(&paths)
            .find(|(l, p)| !l.optional && p.is_none())
        {
            return Err(StageError::usage(format!(
                "{} needs --layer {}=PATH",
                def.id, missing.0.id
            )));
        }
        Ok(Layers {
            defs: def.layers,
            paths,
        })
    }

    /// The path given for layer `id`, if any.
    pub fn path(&self, id: &str) -> Option<&str> {
        let index = self.defs.iter().position(|l| l.id == id)?;
        self.paths[index].as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Code;
    use crate::registry::{Geometry, Kind, Label};

    static DEF: StageDef = StageDef {
        id: "sample-stage",
        name: "Sample",
        about: "A made up stage for the tests of the parser.",
        label: Label::Generic,
        kind: Kind::Frame,
        spool: false,
        geometry: Geometry::Preserve,
        depths: &[8],
        params: &[
            ParamDef {
                id: "count",
                name: "Count",
                kind: ParamKind::Int {
                    default: 3,
                    min: -2,
                    max: 9,
                },
                keyed: false,
                about: "A made up whole number.",
            },
            ParamDef {
                id: "gain",
                name: "Gain",
                kind: ParamKind::Float {
                    default: 0.5,
                    min: 0.0,
                    max: 2.0,
                },
                keyed: true,
                about: "A made up float.",
            },
            ParamDef {
                id: "mode",
                name: "Mode",
                kind: ParamKind::Choice {
                    default: "plain",
                    choices: &["plain", "other-one"],
                },
                keyed: false,
                about: "A made up choice.",
            },
            ParamDef {
                id: "flip",
                name: "Flip",
                kind: ParamKind::Checkbox { default: false },
                keyed: false,
                about: "A made up checkbox.",
            },
            ParamDef {
                id: "note",
                name: "Note",
                kind: ParamKind::Text { default: "" },
                keyed: false,
                about: "A made up text.",
            },
        ],
        layers: &[
            LayerDef {
                id: "keys",
                optional: true,
                form: "keys-1",
                about: "A key file.",
            },
            LayerDef {
                id: "matte",
                optional: false,
                form: "frames",
                about: "A made up required layer.",
            },
        ],
    };

    static BARE: StageDef = StageDef {
        id: "bare-stage",
        name: "Bare",
        about: "A made up stage without parameters.",
        label: Label::SelfTest,
        kind: Kind::Frame,
        spool: false,
        geometry: Geometry::Preserve,
        depths: &[8],
        params: &[],
        layers: &[],
    };

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn refused(list: &[(&str, &str)]) -> StageError {
        Values::parse(&DEF, &pairs(list)).expect_err("refused")
    }

    #[test]
    fn defaults_fill_every_parameter() {
        let v = Values::parse(&DEF, &[]).unwrap();
        assert_eq!(v.int("count"), 3);
        assert_eq!(v.float("gain"), 0.5);
        assert_eq!(v.choice("mode"), "plain");
        assert!(!v.flag("flip"));
        assert_eq!(v.text("note"), "");
        assert_eq!(v, Values::defaults(&DEF));
    }

    #[test]
    fn typed_values_parse() {
        let v = Values::parse(
            &DEF,
            &pairs(&[
                ("count", "-2"),
                ("gain", "1.25"),
                ("mode", "other-one"),
                ("flip", "on"),
                ("note", "a=b@c"),
            ]),
        )
        .unwrap();
        assert_eq!(v.int("count"), -2);
        assert_eq!(v.float("gain"), 1.25);
        assert_eq!(v.choice("mode"), "other-one");
        assert!(v.flag("flip"));
        assert_eq!(v.text("note"), "a=b@c");
    }

    #[test]
    fn checkbox_words() {
        for (word, want) in [
            ("1", true),
            ("true", true),
            ("on", true),
            ("0", false),
            ("false", false),
            ("off", false),
        ] {
            let v = Values::parse(&DEF, &pairs(&[("flip", word)])).unwrap();
            assert_eq!(v.flag("flip"), want, "{word}");
        }
        assert_eq!(refused(&[("flip", "yes")]).code, Code::Unsupported);
    }

    #[test]
    fn an_unknown_id_is_unsupported_and_names_the_parameters() {
        let e = refused(&[("nope", "1")]);
        assert_eq!(e.code, Code::Unsupported);
        assert_eq!(
            e.message,
            "sample-stage has no parameter `nope`: it takes count, gain, mode, flip, note"
        );
    }

    #[test]
    fn a_stage_without_parameters_says_so() {
        let e = Values::parse(&BARE, &pairs(&[("mode", "fast")])).unwrap_err();
        assert_eq!(e.code, Code::Unsupported);
        assert_eq!(
            e.message,
            "bare-stage takes no parameters: `mode` is not one"
        );
        let e = Layers::parse(&BARE, &pairs(&[("alt", "some.bin")])).unwrap_err();
        assert_eq!(e.code, Code::Unsupported);
        assert_eq!(e.message, "bare-stage takes no layers: `alt` is not one");
    }

    #[test]
    fn an_id_given_twice_is_usage() {
        let e = refused(&[("count", "1"), ("count", "1")]);
        assert_eq!(e.code, Code::Usage);
        assert_eq!(e.exit(), 2);
    }

    #[test]
    fn bad_values_are_unsupported() {
        for (key, text) in [
            ("count", "10"),
            ("count", "-3"),
            ("count", "1.0"),
            ("count", "+1"),
            ("count", ""),
            ("count", "x"),
            ("count", "99999999999999999999"),
            ("gain", "2.5"),
            ("gain", "-0.1"),
            ("gain", "nan"),
            ("gain", "inf"),
            ("gain", "+1"),
            ("gain", ".5"),
            ("gain", "1e999"),
            ("gain", ""),
            ("mode", "Plain"),
            ("mode", "1"),
            ("flip", "2"),
        ] {
            let e = refused(&[(key, text)]);
            assert_eq!(e.code, Code::Unsupported, "{key}={text}");
            assert!(
                e.message.starts_with(&format!("bad value for `{key}`: ")),
                "{}",
                e.message
            );
        }
    }

    #[test]
    fn float_forms() {
        assert_eq!(parse_float("0"), Some(0.0));
        assert_eq!(parse_float("-12.5"), Some(-12.5));
        assert_eq!(parse_float("1e-3"), Some(0.001));
        assert_eq!(parse_float("2E2"), Some(200.0));
        for bad in ["", "-", "e3", "1 ", " 1", "0x10", "1,5", "infinity", "NaN"] {
            assert_eq!(parse_float(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_keyed_value_on_the_command_line_is_refused() {
        let e = refused(&[("gain", "1@0;2@10")]);
        assert_eq!(e.code, Code::Unsupported);
        assert!(e.message.contains("--layer keys=PATH"), "{}", e.message);
        assert_eq!(refused(&[("count", "1@0")]).code, Code::Unsupported);
    }

    #[test]
    fn set_float_replaces_only_floats() {
        let mut v = Values::defaults(&DEF);
        v.set_float("gain", 1.5);
        v.set_float("count", 7.0);
        v.set_float("absent", 7.0);
        assert_eq!((v.float("gain"), v.int("count")), (1.5, 3));
    }

    #[test]
    #[should_panic(expected = "has no parameter `absent`")]
    fn asking_for_an_undeclared_parameter_panics() {
        Values::defaults(&DEF).int("absent");
    }

    #[test]
    fn layers_parse_and_report() {
        let l = Layers::parse(&DEF, &pairs(&[("matte", "m.raw"), ("keys", "k.txt")])).unwrap();
        assert_eq!(l.path("matte"), Some("m.raw"));
        assert_eq!(l.path("keys"), Some("k.txt"));
        assert_eq!(l.path("absent"), None);
        let l = Layers::parse(&DEF, &pairs(&[("matte", "m.raw")])).unwrap();
        assert_eq!(l.path("keys"), None);
    }

    #[test]
    fn layer_refusals() {
        let e = Layers::parse(&DEF, &pairs(&[("keys", "k.txt")])).unwrap_err();
        assert_eq!(e.code, Code::Usage);
        assert_eq!(e.message, "sample-stage needs --layer matte=PATH");
        let e = Layers::parse(&DEF, &pairs(&[("matte", "a"), ("matte", "b")])).unwrap_err();
        assert_eq!(e.code, Code::Usage);
        let e = Layers::parse(&DEF, &pairs(&[("matte", "")])).unwrap_err();
        assert_eq!(e.code, Code::Usage);
        let e = Layers::parse(&DEF, &pairs(&[("matte", "a"), ("alt", "b")])).unwrap_err();
        assert_eq!(e.code, Code::Unsupported);
        assert_eq!(
            e.message,
            "sample-stage has no layer `alt`: it takes keys, matte"
        );
    }
}
