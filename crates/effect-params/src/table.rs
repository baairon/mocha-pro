//! Parameter definitions and the predicted-table text format.
//!
//! Format: UTF-8 lines. Lines starting with `#` and blank lines are ignored. The first
//! data line is the magic `table v1`. Each row has six tab-separated cells:
//! `name`, `kind`, `default`, `min`, `max`, `flags`.
//! A value cell is `?` (unknown), `i:<decimal>` (integer), `f:<decimal>` (float, parsed
//! on load) or `b:<16 hex digits>` (float as its exact bit pattern). The flags cell is
//! `?` or `0x<hex>`. The serializer always writes floats as `b:` so a round trip is exact.

use crate::{flags::ParamFlags, kind::ParamKind};
use std::fmt;

/// A table cell that may be unknown. Unknown is never the same as zero.
#[derive(Clone, Copy, Debug)]
pub enum Cell {
    Unknown,
    Int(i64),
    Float(f64),
}

impl Cell {
    /// Bit identity: floats by `to_bits` (so NaN payloads and signed zero count), ints by value.
    pub fn bits_eq(&self, other: &Cell) -> bool {
        match (self, other) {
            (Cell::Unknown, Cell::Unknown) => true,
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
            Cell::Int(v) => format!("i:{v}"),
            Cell::Float(v) => format!("b:{:016x}", v.to_bits()),
        }
    }

    fn parse(s: &str) -> Result<Cell, String> {
        if s == "?" {
            return Ok(Cell::Unknown);
        }
        if let Some(r) = s.strip_prefix("i:") {
            return r
                .parse::<i64>()
                .map(Cell::Int)
                .map_err(|e| format!("bad int {s:?}: {e}"));
        }
        if let Some(r) = s.strip_prefix("f:") {
            return r
                .parse::<f64>()
                .map(Cell::Float)
                .map_err(|e| format!("bad float {s:?}: {e}"));
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

/// One parameter definition. `flags` is `None` when unknown.
#[derive(Clone, Debug)]
pub struct ParamDef {
    pub name: String,
    pub kind: ParamKind,
    pub default: Cell,
    pub min: Cell,
    pub max: Cell,
    pub flags: Option<ParamFlags>,
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

const MAGIC: &str = "table v1";

/// The text the standard integer parser gives for a byte that is not a digit; kept so that
/// the refusal of a sign reads like the refusal of any other bad digit.
const NOT_A_DIGIT: &str = "invalid digit found in string";

/// True when every byte is an ASCII hexadecimal digit (either case). The integer parser also
/// takes a leading sign, which a hexadecimal cell does not have.
fn all_hex_digits(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_hexdigit())
}

impl Table {
    pub fn parse(text: &str) -> Result<Table, ParseError> {
        let mut seen_magic = false;
        let mut params = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = i + 1;
            let err = |message: String| ParseError { line, message };
            if raw.trim().is_empty() || raw.starts_with('#') {
                continue;
            }
            if !seen_magic {
                if raw.trim() != MAGIC {
                    return Err(err(format!("expected {MAGIC:?}")));
                }
                seen_magic = true;
                continue;
            }
            let cells: Vec<&str> = raw.split('\t').collect();
            if cells.len() != 6 {
                return Err(err(format!("expected 6 cells, found {}", cells.len())));
            }
            if cells[0].is_empty() {
                return Err(err("empty name".into()));
            }
            let kind = ParamKind::parse(cells[1])
                .ok_or_else(|| err(format!("unknown kind {:?}", cells[1])))?;
            let default = Cell::parse(cells[2]).map_err(err)?;
            let min = Cell::parse(cells[3]).map_err(err)?;
            let max = Cell::parse(cells[4]).map_err(err)?;
            let flags = if cells[5] == "?" {
                None
            } else if let Some(h) = cells[5].strip_prefix("0x") {
                if !all_hex_digits(h) {
                    return Err(err(format!("bad flags: {NOT_A_DIGIT}")));
                }
                Some(ParamFlags(
                    u32::from_str_radix(h, 16).map_err(|e| err(format!("bad flags: {e}")))?,
                ))
            } else {
                return Err(err(format!("bad flags cell {:?}", cells[5])));
            };
            params.push(ParamDef {
                name: cells[0].to_string(),
                kind,
                default,
                min,
                max,
                flags,
            });
        }
        if !seen_magic {
            return Err(ParseError {
                line: 0,
                message: format!("missing {MAGIC:?}"),
            });
        }
        Ok(Table { params })
    }

    pub fn serialize(&self) -> String {
        let mut out = String::from(MAGIC);
        out.push('\n');
        for p in &self.params {
            let flags = match p.flags {
                Some(f) => format!("0x{:x}", f.bits()),
                None => "?".to_string(),
            };
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                p.name,
                p.kind.as_str(),
                p.default.text(),
                p.min.text(),
                p.max.text(),
                flags
            ));
        }
        out
    }
}
