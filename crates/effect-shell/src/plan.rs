//! The parameter plan: an effect-params table plus optional directive lines.
//!
//! The table text is the effect-params format (magic `table v1`, six tab separated cells). The
//! shell adds information the six cells cannot carry through comment lines that start with `#@`;
//! effect-params ignores them as comments, so the same file stays valid for both crates.
//!
//! * `#@global` followed by tab separated `key=value` pairs sets `my_version`, `out_flags`,
//!   `out_flags2` (numbers, decimal or 0x hex) and `about` (text of the About box).
//! * `#@shell` followed by `key=value` pairs sets the registration options: `reset` (`each`:
//!   the definition structure is refilled before every row, `never`: one structure is reused so
//!   a field the table does not state keeps what the previous row left), `fill` (the byte the
//!   structure is filled with, default 0), `refcon` (the reference constant written into every
//!   arbitrary data definition, default 0) and `register_ui` (the events word of the final
//!   custom ui registration call; without it no such call is made), `placeholder_name` and
//!   `placeholder_flags` (the name and host flags word of the hidden placeholder button that an
//!   `ae_only` row becomes in another host; default: empty name, flags 0).
//! * Any other `#@` line holds tab separated `key=value` pairs for the NEXT data row:
//!   `type` (slider, fixslider, angle, checkbox, color, point, popup, custom, no_data, float,
//!   arb, path, group_start, group_end, button, point3d, layer), `id`, `ui`, `ui_w`, `ui_h`,
//!   `sdkflags` (replaces the mapped flag word), `choices` (a `|` list), `choice_bits` (a `,`
//!   list, one entry per choice, `-` for always shown, otherwise the engine flags bit that must
//!   be set), `vis_bit` (the row is invisible unless that engine flags bit is set), `ae_only`
//!   (1: in any other host the row becomes a hidden placeholder button), `label`, `slider`
//!   (`min,max`), `precision`, `display`, `fs_flags`, `tol`, `xy` (`x,y` for a point) and
//!   `restrict` (1), `ui_suite` (text: before this row is added, the effect ui suite is acquired
//!   and the options button name is set to the text, see setup.rs) and `defer_err` (1: a host
//!   error for this row is remembered and returned at the end instead of stopping the setup).
//!
//! Version 2 tables (effect-params format version 2) state `id`, `host_flags`, `host_ui_flags`
//! and the extra cell (choices, slider, point, button label, arbitrary data bytes) per row; the
//! shell writes exactly those fields. A field a version 2 row does not state is not written at
//! all and keeps the content of the structure (see `reset`). A name of 32 bytes or more is
//! refused (a version 1 table still cuts at 31 bytes).

#[cfg(feature = "freestanding")]
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;
use effect_params::{Cell, Extra, ParamKind, Table};

use crate::layout::{
    ParamType, PARAM_FLAG_CANNOT_TIME_VARY, PARAM_FLAG_START_COLLAPSED, PARAM_FLAG_SUPERVISE,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Globals {
    pub my_version: u32,
    pub out_flags: u32,
    pub out_flags2: u32,
    pub about: String,
}

/// How the definition structure is prepared before each row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reset {
    /// Filled with the fill byte before every row.
    Each,
    /// Filled once before the first row; later rows write over the previous row's content.
    Never,
}

/// Registration options of the plan (the `#@shell` line).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub reset: Reset,
    pub fill: u8,
    pub refcon: u64,
    pub register_ui: Option<u32>,
    /// Name written into the placeholder button of an `ae_only` row in another host (`#@shell placeholder_name`).
    pub placeholder_name: String,
    /// Host flags word of that placeholder (`#@shell placeholder_flags`).
    pub placeholder_flags: u32,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            reset: Reset::Each,
            fill: 0,
            refcon: 0,
            register_ui: None,
            placeholder_name: String::new(),
            placeholder_flags: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub name: String,
    pub ty: ParamType,
    /// None: the table does not state it and the field is not written.
    pub id: Option<i32>,
    pub ui_flags: Option<u32>,
    pub ui_width: Option<i16>,
    pub ui_height: Option<i16>,
    pub flags: Option<u32>,
    /// Arbitrary data bytes (the extra cell `d:`), the content of the default handle.
    pub data: Vec<u8>,
    pub ui_suite: Option<String>,
    pub defer_err: bool,
    /// Version 1 convention: a popup default is a 1 based index kept inside the shown list.
    pub clamp_default: bool,
    pub default: Cell,
    pub min: Cell,
    pub max: Cell,
    pub choices: Vec<(String, Option<u8>)>,
    pub vis_bit: Option<u8>,
    pub ae_only: bool,
    pub label: String,
    pub slider: Option<(f64, f64)>,
    pub precision: i16,
    pub display: i16,
    pub fs_flags: i32,
    pub tolerance: f32,
    pub xy: Option<(f64, f64)>,
    pub restrict: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub globals: Globals,
    pub options: Options,
    /// Table format version, 1 or 2.
    pub version: u8,
    pub rows: Vec<Row>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlanError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl core::error::Error for PlanError {}

