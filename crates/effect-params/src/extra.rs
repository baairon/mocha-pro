//! Kind specific extra data of a parameter: popup choices, slider precision and display
//! flags, a point default, arbitrary data and a button label.
//!
//! Text form (the last cell of a table row):
//!
//! - `?` unknown, `-` none.
//! - `c:<choice>|<choice>...` popup choices. `c:` followed by nothing is ZERO choices; one
//!   empty choice is `c:\e`. An empty part between bars (`c:a||b`) is refused in the strict
//!   (version 2) form and read as an empty choice by the lenient (version 1) form.
//! - `l:<label>` button label; an empty label is `l:\e` (`l:` alone is refused when strict).
//! - Escapes in a choice or a label: `\\` backslash, `\p` bar, `\t` tab, `\n` line feed,
//!   `\r` carriage return, and `\e` only as the whole text of an empty choice or label.
//!   Any other backslash sequence is an error.
//! - `s:<precision>:0x<display>:<b16>:<b16>` slider: precision is a decimal u8 (0 to 255; a
//!   plus sign only when lenient), display is `0x` and one or more hex digits of a u32, then
//!   the slider min and max as 16 hex digit float bit patterns.
//! - `p:<b16>:<b16>` point default.
//! - `d:<hex>` arbitrary data bytes: an even number of hex digits (either case), may be empty.
//!
//! The writer emits lower case hex and the escapes above only.

#[cfg(not(feature = "std"))]
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

/// Extra data of a parameter. Unknown is never the same as `Absent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Extra {
    Unknown,
    Absent,
    Choices(Vec<String>),
    Slider {
        precision: u8,
        display: u32,
        slider_min: u64,
        slider_max: u64,
    },
    Point {
        x: u64,
        y: u64,
    },
    Data(Vec<u8>),
    Label(String),
}

impl Extra {
    pub fn is_unknown(&self) -> bool {
        matches!(self, Extra::Unknown)
    }

    /// A slider from float values; compared by bit pattern.
    pub fn slider(precision: u8, display: u32, slider_min: f64, slider_max: f64) -> Extra {
        Extra::Slider {
            precision,
            display,
            slider_min: slider_min.to_bits(),
            slider_max: slider_max.to_bits(),
        }
    }

    pub fn point(x: f64, y: f64) -> Extra {
        Extra::Point {
            x: x.to_bits(),
            y: y.to_bits(),
        }
    }

    pub fn text(&self) -> String {
        match self {
            Extra::Unknown => "?".into(),
            Extra::Absent => "-".into(),
            Extra::Choices(c) => format!(
                "c:{}",
                c.iter().map(|s| escape(s)).collect::<Vec<_>>().join("|")
            ),
            Extra::Slider {
                precision,
                display,
                slider_min,
                slider_max,
            } => format!("s:{precision}:0x{display:x}:{slider_min:016x}:{slider_max:016x}"),
            Extra::Point { x, y } => format!("p:{x:016x}:{y:016x}"),
            Extra::Data(d) => {
                let mut s = String::from("d:");
                for b in d {
                    s.push_str(&format!("{b:02x}"));
                }
                s
            }
            Extra::Label(l) => format!("l:{}", escape(l)),
        }
    }

    /// Parse an extra cell with the leniency of version 1 (see the module documentation).
    pub fn parse(s: &str) -> Result<Extra, String> {
        Extra::parse_with(s, false)
    }

    /// Parse an extra cell in the strict version 2 form: additionally refused are a plus sign
    /// on the slider precision, an empty choice written as nothing between two bars (write
    /// `\e`), and an empty button label written as nothing after `l:` (write `l:\e`).
    pub fn parse_strict(s: &str) -> Result<Extra, String> {
        Extra::parse_with(s, true)
    }

    fn parse_with(s: &str, strict: bool) -> Result<Extra, String> {
        let bad = |m: &str| format!("bad extra {s:?}: {m}");
        if s == "?" {
            return Ok(Extra::Unknown);
        }
        if s == "-" {
            return Ok(Extra::Absent);
        }
        if let Some(r) = s.strip_prefix("c:") {
            let mut v = Vec::new();
            if !r.is_empty() {
                for part in r.split('|') {
                    if strict && part.is_empty() {
                        return Err(bad("empty choice, write \\e"));
                    }
                    v.push(unescape(part).map_err(|m| bad(&m))?);
                }
            }
            return Ok(Extra::Choices(v));
        }
        if let Some(r) = s.strip_prefix("l:") {
            if strict && r.is_empty() {
                return Err(bad("empty label, write \\e"));
            }
            return Ok(Extra::Label(unescape(r).map_err(|m| bad(&m))?));
        }
        if let Some(r) = s.strip_prefix("d:") {
            if r.len() % 2 != 0 || !r.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(bad("odd or non ascii hex"));
            }
            let mut v = Vec::new();
            for i in (0..r.len()).step_by(2) {
                v.push(u8::from_str_radix(&r[i..i + 2], 16).map_err(|e| bad(&e.to_string()))?);
            }
            return Ok(Extra::Data(v));
        }
        if let Some(r) = s.strip_prefix("p:") {
            let p: Vec<&str> = r.split(':').collect();
            if p.len() != 2 {
                return Err(bad("point needs two parts"));
            }
            return Ok(Extra::Point {
                x: bits(p[0]).map_err(|m| bad(&m))?,
                y: bits(p[1]).map_err(|m| bad(&m))?,
            });
        }
        if let Some(r) = s.strip_prefix("s:") {
            let p: Vec<&str> = r.split(':').collect();
            if p.len() != 4 {
                return Err(bad("slider needs four parts"));
            }
            if strict && p[0].starts_with('+') {
                return Err(bad("plus sign in precision"));
            }
            let precision = p[0].parse::<u8>().map_err(|e| bad(&e.to_string()))?;
            let h = p[1]
                .strip_prefix("0x")
                .ok_or_else(|| bad("display needs 0x"))?;
            if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(bad("display not hex"));
            }
            let display = u32::from_str_radix(h, 16).map_err(|e| bad(&e.to_string()))?;
            return Ok(Extra::Slider {
                precision,
                display,
                slider_min: bits(p[2]).map_err(|m| bad(&m))?,
                slider_max: bits(p[3]).map_err(|m| bad(&m))?,
            });
        }
        Err(bad("unknown tag"))
    }
}

fn bits(s: &str) -> Result<u64, String> {
    if s.len() != 16 {
        return Err("bit pattern needs 16 hex digits".into());
    }
    if !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("not a hex digit".into());
    }
    u64::from_str_radix(s, 16).map_err(|e| e.to_string())
}

fn escape(s: &str) -> String {
    if s.is_empty() {
        return "\\e".into();
    }
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '|' => o.push_str("\\p"),
            '\t' => o.push_str("\\t"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            c => o.push(c),
        }
    }
    o
}

fn unescape(s: &str) -> Result<String, String> {
    if s == "\\e" {
        return Ok(String::new());
    }
    let mut o = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            o.push(c);
            continue;
        }
        match it.next() {
            Some('\\') => o.push('\\'),
            Some('p') => o.push('|'),
            Some('t') => o.push('\t'),
            Some('n') => o.push('\n'),
            Some('r') => o.push('\r'),
            _ => return Err("bad escape".into()),
        }
    }
    Ok(o)
}
