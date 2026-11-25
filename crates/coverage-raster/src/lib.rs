//! `coverage-raster`: closed polygons in floating point coordinates to a one channel coverage buffer.
//!
//! The input is a list of [`Polygon`] values (each several closed rings) and a [`Target`] rectangle of
//! pixels. The output is a [`Coverage`] plane of doubles in 0 to 1, or a [`Buffer`] of 8 bit, 16 bit
//! or 32 bit float samples. The crate has no dependency and no platform specific code.
//!
//! Every convention that programs differ on is a field of [`Config`], and none is fixed in the code:
//!
//! - the coverage method ([`Method`]): exact area, a point at the pixel centre, or a fraction of a
//!   sample pattern ([`Pattern`]: a regular grid, a rotated grid, or explicit offsets);
//! - the fill rule ([`FillRule`]): non-zero or even odd winding;
//! - the pixel centre (`centre`: the centre of pixel `i` is at coordinate `i + centre`), the direction
//!   of rows (`y_down`) and snapping of coordinates to a grid (`snap_bits`);
//! - how coverage becomes a sample ([`Quantise`]: depth, scale, rounding mode);
//! - how several polygons combine into one plane ([`Combine`]) and the arithmetic width
//!   ([`Precision`]).
//!
//! [`Config::canonical`] is a stated default that claims nothing about any other program, and
//! [`Space::enumerate`] lists configurations in a fixed order for a search.
//!
//! Published methods: T. Duff, "Polygon Scan Conversion by Exact Area Calculation", in Raster Imaging
//! and Digital Typography, 1989 (exact area coverage); the winding rules of the OpenGL Utility Library
//! 1.3 specification, section 5; the top left tie rule of Microsoft's "Rasterization Rules (Direct3D
//! 11)" and the point sampling rule of the OpenGL specification. The exact area is found here by
//! cutting each pixel row into slabs (see `exact.rs`), which keeps it exact for self intersecting
//! input under either fill rule, where a plain accumulation of signed cell areas is not.
//!
//! # Exactness
//!
//! All arithmetic is double precision with the operations in a fixed order and no fused multiply add
//! (Rust does not fuse), so a result is bit for bit the same on every platform that follows IEEE
//! 754 binary64. `ExactArea` equals the true area exactly when the vertices are multiples of `2^-k`
//! for small `k` and no two edges cross inside a pixel row; in general its error is of the order of
//! `M * 2^-53` per slab, with `M` the largest absolute coordinate in pixel space (at most
//! [`MAX_COORD`] before the shift). `Point` and `Samples` are exact counts on their sample points,
//! up to the rounding of an edge's x at a sample height.

mod config;
mod edges;
mod error;
mod exact;
mod quant;
mod sample;

pub use config::{
    Combine, Config, Depth, FillRule, Method, Pattern, Polygon, Precision, Quantise, Rounding,
    Scale, Space, Target,
};
pub use error::Error;
pub use quant::{code, Buffer};
pub use sample::pattern_offsets;

/// The largest absolute coordinate accepted, `2^50`.
pub const MAX_COORD: f64 = 1_125_899_906_842_624.0;
/// The largest width or height of a target.
pub const MAX_SIDE: u32 = 1 << 16;
/// The largest number of pixels of a target.
pub const MAX_PIXELS: u64 = 1 << 26;

/// A plane of coverage values in 0 to 1, row by row.
#[derive(Debug, Clone, PartialEq)]
pub struct Coverage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f64>,
}

impl Coverage {
    /// The value at column `x`, row `y`.
    pub fn at(&self, x: u32, y: u32) -> f64 {
        self.data[y as usize * self.width as usize + x as usize]
    }

    /// The sum of all values (summed in buffer order).
    pub fn sum(&self) -> f64 {
        let mut s = 0.0;
        for &v in &self.data {
            s += v;
        }
        s
    }
}

fn round_p(v: f64, p: Precision) -> f64 {
    match p {
        Precision::Double => v,
        Precision::Single => (v as f32) as f64,
    }
}

fn fold(a: f64, b: f64, c: Combine, p: Precision) -> f64 {
    let r = match c {
        Combine::Max => a.max(b),
        Combine::AddClamp => (a + b).min(1.0),
        Combine::Over => a + b * (1.0 - a),
        Combine::Replace => {
            if b > 0.0 {
                b
            } else {
                a
            }
        }
    };
    round_p(r, p)
}

/// The coverage of one polygon alone.
fn one_polygon(
    index: usize,
    poly: &Polygon,
    target: &Target,
    cfg: &Config,
    offsets: &[(f64, f64)],
) -> Result<Vec<f64>, Error> {
    let w = target.width as usize;
    let h = target.height as usize;
    let edges = edges::polygon_edges(index, poly, target, cfg)?;
    let rows = edges::bucket(&edges, h);
    let mut plane = vec![0.0f64; w * h];
    let mut counts = vec![0u32; w];
    for (j, ids) in rows.iter().enumerate() {
        let row = &mut plane[j * w..(j + 1) * w];
        match &cfg.method {
            Method::ExactArea => {
                exact::exact_row(&edges, ids, j, cfg.fill, row);
                for v in row.iter_mut() {
                    *v = round_p(v.clamp(0.0, 1.0), cfg.precision);
                }
            }
            _ => {
                counts.iter_mut().for_each(|c| *c = 0);
                sample::sample_row(&edges, ids, j, offsets, cfg.fill, &mut counts);
                let n = offsets.len() as f64;
                for (v, &c) in row.iter_mut().zip(counts.iter()) {
                    *v = round_p(c as f64 / n, cfg.precision);
                }
            }
        }
    }
    Ok(plane)
}

/// Rasterise `polys` into a coverage plane covering `target`, under `cfg`. Polygons are folded into
/// the plane in order with `cfg.combine`. Parts outside the target are clipped; the inside area is
/// unchanged. Empty input gives zeros.
pub fn rasterise(polys: &[Polygon], target: &Target, cfg: &Config) -> Result<Coverage, Error> {
    cfg.validate()?;
    edges::check_target(target)?;
    let offsets = match &cfg.method {
        Method::ExactArea => Vec::new(),
        Method::Point => vec![(0.5, 0.5)],
        Method::Samples(p) => pattern_offsets(p),
    };
    let size = target.width as usize * target.height as usize;
    let mut acc = vec![0.0f64; size];
    for (index, poly) in polys.iter().enumerate() {
        let plane = one_polygon(index, poly, target, cfg, &offsets)?;
        for (a, &b) in acc.iter_mut().zip(plane.iter()) {
            *a = fold(*a, b, cfg.combine, cfg.precision);
        }
    }
    Ok(Coverage {
        width: target.width,
        height: target.height,
        data: acc,
    })
}

/// [`rasterise`], then the conversion of `cfg.quantise` (`None` gives `Buffer::F32`).
pub fn rasterise_buffer(polys: &[Polygon], target: &Target, cfg: &Config) -> Result<Buffer, Error> {
    let cov = rasterise(polys, target, cfg)?;
    Ok(quant::quantise(&cov.data, cfg.quantise.as_ref()))
}
