//! Plain text case form read by the predictor (one contour, `key=value` lines).
//!
//! ```text
//! case=1
//! bounds=1              (optional, default 0)
//! meta=1                 (optional, default 0: the meta section stays empty)
//! time=<hex>              (repeat, query times in order)
//! key=<hex time>          (starts a keyframe)
//! pt=cusp,enabled,x,y,edge_w,edge_r,bx,by,fx,fy
//! meta.length=<text>      (optional, default UNKNOWN; same for rate, span_start, span_end)
//! ```

use crate::{parse_hex, UNKNOWN};
use shape_model::{Keyframe, PointData};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    pub length: String,
    pub rate: String,
    pub span_start: String,
    pub span_end: String,
}

impl Default for Meta {
    fn default() -> Meta {
        let u = || UNKNOWN.to_string();
        Meta {
            length: u(),
            rate: u(),
            span_start: u(),
            span_end: u(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    pub with_bounds: bool,
    pub with_meta: bool,
    pub times: Vec<f64>,
    pub keys: Vec<Keyframe>,
    pub meta: Meta,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CaseError {
    pub line: usize,
    pub what: String,
}

impl fmt::Display for CaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.what)
    }
}

impl std::error::Error for CaseError {}

fn err(line: usize, what: &str) -> CaseError {
    CaseError {
        line,
        what: what.to_string(),
    }
}

fn hexf(s: &str, line: usize) -> Result<f64, CaseError> {
    parse_hex(s).ok_or_else(|| err(line, "bad hex float"))
}

fn opt(s: &str, line: usize) -> Result<Option<f64>, CaseError> {
    if s == "none" {
        Ok(None)
    } else {
        hexf(s, line).map(Some)
    }
}

fn flag(s: &str, line: usize) -> Result<bool, CaseError> {
    match s {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(err(line, "bad flag")),
    }
}

fn parse_point(v: &str, line: usize) -> Result<PointData, CaseError> {
    let f: Vec<&str> = v.split(',').collect();
    if f.len() != 10 {
        return Err(err(line, "pt needs 10 fields"));
    }
    Ok(PointData {
        cusp: flag(f[0], line)?,
        enabled: flag(f[1], line)?,
        x: hexf(f[2], line)?,
        y: hexf(f[3], line)?,
        edge_w: opt(f[4], line)?,
        edge_r: opt(f[5], line)?,
        tan_in: (hexf(f[6], line)?, hexf(f[7], line)?),
        tan_out: (hexf(f[8], line)?, hexf(f[9], line)?),
    })
}

impl Case {
    pub fn parse(text: &str) -> Result<Case, CaseError> {
        let mut c = Case {
            with_bounds: false,
            with_meta: false,
            times: Vec::new(),
            keys: Vec::new(),
            meta: Meta::default(),
        };
        let mut seen_header = false;
        for (i, raw) in text.lines().enumerate() {
            let ln = i + 1;
            if raw.is_empty() {
                continue;
            }
            let (k, v) = raw.split_once('=').ok_or_else(|| err(ln, "no ="))?;
            match k {
                "case" if v == "1" => seen_header = true,
                "bounds" => c.with_bounds = flag(v, ln)?,
                "meta" => c.with_meta = flag(v, ln)?,
                "time" => c.times.push(hexf(v, ln)?),
                "key" => c.keys.push(Keyframe {
                    time: hexf(v, ln)?,
                    points: Vec::new(),
                }),
                "pt" => {
                    let p = parse_point(v, ln)?;
                    c.keys
                        .last_mut()
                        .ok_or_else(|| err(ln, "pt before key"))?
                        .points
                        .push(p);
                }
                "meta.length" => c.meta.length = v.to_string(),
                "meta.rate" => c.meta.rate = v.to_string(),
                "meta.span_start" => c.meta.span_start = v.to_string(),
                "meta.span_end" => c.meta.span_end = v.to_string(),
                _ => return Err(err(ln, "unknown key")),
            }
        }
        if !seen_header {
            return Err(err(0, "missing case=1"));
        }
        Ok(c)
    }
}
