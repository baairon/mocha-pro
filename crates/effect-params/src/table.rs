//! Parameter definitions and the predicted-table text format.
//!
//! # Version 2 (written by `serialize`)
//!
//! UTF-8 text. Line ends are LF; CRLF is also read (the carriage return is dropped), and
//! `serialize` writes LF only. A line that is empty or starts with `#` in column one is
//! ignored, and so is a line of whitespace without a tab. A line that holds a tab, after the
//! magic of a version 2 file, is a row and is never skipped: a line of only tabs is refused
//! as a row with empty cells. The first other line is the magic `table v2` (leading and
//! trailing whitespace allowed). Each row has exactly ten tab-separated cells, in this order:
//!
//! 1. `id`: the host parameter id, `?` (unknown) or `i:<decimal>` (a signed 64 bit integer,
//!    `-` allowed, no `+`). A known id is unique in a table.
//! 2. `name`: the parameter name with the escapes `\\`, `\t`, `\n`, `\r` (any other
//!    backslash sequence is an error; `\e` is not a name escape). It may be EMPTY, but only
//!    when the id is known (the id is then the row's key).
//! 3. `kind`: a kind name (see [`ParamKind`]).
//! 4. `default`, 5. `min`, 6. `max`: value cells: `?`, `i:<decimal>` (no `+`), `f:<decimal>`
//!    (the standard float syntax: optional `-`, digits, optional fraction and exponent; no
//!    leading `+`, and finite: `nan`, `inf` and an overflowing exponent are refused) or
//!    `b:<16 hex digits>` (float bits, either digit case). The default cell of a `layer` row
//!    may also be `none`: the layer default is "no layer", which is not the same as unknown
//!    and not the same as layer index 0. `none` is refused in every other cell.
//! 7. `flags`: this crate's own five flag bits, `?` or `0x<hex>` (lower case `0x`, one or more
//!    hex digits of either case, leading zeros allowed, value at most 32 bits).
//! 8. `host_flags`: the raw host flags word, `?` or `0x<1 to 8 hex digits>` (32 bit).
//! 9. `host_ui_flags`: the raw host ui flags word, same form.
//! 10. `extra`: the kind specific extra cell of [`Extra`] (strict form), `?` when unknown.
//!
//! `serialize` writes only the canonical spelling: lower case hex digits, no leading zeros in
//! the words, floats as `b:`. It writes nothing that the parser refuses: [`Table::validate`]
//! is applied first, and `serialize` panics on an invalid table (see
//! [`Table::try_serialize`] for the `Result` form). A missing magic is reported at the line
//! after the last line of the text (line 1 for an empty text).
//!
//! # Version 1 (still read)
//!
//! Format: UTF-8 lines. Lines starting with `#` and blank lines are ignored. The first
//! data line is the magic `table v1`. Each row has six tab-separated cells:
//! `name`, `kind`, `default`, `min`, `max`, `flags`, and optionally a seventh `extra` cell.
//! The name cell is raw text: version 1 has NO escapes, a backslash is a backslash. Whitespace
//! lines, also with tabs, are blank. Version 1 keeps the standard integer and float parsers'
//! full leniency (`i:+5`, `f:nan`, `f:+1.5`) and the lenient extra cell (empty choice or label
//! written as nothing, a plus sign in the slider precision).
//! A value cell is `?` (unknown), `i:<decimal>` (integer), `f:<decimal>` (float, parsed
//! on load) or `b:<16 hex digits>` (float as its exact bit pattern). The flags cell is
//! `?` or `0x<hex>`. The serializer always writes floats as `b:` so a round trip is exact.

use crate::{extra::Extra, flags::ParamFlags, kind::ParamKind};
use std::fmt;

/// A table cell that may be unknown. Unknown is never the same as zero.
#[derive(Clone, Copy, Debug)]
pub enum Cell {
    Unknown,
    Int(i64),
    Float(f64),
    /// "No value": the default of a layer parameter that names no layer. Text form `none`,
    /// legal only in the default cell of a `layer` row of a version 2 table.
    None,
}

