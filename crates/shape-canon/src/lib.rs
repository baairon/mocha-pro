//! Canonical dump serializer, hex float codec, case text form and predictor.
//!
//! Dump rules: UTF-8, LF line ends, one `key=value` record per line in a fixed order,
//! floats as hexadecimal text (sign, `0x`, one leading digit, 13 fraction digits, `p`, signed
//! decimal exponent; `inf`, `-inf`, `nan` for the specials), integers in decimal,
//! booleans 0 or 1, absent optional values as `none`.

pub mod case;
pub mod hexfloat;

use shape_model::{Contour, PointData};

pub use case::{Case, CaseError, Meta};
pub use hexfloat::{fmt_hex, parse_hex};

/// Placeholder written for a value the predictor has no basis for.
pub const UNKNOWN: &str = "UNKNOWN";

pub fn fmt_bool(b: bool) -> &'static str {
    if b {
        "1"
    } else {
        "0"
    }
}

pub fn fmt_opt(v: Option<f64>) -> String {
    v.map_or_else(|| "none".to_string(), fmt_hex)
}

fn fmt_pair(p: (f64, f64)) -> String {
    format!("{},{}", fmt_hex(p.0), fmt_hex(p.1))
}

/// Accumulates records and renders the final bytes.
#[derive(Default)]
pub struct DumpWriter {
    out: String,
}

impl DumpWriter {
    pub fn new() -> DumpWriter {
        DumpWriter::default()
    }

    pub fn record(&mut self, key: &str, value: &str) {
        self.out.push_str(key);
        self.out.push('=');
        self.out.push_str(value);
        self.out.push('\n');
    }

    pub fn point(&mut self, prefix: &str, p: &PointData) {
        self.record(&format!("{prefix}.cusp"), fmt_bool(p.cusp));
        self.record(&format!("{prefix}.enabled"), fmt_bool(p.enabled));
        self.record(&format!("{prefix}.x"), &fmt_hex(p.x));
        self.record(&format!("{prefix}.y"), &fmt_hex(p.y));
        self.record(&format!("{prefix}.edge_w"), &fmt_opt(p.edge_w));
        self.record(&format!("{prefix}.edge_r"), &fmt_opt(p.edge_r));
        self.record(&format!("{prefix}.tan_in"), &fmt_pair(p.tan_in));
        self.record(&format!("{prefix}.tan_out"), &fmt_pair(p.tan_out));
    }

    pub fn finish(self) -> Vec<u8> {
        self.out.into_bytes()
    }
}

/// Render the full predicted dump for `contours` queried at `times`, with the meta block.
pub fn render_dump(
    contours: &[Contour],
    times: &[f64],
    with_bounds: bool,
    meta: Option<&Meta>,
) -> Vec<u8> {
    let mut w = DumpWriter::new();
    w.record("format", "shape-1");
    w.record("section", "shape");
    w.record("contours", &contours.len().to_string());
    for (ci, c) in contours.iter().enumerate() {
        w.record(&format!("c{ci}.points"), &c.point_count().to_string());
        for (ti, &t) in times.iter().enumerate() {
            let base = format!("c{ci}.t{ti}");
            w.record(&format!("{base}.time"), &fmt_hex(t));
            for (pi, p) in c.points_at(t).iter().enumerate() {
                w.point(&format!("{base}.p{pi}"), p);
            }
            if with_bounds {
                let r = c.bounds_at(t);
                let v = [r.left, r.top, r.width, r.height].map(fmt_hex).join(",");
                w.record(&format!("{base}.bounds"), &v);
            }
        }
    }
    w.record("section", "meta");
    if let Some(meta) = meta {
        w.record("project.length", &meta.length);
        w.record("project.rate", &meta.rate);
        w.record("layer.span_start", &meta.span_start);
        w.record("layer.span_end", &meta.span_end);
    }
    w.finish()
}

/// Predict the dump of a parsed case.
pub fn predict(case: &Case) -> Result<Vec<u8>, shape_model::ModelError> {
    let contour = Contour::new(case.keys.clone())?;
    Ok(render_dump(
        &[contour],
        &case.times,
        case.with_bounds,
        case.with_meta.then_some(&case.meta),
    ))
}