pub fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim();
    let (neg, body) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let v = if let Some(h) = body.strip_prefix("0x") {
        i64::from_str_radix(h, 16).ok()?
    } else {
        body.parse::<i64>().ok()?
    };
    Some(if neg { -v } else { v })
}

fn parse_pair(s: &str) -> Option<(f64, f64)> {
    let (a, b) = s.split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn type_from_name(s: &str) -> Option<ParamType> {
    Some(match s {
        "layer" => ParamType::Layer,
        "slider" => ParamType::Slider,
        "fixslider" => ParamType::FixSlider,
        "angle" => ParamType::Angle,
        "checkbox" => ParamType::Checkbox,
        "color" => ParamType::Color,
        "point" => ParamType::Point,
        "popup" => ParamType::Popup,
        "custom" => ParamType::Custom,
        "no_data" => ParamType::NoData,
        "float" => ParamType::FloatSlider,
        "arb" => ParamType::ArbitraryData,
        "path" => ParamType::Path,
        "group_start" => ParamType::GroupStart,
        "group_end" => ParamType::GroupEnd,
        "button" => ParamType::Button,
        "point3d" => ParamType::Point3d,
        _ => return None,
    })
}

fn default_type(k: ParamKind) -> ParamType {
    match k {
        ParamKind::Float => ParamType::FloatSlider,
        ParamKind::Int => ParamType::Slider,
        ParamKind::Checkbox => ParamType::Checkbox,
        ParamKind::Popup => ParamType::Popup,
        ParamKind::Button => ParamType::Button,
        ParamKind::Layer => ParamType::Layer,
        ParamKind::Point => ParamType::Point,
        ParamKind::Angle => ParamType::Angle,
        ParamKind::Color => ParamType::Color,
        ParamKind::Group => ParamType::GroupStart,
        ParamKind::GroupEnd => ParamType::GroupEnd,
        ParamKind::Arbitrary => ParamType::ArbitraryData,
        ParamKind::NoData => ParamType::NoData,
    }
}

/// The SDK flag word for this crate's own flag bits (SUPERVISED to [`PARAM_FLAG_SUPERVISE`],
/// COLLAPSED to [`PARAM_FLAG_START_COLLAPSED`], NOT_ANIMATABLE to
/// [`PARAM_FLAG_CANNOT_TIME_VARY`]). HIDDEN and REDRAW_ON_CHANGE have no flag word bit here.
pub fn map_flags(f: effect_params::ParamFlags) -> u32 {
    let mut out = 0;
    if f.contains(effect_params::ParamFlags::SUPERVISED) {
        out |= PARAM_FLAG_SUPERVISE;
    }
    if f.contains(effect_params::ParamFlags::COLLAPSED) {
        out |= PARAM_FLAG_START_COLLAPSED;
    }
    if f.contains(effect_params::ParamFlags::NOT_ANIMATABLE) {
        out |= PARAM_FLAG_CANNOT_TIME_VARY;
    }
    out
}

type Pairs = Vec<(String, String)>;

fn pairs(rest: &str, line: usize) -> Result<Pairs, PlanError> {
    let mut out = Vec::new();
    for item in rest.split('\t').filter(|s| !s.is_empty()) {
        let (k, v) = item.split_once('=').ok_or_else(|| PlanError {
            line,
            message: format!("directive item without '=': {item:?}"),
        })?;
        out.push((k.trim().to_string(), v.to_string()));
    }
    Ok(out)
}

fn num(v: &str, line: usize, key: &str) -> Result<i64, PlanError> {
    parse_int(v).ok_or_else(|| PlanError {
        line,
        message: format!("bad number for {key}: {v:?}"),
    })
}

/// Copies the stated extra cell of a row into the row's own fields.
fn apply_extra(row: &mut Row, extra: &Extra, line: usize) -> Result<(), PlanError> {
    match extra {
        Extra::Unknown | Extra::Absent => {}
        Extra::Choices(c) => row.choices = c.iter().map(|s| (s.clone(), None)).collect(),
        Extra::Slider {
            precision,
            display,
            slider_min,
            slider_max,
        } => {
            row.precision = *precision as i16;
            row.display = i16::try_from(*display).map_err(|_| PlanError {
                line,
                message: format!("slider display word {display:#x} does not fit 16 bits"),
            })?;
            row.slider = Some((f64::from_bits(*slider_min), f64::from_bits(*slider_max)));
        }
        Extra::Point { x, y } => row.xy = Some((f64::from_bits(*x), f64::from_bits(*y))),
        Extra::Data(d) => row.data = d.clone(),
        Extra::Label(l) => row.label = l.clone(),
    }
    Ok(())
}

impl Plan {
    pub fn parse(text: &str) -> Result<Plan, PlanError> {
        let table = Table::parse(text).map_err(|e| PlanError {
            line: e.line,
            message: e.message,
        })?;
        let mut globals = Globals::default();
        let mut options = Options::default();
        let mut version = 1u8;
        let mut pending: Vec<(usize, Pairs)> = Vec::new();
        let mut rows = Vec::new();
        let mut seen_magic = false;
        let mut idx = 0usize;
        for (i, raw) in text.lines().enumerate() {
            let line = i + 1;
            if let Some(rest) = raw.strip_prefix("#@global") {
                for (k, v) in pairs(rest, line)? {
                    match k.as_str() {
                        "my_version" => globals.my_version = num(&v, line, &k)? as u32,
                        "out_flags" => globals.out_flags = num(&v, line, &k)? as u32,
                        "out_flags2" => globals.out_flags2 = num(&v, line, &k)? as u32,
                        "about" => globals.about = v,
                        _ => {
                            return Err(PlanError {
                                line,
                                message: format!("unknown global key {k:?}"),
                            })
                        }
                    }
                }
                continue;
            }
            if let Some(rest) = raw.strip_prefix("#@shell") {
                for (k, v) in pairs(rest, line)? {
                    match k.as_str() {
                        "reset" => {
                            options.reset = match v.as_str() {
                                "each" => Reset::Each,
                                "never" => Reset::Never,
                                _ => {
                                    return Err(PlanError {
                                        line,
                                        message: format!("bad reset mode {v:?}"),
                                    })
                                }
                            }
                        }
                        "fill" => {
                            let n = num(&v, line, &k)?;
                            if !(0..=255).contains(&n) {
                                return Err(PlanError {
                                    line,
                                    message: format!("fill byte out of range: {n}"),
                                });
                            }
                            options.fill = n as u8
                        }
                        "refcon" => options.refcon = num(&v, line, &k)? as u64,
                        "register_ui" => options.register_ui = Some(num(&v, line, &k)? as u32),
                        "placeholder_name" => options.placeholder_name = v,
                        "placeholder_flags" => {
                            options.placeholder_flags = num(&v, line, &k)? as u32
                        }
                        _ => {
                            return Err(PlanError {
                                line,
                                message: format!("unknown shell key {k:?}"),
                            })
                        }
                    }
                }
                continue;
            }
            if let Some(rest) = raw.strip_prefix("#@") {
                pending.push((line, pairs(rest, line)?));
                continue;
            }
            if raw.trim().is_empty() || raw.starts_with('#') {
                continue;
            }
            if !seen_magic {
                seen_magic = true;
                if raw.trim() == "table v2" {
                    version = 2;
                }
                continue;
            }
            let def = &table.params[idx];
            idx += 1;
            let v2 = version == 2;
            let to_i32 = |v: i64| {
                i32::try_from(v).map_err(|_| PlanError {
                    line,
                    message: format!("id {v} does not fit 32 bits"),
                })
            };
            if v2 && def.name.len() > 31 {
                return Err(PlanError {
                    line,
                    message: format!(
                        "name of {} bytes does not fit the 32 byte field",
                        def.name.len()
                    ),
                });
            }
            let mut row = Row {
                name: def.name.clone(),
                ty: default_type(def.kind),
                id: if v2 {
                    def.id.map(to_i32).transpose()?
                } else {
                    Some(0)
                },
                ui_flags: if v2 { def.host_ui_flags } else { Some(0) },
                ui_width: if v2 { None } else { Some(0) },
                ui_height: if v2 { None } else { Some(0) },
                flags: if v2 {
                    def.host_flags
                } else {
                    Some(def.flags.map(map_flags).unwrap_or(0))
                },
                data: Vec::new(),
                ui_suite: None,
                defer_err: false,
                clamp_default: !v2,
                default: def.default,
                min: def.min,
                max: def.max,
                choices: Vec::new(),
                vis_bit: None,
                ae_only: false,
                label: String::new(),
                slider: None,
                precision: 0,
                display: 0,
                fs_flags: 0,
                tolerance: 0.0,
                xy: None,
                restrict: false,
            };
            if !v2
                && def
                    .flags
                    .map(|f| f.contains(effect_params::ParamFlags::HIDDEN))
                    == Some(true)
            {
                row.ui_flags = Some(crate::layout::PUI_INVISIBLE);
            }
            apply_extra(&mut row, &def.extra, line)?;
            let mut bits: Option<(usize, String)> = None;
            for (l, ps) in pending.drain(..) {
                for (k, v) in ps {
                    match k.as_str() {
                        "type" => {
                            row.ty = type_from_name(&v).ok_or_else(|| PlanError {
                                line: l,
                                message: format!("unknown type {v:?}"),
                            })?
                        }
                        "id" => row.id = Some(num(&v, l, &k)? as i32),
                        "ui" => {
                            row.ui_flags = Some(row.ui_flags.unwrap_or(0) | num(&v, l, &k)? as u32)
                        }
                        "ui_w" => row.ui_width = Some(num(&v, l, &k)? as i16),
                        "ui_h" => row.ui_height = Some(num(&v, l, &k)? as i16),
                        "sdkflags" => row.flags = Some(num(&v, l, &k)? as u32),
                        "ui_suite" => row.ui_suite = Some(v),
                        "defer_err" => row.defer_err = num(&v, l, &k)? != 0,
                        "choices" => {
                            row.choices = v.split('|').map(|s| (s.to_string(), None)).collect()
                        }
                        "choice_bits" => bits = Some((l, v)),
                        "vis_bit" => {
                            let b = num(&v, l, &k)?;
                            if !(0..64).contains(&b) {
                                return Err(PlanError {
                                    line: l,
                                    message: format!("vis_bit out of range: {b}"),
                                });
                            }
                            row.vis_bit = Some(b as u8)
                        }
                        "ae_only" => row.ae_only = num(&v, l, &k)? != 0,
                        "label" => row.label = v,
                        "slider" => {
                            row.slider = Some(parse_pair(&v).ok_or_else(|| PlanError {
                                line: l,
                                message: format!("bad slider pair {v:?}"),
                            })?)
                        }
                        "precision" => row.precision = num(&v, l, &k)? as i16,
                        "display" => row.display = num(&v, l, &k)? as i16,
                        "fs_flags" => row.fs_flags = num(&v, l, &k)? as i32,
                        "tol" => {
                            row.tolerance = v.trim().parse().map_err(|_| PlanError {
                                line: l,
                                message: format!("bad tolerance {v:?}"),
                            })?
                        }
                        "xy" => {
                            row.xy = Some(parse_pair(&v).ok_or_else(|| PlanError {
                                line: l,
                                message: format!("bad xy pair {v:?}"),
                            })?)
                        }
                        "restrict" => row.restrict = num(&v, l, &k)? != 0,
                        _ => {
                            return Err(PlanError {
                                line: l,
                                message: format!("unknown row key {k:?}"),
                            })
                        }
                    }
                }
            }
            if let Some((l, b)) = bits {
                let items: Vec<&str> = b.split(',').collect();
                if items.len() != row.choices.len() {
                    return Err(PlanError {
                        line: l,
                        message: format!(
                            "choice_bits has {} entries for {} choices",
                            items.len(),
                            row.choices.len()
                        ),
                    });
                }
                for (c, it) in row.choices.iter_mut().zip(items) {
                    c.1 = if it.trim() == "-" {
                        None
                    } else {
                        let n = num(it, l, "choice_bits")?;
                        if !(0..64).contains(&n) {
                            return Err(PlanError {
                                line: l,
                                message: format!("choice bit out of range: {n}"),
                            });
                        }
                        Some(n as u8)
                    };
                }
            }
            rows.push(row);
        }
        if let Some((l, _)) = pending.first() {
            return Err(PlanError {
                line: *l,
                message: "directive line with no row after it".into(),
            });
        }
        Ok(Plan {
            globals,
            options,
            version,
            rows,
        })
    }
}

impl Row {
    /// The choices shown for this engine flags word.
    pub fn choices_for(&self, engine_flags: u64) -> Vec<&str> {
        self.choices
            .iter()
            .filter(|(_, g)| g.is_none_or(|b| engine_flags >> b & 1 == 1))
            .map(|(s, _)| s.as_str())
            .collect()
    }

    /// The ui flags for this engine flags word: the stated bits, plus INVISIBLE when a gate bit is clear.
    pub fn ui_for(&self, engine_flags: u64) -> Option<u32> {
        match self.vis_bit {
            Some(b) if engine_flags >> b & 1 == 0 => {
                Some(self.ui_flags.unwrap_or(0) | crate::layout::PUI_INVISIBLE)
            }
            _ => self.ui_flags,
        }
    }
}