impl Cell {
    /// Bit identity: floats by `to_bits` (so NaN payloads and signed zero count), ints by value.
    pub fn bits_eq(&self, other: &Cell) -> bool {
        match (self, other) {
            (Cell::Unknown, Cell::Unknown) | (Cell::None, Cell::None) => true,
            (Cell::Int(a), Cell::Int(b)) => a == b,
            (Cell::Float(a), Cell::Float(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Cell::Unknown)
    }

    fn text(&self) -> String {
        match self {
            Cell::Unknown => "?".to_string(),
            Cell::None => "none".to_string(),
            Cell::Int(v) => format!("i:{v}"),
            Cell::Float(v) => format!("b:{:016x}", v.to_bits()),
        }
    }

    /// Parse a value cell. `strict` is the version 2 form: no plus sign before an integer or
    /// a float, and a float must be finite (`nan`, `inf`, `infinity` and an overflowing
    /// exponent are refused; write such a value as `b:`). Version 1 cells (`strict` false)
    /// keep the standard parsers' full leniency so that every file that loaded still loads.
    fn parse(s: &str, strict: bool) -> Result<Cell, String> {
        if s == "?" {
            return Ok(Cell::Unknown);
        }
        if let Some(r) = s.strip_prefix("i:") {
            if strict && r.starts_with('+') {
                return Err(format!("bad int {s:?}: plus sign"));
            }
            return r
                .parse::<i64>()
                .map(Cell::Int)
                .map_err(|e| format!("bad int {s:?}: {e}"));
        }
        if let Some(r) = s.strip_prefix("f:") {
            if strict && r.starts_with('+') {
                return Err(format!("bad float {s:?}: plus sign"));
            }
            let v = r
                .parse::<f64>()
                .map_err(|e| format!("bad float {s:?}: {e}"))?;
            if strict && !v.is_finite() {
                return Err(format!("bad float {s:?}: not finite, use b:"));
            }
            return Ok(Cell::Float(v));
        }
        if let Some(r) = s.strip_prefix("b:") {
            if r.len() != 16 {
                return Err(format!("bit cell needs 16 hex digits: {s:?}"));
            }
            if !all_hex_digits(r) {
                return Err(format!("bad bits {s:?}: {NOT_A_DIGIT}"));
            }
            return u64::from_str_radix(r, 16)
                .map(|b| Cell::Float(f64::from_bits(b)))
                .map_err(|e| format!("bad bits {s:?}: {e}"));
        }
        Err(format!("unrecognised cell {s:?}"))
    }
}

/// One parameter definition. `flags`, `id`, `host_flags` and `host_ui_flags` are `None` when
/// unknown. `flags` holds this crate's own five bits; `host_flags` and `host_ui_flags` hold the
/// raw words of the host registration, kept apart from them.
#[derive(Clone, Debug)]
pub struct ParamDef {
    pub name: String,
    pub kind: ParamKind,
    pub default: Cell,
    pub min: Cell,
    pub max: Cell,
    pub flags: Option<ParamFlags>,
    pub extra: Extra,
    pub id: Option<i64>,
    pub host_flags: Option<u32>,
    pub host_ui_flags: Option<u32>,
}

impl ParamDef {
    pub fn new(name: &str, kind: ParamKind) -> ParamDef {
        ParamDef {
            name: name.to_string(),
            kind,
            default: Cell::Unknown,
            min: Cell::Unknown,
            max: Cell::Unknown,
            flags: None,
            extra: Extra::Unknown,
            id: None,
            host_flags: None,
            host_ui_flags: None,
        }
    }
}

/// An ordered list of parameter definitions.
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub params: Vec<ParamDef>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// A table that [`Table::serialize`] would write in a form the parser refuses.
#[derive(Debug, PartialEq, Eq)]
pub struct ValidateError {
    /// 1 based row number in table order.
    pub row: usize,
    pub message: String,
}

impl fmt::Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "row {}: {}", self.row, self.message)
    }
}

impl std::error::Error for ValidateError {}

const MAGIC_V1: &str = "table v1";
const MAGIC_V2: &str = "table v2";

/// The text the standard integer parser gives for a byte that is not a digit; kept so that
/// the refusal of a sign reads like the refusal of any other bad digit.
const NOT_A_DIGIT: &str = "invalid digit found in string";

/// True when every byte is an ASCII hexadecimal digit (either case). The integer parser also
/// takes a leading sign, which a hexadecimal cell does not have.
fn all_hex_digits(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A flags cell of the own bits: `?` or `0x<hex>`.
fn parse_flags(cell: &str) -> Result<Option<ParamFlags>, String> {
    if cell == "?" {
        return Ok(None);
    }
    if let Some(h) = cell.strip_prefix("0x") {
        if !all_hex_digits(h) {
            return Err(format!("bad flags: {NOT_A_DIGIT}"));
        }
        return u32::from_str_radix(h, 16)
            .map(|v| Some(ParamFlags(v)))
            .map_err(|e| format!("bad flags: {e}"));
    }
    Err(format!("bad flags cell {cell:?}"))
}

/// A raw host word cell of version 2: `?` or `0x` and one to eight hexadecimal digits.
fn parse_word(cell: &str, what: &str) -> Result<Option<u32>, String> {
    if cell == "?" {
        return Ok(None);
    }
    let h = cell
        .strip_prefix("0x")
        .ok_or_else(|| format!("bad {what} cell {cell:?}"))?;
    if h.is_empty() || h.len() > 8 || !all_hex_digits(h) {
        return Err(format!("bad {what} cell {cell:?}: need 1 to 8 hex digits"));
    }
    u32::from_str_radix(h, 16)
        .map(Some)
        .map_err(|e| format!("bad {what} cell {cell:?}: {e}"))
}

fn parse_id(cell: &str) -> Result<Option<i64>, String> {
    if cell == "?" {
        return Ok(None);
    }
    let r = cell
        .strip_prefix("i:")
        .ok_or_else(|| format!("bad id cell {cell:?}"))?;
    if r.starts_with('+') {
        return Err(format!("bad id cell {cell:?}: plus sign"));
    }
    r.parse::<i64>()
        .map(Some)
        .map_err(|e| format!("bad id cell {cell:?}: {e}"))
}

fn escape_name(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\t' => o.push_str("\\t"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            c => o.push(c),
        }
    }
    o
}

fn unescape_name(s: &str) -> Result<String, String> {
    let mut o = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            o.push(c);
            continue;
        }
        match it.next() {
            Some('\\') => o.push('\\'),
            Some('t') => o.push('\t'),
            Some('n') => o.push('\n'),
            Some('r') => o.push('\r'),
            _ => return Err("bad escape in name".into()),
        }
    }
    Ok(o)
}

/// A blank line is empty, or holds only whitespace. In a version 2 body (`strict`) a line
/// that holds a tab is a row, possibly with empty cells, and is refused as such. Before the
/// magic line and in a version 1 body a whitespace line with tabs is still blank, as it
/// always was.
fn is_blank(raw: &str, strict: bool) -> bool {
    raw.is_empty() || (!(strict && raw.contains('\t')) && raw.trim().is_empty())
}

fn word_text(w: Option<u32>) -> String {
    match w {
        Some(v) => format!("0x{v:x}"),
        None => "?".to_string(),
    }
}

impl Table {
    /// Parse a version 1 or a version 2 table; the magic line decides.
    pub fn parse(text: &str) -> Result<Table, ParseError> {
        let mut version = 0u8;
        let mut params: Vec<ParamDef> = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = i + 1;
            let err = |message: String| ParseError { line, message };
            if is_blank(raw, version == 2) || raw.starts_with('#') {
                continue;
            }
            if version == 0 {
                version = match raw.trim() {
                    MAGIC_V1 => 1,
                    MAGIC_V2 => 2,
                    _ => return Err(err(format!("expected {MAGIC_V1:?} or {MAGIC_V2:?}"))),
                };
                continue;
            }
            let def = if version == 1 {
                parse_row_v1(raw).map_err(err)?
            } else {
                parse_row_v2(raw).map_err(err)?
            };
            if let Some(id) = def.id {
                if params.iter().any(|p| p.id == Some(id)) {
                    return Err(err(format!("duplicate id {id}")));
                }
            }
            params.push(def);
        }
        if version == 0 {
            return Err(ParseError {
                line: text.lines().count() + 1,
                message: format!("missing {MAGIC_V1:?} or {MAGIC_V2:?}"),
            });
        }
        Ok(Table { params })
    }

    /// Check that [`Table::serialize`] would write text that [`Table::parse`] accepts and
    /// reads back to the same table. Refused: a row with an empty name and no id, a known id
    /// used by two rows, `Cell::None` anywhere except the default of a `layer` row. Rows are
    /// numbered from 1 in table order.
    pub fn validate(&self) -> Result<(), ValidateError> {
        let mut seen: Vec<i64> = Vec::new();
        for (i, p) in self.params.iter().enumerate() {
            let row = i + 1;
            let bad = |message: String| ValidateError { row, message };
            if p.name.is_empty() && p.id.is_none() {
                return Err(bad("empty name needs a known id".into()));
            }
            if let Some(id) = p.id {
                if seen.contains(&id) {
                    return Err(bad(format!("duplicate id {id}")));
                }
                seen.push(id);
            }
            if matches!(p.default, Cell::None) && p.kind != ParamKind::Layer {
                return Err(bad("default none is only legal for a layer".into()));
            }
            if matches!(p.min, Cell::None) || matches!(p.max, Cell::None) {
                return Err(bad("none is only legal as the default of a layer".into()));
            }
        }
        Ok(())
    }

    /// [`Table::validate`] then write the table in version 2 form.
    pub fn try_serialize(&self) -> Result<String, ValidateError> {
        self.validate()?;
        Ok(self.write_v2())
    }

    /// Write the table in version 2 form (ten cells per row, see the module documentation).
    ///
    /// # Panics
    ///
    /// Panics when [`Table::validate`] refuses the table: the serializer never writes text
    /// that its own parser refuses. Use [`Table::try_serialize`] for a table that was built
    /// by hand and is not known to be valid. A table made by [`Table::parse`] is always valid.
    pub fn serialize(&self) -> String {
        match self.try_serialize() {
            Ok(t) => t,
            Err(e) => panic!("Table::serialize on an invalid table: {e}"),
        }
    }

    fn write_v2(&self) -> String {
        let mut out = String::from(MAGIC_V2);
        out.push('\n');
        for p in &self.params {
            let id = match p.id {
                Some(v) => format!("i:{v}"),
                None => "?".to_string(),
            };
            let flags = match p.flags {
                Some(f) => format!("0x{:x}", f.bits()),
                None => "?".to_string(),
            };
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                id,
                escape_name(&p.name),
                p.kind.as_str(),
                p.default.text(),
                p.min.text(),
                p.max.text(),
                flags,
                word_text(p.host_flags),
                word_text(p.host_ui_flags),
                p.extra.text()
            ));
        }
        out
    }
}

fn parse_row_v1(raw: &str) -> Result<ParamDef, String> {
    let cells: Vec<&str> = raw.split('\t').collect();
    if cells.len() != 6 && cells.len() != 7 {
        return Err(format!("expected 6 cells, found {}", cells.len()));
    }
    if cells[0].is_empty() {
        return Err("empty name".into());
    }
    let kind = ParamKind::parse(cells[1]).ok_or_else(|| format!("unknown kind {:?}", cells[1]))?;
    let extra = if cells.len() == 7 {
        Extra::parse(cells[6])?
    } else {
        Extra::Unknown
    };
    Ok(ParamDef {
        name: cells[0].to_string(),
        kind,
        default: Cell::parse(cells[2], false)?,
        min: Cell::parse(cells[3], false)?,
        max: Cell::parse(cells[4], false)?,
        flags: parse_flags(cells[5])?,
        extra,
        id: None,
        host_flags: None,
        host_ui_flags: None,
    })
}

fn parse_row_v2(raw: &str) -> Result<ParamDef, String> {
    let cells: Vec<&str> = raw.split('\t').collect();
    if cells.len() != 10 {
        return Err(format!("expected 10 cells, found {}", cells.len()));
    }
    let id = parse_id(cells[0])?;
    let name = unescape_name(cells[1])?;
    if name.is_empty() && id.is_none() {
        return Err("empty name needs a known id".into());
    }
    let kind = ParamKind::parse(cells[2]).ok_or_else(|| format!("unknown kind {:?}", cells[2]))?;
    let default = if cells[3] == "none" {
        if kind != ParamKind::Layer {
            return Err("default none is only legal for a layer".into());
        }
        Cell::None
    } else {
        Cell::parse(cells[3], true)?
    };
    Ok(ParamDef {
        name,
        kind,
        default,
        min: Cell::parse(cells[4], true)?,
        max: Cell::parse(cells[5], true)?,
        flags: parse_flags(cells[6])?,
        extra: Extra::parse_strict(cells[9])?,
        id,
        host_flags: parse_word(cells[7], "host flags")?,
        host_ui_flags: parse_word(cells[8], "host ui flags")?,
    })
}
